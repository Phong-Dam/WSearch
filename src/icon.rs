//! File icon extraction using Windows Shell

use crate::types::AppIndex;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use std::path::Path;
use tauri::State;

use windows::core::HSTRING;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject,
    SelectObject, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL};

#[allow(dead_code)]
pub fn get_file_icon_index(path: &str) -> i32 {
    unsafe {
        let mut info = SHFILEINFOW::default();
        let path_wide = HSTRING::from(path);
        let result = SHGetFileInfoW(
            &path_wide,
            FILE_ATTRIBUTE_NORMAL,
            Some(&mut info as *mut _),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            windows::Win32::UI::Shell::SHGFI_SYSICONINDEX,
        );
        if result != 0 {
            info.iIcon as i32
        } else {
            -1
        }
    }
}

/// Extract icon and convert to base64 PNG data URI
pub fn extract_file_icon_base64(path: &str) -> Result<String, String> {
    unsafe {
        let mut info = SHFILEINFOW::default();
        let path_wide = HSTRING::from(path);
        
        let result = SHGetFileInfoW(
            &path_wide,
            FILE_ATTRIBUTE_NORMAL,
            Some(&mut info as *mut _),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON,
        );
        
        if result == 0 || info.hIcon.is_invalid() {
            return Err("Failed to get icon".to_string());
        }
        
        let hicon = info.hIcon;
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
                biHeight: -size,
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
        
        // Fill with white background
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
        
        // Draw icon
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
            bits.swap(i, i + 2);
        }
        
        // Encode to PNG via canvas
        let base64_str = STANDARD.encode(&bits);
        Ok(format!("data:image/png;base64,{}", base64_str))
    }
}

#[tauri::command]
pub async fn get_file_icon(path: String, state: State<'_, AppIndex>) -> Result<String, String> {
    let extension = Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("_folder_")
        .to_lowercase();
    
    // Files with unique icons per file (not by extension)
    const UNIQUE_ICON_EXTS: &[&str] = &["exe", "lnk", "dll", "ico", "scr", "cpl", "msc"];
    
    // Determine cache key
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
            if let Ok(mut cache) = state.icon_cache.write() {
                cache.insert(cache_key, icon_data.clone());
            }
            Ok(icon_data)
        }
        Err(e) => Err(e),
    }
}
