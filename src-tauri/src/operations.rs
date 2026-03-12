use super::*;
use std::io::{Read, Write};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use tauri::Emitter;

#[tauri::command]
pub fn extract_item(src: String, dest: String) -> Result<(), String> {
    let file = std::fs::File::open(&src).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    archive.extract(&dest).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn compress_item(src: String, dest: String, method: String, level: i32) -> Result<(), String> {
    let file = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);

    let comp_method = match method.as_str() {
        "stored" => zip::CompressionMethod::Stored,
        _ => zip::CompressionMethod::Deflated,
    };

    let options = zip::write::FileOptions::default()
        .compression_method(comp_method)
        .compression_level(Some(level));
    
    let src_path = std::path::Path::new(&src);
    if src_path.is_file() {
        zip.start_file(src_path.file_name().unwrap().to_string_lossy(), options).map_err(|e| e.to_string())?;
        let mut f = std::fs::File::open(src_path).map_err(|e| e.to_string())?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
        zip.write_all(&buffer).map_err(|e| e.to_string())?;
    } else if src_path.is_dir() {
        let prefix = src_path.parent().unwrap_or(src_path);
        for entry in jwalk::WalkDir::new(src_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = path.strip_prefix(prefix).unwrap().to_string_lossy().into_owned().replace("\\", "/");
            if path.is_file() {
                zip.start_file(name, options).map_err(|e| e.to_string())?;
                let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
                zip.write_all(&buffer).map_err(|e| e.to_string())?;
            } else if !name.is_empty() {
                zip.add_directory(name, options).map_err(|e| e.to_string())?;
            }
        }
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn open_with(path: String) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("rundll32.exe")
        .raw_arg(format!("shell32.dll,OpenAs_RunDLL {}", path))
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_file(path: String) -> Result<(), String> {
    open::that(path).map_err(|e| e.to_string())
}

pub unsafe fn create_file_operation() -> Result<windows::Win32::UI::Shell::IFileOperation, String> {
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Shell::FileOperation;
    CoCreateInstance(&FileOperation, None, CLSCTX_INPROC_SERVER).map_err(|e| e.to_string())
}

pub unsafe fn get_shell_item(path: &str) -> Result<windows::Win32::UI::Shell::IShellItem, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::SHCreateItemFromParsingName;
    let wide_path: Vec<u16> = std::ffi::OsStr::new(path).encode_wide().chain(std::iter::once(0)).collect();
    SHCreateItemFromParsingName(PCWSTR(wide_path.as_ptr()), None).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_items(paths: Vec<String>, permanent: bool) -> Result<(), String> {
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::FOF_ALLOWUNDO;

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let op = create_file_operation()?;
        
        if !permanent {
            op.SetOperationFlags(FOF_ALLOWUNDO).map_err(|e| e.to_string())?;
        }
        
        // Stage all paths into a single operation buffer
        for path in paths {
            if let Ok(item) = get_shell_item(&path) {
                let _ = op.DeleteItem(&item, None);
            }
        }
        
        // Executes the operation, showing the OS dialogue and sound
        op.PerformOperations().map_err(|e| e.to_string())?;
        
        CoUninitialize();
    }
    Ok(())
}

#[tauri::command]
pub fn create_item(path: String, is_dir: bool) -> Result<(), String> {
    if is_dir { std::fs::create_dir(path) } else { std::fs::File::create(path).map(|_| ()) }.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_item(old_path: String, new_path: String) -> Result<(), String> {
    std::fs::rename(old_path, new_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_item(src: String, dest: String) -> Result<(), String> {
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
    use windows::core::PCWSTR;
    use std::os::windows::ffi::OsStrExt;

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let op = create_file_operation()?;
        let src_item = get_shell_item(&src)?;
        let dest_path = std::path::Path::new(&dest);
        let dest_folder = dest_path.parent().unwrap_or(dest_path).to_string_lossy().to_string();
        let mut dest_name = dest_path.file_name().unwrap_or_default().to_string_lossy().to_string();

        if dest_path.exists() {
            let stem = dest_path.file_stem().unwrap_or_default().to_string_lossy();
            let ext = dest_path.extension().unwrap_or_default().to_string_lossy();
            dest_name = if ext.is_empty() { format!("{} (1)", stem) } else { format!("{} (1).{}", stem, ext) };
        }

        let dest_folder_item = get_shell_item(&dest_folder)?;
        let wide_dest_name: Vec<u16> = std::ffi::OsStr::new(&dest_name).encode_wide().chain(std::iter::once(0)).collect();

        op.CopyItem(&src_item, &dest_folder_item, PCWSTR(wide_dest_name.as_ptr()), None).map_err(|e| e.to_string())?;
        op.PerformOperations().map_err(|e| e.to_string())?;
        CoUninitialize();
    }
    Ok(())
}

#[tauri::command]
pub fn move_item(src: String, dest: String) -> Result<(), String> {
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
    use windows::core::PCWSTR;
    use std::os::windows::ffi::OsStrExt;

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let op = create_file_operation()?;
        let src_item = get_shell_item(&src)?;
        let dest_path = std::path::Path::new(&dest);
        let dest_folder = dest_path.parent().unwrap_or(dest_path).to_string_lossy().to_string();
        let mut dest_name = dest_path.file_name().unwrap_or_default().to_string_lossy().to_string();

        if dest_path.exists() {
            let stem = dest_path.file_stem().unwrap_or_default().to_string_lossy();
            let ext = dest_path.extension().unwrap_or_default().to_string_lossy();
            dest_name = if ext.is_empty() { format!("{} (1)", stem) } else { format!("{} (1).{}", stem, ext) };
        }

        let dest_folder_item = get_shell_item(&dest_folder)?;
        let wide_dest_name: Vec<u16> = std::ffi::OsStr::new(&dest_name).encode_wide().chain(std::iter::once(0)).collect();

        op.MoveItem(&src_item, &dest_folder_item, PCWSTR(wide_dest_name.as_ptr()), None).map_err(|e| e.to_string())?;
        op.PerformOperations().map_err(|e| e.to_string())?;
        CoUninitialize();
    }
    Ok(())
}

#[tauri::command]
pub fn read_text_file(path: String, offset: u64, length: u64) -> Result<String, String> {
    let file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    let file_len = file.metadata().map_err(|e| e.to_string())?.len();

    if offset >= file_len { return Ok("".to_string()); }
    let end = (offset + length).min(file_len);
    let actual_length = (end - offset) as usize;

    let mmap = unsafe { 
        memmap2::MmapOptions::new().offset(offset).len(actual_length).map(&file).map_err(|e| e.to_string())? 
    };
    Ok(String::from_utf8_lossy(&mmap).into_owned())
}

#[tauri::command]
pub fn read_file_raw(path: String) -> Result<Vec<u8>, String> {
    std::fs::read(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_text_file(path: String, contents: String) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_in_terminal(path: String, admin: bool, shell: String) -> Result<(), String> {
    let shell_cmd = if shell == "powershell" {
        if std::process::Command::new("pwsh").arg("-Version").output().is_ok() { "pwsh".to_string() } else { "powershell".to_string() }
    } else if shell == "cmd" { "cmd".to_string() } else { shell.clone() };

    if admin {
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &format!("& {{ Start-Process '{}' -WorkingDirectory $args[0] -Verb RunAs }}", shell_cmd), "-args", &path])
            .spawn()
    } else {
        std::process::Command::new("cmd")
            .args(["/c", "start", "\"\"", &shell_cmd])
            .current_dir(path)
            .spawn()
    }.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn window_minimize(window: tauri::Window) {
    let _ = window.minimize();
}

#[tauri::command]
pub fn window_toggle_maximize(window: tauri::Window) {
    if let Ok(is_max) = window.is_maximized() { let _ = if is_max { window.unmaximize() } else { window.maximize() }; }
}

#[tauri::command]
pub fn window_toggle_fullscreen(window: tauri::Window) {
    if let Ok(is_fullscreen) = window.is_fullscreen() { let _ = window.set_fullscreen(!is_fullscreen); }
}

#[tauri::command]
pub fn window_close(window: tauri::Window) {
    let _ = window.close();
}

#[tauri::command]
pub fn spawn_pty(id: String, rows: u16, cols: u16, cwd: String, shell: String, app_handle: tauri::AppHandle, state: tauri::State<'_, PtyState>) -> Result<(), String> {
    let pty_system = NativePtySystem::default();
    let size = PtySize { rows, cols, pixel_width: 0, pixel_height: 0 };
    let pair = pty_system.openpty(size).map_err(|e| e.to_string())?;
    
    let mut cmd = if shell == "powershell" {
        if std::process::Command::new("pwsh").arg("-Version").output().is_ok() { CommandBuilder::new("pwsh") } else { CommandBuilder::new("powershell") }
    } else if shell == "cmd" { CommandBuilder::new("cmd.exe") } else { CommandBuilder::new(&shell) };
    cmd.cwd(&cwd);

    let _child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    state.0.lock().unwrap().insert(id.clone(), PtyInstance { master: pair.master, writer });

    std::thread::spawn(move || {
        let mut buf = [0; 1024];
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 { break; }
            let _ = app_handle.emit(&format!("pty-out-{}", id), String::from_utf8_lossy(&buf[..n]).to_string());
        }
    });
    Ok(())
}

#[tauri::command]
pub fn write_pty(id: String, data: String, state: tauri::State<'_, PtyState>) -> Result<(), String> {
    if let Some(instance) = state.0.lock().unwrap().get_mut(&id) { let _ = instance.writer.write_all(data.as_bytes()); }
    Ok(())
}

#[tauri::command]
pub fn resize_pty(id: String, rows: u16, cols: u16, state: tauri::State<'_, PtyState>) -> Result<(), String> {
    if let Some(instance) = state.0.lock().unwrap().get_mut(&id) { let _ = instance.master.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 }); }
    Ok(())
}

#[tauri::command]
pub fn kill_pty(id: String, state: tauri::State<'_, PtyState>) -> Result<(), String> {
    state.0.lock().unwrap().remove(&id);
    Ok(())
}

#[tauri::command]
pub fn set_theme(window: tauri::Window, theme: String) -> Result<(), String> {
    let _ = window.set_theme(match theme.as_str() { "dark" => Some(tauri::Theme::Dark), "light" => Some(tauri::Theme::Light), _ => None });
    Ok(())
}

use sha2::{Sha256, Digest};

#[tauri::command]
pub fn calculate_file_checksum(path: String) -> Result<String, String> {
    let mut file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192]; // 8KB read buffer
    
    while let Ok(n) = file.read(&mut buffer) {
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }
    
    Ok(format!("{:x}", hasher.finalize()))
}