// Validation module for cross-tool comparison and quality assurance
//
// This module provides comprehensive validation against reference tools to ensure
// 100% byte-level accuracy and visual correctness of extracted sprites.

pub mod byte_comparison;
pub mod visual_validation;
pub mod regression_suite;

pub use byte_comparison::{ByteComparison, ByteComparisonResult};
pub use visual_validation::{VisualComparison, VisualComparisonResult};
pub use regression_suite::{RegressionTestSuite, KnownGoodExtraction};

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
