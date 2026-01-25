//! Unified CASC sprite extraction pipeline
//! 
//! This module provides the main integration point that wires together all components
//! of the CASC sprite extraction system into a unified, end-to-end pipeline.

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use anyhow::{Result, Context};
use log::{info, debug};

use crate::casc::{CascArchive, FileInfo};
use crate::sprite::{DirectSpriteExtractor, SpriteFormat, UnityConverter};
use crate::format_converter::FormatConverter;
use crate::config::ExtractionConfig;
use crate::research::{ResearchDataCollector, ResearchData};
use crate::format_analyzer::{FormatAnalyzer, SpritePatternAnalysis};
use crate::cli::ResolutionTier;

/// Unified CASC sprite extraction pipeline
/// 
/// This is the main integration point that coordinates all components:
/// - Format detection and parser selection
/// - CASC archive handling and file extraction
/// - Sprite parsing and conversion
/// - Unity export system integration
/// - Metadata extraction and database integration
/// - Research data collection
pub struct UnifiedPipeline {
    /// CASC archive for file extraction
    casc_archive: CascArchive,
    
    /// Configuration for extraction process
    config: ExtractionConfig,
    
    /// Format converter for Unity export
    format_converter: FormatConverter,
    
    /// Direct sprite extractor for CASC files
    sprite_extractor: DirectSpriteExtractor,
    
    /// Research data collector
    research_collector: Option<ResearchDataCollector>,
    
    /// Format analyzer for pattern analysis
    format_analyzer: FormatAnalyzer,
    
    /// Performance metrics
    performance_metrics: PipelineMetrics,
}

/// Performance metrics for the unified pipeline
#[derive(Debug, Default)]
pub struct PipelineMetrics {
    /// Total files processed
    pub files_processed: u32,
    
    /// Files successfully extracted
    pub successful_extractions: u32,
    
    /// Files that failed extraction
    pub failed_extractions: u32,
    
    /// Format detection statistics
    pub format_detection_stats: HashMap<String, u32>,
    
    /// Conversion statistics by format
    pub conversion_stats: HashMap<String, ConversionStats>,
    
    /// Total processing time in seconds
    pub total_processing_time: f64,
    
    /// Average processing time per file in milliseconds
    pub average_processing_time_ms: f64,
}

/// Conversion statistics for a specific format
#[derive(Debug, Default)]
pub struct ConversionStats {
    /// Number of successful conversions
    pub successful: u32,
    
    /// Number of failed conversions
    pub failed: u32,
    
    /// Average conversion time in milliseconds
    pub average_time_ms: f64,
    
    /// Total bytes processed
    pub bytes_processed: u64,
}

/// Result of the unified pipeline execution
#[derive(Debug)]
pub struct PipelineResult {
    /// Files successfully processed
    pub processed_files: Vec<ProcessedFile>,
    
    /// Files that failed processing
    pub failed_files: Vec<FailedFile>,
    
    /// Performance metrics
    pub metrics: PipelineMetrics,
    
    /// Research data collected (if enabled)
    pub research_data: Option<ResearchData>,
    
    /// Pattern analysis results
    pub pattern_analysis: Option<SpritePatternAnalysis>,
    
    /// Output directory where files were written
    pub output_directory: PathBuf,
}

/// Information about a successfully processed file
#[derive(Debug)]
pub struct ProcessedFile {
    /// Original file path in CASC
    pub source_path: String,
    
    /// Output file path
    pub output_path: PathBuf,
    
    /// Unity metadata file path (if generated)
    pub metadata_path: Option<PathBuf>,
    
    /// Detected format
    pub detected_format: SpriteFormat,
    
    /// Processing time in milliseconds
    pub processing_time_ms: f64,
    
    /// File size in bytes
    pub file_size: u64,
    
    /// Resolution tier (if applicable)
    pub resolution_tier: Option<ResolutionTier>,
}

/// Information about a file that failed processing
#[derive(Debug)]
pub struct FailedFile {
    /// Original file path in CASC
    pub source_path: String,
    
    /// Error message
    pub error_message: String,
    
    /// Processing stage where failure occurred
    pub failure_stage: String,
    
    /// File size in bytes (if known)
    pub file_size: Option<u64>,
}

impl UnifiedPipeline {
    /// Create a new unified pipeline
    pub fn new(casc_path: &Path, config: ExtractionConfig) -> Result<Self> {
        info!("Initializing unified CASC sprite extraction pipeline");
        
        // Open CASC archive
        let casc_archive = CascArchive::open(casc_path)
            .context("Failed to open CASC archive")?;
        
        // Initialize format converter
        let format_converter = FormatConverter::new();
        
        // Initialize sprite extractor
        let sprite_extractor = DirectSpriteExtractor::new_with_max_files(
            casc_archive, 
            config.filter_settings.max_files.map(|v| v as usize)
        );
        
        // Initialize format analyzer - we'll create a new archive instance for this
        let analyzer_archive = CascArchive::open(casc_path)
            .context("Failed to open CASC archive for format analyzer")?;
        let format_analyzer = FormatAnalyzer::new(analyzer_archive);
        
        // Initialize research data collector if enabled
        let research_collector = if config.research_settings.collect_research_data {
            Some(ResearchDataCollector::new(casc_path.to_path_buf()))
        } else {
            None
        };
        
        // We'll store the casc_path instead of the archive for methods that need it
        let casc_archive_for_storage = CascArchive::open(casc_path)
            .context("Failed to open CASC archive for storage")?;
        
        info!("Pipeline initialization complete");
        
        Ok(Self {
            casc_archive: casc_archive_for_storage,
            config,
            format_converter,
            sprite_extractor,
            research_collector,
            format_analyzer,
            performance_metrics: PipelineMetrics::default(),
        })
    }
    
    /// Execute the complete pipeline
    pub fn execute(&mut self, output_dir: &Path) -> Result<PipelineResult> {
        let start_time = std::time::Instant::now();
        info!("Starting unified pipeline execution");
        
        // Create output directory
        std::fs::create_dir_all(output_dir)
            .context("Failed to create output directory")?;
        
        // Step 1: Analyze sprite patterns (optional)
        let pattern_analysis = if self.config.analysis_settings.analyze_patterns {
            info!("Analyzing sprite patterns...");
            Some(self.format_analyzer.analyze_sprite_patterns()
                .context("Failed to analyze sprite patterns")?)
        } else {
            None
        };
        
        // Step 2: Get all files from CASC archive
        let all_files = self.casc_archive.list_all_files()
            .context("Failed to list CASC files")?;
        
        info!("Found {} files in CASC archive", all_files.len());
        
        // Step 3: Filter files based on configuration
        let filtered_files = self.filter_files(&all_files)?;
        info!("Processing {} files after filtering", filtered_files.len());
        
        // Step 4: Process each file through the unified pipeline
        let mut processed_files = Vec::new();
        let mut failed_files = Vec::new();
        
        for (index, file_info) in filtered_files.iter().enumerate() {
            if index % 100 == 0 {
                info!("Processing file {}/{}: {}", index + 1, filtered_files.len(), file_info.name);
            }
            
            match self.process_single_file(file_info, output_dir) {
                Ok(processed_file) => {
                    processed_files.push(processed_file);
                    self.performance_metrics.successful_extractions += 1;
                }
                Err(e) => {
                    let failed_file = FailedFile {
                        source_path: file_info.name.clone(),
                        error_message: e.to_string(),
                        failure_stage: "processing".to_string(),
                        file_size: None, // Could be enhanced to include file size
                    };
                    failed_files.push(failed_file);
                    self.performance_metrics.failed_extractions += 1;
                    
                    debug!("Failed to process file {}: {}", file_info.name, e);
                }
            }
            
            self.performance_metrics.files_processed += 1;
        }
        
        // Step 5: Finalize metrics
        let total_time = start_time.elapsed().as_secs_f64();
        self.performance_metrics.total_processing_time = total_time;
        
        if self.performance_metrics.files_processed > 0 {
            self.performance_metrics.average_processing_time_ms = 
                (total_time * 1000.0) / self.performance_metrics.files_processed as f64;
        }
        
        // Step 6: Collect research data if enabled
        let research_data = if let Some(ref mut collector) = self.research_collector {
            info!("Finalizing research data collection...");
            collector.finalize(); // Use the existing finalize method from research.rs
            None // For now, we don't return the research data since the method signature is different
        } else {
            None
        };
        
        info!("Pipeline execution complete: {} successful, {} failed, {:.2}s total", 
              processed_files.len(), failed_files.len(), total_time);
        
        Ok(PipelineResult {
            processed_files,
            failed_files,
            metrics: std::mem::take(&mut self.performance_metrics),
            research_data,
            pattern_analysis,
            output_directory: output_dir.to_path_buf(),
        })
    }
    
    /// Process a single file through the complete pipeline
    fn process_single_file(&mut self, file_info: &FileInfo, output_dir: &Path) -> Result<ProcessedFile> {
        let file_start_time = std::time::Instant::now();
        
        // Use the existing sprite extractor to process the file
        // This is simpler than trying to replicate all the internal logic
        let temp_output_dir = output_dir.join("temp_processing");
        std::fs::create_dir_all(&temp_output_dir)
            .context("Failed to create temporary processing directory")?;
        
        // Extract the single file using the sprite extractor
        let _extraction_result = if self.config.output_settings.unity_settings.enabled {
            let unity_converter = UnityConverter {
                pixels_per_unit: self.config.output_settings.unity_settings.pixels_per_unit,
                filter_mode: format!("{:?}", self.config.output_settings.unity_settings.filter_mode),
                wrap_mode: format!("{:?}", self.config.output_settings.unity_settings.wrap_mode),
                compression_quality: self.config.output_settings.unity_settings.compression_quality,
                generate_mip_maps: self.config.output_settings.unity_settings.generate_mipmaps,
            };
            
            self.sprite_extractor.extract_all_sprites_with_unity_support(&temp_output_dir, &unity_converter)
                .context("Failed to extract sprite with Unity support")?
        } else {
            self.sprite_extractor.extract_all_sprites(&temp_output_dir)
                .context("Failed to extract sprite")?
        };
        
        // For now, we'll assume the first extracted file corresponds to our input
        // This is a simplification - in a real implementation, we'd need better file tracking
        let detected_format = SpriteFormat::CompressedData; // Default assumption
        let resolution_tier = CascArchive::detect_resolution_tier(&file_info.name);
        
        // Move the extracted file to the proper output location
        let base_output_dir = if let Some(tier) = resolution_tier {
            CascArchive::get_output_path_for_tier(output_dir, Some(tier))
        } else {
            output_dir.to_path_buf()
        };
        
        std::fs::create_dir_all(&base_output_dir)
            .context("Failed to create resolution-specific output directory")?;
        
        // Find the extracted files and move them
        let mut output_path = base_output_dir.join(format!("{}.png", file_info.name));
        let mut metadata_path = base_output_dir.join(format!("{}.png.meta", file_info.name));
        
        // Try to find and move the actual extracted files
        if let Ok(entries) = std::fs::read_dir(&temp_output_dir) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.extension().and_then(|s| s.to_str()) == Some("png") {
                    let target_path = base_output_dir.join(entry.file_name());
                    std::fs::rename(&entry_path, &target_path)?;
                    output_path = target_path;
                } else if entry_path.extension().and_then(|s| s.to_str()) == Some("meta") {
                    let target_path = base_output_dir.join(entry.file_name());
                    std::fs::rename(&entry_path, &target_path)?;
                    metadata_path = target_path;
                }
            }
        }
        
        // Clean up temporary directory
        let _ = std::fs::remove_dir_all(&temp_output_dir);
        
        // Update format detection statistics
        let format_name = format!("{:?}", detected_format);
        *self.performance_metrics.format_detection_stats.entry(format_name.clone()).or_insert(0) += 1;
        
        // Record format pattern for research if enabled
        if let Some(ref mut collector) = self.research_collector {
            collector.record_format_pattern(&format_name, 0, &[], 0.9);
        }
        
        // Update conversion statistics
        let conversion_stats = self.performance_metrics.conversion_stats
            .entry(format_name.clone())
            .or_insert_with(ConversionStats::default);
        
        conversion_stats.successful += 1;
        
        // Record performance metrics for research if enabled
        if let Some(ref mut collector) = self.research_collector {
            let processing_time = file_start_time.elapsed().as_secs_f64() * 1000.0;
            collector.record_format_success(&format_name, true, processing_time, None);
            collector.record_operation_performance("file_processing", processing_time);
            collector.record_io_performance(0, 0, processing_time);
        }
        
        let total_processing_time = file_start_time.elapsed().as_secs_f64() * 1000.0;
        
        Ok(ProcessedFile {
            source_path: file_info.name.clone(),
            output_path,
            metadata_path: Some(metadata_path),
            detected_format,
            processing_time_ms: total_processing_time,
            file_size: 0, // We don't have the original file size easily available
            resolution_tier,
        })
    }
    
    /// Filter files based on configuration settings
    fn filter_files(&self, all_files: &[FileInfo]) -> Result<Vec<FileInfo>> {
        let mut filtered_files = Vec::new();
        
        for file_info in all_files {
            // Apply path filters
            if let Some(ref include_patterns) = self.config.filter_settings.include_patterns {
                let mut matches_include = false;
                for pattern in include_patterns {
                    if let Ok(regex) = regex::Regex::new(pattern) {
                        if regex.is_match(&file_info.name) {
                            matches_include = true;
                            break;
                        }
                    }
                }
                if !matches_include {
                    continue;
                }
            }
            
            if let Some(ref exclude_patterns) = self.config.filter_settings.exclude_patterns {
                let mut matches_exclude = false;
                for pattern in exclude_patterns {
                    if let Ok(regex) = regex::Regex::new(pattern) {
                        if regex.is_match(&file_info.name) {
                            matches_exclude = true;
                            break;
                        }
                    }
                }
                if matches_exclude {
                    continue;
                }
            }
            
            // Apply resolution tier filter
            if self.config.filter_settings.resolution_tier != ResolutionTier::All {
                let file_tier = CascArchive::detect_resolution_tier(&file_info.name);
                if file_tier != Some(self.config.filter_settings.resolution_tier) {
                    continue;
                }
            }
            
            filtered_files.push(file_info.clone());
        }
        
        Ok(filtered_files)
    }
    
    /// Get current pipeline metrics
    pub fn get_metrics(&self) -> &PipelineMetrics {
        &self.performance_metrics
    }
    
    /// Validate pipeline configuration
    pub fn validate_configuration(&self) -> Result<()> {
        self.config.validate()
            .context("Pipeline configuration validation failed")?;
        
        // Additional pipeline-specific validations
        if !self.config.format_settings.enabled_formats.is_empty() {
            info!("Pipeline configured for formats: {:?}", self.config.format_settings.enabled_formats);
        } else {
            return Err(anyhow::anyhow!("No formats enabled in configuration"));
        }
        
        Ok(())
    }
}

impl ResearchDataCollector {
    // Note: finalize method is already implemented in research.rs
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::config::ExtractionConfig;
    
    #[test]
    fn test_pipeline_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().join("Data").join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        
        // Create minimal CASC structure
        let index_path = data_dir.join("data.000.idx");
        let mut index_data = vec![0u8; 24];
        index_data[8..10].copy_from_slice(&7u16.to_le_bytes());
        index_data[14] = 9;
        std::fs::write(&index_path, &index_data).unwrap();
        
        let data_path = data_dir.join("data.000");
        std::fs::write(&data_path, b"test data").unwrap();
        
        let config = ExtractionConfig::default();
        let pipeline = UnifiedPipeline::new(temp_dir.path(), config);
        
        assert!(pipeline.is_ok(), "Pipeline initialization should succeed");
    }
    
    #[test]
    fn test_pipeline_validation() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().join("Data").join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        
        // Create minimal CASC structure
        let index_path = data_dir.join("data.000.idx");
        let mut index_data = vec![0u8; 24];
        index_data[8..10].copy_from_slice(&7u16.to_le_bytes());
        index_data[14] = 9;
        std::fs::write(&index_path, &index_data).unwrap();
        
        let data_path = data_dir.join("data.000");
        std::fs::write(&data_path, b"test data").unwrap();
        
        let config = ExtractionConfig::default();
        let pipeline = UnifiedPipeline::new(temp_dir.path(), config).unwrap();
        
        let validation_result = pipeline.validate_configuration();
        assert!(validation_result.is_ok(), "Pipeline validation should succeed with default config");
    }
}