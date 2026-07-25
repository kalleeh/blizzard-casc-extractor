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
use std::path::PathBuf;

/// Which converted image formats an extraction should emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormatFilterOption {
    Png,
    Jpeg,
    Images,
    All,
}

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
}

/// Quality and output configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QualitySettings {
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

/// Unity-specific export settings.
///
/// Only the knobs the extractor actually honours are modelled here. Unknown keys
/// in a config file are ignored, so files written for earlier versions (which
/// carried filter/wrap modes, mipmaps, pivots and a Unity `.meta` toggle that
/// nothing ever read) still load.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UnityExportSettings {
    /// Pixels per unit written into the sprite metadata JSON
    pub pixels_per_unit: f32,
}

/// Metadata generation options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MetadataOptions {
    /// Generate JSON metadata files
    pub generate_json: bool,
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

/// File filtering settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FilterSettings {
    /// Include patterns (regex)
    pub include_patterns: Option<Vec<String>>,

    /// Exclude patterns (regex)
    pub exclude_patterns: Option<Vec<String>>,

    /// Maximum number of files to process
    pub max_files: Option<u64>,
}

impl Default for QualitySettings {
    fn default() -> Self {
        Self {
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
            pixels_per_unit: 100.0,
        }
    }
}

impl Default for MetadataOptions {
    fn default() -> Self {
        Self {
            generate_json: true,
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
        let decoded: ExtractionConfig = serde_json::from_str(&json).expect("deserialize failed");
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
    fn legacy_config_with_removed_keys_still_loads() {
        // Fields that were deserialized but never read have been removed. Config
        // files written for earlier versions must keep working, with the settings
        // that ARE honoured still taking effect.
        let json = r#"{
            "quality_settings": {"resolution_tier": "All", "format_filter": "All",
                                 "png_compression_level": 3},
            "output_settings": {
                "output_directory": "/tmp/out",
                "unity_settings": {"enabled": true, "pixels_per_unit": 64.0,
                                   "filter_mode": "Bilinear", "wrap_mode": "Clamp",
                                   "compression_quality": 50, "generate_mipmaps": false,
                                   "pivot_point": "Center", "generate_meta_files": true},
                "metadata_options": {"generate_json": true, "generate_unity_meta": false,
                                     "include_animation_data": true,
                                     "include_database_info": true},
                "overwrite_behavior": "Never"
            },
            "filter_settings": {"resolution_tier": "All", "max_files": 5},
            "custom_settings": {"anything": 1}
        }"#;
        let config: ExtractionConfig =
            serde_json::from_str(json).expect("legacy config should still deserialize");
        assert_eq!(config.quality_settings.png_compression_level, 3);
        assert_eq!(config.output_settings.unity_settings.pixels_per_unit, 64.0);
        assert_eq!(
            config.output_settings.overwrite_behavior,
            OverwriteBehavior::Never
        );
        assert_eq!(config.filter_settings.max_files, Some(5));
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
