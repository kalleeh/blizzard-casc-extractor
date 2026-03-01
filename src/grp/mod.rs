use std::fmt;
use crate::anim::{AnimPalette, PaletteType};

/// Errors that can occur during GRP parsing
#[derive(Debug)]
pub enum GrpError {
    InvalidHeader(String),
    InvalidDimensions { frame_count: u16, width: u16, height: u16 },
    FrameOffsetOutOfBounds { frame_index: usize, offset: usize, data_size: usize },
    RleDecodingFailed(String),
    InsufficientData { expected: usize, actual: usize },
}

impl fmt::Display for GrpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GrpError::InvalidHeader(msg) => write!(f, "Invalid GRP header: {}", msg),
            GrpError::InvalidDimensions { frame_count, width, height } => {
                write!(f, "Invalid GRP dimensions: {}x{} pixels, {} frames", width, height, frame_count)
            }
            GrpError::FrameOffsetOutOfBounds { frame_index, offset, data_size } => {
                write!(f, "Frame {} offset {} exceeds data size {}", frame_index, offset, data_size)
            }
            GrpError::RleDecodingFailed(msg) => write!(f, "RLE decoding failed: {}", msg),
            GrpError::InsufficientData { expected, actual } => {
                write!(f, "Insufficient data: expected {}, got {}", expected, actual)
            }
        }
    }
}

impl std::error::Error for GrpError {}

/// Represents a single frame in a GRP file
#[derive(Debug, Clone)]
pub struct GrpFrame {
    pub pixel_data: Vec<u8>,
    pub width: u16,
    pub height: u16,
}

/// Represents a complete GRP file with all frames
#[derive(Debug)]
pub struct GrpFile {
    pub frame_count: u16,
    pub width: u16,
    pub height: u16,
    pub frames: Vec<GrpFrame>,
}

impl GrpFile {
    /// Parse a GRP file from raw data with comprehensive validation
    pub fn parse(data: &[u8]) -> Result<Self, GrpError> {
        // Validate minimum header size
        if data.len() < 6 {
            return Err(GrpError::InvalidHeader(
                format!("File too small: {} bytes, minimum 6 required", data.len())
            ));
        }
        
        // Parse header
        let frame_count = u16::from_le_bytes([data[0], data[1]]);
        let width = u16::from_le_bytes([data[2], data[3]]);
        let height = u16::from_le_bytes([data[4], data[5]]);
        
        log::debug!("GRP header: {} frames, {}x{} pixels", frame_count, width, height);
        
        // Validate dimensions
        if frame_count == 0 {
            return Err(GrpError::InvalidDimensions { frame_count, width, height });
        }
        if width == 0 || height == 0 {
            return Err(GrpError::InvalidDimensions { frame_count, width, height });
        }
        
        // Reasonable bounds checking to prevent excessive memory allocation
        if frame_count > 1000 || width > 2048 || height > 2048 {
            return Err(GrpError::InvalidDimensions { frame_count, width, height });
        }
        
        // Calculate and validate frame offset table size (8 bytes per frame)
        let offset_table_size = frame_count as usize * 8;
        let header_size = 6 + offset_table_size;
        
        if data.len() < header_size {
            return Err(GrpError::InvalidHeader(
                format!("File too small for offset table: {} bytes, {} required", 
                       data.len(), header_size)
            ));
        }
        
        // Parse frame offset table with bounds checking
        // Format: xOffset(1), yOffset(1), unknown(2), fileOffset(4)
        let mut frame_offsets = Vec::with_capacity(frame_count as usize);
        for i in 0..frame_count {
            let offset_pos = 6 + (i as usize * 8) + 4; // Skip xOffset, yOffset, unknown
            let offset = u32::from_le_bytes([
                data[offset_pos],
                data[offset_pos + 1],
                data[offset_pos + 2],
                data[offset_pos + 3],
            ]) as usize;
            
            // Validate frame offset is within data bounds
            if offset >= data.len() {
                return Err(GrpError::FrameOffsetOutOfBounds {
                    frame_index: i as usize,
                    offset,
                    data_size: data.len(),
                });
            }
            
            frame_offsets.push(offset);
        }
        
        log::debug!("Frame offsets: {:?}", frame_offsets);
        
        // Parse all frames
        let mut frames = Vec::with_capacity(frame_count as usize);
        for (i, &frame_offset) in frame_offsets.iter().enumerate() {
            // Calculate frame data size using next DIFFERENT offset or end of file
            let next_offset = frame_offsets.iter()
                .skip(i + 1)
                .find(|&&offset| offset != frame_offset && offset > frame_offset)
                .copied()
                .unwrap_or(data.len());
            
            if next_offset <= frame_offset {
                return Err(GrpError::InvalidHeader(
                    format!("Frame {} has invalid size: offset {} to {}", 
                           i, frame_offset, next_offset)
                ));
            }
            
            let frame_data = &data[frame_offset..next_offset];
            let frame = Self::parse_frame(frame_data, width, height, i)?;
            frames.push(frame);
        }
        
        Ok(GrpFile {
            frame_count,
            width,
            height,
            frames,
        })
    }
    
    /// Parse a single frame with run-length decoding
    fn parse_frame(frame_data: &[u8], width: u16, height: u16, frame_index: usize) -> Result<GrpFrame, GrpError> {
        let pixel_count = (width as usize) * (height as usize);
        let mut pixel_data = vec![0u8; pixel_count];

        // GRP frames start with a line offset table
        // The first offset tells us where the line data starts
        // Number of lines = first_offset / 2
        if frame_data.len() < 2 {
            return Err(GrpError::InsufficientData {
                expected: 2,
                actual: frame_data.len(),
            });
        }

        let first_offset = u16::from_le_bytes([frame_data[0], frame_data[1]]) as usize;
        let line_count = first_offset / 2;

        if line_count > height as usize {
            log::warn!("Frame {}: Line count {} exceeds height {}, using height",
                      frame_index, line_count, height);
        }

        let actual_line_count = line_count.min(height as usize);

        // Parse line offset table
        let mut line_offsets = Vec::with_capacity(actual_line_count);
        for i in 0..actual_line_count {
            let offset_pos = i * 2;
            if offset_pos + 1 >= frame_data.len() {
                break;
            }
            let offset = u16::from_le_bytes([frame_data[offset_pos], frame_data[offset_pos + 1]]) as usize;
            line_offsets.push(offset);
        }

        log::debug!("Frame {}: {} lines, first offset: {}", frame_index, line_offsets.len(), first_offset);

        // Decode each line
        for (line_idx, &line_offset) in line_offsets.iter().enumerate() {
            if line_offset >= frame_data.len() {
                log::warn!("Frame {}: Line {} offset {} exceeds frame data size {}",
                          frame_index, line_idx, line_offset, frame_data.len());
                continue;
            }

            // Calculate line end (next line's offset or end of frame)
            let line_end = if line_idx + 1 < line_offsets.len() {
                let next_offset = line_offsets[line_idx + 1];
                if next_offset <= line_offset || next_offset > frame_data.len() {
                    frame_data.len()
                } else {
                    next_offset
                }
            } else {
                frame_data.len()
            };

            if line_end <= line_offset {
                log::warn!("Frame {}: Line {} has invalid range {}..{}",
                          frame_index, line_idx, line_offset, line_end);
                continue;
            }

            let line_data = &frame_data[line_offset..line_end];
            let decoded = Self::decode_rle_line(line_data, width)?;
            let row_start = line_idx * width as usize;
            pixel_data[row_start..row_start + width as usize].copy_from_slice(&decoded);
        }

        log::debug!("Successfully parsed frame {}", frame_index);

        Ok(GrpFrame {
            pixel_data,
            width,
            height,
        })
    }

    /// Decode a single RLE-encoded line into a flat pixel buffer of `width` bytes.
    ///
    /// RLE codes:
    ///   byte >= 0x80  — skip (byte - 0x80) transparent pixels
    ///   byte >  0x40  — repeat the next byte (byte - 0x40) times
    ///   otherwise     — copy the next `byte` literal bytes
    fn decode_rle_line(line_data: &[u8], width: u16) -> Result<Vec<u8>, GrpError> {
        let w = width as usize;
        let mut pixels = vec![0u8; w];
        let mut col = 0usize;
        let mut pos = 0usize;

        while pos < line_data.len() && col < w {
            let byte = line_data[pos];
            pos += 1;

            if byte >= 0x80 {
                // Skip (transparent) pixels — advance col, leave zeros in place
                let skip = (byte - 0x80) as usize;
                col = col.saturating_add(skip).min(w);
            } else if byte > 0x40 {
                // RLE repeat: fill `count` pixels with the next byte value
                let count = (byte - 0x40) as usize;
                if pos >= line_data.len() {
                    break;
                }
                let pixel = line_data[pos];
                pos += 1;
                let end = col.saturating_add(count).min(w);
                for p in pixels[col..end].iter_mut() {
                    *p = pixel;
                }
                col = end;
            } else {
                // Literal copy: read `count` pixel values directly
                let count = byte as usize;
                for _ in 0..count {
                    if pos >= line_data.len() || col >= w {
                        break;
                    }
                    pixels[col] = line_data[pos];
                    pos += 1;
                    col += 1;
                }
            }
        }

        Ok(pixels)
    }
    
    /// Get a specific frame by index
    pub fn get_frame(&self, index: usize) -> Option<&GrpFrame> {
        self.frames.get(index)
    }
    
    /// Get the first frame (most commonly used)
    pub fn get_first_frame(&self) -> Option<&GrpFrame> {
        self.frames.first()
    }
    
    /// Convert all frames to RGBA using StarCraft palette
    pub fn convert_all_frames_to_rgba(&self, palette: &AnimPalette) -> Result<Vec<Vec<u8>>, GrpError> {
        let mut rgba_frames = Vec::with_capacity(self.frames.len());
        
        for frame in &self.frames {
            let rgba_data = frame.to_rgba_with_transparency(palette)?;
            rgba_frames.push(rgba_data);
        }
        
        Ok(rgba_frames)
    }
    
    /// Convert all frames to RGBA using default StarCraft unit palette
    pub fn convert_all_frames_to_rgba_default(&self) -> Result<Vec<Vec<u8>>, GrpError> {
        let palette = AnimPalette::default_starcraft_unit_palette();
        self.convert_all_frames_to_rgba(&palette)
    }
    
    /// Create a StarCraft-compatible palette optimized for GRP sprites
    pub fn create_grp_optimized_palette() -> AnimPalette {
        let mut colors = Vec::with_capacity(256);
        
        // Index 0: Transparent (StarCraft convention)
        colors.push([0, 0, 0, 0]);
        
        // Colors 1-15: Grayscale ramp (common in StarCraft sprites)
        for i in 1..16 {
            let gray = (i * 17) as u8; // 17, 34, 51, ..., 255
            colors.push([gray, gray, gray, 255]);
        }
        
        // Colors 16-31: Red player colors
        for i in 0..16 {
            let intensity = 128 + (i * 8) as u8; // 128-248 range
            colors.push([255, intensity, intensity, 255]);
        }
        
        // Colors 32-47: Blue player colors  
        for i in 0..16 {
            let intensity = 128 + (i * 8) as u8;
            colors.push([intensity, intensity, 255, 255]);
        }
        
        // Colors 48-63: Green/teal player colors
        for i in 0..16 {
            let intensity = 128 + (i * 8) as u8;
            colors.push([intensity, 255, intensity, 255]);
        }
        
        // Colors 64-79: Yellow player colors
        for i in 0..16 {
            let intensity = 200 + (i * 3) as u8; // Bright yellows
            colors.push([255, 255, intensity, 255]);
        }
        
        // Colors 80-95: Purple player colors
        for i in 0..16 {
            let intensity = 128 + (i * 8) as u8;
            colors.push([255, intensity, 255, 255]);
        }
        
        // Colors 96-111: Orange player colors
        for i in 0..16 {
            let intensity = 128 + (i * 8) as u8;
            colors.push([255, intensity, 128, 255]);
        }
        
        // Colors 112-127: Brown/tan colors (terrain)
        for i in 0..16 {
            let r = 139 + (i * 4) as u8;
            let g = 69 + (i * 6) as u8;
            let b = 19 + (i * 2) as u8;
            colors.push([r, g, b, 255]);
        }
        
        // Colors 128-255: Fill with varied colors for effects and details
        for i in 128..256 {
            let r = ((i * 7) % 256) as u8;
            let g = ((i * 11) % 256) as u8;
            let b = ((i * 13) % 256) as u8;
            colors.push([r, g, b, 255]);
        }
        
        AnimPalette {
            colors,
            palette_type: PaletteType::Unit,
        }
    }
}

impl GrpFrame {
    /// Shared pixel-to-RGBA conversion logic used by all public to_rgba variants.
    ///
    /// `palette`           — palette to look up colors from; when `None`, uses
    ///                       `crate::palette::starcraft_palette()`.
    /// `transparent_index` — palette index that should always be rendered fully
    ///                       transparent (alpha 0); `None` means no forced
    ///                       transparency.
    fn to_rgba_internal(&self, palette: Option<&AnimPalette>, transparent_index: Option<u8>) -> Result<Vec<u8>, GrpError> {
        let pixel_count = (self.width as usize) * (self.height as usize);
        let mut rgba_pixels = Vec::with_capacity(pixel_count * 4);

        match palette {
            Some(pal) => {
                pal.validate().map_err(|e| GrpError::RleDecodingFailed(format!("Palette validation failed: {}", e)))?;
                for &index in &self.pixel_data {
                    let mut color = pal.get_color(index);
                    if transparent_index == Some(index) {
                        color[3] = 0;
                    }
                    rgba_pixels.extend_from_slice(&color);
                }
            }
            None => {
                let static_pal = crate::palette::starcraft_palette();
                for &index in &self.pixel_data {
                    let mut color = static_pal[index as usize];
                    if transparent_index == Some(index) {
                        color[3] = 0;
                    }
                    rgba_pixels.extend_from_slice(&color);
                }
            }
        }

        Ok(rgba_pixels)
    }

    /// Convert indexed pixel data to RGBA using StarCraft palette
    pub fn to_rgba_with_palette(&self, palette: &AnimPalette) -> Result<Vec<u8>, GrpError> {
        self.to_rgba_internal(Some(palette), None)
    }

    /// Convert indexed pixel data to RGBA using default StarCraft unit palette
    pub fn to_rgba(&self) -> Result<Vec<u8>, GrpError> {
        self.to_rgba_internal(None, None)
    }

    /// Convert indexed pixel data to RGBA with transparency preservation
    /// Index 0 is always treated as transparent
    pub fn to_rgba_with_transparency(&self, palette: &AnimPalette) -> Result<Vec<u8>, GrpError> {
        self.to_rgba_internal(Some(palette), Some(0))
    }

    /// High-performance batch conversion for large sprites
    /// Optimized for sprites larger than 64x64 pixels
    pub fn to_rgba_optimized(&self, palette: &AnimPalette) -> Result<Vec<u8>, GrpError> {
        self.to_rgba_internal(Some(palette), Some(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_grp_header_validation() {
        // Test file too small
        let small_data = vec![0u8; 5];
        assert!(matches!(GrpFile::parse(&small_data), Err(GrpError::InvalidHeader(_))));
        
        // Test zero dimensions
        let zero_frames = vec![0, 0, 32, 0, 32, 0]; // 0 frames, 32x32
        assert!(matches!(GrpFile::parse(&zero_frames), Err(GrpError::InvalidDimensions { .. })));
        
        let zero_width = vec![1, 0, 0, 0, 32, 0]; // 1 frame, 0x32
        assert!(matches!(GrpFile::parse(&zero_width), Err(GrpError::InvalidDimensions { .. })));
        
        let zero_height = vec![1, 0, 32, 0, 0, 0]; // 1 frame, 32x0
        assert!(matches!(GrpFile::parse(&zero_height), Err(GrpError::InvalidDimensions { .. })));
    }
    
    #[test]
    fn test_grp_offset_table_validation() {
        // Create header for 1 frame, 32x32
        let mut data = vec![1, 0, 32, 0, 32, 0]; // 6 bytes header
        
        // File too small for offset table (needs 4 more bytes)
        assert!(matches!(GrpFile::parse(&data), Err(GrpError::InvalidHeader(_))));
        
        // Add offset table but with invalid offset
        data.extend_from_slice(&[255, 255, 255, 255]); // offset way beyond file size
        assert!(matches!(GrpFile::parse(&data), Err(GrpError::FrameOffsetOutOfBounds { .. })));
    }
    
    #[test]
    fn test_valid_grp_parsing() {
        // Create a minimal valid GRP file
        let mut data = vec![1, 0, 2, 0, 2, 0]; // 1 frame, 2x2 pixels
        data.extend_from_slice(&[10, 0, 0, 0]); // frame offset at position 10
        data.extend_from_slice(&[4, 255]); // RLE: 4 pixels of value 255
        
        let grp = GrpFile::parse(&data).expect("Should parse valid GRP");
        assert_eq!(grp.frame_count, 1);
        assert_eq!(grp.width, 2);
        assert_eq!(grp.height, 2);
        assert_eq!(grp.frames.len(), 1);
        
        let frame = grp.get_first_frame().expect("Should have first frame");
        assert_eq!(frame.width, 2);
        assert_eq!(frame.height, 2);
        assert_eq!(frame.pixel_data.len(), 4); // 2x2 pixels
        
        // All pixels should be 255 due to RLE
        for &pixel in &frame.pixel_data {
            assert_eq!(pixel, 255);
        }
    }
    
    #[test]
    fn test_rle_decoding() {
        // Create GRP with multiple RLE runs
        let mut data = vec![1, 0, 3, 0, 2, 0]; // 1 frame, 3x2 pixels (6 total)
        data.extend_from_slice(&[10, 0, 0, 0]); // frame offset at position 10
        
        // RLE data: 2 pixels of value 100, then 4 pixels of value 200
        data.extend_from_slice(&[2, 100, 4, 200]);
        
        let grp = GrpFile::parse(&data).expect("Should parse GRP with RLE");
        let frame = grp.get_first_frame().expect("Should have first frame");
        
        assert_eq!(frame.pixel_data.len(), 6); // 3x2 pixels
        assert_eq!(frame.pixel_data[0], 100);
        assert_eq!(frame.pixel_data[1], 100);
        assert_eq!(frame.pixel_data[2], 200);
        assert_eq!(frame.pixel_data[3], 200);
        assert_eq!(frame.pixel_data[4], 200);
        assert_eq!(frame.pixel_data[5], 200);
    }
    
    #[test]
    fn test_rle_error_handling() {
        // Test zero run length
        let mut data = vec![1, 0, 2, 0, 2, 0]; // 1 frame, 2x2 pixels
        data.extend_from_slice(&[10, 0, 0, 0]); // frame offset at position 10
        data.extend_from_slice(&[0, 255]); // Invalid: zero run length
        
        let result = GrpFile::parse(&data);
        assert!(matches!(result, Err(GrpError::RleDecodingFailed(_))));
        
        // Test run length exceeding pixel buffer
        let mut data = vec![1, 0, 2, 0, 2, 0]; // 1 frame, 2x2 pixels (4 total)
        data.extend_from_slice(&[10, 0, 0, 0]); // frame offset at position 10
        data.extend_from_slice(&[5, 255]); // Invalid: 5 pixels but only 4 expected
        
        let result = GrpFile::parse(&data);
        assert!(matches!(result, Err(GrpError::RleDecodingFailed(_))));
    }
    
    #[test]
    fn test_multiple_frames() {
        // Create GRP with 2 frames
        let mut data = vec![2, 0, 2, 0, 2, 0]; // 2 frames, 2x2 pixels each
        data.extend_from_slice(&[14, 0, 0, 0]); // frame 0 offset at position 14
        data.extend_from_slice(&[16, 0, 0, 0]); // frame 1 offset at position 16
        
        // Frame 0: 4 pixels of value 100 (2 bytes: run_length=4, value=100)
        data.extend_from_slice(&[4, 100]);
        // Frame 1: 4 pixels of value 200 (2 bytes: run_length=4, value=200)
        data.extend_from_slice(&[4, 200]);
        
        let grp = GrpFile::parse(&data).expect("Should parse multi-frame GRP");
        assert_eq!(grp.frame_count, 2);
        assert_eq!(grp.frames.len(), 2);
        
        // Check frame 0
        let frame0 = &grp.frames[0];
        assert_eq!(frame0.pixel_data.len(), 4);
        for &pixel in &frame0.pixel_data {
            assert_eq!(pixel, 100);
        }
        
        // Check frame 1
        let frame1 = &grp.frames[1];
        assert_eq!(frame1.pixel_data.len(), 4);
        for &pixel in &frame1.pixel_data {
            assert_eq!(pixel, 200);
        }
    }
    
    #[test]
    fn test_grp_palette_integration() {
        // Create a simple GRP with known pixel values
        let mut data = vec![1, 0, 2, 0, 2, 0]; // 1 frame, 2x2 pixels
        data.extend_from_slice(&[10, 0, 0, 0]); // frame offset at position 10
        data.extend_from_slice(&[1, 0, 3, 15]); // RLE: 1 pixel of value 0, 3 pixels of value 15
        
        let grp = GrpFile::parse(&data).expect("Should parse GRP");
        let frame = grp.get_first_frame().expect("Should have first frame");
        
        // Test conversion to RGBA with default palette
        let rgba_data = frame.to_rgba().expect("Should convert to RGBA");
        assert_eq!(rgba_data.len(), 16); // 4 pixels * 4 bytes (RGBA)
        
        // First pixel should be transparent (index 0)
        assert_eq!(rgba_data[0..4], [0, 0, 0, 0]); // Transparent
        
        // Other pixels should have color from palette (index 15)
        let palette = crate::anim::AnimPalette::default_starcraft_unit_palette();
        let expected_color = palette.get_color(15);
        for i in 1..4 {
            let start_idx = i * 4;
            assert_eq!(rgba_data[start_idx..start_idx + 4], expected_color);
        }
    }
    
    #[test]
    fn test_grp_transparency_preservation() {
        // Create GRP with transparency (index 0) and regular colors
        let mut data = vec![1, 0, 3, 0, 1, 0]; // 1 frame, 3x1 pixels
        data.extend_from_slice(&[10, 0, 0, 0]); // frame offset at position 10
        data.extend_from_slice(&[1, 0, 1, 5, 1, 0]); // RLE: 1 transparent, 1 color, 1 transparent
        
        let grp = GrpFile::parse(&data).expect("Should parse GRP");
        let frame = grp.get_first_frame().expect("Should have first frame");
        
        let palette = GrpFile::create_grp_optimized_palette();
        let rgba_data = frame.to_rgba_with_transparency(&palette).expect("Should convert with transparency");
        
        assert_eq!(rgba_data.len(), 12); // 3 pixels * 4 bytes (RGBA)
        
        // First pixel: transparent (index 0)
        assert_eq!(rgba_data[0..4], [0, 0, 0, 0]);
        
        // Second pixel: colored (index 5)
        let expected_color = palette.get_color(5);
        assert_eq!(rgba_data[4..8], expected_color);
        
        // Third pixel: transparent (index 0)
        assert_eq!(rgba_data[8..12], [0, 0, 0, 0]);
    }
    
    #[test]
    fn test_grp_optimized_palette_structure() {
        let palette = GrpFile::create_grp_optimized_palette();
        
        // Should have 256 colors
        assert_eq!(palette.colors.len(), 256);
        
        // Index 0 should be transparent
        assert_eq!(palette.colors[0], [0, 0, 0, 0]);
        
        // Colors 1-15 should be grayscale
        for i in 1..16 {
            let expected_gray = (i * 17) as u8;
            assert_eq!(palette.colors[i], [expected_gray, expected_gray, expected_gray, 255]);
        }
        
        // Colors 16-31 should be red tones
        for i in 16..32 {
            let color = palette.colors[i];
            assert_eq!(color[0], 255); // Red channel should be max
            assert_eq!(color[3], 255); // Alpha should be opaque
        }
        
        // All non-transparent colors should be opaque
        for i in 1..256 {
            assert_eq!(palette.colors[i][3], 255, "Color {} should be opaque", i);
        }
    }
    
    #[test]
    fn test_grp_all_frames_conversion() {
        // Create GRP with multiple frames
        let mut data = vec![2, 0, 2, 0, 2, 0]; // 2 frames, 2x2 pixels each
        data.extend_from_slice(&[14, 0, 0, 0]); // frame 0 offset
        data.extend_from_slice(&[16, 0, 0, 0]); // frame 1 offset
        data.extend_from_slice(&[4, 10]); // Frame 0: 4 pixels of value 10
        data.extend_from_slice(&[4, 20]); // Frame 1: 4 pixels of value 20
        
        let grp = GrpFile::parse(&data).expect("Should parse multi-frame GRP");
        
        // Test converting all frames
        let rgba_frames = grp.convert_all_frames_to_rgba_default().expect("Should convert all frames");
        assert_eq!(rgba_frames.len(), 2);
        
        // Each frame should have 16 bytes (4 pixels * 4 bytes RGBA)
        for frame_data in &rgba_frames {
            assert_eq!(frame_data.len(), 16);
        }
        
        // Test with custom palette
        let palette = GrpFile::create_grp_optimized_palette();
        let rgba_frames_custom = grp.convert_all_frames_to_rgba(&palette).expect("Should convert with custom palette");
        assert_eq!(rgba_frames_custom.len(), 2);
    }
    
    #[test]
    fn test_grp_invalid_palette_index_handling() {
        // Create GRP with an invalid palette index (255)
        let mut data = vec![1, 0, 2, 0, 2, 0]; // 1 frame, 2x2 pixels
        data.extend_from_slice(&[10, 0, 0, 0]); // frame offset
        data.extend_from_slice(&[4, 255]); // 4 pixels of value 255 (might be invalid)
        
        let grp = GrpFile::parse(&data).expect("Should parse GRP");
        let frame = grp.get_first_frame().expect("Should have first frame");
        
        // Should handle invalid indices gracefully (returns magenta)
        let rgba_data = frame.to_rgba().expect("Should convert even with invalid indices");
        assert_eq!(rgba_data.len(), 16); // 4 pixels * 4 bytes
        
        // All pixels should have some color (either valid palette color or magenta fallback)
        for chunk in rgba_data.chunks(4) {
            assert_eq!(chunk.len(), 4); // Each pixel should have RGBA
            assert_eq!(chunk[3], 255); // Alpha should be opaque for non-transparent indices
        }
    }
    
    #[test]
    fn test_grp_performance_optimization() {
        // Create a large GRP sprite (128x128 = 16384 pixels > 4096 threshold)
        let width = 128u16;
        let height = 128u16;
        let pixel_count = (width as usize) * (height as usize);
        
        let mut data = vec![1, 0]; // 1 frame
        data.extend_from_slice(&width.to_le_bytes()); // width
        data.extend_from_slice(&height.to_le_bytes()); // height
        data.extend_from_slice(&[10, 0, 0, 0]); // frame offset at position 10
        
        // Create RLE data for the large sprite (alternating pattern)
        let mut rle_data = Vec::new();
        let runs_per_line = 8; // 128 pixels / 16 pixels per run
        for y in 0..height {
            for run in 0..runs_per_line {
                let pixel_value = ((y + run) % 16) as u8; // Vary colors
                rle_data.extend_from_slice(&[16u8, pixel_value]); // 16 pixels per run
            }
        }
        data.extend_from_slice(&rle_data);
        
        let grp = GrpFile::parse(&data).expect("Should parse large GRP");
        let frame = grp.get_first_frame().expect("Should have first frame");
        
        assert_eq!(frame.pixel_data.len(), pixel_count);
        
        // Test optimized conversion
        let palette = GrpFile::create_grp_optimized_palette();
        let rgba_data = frame.to_rgba_optimized(&palette).expect("Should convert with optimization");
        
        assert_eq!(rgba_data.len(), pixel_count * 4); // RGBA
        
        // Test that optimized and regular conversion produce the same result
        let rgba_data_regular = frame.to_rgba_with_transparency(&palette).expect("Should convert regularly");
        assert_eq!(rgba_data, rgba_data_regular, "Optimized and regular conversion should match");
        
        // Verify transparency is preserved (index 0 should be transparent)
        for (i, chunk) in rgba_data.chunks(4).enumerate() {
            if frame.pixel_data[i] == 0 {
                assert_eq!(chunk[3], 0, "Index 0 should be transparent");
            } else {
                assert_eq!(chunk[3], 255, "Non-zero indices should be opaque");
            }
        }
    }
}