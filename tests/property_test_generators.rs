/// Property-based test generators for ANIM and GRP formats
///
/// This module provides generators for creating valid ANIM and GRP test data
/// for comprehensive property-based testing using proptest.
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Write;

/// Generate valid GRP file data with specified parameters.
///
/// Creates a minimal but valid GRP file structure that can be parsed
/// by the GRP parser. The generated data includes:
/// - Valid GRP header with frame count and dimensions
/// - Frame entry table (8 bytes per frame: xOffset, yOffset, unknown, fileOffset)
/// - Per-frame line offset table (2 bytes per line)
/// - RLE-encoded line data (repeat-encoded, 2 bytes per line)
pub fn generate_valid_grp_data(
    frame_count: u16,
    width: u16,
    height: u16,
) -> Vec<u8> {
    let mut data = Vec::new();

    // GRP header (6 bytes)
    data.write_u16::<LittleEndian>(frame_count).unwrap();
    data.write_u16::<LittleEndian>(width).unwrap();
    data.write_u16::<LittleEndian>(height).unwrap();

    // Build each frame's binary blob ahead of time so we know sizes for the
    // file-offset table.
    //
    // Frame data layout (relative to frame start):
    //   [0 .. height*2)  line offset table: height u16 entries
    //   [height*2 ..)    RLE line data, one entry per line
    //
    // The parser derives line_count = frame_data[0..2] / 2, so the first
    // entry of the line offset table must equal height * 2.
    let mut all_frame_data: Vec<Vec<u8>> = Vec::new();

    for frame_idx in 0..frame_count {
        let pixel_value = (frame_idx % 256) as u8;
        let line_table_size = (height as usize) * 2; // bytes consumed by line offset table

        // Build each RLE line: use the repeat opcode (byte > 0x40, count = byte - 0x40)
        // to encode `width` pixels of the same value.  Max repeat per opcode: 191.
        let mut line_blobs: Vec<Vec<u8>> = Vec::new();
        for _ in 0..height {
            let mut line = Vec::new();
            let mut remaining = width as usize;
            while remaining > 0 {
                let run = remaining.min(191); // repeat opcode: byte = run + 0x40
                line.push((run + 0x40) as u8);
                line.push(pixel_value);
                remaining -= run;
            }
            line_blobs.push(line);
        }

        // Write line offset table then line data
        let mut frame_data = Vec::new();
        let mut line_offset = line_table_size; // first line's data starts after the table
        for (i, blob) in line_blobs.iter().enumerate() {
            frame_data.write_u16::<LittleEndian>(line_offset as u16).unwrap();
            if i + 1 < line_blobs.len() {
                line_offset += blob.len();
            }
        }
        for blob in &line_blobs {
            frame_data.write_all(blob).unwrap();
        }

        all_frame_data.push(frame_data);
    }

    // Frame entry table: 8 bytes per frame
    //   xOffset  u8
    //   yOffset  u8
    //   unknown  u16
    //   fileOffset u32  (absolute offset from start of file)
    let entry_table_size = (frame_count as usize) * 8;
    let mut file_offset = 6 + entry_table_size; // first frame data follows the table

    for frame_data in &all_frame_data {
        data.write_u8(0).unwrap();                                   // xOffset
        data.write_u8(0).unwrap();                                   // yOffset
        data.write_u16::<LittleEndian>(0).unwrap();                  // unknown
        data.write_u32::<LittleEndian>(file_offset as u32).unwrap(); // fileOffset
        file_offset += frame_data.len();
    }

    // Write frame data blobs
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
        assert!(!compressed.is_empty());
    }

    #[test]
    fn test_generate_lz4_compressed_texture() {
        let compressed = generate_lz4_compressed_texture(64, 64);

        // Should have data
        assert!(!compressed.is_empty());

        // LZ4 prepends size, so should have at least 4 bytes
        assert!(compressed.len() >= 4);
    }

    #[test]
    fn test_grp_params_strategy_generates_valid_ranges() {
        let strategy = grp_params_strategy();
        let mut runner = proptest::test_runner::TestRunner::default();

        for _ in 0..10 {
            let (frame_count, width, height) = strategy.new_tree(&mut runner).unwrap().current();

            assert!((1..100).contains(&frame_count));
            assert!((1..512).contains(&width));
            assert!((1..512).contains(&height));
        }
    }
}
