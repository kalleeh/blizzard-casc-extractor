//! Research data generation module
//! 
//! This module provides functionality for collecting and generating research data
//! during CASC extraction operations. The data is suitable for community contribution
//! and helps improve understanding of CASC sprite storage formats.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};

/// Research data collected during extraction operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchData {
    /// Timestamp when research data was collected
    pub timestamp: String,
    
    /// Version of the extraction tool
    pub tool_version: String,
    
    /// Installation path analyzed
    pub installation_path: PathBuf,
    
    /// CASC archive statistics
    pub casc_stats: CascStats,
    
    /// File format analysis results
    pub format_analysis: FormatAnalysis,
    
    /// Extraction statistics
    pub extraction_stats: ExtractionStats,
    
    /// Tool integration results
    pub tool_integration: ToolIntegrationResults,
    
    /// Unknown file signatures discovered
    pub unknown_signatures: Vec<UnknownSignature>,
    
    /// Format-specific success rates and patterns (Requirement 11.1)
    pub format_statistics: FormatStatistics,
    
    /// New format variants discovered during analysis (Requirement 11.2)
    pub format_variants: Vec<FormatVariant>,
    
    /// Performance metrics for community sharing (Requirement 11.3)
    pub performance_metrics: PerformanceMetrics,
}

/// Statistics about the CASC archive structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascStats {
    /// Number of index files found
    pub index_file_count: usize,
    
    /// Number of data files found
    pub data_file_count: usize,
    
    /// Total size of all data files in bytes
    pub total_data_size: u64,
    
    /// Total number of file entries across all indices
    pub total_file_entries: usize,
    
    /// Average entropy of data files (indicates compression level)
    pub average_entropy: f64,
    
    /// Missing or corrupted files detected
    pub corrupted_files: Vec<String>,
}

/// Analysis of file formats found in the archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatAnalysis {
    /// Number of PNG files detected
    pub png_count: u32,
    
    /// Number of JPEG files detected
    pub jpeg_count: u32,
    
    /// Number of DDS files detected
    pub dds_count: u32,
    
    /// Number of ANIM files detected
    pub anim_count: u32,
    
    /// Other file format signatures found
    pub other_formats: HashMap<String, u32>,
    
    /// File size distribution
    pub size_distribution: SizeDistribution,
}

/// Distribution of file sizes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeDistribution {
    /// Files smaller than 1KB
    pub tiny_files: u32,
    
    /// Files between 1KB and 10KB
    pub small_files: u32,
    
    /// Files between 10KB and 100KB
    pub medium_files: u32,
    
    /// Files between 100KB and 1MB
    pub large_files: u32,
    
    /// Files larger than 1MB
    pub huge_files: u32,
}

/// Statistics about the extraction process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionStats {
    /// Number of files successfully extracted
    pub files_extracted: u32,
    
    /// Number of files that failed to extract
    pub extraction_failures: u32,
    
    /// Number of files converted to PNG
    pub png_conversions: u32,
    
    /// Number of conversion failures
    pub conversion_failures: u32,
    
    /// Total extraction time in seconds
    pub extraction_time_seconds: f64,
    
    /// Average file processing time in milliseconds
    pub average_processing_time_ms: f64,
}

/// Results of tool integration attempts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolIntegrationResults {
    /// External tools discovered and tested
    pub tools_tested: Vec<ToolTestResult>,
    
    /// Best performing tool identified
    pub recommended_tool: Option<String>,
    
    /// Integration approach used
    pub integration_method: String,
    
    /// Success rate of tool integration
    pub integration_success_rate: f64,
}

/// Result of testing an external extraction tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTestResult {
    /// Name of the tool tested
    pub tool_name: String,
    
    /// Version of the tool (if available)
    pub tool_version: Option<String>,
    
    /// Whether the tool was compatible
    pub is_compatible: bool,
    
    /// Number of files successfully extracted by the tool
    pub files_extracted: u32,
    
    /// Time taken for extraction in seconds
    pub extraction_time_seconds: f64,
    
    /// Quality assessment of extracted files
    pub output_quality: OutputQuality,
    
    /// Any errors encountered during testing
    pub errors: Vec<String>,
}

/// Assessment of extraction output quality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputQuality {
    /// Percentage of files that are valid images
    pub valid_image_percentage: f64,
    
    /// Percentage of files with correct metadata
    pub correct_metadata_percentage: f64,
    
    /// Whether file structure matches expected format
    pub correct_structure: bool,
    
    /// Overall quality score (0.0 to 1.0)
    pub overall_score: f64,
}

/// Unknown file signature discovered during analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnknownSignature {
    /// First 16 bytes of the file as hex string
    pub signature: String,
    
    /// Number of files with this signature
    pub occurrence_count: u32,
    
    /// Average file size for this signature
    pub average_size: u64,
    
    /// Sample file paths (up to 5)
    pub sample_paths: Vec<String>,
}

/// Format-specific statistics for success rates and patterns (Requirement 11.1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatStatistics {
    /// Success rates by format type
    pub format_success_rates: HashMap<String, FormatSuccessRate>,
    
    /// Pattern analysis for each format
    pub format_patterns: HashMap<String, FormatPattern>,
    
    /// Overall extraction success rate
    pub overall_success_rate: f64,
    
    /// Most successful format parsers
    pub top_performing_formats: Vec<String>,
}

/// Success rate data for a specific format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatSuccessRate {
    /// Format name (ANIM, GRP, PCX, etc.)
    pub format_name: String,
    
    /// Number of files successfully processed
    pub successful_extractions: u32,
    
    /// Number of files that failed processing
    pub failed_extractions: u32,
    
    /// Success rate as percentage (0.0 to 100.0)
    pub success_percentage: f64,
    
    /// Average processing time for successful extractions (ms)
    pub average_processing_time_ms: f64,
    
    /// Common failure reasons
    pub failure_reasons: HashMap<String, u32>,
}

/// Pattern analysis for a specific format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatPattern {
    /// Format name
    pub format_name: String,
    
    /// Common file size ranges
    pub size_patterns: Vec<SizeRange>,
    
    /// Header patterns discovered
    pub header_patterns: Vec<HeaderPattern>,
    
    /// Compression patterns (if applicable)
    pub compression_patterns: Vec<CompressionPattern>,
    
    /// Quality indicators for format detection
    pub detection_confidence_distribution: Vec<ConfidenceRange>,
}

/// File size range pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeRange {
    /// Minimum size in bytes
    pub min_size: u64,
    
    /// Maximum size in bytes
    pub max_size: u64,
    
    /// Number of files in this range
    pub file_count: u32,
    
    /// Success rate for files in this range
    pub success_rate: f64,
}

/// Header pattern discovered in format analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderPattern {
    /// Pattern description
    pub description: String,
    
    /// Hex pattern (first 32 bytes)
    pub hex_pattern: String,
    
    /// Number of files matching this pattern
    pub occurrence_count: u32,
    
    /// Success rate for files with this pattern
    pub success_rate: f64,
}

/// Compression pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionPattern {
    /// Compression type detected
    pub compression_type: String,
    
    /// Number of files using this compression
    pub file_count: u32,
    
    /// Average compression ratio
    pub average_compression_ratio: f64,
    
    /// Decompression success rate
    pub decompression_success_rate: f64,
}

/// Confidence range for format detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceRange {
    /// Minimum confidence (0.0 to 1.0)
    pub min_confidence: f64,
    
    /// Maximum confidence (0.0 to 1.0)
    pub max_confidence: f64,
    
    /// Number of detections in this range
    pub detection_count: u32,
    
    /// Actual success rate for detections in this range
    pub actual_success_rate: f64,
}

/// New format variants discovered during analysis (Requirement 11.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatVariant {
    /// Base format name (ANIM, GRP, PCX, etc.)
    pub base_format: String,
    
    /// Variant identifier
    pub variant_id: String,
    
    /// Description of the variant
    pub description: String,
    
    /// Key differences from base format
    pub differences: Vec<FormatDifference>,
    
    /// Sample files exhibiting this variant
    pub sample_files: Vec<String>,
    
    /// First discovered timestamp
    pub discovered_timestamp: String,
    
    /// Number of files found with this variant
    pub occurrence_count: u32,
    
    /// Extraction success rate for this variant
    pub extraction_success_rate: f64,
}

/// Specific difference from base format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatDifference {
    /// Type of difference (header, structure, compression, etc.)
    pub difference_type: String,
    
    /// Detailed description
    pub description: String,
    
    /// Byte offset where difference occurs (if applicable)
    pub byte_offset: Option<u32>,
    
    /// Expected value in base format
    pub expected_value: Option<String>,
    
    /// Actual value found in variant
    pub actual_value: Option<String>,
}

/// Performance metrics for community sharing (Requirement 11.3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// System information
    pub system_info: SystemInfo,
    
    /// Processing performance by operation type
    pub operation_performance: HashMap<String, OperationPerformance>,
    
    /// Memory usage patterns
    pub memory_usage: MemoryUsageMetrics,
    
    /// I/O performance metrics
    pub io_performance: IoPerformanceMetrics,
    
    /// Scalability metrics
    pub scalability_metrics: ScalabilityMetrics,
    
    /// Comparison with baseline performance
    pub performance_comparison: Option<PerformanceComparison>,
}

/// System information for performance context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system
    pub os: String,
    
    /// CPU information
    pub cpu: String,
    
    /// Total RAM in bytes
    pub total_ram: u64,
    
    /// Available RAM at start in bytes
    pub available_ram: u64,
    
    /// Storage type (SSD, HDD, etc.)
    pub storage_type: String,
    
    /// Number of CPU cores
    pub cpu_cores: u32,
}

/// Performance metrics for a specific operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationPerformance {
    /// Operation name
    pub operation_name: String,
    
    /// Number of operations performed
    pub operation_count: u32,
    
    /// Total time spent in seconds
    pub total_time_seconds: f64,
    
    /// Average time per operation in milliseconds
    pub average_time_ms: f64,
    
    /// Minimum time per operation in milliseconds
    pub min_time_ms: f64,
    
    /// Maximum time per operation in milliseconds
    pub max_time_ms: f64,
    
    /// Standard deviation of operation times
    pub time_std_dev_ms: f64,
    
    /// Operations per second
    pub operations_per_second: f64,
}

/// Memory usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsageMetrics {
    /// Peak memory usage in bytes
    pub peak_memory_usage: u64,
    
    /// Average memory usage in bytes
    pub average_memory_usage: u64,
    
    /// Memory usage by component
    pub component_memory_usage: HashMap<String, u64>,
    
    /// Number of garbage collections (if applicable)
    pub gc_count: u32,
    
    /// Time spent in garbage collection
    pub gc_time_seconds: f64,
}

/// I/O performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoPerformanceMetrics {
    /// Total bytes read
    pub total_bytes_read: u64,
    
    /// Total bytes written
    pub total_bytes_written: u64,
    
    /// Average read speed in MB/s
    pub average_read_speed_mbps: f64,
    
    /// Average write speed in MB/s
    pub average_write_speed_mbps: f64,
    
    /// Number of file operations
    pub file_operations: u32,
    
    /// Average file operation time in milliseconds
    pub average_file_op_time_ms: f64,
}

/// Scalability metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalabilityMetrics {
    /// Performance with different file counts
    pub file_count_scaling: Vec<ScalingDataPoint>,
    
    /// Performance with different file sizes
    pub file_size_scaling: Vec<ScalingDataPoint>,
    
    /// Performance with different thread counts
    pub thread_count_scaling: Vec<ScalingDataPoint>,
    
    /// Memory scaling characteristics
    pub memory_scaling: Vec<ScalingDataPoint>,
}

/// Single data point for scalability analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingDataPoint {
    /// Input parameter value (file count, size, threads, etc.)
    pub parameter_value: f64,
    
    /// Processing time in seconds
    pub processing_time_seconds: f64,
    
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    
    /// Throughput (operations per second)
    pub throughput: f64,
}

/// Performance comparison with baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceComparison {
    /// Baseline version or configuration
    pub baseline_version: String,
    
    /// Performance improvement factor (>1.0 = improvement, <1.0 = regression)
    pub performance_factor: f64,
    
    /// Memory usage factor (>1.0 = more memory, <1.0 = less memory)
    pub memory_factor: f64,
    
    /// Detailed comparison by operation
    pub operation_comparisons: HashMap<String, f64>,
}

/// Validation result for research data quality
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the data passes basic validation
    pub is_valid: bool,
    
    /// Non-critical issues found
    pub warnings: Vec<String>,
    
    /// Critical issues that prevent use
    pub errors: Vec<String>,
    
    /// Completeness score (0.0 to 1.0)
    pub completeness_score: f64,
    
    /// Quality indicators
    pub quality_indicators: HashMap<String, f64>,
}

/// Research data collector that gathers information during extraction
pub struct ResearchDataCollector {
    data: ResearchData,
}

impl ResearchDataCollector {
    /// Create a new research data collector
    pub fn new(installation_path: PathBuf) -> Self {
        let data = ResearchData {
            timestamp: chrono::Utc::now().to_rfc3339(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            installation_path,
            casc_stats: CascStats {
                index_file_count: 0,
                data_file_count: 0,
                total_data_size: 0,
                total_file_entries: 0,
                average_entropy: 0.0,
                corrupted_files: Vec::new(),
            },
            format_analysis: FormatAnalysis {
                png_count: 0,
                jpeg_count: 0,
                dds_count: 0,
                anim_count: 0,
                other_formats: HashMap::new(),
                size_distribution: SizeDistribution {
                    tiny_files: 0,
                    small_files: 0,
                    medium_files: 0,
                    large_files: 0,
                    huge_files: 0,
                },
            },
            extraction_stats: ExtractionStats {
                files_extracted: 0,
                extraction_failures: 0,
                png_conversions: 0,
                conversion_failures: 0,
                extraction_time_seconds: 0.0,
                average_processing_time_ms: 0.0,
            },
            tool_integration: ToolIntegrationResults {
                tools_tested: Vec::new(),
                recommended_tool: None,
                integration_method: "direct_casc_parsing".to_string(),
                integration_success_rate: 0.0,
            },
            unknown_signatures: Vec::new(),
            format_statistics: FormatStatistics {
                format_success_rates: HashMap::new(),
                format_patterns: HashMap::new(),
                overall_success_rate: 0.0,
                top_performing_formats: Vec::new(),
            },
            format_variants: Vec::new(),
            performance_metrics: PerformanceMetrics {
                system_info: SystemInfo {
                    os: std::env::consts::OS.to_string(),
                    cpu: "Unknown".to_string(),
                    total_ram: 0,
                    available_ram: 0,
                    storage_type: "Unknown".to_string(),
                    cpu_cores: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as u32,
                },
                operation_performance: HashMap::new(),
                memory_usage: MemoryUsageMetrics {
                    peak_memory_usage: 0,
                    average_memory_usage: 0,
                    component_memory_usage: HashMap::new(),
                    gc_count: 0,
                    gc_time_seconds: 0.0,
                },
                io_performance: IoPerformanceMetrics {
                    total_bytes_read: 0,
                    total_bytes_written: 0,
                    average_read_speed_mbps: 0.0,
                    average_write_speed_mbps: 0.0,
                    file_operations: 0,
                    average_file_op_time_ms: 0.0,
                },
                scalability_metrics: ScalabilityMetrics {
                    file_count_scaling: Vec::new(),
                    file_size_scaling: Vec::new(),
                    thread_count_scaling: Vec::new(),
                    memory_scaling: Vec::new(),
                },
                performance_comparison: None,
            },
        };
        
        Self {
            data,
        }
    }
    
    /// Record CASC archive statistics
    pub fn record_casc_stats(&mut self, stats: CascStats) {
        self.data.casc_stats = stats;
    }
    
    /// Record file format analysis results
    pub fn record_format_analysis(&mut self, analysis: FormatAnalysis) {
        self.data.format_analysis = analysis;
    }
    
    /// Record extraction statistics
    pub fn record_extraction_stats(&mut self, stats: ExtractionStats) {
        self.data.extraction_stats = stats;
    }
    
    /// Add an unknown file signature
    pub fn add_unknown_signature(&mut self, signature: UnknownSignature) {
        // Check if we already have this signature
        if let Some(existing) = self.data.unknown_signatures.iter_mut()
            .find(|s| s.signature == signature.signature) {
            // Update existing signature with new data
            existing.occurrence_count += signature.occurrence_count;
            existing.average_size = (existing.average_size + signature.average_size) / 2;
            existing.sample_paths.extend(signature.sample_paths);
            existing.sample_paths.truncate(5); // Keep only first 5 samples
        } else {
            self.data.unknown_signatures.push(signature);
        }
    }
    
    /// Finalize research data collection
    pub fn finalize(&mut self) {
        // Calculate overall success rate (Requirement 11.1)
        let total_successful: u32 = self.data.format_statistics.format_success_rates
            .values()
            .map(|rate| rate.successful_extractions)
            .sum();
        
        let total_failed: u32 = self.data.format_statistics.format_success_rates
            .values()
            .map(|rate| rate.failed_extractions)
            .sum();
        
        let total_attempts = total_successful + total_failed;
        if total_attempts > 0 {
            self.data.format_statistics.overall_success_rate = 
                (total_successful as f64 / total_attempts as f64) * 100.0;
        }
        
        // Identify top performing formats
        let mut format_performances: Vec<_> = self.data.format_statistics.format_success_rates
            .iter()
            .map(|(name, rate)| (name.clone(), rate.success_percentage))
            .collect();
        
        format_performances.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        self.data.format_statistics.top_performing_formats = format_performances
            .into_iter()
            .take(5)
            .map(|(name, _)| name)
            .collect();
        
        // Update success rates for patterns
        for (format_name, pattern) in &mut self.data.format_statistics.format_patterns {
            if let Some(success_rate_data) = self.data.format_statistics.format_success_rates.get(format_name) {
                let overall_success = success_rate_data.success_percentage / 100.0;
                
                // Update size pattern success rates (simplified - assume uniform distribution)
                for size_range in &mut pattern.size_patterns {
                    size_range.success_rate = overall_success;
                }
                
                // Update header pattern success rates
                for header_pattern in &mut pattern.header_patterns {
                    header_pattern.success_rate = overall_success;
                }
                
                // Update confidence range success rates
                for confidence_range in &mut pattern.detection_confidence_distribution {
                    confidence_range.actual_success_rate = overall_success;
                }
            }
        }
        
        // Calculate average memory usage
        if !self.data.performance_metrics.memory_usage.component_memory_usage.is_empty() {
            let total_memory: u64 = self.data.performance_metrics.memory_usage.component_memory_usage
                .values().sum();
            self.data.performance_metrics.memory_usage.average_memory_usage = total_memory;
        }
    }
    
    /// Get the collected research data
    pub fn get_data(&self) -> &ResearchData {
        &self.data
    }
    
    /// Save research data to a JSON file
    pub fn save_to_file(&self, output_path: &Path) -> Result<()> {
        let json_data = serde_json::to_string_pretty(&self.data)
            .context("Failed to serialize research data to JSON")?;
        
        std::fs::write(output_path, json_data)
            .with_context(|| format!("Failed to write research data to {:?}", output_path))?;
        
        log::info!("Research data saved to: {:?}", output_path);
        Ok(())
    }
    
    /// Generate a community-shareable research report
    pub fn generate_community_report(&self, output_path: &Path) -> Result<()> {
        let report = self.format_community_report();
        
        std::fs::write(output_path, report)
            .with_context(|| format!("Failed to write community report to {:?}", output_path))?;
        
        log::info!("Community research report saved to: {:?}", output_path);
        Ok(())
    }
    
    /// Format research data as a community-shareable markdown report
    fn format_community_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("# StarCraft: Remastered CASC Extraction Research Report\n\n");
        report.push_str(&format!("**Generated:** {}\n", self.data.timestamp));
        report.push_str(&format!("**Tool Version:** {}\n", self.data.tool_version));
        report.push_str(&format!("**Installation:** {:?}\n\n", self.data.installation_path));
        
        // CASC Archive Statistics
        report.push_str("## CASC Archive Statistics\n\n");
        report.push_str(&format!("- **Index Files:** {}\n", self.data.casc_stats.index_file_count));
        report.push_str(&format!("- **Data Files:** {}\n", self.data.casc_stats.data_file_count));
        report.push_str(&format!("- **Total Size:** {:.2} GB\n", self.data.casc_stats.total_data_size as f64 / 1_073_741_824.0));
        report.push_str(&format!("- **File Entries:** {}\n", self.data.casc_stats.total_file_entries));
        report.push_str(&format!("- **Average Entropy:** {:.3}\n", self.data.casc_stats.average_entropy));
        
        if !self.data.casc_stats.corrupted_files.is_empty() {
            report.push_str(&format!("- **Corrupted Files:** {}\n", self.data.casc_stats.corrupted_files.len()));
        }
        report.push('\n');
        
        // File Format Analysis
        report.push_str("## File Format Analysis\n\n");
        report.push_str(&format!("- **PNG Files:** {}\n", self.data.format_analysis.png_count));
        report.push_str(&format!("- **JPEG Files:** {}\n", self.data.format_analysis.jpeg_count));
        report.push_str(&format!("- **DDS Files:** {}\n", self.data.format_analysis.dds_count));
        report.push_str(&format!("- **ANIM Files:** {}\n", self.data.format_analysis.anim_count));
        
        if !self.data.format_analysis.other_formats.is_empty() {
            report.push_str("- **Other Formats:**\n");
            for (format, count) in &self.data.format_analysis.other_formats {
                report.push_str(&format!("  - {}: {}\n", format, count));
            }
        }
        report.push('\n');
        
        // Size Distribution
        report.push_str("### File Size Distribution\n\n");
        let dist = &self.data.format_analysis.size_distribution;
        report.push_str(&format!("- **< 1KB:** {}\n", dist.tiny_files));
        report.push_str(&format!("- **1KB - 10KB:** {}\n", dist.small_files));
        report.push_str(&format!("- **10KB - 100KB:** {}\n", dist.medium_files));
        report.push_str(&format!("- **100KB - 1MB:** {}\n", dist.large_files));
        report.push_str(&format!("- **> 1MB:** {}\n", dist.huge_files));
        report.push('\n');
        
        // Extraction Results
        report.push_str("## Extraction Results\n\n");
        report.push_str(&format!("- **Files Extracted:** {}\n", self.data.extraction_stats.files_extracted));
        report.push_str(&format!("- **Extraction Failures:** {}\n", self.data.extraction_stats.extraction_failures));
        report.push_str(&format!("- **PNG Conversions:** {}\n", self.data.extraction_stats.png_conversions));
        report.push_str(&format!("- **Conversion Failures:** {}\n", self.data.extraction_stats.conversion_failures));
        report.push_str(&format!("- **Total Time:** {:.2} seconds\n", self.data.extraction_stats.extraction_time_seconds));
        report.push_str(&format!("- **Avg Processing Time:** {:.2} ms/file\n", self.data.extraction_stats.average_processing_time_ms));
        report.push('\n');
        
        // Format Statistics (Requirement 11.1)
        if !self.data.format_statistics.format_success_rates.is_empty() {
            report.push_str("## Format Statistics\n\n");
            report.push_str(&format!("- **Overall Success Rate:** {:.1}%\n", self.data.format_statistics.overall_success_rate));
            
            if !self.data.format_statistics.top_performing_formats.is_empty() {
                report.push_str("- **Top Performing Formats:**\n");
                for format in &self.data.format_statistics.top_performing_formats {
                    if let Some(rate) = self.data.format_statistics.format_success_rates.get(format) {
                        report.push_str(&format!("  - {}: {:.1}% ({} successful, {} failed)\n", 
                                               format, rate.success_percentage, 
                                               rate.successful_extractions, rate.failed_extractions));
                    }
                }
            }
            
            report.push_str("\n### Detailed Format Performance\n\n");
            for (format_name, rate) in &self.data.format_statistics.format_success_rates {
                report.push_str(&format!("#### {}\n", format_name));
                report.push_str(&format!("- **Success Rate:** {:.1}%\n", rate.success_percentage));
                report.push_str(&format!("- **Successful Extractions:** {}\n", rate.successful_extractions));
                report.push_str(&format!("- **Failed Extractions:** {}\n", rate.failed_extractions));
                report.push_str(&format!("- **Average Processing Time:** {:.2} ms\n", rate.average_processing_time_ms));
                
                if !rate.failure_reasons.is_empty() {
                    report.push_str("- **Common Failure Reasons:**\n");
                    for (reason, count) in &rate.failure_reasons {
                        report.push_str(&format!("  - {}: {} occurrences\n", reason, count));
                    }
                }
                report.push('\n');
            }
        }
        
        // Format Variants (Requirement 11.2)
        if !self.data.format_variants.is_empty() {
            report.push_str("## Format Variants Discovered\n\n");
            report.push_str("New format variants discovered during analysis:\n\n");
            
            for variant in &self.data.format_variants {
                report.push_str(&format!("### {} Variant: {}\n", variant.base_format, variant.variant_id));
                report.push_str(&format!("- **Description:** {}\n", variant.description));
                report.push_str(&format!("- **Occurrences:** {}\n", variant.occurrence_count));
                report.push_str(&format!("- **Success Rate:** {:.1}%\n", variant.extraction_success_rate * 100.0));
                report.push_str(&format!("- **First Discovered:** {}\n", variant.discovered_timestamp));
                
                if !variant.differences.is_empty() {
                    report.push_str("- **Key Differences:**\n");
                    for diff in &variant.differences {
                        report.push_str(&format!("  - **{}:** {}\n", diff.difference_type, diff.description));
                        if let Some(offset) = diff.byte_offset {
                            report.push_str(&format!("    - Byte Offset: {}\n", offset));
                        }
                        if let Some(ref expected) = diff.expected_value {
                            report.push_str(&format!("    - Expected: {}\n", expected));
                        }
                        if let Some(ref actual) = diff.actual_value {
                            report.push_str(&format!("    - Actual: {}\n", actual));
                        }
                    }
                }
                
                if !variant.sample_files.is_empty() {
                    report.push_str("- **Sample Files:**\n");
                    for sample in variant.sample_files.iter().take(5) {
                        report.push_str(&format!("  - `{}`\n", sample));
                    }
                }
                report.push('\n');
            }
        }
        
        // Performance Metrics (Requirement 11.3)
        report.push_str("## Performance Metrics\n\n");
        
        // System Information
        report.push_str("### System Information\n\n");
        let sys_info = &self.data.performance_metrics.system_info;
        report.push_str(&format!("- **OS:** {}\n", sys_info.os));
        report.push_str(&format!("- **CPU:** {}\n", sys_info.cpu));
        report.push_str(&format!("- **CPU Cores:** {}\n", sys_info.cpu_cores));
        report.push_str(&format!("- **Total RAM:** {:.2} GB\n", sys_info.total_ram as f64 / 1_073_741_824.0));
        report.push_str(&format!("- **Available RAM:** {:.2} GB\n", sys_info.available_ram as f64 / 1_073_741_824.0));
        report.push_str(&format!("- **Storage Type:** {}\n", sys_info.storage_type));
        report.push('\n');
        
        // Operation Performance
        if !self.data.performance_metrics.operation_performance.is_empty() {
            report.push_str("### Operation Performance\n\n");
            for (op_name, perf) in &self.data.performance_metrics.operation_performance {
                report.push_str(&format!("#### {}\n", op_name));
                report.push_str(&format!("- **Operations:** {}\n", perf.operation_count));
                report.push_str(&format!("- **Total Time:** {:.2}s\n", perf.total_time_seconds));
                report.push_str(&format!("- **Average Time:** {:.2}ms\n", perf.average_time_ms));
                report.push_str(&format!("- **Min Time:** {:.2}ms\n", perf.min_time_ms));
                report.push_str(&format!("- **Max Time:** {:.2}ms\n", perf.max_time_ms));
                report.push_str(&format!("- **Operations/sec:** {:.2}\n", perf.operations_per_second));
                report.push('\n');
            }
        }
        
        // Memory Usage
        let mem_usage = &self.data.performance_metrics.memory_usage;
        if mem_usage.peak_memory_usage > 0 {
            report.push_str("### Memory Usage\n\n");
            report.push_str(&format!("- **Peak Memory:** {:.2} MB\n", mem_usage.peak_memory_usage as f64 / 1_048_576.0));
            report.push_str(&format!("- **Average Memory:** {:.2} MB\n", mem_usage.average_memory_usage as f64 / 1_048_576.0));
            
            if !mem_usage.component_memory_usage.is_empty() {
                report.push_str("- **Memory by Component:**\n");
                for (component, memory) in &mem_usage.component_memory_usage {
                    report.push_str(&format!("  - {}: {:.2} MB\n", component, *memory as f64 / 1_048_576.0));
                }
            }
            report.push('\n');
        }
        
        // I/O Performance
        let io_perf = &self.data.performance_metrics.io_performance;
        if io_perf.file_operations > 0 {
            report.push_str("### I/O Performance\n\n");
            report.push_str(&format!("- **File Operations:** {}\n", io_perf.file_operations));
            report.push_str(&format!("- **Total Bytes Read:** {:.2} MB\n", io_perf.total_bytes_read as f64 / 1_048_576.0));
            report.push_str(&format!("- **Total Bytes Written:** {:.2} MB\n", io_perf.total_bytes_written as f64 / 1_048_576.0));
            report.push_str(&format!("- **Average Read Speed:** {:.2} MB/s\n", io_perf.average_read_speed_mbps));
            report.push_str(&format!("- **Average Write Speed:** {:.2} MB/s\n", io_perf.average_write_speed_mbps));
            report.push_str(&format!("- **Average File Op Time:** {:.2} ms\n", io_perf.average_file_op_time_ms));
            report.push('\n');
        }
        
        // Tool Integration
        if !self.data.tool_integration.tools_tested.is_empty() {
            report.push_str("## Tool Integration Results\n\n");
            report.push_str(&format!("- **Integration Method:** {}\n", self.data.tool_integration.integration_method));
            report.push_str(&format!("- **Success Rate:** {:.1}%\n", self.data.tool_integration.integration_success_rate * 100.0));
            
            if let Some(ref recommended) = self.data.tool_integration.recommended_tool {
                report.push_str(&format!("- **Recommended Tool:** {}\n", recommended));
            }
            
            report.push_str("\n### Tools Tested\n\n");
            for tool in &self.data.tool_integration.tools_tested {
                report.push_str(&format!("#### {}\n", tool.tool_name));
                if let Some(ref version) = tool.tool_version {
                    report.push_str(&format!("- **Version:** {}\n", version));
                }
                report.push_str(&format!("- **Compatible:** {}\n", if tool.is_compatible { "Yes" } else { "No" }));
                report.push_str(&format!("- **Files Extracted:** {}\n", tool.files_extracted));
                report.push_str(&format!("- **Extraction Time:** {:.2}s\n", tool.extraction_time_seconds));
                report.push_str(&format!("- **Quality Score:** {:.2}/1.0\n", tool.output_quality.overall_score));
                
                if !tool.errors.is_empty() {
                    report.push_str("- **Errors:**\n");
                    for error in &tool.errors {
                        report.push_str(&format!("  - {}\n", error));
                    }
                }
                report.push('\n');
            }
        }
        
        // Unknown Signatures
        if !self.data.unknown_signatures.is_empty() {
            report.push_str("## Unknown File Signatures\n\n");
            report.push_str("These signatures were found but not recognized as standard formats:\n\n");
            
            for sig in &self.data.unknown_signatures {
                report.push_str(&format!("### Signature: `{}`\n", sig.signature));
                report.push_str(&format!("- **Occurrences:** {}\n", sig.occurrence_count));
                report.push_str(&format!("- **Average Size:** {} bytes\n", sig.average_size));
                
                if !sig.sample_paths.is_empty() {
                    report.push_str("- **Sample Paths:**\n");
                    for path in &sig.sample_paths {
                        report.push_str(&format!("  - `{}`\n", path));
                    }
                }
                report.push('\n');
            }
        }
        
        // Footer
        report.push_str("---\n\n");
        report.push_str("*This report was generated by the CASC Sprite Extractor research system.*\n");
        report.push_str("*Please share this data with the StarCraft modding community to improve our collective understanding.*\n");
        
        report
    }

}

/// Analyze file data to detect format and generate signature
pub fn analyze_file_data(data: &[u8]) -> (String, Option<String>) {
    if data.len() < 4 {
        return ("unknown".to_string(), None);
    }
    
    // Check for common image formats
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return ("PNG".to_string(), Some("89504E47".to_string()));
    }
    
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return ("JPEG".to_string(), Some("FFD8FF".to_string()));
    }
    
    if data.starts_with(b"DDS ") {
        return ("DDS".to_string(), Some("44445320".to_string()));
    }
    
    if data.starts_with(b"ANIM") {
        return ("ANIM".to_string(), Some("414E494D".to_string()));
    }
    
    // Generate hex signature for unknown formats
    let signature = data.iter()
        .take(16)
        .map(|b| format!("{:02X}", b))
        .collect::<String>();
    
    ("unknown".to_string(), Some(signature))
}

/// Calculate entropy of data (measure of compression/randomness)
pub fn calculate_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    
    let mut counts = [0u32; 256];
    for &byte in data {
        counts[byte as usize] += 1;
    }
    
    let len = data.len() as f64;
    let mut entropy = 0.0;
    
    for &count in &counts {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }
    
    entropy
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_research_data_collector_creation() {
        let temp_dir = TempDir::new().unwrap();
        let collector = ResearchDataCollector::new(temp_dir.path().to_path_buf());
        
        assert_eq!(collector.data.tool_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(collector.data.installation_path, temp_dir.path());
        assert_eq!(collector.data.casc_stats.index_file_count, 0);
    }
    
    #[test]
    fn test_analyze_file_data_png() {
        let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let (format, signature) = analyze_file_data(&png_data);
        
        assert_eq!(format, "PNG");
        assert_eq!(signature, Some("89504E47".to_string()));
    }
    
    #[test]
    fn test_analyze_file_data_jpeg() {
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0];
        let (format, signature) = analyze_file_data(&jpeg_data);
        
        assert_eq!(format, "JPEG");
        assert_eq!(signature, Some("FFD8FF".to_string()));
    }
    
    #[test]
    fn test_analyze_file_data_unknown() {
        let unknown_data = vec![0x12, 0x34, 0x56, 0x78];
        let (format, signature) = analyze_file_data(&unknown_data);
        
        assert_eq!(format, "unknown");
        assert_eq!(signature, Some("12345678".to_string()));
    }
    
    #[test]
    fn test_calculate_entropy_uniform() {
        // Uniform distribution should have high entropy
        let data: Vec<u8> = (0..=255).collect();
        let entropy = calculate_entropy(&data);
        
        // Should be close to 8.0 (maximum entropy for bytes)
        assert!(entropy > 7.9);
        assert!(entropy <= 8.0);
    }
    
    #[test]
    fn test_calculate_entropy_single_value() {
        // Single repeated value should have zero entropy
        let data = vec![0x42; 1000];
        let entropy = calculate_entropy(&data);
        
        assert_eq!(entropy, 0.0);
    }
    
    #[test]
    fn test_unknown_signature_deduplication() {
        let temp_dir = TempDir::new().unwrap();
        let mut collector = ResearchDataCollector::new(temp_dir.path().to_path_buf());
        
        // Add the same signature twice
        let sig1 = UnknownSignature {
            signature: "12345678".to_string(),
            occurrence_count: 5,
            average_size: 1000,
            sample_paths: vec!["path1".to_string()],
        };
        
        let sig2 = UnknownSignature {
            signature: "12345678".to_string(),
            occurrence_count: 3,
            average_size: 2000,
            sample_paths: vec!["path2".to_string()],
        };
        
        collector.add_unknown_signature(sig1);
        collector.add_unknown_signature(sig2);
        
        // Should have only one signature with combined data
        assert_eq!(collector.data.unknown_signatures.len(), 1);
        let combined = &collector.data.unknown_signatures[0];
        assert_eq!(combined.occurrence_count, 8); // 5 + 3
        assert_eq!(combined.average_size, 1500); // (1000 + 2000) / 2
        assert_eq!(combined.sample_paths.len(), 2);
    }
    
    #[test]
    fn test_save_and_load_research_data() {
        let temp_dir = TempDir::new().unwrap();
        let mut collector = ResearchDataCollector::new(temp_dir.path().to_path_buf());
        
        // Add some test data using the proper methods
        let casc_stats = CascStats {
            index_file_count: 16,
            data_file_count: 6,
            total_data_size: 5_687_091_200,
            total_file_entries: 1000,
            average_entropy: 7.97,
            corrupted_files: Vec::new(),
        };
        collector.record_casc_stats(casc_stats);
        
        let format_analysis = FormatAnalysis {
            png_count: 24,
            jpeg_count: 8,
            dds_count: 12,
            anim_count: 4,
            other_formats: HashMap::new(),
            size_distribution: SizeDistribution {
                tiny_files: 10,
                small_files: 20,
                medium_files: 30,
                large_files: 15,
                huge_files: 5,
            },
        };
        collector.record_format_analysis(format_analysis);

        let output_file = temp_dir.path().join("research_data.json");
        collector.save_to_file(&output_file).unwrap();
        
        // Verify file was created and contains expected data
        assert!(output_file.exists());
        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("\"index_file_count\": 16"));
        assert!(content.contains("\"png_count\": 24"));
    }
    
    #[test]
    fn test_generate_community_report() {
        let temp_dir = TempDir::new().unwrap();
        let mut collector = ResearchDataCollector::new(temp_dir.path().to_path_buf());
        
        // Add some test data
        collector.data.casc_stats.index_file_count = 16;
        collector.data.casc_stats.data_file_count = 6;
        collector.data.format_analysis.png_count = 24;
        collector.data.format_analysis.jpeg_count = 8;
        
        let output_file = temp_dir.path().join("community_report.md");
        collector.generate_community_report(&output_file).unwrap();
        
        // Verify report was created and contains expected sections
        assert!(output_file.exists());
        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("# StarCraft: Remastered CASC Extraction Research Report"));
        assert!(content.contains("## CASC Archive Statistics"));
        assert!(content.contains("## File Format Analysis"));
        assert!(content.contains("**Index Files:** 16"));
        assert!(content.contains("**PNG Files:** 24"));
    }
    
}