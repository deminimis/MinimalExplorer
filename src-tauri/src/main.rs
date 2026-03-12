// Prevents additional console window on Windows in release, DO NOT REMOVE
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod commands;
pub mod operations;
pub mod search;
pub mod media;

use std::collections::HashMap;
use std::sync::Mutex;
use redb::{Database, TableDefinition};
use tauri::Manager;

const THUMBNAILS: TableDefinition<&str, &[u8]> = TableDefinition::new("thumbnails");
const SEARCH_INDEX: TableDefinition<&str, &str> = TableDefinition::new("search_index");

pub struct ThumbnailCache(pub std::sync::Arc<Database>);
pub struct SearchIndex(pub std::sync::Arc<Database>);
pub struct WatcherState(pub Mutex<HashMap<String, notify::RecommendedWatcher>>);

pub struct PtyInstance {
    pub master: Box<dyn portable_pty::MasterPty + Send>,
    pub writer: Box<dyn std::io::Write + Send>,
}
pub struct PtyState(pub Mutex<HashMap<String, PtyInstance>>);
pub struct IconCache(pub Mutex<HashMap<String, Vec<u8>>>);

pub struct SearchState {
    pub token: std::sync::atomic::AtomicUsize,
    pub buffer: Mutex<Vec<Vec<u8>>>,
}

pub struct PaneLoadState {
    pub token: std::sync::atomic::AtomicUsize,
    pub buffer: Mutex<Vec<Vec<u8>>>,
}

pub struct LoadState(pub std::collections::HashMap<String, PaneLoadState>);

impl LoadState {
    pub fn get_pane(&self, pane_id: &str) -> &PaneLoadState {
        self.0.get(pane_id).unwrap()
    }
}

fn main() {
    // --- REDB CACHING & EVICTION MECHANISM ---
    let db_path = std::env::temp_dir().join("minimal_explorer_cache.redb");
    if let Ok(metadata) = std::fs::metadata(&db_path) {
        if metadata.len() > 500 * 1024 * 1024 {
            let _ = std::fs::remove_file(&db_path);
        }
    }

    let db = std::sync::Arc::new(Database::create(&db_path).unwrap());
    
    let db_clone = db.clone();
    let write_txn = db_clone.begin_write().unwrap();
    let _ = write_txn.open_table(THUMBNAILS).unwrap();
    let _ = write_txn.open_table(SEARCH_INDEX).unwrap();
    write_txn.commit().unwrap();

    let search_db = db.clone();

    let mut load_states = std::collections::HashMap::new();
    load_states.insert("primary".to_string(), PaneLoadState {
        token: std::sync::atomic::AtomicUsize::new(0),
        buffer: Mutex::new(Vec::new()),
    });
    load_states.insert("secondary".to_string(), PaneLoadState {
        token: std::sync::atomic::AtomicUsize::new(0),
        buffer: Mutex::new(Vec::new()),
    });

    let builder = tauri::Builder::default()
        .manage(ThumbnailCache(db))
        .manage(SearchIndex(search_db))
        .manage(WatcherState(Mutex::new(HashMap::new())))
        .manage(PtyState(Mutex::new(HashMap::new())))
        .manage(IconCache(Mutex::new(HashMap::new())))
        .manage(SearchState {
            token: std::sync::atomic::AtomicUsize::new(0),
            buffer: Mutex::new(Vec::new()),
        })
        .manage(LoadState(load_states))
        .setup(|app| {
            app.get_webview_window("main").unwrap().show().unwrap();
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("thumbnail", media::handle_thumbnail_request)
        .register_asynchronous_uri_scheme_protocol("icon", media::handle_icon_request)
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    commands::register_handlers(builder)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}