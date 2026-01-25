//! Format converter for Unity export system
//! 
//! This module provides functionality for converting sprite data to Unity-compatible
//! formats with proper metadata generation.

use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use log::debug;

use crate::sprite::{SpriteData, SpriteFormat, UnityMetadata, UnityConverter, UnityPivot};

/// Format converter for Unity export system
/// 
/// Handles conversion of sprite data to Unity-compatible formats with metadata
pub struct FormatConverter {
    /// Unity converter for metadata generation
    unity_converter: UnityConverter,
}

/// Result of format conversion
#[derive(Debug)]
pub struct ConversionResult {
    /// Path to the converted image file
    pub image_path: PathBuf,
    
    /// Path to the Unity metadata file
    pub metadata_path: PathBuf,
    
    /// Original format that was converted
    pub source_format: SpriteFormat,
    
    /// Size of the converted image file in bytes
    pub image_size: u64,
    
    /// Size of the metadata file in bytes
    pub metadata_size: u64,
}

impl FormatConverter {
    /// Create a new format converter
    pub fn new() -> Self {
        Self {
            unity_converter: UnityConverter::default(),
        }
    }
    
    /// Create a format converter with custom Unity settings
    pub fn with_unity_converter(unity_converter: UnityConverter) -> Self {
        Self {
            unity_converter,
        }
    }
    
    /// Convert sprite data to Unity format
    /// 
    /// This method handles the complete conversion process:
    /// 1. Convert sprite data to PNG format
    /// 2. Generate Unity metadata
    /// 3. Write both files to the output directory
    /// 4. Return paths and statistics
    pub fn convert_to_unity_format(
        &self,
        sprite_data: &SpriteData,
        output_dir: &Path,
        source_filename: &str
    ) -> Result<(PathBuf, PathBuf)> {
        debug!("Converting sprite data to Unity format: {}", source_filename);
        
        // Generate output filename (replace extension with .png)
        let base_name = Path::new(source_filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("sprite");
        
        let png_filename = format!("{}.png", base_name);
        let metadata_filename = format!("{}.png.meta", base_name);
        
        let png_path = output_dir.join(&png_filename);
        let metadata_path = output_dir.join(&metadata_filename);
        
        // Convert sprite data to PNG
        self.write_png_file(&png_path, sprite_data)
            .context("Failed to write PNG file")?;
        
        // Generate and write Unity metadata
        let unity_metadata = self.generate_unity_metadata(sprite_data)?;
        self.write_unity_metadata(&metadata_path, &unity_metadata)
            .context("Failed to write Unity metadata")?;
        
        debug!("Successfully converted {} to Unity format", source_filename);
        
        Ok((png_path, metadata_path))
    }
    
    /// Convert sprite data to Unity format with detailed result
    pub fn convert_with_result(
        &self,
        sprite_data: &SpriteData,
        output_dir: &Path,
        source_filename: &str
    ) -> Result<ConversionResult> {
        let (image_path, metadata_path) = self.convert_to_unity_format(sprite_data, output_dir, source_filename)?;
        
        // Get file sizes
        let image_size = std::fs::metadata(&image_path)
            .context("Failed to get image file metadata")?
            .len();
        
        let metadata_size = std::fs::metadata(&metadata_path)
            .context("Failed to get metadata file metadata")?
            .len();
        
        Ok(ConversionResult {
            image_path,
            metadata_path,
            source_format: sprite_data.format,
            image_size,
            metadata_size,
        })
    }
    
    /// Write sprite data as PNG file
    fn write_png_file(&self, output_path: &Path, sprite_data: &SpriteData) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create output directory")?;
        }
        
        // Write PNG data to file
        std::fs::write(output_path, &sprite_data.data)
            .context("Failed to write PNG data to file")?;
        
        debug!("Wrote PNG file: {} ({} bytes)", output_path.display(), sprite_data.data.len());
        
        Ok(())
    }
    
    /// Generate Unity metadata for sprite data
    fn generate_unity_metadata(&self, sprite_data: &SpriteData) -> Result<UnityMetadata> {
        let texture_type = match sprite_data.format {
            SpriteFormat::PNG | SpriteFormat::JPEG => "Sprite (2D and UI)",
            SpriteFormat::CompressedData => "Sprite (2D and UI)", // Assume sprite for compressed data
        };
        
        let alpha_is_transparency = sprite_data.metadata.has_transparency;
        let alpha_source = if alpha_is_transparency {
            "FromInput"
        } else {
            "None"
        };
        
        // Get dimensions from metadata or use defaults
        let (max_texture_size, readable) = if let Some(ref dimensions) = sprite_data.metadata.dimensions {
            let max_size = dimensions.width.max(dimensions.height).next_power_of_two();
            (max_size, false) // Non-readable for better performance
        } else {
            (2048, false) // Default values
        };
        
        Ok(UnityMetadata {
            sprite_mode: "Single".to_string(),
            pixels_per_unit: self.unity_converter.pixels_per_unit,
            pivot: UnityPivot { x: 0.5, y: 0.5 }, // Center pivot
            filter_mode: self.unity_converter.filter_mode.clone(),
            wrap_mode: self.unity_converter.wrap_mode.clone(),
            texture_type: texture_type.to_string(),
            max_texture_size,
            texture_format: "Automatic".to_string(),
            compression_quality: self.unity_converter.compression_quality,
            generate_mip_maps: self.unity_converter.generate_mip_maps,
            readable,
            alpha_source: alpha_source.to_string(),
            alpha_is_transparency,
        })
    }
    
    /// Write Unity metadata to file
    fn write_unity_metadata(&self, output_path: &Path, metadata: &UnityMetadata) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create output directory")?;
        }
        
        // Generate Unity metadata content
        let metadata_content = self.generate_unity_metadata_content(metadata)?;
        
        // Write metadata to file
        std::fs::write(output_path, metadata_content)
            .context("Failed to write Unity metadata to file")?;
        
        debug!("Wrote Unity metadata file: {}", output_path.display());
        
        Ok(())
    }
    
    /// Generate Unity metadata file content
    fn generate_unity_metadata_content(&self, metadata: &UnityMetadata) -> Result<String> {
        // Unity .meta file format (simplified version)
        let content = format!(
            r#"fileFormatVersion: 2
guid: {}
TextureImporter:
  internalIDToNameTable: []
  externalObjects: {{}}
  serializedVersion: 12
  mipmaps:
    mipMapMode: 0
    enableMipMap: {}
    sRGBTexture: 1
    linearTexture: 0
    fadeOut: 0
    borderMipMap: 0
    mipMapsPreserveCoverage: 0
    alphaTestReferenceValue: 0.5
    mipMapFadeDistanceStart: 1
    mipMapFadeDistanceEnd: 3
  bumpmap:
    convertToNormalMap: 0
    externalNormalMap: 0
    heightScale: 0.25
    normalMapFilter: 0
  isReadable: {}
  streamingMipmaps: 0
  streamingMipmapsPriority: 0
  vTOnly: 0
  ignoreMasterTextureLimit: 0
  grayScaleToAlpha: 0
  generateCubemap: 6
  cubemapConvolution: 0
  seamlessCubemap: 0
  textureFormat: 1
  maxTextureSize: {}
  textureSettings:
    serializedVersion: 2
    filterMode: {}
    aniso: 1
    mipBias: 0
    wrapU: 1
    wrapV: 1
    wrapW: 1
  nPOTScale: 0
  lightmap: 0
  compressionQuality: {}
  spriteMode: 1
  spriteExtrude: 1
  spriteMeshType: 1
  alignment: 0
  spritePivot: {{x: {}, y: {}}}
  spritePixelsPerUnit: {}
  spriteBorder: {{x: 0, y: 0, z: 0, w: 0}}
  spriteGenerateFallbackPhysicsShape: 1
  alphaUsage: 1
  alphaIsTransparency: {}
  spriteTessellationDetail: -1
  textureType: 8
  textureShape: 1
  singleChannelComponent: 0
  flipbookRows: 1
  flipbookColumns: 1
  maxTextureSizeSet: 0
  compressionQualitySet: 0
  textureFormatSet: 0
  ignorePngGamma: 0
  applyGammaDecoding: 0
  swizzle: 50462976
  platformSettings:
  - serializedVersion: 3
    buildTarget: DefaultTexturePlatform
    maxTextureSize: {}
    resizeAlgorithm: 0
    textureFormat: -1
    textureCompression: 1
    compressionQuality: 50
    crunchedCompression: 0
    allowsAlphaSplitting: 0
    overridden: 0
    androidETC2FallbackOverride: 0
    forceMaximumCompressionQuality_BC6H_BC7: 0
  spriteSheet:
    serializedVersion: 2
    sprites: []
    outline: []
    physicsShape: []
    bones: []
    spriteID: 5e97eb03825dee720800000000000000
    internalID: 0
    vertices: []
    indices: 
    edges: []
    weights: []
    secondaryTextures: []
    nameFileIdTable: {{}}
  mipmapLimitGroupName: 
  pSDRemoveMatte: 0
  userData: 
  assetBundleName: 
  assetBundleVariant: 
"#,
            self.generate_guid(),
            if metadata.generate_mip_maps { 1 } else { 0 },
            if metadata.readable { 1 } else { 0 },
            metadata.max_texture_size,
            self.unity_filter_mode_to_int(&metadata.filter_mode),
            metadata.compression_quality,
            metadata.pivot.x,
            metadata.pivot.y,
            metadata.pixels_per_unit,
            if metadata.alpha_is_transparency { 1 } else { 0 },
            metadata.max_texture_size
        );
        
        Ok(content)
    }
    
    /// Generate a GUID for Unity metadata
    fn generate_guid(&self) -> String {
        // Generate a simple GUID-like string for Unity
        // In a real implementation, this should be a proper GUID
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        std::time::SystemTime::now().hash(&mut hasher);
        let hash = hasher.finish();
        
        format!("{:016x}{:016x}", hash, hash.wrapping_mul(0x9e3779b97f4a7c15))
    }
    
    /// Convert Unity filter mode string to integer
    fn unity_filter_mode_to_int(&self, filter_mode: &str) -> i32 {
        match filter_mode {
            "Point" => 0,
            "Bilinear" => 1,
            "Trilinear" => 2,
            _ => 1, // Default to Bilinear
        }
    }
    
    /// Get the Unity converter reference
    pub fn unity_converter(&self) -> &UnityConverter {
        &self.unity_converter
    }
    
    /// Get a mutable reference to the Unity converter
    pub fn unity_converter_mut(&mut self) -> &mut UnityConverter {
        &mut self.unity_converter
    }
}

impl Default for FormatConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::sprite::{SpriteMetadata, ImageDimensions};
    
    fn create_test_sprite_data() -> SpriteData {
        // Create a simple 2x2 PNG data for testing
        let png_data = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, // IHDR length
            0x49, 0x48, 0x44, 0x52, // IHDR
            0x00, 0x00, 0x00, 0x02, // Width: 2
            0x00, 0x00, 0x00, 0x02, // Height: 2
            0x08, 0x02, 0x00, 0x00, 0x00, // Bit depth, color type, etc.
            0x90, 0x5D, 0x68, 0x82, // CRC
            0x00, 0x00, 0x00, 0x00, // IEND length
            0x49, 0x45, 0x4E, 0x44, // IEND
            0xAE, 0x42, 0x60, 0x82, // IEND CRC
        ];
        
        SpriteData {
            name: "test_sprite".to_string(),
            data: png_data,
            format: SpriteFormat::PNG,
            metadata: SpriteMetadata {
                name: "test_sprite".to_string(),
                format: "PNG".to_string(),
                file_size: 25, // Size of the PNG data above
                resolution_tier: None,
                entropy: 0.5,
                has_transparency: false,
                unity_metadata: None,
                dimensions: Some(ImageDimensions { width: 2, height: 2 }),
                color_depth: Some(24),
                frame_count: Some(1),
                compression_ratio: None,
            },
            resolution_tier: None,
        }
    }
    
    #[test]
    fn test_format_converter_creation() {
        let converter = FormatConverter::new();
        assert_eq!(converter.unity_converter().pixels_per_unit, 100.0);
    }
    
    #[test]
    fn test_convert_to_unity_format() {
        let temp_dir = TempDir::new().unwrap();
        let converter = FormatConverter::new();
        let sprite_data = create_test_sprite_data();
        
        let result = converter.convert_to_unity_format(
            &sprite_data,
            temp_dir.path(),
            "test_sprite.anim"
        );
        
        assert!(result.is_ok(), "Conversion should succeed");
        
        let (png_path, metadata_path) = result.unwrap();
        
        // Check that files were created
        assert!(png_path.exists(), "PNG file should be created");
        assert!(metadata_path.exists(), "Metadata file should be created");
        
        // Check file extensions
        assert_eq!(png_path.extension().unwrap(), "png");
        assert_eq!(metadata_path.extension().unwrap(), "meta");
        
        // Check that PNG file contains the sprite data
        let written_data = std::fs::read(&png_path).unwrap();
        assert_eq!(written_data, sprite_data.data);
    }
    
    #[test]
    fn test_convert_with_result() {
        let temp_dir = TempDir::new().unwrap();
        let converter = FormatConverter::new();
        let sprite_data = create_test_sprite_data();
        
        let result = converter.convert_with_result(
            &sprite_data,
            temp_dir.path(),
            "test_sprite.anim"
        );
        
        assert!(result.is_ok(), "Conversion with result should succeed");
        
        let conversion_result = result.unwrap();
        
        // Check result properties
        assert_eq!(conversion_result.source_format, SpriteFormat::PNG);
        assert!(conversion_result.image_size > 0);
        assert!(conversion_result.metadata_size > 0);
        assert!(conversion_result.image_path.exists());
        assert!(conversion_result.metadata_path.exists());
    }
    
    #[test]
    fn test_unity_metadata_generation() {
        let converter = FormatConverter::new();
        let sprite_data = create_test_sprite_data();
        
        let metadata_result = converter.generate_unity_metadata(&sprite_data);
        assert!(metadata_result.is_ok(), "Unity metadata generation should succeed");
        
        let metadata = metadata_result.unwrap();
        assert_eq!(metadata.sprite_mode, "Single");
        assert_eq!(metadata.pixels_per_unit, 100.0);
        assert!(!metadata.alpha_is_transparency); // Test sprite has no transparency
    }
    
    #[test]
    fn test_guid_generation() {
        let converter = FormatConverter::new();
        
        let guid1 = converter.generate_guid();
        let guid2 = converter.generate_guid();
        
        // GUIDs should be different
        assert_ne!(guid1, guid2);
        
        // GUIDs should be 32 characters long (hex string)
        assert_eq!(guid1.len(), 32);
        assert_eq!(guid2.len(), 32);
        
        // GUIDs should only contain hex characters
        assert!(guid1.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(guid2.chars().all(|c| c.is_ascii_hexdigit()));
    }
}