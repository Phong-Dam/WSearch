//! Shell operations - open files, show in folder

use std::path::Path;
use windows::{
    core::{HSTRING, PCWSTR},
    Win32::UI::Shell::ShellExecuteW,
    Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
};

#[tauri::command]
pub async fn open_path(path: String) -> Result<(), String> {
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
            None,
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
                .map_err(|e| format!("{} {}", result.0 as i32, e));
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn show_in_folder(path: String) -> Result<(), String> {
    let mut clean_path = path.clone();
    if clean_path.starts_with("\\\\?\\") {
        clean_path = clean_path[4..].to_string();
    }
    let path_wide = HSTRING::from(format!("/select,\"{}\"", clean_path));
    let operation = HSTRING::from("open");
    let explorer = HSTRING::from("explorer.exe");
    unsafe {
        let result = ShellExecuteW(
            None,
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
