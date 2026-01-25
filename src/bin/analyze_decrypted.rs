use std::path::Path;
use std::fs;
use anyhow::Result;

fn main() -> Result<()> {
    println!("🔍 Analyzing Decrypted StarCraft Files");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let extracted_dir = Path::new("extracted");
    
    if !extracted_dir.exists() {
        println!("❌ Extracted directory not found");
        return Ok(());
    }
    
    // Find all decrypted files
    let decrypted_files: Vec<_> = fs::read_dir(extracted_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_name().to_string_lossy().starts_with("decrypted_")
        })
        .collect();
    
    println!("📊 Found {} decrypted files", decrypted_files.len());
    
    for (i, entry) in decrypted_files.iter().enumerate() {
        let path = entry.path();
        let filename_os = entry.file_name();
        let filename = filename_os.to_string_lossy();
        
        println!("\n🔍 ANALYZING FILE #{}: {}", i + 1, filename);
        
        match fs::read(&path) {
            Ok(data) => {
                println!("   ✅ Read {} bytes", data.len());
                
                if data.len() >= 32 {
                    println!("   🔢 First 32 bytes: {:02x?}", &data[0..32]);
                    
                    // Check for known format signatures
                    analyze_format_signatures(&data);
                    
                    // Check for StarCraft-specific patterns
                    analyze_starcraft_patterns(&data);
                    
                    // Calculate entropy
                    let entropy = calculate_entropy(&data);
                    println!("   📊 Entropy: {:.3}", entropy);
                    
                    // Try to interpret as different formats
                    try_format_interpretations(&data, &filename);
                }
            }
            Err(e) => {
                println!("   ❌ Failed to read file: {}", e);
            }
        }
    }
    
    Ok(())
}

fn analyze_format_signatures(data: &[u8]) {
    println!("   🎯 Format Signature Analysis:");
    
    if data.len() >= 4 {
        let first_4 = &data[0..4];
        match first_4 {
            b"ANIM" => println!("      ✅ ANIM format detected!"),
            b"DDS " => println!("      ✅ DDS texture format detected!"),
            b"RIFF" => println!("      ✅ RIFF format detected!"),
            _ => {
                let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                println!("      ❓ Unknown signature: {:02x?} (magic: 0x{:08x})", first_4, magic);
            }
        }
    }
    
    // Check for BMP signature
    if data.len() >= 2 && &data[0..2] == b"BM" {
        println!("      ✅ BMP format detected!");
    }
    
    // Check for PNG signature
    if data.len() >= 8 && &data[0..8] == b"\x89PNG\r\n\x1a\n" {
        println!("      ✅ PNG format detected!");
    }
    
    // Check for JPEG signature
    if data.len() >= 2 && &data[0..2] == b"\xFF\xD8" {
        println!("      ✅ JPEG format detected!");
    }
    
    // Check for ZLIB header
    if data.len() >= 2 && data[0] == 0x78 && (data[1] == 0x01 || data[1] == 0x9C || data[1] == 0xDA) {
        println!("      ✅ ZLIB compressed data detected!");
    }
}

fn analyze_starcraft_patterns(data: &[u8]) {
    println!("   🎮 StarCraft Pattern Analysis:");
    
    // Check for GRP format (StarCraft sprite format)
    if data.len() >= 6 {
        let frame_count = u16::from_le_bytes([data[0], data[1]]);
        let width = u16::from_le_bytes([data[2], data[3]]);
        let height = u16::from_le_bytes([data[4], data[5]]);
        
        if frame_count > 0 && frame_count <= 256 && 
           width > 0 && width <= 1024 && 
           height > 0 && height <= 1024 {
            println!("      ✅ Potential GRP format: {}x{} pixels, {} frames", width, height, frame_count);
        } else {
            println!("      ❌ Not GRP format: frames={}, {}x{}", frame_count, width, height);
        }
    }
    
    // Check for potential texture dimensions in various positions
    for offset in [0, 4, 8, 12, 16] {
        if offset + 8 <= data.len() {
            let width = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
            let height = u32::from_le_bytes([data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]]);
            
            if width > 0 && width <= 2048 && height > 0 && height <= 2048 {
                let expected_size = (width * height) as usize;
                if data.len() >= expected_size + offset + 8 {
                    println!("      ✅ Potential texture at offset {}: {}x{} (expected size: {})", 
                            offset, width, height, expected_size);
                }
            }
        }
    }
}

fn calculate_entropy(data: &[u8]) -> f64 {
    let mut byte_counts = [0u32; 256];
    for &byte in data {
        byte_counts[byte as usize] += 1;
    }
    
    let len = data.len() as f64;
    let mut entropy = 0.0;
    
    for &count in &byte_counts {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }
    
    entropy
}

fn try_format_interpretations(data: &[u8], filename: &str) {
    println!("   🔬 Format Interpretation Attempts:");
    
    // Try to save as different formats for manual inspection
    let base_name = filename.replace(".dat", "");
    
    // Save first 1024 bytes as a separate file for hex analysis
    if data.len() >= 1024 {
        let hex_sample = &data[0..1024];
        let hex_filename = format!("extracted/{}_sample.bin", base_name);
        if let Err(e) = std::fs::write(&hex_filename, hex_sample) {
            println!("      ⚠️  Failed to save hex sample: {}", e);
        } else {
            println!("      💾 Saved 1024-byte sample to: {}", hex_filename);
        }
    }
    
    // Try to interpret as raw image data with common dimensions
    let common_dimensions = [(32, 32), (64, 64), (128, 128), (256, 256), (64, 48), (128, 96)];
    
    for &(width, height) in &common_dimensions {
        let pixel_count = width * height;
        
        if data.len() >= pixel_count {
            println!("      🖼️  Could be {}x{} raw image ({} bytes needed, {} available)", 
                    width, height, pixel_count, data.len());
        }
    }
    
    // CRITICAL: Always try ZLIB decompression on decrypted data
    // StarCraft: Remastered uses Encrypt -> Compress -> Encrypt pattern
    println!("      🔧 Attempting ZLIB decompression on decrypted data...");
    
    match try_zlib_decompression(data) {
        Ok(decompressed) => {
            println!("      ✅ ZLIB decompression successful: {} -> {} bytes", data.len(), decompressed.len());
            
            let decompressed_filename = format!("extracted/{}_decompressed.bin", base_name);
            if let Err(e) = std::fs::write(&decompressed_filename, &decompressed) {
                println!("      ⚠️  Failed to save decompressed data: {}", e);
            } else {
                println!("      💾 Saved decompressed data to: {}", decompressed_filename);
                
                // Analyze the decompressed data
                if decompressed.len() >= 32 {
                    println!("      🔢 Decompressed first 32 bytes: {:02x?}", &decompressed[0..32]);
                    
                    // Check entropy of decompressed data
                    let decompressed_entropy = calculate_entropy(&decompressed);
                    println!("      📊 Decompressed entropy: {:.3}", decompressed_entropy);
                    
                    // Analyze format signatures in decompressed data
                    println!("      🎯 Decompressed Format Analysis:");
                    analyze_format_signatures(&decompressed);
                    analyze_starcraft_patterns(&decompressed);
                    
                    // Try to convert to PNG if it looks like sprite data
                    try_sprite_conversion(&decompressed, &base_name);
                }
            }
        }
        Err(e) => {
            println!("      ❌ ZLIB decompression failed: {}", e);
            
            // Try alternative decompression methods
            match try_alternative_decompression_methods(data) {
                Ok(decompressed) => {
                    println!("      ✅ Alternative decompression successful: {} -> {} bytes", data.len(), decompressed.len());
                    
                    let decompressed_filename = format!("extracted/{}_alt_decompressed.bin", base_name);
                    if std::fs::write(&decompressed_filename, &decompressed).is_ok() {
                        println!("      💾 Saved alternative decompressed data to: {}", decompressed_filename);
                        
                        if decompressed.len() >= 32 {
                            println!("      🔢 Alternative decompressed first 32 bytes: {:02x?}", &decompressed[0..32]);
                            analyze_format_signatures(&decompressed);
                        }
                    }
                }
                Err(e) => {
                    println!("      ❌ All alternative decompression methods failed: {}", e);
                    
                    // Final attempt: Try to interpret as a different encoding
                    println!("      🔧 Trying encoding detection...");
                    try_encoding_detection(data, &base_name);
                }
            }
        }
    }
}

fn try_zlib_decompression(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::io::Read;
    use flate2::read::ZlibDecoder;
    
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

fn try_alternative_decompression_methods(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    println!("      🔧 Trying alternative decompression methods...");
    
    // 1. Try raw DEFLATE (without ZLIB wrapper)
    if let Ok(result) = try_raw_deflate_decompression(data) {
        println!("      ✅ Raw DEFLATE decompression successful!");
        return Ok(result);
    }
    
    // 2. Try GZIP decompression
    if let Ok(result) = try_gzip_decompression(data) {
        println!("      ✅ GZIP decompression successful!");
        return Ok(result);
    }
    
    // 3. Try LZ4 decompression (common in game engines)
    if let Ok(result) = try_lz4_decompression(data) {
        println!("      ✅ LZ4 decompression successful!");
        return Ok(result);
    }
    
    // 4. Try LZMA decompression
    if let Ok(result) = try_lzma_decompression(data) {
        println!("      ✅ LZMA decompression successful!");
        return Ok(result);
    }
    
    // 5. Try Brotli decompression
    if let Ok(result) = try_brotli_decompression(data) {
        println!("      ✅ Brotli decompression successful!");
        return Ok(result);
    }
    
    // 6. Try custom StarCraft compression (if it exists)
    if let Ok(result) = try_starcraft_custom_decompression(data) {
        println!("      ✅ StarCraft custom decompression successful!");
        return Ok(result);
    }
    
    Err("All alternative decompression methods failed".into())
}

fn try_raw_deflate_decompression(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::io::Read;
    use flate2::read::DeflateDecoder;
    
    let mut decoder = DeflateDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

fn try_gzip_decompression(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::io::Read;
    use flate2::read::GzDecoder;
    
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

fn try_lz4_decompression(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // LZ4 is not in our current dependencies, but we can add it
    // For now, return an error
    Err("LZ4 decompression not implemented yet".into())
}

fn try_lzma_decompression(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // LZMA is not in our current dependencies, but we can add it
    // For now, return an error
    Err("LZMA decompression not implemented yet".into())
}

fn try_brotli_decompression(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Brotli is not in our current dependencies, but we can add it
    // For now, return an error
    Err("Brotli decompression not implemented yet".into())
}

fn try_starcraft_custom_decompression(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Try to detect and handle StarCraft-specific compression
    // This might involve custom algorithms or proprietary formats
    
    // Check for potential custom headers or patterns
    if data.len() >= 16 {
        // Look for patterns that might indicate custom compression
        let header = &data[0..16];
        
        // Check for potential size headers (common in custom formats)
        let potential_uncompressed_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let potential_compressed_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        
        if potential_uncompressed_size > 0 && potential_uncompressed_size < 10_000_000 &&
           potential_compressed_size > 0 && potential_compressed_size < data.len() as u32 {
            println!("      🔍 Potential custom format detected: uncompressed={}, compressed={}", 
                    potential_uncompressed_size, potential_compressed_size);
            
            // Try to extract the compressed data portion
            let compressed_start = 8; // Skip size headers
            if compressed_start < data.len() {
                let compressed_data = &data[compressed_start..];
                
                // Try various decompression methods on the extracted data
                if let Ok(result) = try_raw_deflate_decompression(compressed_data) {
                    return Ok(result);
                }
                
                if let Ok(result) = try_zlib_decompression(compressed_data) {
                    return Ok(result);
                }
            }
        }
    }
    
    Err("No custom StarCraft compression pattern detected".into())
}

fn try_deflate_decompression(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    try_raw_deflate_decompression(data)
}

fn try_encoding_detection(data: &[u8], base_name: &str) {
    println!("      🔍 Analyzing data patterns for encoding detection...");
    
    // Check for patterns that might indicate different encodings or formats
    if data.len() >= 64 {
        // 1. Check for Base64 patterns
        if is_likely_base64(data) {
            println!("      🔍 Data might be Base64 encoded");
            if let Ok(decoded) = try_base64_decode(data) {
                println!("      ✅ Base64 decoding successful: {} -> {} bytes", data.len(), decoded.len());
                let decoded_filename = format!("extracted/{}_base64_decoded.bin", base_name);
                if std::fs::write(&decoded_filename, &decoded).is_ok() {
                    println!("      💾 Saved Base64 decoded data to: {}", decoded_filename);
                    if decoded.len() >= 32 {
                        println!("      🔢 Base64 decoded first 32 bytes: {:02x?}", &decoded[0..32]);
                        analyze_format_signatures(&decoded);
                    }
                }
            }
        }
        
        // 2. Check for hexadecimal encoding
        if is_likely_hex_encoded(data) {
            println!("      🔍 Data might be hexadecimal encoded");
            if let Ok(decoded) = try_hex_decode(data) {
                println!("      ✅ Hex decoding successful: {} -> {} bytes", data.len(), decoded.len());
                let decoded_filename = format!("extracted/{}_hex_decoded.bin", base_name);
                if std::fs::write(&decoded_filename, &decoded).is_ok() {
                    println!("      💾 Saved hex decoded data to: {}", decoded_filename);
                    if decoded.len() >= 32 {
                        println!("      🔢 Hex decoded first 32 bytes: {:02x?}", &decoded[0..32]);
                        analyze_format_signatures(&decoded);
                    }
                }
            }
        }
        
        // 3. Check for potential bit-level encoding
        if has_unusual_bit_patterns(data) {
            println!("      🔍 Data has unusual bit patterns, might need bit-level decoding");
            try_bit_level_analysis(data, base_name);
        }
        
        // 4. Check for potential multi-layer encryption
        println!("      🔍 Trying multi-layer decryption...");
        try_multi_layer_decryption(data, base_name);
    }
}

fn is_likely_base64(data: &[u8]) -> bool {
    // Check if data contains mostly Base64 characters
    let base64_chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
    let sample_size = data.len().min(256);
    let base64_count = data[0..sample_size].iter()
        .filter(|&&b| base64_chars.contains(&b))
        .count();
    
    base64_count as f64 / sample_size as f64 > 0.8
}

fn try_base64_decode(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use base64::{Engine as _, engine::general_purpose};
    
    let data_str = std::str::from_utf8(data)?;
    let decoded = general_purpose::STANDARD.decode(data_str)?;
    Ok(decoded)
}

fn is_likely_hex_encoded(data: &[u8]) -> bool {
    // Check if data contains mostly hexadecimal characters
    let hex_chars = b"0123456789ABCDEFabcdef";
    let sample_size = data.len().min(256);
    let hex_count = data[0..sample_size].iter()
        .filter(|&&b| hex_chars.contains(&b))
        .count();
    
    hex_count as f64 / sample_size as f64 > 0.8
}

fn try_hex_decode(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let data_str = std::str::from_utf8(data)?;
    let cleaned = data_str.chars().filter(|c| c.is_ascii_hexdigit()).collect::<String>();
    
    if cleaned.len() % 2 != 0 {
        return Err("Odd number of hex characters".into());
    }
    
    let mut decoded = Vec::new();
    for chunk in cleaned.as_bytes().chunks(2) {
        let hex_str = std::str::from_utf8(chunk)?;
        let byte = u8::from_str_radix(hex_str, 16)?;
        decoded.push(byte);
    }
    
    Ok(decoded)
}

fn has_unusual_bit_patterns(data: &[u8]) -> bool {
    // Check for patterns that might indicate bit-level encoding
    let sample_size = data.len().min(256);
    let mut bit_counts = [0u32; 8];
    
    for &byte in &data[0..sample_size] {
        for i in 0..8 {
            if (byte >> i) & 1 == 1 {
                bit_counts[i] += 1;
            }
        }
    }
    
    // Check if bit distribution is very uneven (might indicate bit-level encoding)
    let total_bits = sample_size * 8;
    let expected_per_bit = total_bits / 8;
    
    bit_counts.iter().any(|&count| {
        let diff = if count > expected_per_bit as u32 {
            count - expected_per_bit as u32
        } else {
            expected_per_bit as u32 - count
        };
        diff > expected_per_bit as u32 / 4 // More than 25% deviation
    })
}

fn try_bit_level_analysis(data: &[u8], base_name: &str) {
    println!("      🔧 Performing bit-level analysis...");
    
    // Try bit reversal
    let mut bit_reversed = data.to_vec();
    for byte in &mut bit_reversed {
        *byte = byte.reverse_bits();
    }
    
    let bit_reversed_filename = format!("extracted/{}_bit_reversed.bin", base_name);
    if std::fs::write(&bit_reversed_filename, &bit_reversed).is_ok() {
        println!("      💾 Saved bit-reversed data to: {}", bit_reversed_filename);
        if bit_reversed.len() >= 32 {
            println!("      🔢 Bit-reversed first 32 bytes: {:02x?}", &bit_reversed[0..32]);
            analyze_format_signatures(&bit_reversed);
        }
    }
    
    // Try byte order reversal
    let mut byte_reversed = data.to_vec();
    byte_reversed.reverse();
    
    let byte_reversed_filename = format!("extracted/{}_byte_reversed.bin", base_name);
    if std::fs::write(&byte_reversed_filename, &byte_reversed).is_ok() {
        println!("      💾 Saved byte-reversed data to: {}", byte_reversed_filename);
    }
}

fn try_multi_layer_decryption(data: &[u8], base_name: &str) {
    // Try applying multiple XOR keys in sequence
    let key_sequences = [
        vec![0x42, 0x24], // Apply 0x42, then 0x24
        vec![0x53, 0x43, 0x52, 0x24], // "SCR" then 0x24
        vec![0x24, 0x53, 0x43, 0x52], // 0x24 then "SCR"
    ];
    
    for (i, key_sequence) in key_sequences.iter().enumerate() {
        let mut decrypted = data.to_vec();
        
        // Apply keys in sequence
        for &key in key_sequence {
            for byte in &mut decrypted {
                *byte ^= key;
            }
        }
        
        // Check if this looks better
        let entropy = calculate_entropy(&decrypted);
        if entropy < 7.0 { // Lower entropy might indicate successful decryption
            println!("      ✅ Multi-layer decryption #{} reduced entropy to {:.3}", i + 1, entropy);
            
            let multi_decrypted_filename = format!("extracted/{}_multi_decrypt_{}.bin", base_name, i + 1);
            if std::fs::write(&multi_decrypted_filename, &decrypted).is_ok() {
                println!("      💾 Saved multi-layer decrypted data to: {}", multi_decrypted_filename);
                if decrypted.len() >= 32 {
                    println!("      🔢 Multi-layer decrypted first 32 bytes: {:02x?}", &decrypted[0..32]);
                    analyze_format_signatures(&decrypted);
                }
            }
        }
    }
}

fn try_sprite_conversion(data: &[u8], base_name: &str) {
    println!("      🖼️  Attempting sprite conversion...");
    
    // Try to interpret as GRP format (StarCraft sprite format)
    if data.len() >= 6 {
        let frame_count = u16::from_le_bytes([data[0], data[1]]);
        let width = u16::from_le_bytes([data[2], data[3]]);
        let height = u16::from_le_bytes([data[4], data[5]]);
        
        if frame_count > 0 && frame_count <= 256 && 
           width > 0 && width <= 1024 && 
           height > 0 && height <= 1024 {
            println!("      ✅ Valid GRP format detected: {}x{} pixels, {} frames", width, height, frame_count);
            
            // Try to extract first frame
            match extract_grp_frame(data, width, height, frame_count) {
                Ok(png_data) => {
                    let png_filename = format!("extracted/{}_grp_frame0.png", base_name);
                    if std::fs::write(&png_filename, &png_data).is_ok() {
                        println!("      🎉 Successfully converted GRP to PNG: {}", png_filename);
                    }
                }
                Err(e) => {
                    println!("      ❌ GRP conversion failed: {}", e);
                }
            }
        }
    }
    
    // Try to interpret as ANIM format
    if data.len() >= 4 && &data[0..4] == b"ANIM" {
        println!("      ✅ ANIM format detected!");
        
        // Try to extract ANIM data
        match extract_anim_data(data) {
            Ok(png_data) => {
                let png_filename = format!("extracted/{}_anim.png", base_name);
                if std::fs::write(&png_filename, &png_data).is_ok() {
                    println!("      🎉 Successfully converted ANIM to PNG: {}", png_filename);
                }
            }
            Err(e) => {
                println!("      ❌ ANIM conversion failed: {}", e);
            }
        }
    }
    
    // Try to interpret as raw image data with common dimensions
    let common_dimensions = [(32, 32), (64, 64), (128, 128), (256, 256), (64, 48), (128, 96)];
    
    for &(width, height) in &common_dimensions {
        let pixel_count = width * height;
        
        if data.len() >= pixel_count {
            println!("      🔍 Trying {}x{} raw image interpretation...", width, height);
            
            match create_png_from_raw_data(data, width as u32, height as u32) {
                Ok(png_data) => {
                    let png_filename = format!("extracted/{}_raw_{}x{}.png", base_name, width, height);
                    if std::fs::write(&png_filename, &png_data).is_ok() {
                        println!("      ✅ Created raw image PNG: {}", png_filename);
                        break; // Only create one successful interpretation
                    }
                }
                Err(_) => {
                    // Continue trying other dimensions
                }
            }
        }
    }
}

fn extract_grp_frame(data: &[u8], width: u16, height: u16, frame_count: u16) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Simplified GRP extraction - extract first frame
    let header_size = 6 + (frame_count as usize * 8); // 6 byte header + frame offset table
    
    if data.len() < header_size {
        return Err("GRP data too small for header".into());
    }
    
    // Read first frame offset
    let first_frame_offset = u32::from_le_bytes([
        data[6], data[7], data[8], data[9]
    ]) as usize;
    
    if first_frame_offset >= data.len() {
        return Err("Invalid frame offset".into());
    }
    
    // For now, create a simple grayscale PNG from available data
    let pixel_count = (width as usize) * (height as usize);
    let mut pixel_data = vec![128u8; pixel_count]; // Gray background
    
    // Use some frame data if available
    let available_data = &data[first_frame_offset..];
    let copy_len = available_data.len().min(pixel_count);
    pixel_data[0..copy_len].copy_from_slice(&available_data[0..copy_len]);
    
    create_png_from_raw_data(&pixel_data, width as u32, height as u32)
}

fn extract_anim_data(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Simplified ANIM extraction - create a diagnostic image
    println!("      📊 ANIM file size: {} bytes", data.len());
    
    // Create a 64x64 diagnostic image showing the ANIM data
    let width = 64u32;
    let height = 64u32;
    let pixel_count = (width * height) as usize;
    let mut pixel_data = vec![0u8; pixel_count];
    
    // Map ANIM bytes to pixels
    for (i, &byte) in data.iter().skip(4).take(pixel_count).enumerate() {
        pixel_data[i] = byte;
    }
    
    create_png_from_raw_data(&pixel_data, width, height)
}

fn create_png_from_raw_data(pixel_data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::io::Cursor;
    
    let mut png_data = Vec::new();
    let mut cursor = Cursor::new(&mut png_data);
    
    // Create PNG encoder
    let mut encoder = png::Encoder::new(&mut cursor, width, height);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    
    let mut writer = encoder.write_header()?;
    
    // Ensure we have enough pixel data
    let expected_size = (width * height) as usize;
    let mut final_pixel_data = vec![0u8; expected_size];
    let copy_len = pixel_data.len().min(expected_size);
    final_pixel_data[0..copy_len].copy_from_slice(&pixel_data[0..copy_len]);
    
    writer.write_image_data(&final_pixel_data)?;
    writer.finish()?;
    
    Ok(png_data)
}