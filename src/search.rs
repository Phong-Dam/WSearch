//! Search logic and fuzzy matching

use crate::types::{FileInfo, ScoredFile, AppIndex, SearchResponse};
use rayon::prelude::*;
use tauri::State;

/// Fuzzy matching algorithm that returns a score
/// Higher score = better match
pub fn fuzzy_match(text: &str, pattern: &str) -> Option<i32> {
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
            if i == last_match_idx + 1 {
                consecutive += 1;
                score += 10 + consecutive * 5;
            } else {
                consecutive = 0;
                score += 10;
            }
            
            if pattern_idx == 0 && i == 0 {
                score += 15;
            }
            
            if i > 0 && (text_chars[i-1] == '/' || text_chars[i-1] == '\\' 
                || text_chars[i-1] == ' ' || text_chars[i-1] == '_' || text_chars[i-1] == '-') {
                score += 12;
            }
            
            if pattern_idx > 0 {
                let gap = i - last_match_idx - 1;
                score -= gap as i32;
            }
            
            last_match_idx = i;
            pattern_idx += 1;
            
            if pattern_idx == pattern_chars.len() {
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

#[tauri::command]
pub async fn search_files(
    query: String,
    search_id: u32,
    use_fuzzy: bool,
    state: State<'_, AppIndex>,
) -> Result<SearchResponse, String> {
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

    // Binary search for prefix match start
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

        // Scan remaining for substring matches (after prefix range)
        let mut substring_matches: Vec<FileInfo> = files_read[end_of_prefix..]
            .iter()
            .take(MAX_SUBSTRING_SCAN)
            .collect::<Vec<_>>()
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
        
        // Also scan before prefix range if still not enough
        if results.len() < LIMIT && start_idx > 0 {
            let remaining_needed = LIMIT - results.len();
            let mut prefix_matches: Vec<FileInfo> = files_read[..start_idx]
                .iter()
                .collect::<Vec<_>>()
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
        
        // Fuzzy matching fallback
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
pub async fn record_open(path: String, state: State<'_, AppIndex>) -> Result<(), String> {
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
pub async fn get_index_status(state: State<'_, AppIndex>) -> Result<(usize, bool, bool), String> {
    let files = state.files.read().map_err(|e| e.to_string())?;
    let is_indexing = state.is_indexing.load(std::sync::atomic::Ordering::SeqCst);
    let is_loading = state.is_loading_cache.load(std::sync::atomic::Ordering::SeqCst);
    Ok((files.len(), is_indexing, is_loading))
}

#[tauri::command]
pub async fn cancel_indexing(state: State<'_, AppIndex>) -> Result<(), String> {
    state.cancel_index.store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}
