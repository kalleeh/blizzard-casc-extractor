// Comprehensive validation error reporting system
//
// This module provides detailed failure diagnostics, hex dumps for byte mismatches,
// side-by-side visual comparisons, and actionable fix recommendations.

use super::{ValidationError, ExtractionValidationReport, BatchValidationReport};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;

/// Error report generator for validation failures
pub struct ErrorReporter {
    report_dir: PathBuf,
}

/// Detailed error diagnostic with actionable recommendations
#[derive(Debug, Clone)]
pub struct ErrorDiagnostic {
    pub error_type: ErrorType,
    pub severity: Severity,
    pub file_name: String,
    pub description: String,
    pub hex_dump: Option<String>,
    pub visual_comparison_path: Option<PathBuf>,
    pub recommendations: Vec<String>,
    pub context: Vec<String>,
}

/// Type of validation error
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    ByteMismatch,
    VisualMismatch,
    MetadataMismatch,
    UnityImportFailure,
    ReferenceToolMismatch,
    FormatParsingError,
    DecompressionError,
}

/// Severity level of the error
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Critical,  // Blocks all functionality
    High,      // Major functionality broken
    Medium,    // Partial functionality affected
    Low,       // Minor issue, workaround available
}

impl ErrorReporter {
    /// Create a new error reporter
    pub fn new(report_dir: PathBuf) -> Result<Self, ValidationError> {
        fs::create_dir_all(&report_dir)?;
        Ok(Self { report_dir })
    }

    /// Generate comprehensive error report for a single extraction failure
    pub fn generate_extraction_error_report(
        &self,
        report: &ExtractionValidationReport,
        extracted_file: &Path,
    ) -> Result<PathBuf, ValidationError> {
        let report_path = self.report_dir.join(format!(
            "{}_error_report.txt",
            report.file_name.replace(".", "_")
        ));

        let mut file = fs::File::create(&report_path)?;
        
        writeln!(file, "{}", "=".repeat(80))?;
        writeln!(file, "VALIDATION ERROR REPORT")?;
        writeln!(file, "{}", "=".repeat(80))?;
        writeln!(file)?;
        writeln!(file, "File: {}", report.file_name)?;
        writeln!(file, "Format: {}", report.format)?;
        writeln!(file, "Overall Status: {}", if report.overall_pass { "PASS" } else { "FAIL" })?;
        writeln!(file)?;

        // Reference validation section
        if let Some(ref_result) = &report.reference_validation {
            writeln!(file, "{}", "-".repeat(80))?;
            writeln!(file, "REFERENCE TOOL VALIDATION")?;
            writeln!(file, "{}", "-".repeat(80))?;
            writeln!(file, "Tool: {}", ref_result.tool_name)?;
            writeln!(file, "Status: {}", if ref_result.passed { "PASS" } else { "FAIL" })?;
            writeln!(file, "Details: {}", ref_result.details)?;
            
            if !ref_result.passed {
                writeln!(file)?;
                writeln!(file, "RECOMMENDATIONS:")?;
                writeln!(file, "  1. Compare extraction output with {} reference tool", ref_result.tool_name)?;
                writeln!(file, "  2. Check format parsing logic for {} format", report.format)?;
                writeln!(file, "  3. Verify decompression algorithm matches reference implementation")?;
            }
            writeln!(file)?;
        }

        // Byte comparison section
        if let Some(byte_result) = &report.byte_comparison {
            writeln!(file, "{}", "-".repeat(80))?;
            writeln!(file, "BYTE-LEVEL COMPARISON")?;
            writeln!(file, "{}", "-".repeat(80))?;
            writeln!(file, "Status: {}", if byte_result.passed { "PASS" } else { "FAIL" })?;
            writeln!(file, "SHA256 Match: {}", byte_result.sha256_match)?;
            writeln!(file, "Byte Differences: {}", byte_result.byte_differences)?;
            
            if let Some(hex_dump_path) = &byte_result.hex_dump_path {
                writeln!(file, "Hex Dump: {:?}", hex_dump_path)?;
                
                // Generate hex dump
                self.generate_hex_dump(extracted_file, hex_dump_path)?;
            }
            
            if !byte_result.passed {
                writeln!(file)?;
                writeln!(file, "RECOMMENDATIONS:")?;
                writeln!(file, "  1. Review hex dump at {:?}", byte_result.hex_dump_path.as_ref().unwrap_or(&PathBuf::from("N/A")))?;
                writeln!(file, "  2. Check for endianness issues in binary parsing")?;
                writeln!(file, "  3. Verify all header fields are correctly parsed")?;
                writeln!(file, "  4. Compare byte-by-byte with reference tool output")?;
            }
            writeln!(file)?;
        }

        // Visual comparison section
        if let Some(visual_result) = &report.visual_comparison {
            writeln!(file, "{}", "-".repeat(80))?;
            writeln!(file, "VISUAL VALIDATION")?;
            writeln!(file, "{}", "-".repeat(80))?;
            writeln!(file, "Status: {}", if visual_result.passed { "PASS" } else { "FAIL" })?;
            writeln!(file, "Pixel Perfect Match: {}", visual_result.pixel_perfect_match)?;
            writeln!(file, "Perceptual Hash Match: {}", visual_result.perceptual_hash_match)?;
            
            if let Some(diff_path) = &visual_result.diff_image_path {
                writeln!(file, "Diff Image: {:?}", diff_path)?;
            }
            
            if !visual_result.passed {
                writeln!(file)?;
                writeln!(file, "RECOMMENDATIONS:")?;
                writeln!(file, "  1. Review visual diff image at {:?}", visual_result.diff_image_path.as_ref().unwrap_or(&PathBuf::from("N/A")))?;
                writeln!(file, "  2. Check color palette conversion logic")?;
                writeln!(file, "  3. Verify transparency handling (alpha channel)")?;
                writeln!(file, "  4. Ensure correct pixel format conversion (RGB/RGBA)")?;
            }
            writeln!(file)?;
        }

        // Unity import section
        if let Some(unity_result) = &report.unity_import {
            writeln!(file, "{}", "-".repeat(80))?;
            writeln!(file, "UNITY IMPORT VALIDATION")?;
            writeln!(file, "{}", "-".repeat(80))?;
            writeln!(file, "Status: {}", if unity_result.passed { "PASS" } else { "FAIL" })?;
            writeln!(file, "Import Successful: {}", unity_result.import_successful)?;
            writeln!(file, "Metadata Correct: {}", unity_result.metadata_correct)?;
            writeln!(file, "Details: {}", unity_result.details)?;
            
            if !unity_result.passed {
                writeln!(file)?;
                writeln!(file, "RECOMMENDATIONS:")?;
                writeln!(file, "  1. Check PNG format compatibility with Unity")?;
                writeln!(file, "  2. Verify sprite dimensions are within Unity limits")?;
                writeln!(file, "  3. Ensure proper alpha channel handling")?;
                writeln!(file, "  4. Check Unity import settings (texture type, compression)")?;
            }
            writeln!(file)?;
        }

        // Diagnostics section
        if !report.diagnostics.is_empty() {
            writeln!(file, "{}", "-".repeat(80))?;
            writeln!(file, "DIAGNOSTIC MESSAGES")?;
            writeln!(file, "{}", "-".repeat(80))?;
            for (i, diagnostic) in report.diagnostics.iter().enumerate() {
                writeln!(file, "{}. {}", i + 1, diagnostic)?;
            }
            writeln!(file)?;
        }

        // Summary and next steps
        writeln!(file, "{}", "=".repeat(80))?;
        writeln!(file, "SUMMARY AND NEXT STEPS")?;
        writeln!(file, "{}", "=".repeat(80))?;
        
        if !report.overall_pass {
            writeln!(file, "This extraction FAILED validation. Priority actions:")?;
            writeln!(file)?;
            
            let mut priority = 1;
            
            if report.reference_validation.as_ref().map_or(false, |r| !r.passed) {
                writeln!(file, "{}. FIX REFERENCE TOOL MISMATCH (CRITICAL)", priority)?;
                writeln!(file, "   - Compare with reference tool output byte-by-byte")?;
                writeln!(file, "   - Study reference tool source code if available")?;
                priority += 1;
            }
            
            if report.byte_comparison.as_ref().map_or(false, |r| !r.passed) {
                writeln!(file, "{}. FIX BYTE-LEVEL DIFFERENCES (HIGH)", priority)?;
                writeln!(file, "   - Review hex dump for specific byte differences")?;
                writeln!(file, "   - Check binary parsing logic")?;
                priority += 1;
            }
            
            if report.visual_comparison.as_ref().map_or(false, |r| !r.passed) {
                writeln!(file, "{}. FIX VISUAL DIFFERENCES (HIGH)", priority)?;
                writeln!(file, "   - Review visual diff image")?;
                writeln!(file, "   - Check color conversion and transparency")?;
                priority += 1;
            }
            
            if report.unity_import.as_ref().map_or(false, |r| !r.passed) {
                writeln!(file, "{}. FIX UNITY IMPORT (MEDIUM)", priority)?;
                writeln!(file, "   - Verify PNG format compatibility")?;
                writeln!(file, "   - Check sprite metadata")?;
            }
        } else {
            writeln!(file, "✅ All validations PASSED. Extraction is correct.")?;
        }
        
        writeln!(file)?;
        writeln!(file, "{}", "=".repeat(80))?;

        Ok(report_path)
    }

    /// Generate comprehensive batch error report
    pub fn generate_batch_error_report(
        &self,
        batch_report: &BatchValidationReport,
    ) -> Result<PathBuf, ValidationError> {
        let report_path = self.report_dir.join("batch_validation_report.txt");
        let mut file = fs::File::create(&report_path)?;
        
        writeln!(file, "{}", "=".repeat(80))?;
        writeln!(file, "BATCH VALIDATION REPORT")?;
        writeln!(file, "{}", "=".repeat(80))?;
        writeln!(file)?;
        writeln!(file, "Total Files: {}", batch_report.total_files)?;
        writeln!(file, "Passed: {}", batch_report.passed_files)?;
        writeln!(file, "Failed: {}", batch_report.failed_files)?;
        writeln!(file, "Success Rate: {:.1}%", 
            (batch_report.passed_files as f64 / batch_report.total_files as f64) * 100.0)?;
        writeln!(file, "Overall Status: {}", if batch_report.overall_pass { "PASS" } else { "FAIL" })?;
        writeln!(file)?;

        // Failed files section
        if batch_report.failed_files > 0 {
            writeln!(file, "{}", "-".repeat(80))?;
            writeln!(file, "FAILED EXTRACTIONS")?;
            writeln!(file, "{}", "-".repeat(80))?;
            
            for report in &batch_report.extraction_reports {
                if !report.overall_pass {
                    writeln!(file, "  • {} ({})", report.file_name, report.format)?;
                    for diagnostic in &report.diagnostics {
                        writeln!(file, "    - {}", diagnostic)?;
                    }
                }
            }
            writeln!(file)?;
        }

        // Regression results section
        if let Some(regression) = &batch_report.regression_results {
            writeln!(file, "{}", "-".repeat(80))?;
            writeln!(file, "REGRESSION TEST RESULTS")?;
            writeln!(file, "{}", "-".repeat(80))?;
            writeln!(file, "Total Tests: {}", regression.total_tests)?;
            writeln!(file, "Passed: {}", regression.passed_tests)?;
            writeln!(file, "Failed: {}", regression.failed_tests)?;
            
            if !regression.regressions_detected.is_empty() {
                writeln!(file)?;
                writeln!(file, "REGRESSIONS DETECTED:")?;
                for regression_file in &regression.regressions_detected {
                    writeln!(file, "  • {}", regression_file)?;
                }
            }
            writeln!(file)?;
        }

        // Summary
        writeln!(file, "{}", "=".repeat(80))?;
        writeln!(file, "SUMMARY")?;
        writeln!(file, "{}", "=".repeat(80))?;
        writeln!(file, "{}", batch_report.summary)?;
        writeln!(file)?;

        if !batch_report.overall_pass {
            writeln!(file, "RECOMMENDED ACTIONS:")?;
            writeln!(file, "  1. Review individual error reports for failed extractions")?;
            writeln!(file, "  2. Fix highest priority issues first (reference tool mismatches)")?;
            writeln!(file, "  3. Re-run validation after fixes")?;
            writeln!(file, "  4. Check for patterns in failures (same format, same error type)")?;
        }
        
        writeln!(file, "{}", "=".repeat(80))?;

        Ok(report_path)
    }

    /// Generate hex dump for byte-level analysis
    fn generate_hex_dump(
        &self,
        file_path: &Path,
        output_path: &Path,
    ) -> Result<(), ValidationError> {
        let data = fs::read(file_path)?;
        let mut output = fs::File::create(output_path)?;
        
        writeln!(output, "HEX DUMP: {:?}", file_path)?;
        writeln!(output, "File Size: {} bytes", data.len())?;
        writeln!(output)?;
        writeln!(output, "Offset    Hex                                              ASCII")?;
        writeln!(output, "{}", "-".repeat(80))?;
        
        for (offset, chunk) in data.chunks(16).enumerate() {
            write!(output, "{:08x}  ", offset * 16)?;
            
            // Hex representation
            for (i, byte) in chunk.iter().enumerate() {
                write!(output, "{:02x} ", byte)?;
                if i == 7 {
                    write!(output, " ")?;
                }
            }
            
            // Padding for incomplete lines
            for _ in chunk.len()..16 {
                write!(output, "   ")?;
            }
            
            write!(output, " ")?;
            
            // ASCII representation
            for byte in chunk {
                let ch = if byte.is_ascii_graphic() || *byte == b' ' {
                    *byte as char
                } else {
                    '.'
                };
                write!(output, "{}", ch)?;
            }
            
            writeln!(output)?;
        }
        
        Ok(())
    }

    /// Generate side-by-side visual comparison
    pub fn generate_visual_comparison(
        &self,
        extracted_file: &Path,
        reference_file: &Path,
        output_path: &Path,
    ) -> Result<(), ValidationError> {
        use image::{ImageBuffer, Rgba, RgbaImage};
        
        // Load both images
        let extracted_img = image::open(extracted_file)?;
        let reference_img = image::open(reference_file)?;
        
        let extracted_rgba = extracted_img.to_rgba8();
        let reference_rgba = reference_img.to_rgba8();
        
        // Create side-by-side comparison
        let width = extracted_rgba.width() + reference_rgba.width() + 10;
        let height = extracted_rgba.height().max(reference_rgba.height());
        
        let mut comparison: RgbaImage = ImageBuffer::new(width, height);
        
        // Fill with white background
        for pixel in comparison.pixels_mut() {
            *pixel = Rgba([255, 255, 255, 255]);
        }
        
        // Copy extracted image
        for (x, y, pixel) in extracted_rgba.enumerate_pixels() {
            comparison.put_pixel(x, y, *pixel);
        }
        
        // Copy reference image
        for (x, y, pixel) in reference_rgba.enumerate_pixels() {
            comparison.put_pixel(x + extracted_rgba.width() + 10, y, *pixel);
        }
        
        comparison.save(output_path)?;
        
        Ok(())
    }

    /// Create error diagnostic with recommendations
    pub fn create_diagnostic(
        &self,
        error_type: ErrorType,
        severity: Severity,
        file_name: String,
        description: String,
    ) -> ErrorDiagnostic {
        let recommendations = self.get_recommendations(&error_type);
        
        ErrorDiagnostic {
            error_type,
            severity,
            file_name,
            description,
            hex_dump: None,
            visual_comparison_path: None,
            recommendations,
            context: Vec::new(),
        }
    }

    /// Get actionable recommendations for error type
    fn get_recommendations(&self, error_type: &ErrorType) -> Vec<String> {
        Self::recommendations_for(error_type).iter().map(|s| s.to_string()).collect()
    }

    fn recommendations_for(error_type: &ErrorType) -> &'static [&'static str] {
        match error_type {
            ErrorType::ByteMismatch => &[
                "Compare hex dumps byte-by-byte to identify differences",
                "Check endianness in binary parsing (little-endian vs big-endian)",
                "Verify all header fields are correctly parsed",
                "Compare with reference tool source code if available",
            ],
            ErrorType::VisualMismatch => &[
                "Review visual diff image to identify pixel differences",
                "Check color palette conversion logic",
                "Verify transparency handling (alpha channel)",
                "Ensure correct pixel format conversion (RGB/RGBA)",
            ],
            ErrorType::MetadataMismatch => &[
                "Verify sprite dimensions match expected values",
                "Check frame count and animation timing",
                "Validate texture format and compression settings",
            ],
            ErrorType::UnityImportFailure => &[
                "Check PNG format compatibility with Unity",
                "Verify sprite dimensions are within Unity limits",
                "Ensure proper alpha channel handling",
                "Check Unity import settings (texture type, compression)",
            ],
            ErrorType::ReferenceToolMismatch => &[
                "Compare extraction output with reference tool byte-by-byte",
                "Study reference tool source code if available",
                "Verify decompression algorithm matches reference implementation",
            ],
            ErrorType::FormatParsingError => &[
                "Review format specification documentation",
                "Check for correct header parsing",
                "Verify offset calculations and bounds checking",
            ],
            ErrorType::DecompressionError => &[
                "Verify compression type detection (ZLIB, LZ4, etc.)",
                "Check decompression library usage",
                "Validate compressed data size and format",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_error_reporter_creation() {
        let temp_dir = TempDir::new().unwrap();
        let reporter = ErrorReporter::new(temp_dir.path().to_path_buf());
        assert!(reporter.is_ok());
    }

    #[test]
    fn test_create_diagnostic() {
        let temp_dir = TempDir::new().unwrap();
        let reporter = ErrorReporter::new(temp_dir.path().to_path_buf()).unwrap();
        
        let diagnostic = reporter.create_diagnostic(
            ErrorType::ByteMismatch,
            Severity::High,
            "test.png".to_string(),
            "Byte mismatch detected".to_string(),
        );
        
        assert_eq!(diagnostic.error_type, ErrorType::ByteMismatch);
        assert_eq!(diagnostic.severity, Severity::High);
        assert!(!diagnostic.recommendations.is_empty());
    }

    #[test]
    fn test_get_recommendations() {
        let temp_dir = TempDir::new().unwrap();
        let reporter = ErrorReporter::new(temp_dir.path().to_path_buf()).unwrap();
        
        let recommendations = reporter.get_recommendations(&ErrorType::ByteMismatch);
        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.contains("hex dump")));
    }
}
