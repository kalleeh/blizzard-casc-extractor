//! Configuration System for CASC Sprite Extractor
//! 
//! This module provides a comprehensive configuration system that allows users to:
//! - Specify format priorities and quality settings
//! - Configure selective format extraction options
//! - Set memory usage limits and parallel processing controls
//! - Define format conflict resolution strategies
//! - Create reusable configuration profiles
//! - Enable progress reporting and user feedback options

pub mod profiles;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::{Result, Context};
use crate::cli::{ResolutionTier, FormatFilterOption, UnityFilterMode, UnityWrapMode};

/// Main configuration structure for the CASC sprite extractor
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSettings {
    /// Enable pattern analysis
    pub analyze_patterns: bool,
    
    /// Enable format analysis
    pub analyze_formats: bool,
    
    /// Enable performance analysis
    pub analyze_performance: bool,
}

/// Research data collection settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSettings {
    /// Enable research data collection
    pub collect_research_data: bool,
    
    /// Enable format statistics collection
    pub collect_format_statistics: bool,
    
    /// Enable performance metrics collection
    pub collect_performance_metrics: bool,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            format_settings: FormatSettings::default(),
            quality_settings: QualitySettings::default(),
            performance_settings: PerformanceSettings::default(),
            output_settings: OutputSettings::default(),
            feedback_settings: FeedbackSettings::default(),
            filter_settings: FilterSettings::default(),
            analysis_settings: AnalysisSettings::default(),
            research_settings: ResearchSettings::default(),
            custom_settings: HashMap::new(),
        }
    }
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

impl Default for AnalysisSettings {
    fn default() -> Self {
        Self {
            analyze_patterns: false,
            analyze_formats: false,
            analyze_performance: false,
        }
    }
}

impl Default for ResearchSettings {
    fn default() -> Self {
        Self {
            collect_research_data: false,
            collect_format_statistics: false,
            collect_performance_metrics: false,
        }
    }
}

impl ExtractionConfig {
    /// Create a new configuration with default settings
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create configuration from CLI arguments
    pub fn from_cli_args(args: &crate::cli::CliArgs) -> Result<Self> {
        let mut config = Self::default();
        
        // Configure output settings
        config.output_settings.output_directory = args.output_dir.clone();
        
        // Configure Unity settings if enabled
        if args.should_generate_unity_output() {
            config.output_settings.unity_settings.enabled = true;
            config.output_settings.unity_settings.pixels_per_unit = args.unity_pixels_per_unit;
            config.output_settings.unity_settings.filter_mode = args.unity_filter_mode;
            config.output_settings.unity_settings.wrap_mode = args.unity_wrap_mode;
            config.output_settings.unity_settings.compression_quality = args.unity_compression_quality;
            config.output_settings.unity_settings.generate_mipmaps = args.unity_generate_mipmaps;
            config.output_settings.unity_settings.generate_meta_files = true;
        }
        
        // Configure quality settings
        config.quality_settings.resolution_tier = args.resolution;
        config.quality_settings.format_filter = args.format_filter;
        
        // Configure feedback settings
        config.feedback_settings.verbose_logging = args.verbose;
        
        // Configure filter settings
        config.filter_settings.resolution_tier = args.resolution;
        config.filter_settings.max_files = args.max_files.map(|v| v as u64);
        
        if !args.include_patterns.is_empty() {
            config.filter_settings.include_patterns = Some(args.include_patterns.clone());
        }
        
        if !args.exclude_patterns.is_empty() {
            config.filter_settings.exclude_patterns = Some(args.exclude_patterns.clone());
        }
        
        // Configure analysis settings
        config.analysis_settings.analyze_formats = args.analyze_formats;
        config.analysis_settings.analyze_patterns = args.analyze_formats; // Enable pattern analysis when format analysis is enabled
        
        // Configure research settings
        config.research_settings.collect_research_data = args.analyze_formats;
        config.research_settings.collect_format_statistics = args.analyze_formats;
        config.research_settings.collect_performance_metrics = args.verbose;
        
        // Configure format settings based on CLI filters
        if args.format_filter != crate::cli::FormatFilterOption::All {
            // Adjust enabled formats based on filter
            config.format_settings.enabled_formats.clear();
            match args.format_filter {
                crate::cli::FormatFilterOption::Png => {
                    config.format_settings.enabled_formats.push(FormatType::PNG);
                }
                crate::cli::FormatFilterOption::Jpeg => {
                    config.format_settings.enabled_formats.push(FormatType::JPEG);
                }
                crate::cli::FormatFilterOption::Images => {
                    config.format_settings.enabled_formats.push(FormatType::PNG);
                    config.format_settings.enabled_formats.push(FormatType::JPEG);
                }
                crate::cli::FormatFilterOption::All => {
                    // Already set in default
                }
            }
        }
        
        // Configure performance settings
        if let Some(max_files) = args.max_files {
            // Use max_files to estimate batch size
            config.performance_settings.batch_size = ((max_files as u32) / 10).max(1).min(1000);
        }
        
        // Store additional CLI settings in custom_settings for backward compatibility
        config.custom_settings.insert(
            "validate_only".to_string(),
            serde_json::to_value(args.validate_only)?
        );
        
        // Validate the configuration
        config.validate()?;
        
        Ok(config)
    }
    
    /// Load configuration from a file
    pub fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read configuration file: {:?}", path.as_ref()))?;
        
        let config: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse configuration file: {:?}", path.as_ref()))?;
        
        config.validate()?;
        Ok(config)
    }
    
    /// Save configuration to a file
    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        self.validate()?;
        
        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize configuration")?;
        
        std::fs::write(path.as_ref(), content)
            .with_context(|| format!("Failed to write configuration file: {:?}", path.as_ref()))?;
        
        Ok(())
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
    
    /// Merge this configuration with another, with the other taking precedence
    pub fn merge_with(&mut self, other: &ExtractionConfig) {
        // Merge format settings
        self.format_settings.merge_with(&other.format_settings);
        
        // Merge quality settings
        self.quality_settings.merge_with(&other.quality_settings);
        
        // Merge performance settings
        self.performance_settings.merge_with(&other.performance_settings);
        
        // Merge output settings
        self.output_settings.merge_with(&other.output_settings);
        
        // Merge feedback settings
        self.feedback_settings.merge_with(&other.feedback_settings);
        
        // Merge filter settings
        self.filter_settings.merge_with(&other.filter_settings);
        
        // Merge analysis settings
        self.analysis_settings.merge_with(&other.analysis_settings);
        
        // Merge research settings
        self.research_settings.merge_with(&other.research_settings);
        
        // Merge custom settings
        for (key, value) in &other.custom_settings {
            self.custom_settings.insert(key.clone(), value.clone());
        }
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
    
    fn merge_with(&mut self, other: &FormatSettings) {
        // Merge enabled formats (union)
        for format in &other.enabled_formats {
            if !self.enabled_formats.contains(format) {
                self.enabled_formats.push(*format);
            }
        }
        
        // Merge priorities (other takes precedence)
        for (format, priority) in &other.format_priorities {
            self.format_priorities.insert(*format, *priority);
        }
        
        // Merge quality settings (other takes precedence)
        for (format, quality) in &other.format_quality {
            self.format_quality.insert(*format, quality.clone());
        }
        
        // Update other settings
        self.extraction_mode = other.extraction_mode;
        self.conflict_resolution = other.conflict_resolution;
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
    
    fn merge_with(&mut self, other: &QualitySettings) {
        self.resolution_tier = other.resolution_tier;
        self.format_filter = other.format_filter;
        self.png_compression_level = other.png_compression_level;
        self.jpeg_quality = other.jpeg_quality;
        self.prefer_lossless = other.prefer_lossless;
        self.color_depth = other.color_depth;
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
    
    fn merge_with(&mut self, other: &PerformanceSettings) {
        self.max_memory_usage_mb = other.max_memory_usage_mb;
        self.parallel_threads = other.parallel_threads;
        self.use_streaming_processing = other.use_streaming_processing;
        self.use_memory_mapping = other.use_memory_mapping;
        self.use_lazy_loading = other.use_lazy_loading;
        self.enable_object_pooling = other.enable_object_pooling;
        self.batch_size = other.batch_size;
    }
}

impl OutputSettings {
    fn validate(&self) -> Result<()> {
        // Validate Unity settings
        self.unity_settings.validate()
            .context("Invalid Unity settings")?;
        
        Ok(())
    }
    
    fn merge_with(&mut self, other: &OutputSettings) {
        self.output_directory = other.output_directory.clone();
        self.unity_settings.merge_with(&other.unity_settings);
        self.naming_convention = other.naming_convention;
        self.directory_structure = other.directory_structure;
        self.metadata_options.merge_with(&other.metadata_options);
        self.overwrite_behavior = other.overwrite_behavior;
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
    
    fn merge_with(&mut self, other: &FeedbackSettings) {
        self.enable_progress_reporting = other.enable_progress_reporting;
        self.progress_update_interval_ms = other.progress_update_interval_ms;
        self.verbose_logging = other.verbose_logging;
        self.collect_performance_metrics = other.collect_performance_metrics;
        self.collect_research_data = other.collect_research_data;
        self.user_feedback_options.merge_with(&other.user_feedback_options);
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
    
    fn merge_with(&mut self, other: &UnityExportSettings) {
        self.enabled = other.enabled;
        self.pixels_per_unit = other.pixels_per_unit;
        self.filter_mode = other.filter_mode;
        self.wrap_mode = other.wrap_mode;
        self.compression_quality = other.compression_quality;
        self.generate_mipmaps = other.generate_mipmaps;
        self.pivot_point = other.pivot_point;
        self.generate_meta_files = other.generate_meta_files;
    }
}

impl MetadataOptions {
    fn merge_with(&mut self, other: &MetadataOptions) {
        self.generate_json = other.generate_json;
        self.generate_unity_meta = other.generate_unity_meta;
        self.include_animation_data = other.include_animation_data;
        self.include_database_info = other.include_database_info;
        self.include_performance_metrics = other.include_performance_metrics;
        self.include_research_data = other.include_research_data;
    }
}

impl UserFeedbackOptions {
    fn merge_with(&mut self, other: &UserFeedbackOptions) {
        self.show_progress_bar = other.show_progress_bar;
        self.show_file_progress = other.show_file_progress;
        self.show_performance_stats = other.show_performance_stats;
        self.show_memory_usage = other.show_memory_usage;
        self.show_format_details = other.show_format_details;
        self.show_error_details = other.show_error_details;
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
    
    fn merge_with(&mut self, other: &FilterSettings) {
        if other.include_patterns.is_some() {
            self.include_patterns = other.include_patterns.clone();
        }
        if other.exclude_patterns.is_some() {
            self.exclude_patterns = other.exclude_patterns.clone();
        }
        self.resolution_tier = other.resolution_tier;
        if other.max_files.is_some() {
            self.max_files = other.max_files;
        }
    }
}

impl AnalysisSettings {
    fn validate(&self) -> Result<()> {
        // No specific validation needed for analysis settings
        Ok(())
    }
    
    fn merge_with(&mut self, other: &AnalysisSettings) {
        self.analyze_patterns = other.analyze_patterns;
        self.analyze_formats = other.analyze_formats;
        self.analyze_performance = other.analyze_performance;
    }
}

impl ResearchSettings {
    fn validate(&self) -> Result<()> {
        // No specific validation needed for research settings
        Ok(())
    }
    
    fn merge_with(&mut self, other: &ResearchSettings) {
        self.collect_research_data = other.collect_research_data;
        self.collect_format_statistics = other.collect_format_statistics;
        self.collect_performance_metrics = other.collect_performance_metrics;
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
        fn test_configuration_merge_consistency(
            base_threads in 0u32..=16,
            override_threads in 0u32..=16,
            base_memory in 512u64..=8192,
            override_memory in 512u64..=8192
        ) {
            let mut base_config = ExtractionConfig::default();
            base_config.performance_settings.parallel_threads = base_threads;
            base_config.performance_settings.max_memory_usage_mb = base_memory;
            
            let mut override_config = ExtractionConfig::default();
            override_config.performance_settings.parallel_threads = override_threads;
            override_config.performance_settings.max_memory_usage_mb = override_memory;
            
            // Both configurations should be valid
            prop_assert!(base_config.validate().is_ok());
            prop_assert!(override_config.validate().is_ok());
            
            // Merge configurations
            base_config.merge_with(&override_config);
            
            // Merged configuration should be valid
            prop_assert!(base_config.validate().is_ok());
            
            // Override values should take precedence
            prop_assert_eq!(base_config.performance_settings.parallel_threads, override_threads);
            prop_assert_eq!(base_config.performance_settings.max_memory_usage_mb, override_memory);
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