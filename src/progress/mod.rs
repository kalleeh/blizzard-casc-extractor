use indicatif::ProgressBar;
use std::time::Instant;

#[cfg(test)]
use indicatif::{ProgressStyle, ProgressState};
#[cfg(test)]
use std::time::Duration;
#[cfg(test)]
use std::fmt::Write;

/// Progress reporter for file extraction operations
pub struct ProgressReporter {
    progress_bar: ProgressBar,
    #[allow(dead_code)]
    start_time: Instant,
    #[allow(dead_code)]
    verbose: bool,
    #[allow(dead_code)]
    current_file: Option<String>,
}

impl ProgressReporter {
    #[cfg(test)]
    /// Create a new progress reporter (test-only method)
    /// 
    /// # Arguments
    /// * `total_files` - Total number of files to process
    /// * `verbose` - Whether to show detailed output
    pub fn new(total_files: u64, verbose: bool) -> Self {
        let progress_bar = ProgressBar::new(total_files);
        
        // Set up progress bar style
        let style = if verbose {
            ProgressStyle::with_template(
                "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}"
            )
        } else {
            ProgressStyle::with_template(
                "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} ({eta}) {msg}"
            )
        };
        
        if let Ok(style) = style {
            progress_bar.set_style(style.with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
                write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
            }).progress_chars("##-"));
        }
        
        Self {
            progress_bar,
            start_time: Instant::now(),
            verbose,
            current_file: None,
        }
    }
    
    #[cfg(test)]
    /// Update progress for a file being processed (test-only method)
    /// 
    /// # Arguments
    /// * `file_name` - Name of the current file being processed
    pub fn update_current_file(&mut self, file_name: &str) {
        self.current_file = Some(file_name.to_string());
        
        if self.verbose {
            // In verbose mode, show detailed information
            log::info!("Processing: {}", file_name);
            self.progress_bar.set_message(format!("Processing: {}", file_name));
        } else {
            // In normal mode, show abbreviated file name
            let display_name = if file_name.len() > 40 {
                format!("...{}", &file_name[file_name.len() - 37..])
            } else {
                file_name.to_string()
            };
            self.progress_bar.set_message(display_name);
        }
    }
    
    #[cfg(test)]
    /// Increment progress by one file (test-only method)
    pub fn increment(&mut self) {
        self.progress_bar.inc(1);
    }
    
    #[cfg(test)]
    /// Update progress with current file and increment (test-only method)
    /// 
    /// # Arguments
    /// * `file_name` - Name of the file that was just processed
    pub fn update_and_increment(&mut self, file_name: &str) {
        self.update_current_file(file_name);
        self.increment();
    }
    
    #[cfg(test)]
    /// Finish progress reporting and show completion summary (test-only method)
    /// 
    /// # Arguments
    /// * `extracted_count` - Number of files successfully extracted
    /// * `error_count` - Number of files that failed to extract
    pub fn finish(&self, extracted_count: u64, error_count: u64) {
        let elapsed = self.start_time.elapsed();
        
        self.progress_bar.finish_with_message("Extraction complete!");
        
        // Show completion summary
        log::info!("Extraction completed in {:.2}s", elapsed.as_secs_f64());
        log::info!("Successfully extracted: {} files", extracted_count);
        
        if error_count > 0 {
            log::warn!("Failed to extract: {} files", error_count);
        }
        
        if extracted_count > 0 {
            let files_per_second = extracted_count as f64 / elapsed.as_secs_f64();
            log::info!("Average extraction rate: {:.1} files/second", files_per_second);
        }
    }
    
    #[cfg(test)]
    /// Get the current file being processed (test-only method)
    pub fn current_file(&self) -> Option<&str> {
        self.current_file.as_deref()
    }
    
    #[cfg(test)]
    /// Get elapsed time since progress started (test-only method)
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    #[cfg(test)]
    /// Set a custom message on the progress bar (test-only method)
    pub fn set_message(&self, message: String) {
        self.progress_bar.set_message(message);
    }
    
    #[cfg(test)]
    /// Abandon the progress bar (for error cases) (test-only method)
    pub fn abandon(&self, message: &str) {
        self.progress_bar.abandon_with_message(message.to_string());
    }
}

impl Drop for ProgressReporter {
    fn drop(&mut self) {
        // Ensure progress bar is properly finished if not already done
        if !self.progress_bar.is_finished() {
            self.progress_bar.abandon_with_message("Extraction interrupted".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::thread;
    use std::time::Duration;
    
    // Property test generators
    prop_compose! {
        fn file_name_strategy()(
            name in "[a-zA-Z0-9_.-]{1,100}"
        ) -> String {
            name
        }
    }
    
    prop_compose! {
        fn file_path_strategy()(
            segments in prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5),
            extension in prop_oneof!["anim", "png", "dds", "txt"]
        ) -> String {
            format!("{}.{}", segments.join("/"), extension)
        }
    }
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        // **Feature: casc-sprite-extractor, Property 22: Progress Updates**
        // **Validates: Requirements 6.2**
        fn property_22_progress_updates(
            total_files in 1u64..1000,
            processed_files in prop::collection::vec(file_path_strategy(), 1..100)
        ) {
            let processed_count = processed_files.len().min(total_files as usize);
            let files_to_process = &processed_files[..processed_count];
            
            let mut reporter = ProgressReporter::new(total_files, false);
            
            // For any extraction processing N files, the progress indicator should update N times
            let mut update_count = 0;
            
            for file_name in files_to_process {
                reporter.update_and_increment(file_name);
                update_count += 1;
                
                // Verify that the progress has been updated
                prop_assert!(reporter.progress_bar.position() == update_count as u64);
            }
            
            // The progress indicator should update exactly once per file processed
            prop_assert_eq!(update_count, files_to_process.len());
            prop_assert_eq!(reporter.progress_bar.position(), files_to_process.len() as u64);
        }
        
        #[test]
        // **Feature: casc-sprite-extractor, Property 23: Current File Display**
        // **Validates: Requirements 6.3**
        fn property_23_current_file_display(
            file_name in file_path_strategy()
        ) {
            let mut reporter = ProgressReporter::new(10, true);
            
            // For any file being processed, the current file name should be displayed
            reporter.update_current_file(&file_name);
            
            // Verify that the current file is tracked correctly
            prop_assert_eq!(reporter.current_file(), Some(file_name.as_str()));
            
            // Test that updating with a new file changes the current file
            let new_file = format!("new_{}", file_name);
            reporter.update_current_file(&new_file);
            prop_assert_eq!(reporter.current_file(), Some(new_file.as_str()));
        }
        
        #[test]
        fn test_progress_reporter_creation(
            total_files in 1u64..10000,
            verbose in any::<bool>()
        ) {
            let reporter = ProgressReporter::new(total_files, verbose);
            
            // Verify initial state
            prop_assert_eq!(reporter.progress_bar.length(), Some(total_files));
            prop_assert_eq!(reporter.progress_bar.position(), 0);
            prop_assert_eq!(reporter.current_file(), None);
            prop_assert_eq!(reporter.verbose, verbose);
        }
        
        #[test]
        fn test_file_name_truncation(
            long_file_name in "[a-zA-Z0-9_/.-]{50,200}"
        ) {
            let mut reporter = ProgressReporter::new(1, false);
            
            reporter.update_current_file(&long_file_name);
            
            // In non-verbose mode, long file names should be handled gracefully
            prop_assert_eq!(reporter.current_file(), Some(long_file_name.as_str()));
            
            // The progress bar message should be set (exact format may vary)
            // We just verify that the operation completes without panic
        }
        
        #[test]
        fn test_elapsed_time_tracking(
            total_files in 1u64..100
        ) {
            let reporter = ProgressReporter::new(total_files, false);
            
            // Small delay to ensure elapsed time is measurable
            thread::sleep(Duration::from_millis(1));
            
            let elapsed = reporter.elapsed();
            prop_assert!(elapsed.as_millis() >= 1);
        }
        
        #[test]
        fn test_finish_with_counts(
            extracted_count in 0u64..1000,
            error_count in 0u64..100
        ) {
            let total_files = extracted_count + error_count;
            let reporter = ProgressReporter::new(total_files, false);
            
            // Should complete without panic
            reporter.finish(extracted_count, error_count);
            
            // Verify progress bar is finished
            prop_assert!(reporter.progress_bar.is_finished());
        }
    }
    
    #[test]
    fn test_basic_progress_flow() {
        let mut reporter = ProgressReporter::new(3, false);
        
        // Test basic flow
        reporter.update_current_file("file1.anim");
        assert_eq!(reporter.current_file(), Some("file1.anim"));
        
        reporter.increment();
        assert_eq!(reporter.progress_bar.position(), 1);
        
        reporter.update_and_increment("file2.anim");
        assert_eq!(reporter.current_file(), Some("file2.anim"));
        assert_eq!(reporter.progress_bar.position(), 2);
        
        reporter.finish(2, 0);
        assert!(reporter.progress_bar.is_finished());
    }
    
    #[test]
    fn test_verbose_vs_normal_mode() {
        let mut verbose_reporter = ProgressReporter::new(1, true);
        let mut normal_reporter = ProgressReporter::new(1, false);
        
        verbose_reporter.update_current_file("test_file.anim");
        normal_reporter.update_current_file("test_file.anim");
        
        // Both should track the current file
        assert_eq!(verbose_reporter.current_file(), Some("test_file.anim"));
        assert_eq!(normal_reporter.current_file(), Some("test_file.anim"));
        
        // Verbose flag should be preserved
        assert!(verbose_reporter.verbose);
        assert!(!normal_reporter.verbose);
    }
    
    #[test]
    fn test_abandon_functionality() {
        let reporter = ProgressReporter::new(10, false);
        
        reporter.abandon("Test abandonment");
        
        // Progress bar should be abandoned (not finished normally)
        assert!(reporter.progress_bar.is_finished());
    }
}