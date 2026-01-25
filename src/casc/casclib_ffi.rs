/// Minimal CascLib FFI bindings for file extraction
/// 
/// This provides just enough CascLib functionality to extract files properly.

use std::ffi::{CString, c_void, c_char};
use std::ptr;

type Handle = *mut c_void;
type DWORD = u32;
type BOOL = i32;

#[link(name = "casc")]
extern "C" {
    fn CascOpenStorage(szDataPath: *const c_char, dwLocaleMask: DWORD, phStorage: *mut Handle) -> BOOL;
    fn CascCloseStorage(hStorage: Handle) -> BOOL;
    fn CascOpenFile(hStorage: Handle, szFileName: *const c_char, dwLocale: DWORD, dwFlags: DWORD, phFile: *mut Handle) -> BOOL;
    fn CascReadFile(hFile: Handle, lpBuffer: *mut c_void, dwToRead: DWORD, pdwRead: *mut DWORD) -> BOOL;
    fn CascCloseFile(hFile: Handle) -> BOOL;
    fn CascGetFileSize(hFile: Handle, pdwFileSizeHigh: *mut DWORD) -> DWORD;
}

const CASC_LOCALE_ALL: DWORD = 0xFFFFFFFF;

pub struct CascArchive {
    handle: Handle,
}

impl CascArchive {
    pub fn handle(&self) -> Handle {
        self.handle
    }
    
    pub fn open(path: &str) -> Result<Self, String> {
        let c_path = CString::new(path).map_err(|e| e.to_string())?;
        let mut handle: Handle = ptr::null_mut();
        
        unsafe {
            if CascOpenStorage(c_path.as_ptr(), 0, &mut handle) != 0 {
                Ok(Self { handle })
            } else {
                Err("Failed to open CASC storage".to_string())
            }
        }
    }
    
    pub fn extract_file(&self, filename: &str) -> Result<Vec<u8>, String> {
        let c_filename = CString::new(filename).map_err(|e| e.to_string())?;
        let mut file_handle: Handle = ptr::null_mut();
        
        unsafe {
            // Open file by name (CascLib handles decryption internally)
            if CascOpenFile(self.handle, c_filename.as_ptr(), CASC_LOCALE_ALL, 0, &mut file_handle) == 0 {
                return Err(format!("Failed to open file: {}", filename));
            }
            
            // Get file size
            let mut file_size_high: DWORD = 0;
            let file_size_low = CascGetFileSize(file_handle, &mut file_size_high);
            let file_size = ((file_size_high as u64) << 32) | (file_size_low as u64);
            
            if file_size == 0 {
                CascCloseFile(file_handle);
                return Ok(Vec::new());
            }
            
            // Read file (CascLib handles ALL decryption/decompression internally)
            let mut buffer = vec![0u8; file_size as usize];
            let mut bytes_read: DWORD = 0;
            
            if CascReadFile(file_handle, buffer.as_mut_ptr() as *mut c_void, file_size as DWORD, &mut bytes_read) == 0 {
                CascCloseFile(file_handle);
                return Err(format!("Failed to read file: {}", filename));
            }
            
            buffer.truncate(bytes_read as usize);
            CascCloseFile(file_handle);
            
            Ok(buffer)
        }
    }
}

impl Drop for CascArchive {
    fn drop(&mut self) {
        unsafe {
            CascCloseStorage(self.handle);
        }
    }
}
