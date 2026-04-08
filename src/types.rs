//! Data types and structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{mpsc, Arc, RwLock, atomic::AtomicBool};

/// File information stored in index
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
    #[allow(dead_code)]
    pub icon_index: i32,
}

/// Search response with results and ID
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchResponse {
    pub results: Vec<FileInfo>,
    pub search_id: u32,
}

/// Scored file for fuzzy matching results
#[derive(Debug)]
pub struct ScoredFile {
    pub file: FileInfo,
    pub score: i32,
}

/// Application state for file indexing
pub struct AppIndex {
    pub files: Arc<RwLock<Vec<FileInfo>>>,
    pub path_map: Arc<RwLock<HashMap<String, usize>>>,
    pub save_tx: mpsc::Sender<()>,
    pub cancel_index: Arc<AtomicBool>,
    pub is_loading_cache: Arc<AtomicBool>,
    pub is_indexing: Arc<AtomicBool>,
    pub icon_cache: Arc<RwLock<HashMap<String, String>>>,
}

/// Directories to ignore during indexing
pub const IGNORE_DIRS: &[&str] = &[
    "node_modules", ".git", "AppData", "$Recycle.Bin",
    "Windows", "System32", "ProgramData", "Recovery",
];
