//! Narrow in-process bridge for AppKit previews and security-scoped folder access.
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use std::ffi::{c_char, CStr, CString};
    unsafe extern "C" {
        fn tuck_mac_preview(path: *const c_char) -> i32;
        fn tuck_mac_folder_access(
            root: *const c_char,
            file: *const c_char,
            restore: i32,
        ) -> *mut c_char;
        fn tuck_mac_free(value: *mut c_char);
    }
    pub fn preview(path: &Path) -> Result<(), String> {
        let path = CString::new(path.as_os_str().as_encoded_bytes())
            .map_err(|_| "Invalid document path")?;
        // Caller dispatches onto Tauri's main/UI thread. Swift retains the preview objects.
        if unsafe { tuck_mac_preview(path.as_ptr()) } == 1 {
            Ok(())
        } else {
            Err("The document preview could not open.".into())
        }
    }
    pub fn access(root: &str, directory: &Path, restore: bool) -> Result<PathBuf, String> {
        let root = CString::new(root).map_err(|_| "Invalid folder path")?;
        let file = CString::new(
            directory
                .join("folder-access.json")
                .as_os_str()
                .as_encoded_bytes(),
        )
        .map_err(|_| "Invalid storage path")?;
        // Swift serializes access to the grant and owns the returned allocation until free.
        let pointer =
            unsafe { tuck_mac_folder_access(root.as_ptr(), file.as_ptr(), restore as i32) };
        if pointer.is_null() {
            return Err("Folder access unavailable".into());
        }
        let bytes = unsafe { CStr::from_ptr(pointer).to_bytes().to_vec() };
        unsafe { tuck_mac_free(pointer) };
        let result: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid folder access response")?;
        if let Some(error) = result["error"].as_str() {
            return Err(error.into());
        }
        result["path"]
            .as_str()
            .map(PathBuf::from)
            .ok_or_else(|| "Missing folder access path".into())
    }
}
pub fn folder_access(root: &str, directory: &Path, restore: bool) -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        mac::access(root, directory, restore)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (directory, restore);
        Ok(PathBuf::from(root))
    }
}
pub fn preview(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        mac::preview(path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Err("Native previews are available on macOS.".into())
    }
}
