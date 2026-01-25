// Unity import validation tests
//
// These tests verify that the Unity import validator correctly validates
// sprite metadata and import compatibility.

use casc_extractor::validation::{UnityImportValidator, SpriteMetadata};
use std::path::Path;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_unity_validator_creation() {
    let validator = UnityImportValidator::with_defaults();
    // Should not panic even if Unity is not installed
}

#[test]
fn test_validation_without_unity_configured() {
    let validator = UnityImportValidator::new(None, None);
    let sprite_path = Path::new("test.png");
    
    let result = validator.validate_unity_import(sprite_path).unwrap();
    assert!(result.success);
    assert!(result.log_messages.iter().any(|msg| msg.contains("skipped")));
    assert_eq!(result.diagnostic, "Unity Editor not configured");
}

#[test]
fn test_sprite_metadata_validation_valid() {
    let validator = UnityImportValidator::with_defaults();
    
    // Valid metadata - typical StarCraft sprite
    let metadata = SpriteMetadata {
        width: 64,
        height: 64,
        texture_format: "RGBA32".to_string(),
        has_alpha: true,
        pixels_per_unit: 100.0,
        compression: "Uncompressed".to_string(),
        filter_mode: "Point".to_string(),
    };
    
    assert!(validator.validate_sprite_metadata(&metadata).is_ok());
}

#[test]
fn test_sprite_metadata_validation_various_sizes() {
    let validator = UnityImportValidator::with_defaults();
    
    // Test various valid sizes
    let test_sizes = vec![
        (32, 32),   // Small sprite
        (64, 64),   // Medium sprite
        (128, 128), // Large sprite
        (256, 128), // Rectangular sprite
        (512, 512), // Very large sprite
    ];
    
    for (width, height) in test_sizes {
        let metadata = SpriteMetadata {
            width,
            height,
            texture_format: "RGBA32".to_string(),
            has_alpha: true,
            pixels_per_unit: 100.0,
            compression: "Uncompressed".to_string(),
            filter_mode: "Point".to_string(),
        };
        
        assert!(
            validator.validate_sprite_metadata(&metadata).is_ok(),
            "Validation should pass for {}x{} sprite",
            width,
            height
        );
    }
}

#[test]
fn test_sprite_metadata_validation_zero_dimensions() {
    let validator = UnityImportValidator::with_defaults();
    
    // Zero width
    let metadata = SpriteMetadata {
        width: 0,
        height: 64,
        texture_format: "RGBA32".to_string(),
        has_alpha: true,
        pixels_per_unit: 100.0,
        compression: "Uncompressed".to_string(),
        filter_mode: "Point".to_string(),
    };
    
    assert!(validator.validate_sprite_metadata(&metadata).is_err());
    
    // Zero height
    let metadata = SpriteMetadata {
        width: 64,
        height: 0,
        texture_format: "RGBA32".to_string(),
        has_alpha: true,
        pixels_per_unit: 100.0,
        compression: "Uncompressed".to_string(),
        filter_mode: "Point".to_string(),
    };
    
    assert!(validator.validate_sprite_metadata(&metadata).is_err());
    
    // Both zero
    let metadata = SpriteMetadata {
        width: 0,
        height: 0,
        texture_format: "RGBA32".to_string(),
        has_alpha: true,
        pixels_per_unit: 100.0,
        compression: "Uncompressed".to_string(),
        filter_mode: "Point".to_string(),
    };
    
    assert!(validator.validate_sprite_metadata(&metadata).is_err());
}

#[test]
fn test_sprite_metadata_validation_oversized() {
    let validator = UnityImportValidator::with_defaults();
    
    // Exceeds Unity's 8192x8192 limit
    let metadata = SpriteMetadata {
        width: 10000,
        height: 10000,
        texture_format: "RGBA32".to_string(),
        has_alpha: true,
        pixels_per_unit: 100.0,
        compression: "Uncompressed".to_string(),
        filter_mode: "Point".to_string(),
    };
    
    assert!(validator.validate_sprite_metadata(&metadata).is_err());
    
    // Width exceeds limit
    let metadata = SpriteMetadata {
        width: 9000,
        height: 64,
        texture_format: "RGBA32".to_string(),
        has_alpha: true,
        pixels_per_unit: 100.0,
        compression: "Uncompressed".to_string(),
        filter_mode: "Point".to_string(),
    };
    
    assert!(validator.validate_sprite_metadata(&metadata).is_err());
    
    // Height exceeds limit
    let metadata = SpriteMetadata {
        width: 64,
        height: 9000,
        texture_format: "RGBA32".to_string(),
        has_alpha: true,
        pixels_per_unit: 100.0,
        compression: "Uncompressed".to_string(),
        filter_mode: "Point".to_string(),
    };
    
    assert!(validator.validate_sprite_metadata(&metadata).is_err());
}

#[test]
fn test_sprite_metadata_validation_edge_cases() {
    let validator = UnityImportValidator::with_defaults();
    
    // Exactly at Unity's limit (should pass)
    let metadata = SpriteMetadata {
        width: 8192,
        height: 8192,
        texture_format: "RGBA32".to_string(),
        has_alpha: true,
        pixels_per_unit: 100.0,
        compression: "Uncompressed".to_string(),
        filter_mode: "Point".to_string(),
    };
    
    assert!(validator.validate_sprite_metadata(&metadata).is_ok());
    
    // Just over Unity's limit (should fail)
    let metadata = SpriteMetadata {
        width: 8193,
        height: 8192,
        texture_format: "RGBA32".to_string(),
        has_alpha: true,
        pixels_per_unit: 100.0,
        compression: "Uncompressed".to_string(),
        filter_mode: "Point".to_string(),
    };
    
    assert!(validator.validate_sprite_metadata(&metadata).is_err());
}

#[test]
fn test_sprite_metadata_validation_texture_formats() {
    let validator = UnityImportValidator::with_defaults();
    
    // Test various texture formats
    let formats = vec![
        "RGBA32",
        "RGB24",
        "ARGB32",
        "Alpha8",
        "Rgba",
        "Rgb",
    ];
    
    for format in formats {
        let metadata = SpriteMetadata {
            width: 64,
            height: 64,
            texture_format: format.to_string(),
            has_alpha: true,
            pixels_per_unit: 100.0,
            compression: "Uncompressed".to_string(),
            filter_mode: "Point".to_string(),
        };
        
        // Should not fail for supported formats
        assert!(
            validator.validate_sprite_metadata(&metadata).is_ok(),
            "Validation should pass for format: {}",
            format
        );
    }
}

#[test]
fn test_sprite_metadata_alpha_channel() {
    let validator = UnityImportValidator::with_defaults();
    
    // With alpha
    let metadata = SpriteMetadata {
        width: 64,
        height: 64,
        texture_format: "RGBA32".to_string(),
        has_alpha: true,
        pixels_per_unit: 100.0,
        compression: "Uncompressed".to_string(),
        filter_mode: "Point".to_string(),
    };
    
    assert!(validator.validate_sprite_metadata(&metadata).is_ok());
    
    // Without alpha
    let metadata = SpriteMetadata {
        width: 64,
        height: 64,
        texture_format: "RGB24".to_string(),
        has_alpha: false,
        pixels_per_unit: 100.0,
        compression: "Uncompressed".to_string(),
        filter_mode: "Point".to_string(),
    };
    
    assert!(validator.validate_sprite_metadata(&metadata).is_ok());
}

#[test]
fn test_sprite_metadata_compression_settings() {
    let validator = UnityImportValidator::with_defaults();
    
    let compression_types = vec![
        "Uncompressed",
        "LowQuality",
        "NormalQuality",
        "HighQuality",
    ];
    
    for compression in compression_types {
        let metadata = SpriteMetadata {
            width: 64,
            height: 64,
            texture_format: "RGBA32".to_string(),
            has_alpha: true,
            pixels_per_unit: 100.0,
            compression: compression.to_string(),
            filter_mode: "Point".to_string(),
        };
        
        assert!(
            validator.validate_sprite_metadata(&metadata).is_ok(),
            "Validation should pass for compression: {}",
            compression
        );
    }
}

#[test]
fn test_sprite_metadata_filter_modes() {
    let validator = UnityImportValidator::with_defaults();
    
    let filter_modes = vec!["Point", "Bilinear", "Trilinear"];
    
    for filter_mode in filter_modes {
        let metadata = SpriteMetadata {
            width: 64,
            height: 64,
            texture_format: "RGBA32".to_string(),
            has_alpha: true,
            pixels_per_unit: 100.0,
            compression: "Uncompressed".to_string(),
            filter_mode: filter_mode.to_string(),
        };
        
        assert!(
            validator.validate_sprite_metadata(&metadata).is_ok(),
            "Validation should pass for filter mode: {}",
            filter_mode
        );
    }
}

#[test]
fn test_sprite_metadata_pixels_per_unit() {
    let validator = UnityImportValidator::with_defaults();
    
    let ppu_values = vec![1.0, 10.0, 50.0, 100.0, 200.0, 1000.0];
    
    for ppu in ppu_values {
        let metadata = SpriteMetadata {
            width: 64,
            height: 64,
            texture_format: "RGBA32".to_string(),
            has_alpha: true,
            pixels_per_unit: ppu,
            compression: "Uncompressed".to_string(),
            filter_mode: "Point".to_string(),
        };
        
        assert!(
            validator.validate_sprite_metadata(&metadata).is_ok(),
            "Validation should pass for pixels per unit: {}",
            ppu
        );
    }
}

#[test]
fn test_read_sprite_metadata_from_image() {
    let temp_dir = TempDir::new().unwrap();
    let sprite_path = temp_dir.path().join("test_sprite.png");
    
    // Create a test PNG image
    let img = image::RgbaImage::new(64, 64);
    img.save(&sprite_path).unwrap();
    
    let validator = UnityImportValidator::with_defaults();
    let metadata = validator.read_sprite_metadata_from_image(&sprite_path).unwrap();
    
    assert_eq!(metadata.width, 64);
    assert_eq!(metadata.height, 64);
    assert!(metadata.has_alpha);
    assert_eq!(metadata.pixels_per_unit, 100.0);
}

#[test]
fn test_read_sprite_metadata_from_rgb_image() {
    let temp_dir = TempDir::new().unwrap();
    let sprite_path = temp_dir.path().join("test_sprite_rgb.png");
    
    // Create a test RGB image (no alpha)
    let img = image::RgbImage::new(128, 128);
    img.save(&sprite_path).unwrap();
    
    let validator = UnityImportValidator::with_defaults();
    let metadata = validator.read_sprite_metadata_from_image(&sprite_path).unwrap();
    
    assert_eq!(metadata.width, 128);
    assert_eq!(metadata.height, 128);
    assert!(!metadata.has_alpha);
}

#[test]
fn test_metadata_serialization() {
    let metadata = SpriteMetadata {
        width: 64,
        height: 64,
        texture_format: "RGBA32".to_string(),
        has_alpha: true,
        pixels_per_unit: 100.0,
        compression: "Uncompressed".to_string(),
        filter_mode: "Point".to_string(),
    };
    
    // Test JSON serialization
    let json = serde_json::to_string(&metadata).unwrap();
    assert!(json.contains("\"width\":64"));
    assert!(json.contains("\"height\":64"));
    assert!(json.contains("\"has_alpha\":true"));
    
    // Test deserialization
    let deserialized: SpriteMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.width, metadata.width);
    assert_eq!(deserialized.height, metadata.height);
    assert_eq!(deserialized.has_alpha, metadata.has_alpha);
}

#[test]
fn test_unity_import_result_structure() {
    let result = casc_extractor::validation::UnityImportResult {
        success: true,
        sprite_path: std::path::PathBuf::from("test.png"),
        metadata: Some(SpriteMetadata {
            width: 64,
            height: 64,
            texture_format: "RGBA32".to_string(),
            has_alpha: true,
            pixels_per_unit: 100.0,
            compression: "Uncompressed".to_string(),
            filter_mode: "Point".to_string(),
        }),
        log_messages: vec!["Import successful".to_string()],
        diagnostic: "Test diagnostic".to_string(),
    };
    
    assert!(result.success);
    assert!(result.metadata.is_some());
    assert_eq!(result.log_messages.len(), 1);
}

#[test]
fn test_validation_comprehensive_starcraft_sprites() {
    let validator = UnityImportValidator::with_defaults();
    
    // Test typical StarCraft sprite dimensions
    let starcraft_dimensions = vec![
        (32, 32),   // Small units (SCV, Marine)
        (64, 64),   // Medium units (Tank, Goliath)
        (96, 96),   // Large units (Battlecruiser)
        (128, 128), // Buildings (Command Center)
        (256, 256), // Large buildings (Nexus)
    ];
    
    for (width, height) in starcraft_dimensions {
        let metadata = SpriteMetadata {
            width,
            height,
            texture_format: "RGBA32".to_string(),
            has_alpha: true,
            pixels_per_unit: 100.0,
            compression: "HighQuality".to_string(),
            filter_mode: "Point".to_string(),
        };
        
        assert!(
            validator.validate_sprite_metadata(&metadata).is_ok(),
            "StarCraft sprite {}x{} should pass validation",
            width,
            height
        );
    }
}
