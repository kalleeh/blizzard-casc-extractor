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
pub mod blte;
pub mod blte_enhanced;
pub mod format_converter;
pub mod format_analyzer;
pub mod pipeline;
pub mod integration_tests;
pub mod validation;

pub mod generators;

pub use casc::{CascArchive, CascError, IndexFile, IndexEntry, FileEntry, ValidationReport, SizeValidation, FileAnalysis, CascNavigator, Installation, GameVersion, FileSystemType, NavigatorError, EncryptionHandler, FileAccessLayer, EncryptionError, EncryptionMethod, DecryptionKey};
pub use casc::discovery::{locate_starcraft, open_archive};
pub use anim::{AnimFile, AnimError, CompressionType, PixelFormat, AnimPalette};
pub use grp::{GrpFile, GrpFrame, GrpError};
pub use sprite::{DirectSpriteExtractor, SpriteData, SpriteFormat, SpriteError, ExtractionResult, UnityConverter, SpriteMetadata, UnityMetadata, UnityPivot, ImageDimensions};
pub use sprite::export::{ExportConfig, ExportResult, export_anim, generate_metadata};
pub use cli::{CliArgs, ResolutionTier, FormatFilterOption};
pub use config::{ExtractionConfig, FormatSettings, QualitySettings, PerformanceSettings, OutputSettings, FeedbackSettings, UnityExportSettings, FormatType, ExtractionMode, ConflictResolution, FormatQuality, ColorDepth, NamingConvention, DirectoryStructure, MetadataOptions, OverwriteBehavior, profiles::{ConfigurationProfileManager, ConfigurationProfile, ProfileMetadata}};
pub use filter::{FileFilter, FilterStats, FilterResult, FormatFilter, FileInfo};
pub use progress::ProgressReporter;
pub use research::{ResearchDataCollector, ResearchData, CascStats, FormatAnalysis, ExtractionStats};
pub use blte::{BlteFile, BlteError, is_blte_data, looks_like_blte_data};
pub use format_converter::{FormatConverter, ConversionResult};
pub use format_analyzer::{FormatAnalyzer, SpritePatternAnalysis};
pub use pipeline::{UnifiedPipeline, PipelineResult, PipelineMetrics, ProcessedFile, FailedFile};
pub use validation::{ReferenceValidator, ByteComparison, ByteComparisonResult, VisualComparison, VisualComparisonResult, UnityImportValidator, UnityImportResult, RegressionTestSuite, KnownGoodExtraction, ValidationError, ValidationResult};pub mod dds_converter;
pub mod casclib_ffi;
pub use casclib_ffi::CascStorage;
