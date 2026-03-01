/// CascLib FFI bindings — single extern block shared by CascArchive and CascStorage.

use std::ffi::{CString, CStr, c_void, c_char};
use std::os::raw::c_int;
use std::ptr;

type Handle = *mut c_void;
type DWORD = u32;

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
    fn CascOpenStorage(path: *const c_char, flags: u32, storage: *mut Handle) -> bool;
    fn CascCloseStorage(storage: Handle) -> bool;
    fn CascOpenFile(storage: Handle, filename: *const c_char, locale: u32, flags: u32, file: *mut Handle) -> bool;
    fn CascReadFile(file: Handle, buffer: *mut u8, bytes_to_read: u32, bytes_read: *mut u32) -> bool;
    fn CascCloseFile(file: Handle) -> bool;
    fn CascGetFileSize(hFile: Handle, pdwFileSizeHigh: *mut DWORD) -> DWORD;
    fn CascFindFirstFile(storage: Handle, mask: *const c_char, find_data: *mut CascFindData, listfile: *const c_char) -> Handle;
    fn CascFindNextFile(find: Handle, find_data: *mut CascFindData) -> bool;
    fn CascFindClose(find: Handle) -> bool;
}

const CASC_LOCALE_ALL: u32 = 0xFFFFFFFF;

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
            if CascOpenStorage(c_path.as_ptr(), 0, &mut handle) {
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
            if !CascOpenFile(self.handle, c_filename.as_ptr(), CASC_LOCALE_ALL, 0, &mut file_handle) {
                return Err(format!("Failed to open file: {}", filename));
            }

            let mut file_size_high: DWORD = 0;
            let file_size_low = CascGetFileSize(file_handle, &mut file_size_high);
            let file_size = ((file_size_high as u64) << 32) | (file_size_low as u64);

            if file_size == 0 {
                CascCloseFile(file_handle);
                return Ok(Vec::new());
            }

            const MAX_FILE_SIZE: u64 = 512 * 1024 * 1024; // 512 MB
            if file_size > MAX_FILE_SIZE {
                CascCloseFile(file_handle);
                return Err(format!("File size {} exceeds maximum allowed {}", file_size, MAX_FILE_SIZE));
            }

            let mut buffer = vec![0u8; file_size as usize];
            let mut bytes_read: DWORD = 0;

            if !CascReadFile(file_handle, buffer.as_mut_ptr(), file_size as DWORD, &mut bytes_read) {
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

/// File-enumeration wrapper around CascLib.
pub struct CascStorage {
    handle: Handle,
}

impl CascStorage {
    pub fn open(path: &str) -> Result<Self, String> {
        let c_path = CString::new(path).map_err(|e| e.to_string())?;
        let mut handle: Handle = ptr::null_mut();

        unsafe {
            if CascOpenStorage(c_path.as_ptr(), 0, &mut handle) {
                Ok(Self { handle })
            } else {
                Err("Failed to open CASC storage".to_string())
            }
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

        unsafe {
            let find_handle = CascFindFirstFile(
                self.handle,
                mask.as_ptr(),
                &mut find_data,
                ptr::null(),
            );
            if find_handle.is_null() {
                return Err("Failed to enumerate files".to_string());
            }

            let mut files = Vec::new();
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
            Ok(files)
        }
    }
}

impl Drop for CascStorage {
    fn drop(&mut self) {
        unsafe {
            CascCloseStorage(self.handle);
        }
    }
}
