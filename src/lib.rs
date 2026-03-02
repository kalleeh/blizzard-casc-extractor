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
pub mod cli;
pub mod config;
pub mod filter;
pub mod resolution;
pub mod progress;
pub mod research;
pub mod blte_enhanced;
pub mod format_converter;
pub mod format_analyzer;
pub mod validation;

pub use casc::{CascArchive, CascError, FileEntry, FileAnalysis};
pub use casc::discovery::{locate_starcraft, open_archive};
pub use anim::{AnimFile, AnimError, CompressionType, PixelFormat, AnimPalette};
pub use grp::{GrpFile, GrpFrame, GrpError};
pub use sprite::{DirectSpriteExtractor, SpriteData, SpriteFormat, SpriteError, ExtractionResult, UnityConverter, SpriteMetadata, UnityMetadata, UnityPivot, ImageDimensions};
pub use sprite::export::{ExportConfig, ExportResult, export_anim, generate_metadata};
pub use cli::ResolutionTier;
pub use config::{ExtractionConfig, FormatSettings, QualitySettings, OutputSettings, FeedbackSettings, UnityExportSettings, FormatType, ExtractionMode, FormatQuality, ColorDepth, MetadataOptions, OverwriteBehavior};
pub use filter::{FileFilter, FilterStats, FilterResult, FormatFilter, FileInfo};
pub use progress::ProgressReporter;
pub use research::{ResearchDataCollector, ResearchData, CascStats, FormatAnalysis, ExtractionStats};
pub use casc::casclib_ffi::CascStorage;
pub use format_converter::{FormatConverter, ConversionResult};
pub use format_analyzer::{FormatAnalyzer, SpritePatternAnalysis};
pub use validation::{ReferenceValidator, ByteComparison, ByteComparisonResult, VisualComparison, VisualComparisonResult, UnityImportValidator, UnityImportResult, RegressionTestSuite, KnownGoodExtraction, ValidationError, ValidationResult};
pub mod dds_converter;
