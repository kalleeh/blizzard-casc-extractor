use casc_extractor::casc::casclib_ffi::CascArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎮 Testing CascLib FFI Integration");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Open StarCraft installation
    let archive = CascArchive::open("/Applications/StarCraft")?;
    println!("✅ Successfully opened CASC archive");
    
    // Try to extract a known sprite file
    let test_files = vec![
        "unit\\terran\\marine.grp",
        "unit\\protoss\\zealot.grp",
        "unit\\zerg\\zergling.grp",
    ];
    
    for filename in test_files {
        match archive.extract_file(filename) {
            Ok(data) => {
                println!("✅ Extracted {}: {} bytes", filename, data.len());
                if data.len() > 16 {
                    println!("   First 16 bytes: {:02x?}", &data[..16]);
                }
            }
            Err(e) => {
                println!("❌ Failed to extract {}: {}", filename, e);
            }
        }
    }
    
    Ok(())
}
