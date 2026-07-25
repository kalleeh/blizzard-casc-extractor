//! CASC Sprite Extractor Library
//!
//! This library provides functionality for extracting sprite assets from
//! StarCraft: Remastered CASC archives and converting them to PNG format.

pub mod anim;
pub mod casc;
pub mod config;
pub mod filter;
pub mod grp;
pub mod mapping;
pub mod palette;
pub mod progress;
pub mod resolution;
pub mod sprite;
pub mod validation;

pub use anim::{AnimError, AnimFile, AnimPalette, CompressionType, PixelFormat};
pub use casc::casclib_ffi::CascStorage;
pub use casc::discovery::{locate_starcraft, open_archive};
pub use casc::{CascArchive, CascError, FileAnalysis, FileEntry};
pub use config::{
    ExtractionConfig, FeedbackSettings, MetadataOptions, OutputSettings, OverwriteBehavior,
    QualitySettings, UnityExportSettings,
};
pub use filter::{FormatFilterOption, UnityFilterMode, UnityWrapMode};
pub use grp::{GrpError, GrpFile, GrpFrame};
pub use progress::ProgressReporter;
pub use resolution::ResolutionTier;
pub use sprite::export::{export_anim, generate_metadata, ExportConfig, ExportResult};
pub use validation::{
    ByteComparison, ByteComparisonResult, KnownGoodExtraction, RegressionTestSuite,
    ValidationError, VisualComparison, VisualComparisonResult,
};
pub mod dds_converter;
