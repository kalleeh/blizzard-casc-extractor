use casc_extractor::casc::casclib_ffi::CascArchive;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive = CascArchive::open("/Applications/StarCraft")?;
    
    // Try to find BMP or PCX files that might contain palette data
    let image_files = vec![
        "game\\tunit.pcx",
        "game\\icons.pcx", 
        "unit\\cmdbtns\\cmdicons.pcx",
        "tileset\\badlands\\dark.pcx",
        "glue\\palmm\\title.pcx",
    ];
    
    for filename in image_files {
        match archive.extract_file(filename) {
            Ok(data) => {
                println!("✅ Found {}: {} bytes", filename, data.len());
                
                // PCX files have palette at the end (last 769 bytes: 0x0C marker + 768 bytes RGB)
                if data.len() > 769 && data[data.len() - 769] == 0x0C {
                    println!("   Found PCX palette!");
                    let palette_start = data.len() - 768;
                    let palette_data = &data[palette_start..];
                    
                    // Save as Rust array
                    let mut rust_code = String::from("pub fn starcraft_palette() -> [[u8; 4]; 256] {\n    [\n");
                    for i in 0..256 {
                        let r = palette_data[i * 3];
                        let g = palette_data[i * 3 + 1];
                        let b = palette_data[i * 3 + 2];
                        let a = if i == 0 { 0 } else { 255 };
                        
                        if i % 4 == 0 {
                            rust_code.push_str("        ");
                        }
                        rust_code.push_str(&format!("[{},{},{},{}],", r, g, b, a));
                        if i % 4 == 3 {
                            rust_code.push('\n');
                        }
                    }
                    rust_code.push_str("    ]\n}\n");
                    
                    fs::write("extracted_palette.rs", &rust_code)?;
                    println!("   💾 Saved to extracted_palette.rs");
                    
                    // Also save first 16 colors for inspection
                    println!("   First 16 colors:");
                    for i in 0..16 {
                        let r = palette_data[i * 3];
                        let g = palette_data[i * 3 + 1];
                        let b = palette_data[i * 3 + 2];
                        println!("     {}: RGB({}, {}, {})", i, r, g, b);
                    }
                    
                    return Ok(());
                }
            }
            Err(_) => {}
        }
    }
    
    println!("❌ No palette files found");
    Ok(())
}
