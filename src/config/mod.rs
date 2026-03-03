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
use crate::cli::{ResolutionTier, FormatFilterOption, UnityFilterMode, UnityWrapMode};

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

    /// Analysis settings
    #[serde(default)]
    pub analysis_settings: AnalysisSettings,

    /// Research data collection settings
    #[serde(default)]
    pub research_settings: ResearchSettings,

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
    
    /// JPEG quality (0-100, higher = better quality)
    pub jpeg_quality: u32,
    
    /// Enable lossless compression where possible
    pub prefer_lossless: bool,
    
    /// Color depth preference (8-bit, 16-bit, 32-bit)
    pub color_depth: ColorDepth,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FeedbackSettings {
    /// Enable progress reporting
    pub enable_progress_reporting: bool,
    
    /// Progress update interval in milliseconds
    pub progress_update_interval_ms: u64,
    
    /// Enable verbose logging
    pub verbose_logging: bool,
    
    /// Enable performance metrics collection
    pub collect_performance_metrics: bool,
    
    /// Enable research data collection
    pub collect_research_data: bool,
    
    /// User feedback options
    pub user_feedback_options: UserFeedbackOptions,
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

/// Color depth options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorDepth {
    /// 8-bit color (256 colors)
    Bit8,
    /// 16-bit color (65,536 colors)
    Bit16,
    /// 32-bit color (16.7 million colors + alpha)
    Bit32,
    /// Preserve original depth
    Original,
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
    
    /// Include performance metrics
    pub include_performance_metrics: bool,
    
    /// Include research data
    pub include_research_data: bool,
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

/// User feedback options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UserFeedbackOptions {
    /// Show extraction progress bar
    pub show_progress_bar: bool,
    
    /// Show file-by-file progress
    pub show_file_progress: bool,
    
    /// Show performance statistics
    pub show_performance_stats: bool,
    
    /// Show memory usage information
    pub show_memory_usage: bool,
    
    /// Show format detection details
    pub show_format_details: bool,
    
    /// Show error details
    pub show_error_details: bool,
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

/// Analysis settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisSettings {
    /// Enable pattern analysis
    pub analyze_patterns: bool,
    
    /// Enable format analysis
    pub analyze_formats: bool,
    
    /// Enable performance analysis
    pub analyze_performance: bool,
}

/// Research data collection settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResearchSettings {
    /// Enable research data collection
    pub collect_research_data: bool,
    
    /// Enable format statistics collection
    pub collect_format_statistics: bool,
    
    /// Enable performance metrics collection
    pub collect_performance_metrics: bool,
}

impl Default for QualitySettings {
    fn default() -> Self {
        Self {
            resolution_tier: ResolutionTier::All,
            format_filter: FormatFilterOption::All,
            png_compression_level: 6,
            jpeg_quality: 85,
            prefer_lossless: true,
            color_depth: ColorDepth::Original,
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

impl Default for FeedbackSettings {
    fn default() -> Self {
        Self {
            enable_progress_reporting: true,
            progress_update_interval_ms: 500,
            verbose_logging: false,
            collect_performance_metrics: true,
            collect_research_data: false,
            user_feedback_options: UserFeedbackOptions::default(),
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
            include_performance_metrics: false,
            include_research_data: false,
        }
    }
}

impl Default for UserFeedbackOptions {
    fn default() -> Self {
        Self {
            show_progress_bar: true,
            show_file_progress: false,
            show_performance_stats: true,
            show_memory_usage: false,
            show_format_details: false,
            show_error_details: true,
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

