// Simple CascLib FFI wrapper for StarCraft: Remastered extraction
// This uses the proven CascLib implementation instead of reimplementing decryption

use std::ffi::{CString, CStr};
use std::os::raw::{c_char, c_void, c_int};
use std::path::Path;

#[repr(C)]
struct CascFindData {
    file_size: u64,
    file_name: [c_char; 1024],
    c_key: [u8; 16],
    e_key: [u8; 16],
    file_available: c_int,
}

#[link(name = "casc")]
extern "C" {
    fn CascOpenStorage(path: *const c_char, flags: u32, storage: *mut *mut c_void) -> bool;
    fn CascCloseStorage(storage: *mut c_void) -> bool;
    fn CascOpenFile(storage: *mut c_void, filename: *const c_char, locale: u32, flags: u32, file: *mut *mut c_void) -> bool;
    fn CascReadFile(file: *mut c_void, buffer: *mut u8, bytes_to_read: u32, bytes_read: *mut u32) -> bool;
    fn CascCloseFile(file: *mut c_void) -> bool;
    fn CascFindFirstFile(storage: *mut c_void, mask: *const c_char, find_data: *mut CascFindData, listfile: *const c_char) -> *mut c_void;
    fn CascFindNextFile(find: *mut c_void, find_data: *mut CascFindData) -> bool;
    fn CascFindClose(find: *mut c_void) -> bool;
}

pub struct CascStorage {
    handle: *mut c_void,
}

impl CascStorage {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path_str = CString::new(path.as_ref().to_str().unwrap()).unwrap();
        let mut handle: *mut c_void = std::ptr::null_mut();
        
        unsafe {
            if CascOpenStorage(path_str.as_ptr(), 0, &mut handle) {
                Ok(CascStorage { handle })
            } else {
                Err("Failed to open CASC storage".to_string())
            }
        }
    }
    
    pub fn extract_file(&self, filename: &str) -> Result<Vec<u8>, String> {
        let filename_c = CString::new(filename).unwrap();
        let mut file_handle: *mut c_void = std::ptr::null_mut();
        
        unsafe {
            if !CascOpenFile(self.handle, filename_c.as_ptr(), 0, 0, &mut file_handle) {
                return Err(format!("Failed to open file: {}", filename));
            }
            
            let mut data = Vec::new();
            let mut buffer = [0u8; 4096];
            let mut bytes_read: u32 = 0;
            
            loop {
                if !CascReadFile(file_handle, buffer.as_mut_ptr(), buffer.len() as u32, &mut bytes_read) {
                    break;
                }
                if bytes_read == 0 {
                    break;
                }
                data.extend_from_slice(&buffer[..bytes_read as usize]);
            }
            
            CascCloseFile(file_handle);
            Ok(data)
        }
    }
    
    pub fn list_files(&self) -> Result<Vec<String>, String> {
        let mask = CString::new("*").unwrap();
        let mut find_data = CascFindData {
            file_size: 0,
            file_name: [0; 1024],
            c_key: [0; 16],
            e_key: [0; 16],
            file_available: 0,
        };
        
        let mut files = Vec::new();
        
        unsafe {
            let find_handle = CascFindFirstFile(self.handle, mask.as_ptr(), &mut find_data, std::ptr::null());
            if find_handle.is_null() {
                return Err("Failed to enumerate files".to_string());
            }
            
            loop {
                if find_data.file_available != 0 {
                    let filename = CStr::from_ptr(find_data.file_name.as_ptr())
                        .to_string_lossy()
                        .to_string();
                    files.push(filename);
                }
                
                if !CascFindNextFile(find_handle, &mut find_data) {
                    break;
                }
            }
            
            CascFindClose(find_handle);
        }
        
        Ok(files)
    }
}

impl Drop for CascStorage {
    fn drop(&mut self) {
        unsafe {
            CascCloseStorage(self.handle);
        }
    }
}
