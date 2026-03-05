//! Configuration System for CASC Sprite Extractor
//!
//! This module provides a comprehensive configuration system that allows users to:
//! - Specify format priorities and quality settings
//! - Configure selective format extraction options
//! - Set memory usage limits and parallel processing controls
//! - Define format conflict resolution strategies
//! - Create reusable configuration profiles
//! - Enable progress reporting and user feedback options

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::resolution::ResolutionTier;
use crate::filter::{FormatFilterOption, UnityFilterMode, UnityWrapMode};

/// Main configuration structure for the CASC sprite extractor
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// Quality and output settings
    #[serde(default)]
    pub quality_settings: QualitySettings,

    /// Output and export settings
    #[serde(default)]
    pub output_settings: OutputSettings,

    /// Progress reporting and user feedback settings
    #[serde(default)]
    pub feedback_settings: FeedbackSettings,

    /// File filtering settings
    #[serde(default)]
    pub filter_settings: FilterSettings,

    /// Custom settings for extensibility
    #[serde(default)]
    pub custom_settings: HashMap<String, serde_json::Value>,
}

/// Quality and output configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QualitySettings {
    /// Resolution tier preference
    pub resolution_tier: ResolutionTier,

    /// Format filter for output files
    pub format_filter: FormatFilterOption,

    /// PNG compression level (0-9, higher = smaller files)
    pub png_compression_level: u32,
}

/// Output and export configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputSettings {
    /// Base output directory
    pub output_directory: PathBuf,

    /// Unity-specific export settings
    pub unity_settings: UnityExportSettings,

    /// Metadata generation options
    pub metadata_options: MetadataOptions,

    /// File overwrite behavior
    pub overwrite_behavior: OverwriteBehavior,
}

/// Progress reporting and user feedback settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FeedbackSettings {
    /// Enable verbose logging
    pub verbose_logging: bool,
}

/// Unity-specific export settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UnityExportSettings {
    /// Enable Unity-compatible output
    pub enabled: bool,

    /// Pixels per unit setting
    pub pixels_per_unit: f32,

    /// Texture filter mode
    pub filter_mode: UnityFilterMode,

    /// Texture wrap mode
    pub wrap_mode: UnityWrapMode,

    /// Compression quality (0-100)
    pub compression_quality: u32,

    /// Generate mipmaps
    pub generate_mipmaps: bool,

    /// Sprite pivot point
    pub pivot_point: UnityPivot,

    /// Generate .meta files
    pub generate_meta_files: bool,
}

/// Metadata generation options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MetadataOptions {
    /// Generate JSON metadata files
    pub generate_json: bool,

    /// Generate Unity .meta files
    pub generate_unity_meta: bool,

    /// Include animation data
    pub include_animation_data: bool,

    /// Include database information
    pub include_database_info: bool,
}

/// File overwrite behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverwriteBehavior {
    /// Always overwrite existing files
    Always,
    /// Never overwrite existing files
    Never,
    /// Overwrite only if source is newer
    IfNewer,
    /// Prompt user for each conflict
    Prompt,
    /// Create backup before overwriting
    Backup,
}

/// Unity pivot point options
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum UnityPivot {
    Center,
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    Custom(f32, f32),
}

/// File filtering settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FilterSettings {
    /// Include patterns (regex)
    pub include_patterns: Option<Vec<String>>,

    /// Exclude patterns (regex)
    pub exclude_patterns: Option<Vec<String>>,

    /// Resolution tier filter
    pub resolution_tier: ResolutionTier,

    /// Maximum number of files to process
    pub max_files: Option<u64>,
}

impl Default for QualitySettings {
    fn default() -> Self {
        Self {
            resolution_tier: ResolutionTier::All,
            format_filter: FormatFilterOption::All,
            png_compression_level: 6,
        }
    }
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            output_directory: PathBuf::from("extracted"),
            unity_settings: UnityExportSettings::default(),
            metadata_options: MetadataOptions::default(),
            overwrite_behavior: OverwriteBehavior::IfNewer,
        }
    }
}


impl Default for UnityExportSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            pixels_per_unit: 100.0,
            filter_mode: UnityFilterMode::Bilinear,
            wrap_mode: UnityWrapMode::Clamp,
            compression_quality: 50,
            generate_mipmaps: false,
            pivot_point: UnityPivot::Center,
            generate_meta_files: true,
        }
    }
}

impl Default for MetadataOptions {
    fn default() -> Self {
        Self {
            generate_json: true,
            generate_unity_meta: false,
            include_animation_data: true,
            include_database_info: true,
        }
    }
}

impl Default for FilterSettings {
    fn default() -> Self {
        Self {
            include_patterns: None,
            exclude_patterns: None,
            resolution_tier: ResolutionTier::All,
            max_files: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extraction_config_default_roundtrip() {
        let config = ExtractionConfig::default();
        let json = serde_json::to_string(&config).expect("serialize failed");
        let decoded: ExtractionConfig =
            serde_json::from_str(&json).expect("deserialize failed");
        assert_eq!(
            decoded.output_settings.overwrite_behavior,
            OverwriteBehavior::IfNewer
        );
        assert_eq!(decoded.quality_settings.png_compression_level, 6);
    }

    #[test]
    fn partial_json_deserializes_with_defaults() {
        let json = r#"{"output_settings": {"overwrite_behavior": "Never"}}"#;
        let config: ExtractionConfig =
            serde_json::from_str(json).expect("partial JSON deserialize failed");
        assert_eq!(
            config.output_settings.overwrite_behavior,
            OverwriteBehavior::Never
        );
        assert_eq!(config.quality_settings.png_compression_level, 6);
        assert!(config.filter_settings.include_patterns.is_none());
    }

    #[test]
    fn invalid_json_returns_error() {
        let result = serde_json::from_str::<ExtractionConfig>("{ not valid json }");
        assert!(result.is_err());
    }

    #[test]
    fn output_settings_default_overwrite_behavior_is_if_newer() {
        let settings = OutputSettings::default();
        assert_eq!(settings.overwrite_behavior, OverwriteBehavior::IfNewer);
    }
}
