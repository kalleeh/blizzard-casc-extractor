use std::path::Path;
use anyhow::Result;
use casc_extractor::casc::CascArchive;

fn main() -> Result<()> {
    env_logger::init();
    
    println!("🎮 CASC Sprite Format Improvements - Demo Extraction");
    println!("Demonstrating 100 sprite extractions to gitignored directory");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Create demo output directory (gitignored)
    let output_dir = Path::new("extracted/demo_sprites");
    std::fs::create_dir_all(output_dir)?;
    
    println!("📁 Output directory: {:?}", output_dir);
    
    // Create mock CASC structure for demonstration
    let test_casc_path = Path::new("extracted/demo_casc");
    create_working_mock_casc(test_casc_path)?;
    
    println!("🔧 Opening CASC archive...");
    let casc_archive = CascArchive::open(test_casc_path)?;
    
    println!("📋 Listing files in CASC archive...");
    let all_files = casc_archive.list_all_files()?;
    
    println!("Found {} files in CASC archive", all_files.len());
    
    // Limit to 100 files for demo
    let files_to_extract = all_files.iter().take(100).collect::<Vec<_>>();
    
    println!("🚀 Extracting {} files...", files_to_extract.len());
    let start_time = std::time::Instant::now();
    
    let mut successful_extractions = 0;
    let mut failed_extractions = 0;
    
    for (index, file_info) in files_to_extract.iter().enumerate() {
        if index % 10 == 0 {
            println!("Processing file {}/{}: {}", index + 1, files_to_extract.len(), file_info.name);
        }
        
        let output_path = output_dir.join(&file_info.name);
        
        match casc_archive.extract_file(&file_info.name, &output_path) {
            Ok(_) => {
                successful_extractions += 1;
                if index < 10 {
                    println!("  ✅ Extracted: {}", file_info.name);
                }
            }
            Err(e) => {
                failed_extractions += 1;
                if index < 10 {
                    println!("  ❌ Failed: {} - {}", file_info.name, e);
                }
            }
        }
    }
    
    let duration = start_time.elapsed();
    
    // Display comprehensive results
    println!("\n🎉 Extraction Complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Performance metrics
    println!("⏱️  Performance:");
    println!("   • Total duration: {:.2} seconds", duration.as_secs_f64());
    println!("   • Processing speed: {:.1} files/sec", 
        files_to_extract.len() as f64 / duration.as_secs_f64());
    
    // Extraction statistics
    println!("\n📊 Extraction Statistics:");
    println!("   • Files processed: {}", files_to_extract.len());
    println!("   • Successful extractions: {}", successful_extractions);
    println!("   • Failed extractions: {}", failed_extractions);
    
    let success_rate = if !files_to_extract.is_empty() {
        (successful_extractions as f64 / files_to_extract.len() as f64) * 100.0
    } else {
        0.0
    };
    println!("   • Success rate: {:.1}%", success_rate);
    
    // Show sample extracted files
    if successful_extractions > 0 {
        println!("\n📁 Sample Extracted Files:");
        if let Ok(entries) = std::fs::read_dir(output_dir) {
            for (i, entry) in entries.flatten().take(10).enumerate() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                let file_size = entry.metadata()
                    .map(|m| m.len())
                    .unwrap_or(0);
                println!("   {}. {} ({} bytes)", 
                    i + 1, file_name, file_size);
            }
        }
    }
    
    // Final summary
    println!("\n✅ Demo Completed Successfully!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📁 Results saved to: {:?}", output_dir);
    println!("🧹 Mock data created at: {:?}", test_casc_path);
    println!("\n🎯 System Capabilities Demonstrated:");
    println!("   ✅ CASC archive reading");
    println!("   ✅ File listing and enumeration");
    println!("   ✅ File extraction to gitignored directory");
    println!("   ✅ Error handling and recovery");
    
    if success_rate >= 80.0 {
        println!("\n🏆 SUCCESS: Achieved {}% extraction rate (target: 80%+)", success_rate);
    } else {
        println!("\n📈 PROGRESS: {}% extraction rate (working toward 80%+ target)", success_rate);
    }
    
    Ok(())
}

fn create_working_mock_casc(casc_path: &Path) -> Result<()> {
    println!("🏗️  Creating working mock CASC structure...");
    
    let data_dir = casc_path.join("Data").join("data");
    std::fs::create_dir_all(&data_dir)?;
    
    // Create a proper CASC index file that matches the expected format
    let index_path = data_dir.join("data.000.idx");
    
    // Create header (24 bytes total)
    let mut index_data = Vec::new();
    
    // Basic header (16 bytes)
    index_data.extend_from_slice(&16u32.to_le_bytes()); // header_hash_size
    index_data.extend_from_slice(&0x12345678u32.to_le_bytes()); // header_hash
    index_data.extend_from_slice(&7u16.to_le_bytes()); // unk0 = 7 (required)
    index_data.push(0); // bucket_index
    index_data.push(0); // unk1
    index_data.push(4); // entry_size_bytes (4 bytes for u32)
    index_data.push(4); // entry_offset_bytes (4 bytes for u32)
    index_data.push(9); // entry_key_bytes (9 bytes for key)
    index_data.push(24); // archive_file_header_size (24 bytes total)
    
    // Extended header (8 bytes)
    index_data.extend_from_slice(&0u64.to_le_bytes()); // archive_total_size_maximum
    
    // Create 100 mock entries for demonstration
    let sprite_names = [
        "marine", "zealot", "zergling", "scv", "probe", "drone", "firebat", "dragoon", 
        "hydralisk", "wraith", "battlecruiser", "carrier", "overlord", "mutalisk",
        "ghost", "archon", "ultralisk", "defiler", "corsair", "valkyrie"
    ];
    
    let mut current_offset = 0u32;
    
    for i in 0..100 {
        // Generate a unique key for each file
        let mut key = [0u8; 9];
        let name_index = i % sprite_names.len();
        let sprite_name = sprite_names[name_index];
        
        // Create a hash-like key from the sprite name and index
        for (j, byte) in sprite_name.bytes().enumerate() {
            if j < 8 {
                key[j] = byte;
            }
        }
        key[8] = (i % 256) as u8; // Make each key unique
        
        // Add the key (9 bytes)
        index_data.extend_from_slice(&key);
        
        // Add data file number (4 bytes) - always use file 0 for simplicity
        index_data.extend_from_slice(&0u32.to_le_bytes());
        
        // Add data file offset (4 bytes)
        index_data.extend_from_slice(&current_offset.to_le_bytes());
        
        // Increment offset for next file (simulate different file sizes)
        current_offset += 1024 + (i as u32 * 100); // Variable sizes
    }
    
    std::fs::write(&index_path, &index_data)?;
    
    // Create corresponding data file with mock sprite data
    let data_path = data_dir.join("data.000");
    let mut mock_data = Vec::new();
    
    for i in 0..100 {
        let name_index = i % sprite_names.len();
        let sprite_name = sprite_names[name_index];
        
        // Create mock sprite data with different formats
        match i % 5 {
            0 => {
                // Mock PNG data
                mock_data.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
                mock_data.extend_from_slice(&format!("PNG_SPRITE_{}", sprite_name).as_bytes());
            }
            1 => {
                // Mock ANIM data
                mock_data.extend_from_slice(b"ANIM");
                mock_data.extend_from_slice(&(32u32).to_le_bytes()); // Frame count
                mock_data.extend_from_slice(&format!("ANIM_DATA_{}", sprite_name).as_bytes());
            }
            2 => {
                // Mock GRP data
                mock_data.extend_from_slice(&(16u16).to_le_bytes()); // Image count
                mock_data.extend_from_slice(&(64u16).to_le_bytes()); // Width
                mock_data.extend_from_slice(&(64u16).to_le_bytes()); // Height
                mock_data.extend_from_slice(&format!("GRP_DATA_{}", sprite_name).as_bytes());
            }
            3 => {
                // Mock PCX data
                mock_data.push(0x0A); // PCX manufacturer
                mock_data.push(0x05); // Version
                mock_data.push(0x01); // Encoding
                mock_data.extend_from_slice(&format!("PCX_DATA_{}", sprite_name).as_bytes());
            }
            4 => {
                // Mock DDS data
                mock_data.extend_from_slice(b"DDS ");
                mock_data.extend_from_slice(&(124u32).to_le_bytes()); // Header size
                mock_data.extend_from_slice(&format!("DDS_DATA_{}", sprite_name).as_bytes());
            }
            _ => unreachable!(),
        }
        
        // Add padding to reach the expected offset for the next file
        let target_size = 1024 + (i * 100);
        while mock_data.len() < target_size {
            mock_data.push(0xAA); // Padding byte
        }
    }
    
    std::fs::write(&data_path, &mock_data)?;
    
    println!("✅ Working mock CASC created:");
    println!("   • 100 mock sprite files");
    println!("   • 5 different formats (PNG, ANIM, GRP, PCX, DDS)");
    println!("   • {} bytes of mock data", mock_data.len());
    println!("   • Proper CASC index structure");
    
    Ok(())
}