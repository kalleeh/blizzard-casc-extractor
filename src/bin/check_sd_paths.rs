use casc_extractor::casc::casclib_ffi::CascArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive = CascArchive::open("/Applications/StarCraft")?;
    
    // Try SD (standard definition) paths
    let sd_patterns = vec![
        "SD\\unit\\terran\\wraith.grp",
        "SD\\unit\\terran\\valkyrie.grp",
        "SD\\unit\\terran\\vessel.grp",
        "SD\\unit\\protoss\\darktemplar.grp",
        "SD\\unit\\protoss\\darkarch.grp",
        "SD\\unit\\protoss\\reaver.grp",
        "SD\\unit\\protoss\\observer.grp",
        "SD\\unit\\zerg\\mutalisk.grp",
        "SD\\unit\\zerg\\scourge.grp",
    ];
    
    println!("🔍 Checking SD (Standard Definition) paths...\n");
    
    for path in sd_patterns {
        match archive.extract_file(path) {
            Ok(data) => {
                println!("✅ FOUND: {} ({} bytes)", path, data.len());
            }
            Err(_) => {
                println!("❌ Not found: {}", path);
            }
        }
    }
    
    Ok(())
}
