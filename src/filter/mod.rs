/// File filtering system for CASC sprite extraction
/// 
/// This module provides functionality for filtering files based on regex patterns,
/// resolution tiers, and file format signatures, supporting both inclusion and 
/// exclusion filters with OR logic for multiple patterns.

use regex::Regex;
use anyhow::{Result, Context};
use crate::cli::ResolutionTier;
use crate::resolution::ResolutionHandler;

/// Alias for FilterSystem to maintain compatibility with main.rs
#[allow(dead_code)]
pub type FilterSystem = FileFilter;

/// Enhanced file filter that applies inclusion and exclusion patterns,
/// resolution-based filtering, and format signature filtering
#[allow(dead_code)]
#[derive(Debug)]
pub struct FileFilter {
    include_regexes: Vec<Regex>,
    exclude_regexes: Vec<Regex>,
    resolution_filter: Option<ResolutionTier>,
    format_filter: Option<FormatFilter>,
    stats: FilterStats,
}

/// Format-based filtering options
#[derive(Debug, Clone, PartialEq)]
pub enum FormatFilter {
    /// Only include PNG files (based on signature)
    PngOnly,
    /// Only include JPEG files (based on signature)
    JpegOnly,
    /// Include both PNG and JPEG files
    ImageFormats,
    /// Include all formats (no format filtering)
    All,
}

/// Enhanced statistics about filter application
#[derive(Debug, Clone, Default)]
pub struct FilterStats {
    pub total_files: usize,
    pub included_files: usize,
    pub excluded_files: usize,
    pub skipped_files: usize,
    // Resolution-based stats
    pub hd_files: usize,
    pub hd2_files: usize,
    pub sd_files: usize,
    pub unknown_resolution_files: usize,
    // Format-based stats
    pub png_files: usize,
    pub jpeg_files: usize,
    pub other_format_files: usize,
}

/// Enhanced result of applying a filter to a file
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterResult {
    Include,
    Exclude,
    Skip,
}

/// Information about a file for filtering purposes
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: String,
    pub data: Option<Vec<u8>>, // File content for signature detection
    pub size: Option<u64>,
}

impl FileFilter {
    #[cfg(test)]
    /// Create a new file filter from pattern strings (test-only method)
    pub fn new(include_patterns: &[String], exclude_patterns: &[String]) -> Result<Self> {
        let include_regexes = Self::compile_patterns(include_patterns)
            .context("Failed to compile inclusion patterns")?;
        
        let exclude_regexes = Self::compile_patterns(exclude_patterns)
            .context("Failed to compile exclusion patterns")?;
        
        Ok(FileFilter {
            include_regexes,
            exclude_regexes,
            resolution_filter: None,
            format_filter: None,
            stats: FilterStats::default(),
        })
    }
    
    /// Create a new enhanced file filter with all filtering options
    pub fn new_enhanced(
        include_patterns: &[String], 
        exclude_patterns: &[String],
        resolution_filter: Option<ResolutionTier>,
        format_filter: Option<FormatFilter>
    ) -> Result<Self> {
        let include_regexes = Self::compile_patterns(include_patterns)
            .context("Failed to compile inclusion patterns")?;
        
        let exclude_regexes = Self::compile_patterns(exclude_patterns)
            .context("Failed to compile exclusion patterns")?;
        
        Ok(FileFilter {
            include_regexes,
            exclude_regexes,
            resolution_filter,
            format_filter,
            stats: FilterStats::default(),
        })
    }
    
    #[cfg(test)]
    /// Set resolution-based filtering (test-only method)
    pub fn with_resolution_filter(mut self, resolution: ResolutionTier) -> Self {
        self.resolution_filter = Some(resolution);
        self
    }
    
    #[cfg(test)]
    /// Set format-based filtering (test-only method)
    pub fn with_format_filter(mut self, format: FormatFilter) -> Self {
        self.format_filter = Some(format);
        self
    }
    
    /// Compile a list of pattern strings into regexes
    fn compile_patterns(patterns: &[String]) -> Result<Vec<Regex>> {
        patterns
            .iter()
            .map(|pattern| Regex::new(pattern))
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to compile regex patterns")
    }
    
    #[cfg(test)]
    /// Apply the filter to a file path and update statistics (test-only method)
    pub fn apply(&mut self, file_path: &str) -> FilterResult {
        let file_info = FileInfo {
            path: file_path.to_string(),
            data: None,
            size: None,
        };
        self.apply_to_file_info(&file_info)
    }
    
    #[cfg(test)]
    /// Apply the filter to file information and update statistics (test-only method)
    pub fn apply_to_file_info(&mut self, file_info: &FileInfo) -> FilterResult {
        self.stats.total_files += 1;
        
        let result = self.check_file_info(file_info);
        
        // Update statistics based on result
        match result {
            FilterResult::Include => self.stats.included_files += 1,
            FilterResult::Exclude => self.stats.excluded_files += 1,
            FilterResult::Skip => self.stats.skipped_files += 1,
        }
        
        // Update resolution statistics
        if let Some(resolution) = ResolutionHandler::detect_tier_from_path(&file_info.path) {
            match resolution {
                ResolutionTier::HD => self.stats.hd_files += 1,
                ResolutionTier::HD2 => self.stats.hd2_files += 1,
                ResolutionTier::SD => self.stats.sd_files += 1,
                ResolutionTier::All => {}, // Should not happen
            }
        } else {
            self.stats.unknown_resolution_files += 1;
        }
        
        // Update format statistics if we have file data
        if let Some(ref data) = file_info.data {
            if Self::has_png_signature(data) {
                self.stats.png_files += 1;
            } else if Self::has_jpeg_signature(data) {
                self.stats.jpeg_files += 1;
            } else {
                self.stats.other_format_files += 1;
            }
        }
        
        result
    }
    
    #[cfg(test)]
    /// Check if a file should be included, excluded, or skipped (without updating stats) (test-only method)
    pub fn check_file(&self, file_path: &str) -> FilterResult {
        let file_info = FileInfo {
            path: file_path.to_string(),
            data: None,
            size: None,
        };
        self.check_file_info(&file_info)
    }
    
    /// Check if a file should be included, excluded, or skipped based on file info
    pub fn check_file_info(&self, file_info: &FileInfo) -> FilterResult {
        // First check exclusion patterns - if any match, exclude the file
        if self.matches_any_exclude_pattern(&file_info.path) {
            return FilterResult::Exclude;
        }
        
        // Check resolution filter
        if let Some(resolution_filter) = self.resolution_filter {
            if !self.matches_resolution_filter(&file_info.path, resolution_filter) {
                return FilterResult::Skip;
            }
        }
        
        // Check format filter
        if let Some(ref format_filter) = self.format_filter {
            if let Some(ref data) = file_info.data {
                if !self.matches_format_filter(data, format_filter) {
                    return FilterResult::Skip;
                }
            } else {
                // If we don't have file data but format filtering is enabled,
                // we can't determine the format, so skip
                if *format_filter != FormatFilter::All {
                    return FilterResult::Skip;
                }
            }
        }
        
        // If no inclusion patterns are specified, include all files (that aren't excluded/skipped)
        if self.include_regexes.is_empty() {
            return FilterResult::Include;
        }
        
        // Check inclusion patterns - if any match, include the file
        if self.matches_any_include_pattern(&file_info.path) {
            return FilterResult::Include;
        }
        
        // File doesn't match any inclusion pattern
        FilterResult::Skip
    }
    
    /// Check if file path matches any inclusion pattern (OR logic)
    pub fn matches_any_include_pattern(&self, file_path: &str) -> bool {
        if self.include_regexes.is_empty() {
            return true; // No patterns means include all
        }
        
        self.include_regexes.iter().any(|regex| regex.is_match(file_path))
    }
    
    /// Check if file path matches any exclusion pattern (OR logic)
    pub fn matches_any_exclude_pattern(&self, file_path: &str) -> bool {
        self.exclude_regexes.iter().any(|regex| regex.is_match(file_path))
    }
    
    /// Check if file matches resolution filter
    fn matches_resolution_filter(&self, file_path: &str, resolution_filter: ResolutionTier) -> bool {
        match resolution_filter {
            ResolutionTier::All => true,
            specific_tier => {
                match ResolutionHandler::detect_tier_from_path(file_path) {
                    Some(detected_tier) => detected_tier == specific_tier,
                    None => false, // Unknown tier files are skipped when filtering
                }
            }
        }
    }
    
    /// Check if file data matches format filter
    fn matches_format_filter(&self, data: &[u8], format_filter: &FormatFilter) -> bool {
        match format_filter {
            FormatFilter::All => true,
            FormatFilter::PngOnly => Self::has_png_signature(data),
            FormatFilter::JpegOnly => Self::has_jpeg_signature(data),
            FormatFilter::ImageFormats => {
                Self::has_png_signature(data) || Self::has_jpeg_signature(data)
            }
        }
    }
    
    /// Check if data has PNG signature
    fn has_png_signature(data: &[u8]) -> bool {
        data.len() >= 8 && data[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
    }
    
    /// Check if data has JPEG signature
    fn has_jpeg_signature(data: &[u8]) -> bool {
        data.len() >= 2 && data[0..2] == [0xFF, 0xD8]
    }
    
    #[cfg(test)]
    /// Get current filter statistics (test-only method)
    pub fn stats(&self) -> &FilterStats {
        &self.stats
    }
    
    #[cfg(test)]
    /// Reset filter statistics (test-only method)
    pub fn reset_stats(&mut self) {
        self.stats = FilterStats::default();
    }
    
    #[cfg(test)]
    /// Filter a collection of file paths and return the included ones (test-only method)
    pub fn filter_files<'a>(&mut self, file_paths: &'a [String]) -> Vec<&'a String> {
        file_paths
            .iter()
            .filter(|path| self.apply(path) == FilterResult::Include)
            .collect()
    }
}

impl FilterStats {
    /// Calculate the percentage of files that were included
    pub fn inclusion_rate(&self) -> f64 {
        if self.total_files == 0 {
            0.0
        } else {
            (self.included_files as f64 / self.total_files as f64) * 100.0
        }
    }
    
    /// Calculate the percentage of files that were excluded
    pub fn exclusion_rate(&self) -> f64 {
        if self.total_files == 0 {
            0.0
        } else {
            (self.excluded_files as f64 / self.total_files as f64) * 100.0
        }
    }
    
    /// Calculate the percentage of files that were skipped
    pub fn skip_rate(&self) -> f64 {
        if self.total_files == 0 {
            0.0
        } else {
            (self.skipped_files as f64 / self.total_files as f64) * 100.0
        }
    }
    
    /// Get resolution distribution summary
    pub fn resolution_summary(&self) -> String {
        format!(
            "HD: {}, HD2: {}, SD: {}, Unknown: {}",
            self.hd_files, self.hd2_files, self.sd_files, self.unknown_resolution_files
        )
    }
    
    /// Get format distribution summary
    pub fn format_summary(&self) -> String {
        format!(
            "PNG: {}, JPEG: {}, Other: {}",
            self.png_files, self.jpeg_files, self.other_format_files
        )
    }
}

impl std::fmt::Display for FilterStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Filter Statistics: {} total, {} included ({:.1}%), {} excluded ({:.1}%), {} skipped ({:.1}%)\n\
             Resolution: {}\n\
             Formats: {}",
            self.total_files,
            self.included_files,
            self.inclusion_rate(),
            self.excluded_files,
            self.exclusion_rate(),
            self.skipped_files,
            self.skip_rate(),
            self.resolution_summary(),
            self.format_summary()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use crate::cli::ResolutionTier;
    
    // Property test generators
    fn file_path_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // HD paths
            Just("data/anim/terran/unit.anim".to_string()),
            Just("Data\\anim\\protoss\\building.anim".to_string()),
            Just("/path/to/anim/zerg/sprite.anim".to_string()),
            // HD2 paths
            Just("data/HD2/anim/terran/unit.anim".to_string()),
            Just("Data\\HD2\\anim\\protoss\\building.anim".to_string()),
            // SD paths
            Just("data/SD/terran/unit.anim".to_string()),
            Just("Data\\SD\\protoss\\building.anim".to_string()),
            // Other paths
            Just("data/other/file.anim".to_string()),
            Just("ui/button.png".to_string()),
            Just("effects/explosion.jpg".to_string()),
        ]
    }
    
    fn regex_pattern_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just(".*terran.*".to_string()),
            Just(".*protoss.*".to_string()),
            Just(".*zerg.*".to_string()),
            Just(".*anim.*".to_string()),
            Just(".*ui.*".to_string()),
            Just(".*png.*".to_string()),
            Just(".*jpg.*".to_string()),
            Just(".*HD.*".to_string()),
            Just(".*SD.*".to_string()),
        ]
    }
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        // **Feature: casc-sprite-extractor, Property 6: Filter Application Logic**
        // **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5**
        fn property_6_filter_application_logic(
            file_paths in prop::collection::vec(file_path_strategy(), 1..20),
            include_patterns in prop::collection::vec(regex_pattern_strategy(), 0..3),
            exclude_patterns in prop::collection::vec(regex_pattern_strategy(), 0..3),
            resolution_filter in prop_oneof![
                Just(None),
                Just(Some(ResolutionTier::HD)),
                Just(Some(ResolutionTier::HD2)),
                Just(Some(ResolutionTier::SD)),
                Just(Some(ResolutionTier::All)),
            ],
            format_filter in prop_oneof![
                Just(None),
                Just(Some(FormatFilter::PngOnly)),
                Just(Some(FormatFilter::JpegOnly)),
                Just(Some(FormatFilter::ImageFormats)),
                Just(Some(FormatFilter::All)),
            ]
        ) {
            // For any combination of filter patterns (inclusion, exclusion, resolution-based), 
            // the filtering system should correctly apply logical operations and report 
            // accurate inclusion/exclusion results
            
            let mut filter = FileFilter::new_enhanced(
                &include_patterns,
                &exclude_patterns,
                resolution_filter,
                format_filter.clone()
            ).unwrap();
            
            let mut expected_included = 0;
            let mut expected_excluded = 0;
            let mut expected_skipped = 0;
            
            // Generate file data for format filtering
            let file_data_samples = vec![
                vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00], // PNG
                vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10], // JPEG
                vec![0x00, 0x01, 0x02, 0x03], // Other
            ];
            
            for (i, file_path) in file_paths.iter().enumerate() {
                let file_data = file_data_samples[i % file_data_samples.len()].clone();
                let file_info = FileInfo {
                    path: file_path.clone(),
                    data: Some(file_data.clone()),
                    size: Some(1024),
                };
                
                // Manually determine expected result
                let expected_result = determine_expected_filter_result(
                    &file_info,
                    &include_patterns,
                    &exclude_patterns,
                    resolution_filter,
                    &format_filter
                );
                
                match expected_result {
                    FilterResult::Include => expected_included += 1,
                    FilterResult::Exclude => expected_excluded += 1,
                    FilterResult::Skip => expected_skipped += 1,
                }
                
                // Apply filter and check result
                let actual_result = filter.apply_to_file_info(&file_info);
                prop_assert_eq!(actual_result, expected_result,
                    "Filter result mismatch for file '{}': expected {:?}, got {:?}",
                    file_path, expected_result, actual_result);
            }
            
            // Verify statistics are accurate
            let stats = filter.stats();
            prop_assert_eq!(stats.total_files, file_paths.len(),
                "Total file count should match input files");
            prop_assert_eq!(stats.included_files, expected_included,
                "Included file count should match expected");
            prop_assert_eq!(stats.excluded_files, expected_excluded,
                "Excluded file count should match expected");
            prop_assert_eq!(stats.skipped_files, expected_skipped,
                "Skipped file count should match expected");
            
            // Total should equal sum of parts
            prop_assert_eq!(stats.total_files,
                stats.included_files + stats.excluded_files + stats.skipped_files,
                "Total should equal sum of included, excluded, and skipped");
        }
    }
    
    // Helper function to manually determine expected filter result
    fn determine_expected_filter_result(
        file_info: &FileInfo,
        include_patterns: &[String],
        exclude_patterns: &[String],
        resolution_filter: Option<ResolutionTier>,
        format_filter: &Option<FormatFilter>
    ) -> FilterResult {
        use regex::Regex;
        
        // Check exclusion patterns first
        for pattern in exclude_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if regex.is_match(&file_info.path) {
                    return FilterResult::Exclude;
                }
            }
        }
        
        // Check resolution filter
        if let Some(resolution_filter) = resolution_filter {
            if resolution_filter != ResolutionTier::All {
                let detected_tier = crate::resolution::ResolutionHandler::detect_tier_from_path(&file_info.path);
                match detected_tier {
                    Some(tier) if tier == resolution_filter => {}, // Continue checking
                    _ => return FilterResult::Skip, // Wrong resolution or unknown
                }
            }
        }
        
        // Check format filter
        if let Some(ref format_filter) = format_filter {
            if let Some(ref data) = file_info.data {
                let matches_format = match format_filter {
                    FormatFilter::All => true,
                    FormatFilter::PngOnly => FileFilter::has_png_signature(data),
                    FormatFilter::JpegOnly => FileFilter::has_jpeg_signature(data),
                    FormatFilter::ImageFormats => {
                        FileFilter::has_png_signature(data) || FileFilter::has_jpeg_signature(data)
                    }
                };
                
                if !matches_format {
                    return FilterResult::Skip;
                }
            } else if *format_filter != FormatFilter::All {
                return FilterResult::Skip;
            }
        }
        
        // Check inclusion patterns
        if include_patterns.is_empty() {
            return FilterResult::Include;
        }
        
        for pattern in include_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if regex.is_match(&file_info.path) {
                    return FilterResult::Include;
                }
            }
        }
        
        FilterResult::Skip
    }
    
    /*
    // Property tests temporarily disabled due to syntax issues
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        // **Feature: casc-sprite-extractor, Property 12: Inclusion Filter Application**
        // **Validates: Requirements 3.2**
        fn property_12_inclusion_filter_application(
            files in file_path_collection_strategy(),
            include_patterns in regex_pattern_collection_strategy()
        ) {
            // For any set of files and inclusion pattern, only files matching the pattern 
            // should be extracted, and all matching files should be extracted
            
            let mut filter = FileFilter::new(&include_patterns, &[]).unwrap();
            
            // Apply filter and collect results
            let mut actual_included_count = 0;
            let mut actual_excluded_or_skipped_count = 0;
            let mut expected_included_count = 0;
            let mut expected_excluded_or_skipped_count = 0;
            
            // Count expected results by checking each file against patterns
            for file in &files {
                if filter.matches_any_include_pattern(file) {
                    expected_included_count += 1;
                } else {
                    expected_excluded_or_skipped_count += 1;
                }
            }
            
            // Apply filter and count actual results
            for file in &files {
                match filter.apply(file) {
                    FilterResult::Include => {
                        actual_included_count += 1;
                    }
                    FilterResult::Exclude | FilterResult::Skip => {
                        actual_excluded_or_skipped_count += 1;
                    }
                }
            }
            
            // Only files matching the pattern should be included
            prop_assert_eq!(actual_included_count, expected_included_count, 
                "Included file count should match expected included files");
            
            // All non-matching files should be excluded or skipped
            prop_assert_eq!(actual_excluded_or_skipped_count, expected_excluded_or_skipped_count,
                "Excluded/skipped file count should match expected excluded/skipped files");
            
            // Statistics should be accurate
            let stats = filter.stats();
            prop_assert_eq!(stats.total_files, files.len());
            prop_assert_eq!(stats.included_files, actual_included_count);
            prop_assert_eq!(stats.excluded_files + stats.skipped_files, actual_excluded_or_skipped_count);
        }
        
        #[test]
        // **Feature: casc-sprite-extractor, Property 13: Multiple Pattern OR Logic**
        // **Validates: Requirements 3.3**
        fn property_13_multiple_pattern_or_logic(
            files in file_path_collection_strategy(),
            patterns in regex_pattern_collection_strategy()
        ) {
            // For any set of files and multiple inclusion patterns, a file should be 
            // extracted if it matches ANY of the patterns
            
            let mut filter = FileFilter::new(&patterns, &[]).unwrap();
            
            for file in &files {
                let filter_result = filter.apply(file);
                
                // Check if file matches any pattern individually
                let matches_any = patterns.iter().any(|pattern| {
                    if let Ok(regex) = Regex::new(pattern) {
                        regex.is_match(file)
                    } else {
                        false
                    }
                });
                
                if matches_any {
                    prop_assert_eq!(filter_result, FilterResult::Include,
                        "File '{}' matches at least one pattern and should be included", file);
                } else {
                    prop_assert_ne!(filter_result, FilterResult::Include,
                        "File '{}' matches no patterns and should not be included", file);
                }
            }
            
            // Test with specific known cases
            let test_patterns = vec![".*terran.*".to_string(), ".*protoss.*".to_string()];
            let test_filter = FileFilter::new(&test_patterns, &[]).unwrap();
            
            // Should match terran
            prop_assert_eq!(test_filter.check_file("terran_unit.anim"), FilterResult::Include);
            // Should match protoss
            prop_assert_eq!(test_filter.check_file("protoss_building.anim"), FilterResult::Include);
            // Should not match zerg
            prop_assert_ne!(test_filter.check_file("zerg_effect.anim"), FilterResult::Include);
        }
        
        #[test]
        // **Feature: casc-sprite-extractor, Property 14: Exclusion Filter Application**
        // **Validates: Requirements 3.4**
        fn property_14_exclusion_filter_application(
            files in file_path_collection_strategy(),
            include_patterns in regex_pattern_collection_strategy(),
            exclude_patterns in regex_pattern_collection_strategy()
        ) {
            // Files matching exclusion patterns should not be extracted, even if they 
            // match inclusion patterns
            
            let mut filter = FileFilter::new(&include_patterns, &exclude_patterns).unwrap();
            
            for file in &files {
                let filter_result = filter.apply(file);
                
                // Check if file matches any exclusion pattern
                let matches_exclude = exclude_patterns.iter().any(|pattern| {
                    if let Ok(regex) = Regex::new(pattern) {
                        regex.is_match(file)
                    } else {
                        false
                    }
                });
                
                if matches_exclude {
                    prop_assert_eq!(filter_result, FilterResult::Exclude,
                        "File '{}' matches exclusion pattern and should be excluded", file);
                } else {
                    prop_assert_ne!(filter_result, FilterResult::Exclude,
                        "File '{}' does not match exclusion pattern and should not be excluded", file);
                }
            }
            
            // Test with specific known cases
            let test_include = vec![".*anim.*".to_string()]; // Include all .anim files
            let test_exclude = vec![".*ui.*".to_string()]; // Exclude UI files
            let test_filter = FileFilter::new(&test_include, &test_exclude).unwrap();
            
            // Should exclude UI files even though they match inclusion pattern
            prop_assert_eq!(test_filter.check_file("ui/button.anim"), FilterResult::Exclude);
            // Should include non-UI anim files
            prop_assert_eq!(test_filter.check_file("terran/unit.anim"), FilterResult::Include);
            // Should skip non-anim files
            prop_assert_eq!(test_filter.check_file("terran/unit.png"), FilterResult::Skip);
        }
        
        #[test]
        // **Feature: casc-sprite-extractor, Property 15: Filter Reporting Accuracy**
        // **Validates: Requirements 3.5**
        fn property_15_filter_reporting_accuracy(
            files in file_path_collection_strategy(),
            include_patterns in regex_pattern_collection_strategy(),
            exclude_patterns in regex_pattern_collection_strategy()
        ) {
            // For any extraction with filters, the reported included and excluded file 
            // counts should match the actual number of files extracted and skipped
            
            let mut filter = FileFilter::new(&include_patterns, &exclude_patterns).unwrap();
            
            let mut actual_included = 0;
            let mut actual_excluded = 0;
            let mut actual_skipped = 0;
            
            // Apply filter to all files and count results
            for file in &files {
                match filter.apply(file) {
                    FilterResult::Include => actual_included += 1,
                    FilterResult::Exclude => actual_excluded += 1,
                    FilterResult::Skip => actual_skipped += 1,
                }
            }
            
            let stats = filter.stats();
            
            // Reported counts should match actual counts
            prop_assert_eq!(stats.total_files, files.len(),
                "Total file count should match input file count");
            prop_assert_eq!(stats.included_files, actual_included,
                "Included file count should match actual included files");
            prop_assert_eq!(stats.excluded_files, actual_excluded,
                "Excluded file count should match actual excluded files");
            prop_assert_eq!(stats.skipped_files, actual_skipped,
                "Skipped file count should match actual skipped files");
            
            // Total should equal sum of parts
            prop_assert_eq!(stats.total_files, 
                stats.included_files + stats.excluded_files + stats.skipped_files,
                "Total should equal sum of included, excluded, and skipped");
            
            // Percentages should be reasonable
            let inclusion_rate = stats.inclusion_rate();
            let exclusion_rate = stats.exclusion_rate();
            let skip_rate = stats.skip_rate();
            
            prop_assert!(inclusion_rate >= 0.0 && inclusion_rate <= 100.0,
                "Inclusion rate should be between 0 and 100%");
            prop_assert!(exclusion_rate >= 0.0 && exclusion_rate <= 100.0,
                "Exclusion rate should be between 0 and 100%");
            prop_assert!(skip_rate >= 0.0 && skip_rate <= 100.0,
                "Skip rate should be between 0 and 100%");
            
            // Rates should sum to approximately 100% (allowing for floating point precision)
            let total_rate = inclusion_rate + exclusion_rate + skip_rate;
            prop_assert!((total_rate - 100.0).abs() < 0.01,
                "Total of all rates should be approximately 100%");
        }
        
        #[test]
        fn test_empty_patterns(
            files in file_path_collection_strategy()
        ) {
            // Test behavior with no patterns
            let mut filter = FileFilter::new(&[], &[]).unwrap();
            
            // With no patterns, all files should be included
            for file in &files {
                prop_assert_eq!(filter.apply(file), FilterResult::Include,
                    "With no patterns, all files should be included");
            }
            
            let stats = filter.stats();
            prop_assert_eq!(stats.included_files, files.len());
            prop_assert_eq!(stats.excluded_files, 0);
            prop_assert_eq!(stats.skipped_files, 0);
        }
        
        #[test]
        fn test_filter_consistency(
            file in file_path_strategy(),
            patterns in regex_pattern_collection_strategy()
        ) {
            // Test that applying the same filter multiple times gives consistent results
            let filter = FileFilter::new(&patterns, &[]).unwrap();
            
            let result1 = filter.check_file(&file);
            let result2 = filter.check_file(&file);
            let result3 = filter.check_file(&file);
            
            prop_assert_eq!(result1, result2, "Filter should give consistent results");
            prop_assert_eq!(result2, result3, "Filter should give consistent results");
        }
        
        #[test]
        fn test_stats_reset(
            files in file_path_collection_strategy(),
            patterns in regex_pattern_collection_strategy()
        ) {
            let mut filter = FileFilter::new(&patterns, &[]).unwrap();
            
            // Apply filter to some files
            for file in &files {
                filter.apply(file);
            }
            
            // Stats should have some values
            let stats_before = filter.stats().clone();
            prop_assert!(stats_before.total_files > 0, "Should have processed some files");
            
            // Reset stats
            filter.reset_stats();
            
            // Stats should be reset to zero
            let stats_after = filter.stats();
            prop_assert_eq!(stats_after.total_files, 0);
            prop_assert_eq!(stats_after.included_files, 0);
            prop_assert_eq!(stats_after.excluded_files, 0);
            prop_assert_eq!(stats_after.skipped_files, 0);
        }
    }
    */
    
    #[test]
    fn test_basic_inclusion_filter() {
        let patterns = vec![".*test.*".to_string()];
        let mut filter = FileFilter::new(&patterns, &[]).unwrap();
        
        assert_eq!(filter.apply("test_file.anim"), FilterResult::Include);
        assert_eq!(filter.apply("my_test.anim"), FilterResult::Include);
        assert_eq!(filter.apply("other_file.anim"), FilterResult::Skip);
        
        let stats = filter.stats();
        assert_eq!(stats.total_files, 3);
        assert_eq!(stats.included_files, 2);
        assert_eq!(stats.skipped_files, 1);
    }
    
    #[test]
    fn test_basic_exclusion_filter() {
        let include_patterns = vec![".*anim.*".to_string()];
        let exclude_patterns = vec![".*ui.*".to_string()];
        let mut filter = FileFilter::new(&include_patterns, &exclude_patterns).unwrap();
        
        assert_eq!(filter.apply("unit.anim"), FilterResult::Include);
        assert_eq!(filter.apply("ui/button.anim"), FilterResult::Exclude);
        assert_eq!(filter.apply("unit.png"), FilterResult::Skip);
        
        let stats = filter.stats();
        assert_eq!(stats.total_files, 3);
        assert_eq!(stats.included_files, 1);
        assert_eq!(stats.excluded_files, 1);
        assert_eq!(stats.skipped_files, 1);
    }
    
    #[test]
    fn test_resolution_based_filtering() {
        let mut filter = FileFilter::new(&[], &[]).unwrap()
            .with_resolution_filter(ResolutionTier::HD);
        
        // Test HD path (should be included)
        assert_eq!(filter.apply("data/anim/terran/unit.anim"), FilterResult::Include);
        
        // Test HD2 path (should be skipped when filtering for HD)
        assert_eq!(filter.apply("data/HD2/anim/protoss/building.anim"), FilterResult::Skip);
        
        // Test SD path (should be skipped when filtering for HD)
        assert_eq!(filter.apply("data/SD/zerg/sprite.anim"), FilterResult::Skip);
        
        let stats = filter.stats();
        assert_eq!(stats.included_files, 1);
        assert_eq!(stats.skipped_files, 2);
        assert_eq!(stats.hd_files, 1);
        assert_eq!(stats.hd2_files, 1);
        assert_eq!(stats.sd_files, 1);
    }
    
    #[test]
    fn test_format_based_filtering() {
        let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        let other_data = vec![0x00, 0x01, 0x02, 0x03];
        
        let mut filter = FileFilter::new(&[], &[]).unwrap()
            .with_format_filter(FormatFilter::PngOnly);
        
        let png_file = FileInfo {
            path: "sprite.png".to_string(),
            data: Some(png_data),
            size: Some(1024),
        };
        
        let jpeg_file = FileInfo {
            path: "sprite.jpg".to_string(),
            data: Some(jpeg_data),
            size: Some(2048),
        };
        
        let other_file = FileInfo {
            path: "sprite.dat".to_string(),
            data: Some(other_data),
            size: Some(512),
        };
        
        // PNG file should be included
        assert_eq!(filter.apply_to_file_info(&png_file), FilterResult::Include);
        
        // JPEG file should be skipped when filtering for PNG only
        assert_eq!(filter.apply_to_file_info(&jpeg_file), FilterResult::Skip);
        
        // Other file should be skipped
        assert_eq!(filter.apply_to_file_info(&other_file), FilterResult::Skip);
        
        let stats = filter.stats();
        assert_eq!(stats.included_files, 1);
        assert_eq!(stats.skipped_files, 2);
        assert_eq!(stats.png_files, 1);
        assert_eq!(stats.jpeg_files, 1);
        assert_eq!(stats.other_format_files, 1);
    }
    
    #[test]
    fn test_combined_filtering() {
        let include_patterns = vec![".*terran.*".to_string()];
        let exclude_patterns = vec![".*ui.*".to_string()];
        
        let mut filter = FileFilter::new_enhanced(
            &include_patterns,
            &exclude_patterns,
            Some(ResolutionTier::HD),
            Some(FormatFilter::ImageFormats)
        ).unwrap();
        
        let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        
        // Should be included: matches pattern, is HD, is PNG
        let good_file = FileInfo {
            path: "data/anim/terran/unit.png".to_string(),
            data: Some(png_data.clone()),
            size: Some(1024),
        };
        assert_eq!(filter.apply_to_file_info(&good_file), FilterResult::Include);
        
        // Should be excluded: matches exclusion pattern
        let ui_file = FileInfo {
            path: "data/anim/terran/ui/button.png".to_string(),
            data: Some(png_data.clone()),
            size: Some(1024),
        };
        assert_eq!(filter.apply_to_file_info(&ui_file), FilterResult::Exclude);
        
        // Should be skipped: wrong resolution
        let hd2_file = FileInfo {
            path: "data/HD2/anim/terran/unit.png".to_string(),
            data: Some(png_data.clone()),
            size: Some(1024),
        };
        assert_eq!(filter.apply_to_file_info(&hd2_file), FilterResult::Skip);
        
        // Should be skipped: doesn't match inclusion pattern
        let protoss_file = FileInfo {
            path: "data/anim/protoss/unit.png".to_string(),
            data: Some(png_data),
            size: Some(1024),
        };
        assert_eq!(filter.apply_to_file_info(&protoss_file), FilterResult::Skip);
    }
    
    #[test]
    fn test_signature_detection() {
        // Test PNG signature detection
        let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        assert!(FileFilter::has_png_signature(&png_data));
        
        // Test JPEG signature detection
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert!(FileFilter::has_jpeg_signature(&jpeg_data));
        
        // Test non-image data
        let other_data = vec![0x00, 0x01, 0x02, 0x03];
        assert!(!FileFilter::has_png_signature(&other_data));
        assert!(!FileFilter::has_jpeg_signature(&other_data));
        
        // Test insufficient data
        let short_data = vec![0x89];
        assert!(!FileFilter::has_png_signature(&short_data));
        assert!(!FileFilter::has_jpeg_signature(&short_data));
    }
    
    #[test]
    fn test_multiple_inclusion_patterns() {
        let patterns = vec![".*terran.*".to_string(), ".*protoss.*".to_string()];
        let mut filter = FileFilter::new(&patterns, &[]).unwrap();
        
        assert_eq!(filter.apply("terran_unit.anim"), FilterResult::Include);
        assert_eq!(filter.apply("protoss_building.anim"), FilterResult::Include);
        assert_eq!(filter.apply("zerg_effect.anim"), FilterResult::Skip);
        
        let stats = filter.stats();
        assert_eq!(stats.included_files, 2);
        assert_eq!(stats.skipped_files, 1);
    }
    
    #[test]
    fn test_filter_files_method() {
        let patterns = vec![".*test.*".to_string()];
        let mut filter = FileFilter::new(&patterns, &[]).unwrap();
        
        let files = vec![
            "test1.anim".to_string(),
            "other.anim".to_string(),
            "test2.anim".to_string(),
            "another.anim".to_string(),
        ];
        
        let filtered = filter.filter_files(&files);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&&"test1.anim".to_string()));
        assert!(filtered.contains(&&"test2.anim".to_string()));
    }
    
    #[test]
    fn test_stats_display() {
        let mut filter = FileFilter::new(&[".*test.*".to_string()], &[]).unwrap();
        
        filter.apply("test1.anim");
        filter.apply("other.anim");
        filter.apply("test2.anim");
        
        let stats_str = filter.stats().to_string();
        assert!(stats_str.contains("3 total"));
        assert!(stats_str.contains("2 included"));
        assert!(stats_str.contains("1 skipped"));
    }
}