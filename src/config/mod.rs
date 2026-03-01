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
use anyhow::{Result, Context};
use crate::cli::{ResolutionTier, FormatFilterOption, UnityFilterMode, UnityWrapMode};

/// Main configuration structure for the CASC sprite extractor
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// Format-specific settings and priorities
    pub format_settings: FormatSettings,
    
    /// Quality and output settings
    pub quality_settings: QualitySettings,
    
    /// Performance and resource management settings
    pub performance_settings: PerformanceSettings,
    
    /// Output and export settings
    pub output_settings: OutputSettings,
    
    /// Progress reporting and user feedback settings
    pub feedback_settings: FeedbackSettings,
    
    /// File filtering settings
    pub filter_settings: FilterSettings,
    
    /// Analysis settings
    pub analysis_settings: AnalysisSettings,
    
    /// Research data collection settings
    pub research_settings: ResearchSettings,
    
    /// Custom settings for extensibility
    pub custom_settings: HashMap<String, serde_json::Value>,
}

/// Format-specific configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatSettings {
    /// Enabled formats for extraction (ANIM, GRP, PCX, etc.)
    pub enabled_formats: Vec<FormatType>,
    
    /// Priority order for format detection (higher priority = checked first)
    pub format_priorities: HashMap<FormatType, u32>,
    
    /// Selective format extraction mode
    pub extraction_mode: ExtractionMode,
    
    /// Format conflict resolution strategy
    pub conflict_resolution: ConflictResolution,
    
    /// Format-specific quality settings
    pub format_quality: HashMap<FormatType, FormatQuality>,
}

/// Quality and output configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Performance and resource management settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSettings {
    /// Maximum memory usage in MB (0 = unlimited)
    pub max_memory_usage_mb: u64,
    
    /// Number of parallel processing threads (0 = auto-detect)
    pub parallel_threads: u32,
    
    /// Enable streaming processing for large files
    pub use_streaming_processing: bool,
    
    /// Enable memory-mapped file access
    pub use_memory_mapping: bool,
    
    /// Enable lazy loading of texture data
    pub use_lazy_loading: bool,
    
    /// Object pooling for data structures
    pub enable_object_pooling: bool,
    
    /// Batch size for parallel processing
    pub batch_size: u32,
}

/// Output and export configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSettings {
    /// Base output directory
    pub output_directory: PathBuf,
    
    /// Unity-specific export settings
    pub unity_settings: UnityExportSettings,
    
    /// File naming convention
    pub naming_convention: NamingConvention,
    
    /// Directory organization strategy
    pub directory_structure: DirectoryStructure,
    
    /// Metadata generation options
    pub metadata_options: MetadataOptions,
    
    /// File overwrite behavior
    pub overwrite_behavior: OverwriteBehavior,
}

/// Progress reporting and user feedback settings
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Supported sprite formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FormatType {
    ANIM,
    GRP,
    PCX,
    PNG,
    JPEG,
    Unknown,
}

/// Format extraction modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtractionMode {
    /// Extract all enabled formats
    All,
    /// Extract only ANIM format
    AnimOnly,
    /// Extract only GRP format
    GrpOnly,
    /// Extract only PCX format
    PcxOnly,
    /// Extract only image formats (PNG, JPEG)
    ImagesOnly,
    /// Custom format selection
    Custom,
}

/// Format conflict resolution strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Use highest priority format
    HighestPriority,
    /// Use highest confidence score
    HighestConfidence,
    /// Extract all conflicting formats
    ExtractAll,
    /// Skip conflicting files
    Skip,
    /// Use first successful parser
    FirstSuccess,
}

/// Format-specific quality settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatQuality {
    /// Enable high-quality processing
    pub high_quality: bool,
    
    /// Preserve original color depth
    pub preserve_color_depth: bool,
    
    /// Enable transparency preservation
    pub preserve_transparency: bool,
    
    /// Enable metadata extraction
    pub extract_metadata: bool,
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

/// File naming conventions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NamingConvention {
    /// Original CASC file names
    Original,
    /// Unity-compatible names
    Unity,
    /// Hierarchical names (Race_Unit_Animation_Direction)
    Hierarchical,
    /// Sequential numbering
    Sequential,
    /// Custom naming pattern
    Custom,
}

/// Directory organization strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirectoryStructure {
    /// Flat structure (all files in output directory)
    Flat,
    /// Organized by format (ANIM/, GRP/, PCX/)
    ByFormat,
    /// Organized by resolution (HD/, HD2/, SD/)
    ByResolution,
    /// Organized by race (Terran/, Protoss/, Zerg/)
    ByRace,
    /// Hierarchical organization (Race/Unit/Animation/)
    Hierarchical,
    /// Custom structure
    Custom,
}

/// Metadata generation options
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for FormatSettings {
    fn default() -> Self {
        let mut format_priorities = HashMap::new();
        format_priorities.insert(FormatType::ANIM, 100);
        format_priorities.insert(FormatType::GRP, 90);
        format_priorities.insert(FormatType::PCX, 80);
        format_priorities.insert(FormatType::PNG, 70);
        format_priorities.insert(FormatType::JPEG, 60);
        
        let mut format_quality = HashMap::new();
        for format in &[FormatType::ANIM, FormatType::GRP, FormatType::PCX, FormatType::PNG, FormatType::JPEG] {
            format_quality.insert(*format, FormatQuality::default());
        }
        
        Self {
            enabled_formats: vec![FormatType::ANIM, FormatType::GRP, FormatType::PCX, FormatType::PNG, FormatType::JPEG],
            format_priorities,
            extraction_mode: ExtractionMode::All,
            conflict_resolution: ConflictResolution::HighestPriority,
            format_quality,
        }
    }
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

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            max_memory_usage_mb: 2048, // 2GB default limit
            parallel_threads: 0, // Auto-detect
            use_streaming_processing: true,
            use_memory_mapping: true,
            use_lazy_loading: true,
            enable_object_pooling: true,
            batch_size: 100,
        }
    }
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            output_directory: PathBuf::from("extracted"),
            unity_settings: UnityExportSettings::default(),
            naming_convention: NamingConvention::Hierarchical,
            directory_structure: DirectoryStructure::Hierarchical,
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

impl Default for FormatQuality {
    fn default() -> Self {
        Self {
            high_quality: true,
            preserve_color_depth: true,
            preserve_transparency: true,
            extract_metadata: true,
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

impl ExtractionConfig {
    /// Create a new configuration with default settings
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Validate the configuration settings
    pub fn validate(&self) -> Result<()> {
        // Validate format settings
        self.format_settings.validate()
            .context("Invalid format settings")?;
        
        // Validate quality settings
        self.quality_settings.validate()
            .context("Invalid quality settings")?;
        
        // Validate performance settings
        self.performance_settings.validate()
            .context("Invalid performance settings")?;
        
        // Validate output settings
        self.output_settings.validate()
            .context("Invalid output settings")?;
        
        // Validate feedback settings
        self.feedback_settings.validate()
            .context("Invalid feedback settings")?;
        
        // Validate filter settings
        self.filter_settings.validate()
            .context("Invalid filter settings")?;
        
        // Validate analysis settings
        self.analysis_settings.validate()
            .context("Invalid analysis settings")?;
        
        // Validate research settings
        self.research_settings.validate()
            .context("Invalid research settings")?;
        
        Ok(())
    }
    
    /// Get the effective number of parallel threads
    pub fn get_effective_thread_count(&self) -> usize {
        if self.performance_settings.parallel_threads == 0 {
            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
        } else {
            self.performance_settings.parallel_threads as usize
        }
    }
    
    /// Check if a format is enabled for extraction
    pub fn is_format_enabled(&self, format: FormatType) -> bool {
        self.format_settings.enabled_formats.contains(&format)
    }
    
    /// Get the priority for a format (higher = more priority)
    pub fn get_format_priority(&self, format: FormatType) -> u32 {
        self.format_settings.format_priorities.get(&format).copied().unwrap_or(0)
    }
    
    /// Get quality settings for a specific format
    pub fn get_format_quality(&self, format: FormatType) -> Option<&FormatQuality> {
        self.format_settings.format_quality.get(&format)
    }
}

impl FormatSettings {
    fn validate(&self) -> Result<()> {
        // Ensure at least one format is enabled
        if self.enabled_formats.is_empty() {
            return Err(anyhow::anyhow!("At least one format must be enabled"));
        }
        
        // Validate that all enabled formats have priorities
        for format in &self.enabled_formats {
            if !self.format_priorities.contains_key(format) {
                return Err(anyhow::anyhow!("Missing priority for enabled format: {:?}", format));
            }
        }
        
        // Validate that all enabled formats have quality settings
        for format in &self.enabled_formats {
            if !self.format_quality.contains_key(format) {
                return Err(anyhow::anyhow!("Missing quality settings for enabled format: {:?}", format));
            }
        }
        
        Ok(())
    }
}

impl QualitySettings {
    fn validate(&self) -> Result<()> {
        // Validate PNG compression level
        if self.png_compression_level > 9 {
            return Err(anyhow::anyhow!("PNG compression level must be 0-9, got: {}", self.png_compression_level));
        }
        
        // Validate JPEG quality
        if self.jpeg_quality > 100 {
            return Err(anyhow::anyhow!("JPEG quality must be 0-100, got: {}", self.jpeg_quality));
        }
        
        Ok(())
    }
}

impl PerformanceSettings {
    fn validate(&self) -> Result<()> {
        // Validate batch size
        if self.batch_size == 0 {
            return Err(anyhow::anyhow!("Batch size must be greater than 0"));
        }
        
        Ok(())
    }
}

impl OutputSettings {
    fn validate(&self) -> Result<()> {
        // Validate Unity settings
        self.unity_settings.validate()
            .context("Invalid Unity settings")?;
        
        Ok(())
    }
}

impl FeedbackSettings {
    fn validate(&self) -> Result<()> {
        // Validate progress update interval
        if self.progress_update_interval_ms == 0 {
            return Err(anyhow::anyhow!("Progress update interval must be greater than 0"));
        }
        
        Ok(())
    }
}

impl UnityExportSettings {
    fn validate(&self) -> Result<()> {
        // Validate pixels per unit
        if self.pixels_per_unit <= 0.0 {
            return Err(anyhow::anyhow!("Unity pixels per unit must be positive, got: {}", self.pixels_per_unit));
        }
        
        // Validate compression quality
        if self.compression_quality > 100 {
            return Err(anyhow::anyhow!("Unity compression quality must be 0-100, got: {}", self.compression_quality));
        }
        
        Ok(())
    }
}



impl FilterSettings {
    fn validate(&self) -> Result<()> {
        // Validate include patterns (regex compilation)
        if let Some(ref patterns) = self.include_patterns {
            for pattern in patterns {
                regex::Regex::new(pattern)
                    .with_context(|| format!("Invalid include pattern regex: {}", pattern))?;
            }
        }
        
        // Validate exclude patterns (regex compilation)
        if let Some(ref patterns) = self.exclude_patterns {
            for pattern in patterns {
                regex::Regex::new(pattern)
                    .with_context(|| format!("Invalid exclude pattern regex: {}", pattern))?;
            }
        }
        
        Ok(())
    }
}

impl AnalysisSettings {
    fn validate(&self) -> Result<()> {
        // No specific validation needed for analysis settings
        Ok(())
    }
}

impl ResearchSettings {
    fn validate(&self) -> Result<()> {
        // No specific validation needed for research settings
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        /// **Feature: casc-sprite-format-improvements, Property 14: Configuration consistency**
        /// **Validates: Requirements 12.1, 12.2**
        fn property_14_configuration_consistency(
            png_compression in 0u32..=9,
            jpeg_quality in 0u32..=100,
            pixels_per_unit in 1.0f32..=1000.0,
            compression_quality in 0u32..=100,
            batch_size in 1u32..=1000,
            progress_interval in 1u64..=10000
        ) {
            // Create a configuration with the generated values
            let mut config = ExtractionConfig::default();
            config.quality_settings.png_compression_level = png_compression;
            config.quality_settings.jpeg_quality = jpeg_quality;
            config.output_settings.unity_settings.pixels_per_unit = pixels_per_unit;
            config.output_settings.unity_settings.compression_quality = compression_quality;
            config.performance_settings.batch_size = batch_size;
            config.feedback_settings.progress_update_interval_ms = progress_interval;
            
            // Configuration should be valid
            prop_assert!(config.validate().is_ok());
            
            // Serialization and deserialization should preserve the configuration
            let serialized = serde_json::to_string(&config).unwrap();
            let deserialized: ExtractionConfig = serde_json::from_str(&serialized).unwrap();
            
            // All settings should be preserved
            prop_assert_eq!(config.quality_settings.png_compression_level, deserialized.quality_settings.png_compression_level);
            prop_assert_eq!(config.quality_settings.jpeg_quality, deserialized.quality_settings.jpeg_quality);
            prop_assert_eq!(config.output_settings.unity_settings.pixels_per_unit, deserialized.output_settings.unity_settings.pixels_per_unit);
            prop_assert_eq!(config.output_settings.unity_settings.compression_quality, deserialized.output_settings.unity_settings.compression_quality);
            prop_assert_eq!(config.performance_settings.batch_size, deserialized.performance_settings.batch_size);
            prop_assert_eq!(config.feedback_settings.progress_update_interval_ms, deserialized.feedback_settings.progress_update_interval_ms);
        }
        
        #[test]
        fn test_format_priority_consistency(
            anim_priority in 0u32..=200,
            grp_priority in 0u32..=200,
            pcx_priority in 0u32..=200
        ) {
            let mut config = ExtractionConfig::default();
            config.format_settings.format_priorities.insert(FormatType::ANIM, anim_priority);
            config.format_settings.format_priorities.insert(FormatType::GRP, grp_priority);
            config.format_settings.format_priorities.insert(FormatType::PCX, pcx_priority);
            
            // Configuration should be valid
            prop_assert!(config.validate().is_ok());
            
            // Priority retrieval should be consistent
            prop_assert_eq!(config.get_format_priority(FormatType::ANIM), anim_priority);
            prop_assert_eq!(config.get_format_priority(FormatType::GRP), grp_priority);
            prop_assert_eq!(config.get_format_priority(FormatType::PCX), pcx_priority);
        }
        
        #[test]
        fn test_format_enablement_consistency(
            enable_anim: bool,
            enable_grp: bool,
            enable_pcx: bool
        ) {
            let mut config = ExtractionConfig::default();
            config.format_settings.enabled_formats.clear();
            
            if enable_anim {
                config.format_settings.enabled_formats.push(FormatType::ANIM);
            }
            if enable_grp {
                config.format_settings.enabled_formats.push(FormatType::GRP);
            }
            if enable_pcx {
                config.format_settings.enabled_formats.push(FormatType::PCX);
            }
            
            // If no formats are enabled, validation should fail
            if !enable_anim && !enable_grp && !enable_pcx {
                prop_assert!(config.validate().is_err());
            } else {
                // Otherwise, validation should succeed
                prop_assert!(config.validate().is_ok());
                
                // Format enablement checks should be consistent
                prop_assert_eq!(config.is_format_enabled(FormatType::ANIM), enable_anim);
                prop_assert_eq!(config.is_format_enabled(FormatType::GRP), enable_grp);
                prop_assert_eq!(config.is_format_enabled(FormatType::PCX), enable_pcx);
            }
        }
        
        #[test]
        fn test_thread_count_calculation_consistency(
            configured_threads in 0u32..=32
        ) {
            let mut config = ExtractionConfig::default();
            config.performance_settings.parallel_threads = configured_threads;
            
            let effective_threads = config.get_effective_thread_count();
            
            if configured_threads == 0 {
                // Should auto-detect (use system CPU count)
                prop_assert!(effective_threads > 0);
                prop_assert!(effective_threads <= 128); // Reasonable upper bound
            } else {
                // Should use configured value
                prop_assert_eq!(effective_threads, configured_threads as usize);
            }
        }
    }
}