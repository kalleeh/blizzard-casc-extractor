use casc_extractor::casc::casclib_ffi::CascArchive;
use casc_extractor::grp::GrpFile;
use std::path::Path;
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    println!("🎮 Real StarCraft Sprite Extraction (CascLib FFI)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let archive = CascArchive::open("/Applications/StarCraft")?;
    println!("✅ Opened CASC archive");
    
    let output_dir = Path::new("extracted/casclib_sprites");
    std::fs::create_dir_all(output_dir)?;
    
    // Known StarCraft sprite files
    let sprite_files = vec![
        "unit\\terran\\marine.grp",
        "unit\\terran\\firebat.grp",
        "unit\\terran\\ghost.grp",
        "unit\\terran\\goliath.grp",
        "unit\\terran\\tank.grp",
        "unit\\terran\\wraith.grp",
        "unit\\protoss\\zealot.grp",
        "unit\\protoss\\dragoon.grp",
        "unit\\protoss\\hightemplar.grp",
        "unit\\protoss\\archon.grp",
        "unit\\protoss\\scout.grp",
        "unit\\zerg\\zergling.grp",
        "unit\\zerg\\hydralisk.grp",
        "unit\\zerg\\ultralisk.grp",
        "unit\\zerg\\mutalisk.grp",
    ];
    
    let mut success_count = 0;
    let mut fail_count = 0;
    
    for filename in sprite_files {
        match archive.extract_file(filename) {
            Ok(data) => {
                println!("✅ Extracted {}: {} bytes", filename, data.len());
                
                // Try to parse as GRP
                match GrpFile::parse(&data) {
                    Ok(grp) => {
                        println!("   GRP: {}x{}, {} frames", grp.width, grp.height, grp.frame_count);
                        
                        // Convert first frame to PNG
                        if let Some(frame) = grp.get_first_frame() {
                            let rgba = frame.to_rgba()?;
                            
                            let safe_name = filename.replace("\\", "_").replace("/", "_");
                            let png_path = output_dir.join(format!("{}.png", safe_name));
                            
                            let file = File::create(&png_path)?;
                            let w = BufWriter::new(file);
                            let mut encoder = png::Encoder::new(w, frame.width as u32, frame.height as u32);
                            encoder.set_color(png::ColorType::Rgba);
                            encoder.set_depth(png::BitDepth::Eight);
                            
                            let mut writer = encoder.write_header()?;
                            writer.write_image_data(&rgba)?;
                            
                            println!("   💾 Saved to {:?}", png_path);
                            success_count += 1;
                        }
                    }
                    Err(e) => {
                        println!("   ⚠️  Not a valid GRP file: {}", e);
                        fail_count += 1;
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed: {}", e);
                fail_count += 1;
            }
        }
    }
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Success: {} | ❌ Failed: {}", success_count, fail_count);
    
    Ok(())
}
