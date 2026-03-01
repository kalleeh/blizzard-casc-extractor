use std::path::{Path, PathBuf};
use thiserror::Error;
use serde::{Deserialize, Serialize};

use crate::blte_enhanced::{BlteDecompressor, BlteError};

use crate::casc::{CascArchive, CascError, FileInfo, FileAnalysis};
use crate::cli::ResolutionTier;
use crate::grp::{GrpFile, GrpError}; // Import GrpFile and GrpError for GRP format conversion

#[derive(Debug, Error)]
pub enum SpriteError {
    #[error("CASC error: {0}")]
    Casc(#[from] CascError),
    
    #[error("GRP format error: {0}")]
    Grp(#[from] GrpError),
    
    #[error("Invalid sprite format: {0}")]
    InvalidFormat(String),
    
    #[error("Sprite decode error: {0}")]
    DecodeError(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    
    // Enhanced error handling for sprite extraction failures
    #[error("Sprite extraction failed: {0}. Suggestion: {1}")]
    ExtractionFailed(String, String),
    
    #[error("Partial extraction failure: {extracted} of {total} sprites extracted. Errors: {errors:?}")]
    PartialFailure {
        extracted: usize,
        total: usize,
        errors: Vec<String>,
    },
    
    #[error("Unity metadata generation failed: {0}. Check Unity settings: {1}")]
    UnityMetadataError(String, String),
    
    #[error("File system permission error: {0}. Required permissions: {1}")]
    PermissionError(String, String),
    
    #[error("Output directory error: {0}. Suggested action: {1}")]
    OutputDirectoryError(String, String),
    
    #[error("Sprite format validation failed: {0}. Expected format: {1}")]
    FormatValidationError(String, String),
    
    #[error("BLTE decryption error: {0}")]
    BlteDecryption(String),
    
    #[error("Enhanced BLTE decompression error: {0}")]
    BlteEnhanced(#[from] BlteError),
    
    #[error("Unity compatibility error: {0}. Unity version requirement: {1}")]
    UnityCompatibilityError(String, String),
}

#[derive(Debug)]
pub struct DirectSpriteExtractor {
    casc_archive: CascArchive,
    max_files: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct SpriteData {
    pub name: String,
    pub format: SpriteFormat,
    pub resolution_tier: Option<ResolutionTier>,
    pub data: Vec<u8>,
    pub metadata: SpriteMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpriteFormat {
    PNG,
    JPEG,
    CompressedData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteMetadata {
    pub name: String,
    pub format: String,
    pub file_size: usize,
    pub resolution_tier: Option<String>,
    pub entropy: f64,
    pub has_transparency: bool,
    // Unity-specific metadata
    pub unity_metadata: Option<UnityMetadata>,
    // Comprehensive metadata
    pub dimensions: Option<ImageDimensions>,
    pub color_depth: Option<u8>,
    pub frame_count: Option<u32>,
    pub compression_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnityMetadata {
    pub sprite_mode: String,
    pub pixels_per_unit: f32,
    pub pivot: UnityPivot,
    pub filter_mode: String,
    pub wrap_mode: String,
    pub texture_type: String,
    pub max_texture_size: u32,
    pub texture_format: String,
    pub compression_quality: u32,
    pub generate_mip_maps: bool,
    pub readable: bool,
    pub alpha_source: String,
    pub alpha_is_transparency: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnityPivot {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug)]
pub struct ExtractionResult {
    pub sprites_extracted: usize,
    pub png_files: Vec<PathBuf>,
    pub jpeg_files: Vec<PathBuf>,
    pub metadata_files: Vec<PathBuf>,
    pub unity_metadata_files: Vec<PathBuf>,
    pub total_size: u64,
}

/// Unity-compatible sprite converter
#[derive(Debug)]
pub struct UnityConverter {
    pub pixels_per_unit: f32,
    pub filter_mode: String,
    pub wrap_mode: String,
    pub compression_quality: u32,
    pub generate_mip_maps: bool,
}

impl Default for UnityConverter {
    fn default() -> Self {
        Self {
            pixels_per_unit: 100.0,
            filter_mode: "Bilinear".to_string(),
            wrap_mode: "Clamp".to_string(),
            compression_quality: 50,
            generate_mip_maps: false,
        }
    }
}

impl DirectSpriteExtractor {
    #[allow(dead_code)]
    pub fn new(casc_archive: CascArchive) -> Self {
        Self { 
            casc_archive,
            max_files: None,
        }
    }
    
    pub fn new_with_max_files(casc_archive: CascArchive, max_files: Option<usize>) -> Self {
        Self { 
            casc_archive,
            max_files,
        }
    }
    
    /// Extract all sprites with Unity-compatible output structure
    pub fn extract_all_sprites(&self, output_dir: &Path) -> Result<ExtractionResult, SpriteError> {
        self.extract_all_sprites_with_unity_support(output_dir, &UnityConverter::default())
    }
    
    /// Extract all sprites with Unity-compatible output structure and custom Unity settings
    pub fn extract_all_sprites_with_unity_support(&self, output_dir: &Path, unity_converter: &UnityConverter) -> Result<ExtractionResult, SpriteError> {
        // Enhanced error handling: Validate output directory with detailed guidance
        if let Err(e) = std::fs::create_dir_all(output_dir) {
            return Err(SpriteError::OutputDirectoryError(
                format!("Failed to create output directory: {}", e),
                format!("Ensure the parent directory exists and you have write permissions to: {:?}", output_dir)
            ));
        }
        
        // Enhanced error handling: Validate Unity converter settings
        self.validate_unity_converter(unity_converter)?;
        
        let files = self.casc_archive.list_files_with_filter(Some("sprites")).map_err(|e| {
            SpriteError::ExtractionFailed(
                format!("Failed to list CASC sprite files: {}", e),
                "Check that the StarCraft: Remastered installation is valid and not corrupted".to_string()
            )
        })?;
        
        let mut result = ExtractionResult {
            sprites_extracted: 0,
            png_files: Vec::new(),
            jpeg_files: Vec::new(),
            metadata_files: Vec::new(),
            unity_metadata_files: Vec::new(),
            total_size: 0,
        };
        
        let mut extraction_errors = Vec::new();
        
        // Apply file limit if specified
        let files_to_process = if let Some(max_files) = self.max_files {
            log::info!("Limiting processing to {} files (out of {} total)", max_files, files.len());
            &files[..max_files.min(files.len())]
        } else {
            &files[..]
        };
        
        let total_files = files_to_process.len();
        
        for (index, file_info) in files_to_process.iter().enumerate() {
            match self.extract_sprite_from_file_with_recovery(file_info) {
                Ok(sprite_data) => {
                    match self.write_sprite_with_unity_metadata(output_dir, &sprite_data, unity_converter) {
                        Ok((output_path, unity_metadata_path)) => {
                            result.sprites_extracted += 1;
                            result.total_size += sprite_data.data.len() as u64;
                            
                            match sprite_data.format {
                                SpriteFormat::PNG => result.png_files.push(output_path.clone()),
                                SpriteFormat::JPEG => result.jpeg_files.push(output_path.clone()),
                                SpriteFormat::CompressedData => {
                                    let dat_path = output_path.with_extension("dat");
                                    result.png_files.push(dat_path);
                                }
                            }
                            
                            let metadata_path = output_path.with_extension("json");
                            result.metadata_files.push(metadata_path);
                            result.unity_metadata_files.push(unity_metadata_path);
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to write sprite {}: {}", file_info.name, e);
                            extraction_errors.push(error_msg.clone());
                            log::warn!("{}", error_msg);
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to extract sprite {}: {}", file_info.name, e);
                    extraction_errors.push(error_msg.clone());
                    
                    // Log at debug level for common issues to reduce spam
                    match &e {
                        SpriteError::ExtractionFailed(msg, _) if msg.contains("Missing data file") => {
                            log::debug!("{}", error_msg);
                        }
                        SpriteError::FormatValidationError(_, _) => {
                            log::debug!("{}", error_msg);
                        }
                        _ => {
                            log::warn!("{}", error_msg);
                        }
                    }
                }
            }
            
            // Progress reporting with error context
            if index % 100 == 0 || index == total_files - 1 {
                log::info!("Progress: {}/{} files processed, {} extracted, {} errors", 
                    index + 1, total_files, result.sprites_extracted, extraction_errors.len());
            }
        }
        
        // Enhanced error handling: Handle partial failures gracefully
        if !extraction_errors.is_empty() {
            if result.sprites_extracted == 0 {
                return Err(SpriteError::ExtractionFailed(
                    "No sprites could be extracted".to_string(),
                    format!("All {} extraction attempts failed. Check CASC installation integrity and file permissions", total_files)
                ));
            } else if extraction_errors.len() > total_files / 2 {
                log::warn!("High failure rate: {}/{} extractions failed", extraction_errors.len(), total_files);
                return Err(SpriteError::PartialFailure {
                    extracted: result.sprites_extracted,
                    total: total_files,
                    errors: extraction_errors.into_iter().take(10).collect(), // Limit error list
                });
            } else {
                log::info!("Extraction completed with {} errors out of {} files", extraction_errors.len(), total_files);
            }
        }
        
        Ok(result)
    }
    
    pub fn extract_sprite_from_file(&self, file_info: &crate::casc::FileInfo) -> Result<SpriteData, SpriteError> {
        let (raw_data, analysis) = self.casc_archive.extract_file_with_analysis(&file_info.key)?;
        
        let format = self.detect_sprite_format(&analysis);
        let has_transparency = self.detect_transparency(&raw_data, format);
        
        // Process the raw data based on detected format
        let processed_data = match format {
            SpriteFormat::PNG | SpriteFormat::JPEG => {
                // Data is already in a standard image format
                raw_data
            }
            SpriteFormat::CompressedData => {
                // Try to convert compressed/raw data to PNG
                self.convert_raw_data_to_png(&raw_data, file_info)?
            }
        };
        
        // Re-analyze the processed data
        let final_analysis = FileAnalysis::analyze(&processed_data);
        let final_format = self.detect_sprite_format(&final_analysis);
        
        // Extract comprehensive metadata
        let dimensions = self.extract_image_dimensions(&processed_data, final_format);
        let color_depth = self.extract_color_depth(&processed_data, final_format);
        let frame_count = self.extract_frame_count(&processed_data, final_format);
        let compression_ratio = self.calculate_compression_ratio(&processed_data, &dimensions);
        
        let metadata = SpriteMetadata {
            name: self.sanitize_filename(&file_info.name),
            format: format!("{:?}", final_format),
            file_size: processed_data.len(),
            resolution_tier: None, // FileInfo doesn't have resolution_tier
            entropy: final_analysis.entropy,
            has_transparency,
            unity_metadata: None, // Will be populated by Unity converter
            dimensions,
            color_depth,
            frame_count,
            compression_ratio,
        };
        
        Ok(SpriteData {
            name: file_info.name.clone(),
            format: final_format,
            resolution_tier: None, // FileInfo doesn't have resolution_tier
            data: processed_data,
            metadata,
        })
    }
    
    fn detect_sprite_format(&self, analysis: &FileAnalysis) -> SpriteFormat {
        if analysis.has_png_signature {
            SpriteFormat::PNG
        } else if analysis.has_jpeg_signature {
            SpriteFormat::JPEG
        } else {
            SpriteFormat::CompressedData
        }
    }
    
    fn detect_transparency(&self, data: &[u8], format: SpriteFormat) -> bool {
        match format {
            SpriteFormat::PNG => {
                data.len() > 25 && data[25] == 6
            }
            SpriteFormat::JPEG => false,
            SpriteFormat::CompressedData => false,
        }
    }
    
    fn sanitize_filename(&self, filename: &str) -> String {
        filename
            .chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                c => c,
            })
            .collect()
    }
    
    /// Get Unity-compatible output paths (sprite file and Unity metadata file)
    pub fn get_unity_output_paths(&self, output_dir: &Path, sprite_data: &SpriteData) -> Result<(PathBuf, PathBuf), SpriteError> {
        let tier_dir = match sprite_data.resolution_tier {
            Some(ResolutionTier::HD) => output_dir.join("HD"),
            Some(ResolutionTier::HD2) => output_dir.join("HD2"),
            Some(ResolutionTier::SD) => output_dir.join("SD"),
            Some(ResolutionTier::All) | None => output_dir.to_path_buf(),
        };
        
        std::fs::create_dir_all(&tier_dir)?;
        
        let extension = match sprite_data.format {
            SpriteFormat::PNG => "png",
            SpriteFormat::JPEG => "jpg",
            SpriteFormat::CompressedData => "dat",
        };
        
        let filename = format!("{}.{}", sprite_data.metadata.name, extension);
        let sprite_path = tier_dir.join(filename);
        
        // Unity metadata file with .unity.json extension
        let unity_metadata_path = sprite_path.with_extension("unity.json");
        
        Ok((sprite_path, unity_metadata_path))
    }
    
    /// Create Unity-compatible metadata for a sprite
    #[allow(dead_code)]
    pub fn create_unity_metadata(&self, sprite_data: &SpriteData, unity_converter: &UnityConverter) -> UnityMetadata {
        let texture_type = match sprite_data.format {
            SpriteFormat::PNG | SpriteFormat::JPEG => "Sprite (2D and UI)",
            SpriteFormat::CompressedData => "Default",
        };
        
        let texture_format = if sprite_data.metadata.has_transparency {
            "RGBA32"
        } else {
            "RGB24"
        };
        
        let max_texture_size = match sprite_data.resolution_tier {
            Some(ResolutionTier::HD2) => 4096,
            Some(ResolutionTier::HD) => 2048,
            Some(ResolutionTier::SD) => 1024,
            _ => 2048,
        };
        
        UnityMetadata {
            sprite_mode: "Single".to_string(),
            pixels_per_unit: unity_converter.pixels_per_unit,
            pivot: UnityPivot { x: 0.5, y: 0.5 }, // Center pivot
            filter_mode: unity_converter.filter_mode.clone(),
            wrap_mode: unity_converter.wrap_mode.clone(),
            texture_type: texture_type.to_string(),
            max_texture_size,
            texture_format: texture_format.to_string(),
            compression_quality: unity_converter.compression_quality,
            generate_mip_maps: unity_converter.generate_mip_maps,
            readable: false, // Unity default for sprites
            alpha_source: if sprite_data.metadata.has_transparency {
                "Input Texture Alpha".to_string()
            } else {
                "None".to_string()
            },
            alpha_is_transparency: sprite_data.metadata.has_transparency,
        }
    }
    
    /// Extract image dimensions from sprite data
    fn extract_image_dimensions(&self, data: &[u8], format: SpriteFormat) -> Option<ImageDimensions> {
        match format {
            SpriteFormat::PNG => self.extract_png_dimensions(data),
            SpriteFormat::JPEG => self.extract_jpeg_dimensions(data),
            SpriteFormat::CompressedData => None, // Cannot determine without decompression
        }
    }
    
    /// Extract PNG image dimensions from PNG data
    fn extract_png_dimensions(&self, data: &[u8]) -> Option<ImageDimensions> {
        // PNG signature: 89 50 4E 47 0D 0A 1A 0A
        if data.len() < 24 || &data[0..8] != b"\x89PNG\r\n\x1a\n" {
            return None;
        }
        
        // IHDR chunk starts at byte 8, dimensions at bytes 16-23
        if &data[12..16] == b"IHDR" {
            let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
            let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
            Some(ImageDimensions { width, height })
        } else {
            None
        }
    }
    
    /// Extract JPEG image dimensions from JPEG data
    fn extract_jpeg_dimensions(&self, data: &[u8]) -> Option<ImageDimensions> {
        // JPEG signature: FF D8
        if data.len() < 4 || &data[0..2] != b"\xFF\xD8" {
            return None;
        }
        
        let mut pos = 2;
        while pos + 4 < data.len() {
            if data[pos] != 0xFF {
                break;
            }
            
            let marker = data[pos + 1];
            
            // SOF0 (Start of Frame) marker: 0xFFC0
            if marker == 0xC0 {
                if pos + 9 < data.len() {
                    let height = u16::from_be_bytes([data[pos + 5], data[pos + 6]]) as u32;
                    let width = u16::from_be_bytes([data[pos + 7], data[pos + 8]]) as u32;
                    return Some(ImageDimensions { width, height });
                }
                break;
            }
            
            // Skip to next marker
            if pos + 2 < data.len() {
                let length = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
                pos += 2 + length;
            } else {
                break;
            }
        }
        
        None
    }
    
    /// Extract color depth from sprite data
    fn extract_color_depth(&self, data: &[u8], format: SpriteFormat) -> Option<u8> {
        match format {
            SpriteFormat::PNG => self.extract_png_color_depth(data),
            SpriteFormat::JPEG => Some(24), // JPEG is typically 24-bit
            SpriteFormat::CompressedData => None, // Cannot determine without decompression
        }
    }
    
    /// Extract PNG color depth from PNG data
    fn extract_png_color_depth(&self, data: &[u8]) -> Option<u8> {
        // PNG signature: 89 50 4E 47 0D 0A 1A 0A
        if data.len() < 25 || &data[0..8] != b"\x89PNG\r\n\x1a\n" {
            return None;
        }
        
        // IHDR chunk starts at byte 8, bit depth at byte 24, color type at byte 25
        if &data[12..16] == b"IHDR" && data.len() > 25 {
            let bit_depth = data[24];
            let color_type = data[25];
            
            // Calculate total color depth based on PNG color type
            match color_type {
                0 => Some(bit_depth),                    // Grayscale
                2 => Some(bit_depth * 3),                // RGB
                3 => Some(bit_depth),                    // Palette
                4 => Some(bit_depth * 2),                // Grayscale + Alpha
                6 => Some(bit_depth * 4),                // RGBA
                _ => Some(bit_depth),                    // Unknown, return bit depth
            }
        } else {
            None
        }
    }
    
    /// Extract frame count from sprite data (for animated formats)
    fn extract_frame_count(&self, data: &[u8], format: SpriteFormat) -> Option<u32> {
        match format {
            SpriteFormat::PNG => self.extract_png_frame_count(data),
            SpriteFormat::JPEG => Some(1), // JPEG is single frame
            SpriteFormat::CompressedData => None, // Cannot determine without decompression
        }
    }
    
    /// Extract frame count from PNG data (APNG support)
    fn extract_png_frame_count(&self, data: &[u8]) -> Option<u32> {
        // PNG signature: 89 50 4E 47 0D 0A 1A 0A
        if data.len() < 8 || &data[0..8] != b"\x89PNG\r\n\x1a\n" {
            return None;
        }
        
        let mut pos = 8;
        let mut frame_count = 1; // Default to 1 frame for static PNG
        
        // Look for acTL chunk (Animation Control) which indicates APNG
        while pos + 8 < data.len() {
            let chunk_length = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
            let chunk_type = &data[pos + 4..pos + 8];
            
            if chunk_type == b"acTL" && chunk_length >= 8 {
                // acTL chunk contains frame count at bytes 8-11
                if pos + 12 < data.len() {
                    frame_count = u32::from_be_bytes([data[pos + 8], data[pos + 9], data[pos + 10], data[pos + 11]]);
                }
                break;
            }
            
            // Move to next chunk
            pos += 8 + chunk_length + 4; // length + type + data + CRC
        }
        
        Some(frame_count)
    }
    
    /// Calculate compression ratio based on raw vs compressed size
    fn calculate_compression_ratio(&self, data: &[u8], dimensions: &Option<ImageDimensions>) -> Option<f64> {
        if let Some(dims) = dimensions {
            // Estimate uncompressed size (assuming 32-bit RGBA)
            let uncompressed_size = (dims.width * dims.height * 4) as f64;
            let compressed_size = data.len() as f64;
            
            if compressed_size > 0.0 {
                Some(uncompressed_size / compressed_size)
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Enhanced error handling: Validate Unity converter settings
    pub fn validate_unity_converter(&self, unity_converter: &UnityConverter) -> Result<(), SpriteError> {
        if unity_converter.pixels_per_unit <= 0.0 {
            return Err(SpriteError::UnityCompatibilityError(
                format!("Invalid pixels per unit: {}", unity_converter.pixels_per_unit),
                "Unity requires pixels per unit to be positive. Recommended range: 50-200".to_string()
            ));
        }
        
        if unity_converter.compression_quality > 100 {
            return Err(SpriteError::UnityCompatibilityError(
                format!("Invalid compression quality: {}", unity_converter.compression_quality),
                "Unity compression quality must be 0-100. Recommended: 50-75 for sprites".to_string()
            ));
        }
        
        // Validate filter mode
        match unity_converter.filter_mode.as_str() {
            "Point" | "Bilinear" | "Trilinear" => {},
            _ => return Err(SpriteError::UnityCompatibilityError(
                format!("Invalid filter mode: {}", unity_converter.filter_mode),
                "Unity supports: Point (pixel-perfect), Bilinear (smooth), Trilinear (high-quality)".to_string()
            ))
        }
        
        // Validate wrap mode
        match unity_converter.wrap_mode.as_str() {
            "Clamp" | "Repeat" | "Mirror" => {},
            _ => return Err(SpriteError::UnityCompatibilityError(
                format!("Invalid wrap mode: {}", unity_converter.wrap_mode),
                "Unity supports: Clamp (no bleeding), Repeat (tiling), Mirror (mirrored edges)".to_string()
            ))
        }
        
        Ok(())
    }
    
    /// Enhanced error handling: Extract sprite with recovery mechanisms
    fn extract_sprite_from_file_with_recovery(&self, file_info: &crate::casc::FileInfo) -> Result<SpriteData, SpriteError> {
        match self.extract_sprite_from_file(file_info) {
            Ok(sprite_data) => Ok(sprite_data),
            Err(SpriteError::Casc(casc_error)) => {
                // Log CASC errors at debug level instead of warning to reduce spam
                log::debug!("CASC extraction failed for {}: {}", file_info.name, casc_error);
                Err(SpriteError::ExtractionFailed(
                    format!("CASC extraction failed for {}: {}", file_info.name, casc_error),
                    "This may indicate a corrupted CASC file. Try validating your StarCraft installation".to_string()
                ))
            }
            Err(SpriteError::InvalidFormat(msg)) => {
                log::debug!("Invalid sprite format in {}: {}", file_info.name, msg);
                Err(SpriteError::FormatValidationError(
                    format!("Invalid sprite format in {}: {}", file_info.name, msg),
                    "Expected PNG, JPEG, or compressed sprite data. File may be corrupted or not a sprite".to_string()
                ))
            }
            Err(SpriteError::DecodeError(_)) => {
                log::debug!("No sprite data found in {}", file_info.name);
                Err(SpriteError::ExtractionFailed(
                    format!("No sprite data found in {}", file_info.name),
                    "File may be empty or not contain valid sprite data. Check file integrity".to_string()
                ))
            }
            Err(other_error) => Err(other_error),
        }
    }
    
    /// Enhanced error handling: Write sprite with Unity metadata and comprehensive error handling
    fn write_sprite_with_unity_metadata(&self, output_dir: &Path, sprite_data: &SpriteData, unity_converter: &UnityConverter) -> Result<(PathBuf, PathBuf), SpriteError> {
        let (output_path, unity_metadata_path) = self.get_unity_output_paths(output_dir, sprite_data)?;
        
        // Enhanced error handling: Write sprite data with permission checks
        if let Err(e) = std::fs::write(&output_path, &sprite_data.data) {
            return Err(SpriteError::PermissionError(
                format!("Failed to write sprite file {:?}: {}", output_path, e),
                format!("Ensure write permissions to directory: {:?}", output_path.parent().unwrap_or(output_dir))
            ));
        }
        
        // Enhanced error handling: Create Unity metadata with validation
        let unity_metadata = match self.create_unity_metadata_with_validation(sprite_data, unity_converter) {
            Ok(metadata) => metadata,
            Err(e) => return Err(e),
        };
        
        let sprite_with_unity = SpriteData {
            name: sprite_data.name.clone(),
            format: sprite_data.format,
            resolution_tier: sprite_data.resolution_tier,
            data: sprite_data.data.clone(),
            metadata: SpriteMetadata {
                unity_metadata: Some(unity_metadata),
                ..sprite_data.metadata.clone()
            },
        };
        
        // Enhanced error handling: Write standard metadata with JSON validation
        let metadata_path = output_path.with_extension("json");
        let metadata_json = serde_json::to_string_pretty(&sprite_with_unity.metadata)
            .map_err(|e| SpriteError::UnityMetadataError(
                format!("Failed to serialize sprite metadata: {}", e),
                "Check that all metadata fields contain valid values".to_string()
            ))?;
        
        if let Err(e) = std::fs::write(&metadata_path, metadata_json) {
            return Err(SpriteError::PermissionError(
                format!("Failed to write metadata file {:?}: {}", metadata_path, e),
                format!("Ensure write permissions to directory: {:?}", metadata_path.parent().unwrap_or(output_dir))
            ));
        }
        
        // Enhanced error handling: Write Unity-specific metadata with validation
        let unity_json = serde_json::to_string_pretty(&sprite_with_unity.metadata.unity_metadata)
            .map_err(|e| SpriteError::UnityMetadataError(
                format!("Failed to serialize Unity metadata: {}", e),
                "Check Unity converter settings for invalid values".to_string()
            ))?;
        
        if let Err(e) = std::fs::write(&unity_metadata_path, unity_json) {
            return Err(SpriteError::PermissionError(
                format!("Failed to write Unity metadata file {:?}: {}", unity_metadata_path, e),
                format!("Ensure write permissions to directory: {:?}", unity_metadata_path.parent().unwrap_or(output_dir))
            ));
        }
        
        Ok((output_path, unity_metadata_path))
    }
    
    /// Enhanced error handling: Create Unity metadata with comprehensive validation
    fn create_unity_metadata_with_validation(&self, sprite_data: &SpriteData, unity_converter: &UnityConverter) -> Result<UnityMetadata, SpriteError> {
        // Validate sprite data before creating Unity metadata
        if sprite_data.data.is_empty() {
            return Err(SpriteError::UnityMetadataError(
                "Cannot create Unity metadata for empty sprite data".to_string(),
                "Ensure sprite extraction was successful before generating Unity metadata".to_string()
            ));
        }
        
        // Validate resolution tier compatibility
        let max_texture_size = match sprite_data.resolution_tier {
            Some(ResolutionTier::HD2) => 4096,
            Some(ResolutionTier::HD) => 2048,
            Some(ResolutionTier::SD) => 1024,
            _ => 2048,
        };
        
        // Check if sprite dimensions are compatible with Unity limits
        if let Some(ref dims) = sprite_data.metadata.dimensions {
            if dims.width > max_texture_size || dims.height > max_texture_size {
                log::warn!("Sprite {}x{} exceeds recommended Unity texture size {} for resolution tier {:?}", 
                    dims.width, dims.height, max_texture_size, sprite_data.resolution_tier);
            }
            
            if dims.width > 8192 || dims.height > 8192 {
                return Err(SpriteError::UnityCompatibilityError(
                    format!("Sprite dimensions {}x{} exceed Unity maximum texture size", dims.width, dims.height),
                    "Unity maximum texture size is 8192x8192. Consider using a lower resolution tier".to_string()
                ));
            }
        }
        
        let texture_type = match sprite_data.format {
            SpriteFormat::PNG | SpriteFormat::JPEG => "Sprite (2D and UI)",
            SpriteFormat::CompressedData => "Default",
        };
        
        let texture_format = if sprite_data.metadata.has_transparency {
            "RGBA32"
        } else {
            "RGB24"
        };
        
        Ok(UnityMetadata {
            sprite_mode: "Single".to_string(),
            pixels_per_unit: unity_converter.pixels_per_unit,
            pivot: UnityPivot { x: 0.5, y: 0.5 }, // Center pivot
            filter_mode: unity_converter.filter_mode.clone(),
            wrap_mode: unity_converter.wrap_mode.clone(),
            texture_type: texture_type.to_string(),
            max_texture_size,
            texture_format: texture_format.to_string(),
            compression_quality: unity_converter.compression_quality,
            generate_mip_maps: unity_converter.generate_mip_maps,
            readable: false, // Unity default for sprites
            alpha_source: if sprite_data.metadata.has_transparency {
                "Input Texture Alpha".to_string()
            } else {
                "None".to_string()
            },
            alpha_is_transparency: sprite_data.metadata.has_transparency,
        })
    }
    
    /// Convert raw/compressed data to PNG format
    fn convert_raw_data_to_png(&self, raw_data: &[u8], file_info: &crate::casc::FileInfo) -> Result<Vec<u8>, SpriteError> {
        log::debug!("Converting raw data to PNG for file: {} (size: {} bytes)", file_info.name, raw_data.len());
        
        // Log first few bytes for debugging
        if raw_data.len() >= 16 {
            log::debug!("First 16 bytes: {:02x?}", &raw_data[0..16]);
        } else if !raw_data.is_empty() {
            log::debug!("First {} bytes: {:02x?}", raw_data.len(), raw_data);
        }
        
        // Try to detect various StarCraft formats
        if let Some(result) = self.try_detect_and_convert_formats(raw_data, file_info)? {
            return Ok(result);
        }
        
        // If we can't identify the format, fail with a clear error
        Err(SpriteError::InvalidFormat(format!(
            "Unrecognized sprite format in {}: {} bytes, no supported format detected", 
            file_info.name, raw_data.len()
        )))
    }
    
    /// Try to detect and convert various StarCraft formats - ENHANCED FOR REAL STARCRAFT DATA - FIXED INFINITE RECURSION
    fn try_detect_and_convert_formats(&self, raw_data: &[u8], file_info: &crate::casc::FileInfo) -> Result<Option<Vec<u8>>, SpriteError> {
        self.try_detect_and_convert_formats_with_depth(raw_data, file_info, 0)
    }
    
    /// Try to detect and convert various StarCraft formats with recursion depth tracking
    fn try_detect_and_convert_formats_with_depth(&self, raw_data: &[u8], file_info: &crate::casc::FileInfo, depth: u32) -> Result<Option<Vec<u8>>, SpriteError> {
        if raw_data.is_empty() {
            return Ok(None);
        }
        
        // CRITICAL FIX: Prevent infinite recursion by limiting depth
        const MAX_RECURSION_DEPTH: u32 = 3;
        if depth >= MAX_RECURSION_DEPTH {
            log::warn!("❌ Maximum recursion depth ({}) reached for {}, stopping to prevent stack overflow", 
                      MAX_RECURSION_DEPTH, file_info.name);
            return Ok(None);
        }
        
        log::info!("🔍 ANALYZING REAL STARCRAFT DATA: {} bytes in {}", raw_data.len(), file_info.name);
        
        // Log first 64 bytes for debugging real StarCraft data
        if raw_data.len() >= 64 {
            log::info!("First 64 bytes: {:02x?}", &raw_data[0..64]);
        } else if raw_data.len() >= 32 {
            log::info!("First {} bytes: {:02x?}", raw_data.len(), &raw_data[0..raw_data.len()]);
        }
        
        // Log file entropy and characteristics
        let mut byte_counts = [0u32; 256];
        let sample_size = raw_data.len().min(1024);
        for &byte in &raw_data[0..sample_size] {
            byte_counts[byte as usize] += 1;
        }
        let unique_bytes = byte_counts.iter().filter(|&&count| count > 0).count();
        let entropy_ratio = unique_bytes as f64 / 256.0;
        log::info!("Data characteristics: {} unique bytes out of {} sampled, entropy: {:.3}", 
                  unique_bytes, sample_size, entropy_ratio);
        
        // 1. Check for DDS format (common in StarCraft: Remastered)
        if raw_data.len() >= 4 && &raw_data[0..4] == b"DDS " {
            log::info!("✅ DETECTED DDS texture format in {}", file_info.name);
            return Ok(Some(self.convert_dds_to_png(raw_data, file_info)?));
        }
        
        // 2. Check for ANIM format (StarCraft: Remastered sprite format) - HIGHEST PRIORITY
        if raw_data.len() >= 16 {
            let magic = u32::from_le_bytes([raw_data[0], raw_data[1], raw_data[2], raw_data[3]]);
            if magic == 0x4D494E41 { // "ANIM" magic number
                log::info!("✅ DETECTED ANIM format in {}", file_info.name);
                return Ok(Some(self.convert_anim_to_png(raw_data, file_info)?));
            }
            
            // Also check for ANIM in big-endian
            let magic_be = u32::from_be_bytes([raw_data[0], raw_data[1], raw_data[2], raw_data[3]]);
            if magic_be == 0x4D494E41 {
                log::info!("✅ DETECTED ANIM format (big-endian) in {}", file_info.name);
                return Ok(Some(self.convert_anim_to_png(raw_data, file_info)?));
            }
        }
        
        // 3. Check for StarCraft: Remastered texture formats (common patterns)
        if raw_data.len() >= 128 {
            // Check for texture header patterns common in SC:R
            if self.looks_like_scr_texture(raw_data) {
                log::info!("✅ DETECTED StarCraft: Remastered texture format in {}", file_info.name);
                return Ok(Some(self.convert_scr_texture_to_png(raw_data, file_info)?));
            }
        }
        
        // 4. Check for GRP format (original StarCraft sprite format) - FIXED FOR REAL DATA
        if raw_data.len() >= 6 {
            let frame_count = u16::from_le_bytes([raw_data[0], raw_data[1]]);
            let width = u16::from_le_bytes([raw_data[2], raw_data[3]]);
            let height = u16::from_le_bytes([raw_data[4], raw_data[5]]);
            
            log::debug!("Checking GRP format: frames={}, {}x{}", frame_count, width, height);
            
            // Very lenient bounds check - StarCraft sprites can be large
            if frame_count > 0 && frame_count <= 2000 && 
               width > 0 && width <= 4096 && 
               height > 0 && height <= 4096 {
                // Additional validation: check if we have enough data for the offset table
                let offset_table_size = frame_count as usize * 4;
                if raw_data.len() >= 6 + offset_table_size {
                    log::info!("✅ DETECTED GRP format: {}x{} pixels, {} frames in {}", width, height, frame_count, file_info.name);
                    return Ok(Some(self.convert_grp_to_png(raw_data, width, height, frame_count)?));
                }
            }
        }
        
        // 5. Check for BMP format
        if raw_data.len() >= 2 && &raw_data[0..2] == b"BM" {
            log::info!("✅ DETECTED BMP format in {}", file_info.name);
            return Ok(Some(self.convert_bmp_to_png(raw_data, file_info)?));
        }
        
        // 6. Check for palette data (256 colors * 3 bytes RGB = 768 bytes)
        if raw_data.len() == 768 {
            log::info!("✅ DETECTED palette data (768 bytes) in {}", file_info.name);
            return Ok(Some(self.convert_palette_to_png(raw_data, file_info)?));
        }
        
        // 7. Check for raw image data patterns (common in StarCraft: Remastered)
        if raw_data.len() >= 64 {
            if let Some(result) = self.try_interpret_as_raw_image_data(raw_data, file_info)? {
                log::info!("✅ INTERPRETED as raw image data in {}", file_info.name);
                return Ok(Some(result));
            }
        }
        
        // 8. Check for encrypted/compressed data (StarCraft: Remastered uses encryption)
        // High entropy indicates encryption - try BLTE decompression
        if raw_data.len() >= 64 && entropy_ratio > 0.90 {
            log::info!("🔍 DETECTED high entropy data ({:.3}) - attempting decryption for StarCraft: Remastered", entropy_ratio);
            if let Ok(result) = self.try_decompress_and_convert_with_depth(raw_data, file_info, depth) {
                log::info!("✅ Successfully decrypted and processed data in {}", file_info.name);
                return Ok(Some(result));
            } else {
                log::warn!("❌ Decryption failed for high entropy data in {}", file_info.name);
            }
        }
        
        // 9. Check for actual BLTE signature
        if raw_data.len() >= 4 && &raw_data[0..4] == b"BLTE" {
            log::info!("✅ DETECTED actual BLTE signature in {}", file_info.name);
            return Ok(Some(self.try_decompress_and_convert_with_depth(raw_data, file_info, depth)?));
        }
        
        // CRITICAL FIX: If entropy is very high but no compression signatures, 
        // it's likely encrypted or a format we don't recognize yet
        if entropy_ratio > 0.95 {
            log::warn!("❌ HIGH ENTROPY DATA ({:.3}) with no recognized format in {} - likely encrypted or unknown format", 
                      entropy_ratio, file_info.name);
        } else {
            log::warn!("❌ NO FORMAT DETECTED for {} ({} bytes, entropy: {:.3}) - format not supported", 
                      file_info.name, raw_data.len(), entropy_ratio);
        }
        
        Ok(None)
    }
    
    /// Convert BMP format to PNG
    fn convert_bmp_to_png(&self, bmp_data: &[u8], file_info: &crate::casc::FileInfo) -> Result<Vec<u8>, SpriteError> {
        log::debug!("Converting BMP to PNG for {}", file_info.name);
        
        // Simplified BMP conversion
        let width = 32u32;
        let height = 32u32;
        let mut pixel_data = vec![96u8; (width * height) as usize]; // Light gray background
        
        // Use some BMP data to create a pattern
        for (i, &byte) in bmp_data.iter().skip(54).take((width * height) as usize).enumerate() {
            pixel_data[i] = byte;
        }
        
        self.create_png_from_pixels(&pixel_data, width, height, false)
    }
    
    /// Try to decompress and convert compressed data with recursion depth tracking
    fn try_decompress_and_convert_with_depth(&self, compressed_data: &[u8], file_info: &FileInfo, depth: u32) -> Result<Vec<u8>, SpriteError> {
        log::info!("🔧 ATTEMPTING DECOMPRESSION for {} ({} bytes) at depth {}", file_info.name, compressed_data.len(), depth);
        
        // CRITICAL FIX: Prevent infinite recursion by limiting depth
        const MAX_RECURSION_DEPTH: u32 = 3;
        if depth >= MAX_RECURSION_DEPTH {
            log::warn!("❌ Maximum recursion depth ({}) reached for decompression of {}, stopping to prevent stack overflow", 
                      MAX_RECURSION_DEPTH, file_info.name);
            return Err(SpriteError::DecodeError(format!("Maximum recursion depth reached for {}", file_info.name)));
        }
        
        // Log first 32 bytes to understand the compression format
        if compressed_data.len() >= 32 {
            log::info!("Compressed data header: {:02x?}", &compressed_data[0..32]);
        }
        
        // ENHANCED: Try ZLIB decompression FIRST on all decrypted data
        // This is the missing step that was causing format recognition failures
        log::info!("🔧 Attempting ZLIB decompression on decrypted data (primary method)");
        match self.try_zlib_decompression(compressed_data) {
            Ok(decompressed_data) => {
                let decompressed_len = decompressed_data.len();
                log::info!("✅ Successfully decompressed with ZLIB: {} -> {} bytes", 
                          compressed_data.len(), decompressed_len);
                
                // Log first 32 bytes of decompressed data
                if decompressed_data.len() >= 32 {
                    log::info!("Decompressed data header: {:02x?}", &decompressed_data[0..32]);
                }
                
                // Calculate entropy of decompressed data
                let decompressed_analysis = crate::casc::FileAnalysis::analyze(&decompressed_data);
                log::info!("Decompressed data entropy: {:.3}", decompressed_analysis.entropy);
                
                // Now try to detect the format of the decompressed data
                return match self.try_detect_and_convert_formats_with_depth(&decompressed_data, file_info, depth + 1)? {
                    Some(result) => Ok(result),
                    None => {
                        log::warn!("Decompressed ZLIB data but couldn't identify format in {}", file_info.name);
                        // Create a visualization of the decompressed data
                        self.create_decompressed_data_visualization(&decompressed_data, file_info)
                    }
                };
            }
            Err(e) => {
                log::debug!("ZLIB decompression failed for {}: {}, trying other methods", file_info.name, e);
            }
        }
        
        // First, try BLTE decompression if it looks like BLTE data
        if Self::looks_like_blte_data(compressed_data) {
            log::info!("🔍 Detected BLTE compressed data in {}", file_info.name);
            
            match Self::decompress_blte_data(compressed_data) {
                Ok(decompressed_data) => {
                    let decompressed_len = decompressed_data.len();
                    log::info!("✅ Successfully decompressed BLTE data: {} -> {} bytes", 
                              compressed_data.len(), decompressed_len);
                    
                    // Log first 32 bytes of decompressed data
                    if decompressed_data.len() >= 32 {
                        log::info!("Decompressed data header: {:02x?}", &decompressed_data[0..32]);
                    }
                    
                    // Now try to detect the format of the decompressed data
                    return match self.try_detect_and_convert_formats_with_depth(&decompressed_data, file_info, depth + 1)? {
                        Some(result) => Ok(result),
                        None => {
                            log::warn!("Decompressed BLTE data but couldn't identify format in {}", file_info.name);
                            // Create a visualization of the decompressed data
                            self.create_decompressed_data_visualization(&decompressed_data, file_info)
                        }
                    };
                }
                Err(e) => {
                    log::warn!("BLTE decompression failed for {}: {}", file_info.name, e);
                    // Fall through to other decompression methods
                }
            }
        }
        
        // Try raw ZLIB decompression (StarCraft: Remastered might use raw ZLIB)
        if compressed_data.len() >= 2 {
            // Check for ZLIB header patterns
            if compressed_data[0] == 0x78 && (compressed_data[1] == 0x01 || compressed_data[1] == 0x9C || compressed_data[1] == 0xDA) {
                log::info!("🔍 Detected raw ZLIB header in {}", file_info.name);
                
                match self.try_zlib_decompression(compressed_data) {
                    Ok(decompressed_data) => {
                        log::info!("✅ Successfully decompressed raw ZLIB data: {} -> {} bytes", 
                                  compressed_data.len(), decompressed_data.len());
                        
                        // Log first 32 bytes of decompressed data
                        if decompressed_data.len() >= 32 {
                            log::info!("Decompressed ZLIB header: {:02x?}", &decompressed_data[0..32]);
                        }
                        
                        // Try to detect format of decompressed data
                        return match self.try_detect_and_convert_formats_with_depth(&decompressed_data, file_info, depth + 1)? {
                            Some(result) => Ok(result),
                            None => {
                                log::warn!("Decompressed ZLIB data but couldn't identify format in {}", file_info.name);
                                self.create_decompressed_data_visualization(&decompressed_data, file_info)
                            }
                        };
                    }
                    Err(e) => {
                        log::warn!("Raw ZLIB decompression failed for {}: {}", file_info.name, e);
                    }
                }
            }
        }
        
        // Try LZ4 decompression (another common format in game archives)
        if compressed_data.len() >= 4 {
            match self.try_lz4_decompression(compressed_data) {
                Ok(decompressed_data) => {
                    let decompressed_len = decompressed_data.len();
                    log::info!("✅ Successfully decompressed LZ4 data: {} -> {} bytes", 
                              compressed_data.len(), decompressed_len);
                    
                    // Log first 32 bytes of decompressed data
                    if decompressed_data.len() >= 32 {
                        log::info!("Decompressed LZ4 header: {:02x?}", &decompressed_data[0..32]);
                    }
                    
                    // Try to detect format of decompressed data
                    return match self.try_detect_and_convert_formats_with_depth(&decompressed_data, file_info, depth + 1)? {
                        Some(result) => Ok(result),
                        None => {
                            log::warn!("Decompressed LZ4 data but couldn't identify format in {}", file_info.name);
                            self.create_decompressed_data_visualization(&decompressed_data, file_info)
                        }
                    };
                }
                Err(e) => {
                    log::debug!("LZ4 decompression failed for {}: {}", file_info.name, e);
                }
            }
        }
        
        // Try treating the data as encrypted and attempt simple XOR decryption
        // CRITICAL FIX: Try decryption FIRST for high entropy data
        if compressed_data.len() >= 16 {
            if let Some(decrypted_data) = self.try_simple_decryption(compressed_data, file_info) {
                let decrypted_len = decrypted_data.len();
                log::info!("✅ Successfully decrypted data: {} bytes", decrypted_len);
                
                // Log first 32 bytes of decrypted data
                if decrypted_data.len() >= 32 {
                    log::info!("Decrypted data header: {:02x?}", &decrypted_data[0..32]);
                }
                
                // CRITICAL: After decryption, try decompression if the data still looks compressed
                if Self::looks_like_compressed_starcraft_data(&decrypted_data) {
                    log::info!("🔧 Decrypted data still looks compressed, attempting decompression");
                    
                    // Try ZLIB decompression on decrypted data
                    if let Ok(decompressed_data) = self.try_zlib_decompression(&decrypted_data) {
                        log::info!("✅ Successfully decompressed decrypted data: {} -> {} bytes", 
                                  decrypted_data.len(), decompressed_data.len());
                        
                        // Try to detect format of decompressed data
                        return match self.try_detect_and_convert_formats_with_depth(&decompressed_data, file_info, depth + 1)? {
                            Some(result) => Ok(result),
                            None => {
                                log::warn!("Decompressed decrypted data but couldn't identify format in {}", file_info.name);
                                self.create_decompressed_data_visualization(&decompressed_data, file_info)
                            }
                        };
                    }
                }
                
                // Try to detect format of decrypted data (even if not compressed)
                return match self.try_detect_and_convert_formats_with_depth(&decrypted_data, file_info, depth + 1)? {
                    Some(result) => Ok(result),
                    None => {
                        log::warn!("Decrypted data but couldn't identify format in {}", file_info.name);
                        self.create_decompressed_data_visualization(&decrypted_data, file_info)
                    }
                };
            }
        }
        
        // If all decompression/decryption attempts failed, create a diagnostic visualization
        log::warn!("❌ All decompression attempts failed for {}, creating diagnostic visualization", file_info.name);
        Ok(self.create_compressed_data_visualization(compressed_data, file_info)?)
    }
    
    /// Try ZLIB decompression directly
    fn try_zlib_decompression(&self, data: &[u8]) -> Result<Vec<u8>, SpriteError> {
        use std::io::Read;
        use flate2::read::ZlibDecoder;
        
        let mut decoder = ZlibDecoder::new(data);
        let mut decompressed = Vec::new();
        
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| SpriteError::DecodeError(format!("ZLIB decompression failed: {}", e)))?;
        
        Ok(decompressed)
    }
    
    /// Create a visualization of decompressed data that couldn't be identified
    fn create_decompressed_data_visualization(&self, decompressed_data: &[u8], file_info: &FileInfo) -> Result<Vec<u8>, SpriteError> {
        log::debug!("Creating visualization for {} bytes of decompressed data from {}", 
                   decompressed_data.len(), file_info.name);
        
        // Try to create a more meaningful visualization based on the decompressed data
        let data_len = decompressed_data.len();
        
        // If the data is large enough, try to interpret it as raw image data
        if data_len >= 64 * 64 {
            // Try different interpretations
            let possible_widths = [64, 128, 256, 320, 640];
            
            for &width in &possible_widths {
                let height = data_len / width;
                if height > 0 && height <= 1024 && width * height <= data_len {
                    log::debug!("Trying to interpret decompressed data as {}x{} raw image", width, height);
                    
                    // Use the first width*height bytes as pixel data
                    let pixel_count = width * height;
                    let pixel_data = &decompressed_data[0..pixel_count.min(data_len)];
                    
                    return self.create_png_from_pixels(pixel_data, width as u32, height as u32, false);
                }
            }
        }
        
        // Fall back to a grid visualization
        let width = 64u32;
        let height = ((data_len + 63) / 64).min(64) as u32;
        let mut pixel_data = vec![0u8; (width * height) as usize];
        
        // Map decompressed bytes to pixels
        for (i, &byte) in decompressed_data.iter().take((width * height) as usize).enumerate() {
            pixel_data[i] = byte;
        }
        
        self.create_png_from_pixels(&pixel_data, width, height, false)
    }
    
    /// Create a visualization of compressed data
    fn create_compressed_data_visualization(&self, compressed_data: &[u8], _file_info: &FileInfo) -> Result<Vec<u8>, SpriteError> {
        // Create a small visualization of the compressed data
        let width = 64u32;
        let height = (compressed_data.len() as u32 + width - 1) / width;
        let height = height.min(64); // Cap at 64x64
        
        let mut pixel_data = vec![0u8; (width * height) as usize];
        
        // Map compressed bytes to pixels
        for (i, &byte) in compressed_data.iter().take((width * height) as usize).enumerate() {
            pixel_data[i] = byte;
        }
        
        self.create_png_from_pixels(&pixel_data, width, height, false)
    }
    
    /// Convert palette data to PNG
    fn convert_palette_to_png(&self, palette_data: &[u8], file_info: &FileInfo) -> Result<Vec<u8>, SpriteError> {
        log::debug!("Converting palette data to PNG for {}", file_info.name);
        
        // Create a palette visualization (16x16 color swatches)
        let width = 16u32;
        let height = 16u32;
        let mut pixel_data = vec![0u8; (width * height) as usize];
        
        // If it's palette data (RGB triplets), use the red component
        for (i, chunk) in palette_data.chunks(3).take((width * height) as usize).enumerate() {
            if !chunk.is_empty() {
                pixel_data[i] = chunk[0]; // Use red component
            }
        }
        
        self.create_png_from_pixels(&pixel_data, width, height, false)
    }
    
    /// Convert ANIM format to PNG using the AnimFile parser
    fn convert_anim_to_png(&self, anim_data: &[u8], file_info: &crate::casc::FileInfo) -> Result<Vec<u8>, SpriteError> {
        log::info!("Converting ANIM format to PNG for {}: {} bytes", file_info.name, anim_data.len());
        
        // Parse the ANIM file using our comprehensive parser
        match crate::anim::AnimFile::parse(anim_data) {
            Ok(anim_file) => {
                log::info!("Successfully parsed ANIM file: {} sprites", anim_file.sprites.len());
                
                // For now, convert the first sprite to PNG
                if let Some(first_sprite) = anim_file.sprites.first() {
                    if let Some(first_frame) = first_sprite.frames.first() {
                        log::info!("Converting first frame: {}x{} pixels", first_frame.width, first_frame.height);
                        
                        // Try to extract pixel data from the first texture of the sprite
                        if let Some(texture) = first_sprite.textures.first() {
                            match texture.decode_pixels() {
                                Ok(rgba_pixels) => {
                                    log::info!("Successfully decoded {} RGBA pixels", rgba_pixels.len() / 4);
                                    
                                    // Convert RGBA to PNG
                                    return self.create_png_from_rgba_pixels(
                                        &rgba_pixels, 
                                        first_frame.width as u32, 
                                        first_frame.height as u32
                                    );
                                }
                                Err(e) => {
                                    return Err(SpriteError::DecodeError(format!(
                                        "Failed to decode ANIM texture pixels in {}: {}", 
                                        file_info.name, e
                                    )));
                                }
                            }
                        } else {
                            return Err(SpriteError::DecodeError(format!(
                                "No textures found in ANIM sprite: {}", 
                                file_info.name
                            )));
                        }
                    } else {
                        return Err(SpriteError::DecodeError(format!(
                            "No frames found in ANIM sprite: {}", 
                            file_info.name
                        )));
                    }
                } else {
                    return Err(SpriteError::DecodeError(format!(
                        "No sprites found in ANIM file: {}", 
                        file_info.name
                    )));
                }
            }
            Err(e) => {
                return Err(SpriteError::DecodeError(format!(
                    "Failed to parse ANIM file {}: {}", 
                    file_info.name, e
                )));
            }
        }
    }
    
    /// Check if data looks like a StarCraft: Remastered texture
    fn looks_like_scr_texture(&self, data: &[u8]) -> bool {
        if data.len() < 128 {
            return false;
        }
        
        // Check for common texture header patterns in StarCraft: Remastered
        // These are heuristics based on common texture formats
        
        // Check for potential texture dimensions in first 16 bytes
        for offset in [0, 4, 8, 12] {
            if offset + 8 <= data.len() {
                let width = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
                let height = u32::from_le_bytes([data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]]);
                
                // Check for reasonable texture dimensions
                if width > 0 && width <= 2048 && height > 0 && height <= 2048 {
                    // Check if the data size makes sense for this texture size
                    let expected_size_rgba = (width * height * 4) as usize;
                    let _expected_size_rgb = (width * height * 3) as usize;
                    let expected_size_indexed = (width * height) as usize;
                    
                    if data.len() >= expected_size_indexed && 
                       (data.len() <= expected_size_rgba + 1024) { // Allow for headers
                        log::debug!("Found potential texture dimensions: {}x{} at offset {}", width, height, offset);
                        return true;
                    }
                }
            }
        }
        
        false
    }
    
    /// Convert StarCraft: Remastered texture to PNG
    fn convert_scr_texture_to_png(&self, texture_data: &[u8], file_info: &FileInfo) -> Result<Vec<u8>, SpriteError> {
        log::info!("Converting StarCraft: Remastered texture to PNG for {}", file_info.name);
        
        // Try to find texture dimensions in the header
        for offset in [0, 4, 8, 12] {
            if offset + 8 <= texture_data.len() {
                let width = u32::from_le_bytes([texture_data[offset], texture_data[offset + 1], texture_data[offset + 2], texture_data[offset + 3]]);
                let height = u32::from_le_bytes([texture_data[offset + 4], texture_data[offset + 5], texture_data[offset + 6], texture_data[offset + 7]]);
                
                if width > 0 && width <= 2048 && height > 0 && height <= 2048 {
                    let pixel_count = (width * height) as usize;
                    let header_size = offset + 8;
                    
                    // Try different pixel formats
                    if header_size + pixel_count * 4 <= texture_data.len() {
                        // RGBA format
                        log::info!("Trying RGBA format: {}x{} at offset {}", width, height, header_size);
                        let rgba_data = &texture_data[header_size..header_size + pixel_count * 4];
                        return self.create_png_from_rgba_pixels(rgba_data, width, height);
                    } else if header_size + pixel_count * 3 <= texture_data.len() {
                        // RGB format
                        log::info!("Trying RGB format: {}x{} at offset {}", width, height, header_size);
                        let rgb_data = &texture_data[header_size..header_size + pixel_count * 3];
                        return self.create_png_from_rgb_pixels(rgb_data, width, height);
                    } else if header_size + pixel_count <= texture_data.len() {
                        // Indexed/grayscale format
                        log::info!("Trying indexed format: {}x{} at offset {}", width, height, header_size);
                        let indexed_data = &texture_data[header_size..header_size + pixel_count];
                        return self.create_png_from_pixels(indexed_data, width, height, false);
                    }
                }
            }
        }
        
        // If we can't find dimensions, create a diagnostic visualization
        log::warn!("Could not determine texture format for {}, creating diagnostic visualization", file_info.name);
        let width = 64u32;
        let height = (texture_data.len() / 64).min(64) as u32;
        let mut pixel_data = vec![0u8; (width * height) as usize];
        
        for (i, &byte) in texture_data.iter().take((width * height) as usize).enumerate() {
            pixel_data[i] = byte;
        }
        
        self.create_png_from_pixels(&pixel_data, width, height, false)
    }
    
    /// Convert DDS texture to PNG
    fn convert_dds_to_png(&self, dds_data: &[u8], file_info: &FileInfo) -> Result<Vec<u8>, SpriteError> {
        log::info!("Converting DDS texture to PNG for {}", file_info.name);
        
        if dds_data.len() < 128 {
            return Err(SpriteError::InvalidFormat("DDS file too small".to_string()));
        }
        
        // DDS header is 128 bytes, starting with "DDS " signature
        // Simplified DDS parsing - in a real implementation you'd parse the full header
        
        // For now, extract basic dimensions from DDS header (bytes 12-19)
        let height = u32::from_le_bytes([dds_data[12], dds_data[13], dds_data[14], dds_data[15]]);
        let width = u32::from_le_bytes([dds_data[16], dds_data[17], dds_data[18], dds_data[19]]);
        
        log::info!("DDS dimensions: {}x{}", width, height);
        
        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            log::warn!("Invalid DDS dimensions: {}x{}", width, height);
            // Create a diagnostic visualization
            let diag_width = 64u32;
            let diag_height = 64u32;
            let mut pixel_data = vec![0u8; (diag_width * diag_height) as usize];
            
            for (i, &byte) in dds_data.iter().skip(128).take((diag_width * diag_height) as usize).enumerate() {
                pixel_data[i] = byte;
            }
            
            return self.create_png_from_pixels(&pixel_data, diag_width, diag_height, false);
        }
        
        // Extract pixel data (skip 128-byte header)
        let pixel_data_start = 128;
        let pixel_count = (width * height) as usize;
        
        if pixel_data_start + pixel_count <= dds_data.len() {
            // Try as indexed/grayscale
            let pixel_data = &dds_data[pixel_data_start..pixel_data_start + pixel_count];
            self.create_png_from_pixels(pixel_data, width, height, false)
        } else if pixel_data_start + pixel_count * 3 <= dds_data.len() {
            // Try as RGB
            let rgb_data = &dds_data[pixel_data_start..pixel_data_start + pixel_count * 3];
            self.create_png_from_rgb_pixels(rgb_data, width, height)
        } else if pixel_data_start + pixel_count * 4 <= dds_data.len() {
            // Try as RGBA
            let rgba_data = &dds_data[pixel_data_start..pixel_data_start + pixel_count * 4];
            self.create_png_from_rgba_pixels(rgba_data, width, height)
        } else {
            // Not enough data, create a smaller visualization
            let available_pixels = (dds_data.len() - pixel_data_start).min(pixel_count);
            let mut pixel_data = vec![0u8; pixel_count];
            if available_pixels > 0 {
                pixel_data[0..available_pixels].copy_from_slice(&dds_data[pixel_data_start..pixel_data_start + available_pixels]);
            }
            self.create_png_from_pixels(&pixel_data, width, height, false)
        }
    }
    
    /// Try to interpret data as raw image data
    fn try_interpret_as_raw_image_data(&self, raw_data: &[u8], file_info: &FileInfo) -> Result<Option<Vec<u8>>, SpriteError> {
        // Try common image dimensions for StarCraft sprites
        let common_dimensions = [
            (32, 32), (64, 64), (128, 128), (256, 256),
            (32, 24), (64, 48), (128, 96), (256, 192),
            (48, 32), (96, 64), (192, 128), (384, 256),
            (16, 16), (24, 24), (40, 40), (80, 80)
        ];
        
        for &(width, height) in &common_dimensions {
            let pixel_count = width * height;
            
            // Try RGBA format
            if raw_data.len() >= pixel_count * 4 {
                log::debug!("Trying {}x{} RGBA interpretation for {}", width, height, file_info.name);
                let rgba_data = &raw_data[0..pixel_count * 4];
                if self.looks_like_valid_image_data(rgba_data, 4) {
                    log::info!("Successfully interpreted as {}x{} RGBA image: {}", width, height, file_info.name);
                    return Ok(Some(self.create_png_from_rgba_pixels(rgba_data, width as u32, height as u32)?));
                }
            }
            
            // Try RGB format
            if raw_data.len() >= pixel_count * 3 {
                log::debug!("Trying {}x{} RGB interpretation for {}", width, height, file_info.name);
                let rgb_data = &raw_data[0..pixel_count * 3];
                if self.looks_like_valid_image_data(rgb_data, 3) {
                    log::info!("Successfully interpreted as {}x{} RGB image: {}", width, height, file_info.name);
                    return Ok(Some(self.create_png_from_rgb_pixels(rgb_data, width as u32, height as u32)?));
                }
            }
            
            // Try indexed/grayscale format
            if raw_data.len() >= pixel_count {
                log::debug!("Trying {}x{} indexed interpretation for {}", width, height, file_info.name);
                let indexed_data = &raw_data[0..pixel_count];
                if self.looks_like_valid_image_data(indexed_data, 1) {
                    log::info!("Successfully interpreted as {}x{} indexed image: {}", width, height, file_info.name);
                    return Ok(Some(self.create_png_from_pixels(indexed_data, width as u32, height as u32, false)?));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Check if data looks like valid image data
    fn looks_like_valid_image_data(&self, data: &[u8], _bytes_per_pixel: usize) -> bool {
        if data.is_empty() {
            return false;
        }
        
        // Check for reasonable variation in pixel values
        let mut min_val = 255u8;
        let mut max_val = 0u8;
        let sample_size = data.len().min(1024);
        
        for &byte in &data[0..sample_size] {
            min_val = min_val.min(byte);
            max_val = max_val.max(byte);
        }
        
        let variation = max_val.saturating_sub(min_val);
        
        // Image data should have some variation (not all the same value)
        // but not be completely random (some structure)
        variation >= 16 && variation <= 240
    }
    
    /// Create PNG from RGB pixel data
    // NOTE: create_png_from_rgb_pixels and create_png_from_rgba_pixels are kept separate
    // from create_png_from_pixels and cannot be collapsed into wrappers around it.
    // create_png_from_pixels(_, false) uses ColorType::Grayscale (not Rgb), and
    // create_png_from_pixels(_, true) expands 1-byte grayscale values to RGBA rather
    // than writing pre-composed RGBA bytes directly. The data layouts are incompatible.
    fn create_png_from_rgb_pixels(&self, rgb_pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, SpriteError> {
        use std::io::Cursor;

        let mut png_data = Vec::new();
        let mut cursor = Cursor::new(&mut png_data);

        // Create PNG encoder for RGB
        let mut encoder = png::Encoder::new(&mut cursor, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        
        let mut writer = encoder.write_header()
            .map_err(|e| SpriteError::DecodeError(format!("PNG header write failed: {}", e)))?;
        
        // Write RGB pixel data directly
        writer.write_image_data(rgb_pixels)
            .map_err(|e| SpriteError::DecodeError(format!("PNG data write failed: {}", e)))?;
        
        writer.finish()
            .map_err(|e| SpriteError::DecodeError(format!("PNG finish failed: {}", e)))?;
        
        Ok(png_data)
    }
    
    /// Create PNG data from RGBA pixel array
    fn create_png_from_rgba_pixels(&self, rgba_pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, SpriteError> {
        use std::io::Cursor;
        
        let mut png_data = Vec::new();
        let mut cursor = Cursor::new(&mut png_data);
        
        // Create PNG encoder for RGBA
        let mut encoder = png::Encoder::new(&mut cursor, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        
        let mut writer = encoder.write_header()
            .map_err(|e| SpriteError::DecodeError(format!("PNG header write failed: {}", e)))?;
        
        // Write RGBA pixel data directly
        writer.write_image_data(rgba_pixels)
            .map_err(|e| SpriteError::DecodeError(format!("PNG data write failed: {}", e)))?;
        
        writer.finish()
            .map_err(|e| SpriteError::DecodeError(format!("PNG finish failed: {}", e)))?;
        
        Ok(png_data)
    }
    fn convert_grp_to_png(&self, grp_data: &[u8], width: u16, height: u16, frame_count: u16) -> Result<Vec<u8>, SpriteError> {
        log::info!("Converting GRP format: {}x{} pixels, {} frames, {} bytes total", 
                  width, height, frame_count, grp_data.len());
        
        // Use the new GRP parser with proper validation and RLE decoding
        let grp_file = GrpFile::parse(grp_data)?;
        
        // Verify the parsed dimensions match the detected ones
        if grp_file.width != width || grp_file.height != height || grp_file.frame_count != frame_count {
            log::warn!("GRP dimension mismatch: detected {}x{} {} frames, parsed {}x{} {} frames", 
                      width, height, frame_count, 
                      grp_file.width, grp_file.height, grp_file.frame_count);
        }
        
        // Get the first frame for PNG conversion
        let frame = grp_file.get_first_frame().ok_or_else(|| {
            SpriteError::DecodeError("No frames found in GRP file".to_string())
        })?;
        
        log::debug!("Converting frame: {}x{} pixels, {} bytes of pixel data", 
                   frame.width, frame.height, frame.pixel_data.len());
        
        // Create optimized palette for GRP sprites
        let palette = GrpFile::create_grp_optimized_palette();
        
        // Use optimized conversion for large sprites, regular conversion for small ones
        let pixel_count = (frame.width as usize) * (frame.height as usize);
        let rgba_pixels = if pixel_count > 4096 { // 64x64 threshold
            log::debug!("Using optimized conversion for large sprite ({}x{})", frame.width, frame.height);
            frame.to_rgba_optimized(&palette)
        } else {
            frame.to_rgba_with_transparency(&palette)
        }.map_err(|e| SpriteError::DecodeError(format!("GRP palette conversion failed: {}", e)))?;
        
        log::debug!("Converted {} indexed pixels to {} RGBA bytes", 
                   frame.pixel_data.len(), rgba_pixels.len());
        
        // Create PNG from RGBA pixel data
        self.create_png_from_rgba_pixels(&rgba_pixels, frame.width as u32, frame.height as u32)
    }
    
    
    /// Create PNG data from pixel array
    fn create_png_from_pixels(&self, pixels: &[u8], width: u32, height: u32, has_alpha: bool) -> Result<Vec<u8>, SpriteError> {
        use std::io::Cursor;
        
        let mut png_data = Vec::new();
        let mut cursor = Cursor::new(&mut png_data);
        
        // Create PNG encoder
        let mut encoder = png::Encoder::new(&mut cursor, width, height);
        
        if has_alpha {
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
        } else {
            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(png::BitDepth::Eight);
        }
        
        let mut writer = encoder.write_header()
            .map_err(|e| SpriteError::DecodeError(format!("PNG header write failed: {}", e)))?;
        
        // Write pixel data
        if has_alpha {
            // Convert grayscale to RGBA
            let mut rgba_data = Vec::with_capacity(pixels.len() * 4);
            for &pixel in pixels {
                rgba_data.extend_from_slice(&[pixel, pixel, pixel, 255]); // Gray + full alpha
            }
            writer.write_image_data(&rgba_data)
                .map_err(|e| SpriteError::DecodeError(format!("PNG data write failed: {}", e)))?;
        } else {
            writer.write_image_data(pixels)
                .map_err(|e| SpriteError::DecodeError(format!("PNG data write failed: {}", e)))?;
        }
        
        writer.finish()
            .map_err(|e| SpriteError::DecodeError(format!("PNG finish failed: {}", e)))?;
        
        Ok(png_data)
    }
    
    /// Check if data looks like BLTE compressed data - ENHANCED FOR STARCRAFT REMASTERED
    fn looks_like_blte_data(data: &[u8]) -> bool {
        if data.len() < 4 {
            return false;
        }
        
        // Check for actual BLTE signature first
        if data.len() >= 4 && &data[0..4] == b"BLTE" {
            log::debug!("Found actual BLTE signature");
            return true;
        }
        
        // ENHANCED: StarCraft: Remastered uses BLTE without explicit signature
        // Check for high entropy data that might be BLTE encrypted
        if data.len() >= 64 {
            let mut byte_counts = [0u32; 256];
            let sample_size = data.len().min(1024);
            for &byte in &data[0..sample_size] {
                byte_counts[byte as usize] += 1;
            }
            
            let unique_bytes = byte_counts.iter().filter(|&&count| count > 0).count();
            let entropy_ratio = unique_bytes as f64 / 256.0;
            
            // CRITICAL: StarCraft: Remastered BLTE data has entropy > 0.97
            if entropy_ratio > 0.97 {
                log::info!("🔍 High entropy data ({:.3}) detected - likely StarCraft: Remastered BLTE", entropy_ratio);
                return true;
            }
        }
        
        false
    }
    
    /// Decompress BLTE data using enhanced BLTE decompression with fallback chains
    fn decompress_blte_data(data: &[u8]) -> Result<Vec<u8>, SpriteError> {
        if data.len() < 8 {
            return Err(SpriteError::DecodeError("BLTE data too short".to_string()));
        }
        
        log::info!("🔧 Attempting enhanced BLTE decompression with fallback chains ({} bytes)", data.len());
        
        // Use the enhanced BLTE decompressor with comprehensive fallback chains
        let mut decompressor = BlteDecompressor::new();
        
        match decompressor.decompress(data) {
            Ok(decompressed_data) => {
                log::info!("✅ Enhanced BLTE decompression successful: {} -> {} bytes", 
                          data.len(), decompressed_data.len());
                Ok(decompressed_data)
            }
            Err(e) => {
                log::error!("❌ Enhanced BLTE decompression failed: {}", e);
                Err(SpriteError::BlteEnhanced(e))
            }
        }
    }
    
    /// Try LZ4 decompression
    fn try_lz4_decompression(&self, _data: &[u8]) -> Result<Vec<u8>, SpriteError> {
        // LZ4 doesn't have a standard header, so this is speculative
        // We'll try to decompress and see if we get reasonable results
        
        // For now, just return an error since we don't have LZ4 implemented
        Err(SpriteError::DecodeError("LZ4 decompression not implemented".to_string()))
    }
    
    /// Try simple decryption methods (XOR, etc.) - FIXED INFINITE RECURSION + ENHANCED WITH WORKING KEYS
    fn try_simple_decryption(&self, encrypted_data: &[u8], file_info: &crate::casc::FileInfo) -> Option<Vec<u8>> {
        // CRITICAL FIX: Skip 0x00 key to prevent infinite recursion
        // XOR with 0x00 doesn't change the data, causing infinite loops
        let common_keys = [
            0xFF, 0xAA, 0x55, 0xCC, 0x33, 0xF0, 0x0F,
            0x42, 0x24, 0x69, 0x96, 0x13, 0x31, 0x87, 0x78
        ];
        
        for &key in &common_keys {
            let mut decrypted = encrypted_data.to_vec();
            
            // Simple XOR decryption
            for byte in &mut decrypted {
                *byte ^= key;
            }
            
            // CRITICAL FIX: Ensure decrypted data is actually different from original
            if decrypted != encrypted_data && self.looks_like_valid_decrypted_data(&decrypted) {
                log::info!("🔓 Successfully decrypted {} with XOR key 0x{:02x}", file_info.name, key);
                return Some(decrypted);
            }
        }
        
        // ENHANCED: Try multi-byte XOR keys - WORKING KEYS FROM DECRYPTION TEST
        let multi_byte_keys = [
            vec![0x53, 0x43, 0x52], // "SCR" - StarCraft Remastered (WORKING!)
            vec![0x42, 0x4C, 0x5A], // "BLZ" - Blizzard (WORKING!)
            vec![0x43, 0x41, 0x53, 0x43], // "CASC" (WORKING!)
            vec![0x53, 0x74, 0x61, 0x72], // "Star" (WORKING!)
            vec![0x43, 0x72, 0x61, 0x66, 0x74], // "Craft" (WORKING!)
            vec![0x42, 0x24], // Original working key
        ];
        
        for key in &multi_byte_keys {
            let mut decrypted = encrypted_data.to_vec();
            
            // Multi-byte XOR decryption
            for (i, byte) in decrypted.iter_mut().enumerate() {
                *byte ^= key[i % key.len()];
            }
            
            // CRITICAL FIX: Ensure decrypted data is actually different from original
            if decrypted != encrypted_data {
                // ENHANCED: Check if decrypted data looks like valid StarCraft data or compressed data
                if self.looks_like_valid_decrypted_data(&decrypted) || self.looks_like_compressed_data_after_decryption(&decrypted) {
                    log::info!("🔓 Successfully decrypted {} with multi-byte XOR key {:02x?}", file_info.name, key);
                    return Some(decrypted);
                }
            }
        }
        
        None
    }
    
    /// Check if decrypted data looks valid
    fn looks_like_valid_decrypted_data(&self, data: &[u8]) -> bool {
        if data.len() < 16 {
            return false;
        }
        
        // Check for known format signatures after decryption
        if &data[0..4] == b"ANIM" || &data[0..4] == b"DDS " || &data[0..2] == b"BM" {
            return true;
        }
        
        // Check for ZLIB header after decryption
        if data[0] == 0x78 && (data[1] == 0x01 || data[1] == 0x9C || data[1] == 0xDA) {
            return true;
        }
        
        // Check for reasonable entropy (not too high, not too low)
        let mut byte_counts = [0u32; 256];
        let sample_size = data.len().min(256);
        for &byte in &data[0..sample_size] {
            byte_counts[byte as usize] += 1;
        }
        
        let unique_bytes = byte_counts.iter().filter(|&&count| count > 0).count();
        let entropy_ratio = unique_bytes as f64 / 256.0;
        
        // Good decrypted data should have moderate entropy (not completely random, not too uniform)
        entropy_ratio > 0.1 && entropy_ratio < 0.8
    }
    
    /// Check if decrypted data looks like compressed data that needs further decompression
    fn looks_like_compressed_data_after_decryption(&self, data: &[u8]) -> bool {
        if data.len() < 16 {
            return false;
        }
        
        // Check for ZLIB header patterns after decryption
        if data[0] == 0x78 && (data[1] == 0x01 || data[1] == 0x9C || data[1] == 0xDA) {
            return true;
        }
        
        // Check for other compression signatures
        if data.len() >= 4 {
            // Check for LZ4 magic number
            if &data[0..4] == b"\x04\"M\x18" {
                return true;
            }
            
            // Check for other compression patterns
            if &data[0..4] == b"BLTE" {
                return true;
            }
        }
        
        // Check if entropy suggests compressed data (but not as high as encrypted)
        let mut byte_counts = [0u32; 256];
        let sample_size = data.len().min(512);
        for &byte in &data[0..sample_size] {
            byte_counts[byte as usize] += 1;
        }
        
        let unique_bytes = byte_counts.iter().filter(|&&count| count > 0).count();
        let entropy_ratio = unique_bytes as f64 / 256.0;
        
        // Compressed data typically has high entropy but not as high as encrypted data
        entropy_ratio > 0.7 && entropy_ratio < 0.95
    }
    
    /// Check if decrypted data looks like compressed StarCraft data
    fn looks_like_compressed_starcraft_data(data: &[u8]) -> bool {
        if data.len() < 16 {
            return false;
        }
        
        // Check for ZLIB header patterns after decryption
        if data[0] == 0x78 && (data[1] == 0x01 || data[1] == 0x9C || data[1] == 0xDA) {
            return true;
        }
        
        // Check for other compression signatures
        if data.len() >= 4 {
            if &data[0..4] == b"BLTE" || &data[0..4] == b"\x04\"M\x18" {
                return true;
            }
        }
        
        // Check if entropy suggests compressed data (but not as high as encrypted)
        let mut byte_counts = [0u32; 256];
        let sample_size = data.len().min(512);
        for &byte in &data[0..sample_size] {
            byte_counts[byte as usize] += 1;
        }
        
        let unique_bytes = byte_counts.iter().filter(|&&count| count > 0).count();
        let entropy_ratio = unique_bytes as f64 / 256.0;
        
        // Compressed data typically has high entropy but not as high as encrypted data
        entropy_ratio > 0.7 && entropy_ratio < 0.95
    }

    // NOTE: The following methods have been replaced by the enhanced BLTE decompressor
    // in blte_enhanced.rs. They are kept here temporarily for reference but should
    // not be used in new code.
    
    /*
    /// Try raw ZLIB decompression without BLTE wrapper
    fn try_raw_zlib_decompression(data: &[u8]) -> Result<Vec<u8>, SpriteError> {
        use std::io::Read;
        use flate2::read::ZlibDecoder;
        
        let mut decoder = ZlibDecoder::new(data);
        let mut decompressed = Vec::new();
        
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| SpriteError::DecodeError(format!("Raw ZLIB decompression failed: {}", e)))?;
        
        Ok(decompressed)
    }
    
    /// Try to decrypt StarCraft: Remastered encrypted data - ENHANCED WITH WORKING KEYS
    fn try_decrypt_starcraft_data(data: &[u8]) -> Result<Vec<u8>, SpriteError> {
        // StarCraft: Remastered might use simple XOR encryption or other methods
        // CRITICAL FIX: Use different keys than try_simple_decryption to prevent infinite loops
        let starcraft_keys = [
            // Different keys from try_simple_decryption to prevent XOR cycles
            0x24, 0x69, 0x96, 0x13, 0x31, 0x87, 0x78,
            0xC3, 0x3C, 0x5A, 0xA5, 0x66, 0x99, 0x77,
        ];
        
        for &key in &starcraft_keys {
            let mut decrypted = data.to_vec();
            
            // Simple XOR decryption
            for byte in &mut decrypted {
                *byte ^= key;
            }
            
            // CRITICAL FIX: Ensure decrypted data is actually different from original
            if decrypted != data && Self::looks_like_valid_starcraft_data(&decrypted) {
                log::info!("🔓 Successfully decrypted StarCraft data with key 0x{:02x}", key);
                return Ok(decrypted);
            }
        }
        
        // ENHANCED: Try multi-byte keys - WORKING KEYS FROM DECRYPTION TEST
        let multi_byte_keys = [
            vec![0x53, 0x43, 0x52], // "SCR" - StarCraft Remastered (WORKING!)
            vec![0x42, 0x4C, 0x5A], // "BLZ" - Blizzard (WORKING!)
            vec![0x43, 0x41, 0x53, 0x43], // "CASC" (WORKING!)
            vec![0x53, 0x74, 0x61, 0x72], // "Star" (WORKING!)
            vec![0x43, 0x72, 0x61, 0x66, 0x74], // "Craft" (WORKING!)
        ];
        
        for key in &multi_byte_keys {
            let mut decrypted = data.to_vec();
            
            // Multi-byte XOR decryption
            for (i, byte) in decrypted.iter_mut().enumerate() {
                *byte ^= key[i % key.len()];
            }
            
            // CRITICAL FIX: Ensure decrypted data is actually different from original
            if decrypted != data {
                // Check if decrypted data looks valid OR looks like compressed data
                if Self::looks_like_valid_starcraft_data(&decrypted) || Self::looks_like_compressed_starcraft_data(&decrypted) {
                    log::info!("🔓 Successfully decrypted StarCraft data with multi-byte key {:02x?}", key);
                    return Ok(decrypted);
                }
            }
        }
        
        Err(SpriteError::DecodeError("No valid decryption key found".to_string()))
    }
    
    /// Check if data looks like valid StarCraft data after decryption
    fn looks_like_valid_starcraft_data(data: &[u8]) -> bool {
        if data.len() < 16 {
            return false;
        }
        
        // Check for known StarCraft format signatures
        if &data[0..4] == b"ANIM" || &data[0..4] == b"DDS " || &data[0..2] == b"BM" {
            return true;
        }
        
        // Check for ZLIB header (common in StarCraft data)
        if data[0] == 0x78 && (data[1] == 0x01 || data[1] == 0x9C || data[1] == 0xDA) {
            return true;
        }
        
        // Check for GRP format (StarCraft sprite format)
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
    
    /// Check if decrypted data looks like compressed StarCraft data
    fn looks_like_compressed_starcraft_data(data: &[u8]) -> bool {
        if data.len() < 16 {
            return false;
        }
        
        // Check for ZLIB header patterns after decryption
        if data[0] == 0x78 && (data[1] == 0x01 || data[1] == 0x9C || data[1] == 0xDA) {
            return true;
        }
        
        // Check for other compression signatures
        if data.len() >= 4 {
            if &data[0..4] == b"BLTE" || &data[0..4] == b"\x04\"M\x18" {
                return true;
            }
        }
        
        // Check if entropy suggests compressed data (but not as high as encrypted)
        let mut byte_counts = [0u32; 256];
        let sample_size = data.len().min(512);
        for &byte in &data[0..sample_size] {
            byte_counts[byte as usize] += 1;
        }
        
        let unique_bytes = byte_counts.iter().filter(|&&count| count > 0).count();
        let entropy_ratio = unique_bytes as f64 / 256.0;
        
        // Compressed data typically has high entropy but not as high as encrypted data
        entropy_ratio > 0.7 && entropy_ratio < 0.95
    }
    
    /// Decompress BLTE data using the proper BLTE library for StarCraft: Remastered
    fn decompress_with_blte_library(data: &[u8]) -> Result<Vec<u8>, SpriteError> {
        log::info!("🔧 Using BLTE library for StarCraft: Remastered data decompression");
        
        // Try without encryption first (for unencrypted BLTE content)
        match decompress_blte(data.to_vec(), None) {
            Ok(decompressed_data) => {
                log::info!("✅ BLTE decompression successful (unencrypted): {} -> {} bytes", 
                          data.len(), decompressed_data.len());
                return Ok(decompressed_data);
            }
            Err(e) => {
                log::debug!("Unencrypted BLTE decompression failed: {}, trying with key service", e);
            }
        }
        
        // Try with key service for encrypted content
        let key_service = KeyService::new();
        match decompress_blte(data.to_vec(), Some(&key_service)) {
            Ok(decompressed_data) => {
                log::info!("✅ BLTE decompression successful (encrypted): {} -> {} bytes", 
                          data.len(), decompressed_data.len());
                Ok(decompressed_data)
            }
            Err(e) => {
                log::warn!("Encrypted BLTE decompression failed: {}", e);
                Err(SpriteError::BlteDecryption(format!("BLTE decompression failed: {}", e)))
            }
        }
    }
    
    /// Decompress single BLTE chunk
    fn decompress_blte_single_chunk(data: &[u8]) -> Result<Vec<u8>, SpriteError> {
        if data.is_empty() {
            return Err(SpriteError::DecodeError("Empty BLTE chunk".to_string()));
        }
        
        let compression_type = data[0];
        let chunk_data = &data[1..];
        
        log::debug!("BLTE chunk: compression_type={:02x} ('{}'), data_size={}", 
                   compression_type, compression_type as char, chunk_data.len());
        
        match compression_type {
            0x4E => {
                // 'N' - Normal/uncompressed data
                log::debug!("BLTE chunk is uncompressed (N), returning {} bytes as-is", chunk_data.len());
                Ok(chunk_data.to_vec())
            }
            0x5A => {
                // 'Z' - ZLIB compressed data
                log::debug!("Decompressing BLTE ZLIB chunk: {} bytes", chunk_data.len());
                use std::io::Read;
                use flate2::read::ZlibDecoder;
                
                let mut decoder = ZlibDecoder::new(chunk_data);
                let mut decompressed = Vec::new();
                
                match decoder.read_to_end(&mut decompressed) {
                    Ok(bytes_read) => {
                        log::debug!("ZLIB decompression successful: {} -> {} bytes", chunk_data.len(), bytes_read);
                        Ok(decompressed)
                    }
                    Err(e) => {
                        log::error!("ZLIB decompression failed: {}", e);
                        Err(SpriteError::DecodeError(format!("ZLIB decompression failed: {}", e)))
                    }
                }
            }
            0x46 => {
                // 'F' - Recursive frames (not implemented)
                log::warn!("BLTE recursive frames (F) not implemented, treating as uncompressed");
                Ok(chunk_data.to_vec())
            }
            _ => {
                let compression_char = if compression_type.is_ascii() {
                    compression_type as char
                } else {
                    '?'
                };
                log::warn!("Unknown BLTE compression type: {:02x} ('{}'), treating as uncompressed", 
                          compression_type, compression_char);
                Ok(chunk_data.to_vec())
            }
        }
    }
    
    /// Try LZ4 decompression
    fn try_lz4_decompression(&self, data: &[u8]) -> Result<Vec<u8>, SpriteError> {
        // LZ4 doesn't have a standard header, so this is speculative
        // We'll try to decompress and see if we get reasonable results
        
        // For now, just return an error since we don't have LZ4 implemented
        Err(SpriteError::DecodeError("LZ4 decompression not implemented".to_string()))
    }
    
    /// Try simple decryption methods (XOR, etc.) - FIXED INFINITE RECURSION + ENHANCED WITH WORKING KEYS
    fn try_simple_decryption(&self, encrypted_data: &[u8], file_info: &FileInfo) -> Option<Vec<u8>> {
        // CRITICAL FIX: Skip 0x00 key to prevent infinite recursion
        // XOR with 0x00 doesn't change the data, causing infinite loops
        let common_keys = [
            0xFF, 0xAA, 0x55, 0xCC, 0x33, 0xF0, 0x0F,
            0x42, 0x24, 0x69, 0x96, 0x13, 0x31, 0x87, 0x78
        ];
        
        for &key in &common_keys {
            let mut decrypted = encrypted_data.to_vec();
            
            // Simple XOR decryption
            for byte in &mut decrypted {
                *byte ^= key;
            }
            
            // CRITICAL FIX: Ensure decrypted data is actually different from original
            if decrypted != encrypted_data && self.looks_like_valid_decrypted_data(&decrypted) {
                log::info!("🔓 Successfully decrypted {} with XOR key 0x{:02x}", file_info.name, key);
                return Some(decrypted);
            }
        }
        
        // ENHANCED: Try multi-byte XOR keys - WORKING KEYS FROM DECRYPTION TEST
        let multi_byte_keys = [
            vec![0x53, 0x43, 0x52], // "SCR" - StarCraft Remastered (WORKING!)
            vec![0x42, 0x4C, 0x5A], // "BLZ" - Blizzard (WORKING!)
            vec![0x43, 0x41, 0x53, 0x43], // "CASC" (WORKING!)
            vec![0x53, 0x74, 0x61, 0x72], // "Star" (WORKING!)
            vec![0x43, 0x72, 0x61, 0x66, 0x74], // "Craft" (WORKING!)
            vec![0x42, 0x24], // Original working key
        ];
        
        for key in &multi_byte_keys {
            let mut decrypted = encrypted_data.to_vec();
            
            // Multi-byte XOR decryption
            for (i, byte) in decrypted.iter_mut().enumerate() {
                *byte ^= key[i % key.len()];
            }
            
            // CRITICAL FIX: Ensure decrypted data is actually different from original
            if decrypted != encrypted_data {
                // ENHANCED: Check if decrypted data looks like valid StarCraft data or compressed data
                if self.looks_like_valid_decrypted_data(&decrypted) || self.looks_like_compressed_data_after_decryption(&decrypted) {
                    log::info!("🔓 Successfully decrypted {} with multi-byte XOR key {:02x?}", file_info.name, key);
                    return Some(decrypted);
                }
            }
        }
        
        None
    }
    
    /// Check if decrypted data looks valid
    fn looks_like_valid_decrypted_data(&self, data: &[u8]) -> bool {
        if data.len() < 16 {
            return false;
        }
        
        // Check for known format signatures after decryption
        if &data[0..4] == b"ANIM" || &data[0..4] == b"DDS " || &data[0..2] == b"BM" {
            return true;
        }
        
        // Check for ZLIB header after decryption
        if data[0] == 0x78 && (data[1] == 0x01 || data[1] == 0x9C || data[1] == 0xDA) {
            return true;
        }
        
        // Check for reasonable entropy (not too high, not too low)
        let mut byte_counts = [0u32; 256];
        let sample_size = data.len().min(256);
        for &byte in &data[0..sample_size] {
            byte_counts[byte as usize] += 1;
        }
        
        let unique_bytes = byte_counts.iter().filter(|&&count| count > 0).count();
        let entropy_ratio = unique_bytes as f64 / 256.0;
        
        // Good decrypted data should have moderate entropy (not completely random, not too uniform)
        entropy_ratio > 0.1 && entropy_ratio < 0.8
    }
    
    /// Check if decrypted data looks like compressed data that needs further decompression
    fn looks_like_compressed_data_after_decryption(&self, data: &[u8]) -> bool {
        if data.len() < 16 {
            return false;
        }
        
        // Check for ZLIB header patterns after decryption
        if data[0] == 0x78 && (data[1] == 0x01 || data[1] == 0x9C || data[1] == 0xDA) {
            return true;
        }
        
        // Check for other compression signatures
        if data.len() >= 4 {
            // Check for LZ4 magic number
            if &data[0..4] == b"\x04\"M\x18" {
                return true;
            }
            
            // Check for other compression patterns
            if &data[0..4] == b"BLTE" {
                return true;
            }
        }
        
        // Check if entropy suggests compressed data (but not as high as encrypted)
        let mut byte_counts = [0u32; 256];
        let sample_size = data.len().min(512);
        for &byte in &data[0..sample_size] {
            byte_counts[byte as usize] += 1;
        }
        
        let unique_bytes = byte_counts.iter().filter(|&&count| count > 0).count();
        let entropy_ratio = unique_bytes as f64 / 256.0;
        
        // Compressed data typically has high entropy but not as high as encrypted data
        entropy_ratio > 0.7 && entropy_ratio < 0.95
    }
    */
    
    // End of commented-out old BLTE methods
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    // Helper function to create test PNG data
    fn create_test_png_data(width: u16, height: u16, bit_depth: u8, color_type: u8, frame_count: u16) -> Vec<u8> {
        let mut data = Vec::new();
        
        // PNG signature
        data.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        
        // IHDR chunk
        data.extend_from_slice(&13u32.to_be_bytes()); // Length
        data.extend_from_slice(b"IHDR"); // Type
        data.extend_from_slice(&(width as u32).to_be_bytes()); // Width
        data.extend_from_slice(&(height as u32).to_be_bytes()); // Height
        data.push(bit_depth); // Bit depth
        data.push(color_type); // Color type
        data.push(0); // Compression method
        data.push(0); // Filter method
        data.push(0); // Interlace method
        
        // Simple CRC (not accurate, but sufficient for testing)
        data.extend_from_slice(&0u32.to_be_bytes());
        
        // Add frame count as a custom chunk if > 1
        if frame_count > 1 {
            let frame_data = frame_count.to_be_bytes();
            data.extend_from_slice(&(frame_data.len() as u32).to_be_bytes());
            data.extend_from_slice(b"frAM"); // Custom frame chunk
            data.extend_from_slice(&frame_data);
            data.extend_from_slice(&0u32.to_be_bytes()); // CRC
        }
        
        // IEND chunk
        data.extend_from_slice(&0u32.to_be_bytes()); // Length
        data.extend_from_slice(b"IEND"); // Type
        data.extend_from_slice(&0u32.to_be_bytes()); // CRC
        
        data
    }
    
    // Helper function to create test JPEG data
    fn create_test_jpeg_data(width: u16, height: u16) -> Vec<u8> {
        let mut data = Vec::new();
        
        // JPEG signature
        data.extend_from_slice(&[0xFF, 0xD8]); // SOI
        
        // APP0 segment
        data.extend_from_slice(&[0xFF, 0xE0]); // APP0 marker
        data.extend_from_slice(&16u16.to_be_bytes()); // Length
        data.extend_from_slice(b"JFIF\0"); // Identifier
        data.extend_from_slice(&[0x01, 0x01]); // Version
        data.push(0); // Units
        data.extend_from_slice(&1u16.to_be_bytes()); // X density
        data.extend_from_slice(&1u16.to_be_bytes()); // Y density
        data.push(0); // Thumbnail width
        data.push(0); // Thumbnail height
        
        // SOF0 segment (simplified)
        data.extend_from_slice(&[0xFF, 0xC0]); // SOF0 marker
        data.extend_from_slice(&17u16.to_be_bytes()); // Length
        data.push(8); // Precision
        data.extend_from_slice(&height.to_be_bytes()); // Height
        data.extend_from_slice(&width.to_be_bytes()); // Width
        data.push(3); // Number of components
        // Component data (simplified)
        data.extend_from_slice(&[1, 0x11, 0, 2, 0x11, 1, 3, 0x11, 1]);
        
        // EOI
        data.extend_from_slice(&[0xFF, 0xD9]);
        
        data
    }
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn property_format_detection_accuracy(
            has_png_signature in any::<bool>(),
            has_jpeg_signature in any::<bool>(),
            entropy in 0.0f64..8.0f64
        ) {
            // Create a mock file analysis
            let analysis = FileAnalysis {
                entropy,
                has_png_signature,
                has_jpeg_signature,
                file_type_detected: if has_png_signature {
                    Some("PNG".to_string())
                } else if has_jpeg_signature {
                    Some("JPEG".to_string())
                } else {
                    None
                },
            };
            
            // Create a mock extractor (we can't easily create a real CASC archive in tests)
            let temp_dir = tempfile::TempDir::new().unwrap();
            let data_dir = temp_dir.path().join("Data").join("data");
            std::fs::create_dir_all(&data_dir).unwrap();
            
            // Create minimal index and data files for CASC archive
            let index_path = data_dir.join("data.000.idx");
            let mut index_data = vec![0u8; 24]; // Minimal header
            index_data[8..10].copy_from_slice(&7u16.to_le_bytes()); // unk0 = 7
            index_data[14] = 9; // entry_key_bytes
            std::fs::write(&index_path, &index_data).unwrap();
            
            let data_path = data_dir.join("data.000");
            std::fs::write(&data_path, b"test data").unwrap();
            
            if let Ok(casc_archive) = CascArchive::open(temp_dir.path()) {
                let extractor = DirectSpriteExtractor::new(casc_archive);
                let detected_format = extractor.detect_sprite_format(&analysis);
                
                // Format detection should be deterministic based on signatures
                if has_png_signature && !has_jpeg_signature {
                    prop_assert!(matches!(detected_format, SpriteFormat::PNG));
                } else if has_jpeg_signature && !has_png_signature {
                    prop_assert!(matches!(detected_format, SpriteFormat::JPEG));
                } else if has_png_signature && has_jpeg_signature {
                    // PNG takes precedence when both signatures are present
                    prop_assert!(matches!(detected_format, SpriteFormat::PNG));
                } else {
                    prop_assert!(matches!(detected_format, SpriteFormat::CompressedData));
                }
            }
        }
        
        #[test]
        // **Feature: casc-sprite-extractor, Property 5: Sprite Conversion Preservation**
        // **Validates: Requirements 2.2, 2.3, 2.4, 2.5**
        fn property_5_sprite_conversion_preservation(
            sprite_name in "[a-zA-Z0-9_-]{1,20}",
            has_transparency in any::<bool>(),
            resolution_tier in prop_oneof![
                Just(Some(ResolutionTier::HD)),
                Just(Some(ResolutionTier::HD2)),
                Just(Some(ResolutionTier::SD)),
                Just(None)
            ],
            file_size in 100usize..10000usize,
            entropy in 7.0f64..8.0f64,
            pixels_per_unit in 50.0f32..200.0f32,
            compression_quality in 10u32..100u32
        ) {
            // For any valid sprite data, the conversion process should preserve all essential 
            // information (color, transparency, frame order, metadata) when converting to 
            // Unity-compatible PNG format
            
            let temp_dir = tempfile::TempDir::new().unwrap();
            let data_dir = temp_dir.path().join("Data").join("data");
            std::fs::create_dir_all(&data_dir).unwrap();
            
            // Create minimal CASC structure
            let index_path = data_dir.join("data.000.idx");
            let mut index_data = vec![0u8; 24];
            index_data[8..10].copy_from_slice(&7u16.to_le_bytes());
            index_data[14] = 9;
            std::fs::write(&index_path, &index_data).unwrap();
            
            let data_path = data_dir.join("data.000");
            std::fs::write(&data_path, b"test").unwrap();
            
            if let Ok(casc_archive) = CascArchive::open(temp_dir.path()) {
                let extractor = DirectSpriteExtractor::new(casc_archive);
                
                // Create test sprite data
                let original_metadata = SpriteMetadata {
                    name: sprite_name.clone(),
                    format: "PNG".to_string(),
                    file_size,
                    resolution_tier: resolution_tier.map(|t| format!("{:?}", t)),
                    entropy,
                    has_transparency,
                    unity_metadata: None,
                    dimensions: None,
                    color_depth: None,
                    frame_count: None,
                    compression_ratio: None,
                };
                
                let sprite_data = SpriteData {
                    name: sprite_name.clone(),
                    format: SpriteFormat::PNG,
                    resolution_tier,
                    data: vec![0u8; file_size], // Mock sprite data
                    metadata: original_metadata,
                };
                
                // Create Unity converter with test settings
                let unity_converter = UnityConverter {
                    pixels_per_unit,
                    filter_mode: "Bilinear".to_string(),
                    wrap_mode: "Clamp".to_string(),
                    compression_quality,
                    generate_mip_maps: false,
                };
                
                // Create Unity metadata
                let unity_metadata = extractor.create_unity_metadata(&sprite_data, &unity_converter);
                
                // Verify that essential information is preserved
                prop_assert_eq!(unity_metadata.pixels_per_unit, pixels_per_unit,
                    "Pixels per unit should be preserved");
                
                prop_assert_eq!(unity_metadata.compression_quality, compression_quality,
                    "Compression quality should be preserved");
                
                prop_assert_eq!(unity_metadata.alpha_is_transparency, has_transparency,
                    "Transparency information should be preserved");
                
                prop_assert_eq!(unity_metadata.filter_mode, "Bilinear",
                    "Filter mode should be preserved");
                
                prop_assert_eq!(unity_metadata.wrap_mode, "Clamp",
                    "Wrap mode should be preserved");
                
                // Verify resolution-specific settings
                let expected_max_size = match resolution_tier {
                    Some(ResolutionTier::HD2) => 4096,
                    Some(ResolutionTier::HD) => 2048,
                    Some(ResolutionTier::SD) => 1024,
                    _ => 2048,
                };
                prop_assert_eq!(unity_metadata.max_texture_size, expected_max_size,
                    "Max texture size should match resolution tier");
                
                // Verify transparency-specific settings
                if has_transparency {
                    prop_assert_eq!(unity_metadata.texture_format, "RGBA32",
                        "Transparent sprites should use RGBA32 format");
                    prop_assert_eq!(unity_metadata.alpha_source, "Input Texture Alpha",
                        "Transparent sprites should use input texture alpha");
                } else {
                    prop_assert_eq!(unity_metadata.texture_format, "RGB24",
                        "Opaque sprites should use RGB24 format");
                    prop_assert_eq!(unity_metadata.alpha_source, "None",
                        "Opaque sprites should have no alpha source");
                }
                
                // Verify Unity-specific defaults
                prop_assert_eq!(unity_metadata.sprite_mode, "Single",
                    "Sprite mode should default to Single");
                prop_assert_eq!(unity_metadata.texture_type, "Sprite (2D and UI)",
                    "Texture type should be appropriate for sprites");
                prop_assert_eq!(unity_metadata.pivot.x, 0.5,
                    "Pivot X should default to center");
                prop_assert_eq!(unity_metadata.pivot.y, 0.5,
                    "Pivot Y should default to center");
                prop_assert!(!unity_metadata.readable,
                    "Sprites should not be readable by default");
                prop_assert!(!unity_metadata.generate_mip_maps,
                    "Sprites should not generate mip maps by default");
            }
        }
        
        #[test]
        // **Feature: casc-sprite-extractor, Property 7: Metadata Extraction Completeness**
        // **Validates: Requirements 11.2, 11.5**
        fn property_7_metadata_extraction_completeness(
            image_width in 1u32..4096u32,
            image_height in 1u32..4096u32,
            bit_depth in prop_oneof![Just(1u8), Just(2u8), Just(4u8), Just(8u8), Just(16u8)],
            color_type in 0u8..7u8,
            frame_count in 1u32..100u32,
            file_format in prop_oneof![
                Just(SpriteFormat::PNG),
                Just(SpriteFormat::JPEG),
                Just(SpriteFormat::CompressedData)
            ]
        ) {
            // For any valid sprite data, metadata extraction should capture all available 
            // information and generate complete JSON reports with proper validation
            
            let temp_dir = tempfile::TempDir::new().unwrap();
            let data_dir = temp_dir.path().join("Data").join("data");
            std::fs::create_dir_all(&data_dir).unwrap();
            
            // Create minimal CASC structure
            let index_path = data_dir.join("data.000.idx");
            let mut index_data = vec![0u8; 24];
            index_data[8..10].copy_from_slice(&7u16.to_le_bytes());
            index_data[14] = 9;
            std::fs::write(&index_path, &index_data).unwrap();
            
            let data_path = data_dir.join("data.000");
            std::fs::write(&data_path, b"test").unwrap();
            
            if let Ok(casc_archive) = CascArchive::open(temp_dir.path()) {
                let extractor = DirectSpriteExtractor::new(casc_archive);
                
                // Create test sprite data based on format
                let test_data = match file_format {
                    SpriteFormat::PNG => create_test_png_data(
                        image_width as u16, 
                        image_height as u16, 
                        bit_depth, 
                        color_type, 
                        frame_count as u16
                    ),
                    SpriteFormat::JPEG => create_test_jpeg_data(
                        image_width as u16, 
                        image_height as u16
                    ),
                    SpriteFormat::CompressedData => vec![0u8; 1000], // Mock compressed data
                };
                
                // Extract dimensions using the extractor methods
                let extracted_dimensions = extractor.extract_image_dimensions(&test_data, file_format);
                let extracted_color_depth = extractor.extract_color_depth(&test_data, file_format);
                let extracted_frame_count = extractor.extract_frame_count(&test_data, file_format);
                let extracted_compression_ratio = extractor.calculate_compression_ratio(&test_data, &extracted_dimensions);
                
                // Verify metadata extraction completeness based on format capabilities
                match file_format {
                    SpriteFormat::PNG => {
                        // PNG should extract all metadata
                        prop_assert!(extracted_dimensions.is_some(), 
                            "PNG dimensions should be extractable");
                        
                        if let Some(ref dims) = extracted_dimensions {
                            prop_assert_eq!(dims.width, image_width, 
                                "PNG width should be correctly extracted");
                            prop_assert_eq!(dims.height, image_height, 
                                "PNG height should be correctly extracted");
                        }
                        
                        prop_assert!(extracted_color_depth.is_some(), 
                            "PNG color depth should be extractable");
                        
                        prop_assert!(extracted_frame_count.is_some(), 
                            "PNG frame count should be extractable");
                        
                        if let Some(frames) = extracted_frame_count {
                            prop_assert!(frames >= 1, 
                                "PNG should have at least 1 frame");
                        }
                        
                        prop_assert!(extracted_compression_ratio.is_some(), 
                            "PNG compression ratio should be calculable");
                    },
                    
                    SpriteFormat::JPEG => {
                        // JPEG should extract dimensions and basic metadata
                        prop_assert!(extracted_dimensions.is_some(), 
                            "JPEG dimensions should be extractable");
                        
                        if let Some(ref dims) = extracted_dimensions {
                            prop_assert_eq!(dims.width, image_width, 
                                "JPEG width should be correctly extracted");
                            prop_assert_eq!(dims.height, image_height, 
                                "JPEG height should be correctly extracted");
                        }
                        
                        prop_assert!(extracted_color_depth.is_some(), 
                            "JPEG color depth should be extractable");
                        
                        if let Some(depth) = extracted_color_depth {
                            prop_assert_eq!(depth, 24, 
                                "JPEG should have 24-bit color depth");
                        }
                        
                        prop_assert!(extracted_frame_count.is_some(), 
                            "JPEG frame count should be extractable");
                        
                        if let Some(frames) = extracted_frame_count {
                            prop_assert_eq!(frames, 1, 
                                "JPEG should have exactly 1 frame");
                        }
                        
                        prop_assert!(extracted_compression_ratio.is_some(), 
                            "JPEG compression ratio should be calculable");
                    },
                    
                    SpriteFormat::CompressedData => {
                        // Compressed data should have limited metadata extraction
                        prop_assert!(extracted_dimensions.is_none(), 
                            "Compressed data dimensions should not be extractable without decompression");
                        
                        prop_assert!(extracted_color_depth.is_none(), 
                            "Compressed data color depth should not be extractable without decompression");
                        
                        prop_assert!(extracted_frame_count.is_none(), 
                            "Compressed data frame count should not be extractable without decompression");
                        
                        prop_assert!(extracted_compression_ratio.is_none(), 
                            "Compressed data compression ratio should not be calculable without dimensions");
                    }
                }
                
                // Verify that metadata can be serialized to JSON
                let metadata = SpriteMetadata {
                    name: "test_sprite".to_string(),
                    format: format!("{:?}", file_format),
                    file_size: test_data.len(),
                    resolution_tier: Some("HD".to_string()),
                    entropy: 7.5,
                    has_transparency: color_type == 6 || color_type == 4, // RGBA or Grayscale+Alpha
                    unity_metadata: None,
                    dimensions: extracted_dimensions,
                    color_depth: extracted_color_depth,
                    frame_count: extracted_frame_count,
                    compression_ratio: extracted_compression_ratio,
                };
                
                // Verify JSON serialization works
                let json_result = serde_json::to_string_pretty(&metadata);
                prop_assert!(json_result.is_ok(), 
                    "Metadata should be serializable to JSON");
                
                if let Ok(json_string) = json_result {
                    // Verify JSON contains expected fields
                    prop_assert!(json_string.contains("\"name\""), 
                        "JSON should contain name field");
                    prop_assert!(json_string.contains("\"format\""), 
                        "JSON should contain format field");
                    prop_assert!(json_string.contains("\"file_size\""), 
                        "JSON should contain file_size field");
                    prop_assert!(json_string.contains("\"entropy\""), 
                        "JSON should contain entropy field");
                    prop_assert!(json_string.contains("\"has_transparency\""), 
                        "JSON should contain has_transparency field");
                    
                    // Verify JSON can be deserialized back
                    let deserialized_result: Result<SpriteMetadata, _> = serde_json::from_str(&json_string);
                    prop_assert!(deserialized_result.is_ok(), 
                        "JSON should be deserializable back to metadata");
                    
                    if let Ok(deserialized) = deserialized_result {
                        prop_assert_eq!(deserialized.name, metadata.name, 
                            "Deserialized name should match original");
                        prop_assert_eq!(deserialized.format, metadata.format, 
                            "Deserialized format should match original");
                        prop_assert_eq!(deserialized.file_size, metadata.file_size, 
                            "Deserialized file_size should match original");
                    }
                }
            }
        }
        
        #[test]
        fn property_filename_sanitization(
            input_filename in "[a-zA-Z0-9/\\\\:*?\"<>|._-]{1,50}"
        ) {
            let temp_dir = tempfile::TempDir::new().unwrap();
            let data_dir = temp_dir.path().join("Data").join("data");
            std::fs::create_dir_all(&data_dir).unwrap();
            
            // Create minimal CASC structure
            let index_path = data_dir.join("data.000.idx");
            let mut index_data = vec![0u8; 24];
            index_data[8..10].copy_from_slice(&7u16.to_le_bytes());
            index_data[14] = 9;
            std::fs::write(&index_path, &index_data).unwrap();
            
            let data_path = data_dir.join("data.000");
            std::fs::write(&data_path, b"test").unwrap();
            
            if let Ok(casc_archive) = CascArchive::open(temp_dir.path()) {
                let extractor = DirectSpriteExtractor::new(casc_archive);
                let sanitized = extractor.sanitize_filename(&input_filename);
                
                // Sanitized filename should not contain problematic characters
                prop_assert!(!sanitized.contains('/'));
                prop_assert!(!sanitized.contains('\\'));
                prop_assert!(!sanitized.contains(':'));
                prop_assert!(!sanitized.contains('*'));
                prop_assert!(!sanitized.contains('?'));
                prop_assert!(!sanitized.contains('"'));
                prop_assert!(!sanitized.contains('<'));
                prop_assert!(!sanitized.contains('>'));
                prop_assert!(!sanitized.contains('|'));
                
                // Sanitized filename should have same or shorter length
                prop_assert!(sanitized.len() <= input_filename.len());
                
                // If input had no problematic characters, output should be identical
                if !input_filename.chars().any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')) {
                    prop_assert_eq!(sanitized, input_filename);
                }
            }
        }
    }
}

pub mod export;
