use std::path::Path;
use anyhow::Result;
use casc_extractor::casc::CascArchive;

fn main() -> Result<()> {
    env_logger::init();
    
    println!("🔓 StarCraft Decryption Test");
    println!("Testing various decryption methods on real StarCraft data");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let starcraft_path = Path::new("/Applications/StarCraft");
    
    match CascArchive::open(starcraft_path) {
        Ok(casc_archive) => {
            println!("✅ Successfully opened StarCraft CASC archive!");
            
            let files = casc_archive.list_files_with_filter(Some("sprites"))?;
            println!("📊 Found {} potential sprite files", files.len());
            
            // Test first few files with various decryption methods
            for (i, file_info) in files.iter().take(3).enumerate() {
                println!("\n🔍 TESTING FILE #{}: {}", i + 1, file_info.name);
                
                match casc_archive.extract_file_with_analysis(&file_info.key) {
                    Ok((raw_data, analysis)) => {
                        println!("   ✅ Extracted {} bytes", raw_data.len());
                        println!("   📊 Entropy: {:.3}", analysis.entropy);
                        
                        if raw_data.len() > 0 {
                            // Show first 32 bytes
                            println!("   🔢 First 32 bytes: {:02x?}", &raw_data[0..raw_data.len().min(32)]);
                            
                            // Test comprehensive XOR decryption
                            test_comprehensive_xor_decryption(&raw_data, &file_info.name);
                            
                            // Test BLTE patterns
                            test_blte_patterns(&raw_data, &file_info.name);
                            
                            // Test for known StarCraft patterns after decryption
                            test_starcraft_patterns(&raw_data, &file_info.name);
                        }
                    }
                    Err(e) => {
                        println!("   ❌ Failed to extract: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to open StarCraft CASC archive: {}", e);
        }
    }
    
    Ok(())
}

fn test_comprehensive_xor_decryption(data: &[u8], filename: &str) {
    println!("   🔓 Testing XOR decryption...");
    
    // Test all single-byte XOR keys
    for key in 0x00..=0xFF {
        let mut decrypted = data.to_vec();
        for byte in &mut decrypted {
            *byte ^= key;
        }
        
        // Check for known format signatures after decryption
        if decrypted.len() >= 4 {
            let signature = &decrypted[0..4];
            
            // Check for known signatures
            if signature == b"ANIM" {
                println!("      ✅ Found ANIM signature with XOR key 0x{:02x}!", key);
                save_decrypted_data(&decrypted, filename, key);
            } else if signature == b"DDS " {
                println!("      ✅ Found DDS signature with XOR key 0x{:02x}!", key);
                save_decrypted_data(&decrypted, filename, key);
            } else if signature[0..2] == *b"BM" {
                println!("      ✅ Found BMP signature with XOR key 0x{:02x}!", key);
                save_decrypted_data(&decrypted, filename, key);
            } else if signature == b"RIFF" {
                println!("      ✅ Found RIFF signature with XOR key 0x{:02x}!", key);
                save_decrypted_data(&decrypted, filename, key);
            }
            
            // Check for ZLIB header
            if decrypted[0] == 0x78 && (decrypted[1] == 0x01 || decrypted[1] == 0x9C || decrypted[1] == 0xDA) {
                println!("      ✅ Found ZLIB header with XOR key 0x{:02x}!", key);
                save_decrypted_data(&decrypted, filename, key);
            }
            
            // Check for GRP format (StarCraft sprite format)
            if decrypted.len() >= 6 {
                let frame_count = u16::from_le_bytes([decrypted[0], decrypted[1]]);
                let width = u16::from_le_bytes([decrypted[2], decrypted[3]]);
                let height = u16::from_le_bytes([decrypted[4], decrypted[5]]);
                
                if frame_count > 0 && frame_count <= 256 && 
                   width > 0 && width <= 1024 && 
                   height > 0 && height <= 1024 {
                    println!("      ✅ Found potential GRP format with XOR key 0x{:02x}: {}x{}, {} frames", 
                            key, width, height, frame_count);
                    save_decrypted_data(&decrypted, filename, key);
                }
            }
        }
    }
}

fn test_blte_patterns(data: &[u8], filename: &str) {
    println!("   🔧 Testing BLTE patterns...");
    
    // Check if data starts with BLTE signature
    if data.len() >= 4 && &data[0..4] == b"BLTE" {
        println!("      ✅ Found BLTE signature!");
    }
    
    // Check for encrypted BLTE (high entropy, specific patterns)
    if data.len() >= 64 {
        let mut byte_counts = [0u32; 256];
        let sample_size = data.len().min(1024);
        for &byte in &data[0..sample_size] {
            byte_counts[byte as usize] += 1;
        }
        
        let unique_bytes = byte_counts.iter().filter(|&&count| count > 0).count();
        let entropy_ratio = unique_bytes as f64 / 256.0;
        
        if entropy_ratio > 0.97 {
            println!("      ⚠️  Very high entropy ({:.3}) - likely encrypted BLTE", entropy_ratio);
            
            // Try to decrypt with BLTE library
            match try_blte_decryption(data) {
                Ok(decrypted) => {
                    println!("      ✅ BLTE decryption successful: {} -> {} bytes", data.len(), decrypted.len());
                    save_decrypted_data(&decrypted, filename, 0xBB); // Use 0xBB as marker for BLTE
                }
                Err(e) => {
                    println!("      ❌ BLTE decryption failed: {}", e);
                }
            }
        }
    }
}

fn test_starcraft_patterns(data: &[u8], filename: &str) {
    println!("   🎮 Testing StarCraft-specific patterns...");
    
    // Test multi-byte XOR keys specific to StarCraft
    let starcraft_keys = [
        vec![0x53, 0x43, 0x52], // "SCR" - StarCraft Remastered
        vec![0x42, 0x4C, 0x5A], // "BLZ" - Blizzard
        vec![0x43, 0x41, 0x53, 0x43], // "CASC"
        vec![0x53, 0x74, 0x61, 0x72], // "Star"
        vec![0x43, 0x72, 0x61, 0x66, 0x74], // "Craft"
    ];
    
    for (i, key) in starcraft_keys.iter().enumerate() {
        let mut decrypted = data.to_vec();
        
        // Multi-byte XOR decryption
        for (j, byte) in decrypted.iter_mut().enumerate() {
            *byte ^= key[j % key.len()];
        }
        
        // Check if decryption looks successful
        if looks_like_valid_starcraft_data(&decrypted) {
            println!("      ✅ Multi-byte key #{} ({:02x?}) looks promising!", i + 1, key);
            save_decrypted_data(&decrypted, filename, 0xAA + i as u8); // Use 0xAA+ as marker for multi-byte
        }
    }
}

fn try_blte_decryption(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use blte::decompress_blte;
    use ngdp_crypto::KeyService;
    
    // Try without encryption first
    match decompress_blte(data.to_vec(), None) {
        Ok(decompressed) => return Ok(decompressed),
        Err(_) => {}
    }
    
    // Try with key service
    let key_service = KeyService::new();
    match decompress_blte(data.to_vec(), Some(&key_service)) {
        Ok(decompressed) => Ok(decompressed),
        Err(e) => Err(Box::new(e)),
    }
}

fn looks_like_valid_starcraft_data(data: &[u8]) -> bool {
    if data.len() < 16 {
        return false;
    }
    
    // Check for known StarCraft format signatures
    if &data[0..4] == b"ANIM" || &data[0..4] == b"DDS " || &data[0..2] == b"BM" {
        return true;
    }
    
    // Check for ZLIB header
    if data[0] == 0x78 && (data[1] == 0x01 || data[1] == 0x9C || data[1] == 0xDA) {
        return true;
    }
    
    // Check for GRP format
    if data.len() >= 6 {
        let frame_count = u16::from_le_bytes([data[0], data[1]]);
        let width = u16::from_le_bytes([data[2], data[3]]);
        let height = u16::from_le_bytes([data[4], data[5]]);
        
        if frame_count > 0 && frame_count <= 256 && 
           width > 0 && width <= 1024 && 
           height > 0 && height <= 1024 {
            return true;
        }
    }
    
    // Check for reasonable entropy (not too high, not too low)
    let mut byte_counts = [0u32; 256];
    let sample_size = data.len().min(256);
    for &byte in &data[0..sample_size] {
        byte_counts[byte as usize] += 1;
    }
    
    let unique_bytes = byte_counts.iter().filter(|&&count| count > 0).count();
    let entropy_ratio = unique_bytes as f64 / 256.0;
    
    // Valid decrypted data should have moderate entropy
    entropy_ratio > 0.1 && entropy_ratio < 0.8
}

fn save_decrypted_data(data: &[u8], filename: &str, key: u8) {
    let output_path = format!("extracted/decrypted_{}_{:02x}.dat", filename.replace(".dat", ""), key);
    if let Err(e) = std::fs::write(&output_path, data) {
        println!("      ❌ Failed to save decrypted data: {}", e);
    } else {
        println!("      💾 Saved decrypted data to: {}", output_path);
        
        // Show first 32 bytes of decrypted data
        if data.len() >= 32 {
            println!("      🔢 Decrypted first 32 bytes: {:02x?}", &data[0..32]);
        }
    }
}