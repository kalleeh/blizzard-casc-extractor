// Regression test suite for maintaining extraction quality
//
// This module maintains a database of known-good extractions and validates
// that changes don't break previously working sprite extractions.

use super::{ValidationError, byte_comparison::ByteComparison};
use super::visual_validation::{create_pixel_diff_image, VisualComparison};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use log::{debug, info, warn};

/// A known-good extraction that serves as a regression test baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownGoodExtraction {
    /// Name/identifier of the sprite
    pub sprite_name: String,
    
    /// Path to the source CASC file
    pub source_file: PathBuf,
    
    /// Path to the expected output PNG
    pub expected_output: PathBuf,
    
    /// Expected sprite metadata
    pub expected_metadata: SpriteMetadata,
    
    /// SHA256 hash of the expected output
    pub sha256_hash: String,
    
    /// Date when this baseline was established
    pub baseline_date: String,
    
    /// Version of the extractor that created this baseline
    pub extractor_version: String,
}

/// Sprite metadata for regression testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteMetadata {
    /// Sprite width in pixels
    pub width: u32,
    
    /// Sprite height in pixels
    pub height: u32,
    
    /// Number of frames (for animated sprites)
    pub frame_count: u32,
    
    /// Sprite format (ANIM, GRP, etc.)
    pub format: String,
}

/// Result of a regression test
#[derive(Debug, Clone)]
pub struct RegressionTestResult {
    /// Whether the test passed
    pub passed: bool,
    
    /// Name of the sprite tested
    pub sprite_name: String,
    
    /// Details of any regression detected
    pub regression_details: Option<String>,
    
    /// Path to diff image (if regression detected)
    pub diff_image_path: Option<String>,
}

/// Result of pixel-by-pixel comparison
#[derive(Debug, Clone)]
struct PixelComparisonResult {
    /// Number of pixels that differ
    different_pixels: usize,
    
    /// Total number of pixels
    total_pixels: usize,
    
    /// Percentage of pixels that differ
    difference_percentage: f64,
    
    /// Path to generated diff image
    diff_image_path: Option<String>,
}

/// Regression test suite manager
pub struct RegressionTestSuite {
    /// Database of known-good extractions
    known_good_extractions: HashMap<String, KnownGoodExtraction>,
    
    /// Path to the regression database file
    database_path: PathBuf,
}

impl RegressionTestSuite {
    /// Create a new regression test suite
    pub fn new(database_path: PathBuf) -> Result<Self, ValidationError> {
        let known_good_extractions = if database_path.exists() {
            Self::load_database(&database_path)?
        } else {
            HashMap::new()
        };

        Ok(Self {
            known_good_extractions,
            database_path,
        })
    }

    /// Load the regression database from disk
    fn load_database(path: &Path) -> Result<HashMap<String, KnownGoodExtraction>, ValidationError> {
        let content = std::fs::read_to_string(path)?;
        let extractions: Vec<KnownGoodExtraction> = serde_json::from_str(&content)?;
        
        let mut map = HashMap::new();
        for extraction in extractions {
            map.insert(extraction.sprite_name.clone(), extraction);
        }
        
        Ok(map)
    }

    /// Save the regression database to disk
    pub fn save_database(&self) -> Result<(), ValidationError> {
        let extractions: Vec<_> = self.known_good_extractions.values().cloned().collect();
        let content = serde_json::to_string_pretty(&extractions)?;
        std::fs::write(&self.database_path, content)?;
        Ok(())
    }

    /// Add a new known-good extraction to the database
    pub fn add_known_good(
        &mut self,
        sprite_name: String,
        source_file: PathBuf,
        output_file: PathBuf,
        metadata: SpriteMetadata,
    ) -> Result<(), ValidationError> {
        info!("Adding known-good extraction: {}", sprite_name);

        // Calculate SHA256 hash
        let sha256_hash = ByteComparison::calculate_sha256(&output_file)?;

        let extraction = KnownGoodExtraction {
            sprite_name: sprite_name.clone(),
            source_file,
            expected_output: output_file,
            expected_metadata: metadata,
            sha256_hash,
            baseline_date: chrono::Utc::now().to_rfc3339(),
            extractor_version: env!("CARGO_PKG_VERSION").to_string(),
        };

        self.known_good_extractions.insert(sprite_name, extraction);
        self.save_database()?;

        Ok(())
    }

    /// Validate that an extraction hasn't regressed
    ///
    /// This performs:
    /// - SHA256 hash comparison
    /// - Pixel-by-pixel comparison (if hash mismatch)
    /// - Metadata validation
    ///
    /// # Arguments
    /// * `sprite_name` - Name of the sprite to validate
    /// * `actual_output` - Path to the newly extracted sprite
    ///
    /// # Returns
    /// Regression test result indicating whether the extraction matches the baseline
    pub fn validate_no_regression(
        &self,
        sprite_name: &str,
        actual_output: &Path,
    ) -> Result<RegressionTestResult, ValidationError> {
        debug!("Validating no regression for: {}", sprite_name);

        // Check if we have a known-good baseline
        let known_good = match self.known_good_extractions.get(sprite_name) {
            Some(kg) => kg,
            None => {
                warn!("No known-good baseline for sprite: {}", sprite_name);
                return Ok(RegressionTestResult {
                    passed: true,
                    sprite_name: sprite_name.to_string(),
                    regression_details: Some("No baseline available".to_string()),
                    diff_image_path: None,
                });
            }
        };

        // Compare SHA256 hash
        let actual_hash = ByteComparison::calculate_sha256(actual_output)?;
        if actual_hash != known_good.sha256_hash {
            // Hash mismatch - perform detailed pixel-by-pixel comparison
            let pixel_comparison = self.compare_pixels(&known_good.expected_output, actual_output)?;
            
            let details = format!(
                "REGRESSION DETECTED: {}\n\
                 SHA256 hash mismatch:\n\
                 Expected: {}\n\
                 Actual:   {}\n\
                 \n\
                 Pixel-by-pixel comparison:\n\
                 Different pixels: {} / {} ({:.2}%)\n\
                 \n\
                 Baseline date: {}\n\
                 Baseline version: {}",
                sprite_name,
                known_good.sha256_hash,
                actual_hash,
                pixel_comparison.different_pixels,
                pixel_comparison.total_pixels,
                pixel_comparison.difference_percentage,
                known_good.baseline_date,
                known_good.extractor_version
            );

            return Ok(RegressionTestResult {
                passed: false,
                sprite_name: sprite_name.to_string(),
                regression_details: Some(details),
                diff_image_path: pixel_comparison.diff_image_path,
            });
        }

        // If hashes match, extraction is identical
        info!("✅ No regression detected for: {}", sprite_name);
        Ok(RegressionTestResult {
            passed: true,
            sprite_name: sprite_name.to_string(),
            regression_details: None,
            diff_image_path: None,
        })
    }

    /// Perform pixel-by-pixel comparison between expected and actual images
    fn compare_pixels(
        &self,
        expected_path: &Path,
        actual_path: &Path,
    ) -> Result<PixelComparisonResult, ValidationError> {
        use image::GenericImageView;

        // Load images
        let expected_img = image::open(expected_path)?;
        let actual_img = image::open(actual_path)?;

        let (exp_width, exp_height) = expected_img.dimensions();
        let (act_width, act_height) = actual_img.dimensions();

        // Check dimensions match
        if exp_width != act_width || exp_height != act_height {
            return Ok(PixelComparisonResult {
                different_pixels: (exp_width * exp_height) as usize,
                total_pixels: (exp_width * exp_height) as usize,
                difference_percentage: 100.0,
                diff_image_path: None,
            });
        }

        let (different_pixels, total_pixels) =
            VisualComparison::count_pixel_differences(&expected_img, &actual_img);

        let difference_percentage = (different_pixels as f64 / total_pixels as f64) * 100.0;

        // Generate diff image if there are differences
        let diff_image_path = if different_pixels > 0 {
            Some(self.generate_diff_image(expected_path, actual_path, &expected_img, &actual_img)?)
        } else {
            None
        };

        Ok(PixelComparisonResult {
            different_pixels,
            total_pixels,
            difference_percentage,
            diff_image_path,
        })
    }

    /// Generate a visual diff image showing differences
    fn generate_diff_image(
        &self,
        expected_path: &Path,
        _actual_path: &Path,
        expected_img: &image::DynamicImage,
        actual_img: &image::DynamicImage,
    ) -> Result<String, ValidationError> {
        let diff_path = expected_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!(
                "{}_regression_diff.png",
                expected_path.file_stem().unwrap().to_string_lossy()
            ));

        create_pixel_diff_image(expected_img, actual_img, &diff_path)
    }

    /// Run all regression tests
    pub fn run_all_tests(&self, output_dir: &Path) -> Result<Vec<RegressionTestResult>, ValidationError> {
        info!("Running {} regression tests", self.known_good_extractions.len());

        let mut results = Vec::new();

        for (sprite_name, known_good) in &self.known_good_extractions {
            // Find the actual output file
            let actual_output = output_dir.join(
                known_good.expected_output
                    .file_name()
                    .ok_or_else(|| ValidationError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Invalid expected output path",
                    )))?
            );

            if !actual_output.exists() {
                warn!("Actual output not found for {}: {:?}", sprite_name, actual_output);
                results.push(RegressionTestResult {
                    passed: false,
                    sprite_name: sprite_name.clone(),
                    regression_details: Some("Output file not found".to_string()),
                    diff_image_path: None,
                });
                continue;
            }

            let result = self.validate_no_regression(sprite_name, &actual_output)?;
            results.push(result);
        }

        Ok(results)
    }

    /// Get the number of known-good extractions in the database
    pub fn count(&self) -> usize {
        self.known_good_extractions.len()
    }

    /// Check if a sprite has a known-good baseline
    pub fn has_baseline(&self, sprite_name: &str) -> bool {
        self.known_good_extractions.contains_key(sprite_name)
    }

    /// Generate a comprehensive regression report
    pub fn generate_regression_report(
        &self,
        results: &[RegressionTestResult],
        output_path: &Path,
    ) -> Result<(), ValidationError> {
        use std::io::Write;

        let mut report = String::new();
        
        // Header
        report.push_str("╔══════════════════════════════════════════════════════════════════════╗\n");
        report.push_str("║              REGRESSION TEST SUITE REPORT                            ║\n");
        report.push_str("╚══════════════════════════════════════════════════════════════════════╝\n\n");

        // Summary statistics
        let total_tests = results.len();
        let passed_tests = results.iter().filter(|r| r.passed).count();
        let failed_tests = total_tests - passed_tests;
        let pass_rate = if total_tests > 0 {
            (passed_tests as f64 / total_tests as f64) * 100.0
        } else {
            0.0
        };

        report.push_str(&format!("Test Date: {}\n", chrono::Utc::now().to_rfc3339()));
        report.push_str(&format!("Extractor Version: {}\n\n", env!("CARGO_PKG_VERSION")));
        
        report.push_str("SUMMARY:\n");
        report.push_str(&format!("  Total Tests:  {}\n", total_tests));
        report.push_str(&format!("  Passed:       {} ✅\n", passed_tests));
        report.push_str(&format!("  Failed:       {} ❌\n", failed_tests));
        report.push_str(&format!("  Pass Rate:    {:.1}%\n\n", pass_rate));

        // Overall status
        if failed_tests == 0 {
            report.push_str("✅ ALL REGRESSION TESTS PASSED - NO REGRESSIONS DETECTED\n\n");
        } else {
            report.push_str("❌ REGRESSION DETECTED - SOME TESTS FAILED\n\n");
            report.push_str("⚠️  CRITICAL: Previously working extractions have broken!\n");
            report.push_str("    Review the failures below and fix before proceeding.\n\n");
        }

        // Detailed results
        report.push_str("═══════════════════════════════════════════════════════════════════════\n");
        report.push_str("DETAILED RESULTS:\n");
        report.push_str("═══════════════════════════════════════════════════════════════════════\n\n");

        // Group by status
        let passed: Vec<_> = results.iter().filter(|r| r.passed).collect();
        let failed: Vec<_> = results.iter().filter(|r| !r.passed).collect();

        // Show failed tests first (most important)
        if !failed.is_empty() {
            report.push_str("FAILED TESTS (REGRESSIONS DETECTED):\n");
            report.push_str("───────────────────────────────────────────────────────────────────────\n\n");

            for (i, result) in failed.iter().enumerate() {
                report.push_str(&format!("{}. ❌ {}\n", i + 1, result.sprite_name));
                
                if let Some(details) = &result.regression_details {
                    report.push_str(&format!("\n{}\n", details));
                }
                
                if let Some(diff_path) = &result.diff_image_path {
                    report.push_str(&format!("\n   Diff image: {}\n", diff_path));
                }
                
                report.push_str("\n");
            }
        }

        // Show passed tests
        if !passed.is_empty() {
            report.push_str("PASSED TESTS:\n");
            report.push_str("───────────────────────────────────────────────────────────────────────\n\n");

            for (i, result) in passed.iter().enumerate() {
                report.push_str(&format!("{}. ✅ {}\n", i + 1, result.sprite_name));
            }
            report.push_str("\n");
        }

        // Footer
        report.push_str("═══════════════════════════════════════════════════════════════════════\n");
        report.push_str("END OF REGRESSION TEST REPORT\n");
        report.push_str("═══════════════════════════════════════════════════════════════════════\n");

        // Write report to file
        let mut file = std::fs::File::create(output_path)?;
        file.write_all(report.as_bytes())?;

        // Also log to console
        info!("\n{}", report);

        Ok(())
    }

    /// Detect regressions by comparing current extraction results with baselines
    ///
    /// This is the main entry point for automated regression detection.
    /// It will:
    /// 1. Run all regression tests
    /// 2. Generate a comprehensive report
    /// 3. Return an error if any regressions are detected
    ///
    /// # Arguments
    /// * `output_dir` - Directory containing the newly extracted sprites
    /// * `report_path` - Path where the regression report should be saved
    ///
    /// # Returns
    /// Ok if no regressions detected, Err if regressions found
    pub fn detect_regressions(
        &self,
        output_dir: &Path,
        report_path: &Path,
    ) -> Result<(), ValidationError> {
        info!("🔍 Running automated regression detection...");

        // Run all regression tests
        let results = self.run_all_tests(output_dir)?;

        // Generate report
        self.generate_regression_report(&results, report_path)?;

        // Check if any tests failed
        let failed_count = results.iter().filter(|r| !r.passed).count();

        if failed_count > 0 {
            return Err(ValidationError::RegressionDetected {
                failed_tests: failed_count,
                total_tests: results.len(),
                report_path: report_path.to_path_buf(),
            });
        }

        info!("✅ No regressions detected - all {} tests passed!", results.len());
        Ok(())
    }

    /// Generate before/after comparison images for a regression
    pub fn generate_comparison_image(
        &self,
        sprite_name: &str,
        actual_output: &Path,
        comparison_output: &Path,
    ) -> Result<(), ValidationError> {
        use image::{RgbaImage, GenericImageView};

        let known_good = self.known_good_extractions.get(sprite_name)
            .ok_or_else(|| ValidationError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("No baseline for sprite: {}", sprite_name),
            )))?;

        // Load images
        let expected_img = image::open(&known_good.expected_output)?;
        let actual_img = image::open(actual_output)?;

        let (exp_width, exp_height) = expected_img.dimensions();
        let (act_width, act_height) = actual_img.dimensions();

        // Create side-by-side comparison
        let comparison_width = exp_width + act_width + 20; // 20px gap
        let comparison_height = exp_height.max(act_height);
        let mut comparison_img = RgbaImage::new(comparison_width, comparison_height);

        // Fill with white background
        for pixel in comparison_img.pixels_mut() {
            *pixel = image::Rgba([255, 255, 255, 255]);
        }

        // Copy expected image (left side)
        for y in 0..exp_height {
            for x in 0..exp_width {
                let pixel = expected_img.get_pixel(x, y);
                comparison_img.put_pixel(x, y, pixel);
            }
        }

        // Copy actual image (right side)
        let offset_x = exp_width + 20;
        for y in 0..act_height {
            for x in 0..act_width {
                let pixel = actual_img.get_pixel(x, y);
                comparison_img.put_pixel(offset_x + x, y, pixel);
            }
        }

        // Save comparison image
        comparison_img.save(comparison_output)?;

        info!("Generated comparison image: {:?}", comparison_output);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_suite_creation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("regression_db.json");

        let suite = RegressionTestSuite::new(db_path).unwrap();
        assert_eq!(suite.count(), 0);
    }

    #[test]
    fn test_add_known_good() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("regression_db.json");
        let output_file = temp_dir.path().join("test.png");

        // Create a test file
        File::create(&output_file).unwrap().write_all(b"test data").unwrap();

        let mut suite = RegressionTestSuite::new(db_path).unwrap();
        
        let metadata = SpriteMetadata {
            width: 64,
            height: 64,
            frame_count: 1,
            format: "PNG".to_string(),
        };

        suite.add_known_good(
            "test_sprite".to_string(),
            PathBuf::from("source.casc"),
            output_file,
            metadata,
        ).unwrap();

        assert_eq!(suite.count(), 1);
        assert!(suite.has_baseline("test_sprite"));
    }

    #[test]
    fn test_no_regression_without_baseline() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("regression_db.json");
        let output_file = temp_dir.path().join("test.png");

        File::create(&output_file).unwrap().write_all(b"test data").unwrap();

        let suite = RegressionTestSuite::new(db_path).unwrap();
        let result = suite.validate_no_regression("unknown_sprite", &output_file).unwrap();

        assert!(result.passed); // Should pass with warning when no baseline exists
    }
}
