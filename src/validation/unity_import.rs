// Unity import validation system
//
// This module validates that extracted sprites can be successfully imported
// into Unity Editor without errors.

use super::ValidationError;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use image::GenericImageView;

/// Result of a Unity import validation
#[derive(Debug, Clone)]
pub struct UnityImportResult {
    /// Whether the import was successful
    pub success: bool,
    
    /// Path to the imported sprite in Unity project
    pub sprite_path: PathBuf,
    
    /// Unity sprite metadata
    pub metadata: Option<SpriteMetadata>,
    
    /// Import log messages
    pub log_messages: Vec<String>,
    
    /// Diagnostic information
    pub diagnostic: String,
}

/// Unity sprite metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteMetadata {
    /// Sprite width in pixels
    pub width: u32,
    
    /// Sprite height in pixels
    pub height: u32,
    
    /// Texture format
    pub texture_format: String,
    
    /// Whether sprite has alpha channel
    pub has_alpha: bool,
    
    /// Pixels per unit setting
    pub pixels_per_unit: f32,
    
    /// Texture compression setting
    pub compression: String,
    
    /// Filter mode (Point, Bilinear, Trilinear)
    pub filter_mode: String,
}

/// Unity import validator
pub struct UnityImportValidator {
    /// Path to Unity Editor executable
    unity_editor_path: Option<PathBuf>,
    
    /// Path to Unity test project
    test_project_path: Option<PathBuf>,
}

impl UnityImportValidator {
    /// Create a new Unity import validator
    pub fn new(unity_editor_path: Option<PathBuf>, test_project_path: Option<PathBuf>) -> Self {
        Self {
            unity_editor_path,
            test_project_path,
        }
    }

    /// Create a validator with default paths
    pub fn with_defaults() -> Self {
        // Try to find Unity Editor in common locations
        let unity_editor_path = Self::find_unity_editor();
        let test_project_path = Self::find_test_project();
        
        Self::new(unity_editor_path, test_project_path)
    }

    /// Find Unity Editor in common installation locations
    fn find_unity_editor() -> Option<PathBuf> {
        let common_paths = vec![
            // macOS
            "/Applications/Unity/Hub/Editor/*/Unity.app/Contents/MacOS/Unity",
            "/Applications/Unity/Unity.app/Contents/MacOS/Unity",
            // Windows
            "C:/Program Files/Unity/Hub/Editor/*/Editor/Unity.exe",
            "C:/Program Files/Unity/Editor/Unity.exe",
            // Linux
            "/opt/Unity/Editor/Unity",
            "/usr/bin/unity-editor",
        ];

        for pattern in common_paths {
            if let Ok(paths) = glob::glob(pattern) {
                for path in paths.flatten() {
                    if path.exists() {
                        info!("Found Unity Editor at: {:?}", path);
                        return Some(path);
                    }
                }
            }
        }

        warn!("Unity Editor not found in common locations");
        None
    }

    /// Find Unity test project
    fn find_test_project() -> Option<PathBuf> {
        // Look for Unity project in parent directories
        let current_dir = std::env::current_dir().ok()?;
        
        // Check if we're in the casc-extractor directory
        let mut search_dir = current_dir.clone();
        
        // Go up to find the Unity project root
        for _ in 0..5 {
            let unity_project = search_dir.join("Assets");
            if unity_project.exists() && unity_project.join("_Project").exists() {
                info!("Found Unity project at: {:?}", search_dir);
                return Some(search_dir);
            }
            
            if let Some(parent) = search_dir.parent() {
                search_dir = parent.to_path_buf();
            } else {
                break;
            }
        }

        warn!("Unity test project not found");
        None
    }

    /// Validate that a sprite can be imported into Unity
    ///
    /// This performs:
    /// - Copy sprite to Unity project
    /// - Trigger Unity asset import
    /// - Check for import errors
    /// - Verify sprite metadata
    ///
    /// # Arguments
    /// * `sprite_path` - Path to the sprite to validate
    ///
    /// # Returns
    /// Unity import result with diagnostic information
    pub fn validate_unity_import(&self, sprite_path: &Path) -> Result<UnityImportResult, ValidationError> {
        info!("Validating Unity import for: {:?}", sprite_path);

        // Check if Unity Editor is available
        if self.unity_editor_path.is_none() || self.test_project_path.is_none() {
            debug!("Unity Editor or test project not configured - skipping Unity import validation");
            return Ok(UnityImportResult {
                success: true,
                sprite_path: sprite_path.to_path_buf(),
                metadata: None,
                log_messages: vec!["Unity import validation skipped (not configured)".to_string()],
                diagnostic: "Unity Editor not configured".to_string(),
            });
        }

        let unity_editor = self.unity_editor_path.as_ref().unwrap();
        let test_project = self.test_project_path.as_ref().unwrap();

        // Step 1: Copy sprite to Unity project
        let asset_path = self.copy_to_unity_project(sprite_path, test_project)?;
        
        // Step 2: Trigger Unity asset import
        let _import_result = self.trigger_unity_import(unity_editor, test_project, &asset_path)?;
        
        // Step 3: Check for import errors
        let log_messages = self.read_unity_import_log(test_project)?;
        let has_errors = log_messages.iter().any(|msg| 
            msg.to_lowercase().contains("error") || 
            msg.to_lowercase().contains("failed")
        );
        
        if has_errors {
            return Err(ValidationError::UnityImportFailed {
                details: format!("Unity import errors detected:\n{}", log_messages.join("\n")),
            });
        }
        
        // Step 4: Verify sprite metadata
        let metadata = self.read_sprite_metadata(&asset_path)?;
        self.validate_sprite_metadata(&metadata)?;
        
        let diagnostic = format!("Unity import successful: {}x{} pixels", 
            metadata.width, metadata.height);
        
        Ok(UnityImportResult {
            success: true,
            sprite_path: asset_path,
            metadata: Some(metadata),
            log_messages,
            diagnostic,
        })
    }

    /// Copy sprite to Unity project Assets folder
    fn copy_to_unity_project(&self, sprite_path: &Path, test_project: &Path) -> Result<PathBuf, ValidationError> {
        let assets_dir = test_project.join("Assets").join("_Project").join("Art").join("Sprites").join("TestSprites");
        
        // Create directory if it doesn't exist
        fs::create_dir_all(&assets_dir)?;
        
        // Copy sprite with unique name to avoid conflicts
        let sprite_name = sprite_path.file_name()
            .ok_or_else(|| ValidationError::UnityImportFailed {
                details: "Invalid sprite path".to_string(),
            })?;
        
        let target_path = assets_dir.join(sprite_name);
        fs::copy(sprite_path, &target_path)?;
        
        info!("Copied sprite to Unity project: {:?}", target_path);
        Ok(target_path)
    }

    /// Trigger Unity asset import via command line
    fn trigger_unity_import(&self, unity_editor: &Path, test_project: &Path, _asset_path: &Path) -> Result<(), ValidationError> {
        info!("Triggering Unity asset import...");
        
        // Use Unity's batch mode to refresh assets
        let output = Command::new(unity_editor)
            .arg("-batchmode")
            .arg("-quit")
            .arg("-projectPath")
            .arg(test_project)
            .arg("-executeMethod")
            .arg("UnityEditor.AssetDatabase.Refresh")
            .arg("-logFile")
            .arg(test_project.join("Logs").join("import_validation.log"))
            .output()
            .map_err(|e| ValidationError::UnityImportFailed {
                details: format!("Failed to execute Unity Editor: {}", e),
            })?;
        
        if !output.status.success() {
            return Err(ValidationError::UnityImportFailed {
                details: format!("Unity import command failed with status: {}", output.status),
            });
        }
        
        info!("Unity asset import triggered successfully");
        Ok(())
    }

    /// Read Unity import log
    fn read_unity_import_log(&self, test_project: &Path) -> Result<Vec<String>, ValidationError> {
        let log_path = test_project.join("Logs").join("import_validation.log");
        
        if !log_path.exists() {
            debug!("Unity import log not found, using Editor.log");
            // Try Editor.log as fallback
            let editor_log = test_project.join("Logs").join("Editor.log");
            if editor_log.exists() {
                let content = fs::read_to_string(editor_log)?;
                return Ok(content.lines().map(String::from).collect());
            }
            return Ok(Vec::new());
        }
        
        let content = fs::read_to_string(log_path)?;
        Ok(content.lines().map(String::from).collect())
    }

    /// Read sprite metadata from Unity .meta file
    fn read_sprite_metadata(&self, asset_path: &Path) -> Result<SpriteMetadata, ValidationError> {
        let meta_path = asset_path.with_extension("png.meta");
        
        if !meta_path.exists() {
            // If .meta file doesn't exist yet, read image dimensions directly
            return self.read_sprite_metadata_from_image(asset_path);
        }
        
        let meta_content = fs::read_to_string(&meta_path)?;
        
        // Parse Unity .meta file (YAML format)
        // This is a simplified parser - in production, use a proper YAML parser
        let width = Self::extract_meta_value(&meta_content, "width")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let height = Self::extract_meta_value(&meta_content, "height")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let texture_format = Self::extract_meta_value(&meta_content, "textureFormat")
            .unwrap_or_else(|| "Unknown".to_string());
        let has_alpha = Self::extract_meta_value(&meta_content, "alphaIsTransparency")
            .map(|s| s == "1")
            .unwrap_or(false);
        let pixels_per_unit = Self::extract_meta_value(&meta_content, "pixelsPerUnit")
            .and_then(|s| s.parse().ok())
            .unwrap_or(100.0);
        let compression = Self::extract_meta_value(&meta_content, "textureCompression")
            .unwrap_or_else(|| "Uncompressed".to_string());
        let filter_mode = Self::extract_meta_value(&meta_content, "filterMode")
            .unwrap_or_else(|| "Point".to_string());
        
        Ok(SpriteMetadata {
            width,
            height,
            texture_format,
            has_alpha,
            pixels_per_unit,
            compression,
            filter_mode,
        })
    }

    /// Read sprite metadata directly from image file
    pub fn read_sprite_metadata_from_image(&self, asset_path: &Path) -> Result<SpriteMetadata, ValidationError> {
        let img = image::open(asset_path)?;
        let (width, height) = img.dimensions();
        let has_alpha = img.color().has_alpha();
        
        Ok(SpriteMetadata {
            width,
            height,
            texture_format: format!("{:?}", img.color()),
            has_alpha,
            pixels_per_unit: 100.0,
            compression: "Uncompressed".to_string(),
            filter_mode: "Point".to_string(),
        })
    }

    /// Extract value from Unity .meta file
    fn extract_meta_value(content: &str, key: &str) -> Option<String> {
        for line in content.lines() {
            if line.trim().starts_with(key) {
                if let Some(value) = line.split(':').nth(1) {
                    return Some(value.trim().to_string());
                }
            }
        }
        None
    }

    /// Validate sprite metadata meets requirements
    pub fn validate_sprite_metadata(&self, metadata: &SpriteMetadata) -> Result<(), ValidationError> {
        // Check dimensions are reasonable
        if metadata.width == 0 || metadata.height == 0 {
            return Err(ValidationError::MetadataMismatch {
                details: format!("Invalid sprite dimensions: {}x{}", metadata.width, metadata.height),
            });
        }
        
        // Check dimensions are within Unity limits (max 8192x8192)
        if metadata.width > 8192 || metadata.height > 8192 {
            return Err(ValidationError::MetadataMismatch {
                details: format!("Sprite dimensions exceed Unity limits: {}x{}", metadata.width, metadata.height),
            });
        }
        
        // Verify texture format is supported
        let supported_formats = vec!["RGBA32", "RGB24", "ARGB32", "Alpha8", "Rgba", "Rgb"];
        if !supported_formats.iter().any(|f| metadata.texture_format.contains(f)) {
            warn!("Unusual texture format: {}", metadata.texture_format);
        }
        
        info!("Sprite metadata validation passed: {}x{} pixels, format: {}", 
            metadata.width, metadata.height, metadata.texture_format);
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let validator = UnityImportValidator::with_defaults();
        // Should not panic even if Unity is not installed
    }

    #[test]
    fn test_validation_without_unity() {
        let validator = UnityImportValidator::new(None, None);
        let sprite_path = Path::new("test.png");
        
        let result = validator.validate_unity_import(sprite_path).unwrap();
        assert!(result.success); // Should succeed with warning when Unity not configured
        assert!(result.log_messages.iter().any(|msg| msg.contains("skipped")));
    }

    #[test]
    fn test_metadata_validation() {
        let validator = UnityImportValidator::with_defaults();
        
        // Valid metadata
        let valid_metadata = SpriteMetadata {
            width: 64,
            height: 64,
            texture_format: "RGBA32".to_string(),
            has_alpha: true,
            pixels_per_unit: 100.0,
            compression: "Uncompressed".to_string(),
            filter_mode: "Point".to_string(),
        };
        assert!(validator.validate_sprite_metadata(&valid_metadata).is_ok());
        
        // Invalid dimensions (zero)
        let invalid_metadata = SpriteMetadata {
            width: 0,
            height: 0,
            texture_format: "RGBA32".to_string(),
            has_alpha: true,
            pixels_per_unit: 100.0,
            compression: "Uncompressed".to_string(),
            filter_mode: "Point".to_string(),
        };
        assert!(validator.validate_sprite_metadata(&invalid_metadata).is_err());
        
        // Invalid dimensions (too large)
        let oversized_metadata = SpriteMetadata {
            width: 10000,
            height: 10000,
            texture_format: "RGBA32".to_string(),
            has_alpha: true,
            pixels_per_unit: 100.0,
            compression: "Uncompressed".to_string(),
            filter_mode: "Point".to_string(),
        };
        assert!(validator.validate_sprite_metadata(&oversized_metadata).is_err());
    }
}

