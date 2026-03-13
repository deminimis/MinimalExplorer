use serde::Serialize;
use std::time::SystemTime;
use std::os::windows::fs::MetadataExt;
use notify::{Watcher, RecursiveMode, EventKind};
use tauri::Emitter;

use super::*;

#[tauri::command]
pub fn cancel_load(pane_id: String, state: tauri::State<'_, LoadState>) {
    if let Some(pane) = state.0.get(&pane_id) {
        pane.token.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        pane.buffer.lock().unwrap().clear();
    }
}


#[tauri::command]
pub fn poll_load_chunk(pane_id: String, state: tauri::State<'_, LoadState>) -> Result<tauri::ipc::Response, String> {
    if let Some(pane) = state.0.get(&pane_id) {
        let mut chunks = pane.buffer.lock().unwrap();
        if chunks.is_empty() {
            Ok(tauri::ipc::Response::new(Vec::new()))
        } else {
            Ok(tauri::ipc::Response::new(chunks.remove(0)))
        }
    } else {
        Ok(tauri::ipc::Response::new(Vec::new()))
    }
}

// Zero-copy Binary Encoder
pub fn encode_files(items: &[FileItem]) -> Vec<u8> {
    let exact_size = 4 + items.iter().map(|item| {
        2 + item.name.len() + 
        2 + item.path.len() + 
        8 + 8 + 1 + 
        item.snippet.as_ref().map_or(0, |s| 2 + s.len())
    }).sum::<usize>();

    let mut buf = Vec::with_capacity(exact_size);
    buf.extend_from_slice(&(items.len() as u32).to_le_bytes()); // Total items
    for item in items {
        let name_b = item.name.as_bytes();
        buf.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        buf.extend_from_slice(name_b);

        let path_b = item.path.as_bytes();
        buf.extend_from_slice(&(path_b.len() as u16).to_le_bytes());
        buf.extend_from_slice(path_b);

        buf.extend_from_slice(&item.size.to_le_bytes());
        buf.extend_from_slice(&item.modified.to_le_bytes());

        let mut flags = 0u8;
        if item.is_dir { flags |= 1; }
        if item.is_hidden { flags |= 2; }
        if item.snippet.is_some() { flags |= 4; }
        buf.push(flags);

        if let Some(ref snippet) = item.snippet {
            let snip_b = snippet.as_bytes();
            buf.extend_from_slice(&(snip_b.len() as u16).to_le_bytes());
            buf.extend_from_slice(snip_b);
        }
    }
    buf
}


#[derive(Clone, Serialize)]
pub struct  WatchEvent {
    watched_path: String,
    item: Option<FileItem>,
    deleted_path: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct FileItem {
    pub name: String,
    pub is_dir: bool,
    pub path: String,
    pub size: u64,
    pub modified: u64,
    pub is_hidden: bool,
    pub snippet: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct  QuickAccessItem {
    name: String,
    path: String,
    is_drive: bool,
    total_space: Option<u64>,
    free_space: Option<u64>,
}

#[derive(Clone, Serialize)]
pub struct  FileProperties {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    created: u64,
    modified: u64,
    readonly: bool,
}

#[tauri::command]
pub fn get_file_properties(path: String, app: tauri::AppHandle) -> Result<FileProperties, String> {
    let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    let name = std::path::Path::new(&path).file_name().unwrap_or_default().to_string_lossy().into_owned();
    
    let created = meta.created().ok().and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    let modified = meta.modified().ok().and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    let readonly = meta.permissions().readonly();

    // Default size to 0 for directories so the modal opens instantly
    let size = if meta.is_dir() { 0 } else { meta.len() };

    if meta.is_dir() {
        let path_clone = path.clone();
        let app_clone = app.clone(); 
        
        tauri::async_runtime::spawn(async move {
            let total_size: u64 = jwalk::WalkDir::new(&path_clone)
                .into_iter()
                .filter_map(|e| e.ok())
                .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
                .sum();

            #[derive(Clone, serde::Serialize)]
            struct FolderSizePayload {
                path: String,
                size: u64,
            }

            let _ = app_clone.emit("folder_size_update", FolderSizePayload {
                path: path_clone,
                size: total_size,
            });
        });
    }

    Ok(FileProperties {
        name: if name.is_empty() { path.clone() } else { name },
        path,
        is_dir: meta.is_dir(),
        size,
        created,
        modified,
        readonly,
    })
}

#[tauri::command]
pub fn show_windows_properties(path: String) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_INVOKEIDLIST};
    use windows::core::PCWSTR;

    let wide_path: Vec<u16> = std::ffi::OsStr::new(&path).encode_wide().chain(std::iter::once(0)).collect();
    let verb: Vec<u16> = std::ffi::OsStr::new("properties").encode_wide().chain(std::iter::once(0)).collect();

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_INVOKEIDLIST,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(wide_path.as_ptr()),
        nShow: 5, 
        ..Default::default()
    };

    unsafe {
        ShellExecuteExW(&mut info).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_quick_access() -> Result<Vec<QuickAccessItem>, String> {
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;

    let mut items = Vec::new();
    // Mounted Drives (A-Z)
    for i in b'A'..=b'Z' {
        let drive = format!("{}:\\", i as char);
        if std::path::Path::new(&drive).exists() {
            let wide_path: Vec<u16> = std::ffi::OsStr::new(&drive).encode_wide().chain(std::iter::once(0)).collect();
            let mut free_bytes: u64 = 0;
            let mut total_bytes: u64 = 0;
            let (total_space, free_space) = if unsafe { GetDiskFreeSpaceExW(PCWSTR(wide_path.as_ptr()), Some(&mut free_bytes), Some(&mut total_bytes), None) }.is_ok() {
                (Some(total_bytes), Some(free_bytes))
            } else {
                (None, None)
            };
            items.push(QuickAccessItem { 
                name: format!("Local Disk ({}:)", i as char), 
                path: drive.clone(), 
                is_drive: true,
                total_space,
                free_space
            });
        }
    }
    
    items.push(QuickAccessItem { name: "Desktop".to_string(), path: std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string()) + "\\Desktop\\", is_drive: false, total_space: None, free_space: None });
    items.push(QuickAccessItem { name: "Downloads".to_string(), path: std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string()) + "\\Downloads\\", is_drive: false, total_space: None, free_space: None });
    items.push(QuickAccessItem { name: "Documents".to_string(), path: std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string()) + "\\Documents\\", is_drive: false, total_space: None, free_space: None });
    items.push(QuickAccessItem { name: "Pictures".to_string(), path: std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string()) + "\\Pictures\\", is_drive: false, total_space: None, free_space: None });

    Ok(items)
}


#[tauri::command]
pub async fn get_wsl_distros() -> Result<Vec<QuickAccessItem>, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let mut distros = Vec::new();
    
    // Dynamically determine the correct WSL base path (Win 11 vs Win 10)
    let base_path = if std::path::Path::new(r"\\wsl.localhost\").exists() {
        r"\\wsl.localhost\"
    } else {
        r"\\wsl$\"
    };

    // Use wsl.exe to list distros natively
    if let Ok(output) = std::process::Command::new("wsl")
        .arg("-l")
        .arg("-q")
        .creation_flags(CREATE_NO_WINDOW) 
        .output() 
    {
        // WSL output is typically UTF-16 LE
        let stdout = if output.stdout.len() >= 2 && output.stdout[0] == 0xFF && output.stdout[1] == 0xFE {
            let u16_words: Vec<u16> = output.stdout[2..]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&u16_words)
        } else {
            String::from_utf8_lossy(&output.stdout).to_string()
        };

        for line in stdout.lines() {
            let distro = line.replace('\x00', "").trim().to_string();
            if !distro.is_empty() && !distro.contains("Windows Subsystem") {
                distros.push(QuickAccessItem {
                    name: distro.clone(),
                    path: format!("{}{}\\", base_path, distro), // Enforce trailing slash
                    is_drive: true,
                    total_space: None,
                    free_space: None,
                });
            }
        }
    }
    
    // Fallback if wsl.exe is missing/fails but the network share still exists
    if distros.is_empty() {
        if let Ok(entries) = std::fs::read_dir(base_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let distro_name = entry.file_name().to_string_lossy().into_owned();
                distros.push(QuickAccessItem {
                    name: distro_name.clone(),
                    path: format!("{}{}", base_path, distro_name), 
                    is_drive: true,
                    total_space: None,
                    free_space: None,
                });
            }
        }
    }
    Ok(distros)
}

#[tauri::command]
pub fn get_directories(path: String, show_git_badges: bool) -> Result<Vec<FileItem>, String> {
    // Intercept virtual "This PC" path
    if path == "This PC" {
        if let Ok(drives) = get_quick_access() {
            return Ok(drives.into_iter().map(|d| FileItem {
                name: d.name, is_dir: true, path: d.path, size: 0, modified: 0, is_hidden: false, snippet: None
            }).collect());
        }
        return Ok(Vec::new());
    }

    let mut dirs = Vec::new();
    
    // UNC paths (like WSL) strictly require a trailing slash to be read
    let mut fetch_path = path.clone();
    if fetch_path.starts_with(r"\\") && !fetch_path.ends_with('\\') {
        fetch_path.push('\\');
    }

    // If access is denied, just return an empty vector.
    let Ok(entries) = std::fs::read_dir(&fetch_path) else {
        return Ok(Vec::new());
    };

    for entry in entries.flatten() {
        if let Ok(file_type) = entry.file_type() {
            if file_type.is_dir() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let path_str = entry.path().to_string_lossy().into_owned();
                
                let is_hidden = name.starts_with('.') || 
                    entry.metadata().map(|m| m.file_attributes() & 2 != 0).unwrap_or(false);
                
                let mut snippet = None;
                if show_git_badges {
                    let head_path = format!("{}\\{}", path_str, ".git\\HEAD");
                    if let Ok(head) = std::fs::read_to_string(&head_path) {
                        if head.starts_with("ref: refs/heads/") {
                            snippet = Some(head.trim_start_matches("ref: refs/heads/").trim().to_string());
                        } else if head.len() >= 7 {
                            snippet = Some(head[0..7].to_string());
                        }
                    }
                }

                dirs.push(FileItem {
                    name,
                    is_dir: true,
                    path: path_str,
                    size: 0,
                    modified: 0,
                    is_hidden,
                    snippet,
                });
            }
        }
    }
    
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(dirs)
}

#[tauri::command]
pub async fn get_files(pane_id: String, path: String, _sort_by: String, _sort_ascending: bool, show_git_badges: bool, state: tauri::State<'_, LoadState>) -> Result<(), String> {
    let pane = state.get_pane(&pane_id);
    let token = pane.token.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;

    // Intercept virtual "This PC" path
    if path == "This PC" {
        if let Ok(drives) = get_quick_access() {
            let items: Vec<FileItem> = drives.into_iter().map(|d| FileItem {
                name: d.name,
                is_dir: true,
                path: d.path,
                size: 0,
                modified: 0,
                is_hidden: false,
                snippet: None
            }).collect();
            if pane.token.load(std::sync::atomic::Ordering::Relaxed) == token {
                pane.buffer.lock().unwrap().push(encode_files(&items));
            }
        }
        return Ok(());
    }

    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        FindFirstFileW, FindNextFileW, FindClose, WIN32_FIND_DATAW, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_HIDDEN
    };

    let mut items: Vec<FileItem> = Vec::new();

    if path.starts_with(r"\\") {
        // BYPASS FindFirstFileW for UNC paths (WSL, Network Drives)
        let mut fetch_path = path.clone();
        if !fetch_path.ends_with('\\') { fetch_path.push('\\'); }
        if let Ok(entries) = std::fs::read_dir(&fetch_path) {
            for entry in entries.flatten() {
                if pane.token.load(std::sync::atomic::Ordering::Relaxed) != token { return Ok(()); }
                let name = entry.file_name().to_string_lossy().into_owned();
                let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                let mut size = 0;
                let mut modified = 0;
                let mut is_hidden = name.starts_with('.');
                if let Ok(meta) = entry.metadata() {
                    size = meta.len();
                    modified = meta.modified().ok().and_then(|t| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
                    is_hidden = is_hidden || (meta.file_attributes() & 2 != 0);
                }
                let path_str = format!("{}{}", fetch_path, name);
                items.push(FileItem { name, is_dir, path: path_str, size, modified, is_hidden, snippet: None });
                
                if items.len() >= 250 {
                    pane.buffer.lock().unwrap().push(encode_files(&items));
                    items.clear();
                }
            }
        }
    } else {
        let search_path = if path.ends_with('\\') || path.ends_with('/') { format!("{}*", path) } else { format!("{}\\*", path) };
        let wide_path: Vec<u16> = std::ffi::OsStr::new(&search_path).encode_wide().chain(std::iter::once(0)).collect();

        unsafe {
            let mut find_data: WIN32_FIND_DATAW = std::mem::zeroed();
            if let Ok(handle) = FindFirstFileW(PCWSTR(wide_path.as_ptr()), &mut find_data) {
                if !handle.is_invalid() {
                    loop {
                        if pane.token.load(std::sync::atomic::Ordering::Relaxed) != token { return Ok(()); }

                        let name_len = find_data.cFileName.iter().position(|&c| c == 0).unwrap_or(find_data.cFileName.len());
                        let name_slice = &find_data.cFileName[..name_len];
                        
                        if name_slice != [46] && name_slice != [46, 46] { // Skip "." and ".."
                            let name = String::from_utf16_lossy(name_slice);
                            
                            let is_dir = (find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0) != 0;
                            let is_hidden = name.starts_with('.') || (find_data.dwFileAttributes & FILE_ATTRIBUTE_HIDDEN.0) != 0;
                            
                            let size = ((find_data.nFileSizeHigh as u64) << 32) | (find_data.nFileSizeLow as u64);
                            
                            let ticks = ((find_data.ftLastWriteTime.dwHighDateTime as u64) << 32) | (find_data.ftLastWriteTime.dwLowDateTime as u64);
                            let modified = if ticks >= 116444736000000000 { (ticks - 116444736000000000) / 10000000 } else { 0 };

                            let path_str = if path.ends_with('\\') || path.ends_with('/') { format!("{}{}", path, name) } else { format!("{}\\{}", path, name) };

                            let mut snippet = None;
                            if show_git_badges && is_dir {
                                let head_path = format!("{}\\{}", path_str, ".git\\HEAD");
                                if let Ok(head) = std::fs::read_to_string(&head_path) {
                                    if head.starts_with("ref: refs/heads/") {
                                        snippet = Some(head.trim_start_matches("ref: refs/heads/").trim().to_string());
                                    } else if head.len() >= 7 {
                                        snippet = Some(head[0..7].to_string());
                                    }
                                }
                            }

                            items.push(FileItem { name, is_dir, path: path_str, size, modified, is_hidden, snippet });

                            if items.len() >= 250 {
                                pane.buffer.lock().unwrap().push(encode_files(&items));
                                items.clear();
                            }
                        }
                        if FindNextFileW(handle, &mut find_data).is_err() { break; }
                    }
                    let _ = FindClose(handle);
                }
            }
        }
    }

    if !items.is_empty() {
        if pane.token.load(std::sync::atomic::Ordering::Relaxed) == token {
            pane.buffer.lock().unwrap().push(encode_files(&items));
        }
    }

    Ok(())
}


#[tauri::command]
pub fn get_launch_path() -> Option<String> { 
    std::env::args().nth(1) 
}



#[tauri::command]
pub fn watch_directory(path: String, app_handle: tauri::AppHandle, state: tauri::State<'_, WatcherState>) -> Result<(), String> {
    // Bypass watcher for virtual folders and UNC paths (WSL & Network drives).
    if path == "This PC" || path.starts_with(r"\\") || path.starts_with("//") { 
        return Ok(()); 
    }
    
    let mut watchers = state.0.lock().unwrap();
    if watchers.contains_key(&path) { return Ok(()); }
    
    let path_clone = path.clone();
    let (tx, rx) = std::sync::mpsc::channel::<WatchEvent>();
    let app_handle_thread = app_handle.clone();
    
    std::thread::spawn(move || {
        let mut dedupe_map = std::collections::HashMap::new();
        loop {
            match rx.recv() {
                Ok(event) => {
                    let key = event.item.as_ref().map(|i| i.path.clone()).unwrap_or_else(|| event.deleted_path.clone().unwrap_or_default());
                    dedupe_map.insert(key, event);
                    
                    let batch_start = std::time::Instant::now(); // Track how long we've been holding events

                    // 100ms trailing debounce, with a strict 500ms maximum hold time
                    loop {
                        match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                            Ok(evt) => {
                                let evt_key = evt.item.as_ref().map(|i| i.path.clone()).unwrap_or_else(|| evt.deleted_path.clone().unwrap_or_default());
                                dedupe_map.insert(evt_key, evt); 
                                
                                // Force flush to the UI if we've been batching for over 500ms
                                if batch_start.elapsed().as_millis() >= 500 {
                                    break;
                                }
                            }
                            Err(_) => break, // Timeout or disconnected, safe to flush
                        }
                    }
                    
                    let buffer: Vec<WatchEvent> = dedupe_map.drain().map(|(_, v)| v).collect();
                    let _ = app_handle_thread.emit("items_changed", &buffer);
                }
                Err(_) => break,
            }
        }
    });

    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
        if let Ok(event) = res {
            for p in event.paths {
                let path_str = p.to_string_lossy().to_string();
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        if let Ok(metadata) = std::fs::metadata(&p) {
                            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                            let is_dir = metadata.is_dir();
                            let size = metadata.len();
                            let modified = metadata.modified().ok().and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
                            let is_hidden = name.starts_with('.') || (metadata.file_attributes() & 2 != 0);

                            let item = FileItem { name, is_dir, path: path_str, size, modified, is_hidden, snippet: None };
                            let _ = tx.send(WatchEvent {
                                watched_path: path_clone.clone(),
                                item: Some(item),
                                deleted_path: None,
                            });
                        }
                    },
                    EventKind::Remove(_) => {
                        let _ = tx.send(WatchEvent {
                            watched_path: path_clone.clone(),
                            item: None,
                            deleted_path: Some(path_str),
                        });
                    },
                    _ => {}
                }
            }
        }
    }).map_err(|e| e.to_string())?;
    
    // Fail gracefully if the OS refuses to watch the path (like Network/WSL drives)
    if let Err(e) = watcher.watch(std::path::Path::new(&path), RecursiveMode::NonRecursive) {
        println!("Warning: Could not watch directory: {}", e);
        return Ok(()); // Return Ok so the frontend continues loading the folder anyway
    }
    
    watchers.insert(path, watcher);
    
    Ok(())
}

#[tauri::command]
pub fn unwatch_all_except(active_paths: Vec<String>, state: tauri::State<'_, WatcherState>) -> Result<(), String> {
    let mut watchers = state.0.lock().unwrap();
    watchers.retain(|k, _| active_paths.contains(k));
    Ok(())
}

pub fn register_handlers(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.invoke_handler(tauri::generate_handler![
        get_files, 
        get_directories,
        get_quick_access,
        get_wsl_distros,
        get_launch_path,
        cancel_load, 
        poll_load_chunk, 
        get_file_properties,
        show_windows_properties, 
        watch_directory, 
        unwatch_all_except, 

        crate::media::precache_thumbnails,

        crate::search::search_contents, 
        crate::search::search_directory,
        crate::search::cancel_search,
        crate::search::poll_search_chunk, 
        crate::search::rebuild_index, 
        crate::search::instant_search,

        crate::operations::open_file,
        crate::operations::open_with, 
        crate::operations::delete_items,
        crate::operations::rename_item,
        crate::operations::create_item, 
        crate::operations::copy_item,
        crate::operations::move_item, 
        crate::operations::read_text_file, 
        crate::operations::read_file_raw,
        crate::operations::write_text_file,
        crate::operations::compress_item, 
        crate::operations::extract_item,
        crate::operations::open_in_terminal, 
        crate::operations::spawn_pty, 
        crate::operations::write_pty,
        crate::operations::resize_pty, 
        crate::operations::kill_pty, 
        crate::operations::window_minimize,
        crate::operations::window_toggle_maximize, 
        crate::operations::window_close, 
        crate::operations::window_toggle_fullscreen,
        crate::operations::set_theme,
        crate::operations::calculate_file_checksum,
    ])
}