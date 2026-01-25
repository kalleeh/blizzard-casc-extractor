/// Property-based test generators for ANIM and GRP formats
/// 
/// This module provides generators for creating valid ANIM and GRP test data
/// for comprehensive property-based testing using proptest.

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Write;

/// Generate valid ANIM file data with specified parameters
/// 
/// Creates a minimal but valid ANIM file structure that can be parsed
/// by the ANIM parser. The generated data includes:
/// - Valid ANIM header with magic number
/// - Sprite count, texture count, frame count
/// - Texture data with specified compression type
pub fn generate_valid_anim_data(
    sprite_count: u32,
    texture_count: u32,
    frame_count: u32,
    width: u16,
    height: u16,
    compression_type: u8,
) -> Vec<u8> {
    let mut data = Vec::new();
    
    // ANIM magic number: "ANIM" (0x4D494E41 in little-endian)
    data.write_u32::<LittleEndian>(0x4D494E41).unwrap();
    
    // Version number (typically 0x00000001)
    data.write_u32::<LittleEndian>(0x00000001).unwrap();
    
    // Sprite count
    data.write_u32::<LittleEndian>(sprite_count).unwrap();
    
    // Texture count
    data.write_u32::<LittleEndian>(texture_count).unwrap();
    
    // Frame count
    data.write_u32::<LittleEndian>(frame_count).unwrap();
    
    // Animation count (set to 0 for simplicity)
    data.write_u32::<LittleEndian>(0).unwrap();
    
    // Palette count (set to 0 for simplicity)
    data.write_u32::<LittleEndian>(0).unwrap();
    
    // Reserved/flags
    data.write_u32::<LittleEndian>(0).unwrap();
    
    // Add minimal sprite entries (simplified structure)
    for _ in 0..sprite_count {
        // Sprite entry: frame reference, texture reference, metadata
        data.write_u32::<LittleEndian>(0).unwrap(); // frame_ref
        data.write_u32::<LittleEndian>(0).unwrap(); // texture_ref
        data.write_u16::<LittleEndian>(width).unwrap();
        data.write_u16::<LittleEndian>(height).unwrap();
    }
    
    // Add minimal texture entries
    for _ in 0..texture_count {
        // Texture entry header
        data.write_u8(compression_type).unwrap(); // compression type
        data.write_u8(0).unwrap(); // pixel format (RGBA32)
        data.write_u16::<LittleEndian>(width).unwrap();
        data.write_u16::<LittleEndian>(height).unwrap();
        
        // Texture data size
        let pixel_count = (width as usize) * (height as usize);
        let data_size = pixel_count * 4; // RGBA32 = 4 bytes per pixel
        data.write_u32::<LittleEndian>(data_size as u32).unwrap();
        
        // Generate simple texture data (all zeros for simplicity)
        let texture_data = vec![0u8; data_size];
        data.write_all(&texture_data).unwrap();
    }
    
    data
}

/// Generate valid GRP file data with specified parameters
/// 
/// Creates a minimal but valid GRP file structure that can be parsed
/// by the GRP parser. The generated data includes:
/// - Valid GRP header with frame count and dimensions
/// - Frame offset table
/// - RLE-encoded frame data
pub fn generate_valid_grp_data(
    frame_count: u16,
    width: u16,
    height: u16,
) -> Vec<u8> {
    let mut data = Vec::new();
    
    // GRP header
    data.write_u16::<LittleEndian>(frame_count).unwrap();
    data.write_u16::<LittleEndian>(width).unwrap();
    data.write_u16::<LittleEndian>(height).unwrap();
    
    // Generate RLE-encoded frame data for each frame FIRST
    let pixel_count = (width as usize) * (height as usize);
    let mut all_frame_data = Vec::new();
    
    for frame_idx in 0..frame_count {
        let pixel_value = (frame_idx % 256) as u8; // Vary pixel values per frame
        let mut frame_rle = Vec::new();
        
        // Simple RLE encoding: encode all pixels with the same value
        let mut remaining_pixels = pixel_count;
        while remaining_pixels > 0 {
            let run_length = remaining_pixels.min(255); // Max run length is 255
            frame_rle.push(run_length as u8);
            frame_rle.push(pixel_value);
            remaining_pixels -= run_length;
        }
        
        all_frame_data.push(frame_rle);
    }
    
    // Calculate frame offsets based on ACTUAL frame sizes
    let header_size = 6; // 6 bytes for header
    let offset_table_size = (frame_count as usize) * 4; // 4 bytes per offset
    let mut frame_offsets = Vec::new();
    let mut current_offset = header_size + offset_table_size;
    
    for frame_data in &all_frame_data {
        frame_offsets.push(current_offset);
        current_offset += frame_data.len();
    }
    
    // Write frame offset table
    for offset in &frame_offsets {
        data.write_u32::<LittleEndian>(*offset as u32).unwrap();
    }
    
    // Write the actual frame data
    for frame_data in &all_frame_data {
        data.write_all(frame_data).unwrap();
    }
    
    data
}

/// Proptest strategy for generating valid ANIM parameters
pub fn anim_params_strategy() -> impl Strategy<Value = (u32, u32, u32, u16, u16, u8)> {
    (
        1u32..10,      // sprite_count: 1-9 sprites
        1u32..10,      // texture_count: 1-9 textures
        1u32..100,     // frame_count: 1-99 frames
        1u16..512,     // width: 1-511 pixels
        1u16..512,     // height: 1-511 pixels
        0u8..3,        // compression_type: 0=None, 1=Zlib, 2=Lz4
    )
}

/// Proptest strategy for generating valid GRP parameters
pub fn grp_params_strategy() -> impl Strategy<Value = (u16, u16, u16)> {
    (
        1u16..100,     // frame_count: 1-99 frames
        1u16..512,     // width: 1-511 pixels
        1u16..512,     // height: 1-511 pixels
    )
}

/// Test data structure for GRP format
#[derive(Debug, Clone)]
pub struct GrpTestData {
    pub frame_count: u16,
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
}

/// Proptest strategy for generating valid GRP test data
pub fn valid_grp_data() -> impl Strategy<Value = GrpTestData> {
    grp_params_strategy().prop_map(|(frame_count, width, height)| {
        let data = generate_valid_grp_data(frame_count, width, height);
        GrpTestData {
            frame_count,
            width,
            height,
            data,
        }
    })
}

/// Generate compressed texture data using ZLIB
pub fn generate_zlib_compressed_texture(width: u16, height: u16) -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    
    let pixel_count = (width as usize) * (height as usize);
    let uncompressed_data = vec![128u8; pixel_count * 4]; // Gray RGBA pixels
    
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&uncompressed_data).unwrap();
    encoder.finish().unwrap()
}

/// Generate compressed texture data using LZ4
pub fn generate_lz4_compressed_texture(width: u16, height: u16) -> Vec<u8> {
    let pixel_count = (width as usize) * (height as usize);
    let uncompressed_data = vec![128u8; pixel_count * 4]; // Gray RGBA pixels
    
    lz4_flex::compress_prepend_size(&uncompressed_data)
}

/// Generate valid ANIM data with ZLIB compression
pub fn generate_anim_with_zlib_compression(
    sprite_count: u32,
    texture_count: u32,
    frame_count: u32,
    width: u16,
    height: u16,
) -> Vec<u8> {
    let mut data = Vec::new();
    
    // ANIM header
    data.write_u32::<LittleEndian>(0x4D494E41).unwrap(); // Magic
    data.write_u32::<LittleEndian>(0x00000001).unwrap(); // Version
    data.write_u32::<LittleEndian>(sprite_count).unwrap();
    data.write_u32::<LittleEndian>(texture_count).unwrap();
    data.write_u32::<LittleEndian>(frame_count).unwrap();
    data.write_u32::<LittleEndian>(0).unwrap(); // Animation count
    data.write_u32::<LittleEndian>(0).unwrap(); // Palette count
    data.write_u32::<LittleEndian>(0).unwrap(); // Reserved
    
    // Sprite entries
    for _ in 0..sprite_count {
        data.write_u32::<LittleEndian>(0).unwrap();
        data.write_u32::<LittleEndian>(0).unwrap();
        data.write_u16::<LittleEndian>(width).unwrap();
        data.write_u16::<LittleEndian>(height).unwrap();
    }
    
    // Texture entries with ZLIB compression
    for _ in 0..texture_count {
        data.write_u8(1).unwrap(); // compression_type = Zlib
        data.write_u8(0).unwrap(); // pixel_format = RGBA32
        data.write_u16::<LittleEndian>(width).unwrap();
        data.write_u16::<LittleEndian>(height).unwrap();
        
        let compressed_data = generate_zlib_compressed_texture(width, height);
        data.write_u32::<LittleEndian>(compressed_data.len() as u32).unwrap();
        data.write_all(&compressed_data).unwrap();
    }
    
    data
}

/// Generate valid ANIM data with LZ4 compression
pub fn generate_anim_with_lz4_compression(
    sprite_count: u32,
    texture_count: u32,
    frame_count: u32,
    width: u16,
    height: u16,
) -> Vec<u8> {
    let mut data = Vec::new();
    
    // ANIM header
    data.write_u32::<LittleEndian>(0x4D494E41).unwrap(); // Magic
    data.write_u32::<LittleEndian>(0x00000001).unwrap(); // Version
    data.write_u32::<LittleEndian>(sprite_count).unwrap();
    data.write_u32::<LittleEndian>(texture_count).unwrap();
    data.write_u32::<LittleEndian>(frame_count).unwrap();
    data.write_u32::<LittleEndian>(0).unwrap(); // Animation count
    data.write_u32::<LittleEndian>(0).unwrap(); // Palette count
    data.write_u32::<LittleEndian>(0).unwrap(); // Reserved
    
    // Sprite entries
    for _ in 0..sprite_count {
        data.write_u32::<LittleEndian>(0).unwrap();
        data.write_u32::<LittleEndian>(0).unwrap();
        data.write_u16::<LittleEndian>(width).unwrap();
        data.write_u16::<LittleEndian>(height).unwrap();
    }
    
    // Texture entries with LZ4 compression
    for _ in 0..texture_count {
        data.write_u8(2).unwrap(); // compression_type = Lz4
        data.write_u8(0).unwrap(); // pixel_format = RGBA32
        data.write_u16::<LittleEndian>(width).unwrap();
        data.write_u16::<LittleEndian>(height).unwrap();
        
        let compressed_data = generate_lz4_compressed_texture(width, height);
        data.write_u32::<LittleEndian>(compressed_data.len() as u32).unwrap();
        data.write_all(&compressed_data).unwrap();
    }
    
    data
}

/// Generate GRP data with varied pixel patterns for testing
pub fn generate_grp_with_pattern(
    frame_count: u16,
    width: u16,
    height: u16,
    pattern_type: u8,
) -> Vec<u8> {
    let mut data = Vec::new();
    
    // GRP header
    data.write_u16::<LittleEndian>(frame_count).unwrap();
    data.write_u16::<LittleEndian>(width).unwrap();
    data.write_u16::<LittleEndian>(height).unwrap();
    
    // Frame offset table
    let header_size = 6;
    let offset_table_size = (frame_count as usize) * 4;
    let mut frame_offsets = Vec::new();
    let mut current_offset = header_size + offset_table_size;
    
    let pixel_count = (width as usize) * (height as usize);
    let frame_data_size = 2 + (pixel_count / 255) * 2 + 2;
    
    for _ in 0..frame_count {
        frame_offsets.push(current_offset);
        current_offset += frame_data_size;
    }
    
    for offset in &frame_offsets {
        data.write_u32::<LittleEndian>(*offset as u32).unwrap();
    }
    
    // Generate frame data with different patterns
    for frame_idx in 0..frame_count {
        let pixel_value = match pattern_type {
            0 => (frame_idx % 256) as u8,           // Sequential
            1 => ((frame_idx * 17) % 256) as u8,    // Stepped
            2 => if frame_idx % 2 == 0 { 0 } else { 255 }, // Alternating
            _ => 128,                                // Constant gray
        };
        
        // RLE encode the frame
        let mut remaining_pixels = pixel_count;
        while remaining_pixels > 0 {
            let run_length = remaining_pixels.min(255);
            data.write_u8(run_length as u8).unwrap();
            data.write_u8(pixel_value).unwrap();
            remaining_pixels -= run_length;
        }
    }
    
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_valid_anim_data() {
        let data = generate_valid_anim_data(1, 1, 1, 64, 64, 0);
        
        // Check magic number
        assert_eq!(&data[0..4], &[0x41, 0x4E, 0x49, 0x4D]); // "ANIM" in little-endian
        
        // Check version
        assert_eq!(&data[4..8], &[0x01, 0x00, 0x00, 0x00]);
        
        // Should have reasonable size
        assert!(data.len() > 32); // At least header + some data
    }
    
    #[test]
    fn test_generate_valid_grp_data() {
        let data = generate_valid_grp_data(1, 32, 32);
        
        // Check frame count
        assert_eq!(u16::from_le_bytes([data[0], data[1]]), 1);
        
        // Check dimensions
        assert_eq!(u16::from_le_bytes([data[2], data[3]]), 32);
        assert_eq!(u16::from_le_bytes([data[4], data[5]]), 32);
        
        // Should have reasonable size
        assert!(data.len() > 10); // At least header + offset table + some data
    }
    
    #[test]
    fn test_generate_zlib_compressed_texture() {
        let compressed = generate_zlib_compressed_texture(64, 64);
        
        // Compressed data should be smaller than uncompressed
        let uncompressed_size = 64 * 64 * 4; // RGBA
        assert!(compressed.len() < uncompressed_size);
        assert!(compressed.len() > 0);
    }
    
    #[test]
    fn test_generate_lz4_compressed_texture() {
        let compressed = generate_lz4_compressed_texture(64, 64);
        
        // Should have data
        assert!(compressed.len() > 0);
        
        // LZ4 prepends size, so should have at least 4 bytes
        assert!(compressed.len() >= 4);
    }
    
    #[test]
    fn test_anim_params_strategy_generates_valid_ranges() {
        // This test verifies the strategy generates values in expected ranges
        let strategy = anim_params_strategy();
        let mut runner = proptest::test_runner::TestRunner::default();
        
        for _ in 0..10 {
            let (sprite_count, texture_count, frame_count, width, height, compression_type) = 
                strategy.new_tree(&mut runner).unwrap().current();
            
            assert!(sprite_count >= 1 && sprite_count < 10);
            assert!(texture_count >= 1 && texture_count < 10);
            assert!(frame_count >= 1 && frame_count < 100);
            assert!(width >= 1 && width < 512);
            assert!(height >= 1 && height < 512);
            assert!(compression_type < 3);
        }
    }
    
    #[test]
    fn test_grp_params_strategy_generates_valid_ranges() {
        let strategy = grp_params_strategy();
        let mut runner = proptest::test_runner::TestRunner::default();
        
        for _ in 0..10 {
            let (frame_count, width, height) = strategy.new_tree(&mut runner).unwrap().current();
            
            assert!(frame_count >= 1 && frame_count < 100);
            assert!(width >= 1 && width < 512);
            assert!(height >= 1 && height < 512);
        }
    }
}
