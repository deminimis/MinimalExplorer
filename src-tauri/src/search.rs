use crate::commands::{encode_files, FileItem};
use crate::{SearchState, SearchIndex, SEARCH_INDEX};
use redb::ReadableTable;
use std::time::SystemTime;
use std::os::windows::fs::MetadataExt;

#[tauri::command]
pub fn cancel_search(state: tauri::State<'_, SearchState>) {
    state.token.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    state.buffer.lock().unwrap().clear();
}

#[tauri::command]
pub fn poll_search_chunk(state: tauri::State<'_, SearchState>) -> Result<tauri::ipc::Response, String> {
    let mut chunks = state.buffer.lock().unwrap();
    if chunks.is_empty() {
        Ok(tauri::ipc::Response::new(Vec::new()))
    } else {
        Ok(tauri::ipc::Response::new(chunks.remove(0)))
    }
}

// --- Advanced Search Parser ---
pub struct SearchFilters {
    name_query: String,
    ext: Option<String>,
    min_size: Option<u64>,
    max_size: Option<u64>,
    is_dir: Option<bool>,
    min_date: Option<u64>,
    max_date: Option<u64>,
    is_hidden: Option<bool>,
}

pub fn parse_query(query: &str) -> SearchFilters {
    let mut name_parts = Vec::new();
    let mut ext = None;
    let mut min_size = None;
    let mut max_size = None;
    let mut is_dir = None;
    let mut min_date = None;
    let mut max_date = None;
    let mut is_hidden = None;

    let now = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs();

    for part in query.split_whitespace() {
        let lower = part.to_lowercase();
        if lower.starts_with("ext:") {
            ext = Some(lower[4..].trim_start_matches('.').to_string());
        } else if lower.starts_with("size:>") {
            min_size = parse_size(&lower[6..]);
        } else if lower.starts_with("size:<") {
            max_size = parse_size(&lower[6..]);
        } else if lower.starts_with("date:>") {
            min_date = parse_date_offset(&lower[6..], now).map(|offset| now.saturating_sub(offset));
        } else if lower.starts_with("date:<") {
            max_date = parse_date_offset(&lower[6..], now).map(|offset| now.saturating_sub(offset));
        } else if lower == "type:dir" || lower == "type:folder" {
            is_dir = Some(true);
        } else if lower == "type:file" {
            is_dir = Some(false);
        } else if lower == "is:hidden" {
            is_hidden = Some(true);
        } else {
            name_parts.push(part);
        }
    }

    SearchFilters {
        name_query: name_parts.join(" ").to_lowercase(),
        ext, min_size, max_size, is_dir, min_date, max_date, is_hidden
    }
}

pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim();
    let mut multiplier: f64 = 1.0;
    let num_str = if s.ends_with("kb") { multiplier = 1024.0; &s[..s.len()-2] }
                  else if s.ends_with("mb") { multiplier = 1048576.0; &s[..s.len()-2] }
                  else if s.ends_with("gb") { multiplier = 1073741824.0; &s[..s.len()-2] }
                  else if s.ends_with("b") { &s[..s.len()-1] }
                  else { s };
    num_str.parse::<f64>().ok().map(|n| (n * multiplier) as u64)
}

pub fn parse_date_offset(s: &str, _now: u64) -> Option<u64> {
    let s = s.trim();
    let mut multiplier: u64 = 1;
    let num_str = if s.ends_with("h") { multiplier = 3600; &s[..s.len()-1] }
                  else if s.ends_with("d") { multiplier = 86400; &s[..s.len()-1] }
                  else if s.ends_with("w") { multiplier = 604800; &s[..s.len()-1] }
                  else { s };
    num_str.parse::<u64>().ok().map(|n| n * multiplier)
}

#[tauri::command]
pub async fn search_directory(path: String, query: String, state: tauri::State<'_, SearchState>) -> Result<(), String> {
    let token = state.token.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    state.buffer.lock().unwrap().clear();

    let filters = parse_query(&query);
    let mut batch = Vec::new();
    let want_hidden = filters.is_hidden.unwrap_or(false);

    // BFS Queue Setup
    let mut dirs_to_visit = std::collections::VecDeque::new();
    dirs_to_visit.push_back(std::path::PathBuf::from(&path));

    while let Some(current_dir) = dirs_to_visit.pop_front() {
        if state.token.load(std::sync::atomic::Ordering::Relaxed) != token { return Ok(()); }
        
        let Ok(entries) = std::fs::read_dir(current_dir) else { continue; };

        for entry in entries.filter_map(|e| e.ok()) {
            if state.token.load(std::sync::atomic::Ordering::Relaxed) != token { return Ok(()); }

            let name = entry.file_name().to_string_lossy().into_owned();
            let name_lower = name.to_lowercase();
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

            // Fetch metadata once for efficiency
            let (size, modified, is_hidden) = if let Ok(meta) = entry.metadata() {
                let s = meta.len();
                let m = meta.modified().ok().and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
                let h = name.starts_with('.') || (meta.file_attributes() & 2 != 0);
                (s, m, h)
            } else {
                if filters.min_size.is_some() || filters.max_size.is_some() { continue; }
                (0, 0, false)
            };

            // Filter hidden items 
            if !want_hidden && is_hidden { continue; }

            // Add subdirectories to the back of the queue
            if is_dir { dirs_to_visit.push_back(entry.path()); }

            // Apply all search filters
            if !filters.name_query.is_empty() && !name_lower.contains(&filters.name_query) { continue; }
            if let Some(want_dir) = filters.is_dir { if is_dir != want_dir { continue; } }
            if let Some(ref ext) = filters.ext { if is_dir || !name_lower.ends_with(&format!(".{}", ext)) { continue; } }
            if let Some(min_s) = filters.min_size { if size < min_s { continue; } }
            if let Some(max_s) = filters.max_size { if size > max_s { continue; } }
            if let Some(min_d) = filters.min_date { if modified < min_d { continue; } }
            if let Some(max_d) = filters.max_date { if modified > max_d { continue; } }

            let path_str = entry.path().to_string_lossy().into_owned();
            batch.push(FileItem { name, is_dir, path: path_str, size, modified, is_hidden, snippet: None });

            if batch.len() >= 100 {
                state.buffer.lock().unwrap().push(encode_files(&batch));
                batch.clear();
            }
        }
    }
    
    if !batch.is_empty() && state.token.load(std::sync::atomic::Ordering::Relaxed) == token {
        state.buffer.lock().unwrap().push(encode_files(&batch));
    }
    Ok(())
}

#[tauri::command]
pub async fn search_contents(path: String, query: String, state: tauri::State<'_, SearchState>) -> Result<(), String> {
    let token = state.token.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    state.buffer.lock().unwrap().clear();

    let filters = parse_query(&query);
    let mut batch = Vec::new();
    let want_hidden = filters.is_hidden.unwrap_or(false);

    let mut dirs_to_visit = std::collections::VecDeque::new();
    dirs_to_visit.push_back(std::path::PathBuf::from(&path));

    while let Some(current_dir) = dirs_to_visit.pop_front() {
        if state.token.load(std::sync::atomic::Ordering::Relaxed) != token { return Ok(()); }
        
        let Ok(entries) = std::fs::read_dir(current_dir) else { continue; };

        for entry in entries.filter_map(|e| e.ok()) {
            if state.token.load(std::sync::atomic::Ordering::Relaxed) != token { return Ok(()); }

            let name = entry.file_name().to_string_lossy().into_owned();
            let name_lower = name.to_lowercase();
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

            let meta = entry.metadata().ok();
            let is_hidden = meta.as_ref().map(|m| name.starts_with('.') || (m.file_attributes() & 2 != 0)).unwrap_or(false);
            
            // Queue directories and skip file analysis
            if !want_hidden && is_hidden { continue; }
            if is_dir { 
                dirs_to_visit.push_back(entry.path()); 
                continue; 
            }

            // File Filters
            if let Some(ref ext) = filters.ext { if !name_lower.ends_with(&format!(".{}", ext)) { continue; } }

            let mut size = 0;
            let mut modified = 0;

            if let Some(m) = &meta {
                let attrs = m.file_attributes();
                if (attrs & 0x00400000) != 0 || (attrs & 0x1000) != 0 { continue; } // Skip cloud links
                
                size = m.len();
                if size > 50 * 1024 * 1024 { continue; } // Skip large files > 50MB
                if let Some(min_s) = filters.min_size { if size < min_s { continue; } }
                if let Some(max_s) = filters.max_size { if size > max_s { continue; } }

                modified = m.modified().ok().and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
            } else {
                if filters.min_size.is_some() || filters.max_size.is_some() { continue; }
            }

            if let Some(min_d) = filters.min_date { if modified < min_d { continue; } }
            if let Some(max_d) = filters.max_date { if modified > max_d { continue; } }

            // Read & Map Contents using memmap2 for zero-allocation searching
                let path_str = entry.path().to_string_lossy().into_owned();
                let file = match std::fs::File::open(entry.path()) {
                    Ok(file) => file,
                    Err(_) => continue,
                };

                // Memory-map the file
                let mmap = match unsafe { memmap2::MmapOptions::new().map(&file) } {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                if mmap.is_empty() {
                    continue;
                }

                // skip if the first chunk contains a null byte
                let head_len = mmap.len().min(1024);
                if mmap[..head_len].contains(&0) {
                    continue;
                }

                let query_bytes = filters.name_query.as_bytes();
                let mut found_pos = None;

                if !query_bytes.is_empty() {
                    // Search directly within the memory-mapped virtual bytes
                    if let Some(pos) = mmap.windows(query_bytes.len()).position(|w| w == query_bytes) {
                        found_pos = Some(pos);
                    }
                } else {
                    found_pos = Some(0);
                }

            if let Some(pos) = found_pos {
                let start = pos.saturating_sub(40);
                let end = (pos + filters.name_query.len() + 40).min(mmap.len());
                let raw_snippet = &mmap[start..end];
                let snippet = String::from_utf8_lossy(raw_snippet).replace('\n', " ").trim().to_string();
                
                batch.push(FileItem { 
                    name, is_dir: false, path: path_str, size, modified, is_hidden, 
                    snippet: if query_bytes.is_empty() { None } else { Some(format!("...{}...", snippet)) } 
                });
                
                if batch.len() >= 50 {
                    state.buffer.lock().unwrap().push(encode_files(&batch));
                    batch.clear();
                }
            }
        }
    }

    if !batch.is_empty() && state.token.load(std::sync::atomic::Ordering::Relaxed) == token {
        state.buffer.lock().unwrap().push(encode_files(&batch));
    }
    Ok(())
}

#[tauri::command]
pub fn rebuild_index(state: tauri::State<'_, SearchIndex>) -> Result<(), String> {
    let db = state.0.clone(); 
    
    std::thread::spawn(move || {
        let Ok(write_txn) = db.begin_write() else { return };
        {
            let Ok(mut table) = write_txn.open_table(SEARCH_INDEX) else { return };
            let user_profile = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
            
            for entry in jwalk::WalkDir::new(user_profile)
                .into_iter()
                .filter_map(|e| e.ok()) {
                    let path = entry.path().to_string_lossy().into_owned();
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let _ = table.insert(path.as_str(), name.as_str());
            }
        }
        let _ = write_txn.commit();
    });
    Ok(())
}

#[tauri::command]
pub fn instant_search(query: String, state: tauri::State<'_, SearchIndex>) -> Result<Vec<FileItem>, String> {
    let db = &state.0;
    let read_txn = db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(SEARCH_INDEX).map_err(|e| e.to_string())?;
    
    let mut results = Vec::new();
    let query_lower = query.to_lowercase();

    for res in table.iter().map_err(|e| e.to_string())? {
        let (path_guard, name_guard) = res.map_err(|e| e.to_string())?;
        let path = path_guard.value();
        let name = name_guard.value();
        
        if name.to_lowercase().contains(&query_lower) {
            results.push(FileItem {
                name: name.to_string(),
                path: path.to_string(),
                is_dir: std::path::Path::new(path).is_dir(),
                size: 0, 
                modified: 0,
                is_hidden: false,
                snippet: None,
            });
        }
        if results.len() > 100 { break; } 
    }
    Ok(results)
}