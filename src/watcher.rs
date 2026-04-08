//! File system watcher for real-time updates

use crate::types::{FileInfo, IGNORE_DIRS};
use jwalk::WalkDir;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

/// Start file watcher thread - monitors disk changes in real-time
pub fn start_watcher(
    files: Arc<RwLock<Vec<FileInfo>>>,
    path_map: Arc<RwLock<std::collections::HashMap<String, usize>>>,
    disk_paths: Vec<String>,
    save_tx: Sender<()>,
) {
    thread::spawn(move || {
        // Use channel-based watcher
        let (tx, rx) = std::sync::mpsc::channel();
        
        let mut watcher = match RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(2)),
        ) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to create watcher: {}", e);
                return;
            }
        };
        
        // Watch all disk mount points
        for disk_path in &disk_paths {
            let path = Path::new(disk_path);
            if path.exists() {
                if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                    eprintln!("Failed to watch {}: {}", disk_path, e);
                }
            }
        }
        
        // Collect recent changes for batching (debounce rapid events)
        let mut pending_changes: Vec<(String, bool)> = Vec::new(); // (path, is_removal)
        let mut last_process_time = Instant::now();
        let debounce_duration = Duration::from_millis(500);
        
        // Process events
        let ignored_names: HashSet<&str> = IGNORE_DIRS.iter().cloned().collect();
        
        loop {
            // Check for pending changes and process if debounce time passed
            if !pending_changes.is_empty() && last_process_time.elapsed() >= debounce_duration {
                let changes: Vec<(String, bool)> = std::mem::take(&mut pending_changes);
                
                // Group by removal vs add/update
                let removals: Vec<String> = changes.iter()
                    .filter(|(_, is_rem)| *is_rem)
                    .map(|(p, _)| p.clone())
                    .collect();
                let additions: Vec<String> = changes.iter()
                    .filter(|(_, is_rem)| !*is_rem)
                    .map(|(p, _)| p.clone())
                    .collect();
                
                if !removals.is_empty() {
                    handle_file_changes(&removals, true, &files, &path_map);
                    let _ = save_tx.send(());
                }
                if !additions.is_empty() {
                    handle_file_changes(&additions, false, &files, &path_map);
                    let _ = save_tx.send(());
                }
            }
            
            // Collect events for a short period
            let collect_until = Instant::now() + Duration::from_millis(100);
            while Instant::now() < collect_until {
                match rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(event) => {
                        let is_removal = matches!(event.kind, notify::EventKind::Remove(_));
                        
                        // Only process file changes
                        if matches!(
                            event.kind,
                            notify::EventKind::Create(_) | notify::EventKind::Modify(_) | notify::EventKind::Remove(_)
                        ) {
                            for p in &event.paths {
                                let path_str = p.to_string_lossy().to_string();
                                
                                // Skip ignored directories - check path components
                                let path_lower = path_str.to_lowercase();
                                let should_skip = ignored_names.iter().any(|&ign| {
                                    path_lower.contains(&format!("\\{}", ign))
                                });
                                
                                if !should_skip {
                                    pending_changes.push((path_str, is_removal));
                                }
                            }
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // Continue collecting
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        // Process remaining changes before exiting
                        if !pending_changes.is_empty() {
                            let changes: Vec<(String, bool)> = std::mem::take(&mut pending_changes);
                            let removals: Vec<String> = changes.iter()
                                .filter(|(_, is_rem)| *is_rem)
                                .map(|(p, _)| p.clone())
                                .collect();
                            let additions: Vec<String> = changes.iter()
                                .filter(|(_, is_rem)| !*is_rem)
                                .map(|(p, _)| p.clone())
                                .collect();
                            
                            if !removals.is_empty() {
                                handle_file_changes(&removals, true, &files, &path_map);
                            }
                            if !additions.is_empty() {
                                handle_file_changes(&additions, false, &files, &path_map);
                            }
                        }
                        return;
                    }
                }
            }
            
            if !pending_changes.is_empty() {
                last_process_time = Instant::now();
            }
        }
    });
}

/// Scan a directory recursively for files (not dirs)
fn scan_directory(path: &Path) -> Vec<FileInfo> {
    let mut files = Vec::new();
    
    let walker = WalkDir::new(path)
        .skip_hidden(true)
        .process_read_dir(|_, _, _, children| {
            children.retain(|r| {
                r.as_ref().map(|entry| {
                    let name = entry.file_name().to_string_lossy();
                    !IGNORE_DIRS.iter().any(|&d| name.eq_ignore_ascii_case(d))
                }).unwrap_or(false)
            });
        });
    
    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let name = entry.file_name().to_string_lossy().to_string();
            let name_lower = name.to_lowercase();
            
            // Skip ignored files
            if IGNORE_DIRS.iter().any(|&ign| name_lower.eq_ignore_ascii_case(ign)) {
                continue;
            }
            
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let path_str = entry.path().to_string_lossy().to_string();
            
            files.push(FileInfo {
                name,
                name_lowercase: name_lower,
                path: path_str,
                size,
                is_dir: false,
                open_count: 0,
                icon_index: -1,
            });
        }
    }
    
    files
}

/// Handle file changes - add, update, or remove entries
fn handle_file_changes(
    paths: &[String],
    is_removal: bool,
    files: &Arc<RwLock<Vec<FileInfo>>>,
    path_map: &Arc<RwLock<std::collections::HashMap<String, usize>>>,
) {
    if is_removal {
        // Remove deleted files
        let mut to_remove: Vec<String> = Vec::new();
        
        for path in paths {
            if let Ok(map_r) = path_map.read() {
                if map_r.contains_key(path) {
                    to_remove.push(path.clone());
                }
            }
        }
        
        if !to_remove.is_empty() {
            if let Ok(mut files_w) = files.write() {
                if let Ok(mut map_w) = path_map.write() {
                    // Sort indices in reverse order for correct removal
                    let mut indices: Vec<usize> = to_remove.iter().filter_map(|p| map_w.remove(p)).collect();
                    indices.sort_unstable();
                    indices.reverse();

                    for idx in indices {
                        if idx < files_w.len() {
                            files_w.remove(idx);
                        }
                    }

                    // Rebuild path_map only once after all removals
                    map_w.clear();
                    for (i, f) in files_w.iter().enumerate() {
                        map_w.insert(f.path.clone(), i);
                    }
                }
            }

            eprintln!("Removed {} stale entries", to_remove.len());
        }
    } else {
        // Add or update files - first collect all file paths from directories
        let mut all_paths: Vec<String> = Vec::new();
        
        for path in paths {
            let p = Path::new(path);
            if p.is_dir() {
                // Recursively scan the directory for files
                let mut dir_files = scan_directory(p);
                for f in dir_files.drain(..) {
                    all_paths.push(f.path);
                }
            } else if p.is_file() {
                all_paths.push(path.clone());
            }
        }
        
        if all_paths.is_empty() {
            return;
        }
        
        // Process all collected paths
        let new_files: Vec<FileInfo> = all_paths
            .par_iter()
            .filter_map(|path| {
                let p = Path::new(path);
                
                // Skip directories and ignored paths
                if p.is_dir() {
                    return None;
                }
                
                // Get file metadata
                let metadata = match std::fs::metadata(p) {
                    Ok(m) => m,
                    Err(_) => return None,
                };
                
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                
                // Skip ignored files
                let name_lower = name.to_lowercase();
                if IGNORE_DIRS.iter().any(|&ign| name_lower.eq_ignore_ascii_case(ign)) {
                    return None;
                }
                
                Some(FileInfo {
                    name: name.clone(),
                    name_lowercase: name_lower,
                    path: path.clone(),
                    size: metadata.len(),
                    is_dir: false,
                    open_count: 0,
                    icon_index: -1,
                })
            })
            .collect();
        
        if !new_files.is_empty() {
            let count = new_files.len();
            if let Ok(mut files_w) = files.write() {
                if let Ok(mut map_w) = path_map.write() {
                    // Insert files at correct sorted positions (no full re-sort needed)
                    for file in new_files {
                        // Binary search for insertion point
                        let search_result = files_w.binary_search_by(|f| {
                            f.name_lowercase.cmp(&file.name_lowercase)
                        });

                        let insert_idx = match search_result {
                            Ok(existing_idx) => {
                                // File exists - update it
                                files_w[existing_idx] = file;
                                // path_map already has this entry (index unchanged)
                                continue;
                            }
                            Err(insert_idx) => insert_idx,
                        };

                        // Insert new file at sorted position
                        files_w.insert(insert_idx, file);

                        // Update path_map for entries after insert point
                        // All entries from insert_idx onwards shift by 1
                        let len = files_w.len();
                        for i in insert_idx..len {
                            let p = files_w[i].path.clone();
                            map_w.insert(p, i);
                        }
                    }
                }
            }

            eprintln!("Added/updated {} files", count);
        }
    }
}
