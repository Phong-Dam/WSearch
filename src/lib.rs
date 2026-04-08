//! WSearch - Fast file search application
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cache;
mod icon;
mod search;
mod shell;
mod types;
mod watcher;

use std::collections::HashMap;
use std::sync::{atomic::{AtomicBool, Ordering}, mpsc, Arc, RwLock, Mutex};
use std::thread;
use sysinfo::Disks;
use tauri::{generate_context, generate_handler, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use types::{AppIndex, BenchmarkMetrics};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let files = Arc::new(RwLock::new(Vec::with_capacity(500_000)));
    let path_map = Arc::new(RwLock::new(HashMap::with_capacity(500_000)));
    let icon_cache = Arc::new(RwLock::new(HashMap::new()));
    let metrics = Arc::new(Mutex::new(BenchmarkMetrics::default()));
    let (save_tx, save_rx) = mpsc::channel::<()>();
    let cancel_index = Arc::new(AtomicBool::new(false));
    let is_loading_cache = Arc::new(AtomicBool::new(true));
    let is_indexing = Arc::new(AtomicBool::new(true));
    let cache_path = cache::get_cache_path();

    let files_setup = files.clone();
    let path_map_setup = path_map.clone();
    let save_tx_setup = save_tx.clone();
    let save_rx_setup = save_rx;
    let cancel_setup = cancel_index.clone();
    let indexing_setup = is_indexing.clone();
    let is_loading_setup = is_loading_cache.clone();
    let metrics_setup = metrics.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            let main_window = app.get_webview_window("main").unwrap();
            let cache_p = cache_path.clone();

            // Start auto-save thread
            cache::start_cache_saver(files_setup.clone(), cache_p.clone(), save_rx_setup);
            
            // Start background cleanup thread
            cache::start_cleanup_thread(files_setup.clone(), path_map_setup.clone(), cancel_setup.clone());
            
            // Start file watcher for real-time updates
            let disk_paths: Vec<String> = Disks::new()
                .iter()
                .map(|d| d.mount_point().to_string_lossy().to_string())
                .collect();
            
            let (_watcher_tx, _watcher_rx) = mpsc::channel::<()>();
            
            // Watcher thread
            let w_files = files_setup.clone();
            let w_map = path_map_setup.clone();
            let w_save_tx = save_tx_setup.clone();
            thread::spawn(move || {
                watcher::start_watcher(w_files, w_map, disk_paths, w_save_tx);
            });

            // Spawn indexing thread (load cache + scan new files)
            let i_files = files_setup.clone();
            let i_map = path_map_setup.clone();
            let i_tx = save_tx_setup.clone();
            let i_cancel = cancel_setup.clone();
            let i_loading = is_loading_setup.clone();
            let i_indexing = indexing_setup.clone();
            
            thread::spawn(move || {
                // Load cached index
                let _ = cache::load_cache(&cache_p, i_files.clone(), i_map.clone(), Some(metrics_setup));
                // Mark cache loaded, now start scanning
                i_loading.store(false, Ordering::SeqCst);
                // Scan and index new files
                cache::scan_and_index(i_files, i_map, i_tx, i_cancel, i_indexing);
            });

            // Global shortcuts
            let mw1 = main_window.clone();
            match app.global_shortcut().on_shortcut("alt+space", move |_app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    let is_visible = mw1.is_visible().unwrap_or(false);
                    if is_visible {
                        let _ = mw1.hide();
                    } else {
                        let _ = mw1.show();
                        let _ = mw1.set_focus();
                    }
                }
            }) {
                Ok(_) => {},
                Err(e) => eprintln!("Failed to register Alt+Space: {}", e),
            }

            let mw2 = main_window.clone();
            match app.global_shortcut().on_shortcut("ctrl+space", move |_app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    let is_visible = mw2.is_visible().unwrap_or(false);
                    if is_visible {
                        let _ = mw2.hide();
                    } else {
                        let _ = mw2.show();
                        let _ = mw2.set_focus();
                    }
                }
            }) {
                Ok(_) => {},
                Err(e) => eprintln!("Failed to register Ctrl+Space: {}", e),
            }

            Ok(())
        })
        .manage(AppIndex { files, path_map, save_tx, cancel_index, is_loading_cache: is_loading_cache.clone(), is_indexing, icon_cache, metrics })
        .invoke_handler(generate_handler![
            search::search_files,
            search::get_index_status,
            search::record_open,
            search::cancel_indexing,
            search::get_benchmark_metrics,
            shell::open_path,
            shell::show_in_folder,
            icon::get_file_icon
        ])
        .run(generate_context!())
        .expect("error while running tauri application");
}
