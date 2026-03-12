use crate::{IconCache, ThumbnailCache, THUMBNAILS};
use std::os::windows::fs::MetadataExt;

// serving OS Thumbnails via COM
pub fn get_or_create_thumbnail(decoded_path: &str, db: &redb::Database) -> Result<Vec<u8>, String> {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
    use windows::Win32::UI::Shell::{SHCreateItemFromParsingName, IShellItemImageFactory, SIIGBF_RESIZETOFIT};
    use windows::Win32::Graphics::Gdi::{GetObjectW, GetDIBits, DeleteObject, CreateCompatibleDC, DeleteDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HBITMAP};
    use windows::Win32::Foundation::SIZE;

    let mut hasher = DefaultHasher::new();
    decoded_path.hash(&mut hasher);

    let mut mod_time = 0;
    if let Ok(meta) = std::fs::metadata(&decoded_path) {
        // Skip dehydrated cloud files to prevent forced downloads
        let attrs = meta.file_attributes();
        if (attrs & 0x00400000) != 0 || (attrs & 0x1000) != 0 {
            return Err("Dehydrated cloud file".into());
        }

        if let Ok(modified) = meta.modified() {
            if let Ok(dur) = modified.duration_since(std::time::SystemTime::UNIX_EPOCH) {
                mod_time = dur.as_secs();
            }
        }
    }

    let cache_key = format!("{}_{}", hasher.finish(), mod_time);

    if let Ok(read_txn) = db.begin_read() {
        if let Ok(table) = read_txn.open_table(THUMBNAILS) {
            if let Ok(Some(cached_bytes)) = table.get(cache_key.as_str()) {
                return Ok(cached_bytes.value().to_vec());
            }
        }
    }

    let bytes = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();
        pub struct ComGuard;
        impl Drop for ComGuard { fn drop(&mut self) { unsafe { CoUninitialize(); } } }
        let _guard = ComGuard;
        
        let wide_path: Vec<u16> = std::ffi::OsStr::new(&decoded_path).encode_wide().chain(std::iter::once(0)).collect();

        let item: IShellItemImageFactory = SHCreateItemFromParsingName(PCWSTR(wide_path.as_ptr()), None).map_err(|e| e.to_string())?;
        let size = SIZE { cx: 256, cy: 256 };
        let hbitmap: HBITMAP = item.GetImage(size, SIIGBF_RESIZETOFIT).map_err(|e| e.to_string())?;
        
        pub struct HbitmapGuard(HBITMAP);
        impl Drop for HbitmapGuard { fn drop(&mut self) { unsafe { let _ = DeleteObject(self.0); } } }
        let _bmp_guard = HbitmapGuard(hbitmap);

        let mut bmp: BITMAP = std::mem::zeroed();
        GetObjectW(hbitmap, std::mem::size_of::<BITMAP>() as i32, Some(&mut bmp as *mut _ as *mut _));

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: bmp.bmWidth,
                biHeight: -bmp.bmHeight,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut pixels = vec![0u8; (bmp.bmWidth * bmp.bmHeight * 4) as usize];
        let hdc = CreateCompatibleDC(None);
        GetDIBits(hdc, hbitmap, 0, bmp.bmHeight as u32, Some(pixels.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS);
        let _ = DeleteDC(hdc);

        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
            if chunk[3] == 0 { chunk[3] = 255; } 
        }

        let mut cursor = std::io::Cursor::new(Vec::new());
        if let Some(img) = image::RgbaImage::from_raw(bmp.bmWidth as u32, bmp.bmHeight as u32, pixels) {
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 75);
            encoder.encode_image(&img).map_err(|e| e.to_string())?;
        } else {
            return Err("Failed to convert raw pixels".into());
        }

        let out_bytes = cursor.into_inner();
        
        if let Ok(write_txn) = db.begin_write() {
            if let Ok(mut table) = write_txn.open_table(THUMBNAILS) {
                let _ = table.insert(cache_key.as_str(), out_bytes.as_slice());
            }
            let _ = write_txn.commit();
        }
        
        out_bytes
    };
    
    Ok(bytes)
}

pub fn handle_thumbnail_request(ctx: tauri::UriSchemeContext<'_, tauri::Wry>,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    use tauri::Manager;
    let app_handle = ctx.app_handle();
    let uri = request.uri().to_string();
    let path_str = uri.strip_prefix("thumbnail://localhost/").unwrap_or(&uri);
    let decoded_path = urlencoding::decode(path_str).unwrap_or_default().into_owned();

    let cache_state = app_handle.state::<ThumbnailCache>();
    
    match get_or_create_thumbnail(&decoded_path, &cache_state.0) {
        Ok(bytes) => {
            let res = tauri::http::Response::builder()
                .header("Content-Type", "image/jpeg")
                .body(bytes).unwrap();
            responder.respond(res);
        },
        Err(_) => responder.respond(tauri::http::Response::builder().status(404).body(Vec::new()).unwrap()),
    }
}

#[tauri::command]
pub fn precache_thumbnails(paths: Vec<String>, app_handle: tauri::AppHandle) {
    use tauri::Manager;
    // Limit concurrency to 4 threads 
    let semaphore = std::sync::Arc::new(std::sync::Condvar::new());
    let active_count = std::sync::Arc::new(std::sync::Mutex::new(0));

    for path in paths {
        let app_h = app_handle.clone();
        let sem = semaphore.clone();
        let counter = active_count.clone();

        std::thread::spawn(move || {
            {
                let mut count = counter.lock().unwrap();
                while *count >= 4 {
                    count = sem.wait(count).unwrap();
                }
                *count += 1;
            }

            let cache_state = app_h.state::<ThumbnailCache>();
            let _ = get_or_create_thumbnail(&path, &cache_state.0);

            {
                let mut count = counter.lock().unwrap();
                *count -= 1;
                sem.notify_one();
            }
        });
    }
}

pub fn extract_icon_to_png(path_or_ext: &str, is_dir: bool) -> Result<Vec<u8>, String> {
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_USEFILEATTRIBUTES};
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, DestroyIcon, ICONINFO};
    use windows::Win32::Graphics::Gdi::{GetObjectW, GetDIBits, CreateCompatibleDC, DeleteDC, DeleteObject, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS};
    use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_DIRECTORY};
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;

    unsafe {
        let wide_path: Vec<u16> = std::ffi::OsStr::new(path_or_ext).encode_wide().chain(std::iter::once(0)).collect();
        let mut shfi = SHFILEINFOW::default();
        
        // Determine if this is an absolute path or a generic extension string
        let path_obj = std::path::Path::new(path_or_ext);
        let is_real_path = path_obj.exists() || path_or_ext.starts_with("\\\\") || path_or_ext.ends_with(":\\");
        
        let (flags, attrs) = if is_real_path {
            (SHGFI_ICON | SHGFI_LARGEICON, 0) // Exact file/drive lookup
        } else {
            let attrs = if is_dir { FILE_ATTRIBUTE_DIRECTORY.0 } else { FILE_ATTRIBUTE_NORMAL.0 };
            (SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES, attrs) // Generic extension lookup
        };

        let mut final_wide = wide_path.clone();
        if !is_real_path && !is_dir && !path_or_ext.starts_with('.') && !path_or_ext.is_empty() {
            let dot_ext = format!(".{}", path_or_ext);
            final_wide = std::ffi::OsStr::new(&dot_ext).encode_wide().chain(std::iter::once(0)).collect();
        }

        let res = SHGetFileInfoW(
            PCWSTR(final_wide.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(attrs),
            Some(&mut shfi as *mut _),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            flags,
        );
        
        // Fallback to generic icon if exact lookup failed
        if res == 0 || shfi.hIcon.is_invalid() {
            let fallback_attrs = if is_dir { FILE_ATTRIBUTE_DIRECTORY.0 } else { FILE_ATTRIBUTE_NORMAL.0 };
            let fallback_wide: Vec<u16> = std::ffi::OsStr::new(if is_dir { "folder" } else { ".unknown" }).encode_wide().chain(std::iter::once(0)).collect();
            let fallback_res = SHGetFileInfoW(
                PCWSTR(fallback_wide.as_ptr()),
                windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(fallback_attrs),
                Some(&mut shfi as *mut _),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES,
            );
            if fallback_res == 0 || shfi.hIcon.is_invalid() { return Err("Failed to get icon".into()); }
        }

        pub struct IconGuard(windows::Win32::UI::WindowsAndMessaging::HICON);
        impl Drop for IconGuard {
            fn drop(&mut self) { unsafe { let _ = DestroyIcon(self.0); } }
        }
        let _icon_guard = IconGuard(shfi.hIcon);
        
        let mut icon_info = ICONINFO::default();
        if GetIconInfo(shfi.hIcon, &mut icon_info).is_err() { return Err("Failed to get icon info".into()); }

        pub struct BmpGuard(windows::Win32::Graphics::Gdi::HBITMAP);
        impl Drop for BmpGuard {
            fn drop(&mut self) { unsafe { if !self.0.is_invalid() { let _ = DeleteObject(self.0); } } }
        }
        let _color_guard = BmpGuard(icon_info.hbmColor);
        let _mask_guard = BmpGuard(icon_info.hbmMask);

        let mut bmp: BITMAP = std::mem::zeroed();
        GetObjectW(icon_info.hbmColor, std::mem::size_of::<BITMAP>() as i32, Some(&mut bmp as *mut _ as *mut _));
        
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: bmp.bmWidth,
                biHeight: -bmp.bmHeight,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        
        let mut pixels = vec![0u8; (bmp.bmWidth * bmp.bmHeight * 4) as usize];
        let hdc = CreateCompatibleDC(None);
        GetDIBits(hdc, icon_info.hbmColor, 0, bmp.bmHeight as u32, Some(pixels.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS);
        let _ = DeleteDC(hdc);
        
        for chunk in pixels.chunks_exact_mut(4) { chunk.swap(0, 2); }

        let img = image::RgbaImage::from_raw(bmp.bmWidth as u32, bmp.bmHeight as u32, pixels).ok_or("Failed to create image")?;
        let mut cursor = std::io::Cursor::new(Vec::new());
        img.write_to(&mut cursor, image::ImageFormat::Png).map_err(|e| e.to_string())?;

        Ok(cursor.into_inner())
    }
}

pub fn handle_icon_request(ctx: tauri::UriSchemeContext<'_, tauri::Wry>,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    use tauri::Manager;
    let app_handle = ctx.app_handle();
    let uri = request.uri().to_string();
    let path_and_query = uri.strip_prefix("icon://localhost/").unwrap_or(&uri);
    
    let mut parts = path_and_query.split('?');
    let path_str = parts.next().unwrap_or("").trim_start_matches('/');
    let is_dir = parts.next().unwrap_or("").contains("is_dir=true");
    let decoded_path = urlencoding::decode(path_str).unwrap_or_default().into_owned();

    let cache_key = format!("{}_{}", decoded_path, is_dir);
    let state = app_handle.state::<IconCache>();
    
    let mut cache = state.0.lock().unwrap();
    if let Some(cached) = cache.get(&cache_key) {
        let res = tauri::http::Response::builder().header("Content-Type", "image/png").body(cached.clone()).unwrap();
        return responder.respond(res);
    }

    if let Ok(bytes) = extract_icon_to_png(&decoded_path, is_dir) {
        cache.insert(cache_key, bytes.clone());
        let res = tauri::http::Response::builder().header("Content-Type", "image/png").body(bytes).unwrap();
        responder.respond(res);
    } else {
        responder.respond(tauri::http::Response::builder().status(404).body(Vec::new()).unwrap());
    }
}