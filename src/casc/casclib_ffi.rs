/// CascLib FFI bindings — single extern block shared by CascArchive and CascStorage.
use std::ffi::{CString, CStr, c_void, c_char};
use std::os::raw::c_int;
use std::ptr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CascLibError {
    #[error("Failed to open CASC storage at '{path}'")]
    OpenFailed { path: String },

    #[error("File not found in archive: '{path}'")]
    FileNotFound { path: String },

    #[error("Failed to read file '{path}'")]
    ReadFailed { path: String },

    #[error("Failed to list archive files")]
    ListFailed,

    #[error("Invalid path: {reason}")]
    InvalidPath { reason: String },

    #[error("File size {size} bytes exceeds maximum allowed")]
    FileTooLarge { size: u64 },
}

type Handle = *mut c_void;
type Dword = u32;

/// Mirror of CascLib's `CASC_FIND_DATA` (CascLib.h). Field order, types, and
/// sizes MUST match the C struct exactly: `CascFindFirstFile` writes the full
/// struct into this memory, so any layout mismatch overflows the buffer (SIGSEGV).
/// `MAX_PATH` is 1024 in CascLib's CascPort.h on macOS/Linux (not the Windows 260).
/// `bFileAvailable` is a `DWORD:1` bitfield, stored in a full 4-byte unit.
#[repr(C)]
struct CascFindData {
    file_name: [c_char; 1024],
    c_key: [u8; 16],
    e_key: [u8; 16],
    tag_bit_mask: u64,
    file_size: u64,
    plain_name: *mut c_char,
    file_data_id: u32,
    locale_flags: u32,
    content_flags: u32,
    span_count: u32,
    file_available: u32,
    name_type: c_int,
}

// Guard against layout drift: must match sizeof(CASC_FIND_DATA) on 64-bit
// (1024 + 16 + 16 + 8 + 8 + 8 + 4*6, padded to 8-byte alignment = 1104).
const _: () = assert!(std::mem::size_of::<CascFindData>() == 1104);

#[link(name = "casc")]
extern "C" {
    // Second parameter is dwLocaleMask, NOT a feature-flag field.
    fn CascOpenStorage(path: *const c_char, locale_mask: u32, storage: *mut Handle) -> bool;
    fn CascOpenOnlineStorage(params: *const c_char, locale_mask: u32, storage: *mut Handle) -> bool;
    fn CascCloseStorage(storage: Handle) -> bool;
    fn CascOpenFile(storage: Handle, filename: *const c_char, locale: u32, flags: u32, file: *mut Handle) -> bool;
    fn CascReadFile(file: Handle, buffer: *mut u8, bytes_to_read: u32, bytes_read: *mut u32) -> bool;
    fn CascCloseFile(file: Handle) -> bool;
    fn CascGetFileSize(hFile: Handle, pdwFileSizeHigh: *mut Dword) -> Dword;
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

    pub fn open(path: &str) -> Result<Self, CascLibError> {
        let c_path = CString::new(path).map_err(|e| CascLibError::InvalidPath { reason: e.to_string() })?;
        let mut handle: Handle = ptr::null_mut();

        unsafe {
            if CascOpenStorage(c_path.as_ptr(), 0, &mut handle) {
                Ok(Self { handle })
            } else {
                Err(CascLibError::OpenFailed { path: path.to_string() })
            }
        }
    }

    /// Open CDN-backed online storage.
    ///
    /// `params` is CascLib's `cache_path*product*region` string — e.g.
    /// `/tmp/casc-cache*s1*eu` — *not* a plain install directory. Fields are
    /// separated by `CASC_PARAM_SEPARATOR` (`'*'`). This must go through
    /// `CascOpenOnlineStorage`; passing a flag to `CascOpenStorage` cannot work,
    /// because that argument is `dwLocaleMask` (the old value 0x40 silently
    /// meant CASC_LOCALE_ZHCN).
    pub fn open_online(params: &str) -> Result<Self, CascLibError> {
        let c_params = CString::new(params).map_err(|e| CascLibError::InvalidPath { reason: e.to_string() })?;
        let mut handle: Handle = ptr::null_mut();

        unsafe {
            if CascOpenOnlineStorage(c_params.as_ptr(), CASC_LOCALE_ALL, &mut handle) {
                Ok(Self { handle })
            } else {
                Err(CascLibError::OpenFailed { path: params.to_string() })
            }
        }
    }

    pub fn extract_file(&self, filename: &str) -> Result<Vec<u8>, CascLibError> {
        let c_filename = CString::new(filename).map_err(|e| CascLibError::InvalidPath { reason: e.to_string() })?;
        let mut file_handle: Handle = ptr::null_mut();

        unsafe {
            if !CascOpenFile(self.handle, c_filename.as_ptr(), CASC_LOCALE_ALL, 0, &mut file_handle) {
                return Err(CascLibError::FileNotFound { path: filename.to_string() });
            }

            let mut file_size_high: Dword = 0;
            let file_size_low = CascGetFileSize(file_handle, &mut file_size_high);
            // On failure CascGetFileSize returns CASC_INVALID_SIZE and leaves
            // pdwFileSizeHigh untouched; without this the size reads as 4294967295
            // and surfaces as a bogus "file too large" error.
            const CASC_INVALID_SIZE: Dword = 0xFFFF_FFFF;
            if file_size_low == CASC_INVALID_SIZE && file_size_high == 0 {
                CascCloseFile(file_handle);
                return Err(CascLibError::ReadFailed { path: filename.to_string() });
            }
            let file_size = ((file_size_high as u64) << 32) | (file_size_low as u64);

            if file_size == 0 {
                CascCloseFile(file_handle);
                return Ok(Vec::new());
            }

            const MAX_FILE_SIZE: u64 = 512 * 1024 * 1024; // 512 MB
            if file_size > MAX_FILE_SIZE {
                CascCloseFile(file_handle);
                return Err(CascLibError::FileTooLarge { size: file_size });
            }

            let mut buffer = vec![0u8; file_size as usize];
            let mut bytes_read: Dword = 0;

            if !CascReadFile(file_handle, buffer.as_mut_ptr(), file_size as Dword, &mut bytes_read) {
                CascCloseFile(file_handle);
                return Err(CascLibError::ReadFailed { path: filename.to_string() });
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
    pub fn open(path: &str) -> Result<Self, CascLibError> {
        let c_path = CString::new(path).map_err(|e| CascLibError::InvalidPath { reason: e.to_string() })?;
        let mut handle: Handle = ptr::null_mut();

        unsafe {
            if CascOpenStorage(c_path.as_ptr(), 0, &mut handle) {
                Ok(Self { handle })
            } else {
                Err(CascLibError::OpenFailed { path: path.to_string() })
            }
        }
    }

    pub fn list_files(&self) -> Result<Vec<String>, CascLibError> {
        let mask = CString::new("*").unwrap();
        let mut find_data = CascFindData {
            file_name: [0; 1024],
            c_key: [0; 16],
            e_key: [0; 16],
            tag_bit_mask: 0,
            file_size: 0,
            plain_name: ptr::null_mut(),
            file_data_id: 0,
            locale_flags: 0,
            content_flags: 0,
            span_count: 0,
            file_available: 0,
            name_type: 0,
        };

        unsafe {
            let find_handle = CascFindFirstFile(
                self.handle,
                mask.as_ptr(),
                &mut find_data,
                ptr::null(),
            );
            // CascFindFirstFile signals failure with INVALID_HANDLE_VALUE ((HANDLE)-1),
            // not NULL. Checking only for null let a broken archive fall through to
            // the loop and return an empty list as success, so callers reported
            // "0 files found" and exited 0.
            const INVALID_HANDLE_VALUE: Handle = usize::MAX as Handle;
            if find_handle.is_null() || find_handle == INVALID_HANDLE_VALUE {
                return Err(CascLibError::ListFailed);
            }

            let mut files = Vec::new();
            loop {
                // bFileAvailable is a DWORD:1 bitfield; the remaining 31 bits are
                // padding the library never explicitly initialises.
                if find_data.file_available & 1 != 0 {
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
