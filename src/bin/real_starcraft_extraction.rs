use std::path::Path;
use anyhow::Result;
use casc_extractor::sprite::{DirectSpriteExtractor, UnityConverter};
use casc_extractor::casc::CascArchive;

fn main() -> Result<()> {
    env_logger::init();
    
    println!("🎮 Real StarCraft Sprite Extraction");
    println!("Extracting actual sprites from your StarCraft installation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Use your actual StarCraft installation
    let starcraft_path = Path::new("/Applications/StarCraft");
    let output_dir = Path::new("extracted/real_starcraft_sprites");
    
    println!("📁 StarCraft installation: {:?}", starcraft_path);
    println!("📁 Output directory: {:?}", output_dir);
    
    std::fs::create_dir_all(output_dir)?;
    
    // Try to open your actual StarCraft CASC archive
    println!("🔧 Opening real StarCraft CASC archive...");
    match CascArchive::open(starcraft_path) {
        Ok(casc_archive) => {
            println!("✅ Successfully opened StarCraft CASC archive!");
            
            // Create DirectSpriteExtractor for real data
            println!("🚀 Initializing DirectSpriteExtractor for real StarCraft data...");
            let sprite_extractor = DirectSpriteExtractor::new_with_max_files(
                casc_archive,
                Some(100) // max_files - increase to 100 for better testing
            );
            
            // Create Unity converter
            let unity_converter = UnityConverter {
                pixels_per_unit: 100.0,
                filter_mode: "Bilinear".to_string(),
                wrap_mode: "Clamp".to_string(),
                compression_quality: 75,
                generate_mip_maps: false,
            };
            
            println!("🎨 Starting extraction from REAL StarCraft data...");
            let start_time = std::time::Instant::now();
            
            let result = sprite_extractor.extract_all_sprites_with_unity_support(output_dir, &unity_converter)?;
            
            let duration = start_time.elapsed();
            
            // Display results
            println!("\n🎉 Real StarCraft Sprite Extraction Complete!");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
            println!("⏱️  Performance:");
            println!("   • Total duration: {:.2} seconds", duration.as_secs_f64());
            if result.sprites_extracted > 0 {
                println!("   • Processing speed: {:.1} sprites/sec", 
                    result.sprites_extracted as f64 / duration.as_secs_f64());
            }
            
            println!("\n📊 Real StarCraft Extraction Statistics:");
            println!("   • Sprites processed: {}", result.sprites_extracted);
            println!("   • PNG files generated: {}", count_png_files(output_dir)?);
            println!("   • Unity metadata files: {}", count_meta_files(output_dir)?);
            
            let success_rate = if result.sprites_extracted > 0 {
                (count_png_files(output_dir)? as f64 / result.sprites_extracted as f64) * 100.0
            } else {
                0.0
            };
            println!("   • PNG conversion rate: {:.1}%", success_rate);
            
            // Show generated files
            println!("\n📁 Generated Files from Real StarCraft Data:");
            if let Ok(entries) = std::fs::read_dir(output_dir) {
                let mut png_count = 0;
                
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let file_size = entry.metadata()
                        .map(|m| m.len())
                        .unwrap_or(0);
                    
                    if file_name.ends_with(".png") {
                        png_count += 1;
                        if png_count <= 5 {
                            println!("   🖼️  {} ({} bytes)", file_name, file_size);
                            
                            // Validate PNG
                            let png_path = output_dir.join(&file_name);
                            if let Ok(png_data) = std::fs::read(&png_path) {
                                if png_data.len() >= 8 && &png_data[0..8] == b"\x89PNG\r\n\x1a\n" {
                                    println!("      ✅ Valid PNG from real StarCraft data");
                                } else {
                                    println!("      ⚠️  Invalid PNG signature");
                                }
                            }
                        }
                    }
                }
                
                if png_count > 5 {
                    println!("   ... and {} more PNG files", png_count - 5);
                }
            }
            
            // Show first PNG for user to open
            if let Ok(entries) = std::fs::read_dir(output_dir) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.ends_with(".png") {
                        let full_path = output_dir.join(&file_name);
                        println!("\n🎯 Open this REAL StarCraft sprite:");
                        println!("   open {:?}", full_path);
                        break;
                    }
                }
            }
            
            if success_rate >= 80.0 {
                println!("\n🏆 SUCCESS: Achieved {}% PNG conversion rate from real StarCraft data!", success_rate);
            } else {
                println!("\n📈 PROGRESS: {}% PNG conversion rate from real StarCraft data", success_rate);
            }
        }
        Err(e) => {
            println!("❌ Failed to open StarCraft CASC archive: {}", e);
            println!("\n🔍 Troubleshooting:");
            println!("   • Check that StarCraft is installed at: {:?}", starcraft_path);
            println!("   • Look for Data/ subdirectory with CASC files");
            println!("   • Ensure you have read permissions");
            
            // Try to find the actual CASC data directory
            println!("\n🔍 Searching for CASC data directories...");
            let possible_paths = [
                "/Applications/StarCraft/Data",
                "/Applications/StarCraft/x86_64/Data", 
                "/Applications/StarCraft/StarCraft.app/Contents/Resources/Data",
                "/Applications/StarCraft/x86_64/StarCraft.app/Contents/Resources/Data",
            ];
            
            for path in &possible_paths {
                if Path::new(path).exists() {
                    println!("   ✅ Found potential CASC data at: {}", path);
                    
                    // Try this path
                    match CascArchive::open(Path::new(path)) {
                        Ok(casc_archive) => {
                            println!("   🎉 Successfully opened CASC at: {}", path);
                            
                            let sprite_extractor = DirectSpriteExtractor::new_with_max_files(
                                casc_archive,
                                Some(5)
                            );
                            
                            let unity_converter = UnityConverter::default();
                            
                            println!("   🎨 Extracting from real StarCraft data...");
                            let result = sprite_extractor.extract_all_sprites_with_unity_support(output_dir, &unity_converter)?;
                            
                            println!("   ✅ Extracted {} sprites from real StarCraft!", result.sprites_extracted);
                            
                            if let Ok(entries) = std::fs::read_dir(output_dir) {
                                for entry in entries.flatten() {
                                    let file_name = entry.file_name().to_string_lossy().to_string();
                                    if file_name.ends_with(".png") {
                                        let full_path = output_dir.join(&file_name);
                                        println!("\n🎯 Open this REAL StarCraft sprite:");
                                        println!("   open {:?}", full_path);
                                        break;
                                    }
                                }
                            }
                            
                            return Ok(());
                        }
                        Err(e) => {
                            println!("   ❌ Failed to open CASC at {}: {}", path, e);
                        }
                    }
                } else {
                    println!("   ❌ Not found: {}", path);
                }
            }
            
            return Err(e.into());
        }
    }
    
    Ok(())
}

fn count_png_files(dir: &Path) -> Result<usize> {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().ends_with(".png") {
                count += 1;
            }
        }
    }
    Ok(count)
}

fn count_meta_files(dir: &Path) -> Result<usize> {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".meta") || name.ends_with(".json") {
                count += 1;
            }
        }
    }
    Ok(count)
}