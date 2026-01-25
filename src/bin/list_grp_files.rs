use casc_extractor::casc::casclib_ffi::CascArchive;
use std::ffi::CString;
use std::os::raw::{c_char, c_void};

type Handle = *mut c_void;
type DWORD = u32;
type BOOL = i32;

#[repr(C)]
struct CascFindData {
    sz_file_name: [c_char; 1024],
    dw_locale_flags: DWORD,
    dw_file_data_id: DWORD,
    dw_file_size: DWORD,
}

extern "C" {
    fn CascFindFirstFile(storage: Handle, mask: *const c_char, find_data: *mut CascFindData, sz_list_file: *const c_char) -> Handle;
    fn CascFindNextFile(find: Handle, find_data: *mut CascFindData) -> BOOL;
    fn CascFindClose(find: Handle) -> BOOL;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive = CascArchive::open("/Applications/StarCraft")?;
    
    println!("🔍 Searching for all .grp files in unit folders...\n");
    
    let patterns = vec!["unit\\terran\\*.grp", "unit\\protoss\\*.grp", "unit\\zerg\\*.grp"];
    
    for pattern in patterns {
        println!("\n📁 {}", pattern);
        let pattern_c = CString::new(pattern)?;
        let mut find_data = CascFindData {
            sz_file_name: [0; 1024],
            dw_locale_flags: 0,
            dw_file_data_id: 0,
            dw_file_size: 0,
        };
        
        unsafe {
            let find_handle = CascFindFirstFile(
                archive.handle(),
                pattern_c.as_ptr(),
                &mut find_data,
                std::ptr::null()
            );
            
            if !find_handle.is_null() {
                loop {
                    let filename = CString::from_raw(find_data.sz_file_name.as_ptr() as *mut c_char);
                    println!("  - {}", filename.to_string_lossy());
                    std::mem::forget(filename);
                    
                    if CascFindNextFile(find_handle, &mut find_data) == 0 {
                        break;
                    }
                }
                CascFindClose(find_handle);
            }
        }
    }
    
    Ok(())
}
