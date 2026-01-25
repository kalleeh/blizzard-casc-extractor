use std::path::Path;
use anyhow::Result;
use casc_extractor::casc::CascArchive;

fn main() -> Result<()> {
    env_logger::init();
    
    println!("🔍 StarCraft Data Diagnostic Tool");
    println!("Analyzing raw data from your StarCraft installation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Use your actual StarCraft installation
    let starcraft_path = Path::new("/Applications/StarCraft");
    
    println!("📁 StarCraft installation: {:?}", starcraft_path);
    
    // Try to open your actual StarCraft CASC archive
    println!("🔧 Opening real StarCraft CASC archive...");
    match CascArchive::open(starcraft_path) {
        Ok(casc_archive) => {
            println!("✅ Successfully opened StarCraft CASC archive!");
            
            // List files and analyze the first few
            let files = casc_archive.list_files_with_filter(Some("sprites"))?;
            println!("📊 Found {} potential sprite files", files.len());
            
            // Analyze first 5 files in detail
            for (index, file_info) in files.iter().take(5).enumerate() {
                println!("\n🔍 ANALYZING FILE #{}: {}", index + 1, file_info.name);
                println!("   Key: {:?}", file_info.key);
                
                match casc_archive.extract_file_with_analysis(&file_info.key) {
                    Ok((raw_data, analysis)) => {
                        println!("   ✅ Successfully extracted {} bytes", raw_data.len());
                        println!("   📊 Analysis:");
                        println!("      • Entropy: {:.3}", analysis.entropy);
                        println!("      • Has PNG signature: {}", analysis.has_png_signature);
                        println!("      • Has JPEG signature: {}", analysis.has_jpeg_signature);
                        println!("      • Detected file type: {:?}", analysis.file_type_detected);
                        
                        // Show first 64 bytes in hex
                        if raw_data.len() >= 64 {
                            println!("   🔢 First 64 bytes (hex):");
                            for (i, chunk) in raw_data[0..64].chunks(16).enumerate() {
                                print!("      {:04x}: ", i * 16);
                                for byte in chunk {
                                    print!("{:02x} ", byte);
                                }
                                // Add ASCII representation
                                print!(" |");
                                for byte in chunk {
                                    if *byte >= 32 && *byte <= 126 {
                                        print!("{}", *byte as char);
                                    } else {
                                        print!(".");
                                    }
                                }
                                println!("|");
                            }
                        } else if !raw_data.is_empty() {
                            println!("   🔢 All {} bytes (hex): {:02x?}", raw_data.len(), raw_data);
                        }
                        
                        // Try to identify what this data might be
                        println!("   🎯 Format Analysis:");
                        
                        // Check for known signatures
                        if raw_data.len() >= 4 {
                            let first_4 = &raw_data[0..4];
                            match first_4 {
                                b"DDS " => println!("      ✅ DDS texture format detected"),
                                b"ANIM" => println!("      ✅ ANIM format detected"),
                                b"BM\x00\x00" => println!("      ✅ BMP format detected"),
                                _ => {
                                    let magic = u32::from_le_bytes([raw_data[0], raw_data[1], raw_data[2], raw_data[3]]);
                                    println!("      ❓ Unknown 4-byte signature: {:02x?} (magic: 0x{:08x})", first_4, magic);
                                }
                            }
                        }
                        
                        // Check if it looks like GRP format
                        if raw_data.len() >= 6 {
                            let frame_count = u16::from_le_bytes([raw_data[0], raw_data[1]]);
                            let width = u16::from_le_bytes([raw_data[2], raw_data[3]]);
                            let height = u16::from_le_bytes([raw_data[4], raw_data[5]]);
                            
                            if frame_count > 0 && frame_count <= 256 && 
                               width > 0 && width <= 1024 && 
                               height > 0 && height <= 1024 {
                                println!("      ✅ Possible GRP format: {}x{} pixels, {} frames", width, height, frame_count);
                            } else {
                                println!("      ❌ Not GRP format: frames={}, {}x{}", frame_count, width, height);
                            }
                        }
                        
                        // Check entropy characteristics
                        if analysis.entropy > 0.95 {
                            println!("      ⚠️  Very high entropy ({:.3}) - likely compressed or encrypted", analysis.entropy);
                        } else if analysis.entropy < 0.3 {
                            println!("      ⚠️  Very low entropy ({:.3}) - likely uniform data or simple pattern", analysis.entropy);
                        } else {
                            println!("      ✅ Moderate entropy ({:.3}) - likely uncompressed structured data", analysis.entropy);
                        }
                        
                        // Check for compression signatures
                        if raw_data.len() >= 2 {
                            if raw_data[0] == 0x78 && (raw_data[1] == 0x01 || raw_data[1] == 0x9C || raw_data[1] == 0xDA) {
                                println!("      ✅ ZLIB compression header detected");
                            }
                        }
                        
                        // Save raw data for manual inspection
                        let raw_filename = format!("extracted/diagnostic_raw_data_{:02}_{}.dat", index + 1, file_info.name.replace("/", "_"));
                        if let Err(e) = std::fs::write(&raw_filename, &raw_data) {
                            println!("      ⚠️  Failed to save raw data: {}", e);
                        } else {
                            println!("      💾 Raw data saved to: {}", raw_filename);
                        }
                        
                    }
                    Err(e) => {
                        println!("   ❌ Failed to extract file: {}", e);
                    }
                }
            }
            
            println!("\n🎯 SUMMARY:");
            println!("   • Analyzed {} files from real StarCraft installation", files.len().min(5));
            println!("   • Raw data files saved to extracted/ directory for manual inspection");
            println!("   • Use hex editors or other tools to analyze the .dat files");
            println!("   • This will help us understand what formats StarCraft: Remastered actually uses");
            
        }
        Err(e) => {
            println!("❌ Failed to open StarCraft CASC archive: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}