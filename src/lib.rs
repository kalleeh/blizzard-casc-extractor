//! CASC Sprite Extractor Library
//! 
//! This library provides functionality for extracting sprite assets from 
//! StarCraft: Remastered CASC archives and converting them to PNG format.

pub mod casc;
pub mod anim;
pub mod grp;
pub mod sprite;
pub mod palette;
pub mod mapping;
pub mod config;
pub mod filter;
pub mod resolution;
pub mod progress;
pub mod validation;

pub use casc::{CascArchive, CascError, FileEntry, FileAnalysis};
pub use casc::discovery::{locate_starcraft, open_archive};
pub use anim::{AnimFile, AnimError, CompressionType, PixelFormat, AnimPalette};
pub use grp::{GrpFile, GrpFrame, GrpError};
pub use sprite::export::{ExportConfig, ExportResult, export_anim, generate_metadata};
pub use resolution::ResolutionTier;
pub use config::{ExtractionConfig, QualitySettings, OutputSettings, FeedbackSettings, UnityExportSettings, MetadataOptions, OverwriteBehavior};
pub use filter::{FileFilter, FilterStats, FilterResult, FormatFilter, FileInfo};
pub use progress::ProgressReporter;
pub use casc::casclib_ffi::CascStorage;
pub use validation::{ReferenceValidator, ByteComparison, ByteComparisonResult, VisualComparison, VisualComparisonResult, UnityImportValidator, UnityImportResult, RegressionTestSuite, KnownGoodExtraction, ValidationError, ValidationResult};
pub mod dds_converter;
