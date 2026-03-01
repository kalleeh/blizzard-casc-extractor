// Validation module for cross-tool comparison and quality assurance
//
// This module provides comprehensive validation against reference tools to ensure
// 100% byte-level accuracy and visual correctness of extracted sprites.

pub mod reference_validator;
pub mod byte_comparison;
pub mod visual_validation;
pub mod unity_import;
pub mod regression_suite;
pub mod pipeline;
pub mod error_reporting;

pub use reference_validator::ReferenceValidator;
pub use byte_comparison::{ByteComparison, ByteComparisonResult};
pub use visual_validation::{VisualComparison, VisualComparisonResult};
pub use unity_import::{UnityImportValidator, UnityImportResult, SpriteMetadata};
pub use regression_suite::{RegressionTestSuite, KnownGoodExtraction};
pub use pipeline::{
    ValidationPipeline, ValidationConfig, ExtractionValidationReport,
    BatchValidationReport, PipelineReferenceResult, PipelineByteResult,
    PipelineVisualResult, PipelineUnityResult,
    RegressionTestResult,
};
pub use error_reporting::{
    ErrorReporter, ErrorDiagnostic, ErrorType, Severity,
};

use std::path::PathBuf;
use thiserror::Error;

/// Validation errors that can occur during reference tool comparison
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Reference tool not found: {tool} at path {path:?}")]
    ReferenceToolNotFound { tool: String, path: PathBuf },

    #[error("Reference extraction failed: {tool} - {reason}")]
    ReferenceExtractionFailed { tool: String, reason: String },

    #[error("Byte-level mismatch: {details}")]
    ByteMismatch { details: String },

    #[error("Visual mismatch: {details}")]
    VisualMismatch { details: String },

    #[error("Metadata mismatch: {details}")]
    MetadataMismatch { details: String },

    #[error("Unity import failed: {details}")]
    UnityImportFailed { details: String },

    #[error("Regression detected: {failed_tests} of {total_tests} tests failed. Report: {report_path:?}")]
    RegressionDetected {
        failed_tests: usize,
        total_tests: usize,
        report_path: PathBuf,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image processing error: {0}")]
    Image(#[from] image::ImageError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Overall validation result combining all validation checks
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub byte_match: bool,
    pub visual_match: bool,
    pub metadata_match: bool,
    pub unity_import_success: bool,
    pub regression_check_passed: bool,
    pub overall_pass: bool,
    pub diagnostics: Vec<String>,
}

impl ValidationResult {
    /// Create a new validation result with all checks passing
    pub fn success() -> Self {
        Self {
            byte_match: true,
            visual_match: true,
            metadata_match: true,
            unity_import_success: true,
            regression_check_passed: true,
            overall_pass: true,
            diagnostics: Vec::new(),
        }
    }

    /// Create a new validation result with a failure
    pub fn failure(diagnostic: String) -> Self {
        Self {
            byte_match: false,
            visual_match: false,
            metadata_match: false,
            unity_import_success: false,
            regression_check_passed: false,
            overall_pass: false,
            diagnostics: vec![diagnostic],
        }
    }

    /// Add a diagnostic message
    pub fn add_diagnostic(&mut self, message: String) {
        self.diagnostics.push(message);
    }

    /// Check if all validations passed
    pub fn is_success(&self) -> bool {
        self.overall_pass
    }
}
