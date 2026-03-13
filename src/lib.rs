#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{atomic::{AtomicBool, Ordering}, mpsc, Arc, RwLock};
use std::thread;
use std::time::Duration;

use jwalk::WalkDir;
use rayon::prelude::*;
use sysinfo::Disks;
use tauri::{generate_context, generate_handler, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

use windows::{
    core::{HSTRING, PCWSTR},
    Win32::Foundation::HWND,
    Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, 
        SelectObject, BITMAPINFO, BITMAPINFOHEADER, 
        BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
    },
    Win32::UI::Shell::{
        ShellExecuteW, SHGetFileInfoW, SHFILEINFOW, 
        SHGFI_ICON, SHGFI_SYSICONINDEX,
    },
    Win32::UI::WindowsAndMessaging::{
        DestroyIcon, DrawIconEx, SW_SHOWNORMAL, DI_NORMAL,
    },
    Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileInfo {
    pub name: String,
    #[serde(skip)]
    pub name_lowercase: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    #[serde(default)]
    pub open_count: u32,
    #[serde(skip, default)]
    pub icon_index: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchResponse {
    pub results: Vec<FileInfo>,
    pub search_id: u32,
}

pub struct AppIndex {
    pub files: Arc<RwLock<Vec<FileInfo>>>,
    pub path_map: Arc<RwLock<HashMap<String, usize>>>,
    pub save_tx: mpsc::Sender<()>,
    pub cancel_index: Arc<AtomicBool>,
    pub icon_cache: Arc<RwLock<HashMap<String, String>>>,
}

const IGNORE_DIRS: &[&str] = &[
    "node_modules", ".git", "AppData", "$Recycle.Bin",
    "Windows", "System32", "ProgramData", "Recovery",
];

#[derive(Clone, Debug)]
struct ScoredFile {
    file: FileInfo,
    score: i32,
}

// Fuzzy matching algorithm that returns a score
// Higher score = better match
fn fuzzy_match(text: &str, pattern: &str) -> Option<i32> {
    let text = text.to_lowercase();
    let pattern = pattern.to_lowercase();
    
    let mut pattern_idx = 0;
    let mut score = 0;
    let mut consecutive = 0;
    let mut last_match_idx = 0;
    
    let text_chars: Vec<char> = text.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();
    
    if pattern_chars.is_empty() {
        return Some(0);
    }
    
    for (i, &ch) in text_chars.iter().enumerate() {
        if pattern_idx < pattern_chars.len() && ch == pattern_chars[pattern_idx] {
            // Bonus for consecutive matches
            if i == last_match_idx + 1 {
                consecutive += 1;
                score += 10 + consecutive * 5;
            } else {
                consecutive = 0;
                score += 10;
            }
            
            // Bonus for match at start
            if pattern_idx == 0 && i == 0 {
                score += 15;
            }
            
            // Bonus for match after separator
            if i > 0 && (text_chars[i-1] == '/' || text_chars[i-1] == '\\' || text_chars[i-1] == ' ' || text_chars[i-1] == '_' || text_chars[i-1] == '-') {
                score += 12;
            }
            
            // Penalty for distance from last match
            if pattern_idx > 0 {
                let gap = i - last_match_idx - 1;
                score -= gap as i32;
            }
            
            last_match_idx = i;
            pattern_idx += 1;
            
            if pattern_idx == pattern_chars.len() {
                // All pattern chars matched
                // Bonus for shorter strings (better match)
                score += (100 - text_chars.len() as i32).max(0);
                return Some(score);
            }
        }
    }
    
    if pattern_idx == pattern_chars.len() {
        Some(score)
    } else {
        None
    }
}

#[allow(dead_code)]
fn get_file_icon_index(path: &str) -> i32 {
    unsafe {
        let mut info = SHFILEINFOW::default();
        let path_wide = HSTRING::from(path);
        let result = SHGetFileInfoW(
            &path_wide,
            FILE_ATTRIBUTE_NORMAL,
            Some(&mut info as *mut _),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX,
        );
        if result != 0 {
            info.iIcon as i32
        } else {
            -1
        }
    }
}

// Extract icon and convert to base64 PNG data URI
fn extract_file_icon_base64(path: &str) -> Result<String, String> {
    unsafe {
        let mut info = SHFILEINFOW::default();
        let path_wide = HSTRING::from(path);
        
        // Get icon from actual file (not by extension)
        // Don't use SHGFI_USEFILEATTRIBUTES to read real file icon
        let result = SHGetFileInfoW(
            &path_wide,
            FILE_ATTRIBUTE_NORMAL,
            Some(&mut info as *mut _),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON, // Only SHGFI_ICON to get real file icon
        );
        
        if result == 0 || info.hIcon.is_invalid() {
            return Err("Failed to get icon".to_string());
        }
        
        let hicon = info.hIcon;
        
        // Use 32x32 for better quality, will scale down in CSS
        let size = 32i32;
        
        // Create memory DC
        let screen_dc = windows::Win32::Graphics::Gdi::GetDC(None);
        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        
        if mem_dc.is_invalid() {
            let _ = DestroyIcon(hicon);
            windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
            return Err("Failed to create DC".to_string());
        }
        
        // Create 32-bit bitmap
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                biHeight: -size, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                ..Default::default()
            },
            ..Default::default()
        };
        
        let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbitmap = match windows::Win32::Graphics::Gdi::CreateDIBSection(
            Some(mem_dc),
            &bmi,
            DIB_RGB_COLORS,
            &mut bits_ptr,
            None,
            0,
        ) {
            Ok(bmp) => bmp,
            Err(_) => {
                let _ = DeleteDC(mem_dc);
                windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
                let _ = DestroyIcon(hicon);
                return Err("Failed to create bitmap".to_string());
            }
        };
        
        if hbitmap.is_invalid() || bits_ptr.is_null() {
            let _ = DeleteDC(mem_dc);
            windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
            let _ = DestroyIcon(hicon);
            return Err("Failed to create bitmap".to_string());
        }
        
        // Select bitmap into DC
        let old_bitmap = SelectObject(mem_dc, HGDIOBJ(hbitmap.0));
        
        // Fill with transparent/white background
        let brush = windows::Win32::Graphics::Gdi::CreateSolidBrush(
            windows::Win32::Foundation::COLORREF(0x00FFFFFF)
        );
        let rect = windows::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: size,
            bottom: size,
        };
        windows::Win32::Graphics::Gdi::FillRect(mem_dc, &rect, brush);
        let _ = DeleteObject(HGDIOBJ(brush.0));
        
        // Draw icon on bitmap
        DrawIconEx(
            mem_dc,
            0, 0,
            hicon,
            size, size,
            0,
            None,
            DI_NORMAL,
        ).ok();
        
        // Copy bitmap data
        let buffer_size = (size * size * 4) as usize;
        let mut bits: Vec<u8> = vec![0; buffer_size];
        std::ptr::copy_nonoverlapping(
            bits_ptr as *const u8,
            bits.as_mut_ptr(),
            buffer_size,
        );
        
        // Cleanup
        SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(HGDIOBJ(hbitmap.0));
        let _ = DeleteDC(mem_dc);
        windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
        let _ = DestroyIcon(hicon);
        
        // Convert BGRA to RGBA
        for i in (0..bits.len()).step_by(4) {
            bits.swap(i, i + 2); // B <-> R
        }
        
        // Encode to base64
        use base64::Engine;
        let base64_str = base64::engine::general_purpose::STANDARD.encode(&bits);
        Ok(format!("data:image/rgba;base64,{},32", base64_str)) // Include size
    }
}

fn get_cache_path() -> PathBuf {
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        let cache_path = exe_path.join("index_cache.dat");
        let test_path = exe_path.join(".write_test.tmp");
        if let Ok(_) = std::fs::write(&test_path, b"") {
            let _ = std::fs::remove_file(&test_path);
            return cache_path;
        }
    }
    std::env::temp_dir().join("wsearch_index_cache.dat")
}

#[tauri::command]
async fn open_path(path: String) -> Result<(), String> {
    let path_obj = Path::new(&path);

    let abs_path_buf = if path_obj.is_relative() {
        let mut exe_dir = std::env::current_exe().map_err(|e| e.to_string())?;
        exe_dir.pop();
        exe_dir.join(path_obj)
    } else {
        path_obj.to_path_buf()
    };

    let canonical_path = std::fs::canonicalize(&abs_path_buf).unwrap_or(abs_path_buf);
    let mut abs_path_str = canonical_path.to_string_lossy().to_string();
    
    if abs_path_str.starts_with("\\\\?\\") {
        abs_path_str = abs_path_str[4..].to_string();
    }
    
    let clean_path = Path::new(&abs_path_str);

    let parent_dir = clean_path.parent()
        .map(|p| p.to_string_lossy().to_string())
        .filter(|s| !s.is_empty());

    let path_wide = HSTRING::from(clean_path.to_string_lossy().as_ref());
    let operation = HSTRING::from("open");
    
    let working_dir_hstring = parent_dir.as_ref().map(|s| HSTRING::from(s.as_str()));
    let working_dir_pcwstr = working_dir_hstring.as_ref()
        .map(|h| PCWSTR(h.as_ptr()))
        .unwrap_or(PCWSTR::null());

    unsafe {
        let result = ShellExecuteW(
            Some(HWND::default()),
            &operation,
            &path_wide,
            PCWSTR::null(),
            working_dir_pcwstr,
            SW_SHOWNORMAL,
        );
        
        if result.0 as i32 <= 32 {
            let mut cmd = std::process::Command::new("cmd");
            cmd.arg("/c")
               .arg("start")
               .arg("")
               .arg(&abs_path_str);
               
            if let Some(dir) = parent_dir {
                cmd.current_dir(dir);
            }
            
            return cmd.spawn()
                .map(|_| ())
                .map_err(|e| format!(
                    "{} {}", 
                    result.0 as i32, e
                ));
        }
    }
    Ok(())
}

#[tauri::command]
async fn show_in_folder(path: String) -> Result<(), String> {
    let mut clean_path = path.clone();
    if clean_path.starts_with("\\\\?\\") {
        clean_path = clean_path[4..].to_string();
    }
    let path_wide = HSTRING::from(format!("/select,\"{}\"", clean_path));
    let operation = HSTRING::from("open");
    let explorer = HSTRING::from("explorer.exe");
    unsafe {
        let result = ShellExecuteW(
            Some(HWND::default()),
            &operation,
            &explorer,
            &path_wide,
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
        if result.0 as i32 <= 32 {
            return Err(format!("{}", result.0 as i32));
        }
    }
    Ok(())
}

#[tauri::command]
async fn search_files(query: String, search_id: u32, use_fuzzy: bool, state: State<'_, AppIndex>) -> Result<SearchResponse, String> {
    if query.is_empty() {
        return Ok(SearchResponse { results: vec![], search_id });
    }
    let q = query.to_lowercase();
    const LIMIT: usize = 100;
    const MAX_SUBSTRING_SCAN: usize = 10_000;

    let files_read = state.files.read().map_err(|e| e.to_string())?;

    if files_read.is_empty() {
        return Ok(SearchResponse { results: vec![], search_id });
    }

    let mut results: Vec<FileInfo> = Vec::with_capacity(LIMIT);

    let start_idx = files_read
        .binary_search_by(|file| file.name_lowercase.cmp(&q))
        .unwrap_or_else(|pos| pos);

    for file in &files_read[start_idx..] {
        if file.name_lowercase.starts_with(&q) {
            results.push(file.clone());
            if results.len() >= LIMIT { break; }
        } else {
            break;
        }
    }

    if results.len() < LIMIT {
        let end_of_prefix = start_idx + results.len();
        let remaining_needed = LIMIT - results.len();

        let chunk_size = 5_000.min(MAX_SUBSTRING_SCAN);
        let scan_range = &files_read[end_of_prefix..].iter()
            .take(MAX_SUBSTRING_SCAN)
            .collect::<Vec<_>>();
        
        let mut substring_matches: Vec<FileInfo> = scan_range
            .par_chunks(chunk_size)
            .flat_map(|chunk| {
                chunk.iter()
                    .filter(|file| file.name_lowercase.contains(&q))
                    .map(|&f| f.clone())
                    .collect::<Vec<_>>()
            })
            .collect();
        
        substring_matches.truncate(remaining_needed);
        results.append(&mut substring_matches);
        
        if results.len() < LIMIT && start_idx > 0 {
            let remaining_needed = LIMIT - results.len();
            let prefix_chunk: Vec<&FileInfo> = files_read[..start_idx].iter().collect();
            
            let mut prefix_matches: Vec<FileInfo> = prefix_chunk
                .par_chunks(chunk_size)
                .flat_map(|chunk| {
                    chunk.iter()
                        .filter(|file| file.name_lowercase.contains(&q))
                        .map(|&f| f.clone())
                        .collect::<Vec<_>>()
                })
                .collect();
            
            prefix_matches.truncate(remaining_needed);
            results.append(&mut prefix_matches);
        }
        
        // Apply fuzzy matching if still not enough results and use_fuzzy is enabled
        if use_fuzzy && results.len() < LIMIT && q.len() >= 2 {
            let remaining_needed = LIMIT - results.len();
            let existing_paths: std::collections::HashSet<String> = 
                results.iter().map(|f| f.path.clone()).collect();
            
            let fuzzy_chunk_size = 10_000;
            let mut scored_matches: Vec<ScoredFile> = files_read
                .par_chunks(fuzzy_chunk_size)
                .flat_map(|chunk| {
                    chunk.iter()
                        .filter(|file| !existing_paths.contains(&file.path))
                        .filter_map(|file| {
                            fuzzy_match(&file.name_lowercase, &q).map(|score| ScoredFile {
                                file: file.clone(),
                                score,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
            
            // Sort by fuzzy score descending
            scored_matches.par_sort_unstable_by(|a, b| b.score.cmp(&a.score));
            scored_matches.truncate(remaining_needed);
            
            results.extend(scored_matches.into_iter().map(|sf| sf.file));
        }
    }

    if results.len() > 50 {
        results.par_sort_unstable_by(|a, b| b.open_count.cmp(&a.open_count));
    } else {
        results.sort_unstable_by(|a, b| b.open_count.cmp(&a.open_count));
    }
    results.truncate(LIMIT);

    Ok(SearchResponse { results, search_id })
}

#[tauri::command]
async fn record_open(path: String, state: State<'_, AppIndex>) -> Result<(), String> {
    let mut found = false;
    if let Ok(map_read) = state.path_map.read() {
        if let Some(&idx) = map_read.get(&path) {
            drop(map_read);
            if let Ok(mut files_w) = state.files.write() {
                if idx < files_w.len() && files_w[idx].path == path {
                    files_w[idx].open_count = files_w[idx].open_count.saturating_add(1);
                    found = true;
                }
            }
        }
    }
    if found { let _ = state.save_tx.send(()); }
    Ok(())
}

#[tauri::command]
async fn get_index_status(state: State<'_, AppIndex>) -> Result<usize, String> {
    let files = state.files.read().map_err(|e| e.to_string())?;
    Ok(files.len())
}

#[tauri::command]
async fn cancel_indexing(state: State<'_, AppIndex>) -> Result<(), String> {
    state.cancel_index.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn get_file_icon(path: String, state: State<'_, AppIndex>) -> Result<String, String> {
    // Get extension for caching
    let extension = Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("_folder_")
        .to_lowercase();
    
    // Files with unique icons per file (not by extension)
    const UNIQUE_ICON_EXTS: &[&str] = &["exe", "lnk", "dll", "ico", "scr", "cpl", "msc"];
    
    // Determine cache key: use full path for unique icons, extension for others
    let cache_key = if UNIQUE_ICON_EXTS.contains(&extension.as_str()) {
        format!("file:{}", path)
    } else {
        format!("ext:{}", extension)
    };
    
    // Check cache first
    if let Ok(cache) = state.icon_cache.read() {
        if let Some(cached_icon) = cache.get(&cache_key) {
            return Ok(cached_icon.clone());
        }
    }
    
    // Extract icon
    match extract_file_icon_base64(&path) {
        Ok(icon_data) => {
            // Cache it
            if let Ok(mut cache) = state.icon_cache.write() {
                cache.insert(cache_key, icon_data.clone());
            }
            Ok(icon_data)
        }
        Err(e) => Err(e),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let files = Arc::new(RwLock::new(Vec::with_capacity(500_000)));
    let path_map = Arc::new(RwLock::new(HashMap::with_capacity(500_000)));
    let icon_cache = Arc::new(RwLock::new(HashMap::new()));
    let (save_tx, save_rx) = mpsc::channel::<()>();
    let cancel_index = Arc::new(AtomicBool::new(false));

    let files_saver = files.clone();
    let files_setup = files.clone();
    let path_map_setup = path_map.clone();
    let save_tx_setup = save_tx.clone();
    let cancel_index_setup = cancel_index.clone();
    let cache_path = get_cache_path();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            let main_window = app.get_webview_window("main").unwrap();
            let cache_p = cache_path.clone();

            let s_cache = cache_p.clone();
            let s_files = files_saver.clone();
            thread::spawn(move || {
                loop {
                    if save_rx.recv().is_err() { break; }
                    thread::sleep(Duration::from_secs(2));
                    while save_rx.try_recv().is_ok() {}
                    if let Ok(snapshot) = s_files.read() {
                        if let Ok(encoded) = bincode::serialize(&*snapshot) {
                            let tmp = s_cache.with_extension("tmp");
                            // Compress with gzip
                            if let Ok(file) = File::create(&tmp) {
                                let mut encoder = GzEncoder::new(file, Compression::default());
                                if encoder.write_all(&encoded).is_ok() {
                                    if encoder.finish().is_ok() {
                                        let _ = std::fs::rename(&tmp, &s_cache);
                                    }
                                }
                            }
                        }
                    }
                }
            });

            let i_cache = cache_p.clone();
            let i_files = files_setup.clone();
            let i_map = path_map_setup.clone();
            let i_tx = save_tx_setup.clone();
            let i_cancel = cancel_index_setup.clone();

            thread::spawn(move || {
                if let Ok(f) = File::open(&i_cache) {
                    // Decompress with gzip
                    let mut decoder = GzDecoder::new(f);
                    let mut buf = Vec::new();
                    if decoder.read_to_end(&mut buf).is_ok() {
                        if let Ok(mut cached) = bincode::deserialize::<Vec<FileInfo>>(&buf) {
                            // Rebuild name_lowercase from name
                            for file in &mut cached {
                                file.name_lowercase = file.name.to_lowercase();
                            }
                            cached.sort_unstable_by(|a, b| a.name_lowercase.cmp(&b.name_lowercase));
                            if let Ok(mut files_w) = i_files.write() {
                                *files_w = cached;
                                if let Ok(mut map_w) = i_map.write() {
                                    map_w.clear();
                                    for (i, fi) in files_w.iter().enumerate() {
                                        map_w.insert(fi.path.clone(), i);
                                    }
                                }
                            }
                        }
                    }
                }

                let mut disks = Disks::new();
                disks.refresh(true);

                let existing_paths: std::collections::HashSet<String> = {
                    if let Ok(map_r) = i_map.read() {
                        map_r.keys().cloned().collect()
                    } else {
                        std::collections::HashSet::new()
                    }
                };

                let mut all_new_files: Vec<FileInfo> = Vec::new();

                for disk in &disks {
                    if i_cancel.load(Ordering::SeqCst) { break; }
                    
                    let walker = WalkDir::new(disk.mount_point()).skip_hidden(true).process_read_dir(|_, _, _, children| {
                        children.retain(|r| {
                            r.as_ref().map(|entry| {
                                let name = entry.file_name().to_string_lossy();
                                !IGNORE_DIRS.iter().any(|&d| name.eq_ignore_ascii_case(d))
                            }).unwrap_or(false)
                        });
                    });

                    let mut batch: Vec<FileInfo> = Vec::with_capacity(16_384);

                    for entry in walker.into_iter().filter_map(|e| e.ok()) {
                        if i_cancel.load(Ordering::SeqCst) { break; }

                        let path_str = entry.path().to_string_lossy().to_string();
                        
                        if existing_paths.contains(&path_str) {
                            continue;
                        }

                        let name = entry.file_name().to_string_lossy().to_string();
                        let is_dir = entry.file_type().is_dir();
                        let size = if !is_dir {
                            entry.metadata().map(|m| m.len()).unwrap_or(0)
                        } else {
                            0
                        };
                        
                        batch.push(FileInfo {
                            name_lowercase: name.to_lowercase(),
                            name,
                            path: path_str.clone(),
                            size,
                            is_dir,
                            open_count: 0,
                            icon_index: -1,
                        });

                        if batch.len() >= 16_384 {
                            all_new_files.append(&mut batch);
                            batch = Vec::with_capacity(16_384);
                        }
                    }

                    if !batch.is_empty() {
                        all_new_files.append(&mut batch);
                    }

                    if i_cancel.load(Ordering::SeqCst) { break; }
                }

                if !all_new_files.is_empty() && !i_cancel.load(Ordering::SeqCst) {
                    if let Ok(mut files_w) = i_files.write() {
                        files_w.extend(all_new_files);
                        files_w.par_sort_unstable_by(|a, b| a.name_lowercase.cmp(&b.name_lowercase));

                        if let Ok(mut map_w) = i_map.write() {
                            map_w.clear();
                            for (idx, fi) in files_w.iter().enumerate() {
                                map_w.insert(fi.path.clone(), idx);
                            }
                        }
                    }
                    
                    let _ = i_tx.send(());
                }
            });

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
        .manage(AppIndex { files, path_map, save_tx, cancel_index, icon_cache })
        .invoke_handler(generate_handler![
            search_files,
            get_index_status,
            record_open,
            open_path,
            show_in_folder,
            cancel_indexing,
            get_file_icon
        ])
        .run(generate_context!())
        .expect("error while running tauri application");
}