use std::path::Path;
use anyhow::Result;
use casc_extractor::casc::CascArchive;

fn main() -> Result<()> {
    env_logger::init();
    
    println!("🔍 StarCraft: Remastered Format Analyzer");
    println!("Analyzing actual file formats in your StarCraft installation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let starcraft_path = Path::new("/Applications/StarCraft");
    
    println!("📁 StarCraft installation: {:?}", starcraft_path);
    
    // Try to open your actual StarCraft CASC archive
    println!("🔧 Opening real StarCraft CASC archive...");
    let casc_archive = CascArchive::open(starcraft_path)?;
    println!("✅ Successfully opened StarCraft CASC archive!");
    
    // Get all files and analyze their formats
    let files = casc_archive.list_files_with_filter(None)?;
    println!("📊 Found {} total files in CASC archive", files.len());
    
    // Analyze file extensions and patterns
    let mut extension_counts = std::collections::HashMap::new();
    
    for file_info in &files {
        // Count extensions
        if let Some(ext) = Path::new(&file_info.name).extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            *extension_counts.entry(ext_str).or_insert(0) += 1;
        } else {
            *extension_counts.entry("(no extension)".to_string()).or_insert(0) += 1;
        }
    }
    
    // Sort extensions by frequency
    let mut ext_vec: Vec<_> = extension_counts.iter().collect();
    ext_vec.sort_by(|a, b| b.1.cmp(a.1));
    
    println!("\n📈 File Extension Analysis:");
    for (ext, count) in ext_vec.iter().take(20) {
        println!("   {} files: .{}", count, ext);
    }
    
    println!("\n📊 File Analysis:");
    println!("   Total files: {}", files.len());
    
    // Look for specific StarCraft file patterns
    println!("\n🎮 StarCraft-specific File Analysis:");
    
    let sprite_candidates: Vec<_> = files.iter()
        .filter(|f| {
            let name = f.name.to_lowercase();
            name.contains("sprite") || 
            name.contains("unit") || 
            name.contains("building") || 
            name.contains("marine") ||
            name.contains("zergling") ||
            name.contains("texture") ||
            name.ends_with(".grp") ||
            name.ends_with(".anim") ||
            name.ends_with(".dds") ||
            name.ends_with(".pcx")
        })
        .collect();
    
    println!("   Potential sprite files: {}", sprite_candidates.len());
    
    if !sprite_candidates.is_empty() {
        println!("   Sample sprite files:");
        for file in sprite_candidates.iter().take(10) {
            println!("     {}", file.name);
        }
    }
    
    // Analyze actual file content for a few samples
    println!("\n🔬 Content Analysis (first 5 files):");
    
    for (i, file_info) in files.iter().take(5).enumerate() {
        println!("\n   File {}: {}", i + 1, file_info.name);
        
        match casc_archive.extract_file_with_analysis(&file_info.key) {
            Ok((data, analysis)) => {
                println!("     Entropy: {:.3}", analysis.entropy);
                println!("     PNG signature: {}", analysis.has_png_signature);
                println!("     JPEG signature: {}", analysis.has_jpeg_signature);
                
                if data.len() >= 16 {
                    println!("     First 16 bytes: {:02x?}", &data[0..16]);
                    
                    // Check for common format signatures
                    if data.len() >= 4 {
                        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                        match magic {
                            0x20534444 => println!("     ✅ DDS texture format detected"),
                            0x4D494E41 => println!("     ✅ ANIM format detected"),
                            0x454C5442 => println!("     ✅ BLTE compressed format detected"),
                            _ => {
                                // Check for other signatures
                                if &data[0..4] == b"RIFF" {
                                    println!("     ✅ RIFF format detected");
                                } else if data[0] == 0x78 && (data[1] == 0x01 || data[1] == 0x9C || data[1] == 0xDA) {
                                    println!("     ✅ ZLIB compressed data detected");
                                } else {
                                    println!("     ❓ Unknown format (magic: 0x{:08x})", magic);
                                }
                            }
                        }
                    }
                    
                    // Analyze entropy patterns
                    if analysis.entropy > 0.95 {
                        println!("     🔒 Very high entropy - likely encrypted or compressed");
                    } else if analysis.entropy > 0.8 {
                        println!("     📦 High entropy - likely compressed");
                    } else if analysis.entropy < 0.3 {
                        println!("     📄 Low entropy - likely text or simple data");
                    } else {
                        println!("     🖼️ Moderate entropy - likely image or structured data");
                    }
                } else {
                    println!("     ⚠️ File too small for analysis");
                }
            }
            Err(e) => {
                println!("     ❌ Failed to extract: {}", e);
            }
        }
    }
    
    // Look for files that might contain actual sprites
    println!("\n🎯 Searching for Actual Sprite Files:");
    
    let mut found_sprites = 0;
    for file_info in files.iter().take(50) { // Check first 50 files
        if let Ok((data, analysis)) = casc_archive.extract_file_with_analysis(&file_info.key) {
            // Look for files with moderate entropy that might be actual sprites
            if analysis.entropy > 0.3 && analysis.entropy < 0.9 && data.len() > 100 {
                // Check if it looks like image data
                if analysis.has_png_signature || analysis.has_jpeg_signature {
                    println!("   ✅ Found image file: {} (entropy: {:.3})", 
                            file_info.name, analysis.entropy);
                    found_sprites += 1;
                } else if data.len() >= 6 {
                    // Check for GRP format (StarCraft sprite format)
                    let frame_count = u16::from_le_bytes([data[0], data[1]]);
                    let width = u16::from_le_bytes([data[2], data[3]]);
                    let height = u16::from_le_bytes([data[4], data[5]]);
                    
                    if frame_count > 0 && frame_count <= 100 && 
                       width > 0 && width <= 512 && 
                       height > 0 && height <= 512 {
                        println!("   ✅ Potential GRP sprite: {} ({}x{}, {} frames, entropy: {:.3})", 
                                file_info.name, width, height, frame_count, analysis.entropy);
                        found_sprites += 1;
                    }
                }
            }
        }
    }
    
    if found_sprites == 0 {
        println!("   ❌ No obvious sprite files found in first 50 files");
        println!("   💡 StarCraft: Remastered likely uses encrypted/compressed formats");
        println!("   💡 Consider using specialized tools like CascView or stormex");
    } else {
        println!("   🎉 Found {} potential sprite files!", found_sprites);
    }
    
    println!("\n📋 Summary:");
    println!("   • Total files: {}", files.len());
    println!("   • Most common extension: .{} ({} files)", ext_vec[0].0, ext_vec[0].1);
    println!("   • Potential sprites found: {}", found_sprites);
    println!("   • Recommendation: {}", if found_sprites > 0 {
        "Extract and analyze the found sprite files"
    } else {
        "StarCraft: Remastered uses encrypted formats - need specialized decryption"
    });
    
    Ok(())
}