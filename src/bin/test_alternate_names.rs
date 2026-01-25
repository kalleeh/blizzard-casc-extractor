use casc_extractor::casc::casclib_ffi::CascArchive;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive = CascArchive::open("/Applications/StarCraft")?;
    let names = fs::read_to_string("test_names.txt")?;
    
    println!("🔍 Testing alternate file names...\n");
    
    for line in names.lines() {
        let filename = line.trim();
        if filename.is_empty() {
            continue;
        }
        
        match archive.extract_file(filename) {
            Ok(data) => {
                println!("✅ FOUND: {} ({} bytes)", filename, data.len());
            }
            Err(_) => {}
        }
    }
    
    Ok(())
}
