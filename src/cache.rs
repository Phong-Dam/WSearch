//! File index cache management

use crate::types::{FileInfo, IGNORE_DIRS, BenchmarkMetrics};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use jwalk::WalkDir;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc, RwLock, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::Disks;

/// Get cache file path
pub fn get_cache_path() -> PathBuf {
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        let cache_path = exe_path.join("index_cache.dat");
        let test_path = exe_path.join(".write_test.tmp");
        if std::fs::write(&test_path, b"").is_ok() {
            let _ = std::fs::remove_file(&test_path);
            return cache_path;
        }
    }
    std::env::temp_dir().join("wsearch_index_cache.dat")
}

/// Start cache auto-save thread
pub fn start_cache_saver(
    files: Arc<RwLock<Vec<FileInfo>>>,
    cache_path: PathBuf,
    save_rx: mpsc::Receiver<()>,
) {
    thread::spawn(move || {
        loop {
            if save_rx.recv().is_err() { break; }
            thread::sleep(Duration::from_secs(2));
            while save_rx.try_recv().is_ok() {}
            
            if let Ok(snapshot) = files.read() {
                if let Ok(encoded) = bincode::serialize(&*snapshot) {
                    let tmp = cache_path.with_extension("tmp");
                    if let Ok(file) = File::create(&tmp) {
                        let mut encoder = GzEncoder::new(file, Compression::default());
                        if encoder.write_all(&encoded).is_ok() {
                            if encoder.finish().is_ok() {
                                let _ = std::fs::rename(&tmp, &cache_path);
                            }
                        }
                    }
                }
            }
        }
    });
}

/// Load cached files from disk
pub fn load_cache(
    cache_path: &PathBuf,
    files: Arc<RwLock<Vec<FileInfo>>>,
    path_map: Arc<RwLock<std::collections::HashMap<String, usize>>>,
    metrics: Option<Arc<Mutex<BenchmarkMetrics>>>,
) {
    let load_start = Instant::now();

    if let Ok(f) = File::open(cache_path) {
        let cache_size = f.metadata().map(|m| m.len()).unwrap_or(0);
        let mut decoder = GzDecoder::new(f);
        let mut buf = Vec::new();
        if decoder.read_to_end(&mut buf).is_ok() {
            if let Ok(mut cached) = bincode::deserialize::<Vec<FileInfo>>(&buf) {
                // Populate name_lowercase (skipped during serialization) + build path_map
                let len = cached.len();
                let mut map = std::collections::HashMap::with_capacity(len);
                for (i, fi) in cached.iter_mut().enumerate() {
                    fi.name_lowercase = fi.name.to_lowercase();
                    map.insert(fi.path.clone(), i);
                }

                if let Ok(mut files_w) = files.write() {
                    *files_w = cached;
                    if let Ok(mut map_w) = path_map.write() {
                        *map_w = map;
                    }
                }

                // Update metrics
                if let Some(m) = metrics {
                    if let Ok(mut metrics) = m.lock() {
                        metrics.last_cache_load_ms = load_start.elapsed().as_millis() as u64;
                        metrics.cache_size_bytes = cache_size;
                        metrics.indexed_file_count = len;
                    }
                }
            }
        }
    }
}

/// Scan disks and index new files
pub fn scan_and_index(
    files: Arc<RwLock<Vec<FileInfo>>>,
    path_map: Arc<RwLock<std::collections::HashMap<String, usize>>>,
    save_tx: mpsc::Sender<()>,
    cancel: Arc<std::sync::atomic::AtomicBool>,
    is_indexing: Arc<std::sync::atomic::AtomicBool>,
) {
    // Check if we have existing files (from cache)
    let has_existing = {
        if let Ok(map_r) = path_map.read() {
            !map_r.is_empty()
        } else {
            false
        }
    };
    
    // If no existing cache, we don't need to check for duplicates
    let existing_paths: Option<HashSet<String>> = if has_existing {
        Some({
            if let Ok(map_r) = path_map.read() {
                map_r.keys().cloned().collect()
            } else {
                HashSet::new()
            }
        })
    } else {
        None
    };
    
    let mut disks = Disks::new();
    disks.refresh(true);
    
    // Collect disk mount points
    let disk_mounts: Vec<String> = disks.iter()
        .map(|d| d.mount_point().to_string_lossy().to_string())
        .collect();
    
    // Parallel scan across all disks
    let all_new_files: Vec<FileInfo> = disk_mounts
        .par_iter()
        .filter_map(|disk_mount| {
            if cancel.load(Ordering::SeqCst) { return None; }
            
            let walker = WalkDir::new(disk_mount)
                .skip_hidden(true)
                .process_read_dir(|_, _, _, children| {
                    children.retain(|r| {
                        r.as_ref().map(|entry| {
                            let name = entry.file_name().to_string_lossy();
                            !IGNORE_DIRS.iter().any(|&d| name.eq_ignore_ascii_case(d))
                        }).unwrap_or(false)
                    });
                });
            
            let mut paths: Vec<(String, String, bool, u64)> = Vec::new();
            
            for entry in walker.into_iter().filter_map(|e| e.ok()) {
                if cancel.load(Ordering::SeqCst) { break; }
                
                let path_str = entry.path().to_string_lossy().to_string();
                
                // Skip duplicates if we have cache
                if let Some(ref paths_set) = existing_paths {
                    if paths_set.contains(&path_str) {
                        continue;
                    }
                }
                
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().is_dir();
                let size = if !is_dir {
                    entry.metadata().map(|m| m.len()).unwrap_or(0)
                } else {
                    0
                };
                
                paths.push((path_str, name, is_dir, size));
            }
            
            if cancel.load(Ordering::SeqCst) { return None; }
            
            // Process paths in parallel chunks
            let chunk_size = 100_000;
            let batch: Vec<FileInfo> = paths
                .par_chunks(chunk_size)
                .flat_map(|chunk| {
                    chunk.iter()
                        .filter(|(_, _, is_dir, _)| !*is_dir)
                        .map(|(path, name, _, size)| {
                            FileInfo {
                                name: name.clone(),
                                name_lowercase: name.to_lowercase(),
                                path: path.clone(),
                                size: *size,
                                is_dir: false,
                                open_count: 0,
                                icon_index: -1,
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
            
            Some(batch)
        })
        .flatten()
        .collect();
    
    // Merge with existing files - single sort, single map rebuild
    if !cancel.load(Ordering::SeqCst) {
        if let Ok(mut files_w) = files.write() {
            if has_existing {
                files_w.extend(all_new_files);
            } else {
                *files_w = all_new_files;
            }
            
            // Single sort
            files_w.par_sort_unstable_by(|a, b| a.name_lowercase.cmp(&b.name_lowercase));
            
            // Single path_map rebuild
            if let Ok(mut map_w) = path_map.write() {
                map_w.clear();
                map_w.reserve(files_w.len());
                for (idx, fi) in files_w.iter().enumerate() {
                    map_w.insert(fi.path.clone(), idx);
                }
            }
        }
        let _ = save_tx.send(());
    }
    
    // Mark indexing as complete
    is_indexing.store(false, Ordering::SeqCst);
}

/// Cleanup stale entries - removes files that no longer exist
pub fn cleanup_stale_entries(
    files: Arc<RwLock<Vec<FileInfo>>>,
    path_map: Arc<RwLock<std::collections::HashMap<String, usize>>>,
    cancel: Arc<std::sync::atomic::AtomicBool>,
) -> usize {
    let files_read = match files.read() {
        Ok(f) => f,
        Err(_) => return 0,
    };

    let total = files_read.len();
    if total == 0 {
        return 0;
    }

    // Check a sample of files (10% but at least 1000, max 10000)
    let sample_size = (total / 10).max(1000).min(10000);
    let step = (total / sample_size).max(1);

    // Collect stale indices (files only, not dirs)
    let stale_indices: Vec<usize> = files_read
        .iter()
        .enumerate()
        .step_by(step)
        .filter_map(|(idx, file)| {
            if cancel.load(Ordering::SeqCst) {
                return None;
            }
            if !file.is_dir && fs::metadata(&file.path).is_err() {
                Some(idx)
            } else {
                None
            }
        })
        .collect();
    
    drop(files_read);
    
    if stale_indices.is_empty() {
        return 0;
    }
    
    // Remove stale entries
    if let Ok(mut files_w) = files.write() {
        // Remove in reverse order to maintain correct indices
        for &idx in stale_indices.iter().rev() {
            if idx < files_w.len() {
                let removed_file = files_w.remove(idx);
                
                // Remove from path_map
                if let Ok(mut map_w) = path_map.write() {
                    map_w.remove(&removed_file.path);
                }
            }
        }
        
        // Rebuild path_map to fix indices
        if let Ok(mut map_w) = path_map.write() {
            map_w.clear();
            for (idx, fi) in files_w.iter().enumerate() {
                map_w.insert(fi.path.clone(), idx);
            }
        }
        
        stale_indices.len()
    } else {
        0
    }
}

/// Start background cleanup thread - runs every 30 minutes
pub fn start_cleanup_thread(
    files: Arc<RwLock<Vec<FileInfo>>>,
    path_map: Arc<RwLock<std::collections::HashMap<String, usize>>>,
    cancel: Arc<std::sync::atomic::AtomicBool>,
) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(30 * 60)); // 30 minutes

            let removed = cleanup_stale_entries(files.clone(), path_map.clone(), cancel.clone());
            if removed > 0 {
                eprintln!("Cleaned up {} stale entries", removed);
            }
        }
    });
}
