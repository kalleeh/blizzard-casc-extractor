/// Property-based test generators for ANIM and GRP formats
/// 
/// This module provides generators for creating valid ANIM and GRP test data
/// for comprehensive property-based testing using proptest.

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Write;

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

#[cfg(test)]
mod tests {
    use super::*;

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
