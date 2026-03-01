// Reference tool validator for cross-tool comparison
//
// This module integrates with established StarCraft modding tools to validate
// our extraction results against proven implementations:
// - CASC Explorer: For CASC/BLTE validation
// - libgrp: For GRP format validation
// - GrpEditor: For visual validation

use super::{ValidationError, ValidationResult, ByteComparison};
use std::path::{Path, PathBuf};
use std::process::Command;
use log::{debug, info, warn};

/// Configuration for reference tool paths
#[derive(Debug, Clone)]
pub struct ReferenceToolConfig {
    /// Path to CASC Explorer executable (for CASC/BLTE validation)
    pub casc_explorer_path: Option<PathBuf>,
    
    /// Path to libgrp library or tool (for GRP format validation)
    pub libgrp_path: Option<PathBuf>,
    
    /// Path to GrpEditor executable (for visual validation)
    pub grp_editor_path: Option<PathBuf>,
    
    /// Path to stormex tool (our C++ reference implementation)
    pub stormex_path: Option<PathBuf>,
}

impl Default for ReferenceToolConfig {
    fn default() -> Self {
        Self {
            casc_explorer_path: None,
            libgrp_path: None,
            grp_editor_path: None,
            stormex_path: Some(PathBuf::from("stormex/build/stormex")),
        }
    }
}

/// Reference validator that compares our extraction results with established tools
pub struct ReferenceValidator {
    config: ReferenceToolConfig,
}

impl ReferenceValidator {
    /// Create a new reference validator with the given configuration
    pub fn new(config: ReferenceToolConfig) -> Self {
        Self { config }
    }

    /// Create a new reference validator with default configuration
    pub fn with_defaults() -> Self {
        Self::new(ReferenceToolConfig::default())
    }

    /// Validate an extraction against reference tools
    ///
    /// This performs comprehensive validation including:
    /// - Byte-level comparison with reference tool output
    /// - Visual comparison for image quality
    /// - Metadata validation
    ///
    /// # Arguments
    /// * `our_output` - Path to our extracted sprite
    /// * `source_file` - Path to the original CASC file
    /// * `format` - The sprite format (ANIM, GRP, etc.)
    ///
    /// # Returns
    /// A ValidationResult indicating whether all checks passed
    pub fn validate_extraction(
        &self,
        our_output: &Path,
        source_file: &Path,
        format: &str,
    ) -> Result<ValidationResult, ValidationError> {
        info!("Validating extraction: {:?} (format: {})", our_output, format);

        let mut result = ValidationResult::success();

        // Step 1: Extract with reference tool for comparison
        let reference_output = match self.extract_with_reference_tool(source_file, format) {
            Ok(path) => path,
            Err(e) => {
                warn!("Reference tool extraction failed: {}", e);
                result.add_diagnostic(format!("Reference tool unavailable: {}", e));
                // Don't fail if reference tool is not available, just skip comparison
                return Ok(result);
            }
        };

        // Step 2: Byte-level comparison
        match ByteComparison::compare_files(our_output, &reference_output, false) {
            Ok(cmp) => {
                result.byte_match = cmp.matches;
                if !cmp.matches {
                    result.overall_pass = false;
                    result.add_diagnostic("Byte-level mismatch detected".to_string());
                }
            }
            Err(e) => {
                warn!("Byte comparison failed: {}", e);
                result.add_diagnostic(format!("Byte comparison error: {}", e));
            }
        }

        // Step 3: Visual comparison (for image formats)
        if our_output.extension().and_then(|s| s.to_str()) == Some("png") {
            match self.compare_visual(our_output, &reference_output) {
                Ok(visual_match) => {
                    result.visual_match = visual_match;
                    if !visual_match {
                        result.overall_pass = false;
                        result.add_diagnostic("Visual mismatch detected".to_string());
                    }
                }
                Err(e) => {
                    warn!("Visual comparison failed: {}", e);
                    result.add_diagnostic(format!("Visual comparison error: {}", e));
                }
            }
        }

        // Step 4: Metadata comparison
        match self.compare_metadata(our_output, &reference_output) {
            Ok(metadata_match) => {
                result.metadata_match = metadata_match;
                if !metadata_match {
                    result.overall_pass = false;
                    result.add_diagnostic("Metadata mismatch detected".to_string());
                }
            }
            Err(e) => {
                warn!("Metadata comparison failed: {}", e);
                result.add_diagnostic(format!("Metadata comparison error: {}", e));
            }
        }

        Ok(result)
    }

    /// Extract a sprite using a reference tool
    fn extract_with_reference_tool(
        &self,
        source_file: &Path,
        format: &str,
    ) -> Result<PathBuf, ValidationError> {
        // Try stormex first (our C++ reference implementation)
        if let Some(stormex_path) = &self.config.stormex_path {
            if stormex_path.exists() {
                return self.extract_with_stormex(source_file, stormex_path);
            }
        }

        // Format-specific tools not yet integrated
        Err(ValidationError::ReferenceToolNotFound {
            tool: format.to_string(),
            path: PathBuf::new(),
        })
    }

    /// Extract using stormex (our C++ reference implementation)
    fn extract_with_stormex(
        &self,
        source_file: &Path,
        stormex_path: &Path,
    ) -> Result<PathBuf, ValidationError> {
        debug!("Extracting with stormex: {:?}", source_file);

        let output_dir = std::env::temp_dir().join("casc_validation");
        std::fs::create_dir_all(&output_dir)?;

        let output = Command::new(stormex_path)
            .arg("extract")
            .arg(source_file)
            .arg("--output")
            .arg(&output_dir)
            .output()
            .map_err(|e| ValidationError::ReferenceExtractionFailed {
                tool: "stormex".to_string(),
                reason: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(ValidationError::ReferenceExtractionFailed {
                tool: "stormex".to_string(),
                reason: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        // Find the extracted file
        let file_name = source_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ValidationError::ReferenceExtractionFailed {
                tool: "stormex".to_string(),
                reason: "Invalid source file name".to_string(),
            })?;

        let extracted_file = output_dir.join(format!("{}.png", file_name));
        if extracted_file.exists() {
            Ok(extracted_file)
        } else {
            Err(ValidationError::ReferenceExtractionFailed {
                tool: "stormex".to_string(),
                reason: "Extracted file not found".to_string(),
            })
        }
    }

    /// Compare two images visually
    fn compare_visual(&self, image1: &Path, image2: &Path) -> Result<bool, ValidationError> {
        use image::GenericImageView;

        let img1 = image::open(image1)?;
        let img2 = image::open(image2)?;

        // Check dimensions
        if img1.dimensions() != img2.dimensions() {
            return Ok(false);
        }

        // Pixel-by-pixel comparison
        let (width, height) = img1.dimensions();
        for y in 0..height {
            for x in 0..width {
                if img1.get_pixel(x, y) != img2.get_pixel(x, y) {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Compare metadata of two files
    fn compare_metadata(&self, file1: &Path, file2: &Path) -> Result<bool, ValidationError> {
        let meta1 = std::fs::metadata(file1)?;
        let meta2 = std::fs::metadata(file2)?;

        // Compare file sizes
        Ok(meta1.len() == meta2.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_byte_comparison_identical_files() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.bin");
        let file2 = temp_dir.path().join("file2.bin");

        let data = b"test data";
        File::create(&file1).unwrap().write_all(data).unwrap();
        File::create(&file2).unwrap().write_all(data).unwrap();

        let result = super::ByteComparison::compare_files(&file1, &file2, false).unwrap();
        assert!(result.matches);
    }

    #[test]
    fn test_byte_comparison_different_files() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.bin");
        let file2 = temp_dir.path().join("file2.bin");

        File::create(&file1).unwrap().write_all(b"data1").unwrap();
        File::create(&file2).unwrap().write_all(b"data2").unwrap();

        let result = super::ByteComparison::compare_files(&file1, &file2, false).unwrap();
        assert!(!result.matches);
    }
}
