use casc_extractor::casc::casclib_ffi::CascArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive = CascArchive::open("/Applications/StarCraft")?;
    
    // Try common palette file names
    let palette_files = vec![
        "unit\\cmdbtns\\cmdicons.pal",
        "tileset\\platform.pal",
        "game\\tunit.pal",
        "game\\tminimap.pal",
        "unit\\wirefram\\wirefram.pal",
    ];
    
    for filename in palette_files {
        match archive.extract_file(filename) {
            Ok(data) => {
                println!("✅ Found {}: {} bytes", filename, data.len());
                println!("   First 48 bytes (16 RGB entries): {:02x?}", &data[0..48.min(data.len())]);
            }
            Err(_) => {
                println!("❌ Not found: {}", filename);
            }
        }
    }
    
    Ok(())
}
