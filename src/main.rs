//! casc-extractor — unified CLI entry point for StarCraft: Remastered asset extraction
//!
//! Exposes all extraction functionality under one binary:
//!   casc-extractor extract anim
//!   casc-extractor extract tileset
//!   casc-extractor extract effect
//!   casc-extractor extract organized
//!   casc-extractor sounds extract
//!   casc-extractor sounds list
//!   casc-extractor inspect sprites
//!   casc-extractor inspect archive

mod fallbacks;

use anyhow::{Context as _, Result};
use casc_extractor::anim::HdAnimFile;
use casc_extractor::casc::casclib_ffi::CascArchive;
use casc_extractor::casc::discovery::locate_starcraft;
use casc_extractor::config::{ExtractionConfig, OverwriteBehavior};
use casc_extractor::filter::FormatFilterOption;
use casc_extractor::validation::regression_suite::KnownGoodExtraction;
use casc_extractor::validation::regression_suite::SpriteMetadata as RegressionSpriteMetadata;
use rayon::prelude::*;
use regex::Regex;
use casc_extractor::grp::GrpFile;
use casc_extractor::mapping::SpriteMapping;
use casc_extractor::{export_anim, CascStorage, ExportConfig};
use clap::{Parser, Subcommand, ValueEnum};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Quality level (shared by extract anim/tileset/effect + inspect sprites)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, ValueEnum)]
enum QualityLevel {
    /// Original SD quality (GRP format)
    Sd,
    /// 2x HD quality
    Hd2,
    /// 4x Ultra HD quality (default)
    Hd4,
}

impl QualityLevel {
    fn path_prefix(self) -> &'static str {
        match self {
            QualityLevel::Sd => "SD/",
            QualityLevel::Hd2 => "HD2/",
            QualityLevel::Hd4 => "",
        }
    }

    fn description(self) -> &'static str {
        match self {
            QualityLevel::Sd => "SD (Original)",
            QualityLevel::Hd2 => "2x HD",
            QualityLevel::Hd4 => "4x Ultra HD",
        }
    }
}

// ---------------------------------------------------------------------------
// Sound targets (mirrored from src/bin/extract_sounds.rs)
// ---------------------------------------------------------------------------

/// JSON-serialisable representation of a single sound target entry.
/// Used by `sounds export-targets` (write) and `sounds extract --targets` (read).
#[derive(serde::Serialize, serde::Deserialize)]
struct SoundTargetEntry {
    output: String,
    candidates: Vec<String>,
}

/// (output_filename, list of candidate CASC paths to try in order)
const SOUND_TARGETS: &[(&str, &[&str])] = &[
    ("marine_yes1.ogg", &[
        "sound\\Terran\\marine\\tmayes00.wav",
        "sound\\terran\\marine\\tmayes00.wav",
        "sound\\Terran\\Marine\\tmayes00.wav",
    ]),
    ("marine_move1.ogg", &[
        "sound\\Terran\\marine\\tmamov00.wav",
        "sound\\terran\\marine\\tmamov00.wav",
        "sound\\Terran\\marine\\tmardy00.wav",
        "sound\\terran\\marine\\tmardy00.wav",
    ]),
    ("marine_die.ogg", &[
        "sound\\Terran\\marine\\tmadth00.wav",
        "sound\\terran\\marine\\tmadth00.wav",
        "sound\\Terran\\Marine\\tmadth00.wav",
    ]),
    ("marine_attack.ogg", &[
        "sound\\Terran\\marine\\tmaatt00.wav",
        "sound\\terran\\marine\\tmaatt00.wav",
        "sound\\Weapons\\Terran\\tgun.wav",
        "sound\\Weapons\\terran\\tgun.wav",
        "sound\\weapons\\terran\\tgun.wav",
        "sound\\Terran\\Weapons\\tgun.wav",
        "sound\\Terran\\weapons\\tgun.wav",
        "sound\\Terran\\marine\\tmasti00.wav",
        "sound\\terran\\marine\\tmasti00.wav",
        "sound\\Terran\\marine\\tmawht00.wav",
        "sound\\terran\\marine\\tmawht00.wav",
    ]),
    ("zergling_attack.ogg", &[
        "\\zerg\\zergling\\zlatt00.wav",
        "\\zerg\\Zergling\\ZlAtt00.wav",
        "\\Zerg\\Zergling\\ZlAtt00.wav",
        "\\Zerg\\zergling\\zlatt00.wav",
        "\\sound\\zerg\\zergling\\zlatt00.wav",
        "\\sound\\Zerg\\Zergling\\ZlAtt00.wav",
        "sound\\Zerg\\Zergling\\ZlAtt00.wav",
        "sound/Zerg/Zergling/ZlAtt00.wav",
        "zerg\\zergling\\zlatt00.wav",
        "zerg/zergling/zlatt00.wav",
        "\\zerg\\zergling\\zlwht00.wav",
        "\\Zerg\\Zergling\\ZlWht00.wav",
        "zerg\\zergling\\zlwht00.wav",
        "zerg/zergling/zlwht00.wav",
        "\\zerg\\zergling\\zlatt00.wav",
        "\\zerg\\Zergling\\ZlAtt00.wav",
    ]),
    ("zergling_die.ogg", &[
        "\\zerg\\zergling\\zldth00.wav",
        "\\zerg\\Zergling\\ZlDth00.wav",
        "\\Zerg\\Zergling\\ZlDth00.wav",
        "\\sound\\zerg\\zergling\\zldth00.wav",
        "sound\\Zerg\\Zergling\\ZlDth00.wav",
        "sound/Zerg/Zergling/ZlDth00.wav",
        "zerg\\zergling\\zldth00.wav",
        "zerg/zergling/zldth00.wav",
        "\\zerg\\zergling\\zldth00.wav",
        "\\zerg\\Zergling\\ZlDth00.wav",
    ]),
    ("button.ogg", &[
        "sound\\Misc\\button.wav",
        "sound\\misc\\button.wav",
        "sound\\Glue\\button.wav",
        "sound\\UI\\button.wav",
        "sound\\misc\\buttonclk.wav",
        "sound\\Misc\\Klink.wav",
        "sound\\misc\\klink.wav",
    ]),
    ("mouseover.ogg", &[
        "sound\\Glue\\mouseover.wav",
        "sound\\Glue\\swishlock.wav",
        "sound\\glue\\mouseover.wav",
        "sound\\glue\\swishlock.wav",
        "\\glue\\mouseover.wav",
        "glue\\mouseover.wav",
    ]),
    ("hydralisk_yes1.ogg", &[
        "sound\\Zerg\\hydra\\zhyyes00.wav",
        "sound\\zerg\\hydra\\zhyyes00.wav",
        "sound\\Zerg\\hydra\\zhyyes01.wav",
    ]),
    ("hydralisk_ready.ogg", &[
        "sound\\Zerg\\hydra\\zhyrdy00.wav",
        "sound\\zerg\\hydra\\zhyrdy00.wav",
    ]),
    ("hydralisk_attack.ogg", &[
        "sound\\Zerg\\hydra\\spifir00.wav",
        "sound\\zerg\\hydra\\spifir00.wav",
    ]),
    ("ghost_yes1.ogg", &[
        "sound\\Terran\\ghost\\tghyes01.wav",
        "sound\\terran\\ghost\\tghyes01.wav",
        "sound\\Terran\\ghost\\tghyes00.wav",
    ]),
    ("ghost_ready.ogg", &[
        "sound\\Terran\\ghost\\tghrdy00.wav",
        "sound\\terran\\ghost\\tghrdy00.wav",
    ]),
    ("scv_yes1.ogg", &[
        "sound\\Terran\\scv\\tscpss00.wav",
        "sound\\terran\\scv\\tscpss00.wav",
        "sound\\Terran\\scv\\tscyes00.wav",
    ]),
    ("scv_die.ogg", &[
        "sound\\Terran\\scv\\tscdth00.wav",
        "sound\\terran\\scv\\tscdth00.wav",
    ]),
];

// ---------------------------------------------------------------------------
// CLI structure
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "casc-extractor",
    about = "Extract and inspect StarCraft: Remastered assets from the CASC archive",
    long_about = None,
    version,
)]
struct Cli {
    /// StarCraft installation directory (auto-detected if omitted)
    #[arg(long, global = true)]
    install_path: Option<PathBuf>,

    /// Output directory (overrides config file)
    #[arg(long, global = true)]
    output: Option<PathBuf>,

    /// Path to a JSON config file (see ExtractionConfig)
    #[arg(long, short = 'c', global = true)]
    config: Option<PathBuf>,

    /// Enable verbose/debug logging (overrides config file)
    #[arg(long, short = 'v', global = true)]
    verbose: bool,

    /// Check that the archive can be opened and files found without writing output
    #[arg(long, global = true)]
    validate_only: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract assets from the CASC archive
    Extract {
        #[command(subcommand)]
        target: ExtractCommands,
    },
    /// Audio operations (extract or list sounds)
    Sounds {
        #[command(subcommand)]
        action: SoundsCommands,
    },
    /// Inspect archive contents
    Inspect {
        #[command(subcommand)]
        target: InspectCommands,
    },
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
    /// Validation and regression testing
    Validate {
        #[command(subcommand)]
        action: ValidateCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Write a default config JSON file to disk
    Init {
        /// Output path for the generated config file
        #[arg(long, default_value = "casc-config.json")]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum ValidateCommands {
    /// Register a previously extracted file as a known-good baseline
    Register {
        /// Path to the extracted file to register
        file: PathBuf,

        /// Path to the regression suite JSON file
        #[arg(long, default_value = "validation-suite.json")]
        suite: PathBuf,
    },

    /// Run regression checks against a directory of extracted files
    Run {
        /// Directory of extracted files to validate
        dir: PathBuf,

        /// Path to the regression suite JSON
        #[arg(long, default_value = "validation-suite.json")]
        suite: PathBuf,

        /// Output results as JSON instead of human-readable text
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ExtractCommands {
    /// Extract HD animations to ANIM (optionally converted to PNG + JSON metadata).
    /// By default uses the built-in YAML mapping to produce named output
    /// (e.g. terran/marine.anim).  Pass --raw to get numbered filenames instead.
    Anim {
        /// Quality level
        #[arg(long, value_enum, default_value = "hd4")]
        quality: QualityLevel,

        /// Specific animation IDs to extract, comma-separated (e.g. 0,1,7).
        /// Without --ids, all known IDs in the built-in mapping are extracted.
        #[arg(long, value_delimiter = ',')]
        ids: Option<Vec<u16>>,

        /// Output raw numbered filenames (main_NNN.anim) instead of mapped names.
        /// Without --raw, the built-in mapping produces named subdirectory output.
        #[arg(long)]
        raw: bool,

        /// Override the built-in name mapping with a custom JSON file
        /// (format: {"239": "terran/marine", "228": "protoss/zealot"})
        #[arg(long)]
        name_map: Option<PathBuf>,

        /// Convert ANIM to PNG (extracts diffuse layer)
        #[arg(long)]
        convert_to_png: bool,

        /// Export team-color mask alongside diffuse PNG (requires --convert-to-png)
        #[arg(long)]
        team_color_mask: bool,

        /// Comma-separated list of ANIM layers to export (requires --convert-to-png).
        /// Valid: diffuse,teamcolor,normal,specular,emissive,ao  (default: diffuse)
        #[arg(long, value_delimiter = ',', default_values = ["diffuse"])]
        layers: Vec<String>,

        /// Write the raw diffuse DDS file alongside the PNG
        #[arg(long)]
        save_dds: bool,
    },

    /// Extract HD tilesets to `<output>/tilesets/`
    Tileset {
        /// Quality level
        #[arg(long, value_enum, default_value = "hd4")]
        quality: QualityLevel,

        /// Convert extracted DDS to PNG
        #[arg(long)]
        convert_to_png: bool,
    },

    /// Extract HD effects to `<output>/effects/`
    Effect {
        /// Quality level
        #[arg(long, value_enum, default_value = "hd4")]
        quality: QualityLevel,

        /// Convert extracted DDS/GRP to PNG
        #[arg(long)]
        convert_to_png: bool,
    },

    /// Extract VR4 tileset as a PNG sprite atlas (tiles arranged in a grid)
    TilesetAtlas {
        /// Which tileset to extract
        #[arg(long, default_value = "jungle")]
        tileset: String,

        /// Quality level
        #[arg(long, value_enum, default_value = "hd2")]
        quality: QualityLevel,

        /// Number of tiles per row in the output atlas
        #[arg(long, default_value_t = 32)]
        cols: u32,

        /// Max tiles to extract (0 = all)
        #[arg(long, default_value_t = 512)]
        max_tiles: u32,

        /// Output tile size in pixels (tiles are scaled to this size)
        #[arg(long, default_value_t = 32)]
        tile_size: u32,

        /// Output directory
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Build a megatile atlas from VX4EX + VR4 data (game-ready terrain tiles)
    TilesetMegatile {
        /// Which tileset to extract
        #[arg(long, default_value = "jungle")]
        tileset: String,

        /// Quality level (hd2 recommended; hd4 may not have vr4)
        #[arg(long, value_enum, default_value = "hd2")]
        quality: QualityLevel,

        /// Max megatiles to extract (0 = all; typical map uses ~2000 unique megatiles)
        #[arg(long, default_value_t = 1024)]
        max_tiles: u32,

        /// Output tile size per megatile in atlas (32 = game tile size)
        #[arg(long, default_value_t = 32)]
        tile_size: u32,

        /// Atlas columns
        #[arg(long, default_value_t = 32)]
        cols: u32,

        /// Output directory
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Extract the exact terrain tiles used by a specific BW map (.scm/.scx).
    /// Output: `<output>/map-terrain/` by default.
    MapTerrain {
        /// Path to the .scm or .scx map file
        #[arg(long)]
        scm: PathBuf,

        /// Columns in output atlas
        #[arg(long, default_value_t = 32)]
        cols: u32,

        /// Tile size in output atlas (pixels per megatile)
        #[arg(long, default_value_t = 32)]
        tile_size: u32,

        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Extract dual-layer terrain atlas (diffuse + base) from a BW map (.scm/.scx).
    /// Output: `<output>/map-terrain/` by default.
    MapTerrainDual {
        /// Path to the .scm or .scx map file
        #[arg(long)]
        scm: PathBuf,

        /// Columns in output atlas
        #[arg(long, default_value_t = 32)]
        cols: u32,

        /// Tile size in output atlas (pixels per megatile)
        #[arg(long, default_value_t = 32)]
        tile_size: u32,

        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Extract main-menu UI assets (backgrounds, logos, button videos, music, SFX).
    /// DDS images are automatically converted to PNG.
    Mainmenu {
        /// Output directory (default: ./output/ui/mainmenu)
        #[arg(long)]
        output: Option<PathBuf>,

        /// Convert DDS textures to PNG (default: true; pass --no-convert-dds to skip)
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        convert_dds: bool,
    },

    /// Extract BW data tables: units.dat, upgrades.dat, images.dat, orders.dat, techdata.dat.
    Dat {
        /// Output directory (default: ./output/dat)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Extract and colour-tint the command-card icon atlas (unit\cmdicons\cmdicons.dds.grp).
    ///
    /// Produces a 2176×2944 RGBA PNG atlas (17 cols × 23 rows, 128×128 px per cell)
    /// with a yellow BW-style tint applied.  The 390 frames map directly to
    /// cmdicons.grp slot indices: 0-227 = unit wireframe portraits, 228-389 = command buttons.
    Cmdicons {
        /// Output directory
        #[arg(long)]
        output: Option<PathBuf>,

        /// Yellow tint colour as R,G,B (0-255).  Defaults to BW yellow (255,254,84).
        #[arg(long, value_delimiter = ',', default_values = ["255","254","84"])]
        tint: Vec<u8>,

        /// Write raw untinted greyscale atlas alongside the tinted version
        #[arg(long)]
        save_raw: bool,
    },

    /// Extract raw bytes from one or more CASC archive paths without any processing.
    ///
    /// Files that are CDN-only (not present in the local install) will be checked
    /// against built-in embedded fallbacks before being skipped.  If a matching
    /// embedded copy exists it is written to the output directory and counts as a
    /// successful extraction.  Only files that cannot be resolved by any means are
    /// reported as skipped.
    /// Exit code is non-zero only when ALL requested files failed.
    /// Use --online to fetch files that are CDN-only (not in local install).
    ///
    /// EMBEDDED FALLBACKS: Known SC:R UI layout files (.ui.json) are embedded in
    /// the binary and will be served even without --online or a local install.
    /// Currently embedded: statbtnn, statbtnp, statbtnt, statbtnz, statdata, statport.
    ///
    /// LOCALE PREFIX: Many SC:R files live under a locale prefix in the archive
    /// (e.g. `locales\enUS\Assets\rez\statbtnn.ui.json`). When --online is used and
    /// a positional path does not already start with `locales\` or `locales/`, the
    /// prefix `locales\<locale>\` is prepended automatically.
    Raw {
        /// One or more exact CASC archive paths to extract.
        /// Example: `locales\enUS\Assets\rez\statbtnn.ui.json`
        /// When --online is active and the path does not start with `locales\`,
        /// the locale prefix (`locales\<locale>\`) is added automatically.
        /// Mutually exclusive with --search.
        #[arg(conflicts_with = "search")]
        file_paths: Vec<String>,

        /// Extract all archive files whose path contains this substring (case-insensitive).
        /// Mutually exclusive with positional FILE_PATH arguments.
        #[arg(long, conflicts_with = "file_paths")]
        search: Option<String>,

        /// Locale used when constructing the locale prefix for --online paths and
        /// when filtering --search results (default: enUS).
        #[arg(long, default_value = "enUS")]
        locale: String,

        /// Keep the full archive path as the output sub-path (default: filename only).
        #[arg(long, short = 'p')]
        preserve_path: bool,

        /// Attempt to fetch CDN-only files over the network (requires internet access).
        /// When set, positional paths that lack a `locales\` prefix will have
        /// `locales\<locale>\` prepended automatically.
        #[arg(long)]
        online: bool,
    },

}


#[derive(Subcommand)]
enum SoundsCommands {
    /// Extract known unit and UI sounds from the archive
    Extract {
        /// Output directory for extracted sounds (overrides global --output).
        /// Default: `<output>/sounds/`
        #[arg(long)]
        sounds_output: Option<PathBuf>,

        /// Path to a custom JSON targets file (produced by export-targets)
        #[arg(long)]
        targets: Option<PathBuf>,
    },

    /// List available audio files in the archive.
    /// Use --search to filter all archive paths by a pattern.
    List {
        /// Filter: only show paths containing this string (case-insensitive)
        #[arg(long)]
        search: Option<String>,
    },

    /// Probe known unit sound paths and report which ones resolve in the archive.
    /// Useful for verifying path conventions before adding new sounds.
    Probe {
        /// Only probe paths containing this string (case-insensitive)
        #[arg(long)]
        filter: Option<String>,
    },

    /// Write the built-in sound targets to a JSON file for customisation
    ExportTargets {
        #[arg(long, default_value = "sound-targets.json")]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum InspectCommands {
    /// Scan anim IDs 0-999 and print which ones exist in the archive
    Sprites {
        /// Quality level to probe
        #[arg(long, value_enum, default_value = "hd4")]
        quality: QualityLevel,

        /// Upper bound for IDs to scan (exclusive, max 1000)
        #[arg(long, default_value = "1000")]
        max_id: u16,
    },

    /// Print basic archive information
    Archive,

    /// List or search all files in the CASC archive.
    Files {
        /// Filter: only show paths containing this string (case-insensitive).
        /// Without --search, lists all files.
        #[arg(long)]
        search: Option<String>,

        /// Show file sizes alongside paths
        #[arg(long)]
        sizes: bool,
    },

}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns `true` when the file should be skipped based on overwrite policy.
///
/// `session_start` is used only by `IfNewer`: skip the file if it already
/// exists and its mtime is at or after the session start time (i.e., it was
/// written during this extraction session and need not be re-extracted).
fn should_skip(path: &Path, behavior: OverwriteBehavior, session_start: std::time::SystemTime) -> bool {
    match behavior {
        OverwriteBehavior::Never => path.exists(),
        OverwriteBehavior::IfNewer => {
            if !path.exists() {
                return false;
            }
            // Skip if the file was written at or after the session start time,
            // meaning it was already extracted during this run.
            // On any metadata error, conservatively overwrite.
            std::fs::metadata(path)
                .and_then(|m| m.modified())
                .map(|mtime| mtime >= session_start)
                .unwrap_or(false)
        }
        OverwriteBehavior::Backup => {
            if path.exists() {
                let bak = path.with_extension(
                    format!(
                        "{}.bak",
                        path.extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                    )
                );
                if let Err(e) = fs::rename(path, &bak) {
                    eprintln!("Warning: could not back up {:?}: {}", path, e);
                }
            }
            false
        }
        OverwriteBehavior::Prompt => {
            if !path.exists() {
                return false;
            }
            print!("File {:?} already exists. Overwrite? [y/N]: ", path);
            let _ = std::io::stdout().flush();
            let mut line = String::new();
            let _ = std::io::stdin().read_line(&mut line);
            let answer = line.trim();
            !(answer == "y" || answer == "Y")
        }
        _ => false,
    }
}

/// Map a 0-9 PNG compression level to the `png` crate's compression enum.
fn png_compression(level: u32) -> png::Compression {
    match level {
        0..=2 => png::Compression::Fast,
        7..=9 => png::Compression::Best,
        _ => png::Compression::Default,
    }
}

/// Compile a list of regex pattern strings, silently skipping invalid ones.
fn compile_patterns(patterns: &Option<Vec<String>>) -> Vec<Regex> {
    patterns
        .as_ref()
        .map(|pats| {
            pats.iter()
                .filter_map(|p| Regex::new(p).map_err(|e| {
                    eprintln!("Warning: invalid filter pattern {:?}: {}", p, e);
                }).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Returns `true` if the path passes the include/exclude filter.
/// An empty include list means "allow everything".
fn passes_filter(path: &str, include: &[Regex], exclude: &[Regex]) -> bool {
    if !include.is_empty() && !include.iter().any(|r| r.is_match(path)) {
        return false;
    }
    !exclude.iter().any(|r| r.is_match(path))
}

/// Load ExtractionConfig from a JSON file, falling back to defaults on any error.
fn load_config(path: Option<&Path>) -> ExtractionConfig {
    let path = match path {
        Some(p) => p,
        None => return ExtractionConfig::default(),
    };
    match fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            eprintln!("Warning: could not parse config {:?}: {}", path, e);
            ExtractionConfig::default()
        }),
        Err(e) => {
            eprintln!("Warning: could not read config {:?}: {}", path, e);
            ExtractionConfig::default()
        }
    }
}

const INSTALL_PATH_HINT: &str =
    "Hint: Use --install-path to specify the StarCraft directory.\n\
     Common paths: /Applications/StarCraft (macOS), ~/.local/share/StarCraft or ~/.wine/drive_c/Program Files/StarCraft (Linux)";

/// Resolve the StarCraft install directory to a UTF-8 string.
fn resolve_install_str(install_path: Option<&Path>) -> Result<String> {
    let install_dir = locate_starcraft(install_path)
        .map_err(|e| anyhow::anyhow!("{}\n{}", e, INSTALL_PATH_HINT))?;
    install_dir
        .into_os_string()
        .into_string()
        .map_err(|p| anyhow::anyhow!("Install path is not valid UTF-8: {:?}", p))
}

/// Open the CascLib-backed archive, resolving the install path.
fn open_casc_archive(install_path: Option<&Path>) -> Result<CascArchive> {
    let install_str = resolve_install_str(install_path)?;
    CascArchive::open(&install_str).map_err(|e| {
        anyhow::anyhow!(
            "Failed to open CASC archive at {}: {}\n{}",
            install_str,
            e,
            INSTALL_PATH_HINT
        )
    })
}

/// Open the index-based CascStorage (used for file enumeration).
fn open_casc_storage(install_path: Option<&Path>) -> Result<CascStorage> {
    let install_str = resolve_install_str(install_path)?;
    CascStorage::open(&install_str).map_err(|e| {
        anyhow::anyhow!(
            "Failed to open CascStorage at {}: {}\n{}",
            install_str,
            e,
            INSTALL_PATH_HINT
        )
    })
}

// ---------------------------------------------------------------------------
// Subcommand handlers
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn cmd_extract_anim(
    archive: &CascArchive,
    output: &Path,
    quality: QualityLevel,
    ids: Option<Vec<u16>>,
    raw: bool,
    name_map: Option<PathBuf>,
    convert_to_png: bool,
    team_color_mask: bool,
    config: &ExtractionConfig,
    layers: Vec<String>,
    save_dds: bool,
) -> Result<()> {
    let session_start = std::time::SystemTime::now();

    fs::create_dir_all(output)?;
    println!("Extracting Animations...");
    println!("Quality:  {}", quality.description());
    println!("Output:   {:?}\n", output);

    let overwrite = config.output_settings.overwrite_behavior;
    let gen_meta  = config.output_settings.metadata_options.generate_json;
    let pixels_per_unit = config.output_settings.unity_settings.pixels_per_unit;

    // SD quality uses a single file rather than the per-ID loop.
    if matches!(quality, QualityLevel::Sd) {
        let casc_path = "SD/mainSD.anim";
        let output_path = output.join("mainSD.anim");
        if !should_skip(&output_path, overwrite, session_start) {
            match archive.extract_file(casc_path) {
                Ok(data) => {
                    File::create(&output_path)?.write_all(&data)?;
                    println!("  mainSD.anim ({:.1} MB)", data.len() as f64 / 1_000_000.0);

                    if convert_to_png {
                        let has_anim_magic = data.starts_with(b"ANIM");
                        if has_anim_magic {
                            // mainSD.anim uses the SD ANIM variant (version 0x0101).
                            // The HD parser (HdAnimFile) expects version 0x0202 or 0x0204 and
                            // explicitly rejects 0x0101 — the SD format uses a different
                            // per-sprite structure with palettised pixel data rather than
                            // DDS texture slabs, and is not yet implemented.
                            match HdAnimFile::parse(&data) {
                                Ok(anim) => {
                                    let export_cfg = ExportConfig {
                                        convert_to_png,
                                        team_color_mask,
                                        save_dds,
                                        generate_metadata: gen_meta,
                                        pixels_per_unit,
                                        layers: layers.clone(),
                                    };
                                    match export_anim(
                                        &anim,
                                        &output_path.with_extension(""),
                                        &export_cfg,
                                    ) {
                                        Ok(result) => println!(
                                            "  mainSD ({} frames) exported as PNG",
                                            result.frame_count
                                        ),
                                        Err(e) => println!("  mainSD ANIM export failed: {}", e),
                                    }
                                }
                                Err(_) => {
                                    // HdAnimFile failed — try the SD ANIM v0x0101 parser.
                                    match casc_extractor::anim::SdAnimFile::parse(&data) {
                                        Ok(sd) => {
                                            println!(
                                                "  mainSD parsed: {} sprites (SD ANIM v0x0101)",
                                                sd.sprite_count
                                            );
                                            // Build a name map from the sprite mapping if available:
                                            // "terran/marine" -> "anim/main_000.anim" => id 0 -> "terran/marine"
                                            let mapping_path = PathBuf::from("mappings/starcraft-remastered.yaml");
                                            let id_to_name: std::collections::HashMap<usize, String> =
                                                if mapping_path.exists() {
                                                    SpriteMapping::load(&mapping_path)
                                                        .map(|m| m.entries.into_iter().filter_map(|(name, casc_path)| {
                                                            // Extract numeric ID from "anim/main_NNN.anim"
                                                            casc_path.trim_end_matches(".anim")
                                                                .rsplit('_').next()
                                                                .and_then(|s| s.parse::<usize>().ok())
                                                                .map(|id| (id, name.replace('/', "_")))
                                                        }).collect())
                                                        .unwrap_or_default()
                                                } else {
                                                    std::collections::HashMap::new()
                                                };

                                            let export_teamcolor = team_color_mask;
                                            let mut exported = 0usize;
                                            for (idx, sprite) in sd.sprites.iter().enumerate() {
                                                if sprite.dds1_data.is_empty() { continue; }
                                                let stem = id_to_name.get(&idx)
                                                    .cloned()
                                                    .unwrap_or_else(|| format!("mainSD_sprite_{:03}", idx));
                                                let sprite_path = output.join(format!("{}.png", stem));
                                                if let Err(e) = casc_extractor::dds_converter::save_dds_as_png(
                                                    &sprite.dds1_data, &sprite_path
                                                ) {
                                                    log::debug!("SD sprite export failed: {}", e);
                                                } else {
                                                    exported += 1;
                                                    // Export team-color layer if requested and available
                                                    if export_teamcolor && !sprite.layer2_data.is_empty() {
                                                        let tc_path = output.join(format!("{}_tc.png", stem));
                                                        if let Err(e) = casc_extractor::dds_converter::save_dds_as_png(
                                                            &sprite.layer2_data, &tc_path
                                                        ) {
                                                            log::debug!("SD TC export failed: {}", e);
                                                        }
                                                    }
                                                }
                                            }
                                            println!("  Exported {} SD sprite PNGs", exported);
                                        }
                                        Err(e) => {
                                            let version = if data.len() >= 6 {
                                                u16::from_le_bytes([data[4], data[5]])
                                            } else { 0 };
                                            println!("  SD ANIM parse error: {}", e);
                                            println!(
                                                "  SD ANIM version: 0x{:04X}",
                                                version
                                            );
                                        }
                                    }
                                }
                            }
                        } else if let Ok(grp) = GrpFile::parse(&data) {
                            let png_path = output.join("mainSD.png");
                            let compression = png_compression(config.quality_settings.png_compression_level);
                            match build_spritesheet(&grp, &png_path, compression) {
                                Ok(()) => println!(
                                    "  mainSD spritesheet PNG ({} frames)",
                                    grp.frame_count
                                ),
                                Err(e) => println!("  mainSD spritesheet failed: {}", e),
                            }
                        } else {
                            let hex: Vec<String> = data
                                .iter()
                                .take(16)
                                .map(|b| format!("{:02X}", b))
                                .collect();
                            println!(
                                "  SD PNG conversion: unknown format (first 16 bytes: {})",
                                hex.join(" ")
                            );
                        }
                    }
                }
                Err(e) => println!("  mainSD.anim - {}", e),
            }
        }
        println!("\nExtraction complete!");
        println!("Files saved to: {:?}", output);
        return Ok(());
    }

    let prefix = quality.path_prefix();

    // Build ID → output name map.
    // Priority: explicit --name-map JSON > built-in YAML mapping > raw numbered names.
    let id_to_name: HashMap<String, String> = if raw {
        HashMap::new()
    } else if let Some(ref p) = name_map {
        let json = fs::read_to_string(p)
            .map_err(|e| anyhow::anyhow!("Failed to read name map {:?}: {}", p, e))?;
        serde_json::from_str::<HashMap<String, String>>(&json)
            .map_err(|e| anyhow::anyhow!("Failed to parse name map {:?}: {}", p, e))?
    } else {
        // Default: parse built-in YAML mapping (animations/category/name: anim/main_NNN.anim)
        // and invert it to produce NNN → category/name.
        let yaml = include_str!("../mappings/starcraft-remastered.yaml");
        let mut map = HashMap::new();
        for line in yaml.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.split('#').next().unwrap_or("").trim();
                // Only animation entries: key = "animations/category/name"
                // value = "anim/main_NNN.anim" (possibly with HD2/ prefix)
                if key.starts_with("animations/") {
                    let name = key.trim_start_matches("animations/");
                    // Extract the numeric ID from the CASC path
                    let casc = value.trim_start_matches("HD2/").trim_start_matches("SD/");
                    if let Some(stem) = casc.strip_prefix("anim/main_").and_then(|s| s.strip_suffix(".anim")) {
                        if let Ok(id) = stem.trim_start_matches('0').parse::<u16>().or_else(|_| stem.parse::<u16>()) {
                            map.insert(id.to_string(), name.to_string());
                        }
                    }
                }
            }
        }
        println!("  Using built-in mapping ({} named entries). Pass --raw for numbered output.", map.len());
        map
    };

    // Build id list, then cap to max_files if set.
    let mut id_list: Vec<u16> = ids.unwrap_or_else(|| (0..1000).collect());
    if let Some(max) = config.filter_settings.max_files {
        id_list.truncate(max as usize);
    }

    // Compile include/exclude regex patterns once upfront.
    let include = compile_patterns(&config.filter_settings.include_patterns);
    let exclude = compile_patterns(&config.filter_settings.exclude_patterns);

    if convert_to_png {
        // -----------------------------------------------------------------------
        // Phase 1 — Sequential: extract from the CASC archive (FFI, non-Send).
        // -----------------------------------------------------------------------
        let mut progress = casc_extractor::ProgressReporter::new(
            id_list.len() as u64,
            config.feedback_settings.verbose_logging,
        );

        let mut extracted: Vec<(u16, PathBuf, Vec<u8>)> = Vec::new();

        for &id in &id_list {
            let casc_path = format!("{}anim/main_{:03}.anim", prefix, id);

            if !passes_filter(&casc_path, &include, &exclude) {
                progress.increment();
                continue;
            }

            progress.update_current_file(&casc_path);

            if let Ok(data) = archive.extract_file(&casc_path) {
                let output_path = output.join(format!("main_{:03}.anim", id));
                if !should_skip(&output_path, overwrite, session_start) {
                    if let Err(e) = File::create(&output_path).and_then(|mut f| f.write_all(&data)) {
                        eprintln!("Warning: could not write {:?}: {}", output_path, e);
                    } else {
                        extracted.push((id, output_path, data));
                    }
                }
            }
            // Silently skip missing anim IDs — not every ID exists.

            progress.increment();
        }

        progress.finish(0, 0);

        // -----------------------------------------------------------------------
        // Phase 2 — Parallel: pure-Rust PNG conversion (no archive access).
        // -----------------------------------------------------------------------
        let id_to_name_ref = &id_to_name;
        let output_ref = output;
        let layers_ref = &layers;

        let results: Vec<anyhow::Result<()>> = extracted
            .par_iter()
            .map(|(id, output_path, data)| -> anyhow::Result<()> {
                let mapped_name = id_to_name_ref.get(&id.to_string()).cloned();
                // When a mapping exists, the named path IS the primary output.
                // Create subdirectories as needed (e.g. terran/marine.anim).
                if let Some(ref name) = mapped_name {
                    let named_path = output_ref.join(format!("{}.anim", name));
                    if let Some(parent) = named_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if named_path != *output_path {
                        if let Err(e) = fs::rename(output_path, &named_path) {
                            // rename may fail across filesystems; fall back to copy+delete
                            if fs::copy(output_path, &named_path).is_ok() {
                                let _ = fs::remove_file(output_path);
                            } else {
                                eprintln!("Warning: could not move to named path {:?}: {}", named_path, e);
                            }
                        }
                    }
                }

                match HdAnimFile::parse(data) {
                    Ok(anim) => {
                        let export_cfg = ExportConfig {
                            convert_to_png: true,
                            team_color_mask,
                            save_dds,
                            generate_metadata: gen_meta,
                            pixels_per_unit,
                            layers: layers_ref.clone(),
                        };
                        match export_anim(&anim, &output_path.with_extension(""), &export_cfg) {
                            Ok(result) => {
                                if let Some(ref name) = mapped_name {
                                    println!(
                                        "  {}.anim ({}, {} frames, {:.1} MB) tc={}",
                                        name,
                                        output_path.display(),
                                        result.frame_count,
                                        data.len() as f64 / 1_000_000.0,
                                        result.tc_mask_written
                                    );
                                } else {
                                    println!(
                                        "  {} ({} frames, {:.1} MB) tc={}",
                                        output_path.display(),
                                        result.frame_count,
                                        data.len() as f64 / 1_000_000.0,
                                        result.tc_mask_written
                                    );
                                }
                                Ok(())
                            }
                            Err(e) => Err(anyhow::anyhow!("  Export failed: {}", e)),
                        }
                    }
                    Err(e) => Err(anyhow::anyhow!("  {} - Parse error: {}", output_path.display(), e)),
                }
            })
            .collect();

        for result in results {
            if let Err(e) = result {
                println!("  Export failed: {}", e);
            }
        }
    } else {
        // -----------------------------------------------------------------------
        // Non-PNG path: sequential write, fast enough without parallelism.
        // -----------------------------------------------------------------------
        let mut progress = casc_extractor::ProgressReporter::new(
            id_list.len() as u64,
            config.feedback_settings.verbose_logging,
        );

        for &id in &id_list {
            let casc_path = format!("{}anim/main_{:03}.anim", prefix, id);

            if !passes_filter(&casc_path, &include, &exclude) {
                progress.increment();
                continue;
            }

            progress.update_current_file(&casc_path);

            if let Ok(data) = archive.extract_file(&casc_path) {
                let output_name = format!("main_{:03}.anim", id);
                let output_path = output.join(&output_name);

                if should_skip(&output_path, overwrite, session_start) {
                    progress.increment();
                    continue;
                }

                File::create(&output_path)?.write_all(&data)?;

                let mapped_name = id_to_name.get(&id.to_string()).cloned();
                if let Some(ref name) = mapped_name {
                    let named_path = output.join(format!("{}.anim", name));
                    if let Err(e) = fs::copy(&output_path, &named_path) {
                        eprintln!("Warning: could not write named copy {:?}: {}", named_path, e);
                    }
                    println!(
                        "  {}.anim ({}, {:.1} MB)",
                        name,
                        output_path.display(),
                        data.len() as f64 / 1_000_000.0
                    );
                } else {
                    println!(
                        "  {} ({:.1} MB)",
                        output_path.display(),
                        data.len() as f64 / 1_000_000.0
                    );
                }
            }
            // Silently skip missing anim IDs — not every ID exists.

            progress.increment();
        }

        progress.finish(0, 0);
    }

    println!("\nExtraction complete!");
    println!("Files saved to: {:?}", output);
    Ok(())
}

fn cmd_extract_tileset(
    archive: &CascArchive,
    output: &Path,
    quality: QualityLevel,
    convert_to_png: bool,
    config: &ExtractionConfig,
) -> Result<()> {
    let session_start = std::time::SystemTime::now();

    fs::create_dir_all(output)?;
    println!("Extracting Tilesets...");
    println!("Quality: {}", quality.description());
    println!("Output:  {:?}\n", output);

    let prefix = quality.path_prefix();
    let overwrite = config.output_settings.overwrite_behavior;
    let tilesets = [
        "badlands", "platform", "ashworld", "jungle",
        "desert", "ice", "twilight", "install",
    ];

    // Phase 1 — Sequential: extract raw bytes from the archive (FFI, non-Send).
    let mut extracted: Vec<(String, PathBuf, Vec<u8>)> = Vec::new();
    for tileset in &tilesets {
        let output_name = format!("{}.dds.vr4", tileset);
        let output_path = output.join(&output_name);
        if should_skip(&output_path, overwrite, session_start) {
            println!("  {} (skipped)", output_path.display());
            continue;
        }
        let path = format!("{}tileset/{}.dds.vr4", prefix, tileset);
        match archive.extract_file(&path) {
            Ok(data) => extracted.push((output_name.clone(), output_path, data)),
            Err(e) => println!("  {} - {}", tileset, e),
        }
    }

    // Phase 2 — Parallel: write files to disk (I/O-bound, no archive access).
    let results: Vec<anyhow::Result<()>> = extracted
        .par_iter()
        .map(|(_output_name, output_path, data)| -> anyhow::Result<()> {
            File::create(output_path)?.write_all(data)?;
            let mut note = String::new();
            if convert_to_png {
                // .dds.vr4 files have a 20-byte VR4 header before the DDS data.
                // Search within the first 64 bytes for the DDS magic.
                let dds_offset = data.windows(4).take(64)
                    .position(|w| w == b"DDS ");
                if let Some(offset) = dds_offset {
                    let png_path = output_path.with_extension("png");
                    match casc_extractor::dds_converter::save_dds_as_png(&data[offset..], &png_path) {
                        Ok(()) => note = " → PNG".to_string(),
                        Err(e) => note = format!(" (PNG failed: {})", e),
                    }
                }
            }
            println!("  {} ({:.1} MB){}", output_path.display(), data.len() as f64 / 1_000_000.0, note);
            Ok(())
        })
        .collect();

    for result in results {
        if let Err(e) = result {
            println!("  Write failed: {}", e);
        }
    }

    println!("\nDone. Files saved to: {:?}", output);
    Ok(())
}

fn cmd_extract_effect(
    archive: &CascArchive,
    output: &Path,
    quality: QualityLevel,
    convert_to_png: bool,
    config: &ExtractionConfig,
) -> Result<()> {
    let session_start = std::time::SystemTime::now();

    fs::create_dir_all(output)?;
    println!("Extracting Effects...");
    println!("Quality: {}", quality.description());
    println!("Output:  {:?}\n", output);

    let prefix = quality.path_prefix();
    let overwrite = config.output_settings.overwrite_behavior;
    let effects = ["water_normal_1.dds.grp", "water_normal_2.dds.grp"];

    // Phase 1 — Sequential: extract raw bytes from the archive (FFI, non-Send).
    let mut extracted: Vec<(String, PathBuf, Vec<u8>)> = Vec::new();
    for effect in &effects {
        let output_path = output.join(effect);
        if should_skip(&output_path, overwrite, session_start) {
            println!("  {} (skipped)", effect);
            continue;
        }
        let path = format!("{}effect/{}", prefix, effect);
        match archive.extract_file(&path) {
            Ok(data) => extracted.push((effect.to_string(), output_path, data)),
            Err(e) => println!("  {} - {}", effect, e),
        }
    }

    let compression = png_compression(config.quality_settings.png_compression_level);

    // Phase 2 — Parallel: write files to disk (I/O-bound, no archive access).
    let results: Vec<anyhow::Result<()>> = extracted
        .par_iter()
        .map(|(_output_name, output_path, data)| -> anyhow::Result<()> {
            File::create(output_path)?.write_all(data)?;
            let mut note = String::new();
            if convert_to_png {
                let png_path = output_path.with_extension("png");
                if data.starts_with(b"DDS ") {
                    match casc_extractor::dds_converter::save_dds_as_png(data, &png_path) {
                        Ok(()) => note = " → PNG".to_string(),
                        Err(e) => note = format!(" (PNG failed: {})", e),
                    }
                } else if let Ok(grp) = GrpFile::parse(data) {
                    match build_spritesheet(&grp, &png_path, compression) {
                        Ok(()) => note = format!(" → PNG spritesheet ({} frames)", grp.frame_count),
                        Err(e) => note = format!(" (PNG failed: {})", e),
                    }
                }
            }
            println!("  {} ({:.1} MB){}", output_path.display(), data.len() as f64 / 1_000_000.0, note);
            Ok(())
        })
        .collect();

    for result in results {
        if let Err(e) = result {
            println!("  Write failed: {}", e);
        }
    }

    println!("\nDone. Files saved to: {:?}", output);
    Ok(())
}

/// Build a spritesheet PNG from GRP frames and write it to `output_path`.
fn build_spritesheet(grp: &GrpFile, output_path: &Path, compression: png::Compression) -> Result<()> {
    let frames_per_row = 17usize;
    let rows = (grp.frame_count as usize).div_ceil(frames_per_row);
    let sheet_width = grp.width as u32 * frames_per_row as u32;
    let sheet_height = grp.height as u32 * rows as u32;

    let mut sheet_data = vec![0u8; (sheet_width * sheet_height * 4) as usize];

    for (idx, frame) in grp.frames.iter().enumerate() {
        if let Ok(rgba) = frame.to_rgba() {
            let row = idx / frames_per_row;
            let col = idx % frames_per_row;
            let x_offset = col * grp.width as usize;
            let y_offset = row * grp.height as usize;

            for y in 0..frame.height as usize {
                for x in 0..frame.width as usize {
                    let src_idx = (y * frame.width as usize + x) * 4;
                    let dst_x = x_offset + x;
                    let dst_y = y_offset + y;
                    let dst_idx = (dst_y * sheet_width as usize + dst_x) * 4;
                    sheet_data[dst_idx..dst_idx + 4]
                        .copy_from_slice(&rgba[src_idx..src_idx + 4]);
                }
            }
        }
    }

    let file = File::create(output_path)?;
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, sheet_width, sheet_height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(compression);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&sheet_data)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
/// Scan the archive file listing for an audio entry whose name contains all
/// words in `stem_hint` (case-insensitive), and try to extract the first match.
fn discover_sound(archive: &CascArchive, storage: &CascStorage, stem_hint: &str) -> Option<Vec<u8>> {
    let files = storage.list_files().ok()?;
    let hint_lower = stem_hint.to_lowercase();
    // Split hint on '_' to get keywords (e.g. "zergling_attack" -> ["zergling", "att"])
    let keywords: Vec<String> = hint_lower
        .split('_')
        .map(|w| w[..w.len().min(4)].to_string())
        .collect();

    let candidate = files.iter().find(|f| {
        let fl = f.to_lowercase();
        (fl.ends_with(".wav") || fl.ends_with(".ogg"))
            && keywords.iter().all(|kw| fl.contains(kw.as_str()))
    })?;

    archive.extract_file(candidate).ok().filter(|d| !d.is_empty())
}

fn cmd_sounds_extract(
    archive: &CascArchive,
    storage: &CascStorage,
    output: &Path,
    targets_path: Option<&Path>,
) -> Result<()> {
    println!("  StarCraft Sound Extractor");
    println!("==================================================");
    println!("  Opened CASC archive");

    fs::create_dir_all(output)?;

    // Load custom targets from JSON if provided, otherwise use the built-in list.
    // `custom_targets` owns the heap-allocated strings when a JSON file is used;
    // it must live as long as `targets` to avoid dangling references.
    let custom_targets: Vec<(String, Vec<String>)> = if let Some(path) = targets_path {
        let raw = fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read targets file {:?}: {}", path, e))?;
        let entries: Vec<SoundTargetEntry> = serde_json::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("Failed to parse targets file {:?}: {}", path, e))?;
        entries
            .into_iter()
            .map(|e| (e.output, e.candidates))
            .collect()
    } else {
        Vec::new()
    };

    // Build a uniform `Vec<(&str, Vec<&str>)>` view regardless of source.
    let targets: Vec<(&str, Vec<&str>)> = if targets_path.is_some() {
        custom_targets
            .iter()
            .map(|(out, cands)| (out.as_str(), cands.iter().map(|s| s.as_str()).collect()))
            .collect()
    } else {
        SOUND_TARGETS
            .iter()
            .map(|(out, cands)| (*out, cands.to_vec()))
            .collect()
    };

    let total = targets.len();
    let mut extracted = 0usize;

    for (out_name, candidates) in &targets {
        let dest = output.join(out_name);
        if dest.exists() {
            println!("  {} already exists, skipping", out_name);
            extracted += 1;
            continue;
        }

        let mut found = false;
        for casc_path in candidates {
            let variants = [
                casc_path.to_string(),
                casc_path.replace('\\', "/"),
            ];
            for variant in &variants {
                match archive.extract_file(variant) {
                    Ok(data) if !data.is_empty() => {
                        fs::write(&dest, &data)?;
                        println!(
                            "  {:>7} bytes  {}  ->  {}",
                            data.len(),
                            variant,
                            out_name
                        );
                        found = true;
                        extracted += 1;
                        break;
                    }
                    _ => {}
                }
            }
            if found {
                break;
            }
        }

        if !found {
            // Fall back to dynamic discovery via the file listing.
            let stem = out_name.trim_end_matches(".ogg").trim_end_matches(".wav");
            if let Some(data) = discover_sound(archive, storage, stem) {
                fs::write(&dest, &data)?;
                println!(
                    "  {:>7} bytes  (discovered)  ->  {}",
                    data.len(),
                    out_name
                );
                extracted += 1;
            } else {
                println!(
                    "  {} -- none of {} candidates succeeded (discovery also failed)",
                    out_name,
                    candidates.len()
                );
            }
        }
    }

    println!(
        "\n== Result ==================================================\n  \
         {}/{} sounds extracted to {}",
        extracted,
        total,
        output.display()
    );
    println!(
        "\nIf any are missing, check exact paths with:\n  \
         casc-extractor sounds list"
    );
    Ok(())
}

fn cmd_sounds_export_targets(output: &Path) -> Result<()> {
    let entries: Vec<SoundTargetEntry> = SOUND_TARGETS
        .iter()
        .map(|(out, cands)| SoundTargetEntry {
            output: out.to_string(),
            candidates: cands.iter().map(|s| s.to_string()).collect(),
        })
        .collect();
    let json = serde_json::to_string_pretty(&entries)
        .map_err(|e| anyhow::anyhow!("Failed to serialize targets: {}", e))?;
    fs::write(output, json.as_bytes())
        .map_err(|e| anyhow::anyhow!("Failed to write targets to {:?}: {}", output, e))?;
    println!("Sound targets written to {:?}", output);
    println!("Edit the file then use:  casc-extractor sounds extract --targets {:?}", output);
    Ok(())
}

fn cmd_sounds_list(install_path: Option<&Path>, search: Option<String>) -> Result<()> {
    if let Some(ref pattern) = search {
        // Search mode: open the archive, list all files, filter by pattern (no cap).
        let storage = open_casc_storage(install_path)?;
        let files = storage
            .list_files()
            .map_err(|e| anyhow::anyhow!("list_files failed: {}", e))?;

        let pattern_lower = pattern.to_lowercase();
        let matches: Vec<_> = files
            .iter()
            .filter(|f| f.to_lowercase().contains(&pattern_lower))
            .collect();

        println!("Search results for {:?}:", pattern);
        for f in &matches {
            println!("  {}", f);
        }
        println!("\n{} match(es) found.", matches.len());
        return Ok(());
    }

    // Default mode: probe known paths + enumerate Zerg/UI audio.
    let archive = open_casc_archive(install_path)?;
    println!("  Opened archive\n");

    // Probe a representative set of paths to show what works
    let probes = [
        "enUS\\Assets\\sound\\Zerg\\Zergling\\ZlDth00.wav",
        "enUS\\Assets\\sound\\zerg\\zergling\\zldth00.wav",
        "enUS\\Assets\\sound\\Zerg\\Zergling\\ZlAtt00.wav",
        "NOLA\\Assets\\sound\\Zerg\\Zergling\\ZlDth00.wav",
        "NOLA\\Assets\\sound\\Zerg\\Zergling\\ZlAtt00.wav",
        "Assets\\sound\\Zerg\\Zergling\\ZlDth00.wav",
        "Assets\\sound\\Zerg\\Zergling\\ZlAtt00.wav",
        "sound\\Zerg\\Zergling\\ZlDth00.wav",
        "sound\\zerg\\zergling\\zldth00.wav",
        "sound/Zerg/Zergling/ZlDth00.wav",
        "SD\\sound\\Zerg\\Zergling\\ZlDth00.wav",
        "NOLA\\sound\\Zerg\\Zergling\\ZlDth00.wav",
        "sound\\Misc\\select.wav",
        "sound/Misc/select.wav",
        "SD\\sound\\Misc\\select.wav",
        "sound\\Glue\\select.wav",
        "sound/Glue/select.wav",
        "sound\\Misc\\mousedown.wav",
        "sound/misc/mousedown.wav",
        "sound\\Misc\\klink.wav",
        "sound/misc/klink.wav",
    ];

    for p in &probes {
        match archive.extract_file(p) {
            Ok(data) if !data.is_empty() => {
                println!("  OK {:>8} bytes  {}", data.len(), p)
            }
            _ => println!("  --              {}", p),
        }
    }

    // Use CascStorage to enumerate Zerg and UI audio files
    println!("\nListing Zerg audio from archive...");
    let storage = open_casc_storage(install_path)?;
    let files = storage
        .list_files()
        .map_err(|e| anyhow::anyhow!("list_files failed: {}", e))?;

    let zerg_audio: Vec<_> = files
        .iter()
        .filter(|f| {
            let lower = f.to_lowercase();
            (lower.contains("zerg")
                || lower.contains("\\zl")
                || lower.contains("/zl"))
                && (lower.ends_with(".wav") || lower.ends_with(".ogg"))
        })
        .collect();

    println!("Found {} Zerg audio paths:", zerg_audio.len());
    for f in zerg_audio.iter().take(30) {
        println!("  {}", f);
    }

    println!("\nListing Misc/Glue UI sounds...");
    let ui_audio: Vec<_> = files
        .iter()
        .filter(|f| {
            let lower = f.to_lowercase();
            (lower.contains("misc")
                || lower.contains("glue")
                || lower.contains("\\ui"))
                && (lower.ends_with(".wav") || lower.ends_with(".ogg"))
        })
        .collect();
    println!("Found {} UI audio paths:", ui_audio.len());
    for f in ui_audio.iter().take(30) {
        println!("  {}", f);
    }

    Ok(())
}

fn cmd_inspect_sprites(
    archive: &CascArchive,
    quality: QualityLevel,
    max_id: u16,
) -> Result<()> {
    println!(
        "Scanning anim files at quality {}...\n",
        quality.description()
    );

    // SD quality has a single combined file, not per-ID files.
    if matches!(quality, QualityLevel::Sd) {
        let path = "SD/mainSD.anim";
        match archive.extract_file(path) {
            Ok(data) => println!("  found  mainSD.anim  ({:.1} MB)", data.len() as f64 / 1_000_000.0),
            Err(_)   => println!("  not found: {}", path),
        }
        return Ok(());
    }

    let prefix = quality.path_prefix();
    let mut found = 0usize;

    for id in 0..max_id {
        let path = format!("{}anim/main_{:03}.anim", prefix, id);
        if let Ok(data) = archive.extract_file(&path) {
            println!(
                "  {:>4}  main_{:03}.anim  ({:.1} MB)",
                id,
                id,
                data.len() as f64 / 1_000_000.0
            );
            found += 1;
        }
    }

    println!(
        "\nFound {} anim files out of {} IDs probed.",
        found, max_id
    );
    Ok(())
}

fn cmd_inspect_archive(install_path: Option<&Path>) -> Result<()> {
    let install_dir = locate_starcraft(install_path)?;
    println!("StarCraft installation: {:?}", install_dir);

    let install_str = install_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Install path is not valid UTF-8"))?;

    // CascStorage gives us file enumeration
    match CascStorage::open(install_str) {
        Ok(storage) => match storage.list_files() {
            Ok(files) => {
                println!("Archive file count: {}", files.len());
                println!("\nFirst 10 entries:");
                for f in files.iter().take(10) {
                    println!("  {}", f);
                }
            }
            Err(e) => println!("Could not list files: {}", e),
        },
        Err(e) => println!("CascStorage unavailable: {}", e),
    }

    // Verify the CascLib-backed archive opens too
    match CascArchive::open(install_str) {
        Ok(_) => println!("\nCASC archive opened successfully."),
        Err(e) => println!("\nCASC archive open error: {}", e),
    }

    Ok(())
}

fn cmd_config_init(output: &Path) -> Result<()> {
    let cfg = ExtractionConfig::default();
    let json = serde_json::to_string_pretty(&cfg)
        .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;
    fs::write(output, json.as_bytes())
        .map_err(|e| anyhow::anyhow!("Failed to write config to {:?}: {}", output, e))?;
    println!("Config written to {:?}", output);
    Ok(())
}

fn cmd_validate_register(file: &Path, suite_path: &Path) -> Result<()> {
    // Compute SHA256 of the file
    let mut f = File::open(file)
        .map_err(|e| anyhow::anyhow!("Cannot open file {:?}: {}", file, e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hash = format!("{:x}", hasher.finalize());

    // Load or create suite
    let entries: Vec<KnownGoodExtraction> = if suite_path.exists() {
        let content = fs::read_to_string(suite_path)
            .map_err(|e| anyhow::anyhow!("Cannot read suite {:?}: {}", suite_path, e))?;
        serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Cannot parse suite {:?}: {}", suite_path, e))?
    } else {
        Vec::new()
    };

    let sprite_name = file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let timestamp = chrono::Utc::now().to_rfc3339();

    let entry = KnownGoodExtraction {
        sprite_name: sprite_name.clone(),
        source_file: file.to_path_buf(),
        expected_output: file.to_path_buf(),
        expected_metadata: RegressionSpriteMetadata {
            width: 0,
            height: 0,
            frame_count: 0,
            format: String::new(),
        },
        sha256_hash: hash.clone(),
        baseline_date: timestamp,
        extractor_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    // Replace existing entry with same name, or append
    let mut entries = entries;
    if let Some(pos) = entries.iter().position(|e| e.sprite_name == sprite_name) {
        entries[pos] = entry;
    } else {
        entries.push(entry);
    }

    let json = serde_json::to_string_pretty(&entries)
        .map_err(|e| anyhow::anyhow!("Cannot serialize suite: {}", e))?;
    fs::write(suite_path, json.as_bytes())
        .map_err(|e| anyhow::anyhow!("Cannot write suite {:?}: {}", suite_path, e))?;

    println!(
        "Registered {} (SHA256: {}) in {}",
        sprite_name,
        hash,
        suite_path.display()
    );
    Ok(())
}

fn cmd_validate_run(dir: &Path, suite_path: &Path, json_output: bool) -> Result<()> {
    if !suite_path.exists() {
        if json_output {
            println!("{}", serde_json::json!({"total": 0, "passed": 0, "results": []}));
        } else {
            println!("No suite found at {}", suite_path.display());
        }
        return Ok(());
    }

    let content = fs::read_to_string(suite_path)
        .map_err(|e| anyhow::anyhow!("Cannot read suite {:?}: {}", suite_path, e))?;
    let entries: Vec<KnownGoodExtraction> = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Cannot parse suite {:?}: {}", suite_path, e))?;

    let total = entries.len();
    let mut passed = 0usize;
    let mut json_results: Vec<serde_json::Value> = Vec::new();

    for entry in &entries {
        let filename = entry.expected_output
            .file_name()
            .unwrap_or(entry.expected_output.as_os_str());
        let candidate = dir.join(filename);

        if !candidate.exists() {
            if json_output {
                json_results.push(serde_json::json!({
                    "file": entry.sprite_name,
                    "status": "missing"
                }));
            } else {
                println!("MISSING: {}", entry.sprite_name);
            }
            continue;
        }

        // Compute SHA256 of the candidate file.
        let mut f = File::open(&candidate)
            .map_err(|e| anyhow::anyhow!("Cannot open {:?}: {}", candidate, e))?;
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let actual_hash = format!("{:x}", hasher.finalize());

        if actual_hash == entry.sha256_hash {
            if json_output {
                json_results.push(serde_json::json!({
                    "file": entry.sprite_name,
                    "status": "pass"
                }));
            } else {
                println!("PASS: {}", entry.sprite_name);
            }
            passed += 1;
        } else if json_output {
            json_results.push(serde_json::json!({
                "file": entry.sprite_name,
                "status": "fail",
                "expected": entry.sha256_hash,
                "actual": actual_hash
            }));
        } else {
            println!(
                "FAIL: {} (expected {}, got {})",
                entry.sprite_name, entry.sha256_hash, actual_hash
            );
        }
    }

    if json_output {
        let report = serde_json::json!({
            "total": total,
            "passed": passed,
            "results": json_results
        });
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("\n{}/{} checks passed", passed, total);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_extract_tileset_megatile(
    archive: &CascArchive,
    tileset: &str,
    _quality: QualityLevel,
    max_tiles: u32,
    tile_size: u32,
    cols: u32,
    output: &Path,
) -> Result<()> {
    use image::{ImageBuffer, RgbaImage};

    // 1. Load VR4 pixel data — always use HD4 (no prefix) because VX4EX
    //    stores indices into the full HD4 tile array (up to 14316 tiles).
    //    Using HD2 (~3759 tiles) would leave most indices out of range.
    let vr4_path = format!("tileset/{}.dds.vr4", tileset);
    let vr4_raw = archive.extract_file(&vr4_path)
        .with_context(|| format!("VR4 not found: {}", vr4_path))?;

    // DDS header: 128×128 DXT1 tiles, 8192 bytes each. Each VR4 entry IS a full megatile.
    const VR4_HEADER: usize = 148;
    const TILE_STRIDE: usize = 8192;
    const TILE_W: u32 = 128;
    const TILE_H: u32 = 128;
    let vr4_data = &vr4_raw[VR4_HEADER..];
    let vr4_tile_count = vr4_data.len() / TILE_STRIDE;
    println!("VR4 tiles: {}", vr4_tile_count);

    // 2. Load VX4EX — 1 entry per megatile (4 bytes): vr4_idx = entry>>1, flip_x = entry&1
    let vx4_path = format!("tileset/{}.vx4ex", tileset);
    let vx4_raw = archive.extract_file(&vx4_path)
        .with_context(|| format!("VX4EX not found: {}", vx4_path))?;

    let n_megatiles_vx4 = vx4_raw.len() / 4; // 4 bytes per megatile entry
    println!("VX4EX megatiles: {}", n_megatiles_vx4);

    let n_megatiles = if max_tiles == 0 {
        n_megatiles_vx4 as u32
    } else {
        max_tiles.min(n_megatiles_vx4 as u32)
    };

    let rows = n_megatiles.div_ceil(cols);
    let atlas_w = cols * tile_size;
    let atlas_h = rows * tile_size;
    let mut atlas: RgbaImage = ImageBuffer::new(atlas_w, atlas_h);

    println!("Building megatile atlas: {}×{} ({} megatiles)", atlas_w, atlas_h, n_megatiles);

    for mega_idx in 0..n_megatiles as usize {
        let vx4_offset = mega_idx * 4;
        if vx4_offset + 4 > vx4_raw.len() { continue; }
        let entry = u32::from_le_bytes([
            vx4_raw[vx4_offset], vx4_raw[vx4_offset+1],
            vx4_raw[vx4_offset+2], vx4_raw[vx4_offset+3],
        ]);
        let vr4_idx = (entry >> 1) as usize;
        let flip_x  = (entry & 1) == 1;

        if vr4_idx >= vr4_tile_count { continue; }

        let tile_bytes = &vr4_data[vr4_idx * TILE_STRIDE .. vr4_idx * TILE_STRIDE + TILE_STRIDE];
        let decoded = decode_dxt1_nxn(tile_bytes, TILE_W as usize, TILE_H as usize);

        let mut mini: RgbaImage = ImageBuffer::from_raw(TILE_W, TILE_H, decoded)
            .unwrap_or_else(|| ImageBuffer::new(TILE_W, TILE_H));
        if flip_x { mini = image::imageops::flip_horizontal(&mini); }

        // Scale 128×128 → tile_size×tile_size for the final atlas
        let mega_scaled = if tile_size == TILE_W {
            mini
        } else {
            image::imageops::resize(&mini, tile_size, tile_size, image::imageops::FilterType::Lanczos3)
        };

        let ax = ((mega_idx as u32 % cols) * tile_size) as i64;
        let ay = ((mega_idx as u32 / cols) * tile_size) as i64;
        image::imageops::overlay(&mut atlas, &mega_scaled, ax, ay);
    }

    fs::create_dir_all(output)?;
    let png_name = format!("{}_megatile_{}.png", tileset, tile_size);
    let png_path = output.join(&png_name);
    atlas.save(&png_path)?;
    println!("Saved: {:?}", png_path);

    let meta = serde_json::json!({
        "tileset": tileset,
        "tile_size": tile_size,
        "cols": cols,
        "rows": rows,
        "n_megatiles": n_megatiles,
        "atlas_width": atlas_w,
        "atlas_height": atlas_h,
    });
    let json_path = output.join(format!("{}_megatile_{}.json", tileset, tile_size));
    std::fs::write(&json_path, serde_json::to_string_pretty(&meta)?)?;

    Ok(())
}

/// Decode a 64×64 DXT1 (BC1) compressed tile to raw RGBA pixels.
/// Layer 0 of each VR4 stride slot = DXT1 diffuse color data.
fn decode_dxt1_nxn(data: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut pixels = vec![0u8; w * h * 4];
    let mut block_idx = 0usize;
    for by in (0..h).step_by(4) {
        for bx in (0..w).step_by(4) {
            let base = block_idx * 8;
            if base + 8 > data.len() { break; }
            let c0_raw = u16::from_le_bytes([data[base], data[base+1]]);
            let c1_raw = u16::from_le_bytes([data[base+2], data[base+3]]);
            let bits = u32::from_le_bytes([data[base+4], data[base+5], data[base+6], data[base+7]]);
            let c0 = rgb565_to_rgba(c0_raw);
            let c1 = rgb565_to_rgba(c1_raw);
            let colors: [[u8;4]; 4] = if c0_raw > c1_raw {
                [c0, c1, lerp_color(c0, c1, 1, 3), lerp_color(c0, c1, 2, 3)]
            } else {
                [c0, c1, lerp_color(c0, c1, 1, 2), [0, 0, 0, 0]]
            };
            for py in 0..4 {
                for px in 0..4 {
                    let bit_shift = (py * 4 + px) * 2;
                    let color_idx = ((bits >> bit_shift) & 0x3) as usize;
                    let dst = ((by + py) * w + (bx + px)) * 4;
                    if dst + 4 <= pixels.len() {
                        pixels[dst..dst+4].copy_from_slice(&colors[color_idx]);
                    }
                }
            }
            block_idx += 1;
        }
    }
    pixels
}

#[inline]
fn rgb565_to_rgba(c: u16) -> [u8; 4] {
    let r = ((c >> 11) & 0x1f) as u8;
    let g = ((c >> 5)  & 0x3f) as u8;
    let b = ( c        & 0x1f) as u8;
    [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2), 255]
}

#[inline]
fn lerp_color(a: [u8;4], b: [u8;4], num: u32, den: u32) -> [u8;4] {
    [
        ((a[0] as u32 * (den-num) + b[0] as u32 * num) / den) as u8,
        ((a[1] as u32 * (den-num) + b[1] as u32 * num) / den) as u8,
        ((a[2] as u32 * (den-num) + b[2] as u32 * num) / den) as u8,
        255,
    ]
}

#[allow(clippy::too_many_arguments)]
fn cmd_extract_tileset_atlas(
    archive: &CascArchive,
    tileset: &str,
    quality: QualityLevel,
    cols: u32,
    max_tiles: u32,
    tile_size: u32,
    output: &Path,
) -> Result<()> {
    use image::{ImageBuffer, RgbaImage};

    let prefix = quality.path_prefix();
    let path = format!("{}tileset/{}.dds.vr4", prefix, tileset);
    let data = archive.extract_file(&path)
        .with_context(|| format!("Failed to extract {}", path))?;

    // 20-byte VR4 header + 128-byte DDS header = 148 bytes to skip
    if data.len() < 148 {
        anyhow::bail!("VR4 file too small: {} bytes", data.len());
    }
    let pixel_data = &data[148..];

    // DDS header: 128×128 DXT1 tiles, 8192 bytes each
    const TILE_STRIDE: usize = 8192;
    const TILE_W: u32 = 128;
    const TILE_H: u32 = 128;

    let total_tiles = (pixel_data.len() / TILE_STRIDE) as u32;
    let n_tiles = if max_tiles == 0 { total_tiles } else { max_tiles.min(total_tiles) };

    println!("Tileset: {} ({} total tiles, extracting {})", tileset, total_tiles, n_tiles);

    let rows = n_tiles.div_ceil(cols);
    let atlas_w = cols * tile_size;
    let atlas_h = rows * tile_size;

    let mut atlas: RgbaImage = ImageBuffer::new(atlas_w, atlas_h);

    for i in 0..n_tiles {
        let offset = i as usize * TILE_STRIDE;
        if offset + TILE_STRIDE > pixel_data.len() { break; }
        let tile_bytes = &pixel_data[offset..offset + TILE_STRIDE];

        let decoded = decode_dxt1_nxn(tile_bytes, TILE_W as usize, TILE_H as usize);

        let tile_img: RgbaImage = if tile_size == TILE_W {
            ImageBuffer::from_raw(TILE_W, TILE_H, decoded)
                .ok_or_else(|| anyhow::anyhow!("Failed to create tile image"))?
        } else {
            let src: RgbaImage = ImageBuffer::from_raw(TILE_W, TILE_H, decoded)
                .ok_or_else(|| anyhow::anyhow!("Failed to create tile image"))?;
            image::imageops::resize(&src, tile_size, tile_size, image::imageops::FilterType::Lanczos3)
        };

        let ax = (i % cols) * tile_size;
        let ay = (i / cols) * tile_size;
        image::imageops::overlay(&mut atlas, &tile_img, ax as i64, ay as i64);
    }

    fs::create_dir_all(output)?;

    let filename = format!("{}_{}_atlas.png", tileset, tile_size);
    let out_path = output.join(&filename);
    atlas.save(&out_path)?;
    println!("Saved: {:?} ({}x{}, {} tiles)", out_path, atlas_w, atlas_h, n_tiles);

    let meta = serde_json::json!({
        "tileset": tileset,
        "tile_size": tile_size,
        "cols": cols,
        "rows": rows,
        "total_tiles": n_tiles,
        "atlas_width": atlas_w,
        "atlas_height": atlas_h,
    });
    let meta_path = output.join(format!("{}_{}_atlas.json", tileset, tile_size));
    fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;

    Ok(())
}

fn cmd_extract_map_terrain(
    archive: &CascArchive,
    scm_path: &Path,
    cols: u32,
    tile_size: u32,
    output: &Path,
) -> Result<()> {
    use image::{ImageBuffer, RgbaImage};
    use mpq::Archive;

    fs::create_dir_all(output)?;

    // 1. Parse SCM to get ERA (tileset) and MTXM (megatile IDs)
    let mut archive_mpq = Archive::open(scm_path)
        .with_context(|| format!("Failed to open MPQ: {:?}", scm_path))?;

    // Try each path variant; open_file returns File which requires archive to read.
    let chk_file = archive_mpq.open_file("staredit\\scenario.chk")
        .or_else(|_| archive_mpq.open_file("staredit/scenario.chk"))
        .or_else(|_| archive_mpq.open_file("scenario.chk"))
        .with_context(|| "CHK not found in MPQ")?;

    let chk_size = chk_file.size() as usize;
    let mut chk_bytes = vec![0u8; chk_size];
    chk_file.read(&mut archive_mpq, &mut chk_bytes)
        .context("Failed to read CHK")?;

    // Parse CHK chunks
    let mut era: u16 = 3; // default jungle
    let mut dim_w: u16 = 128;
    let mut dim_h: u16 = 128;
    let mut mtxm: Vec<u16> = Vec::new();

    let mut pos = 0;
    while pos + 8 <= chk_bytes.len() {
        let tag = &chk_bytes[pos..pos+4];
        let size = u32::from_le_bytes(chk_bytes[pos+4..pos+8].try_into().unwrap()) as usize;
        pos += 8;
        if pos + size > chk_bytes.len() { break; }
        let data = &chk_bytes[pos..pos+size];

        match tag {
            b"ERA " if size >= 2 => {
                era = u16::from_le_bytes(data[0..2].try_into().unwrap()) & 7;
            }
            b"DIM " if size >= 4 => {
                dim_w = u16::from_le_bytes(data[0..2].try_into().unwrap());
                dim_h = u16::from_le_bytes(data[2..4].try_into().unwrap());
            }
            b"MTXM" => {
                for i in (0..size).step_by(2) {
                    if i+2 <= size {
                        mtxm.push(u16::from_le_bytes(data[i..i+2].try_into().unwrap()));
                    }
                }
            }
            _ => {}
        }
        pos += size;
    }

    println!("Map: {}×{} tiles, ERA={} (tileset), {} MTXM entries", dim_w, dim_h, era, mtxm.len());

    // Tileset name from ERA
    let tileset_name = match era {
        0 => "badlands",
        1 => "platform",
        2 => "ashworld",
        3 => "jungle",
        4 => "jungle",
        5 => "desert",
        6 => "ice",
        7 => "twilight",
        _ => "jungle",
    };

    // 2. Collect all unique megatile IDs. No cap needed — we use a single atlas PNG,
    // not a GPU texture array, so there is no hardware layer limit.
    let mut unique_ids: Vec<u16> = mtxm.iter().copied()
        .collect::<std::collections::HashSet<u16>>()
        .into_iter()
        .collect();
    unique_ids.sort_unstable(); // stable atlas order
    let n_tiles = unique_ids.len() as u32;
    println!("Unique megatile IDs: {}", n_tiles);

    // 3. Load CV5, VX4EX, classic VR4 (8×8 palette tiles) + WPE palette from CASC
    // CV5: 52 bytes/group. MTXM → (group=id>>4, tile=id&0xf) → CV5[group].tiles[tile] = vx4_idx
    let cv5_data = archive.extract_file(&format!("tileset/{}.cv5", tileset_name))
        .with_context(|| format!("CV5 not found for tileset {}", tileset_name))?;
    const CV5_ENTRY_BYTES: usize = 52;
    const CV5_TILES_OFFSET: usize = 20;
    println!("CV5 groups: {}", cv5_data.len() / CV5_ENTRY_BYTES);

    let mtxm_to_vx4 = |mega_id: u16| -> Option<usize> {
        let group = (mega_id >> 4) as usize;
        let tile  = (mega_id & 0xf) as usize;
        let off = group * CV5_ENTRY_BYTES + CV5_TILES_OFFSET + tile * 2;
        if off + 2 > cv5_data.len() { return None; }
        Some(u16::from_le_bytes(cv5_data[off..off+2].try_into().unwrap()) as usize)
    };

    let vx4_data = archive.extract_file(&format!("tileset/{}.vx4ex", tileset_name))
        .with_context(|| format!("VX4EX not found for tileset {}", tileset_name))?;

    // Classic VR4: 64 bytes/tile = 8×8 palette index pixels. WPE: 256×4 bytes BGR0 palette.
    let classic_vr4 = archive.extract_file(&format!("tileset/{}.vr4", tileset_name))
        .with_context(|| format!("Classic VR4 not found for tileset {}", tileset_name))?;
    let wpe_data = archive.extract_file(&format!("tileset/{}.wpe", tileset_name))
        .with_context(|| format!("WPE palette not found for tileset {}", tileset_name))?;

    // Build RGBA palette lookup from WPE (BGR0 format)
    let palette: Vec<[u8;4]> = (0..256usize).map(|i| {
        let base = i * 4;
        if base + 4 <= wpe_data.len() {
            [wpe_data[base+2], wpe_data[base+1], wpe_data[base], 255] // BGR→RGB
        } else { [0, 0, 0, 255] }
    }).collect();
    let classic_tile_count = classic_vr4.len() / 64;
    println!("Classic VR4: {} tiles. WPE: {} palette entries.", classic_tile_count, wpe_data.len()/4);

    // Decode one 8×8 classic VR4 tile → 32-byte RGBA pixels
    let decode_mini = |vr4_idx: usize| -> Vec<u8> {
        let mut out = vec![0u8; 8 * 8 * 4];
        let base = vr4_idx * 64;
        if base + 64 > classic_vr4.len() { return out; }
        for i in 0..64 {
            let c = palette[classic_vr4[base + i] as usize];
            out[i*4..i*4+4].copy_from_slice(&c);
        }
        out
    };

    // Load VF4 (walkability flags): 16 × u16 per megatile, bit 0x0001 = walkable mini-tile.
    // A megatile is walkable if ANY of its 16 mini-tiles has bit 0x0001 set.
    let vf4_data = archive.extract_file(&format!("tileset/{}.vf4", tileset_name))
        .with_context(|| format!("VF4 not found for tileset {}", tileset_name))?;
    const VF4_ENTRY_BYTES: usize = 32; // 16 × u16

    let is_walkable = |mega_id: u16| -> bool {
        let vx4_idx = match mtxm_to_vx4(mega_id) {
            Some(idx) => idx,
            None => return false,
        };
        let vf4_off = vx4_idx * VF4_ENTRY_BYTES;
        if vf4_off + VF4_ENTRY_BYTES > vf4_data.len() { return false; }
        // Any mini-tile walkable (bit 0x0001) → megatile is walkable
        for i in 0..16 {
            let flags = u16::from_le_bytes(vf4_data[vf4_off + i*2 .. vf4_off + i*2 + 2].try_into().unwrap());
            if flags & 0x0001 != 0 { return true; }
        }
        false
    };

    // Build walkable set for all MTXM values in this map
    let walkable_ids: Vec<u32> = mtxm.iter().copied()
        .collect::<std::collections::HashSet<u16>>()
        .into_iter()
        .filter(|&id| is_walkable(id))
        .map(|id| id as u32)
        .collect();
    println!("Walkable megatile IDs: {}/{}", walkable_ids.len(), mtxm.iter().collect::<std::collections::HashSet<_>>().len());

    // Build per-mini-tile walk map at 4× tile resolution (WalkPosition grid).
    // Each 32×32 tile is divided into 4×4 mini-tiles of 8×8px each.
    // walk_map[(ty*4 + mini_row) * (dim_w*4) + (tx*4 + mini_col)] = 1 if walkable.
    // VF4 entry layout: 16 × u16, indexed as mini_row*4 + mini_col (row-major, top-to-bottom).
    let walk_w = dim_w as usize * 4;
    let walk_h = dim_h as usize * 4;
    let mut walk_map: Vec<u8> = vec![0u8; walk_w * walk_h];
    for ty in 0..dim_h as usize {
        for tx in 0..dim_w as usize {
            let mega_id = mtxm[ty * dim_w as usize + tx];
            let vx4_idx = match mtxm_to_vx4(mega_id) {
                Some(idx) => idx,
                None => continue,
            };
            let vf4_off = vx4_idx * VF4_ENTRY_BYTES;
            if vf4_off + VF4_ENTRY_BYTES > vf4_data.len() { continue; }
            for mini_row in 0..4usize {
                for mini_col in 0..4usize {
                    let i = mini_row * 4 + mini_col;
                    let flags = u16::from_le_bytes(
                        vf4_data[vf4_off + i*2 .. vf4_off + i*2 + 2].try_into().unwrap()
                    );
                    if flags & 0x0001 != 0 {
                        let wx = tx * 4 + mini_col;
                        let wy = ty * 4 + mini_row;
                        walk_map[wy * walk_w + wx] = 1;
                    }
                }
            }
        }
    }

    // Pipeline: MTXM → CV5 → VX4EX → 16 classic VR4 mini-tiles (8×8) → 32×32 megatile.
    // 4. Build atlas: 4×4 classic 8×8 mini-tiles → 32×32 megatile → scaled to tile_size
    const MEGA_W: u32 = 32; // 4 mini-tiles × 8px = 32px megatile
    const MEGA_H: u32 = 32;

    let rows = n_tiles.div_ceil(cols);
    let atlas_w = cols * tile_size;
    let atlas_h = rows * tile_size;
    let mut atlas: RgbaImage = ImageBuffer::new(atlas_w, atlas_h);

    let max_id = *unique_ids.iter().max().unwrap_or(&0) as usize;
    let mut id_to_atlas = vec![u32::MAX; max_id + 1];

    for (atlas_idx, &mega_id) in unique_ids.iter().enumerate() {
        id_to_atlas[mega_id as usize] = atlas_idx as u32;

        let vx4_idx = match mtxm_to_vx4(mega_id) {
            Some(idx) => idx,
            None => continue,
        };
        let vx4_offset = vx4_idx * 64; // 16 × 4-byte entries
        let mut mega_img: RgbaImage = ImageBuffer::new(MEGA_W, MEGA_H);

        for mini_row in 0..4usize {
            for mini_col in 0..4usize {
                let entry_offset = vx4_offset + (mini_row * 4 + mini_col) * 4;
                if entry_offset + 4 > vx4_data.len() { continue; }
                let entry = u32::from_le_bytes(vx4_data[entry_offset..entry_offset+4].try_into().unwrap());
                let vr4_idx = (entry >> 1) as usize;
                let flip_x  = (entry & 1) == 1;
                if vr4_idx >= classic_tile_count { continue; }

                let pixels = decode_mini(vr4_idx);
                let mut mini: RgbaImage = ImageBuffer::from_raw(8, 8, pixels)
                    .unwrap_or_else(|| ImageBuffer::new(8, 8));
                if flip_x { mini = image::imageops::flip_horizontal(&mini); }
                // Place at correct 8×8 position within the 32×32 megatile
                image::imageops::overlay(&mut mega_img, &mini, (mini_col * 8) as i64, (mini_row * 8) as i64);
            }
        }

        let scaled = if tile_size == MEGA_W {
            mega_img
        } else {
            image::imageops::resize(&mega_img, tile_size, tile_size, image::imageops::FilterType::Nearest)
        };

        let ax = (atlas_idx as u32 % cols) * tile_size;
        let ay = (atlas_idx as u32 / cols) * tile_size;
        image::imageops::overlay(&mut atlas, &scaled, ax as i64, ay as i64);
    }

    // 5. Save atlas and mapping
    let map_stem = scm_path.file_stem().unwrap_or_default().to_string_lossy();
    let safe_name: String = map_stem.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();

    let png_path = output.join(format!("{}_terrain.png", safe_name));
    atlas.save(&png_path)?;

    // 6. Generate minimap thumbnail: 1 pixel per tile, sampled from the atlas.
    // This matches BW's minimap which shows terrain colors at low resolution.
    let walkable_set_u16: std::collections::HashSet<u16> = walkable_ids.iter().map(|&x| x as u16).collect();
    let mut minimap: RgbaImage = ImageBuffer::new(dim_w as u32, dim_h as u32);
    for ty in 0..dim_h as u32 {
        for tx in 0..dim_w as u32 {
            let mega_id = mtxm[(ty * dim_w as u32 + tx) as usize];
            let pixel = if let Some(&atlas_idx) = id_to_atlas.get(mega_id as usize).filter(|&&v| v != u32::MAX) {
                let ac = atlas_idx % cols;
                let ar = atlas_idx / cols;
                // Sample centre pixel of the atlas tile
                let px = ac * tile_size + tile_size / 2;
                let py = ar * tile_size + tile_size / 2;
                *atlas.get_pixel(px.min(atlas_w - 1), py.min(atlas_h - 1))
            } else if walkable_set_u16.contains(&mega_id) {
                image::Rgba([30u8, 60, 30, 255])  // walkable fallback: dark green
            } else {
                image::Rgba([40u8, 35, 30, 255])  // cliff fallback: dark brown
            };
            minimap.put_pixel(tx, ty, pixel);
        }
    }
    let mm_path = output.join(format!("{}_minimap.png", safe_name));
    minimap.save(&mm_path)?;

    let meta = serde_json::json!({
        "map": map_stem.as_ref(),
        "tileset": tileset_name,
        "era": era,
        "tile_size": tile_size,
        "cols": cols,
        "atlas_width": atlas_w,
        "atlas_height": atlas_h,
        "map_width": dim_w,
        "map_height": dim_h,
        "n_unique_tiles": n_tiles,
        "id_map": unique_ids.iter().enumerate().map(|(i, &id)| [id as u32, i as u32]).collect::<Vec<_>>(),
        "walkable_ids": walkable_ids,
        "walk_map": walk_map,
        "walk_map_width": walk_w,
        "walk_map_height": walk_h,
    });
    let json_path = output.join(format!("{}_terrain.json", safe_name));
    fs::write(&json_path, serde_json::to_string_pretty(&meta)?)?;

    println!("Saved: {:?} ({}×{}, {} tiles)", png_path, atlas_w, atlas_h, n_tiles);
    println!("Saved minimap: {:?} ({}×{})", mm_path, dim_w, dim_h);
    println!("Saved: {:?}", json_path);
    Ok(())
}

fn cmd_extract_map_terrain_dual(
    archive: &CascArchive,
    scm_path: &Path,
    cols: u32,
    tile_size: u32,
    output: &Path,
) -> Result<()> {
    use image::{ImageBuffer, RgbaImage};
    use mpq::Archive;

    fs::create_dir_all(output)?;

    // 1. Parse SCM to get ERA (tileset) and MTXM (megatile IDs)
    let mut archive_mpq = Archive::open(scm_path)
        .with_context(|| format!("Failed to open MPQ: {:?}", scm_path))?;

    let chk_file = archive_mpq.open_file("staredit\\scenario.chk")
        .or_else(|_| archive_mpq.open_file("staredit/scenario.chk"))
        .or_else(|_| archive_mpq.open_file("scenario.chk"))
        .with_context(|| "CHK not found in MPQ")?;

    let chk_size = chk_file.size() as usize;
    let mut chk_bytes = vec![0u8; chk_size];
    chk_file.read(&mut archive_mpq, &mut chk_bytes)
        .context("Failed to read CHK")?;

    let mut era: u16 = 3;
    let mut dim_w: u16 = 128;
    let mut dim_h: u16 = 128;
    let mut mtxm: Vec<u16> = Vec::new();

    let mut pos = 0;
    while pos + 8 <= chk_bytes.len() {
        let tag = &chk_bytes[pos..pos+4];
        let size = u32::from_le_bytes(chk_bytes[pos+4..pos+8].try_into().unwrap()) as usize;
        pos += 8;
        if pos + size > chk_bytes.len() { break; }
        let data = &chk_bytes[pos..pos+size];

        match tag {
            b"ERA " if size >= 2 => {
                era = u16::from_le_bytes(data[0..2].try_into().unwrap()) & 7;
            }
            b"DIM " if size >= 4 => {
                dim_w = u16::from_le_bytes(data[0..2].try_into().unwrap());
                dim_h = u16::from_le_bytes(data[2..4].try_into().unwrap());
            }
            b"MTXM" => {
                for i in (0..size).step_by(2) {
                    if i+2 <= size {
                        mtxm.push(u16::from_le_bytes(data[i..i+2].try_into().unwrap()));
                    }
                }
            }
            _ => {}
        }
        pos += size;
    }

    println!("Map: {}×{} tiles, ERA={} (tileset), {} MTXM entries", dim_w, dim_h, era, mtxm.len());

    let tileset_name = match era {
        0 => "badlands",
        1 => "platform",
        2 => "ashworld",
        3 => "jungle",
        4 => "jungle",
        5 => "desert",
        6 => "ice",
        7 => "twilight",
        _ => "jungle",
    };

    // 2. Collect unique megatile IDs
    let mut unique_ids: Vec<u16> = mtxm.iter().copied()
        .collect::<std::collections::HashSet<u16>>()
        .into_iter()
        .collect();
    unique_ids.sort_unstable();
    let n_tiles = unique_ids.len() as u32;
    println!("Unique megatile IDs: {}", n_tiles);

    // 3. Load VX4EX and VR4 from CASC
    let vx4_data = archive.extract_file(&format!("tileset/{}.vx4ex", tileset_name))
        .with_context(|| format!("VX4EX not found for tileset {}", tileset_name))?;

    let vr4_raw = archive.extract_file(&format!("tileset/{}.dds.vr4", tileset_name))
        .with_context(|| format!("VR4 not found for tileset {}", tileset_name))?;

    // VR4: 128×128 DXT1 (8192 bytes/tile). VX4EX: 16×u32 = 64 bytes/record. MTXM = VX4EX index.
    const VR4_HEADER: usize = 148;
    const VR4_STRIDE: usize = 8192;
    const VR4_W: u32 = 128;
    const VR4_H: u32 = 128;
    const MINI_W: u32 = 32;
    const MINI_H: u32 = 32;
    const MEGA_W: u32 = 128;
    const MEGA_H: u32 = 128;
    let vr4_data = &vr4_raw[VR4_HEADER..];
    let vr4_tile_count = vr4_data.len() / VR4_STRIDE;
    println!("VR4 tiles: {}", vr4_tile_count);

    // 4. Build atlas
    let rows = n_tiles.div_ceil(cols);
    let atlas_w = cols * tile_size;
    let atlas_h = rows * tile_size;
    let mut atlas_diffuse: RgbaImage = ImageBuffer::new(atlas_w, atlas_h);
    let mut atlas_base: RgbaImage = ImageBuffer::new(atlas_w, atlas_h);

    let max_id = *unique_ids.iter().max().unwrap_or(&0) as usize;
    let mut id_to_atlas = vec![u32::MAX; max_id + 1];

    for (atlas_idx, &mega_id) in unique_ids.iter().enumerate() {
        id_to_atlas[mega_id as usize] = atlas_idx as u32;

        let vx4_offset = mega_id as usize * 64;
        let mut mega_img: RgbaImage = ImageBuffer::new(MEGA_W, MEGA_H);

        for mini_row in 0..4usize {
            for mini_col in 0..4usize {
                let entry_offset = vx4_offset + (mini_row * 4 + mini_col) * 4;
                if entry_offset + 4 > vx4_data.len() { continue; }
                let entry = u32::from_le_bytes(vx4_data[entry_offset..entry_offset+4].try_into().unwrap());
                let vr4_idx = (entry >> 1) as usize;
                let flip_x  = (entry & 1) == 1;
                if vr4_idx >= vr4_tile_count { continue; }

                let tile_bytes = &vr4_data[vr4_idx * VR4_STRIDE .. vr4_idx * VR4_STRIDE + VR4_STRIDE];
                let decoded = decode_dxt1_nxn(tile_bytes, VR4_W as usize, VR4_H as usize);
                let mut mini: RgbaImage = ImageBuffer::from_raw(VR4_W, VR4_H, decoded)
                    .unwrap_or_else(|| ImageBuffer::new(VR4_W, VR4_H));
                if flip_x { mini = image::imageops::flip_horizontal(&mini); }
                let mini_s = image::imageops::resize(&mini, MINI_W, MINI_H, image::imageops::FilterType::Lanczos3);
                image::imageops::overlay(&mut mega_img, &mini_s, (mini_col as i64) * MINI_W as i64, (mini_row as i64) * MINI_H as i64);
            }
        }

        let scaled = if tile_size == MEGA_W {
            mega_img.clone()
        } else {
            image::imageops::resize(&mega_img, tile_size, tile_size, image::imageops::FilterType::Lanczos3)
        };
        let mut base = scaled.clone();
        for px in base.pixels_mut() { px[3] = 255; }

        let ax = (atlas_idx as u32 % cols) * tile_size;
        let ay = (atlas_idx as u32 / cols) * tile_size;
        image::imageops::overlay(&mut atlas_diffuse, &scaled, ax as i64, ay as i64);
        image::imageops::overlay(&mut atlas_base, &base, ax as i64, ay as i64);
    }

    // 5. Compute alpha stats for diffuse
    let total_pixels = (atlas_w * atlas_h) as u64;
    let transparent_pixels = atlas_diffuse.pixels().filter(|p| p[3] == 0).count() as u64;
    let semi_pixels = atlas_diffuse.pixels().filter(|p| p[3] > 0 && p[3] < 255).count() as u64;

    // 6. Save outputs
    let map_stem = scm_path.file_stem().unwrap_or_default().to_string_lossy();
    let safe_name: String = map_stem.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();

    let diffuse_path = output.join(format!("{}_diffuse.png", safe_name));
    let base_path = output.join(format!("{}_base.png", safe_name));
    let json_path = output.join(format!("{}_terrain_dual.json", safe_name));

    atlas_diffuse.save(&diffuse_path)?;
    atlas_base.save(&base_path)?;

    let meta = serde_json::json!({
        "map": map_stem.as_ref(),
        "tileset": tileset_name,
        "era": era,
        "tile_size": tile_size,
        "cols": cols,
        "atlas_width": atlas_w,
        "atlas_height": atlas_h,
        "n_unique_tiles": n_tiles,
        "has_dual": true,
        "diffuse_alpha_stats": {
            "total_pixels": total_pixels,
            "transparent_pixels": transparent_pixels,
            "semi_transparent_pixels": semi_pixels,
            "opaque_pixels": total_pixels - transparent_pixels - semi_pixels,
        },
        "id_map": unique_ids.iter().enumerate().map(|(i, &id)| [id as u32, i as u32]).collect::<Vec<_>>(),
    });
    fs::write(&json_path, serde_json::to_string_pretty(&meta)?)?;

    println!("Saved diffuse: {:?} ({}×{}, {} tiles)", diffuse_path, atlas_w, atlas_h, n_tiles);
    println!("Saved base:    {:?}", base_path);
    println!("Saved JSON:    {:?}", json_path);
    println!("Diffuse alpha: {} transparent, {} semi, {} opaque (of {} total pixels)",
        transparent_pixels, semi_pixels, total_pixels - transparent_pixels - semi_pixels, total_pixels);
    Ok(())
}

// ---------------------------------------------------------------------------
// extract raw
// ---------------------------------------------------------------------------

/// Extract raw bytes from the CASC archive without any format conversion.
///
/// Returns `Ok(())` as long as at least one file was extracted successfully.
/// Returns `Err` only when every requested file failed.
#[allow(clippy::too_many_arguments)]
fn cmd_extract_raw(
    archive: &CascArchive,
    install_path: Option<&Path>,
    file_paths: Vec<String>,
    search: Option<String>,
    locale: &str,
    output: &Path,
    preserve_path: bool,
    online: bool,
) -> Result<()> {
    fs::create_dir_all(output)?;

    // Collect the list of CASC paths to attempt.
    let paths: Vec<String> = if let Some(pattern) = search {
        // Use CascStorage to enumerate all archive paths, then filter by pattern + locale.
        // If the storage cannot be opened (no local install), fall back to searching
        // the embedded fallback table.
        match open_casc_storage(install_path) {
            Ok(storage) => {
                let all_files = storage
                    .list_files()
                    .map_err(|e| anyhow::anyhow!("list_files failed: {}", e))?;

                let pattern_lower = pattern.to_lowercase();
                let locale_lower = locale.to_lowercase();

                all_files
                    .into_iter()
                    .filter(|f| {
                        let fl = f.to_lowercase();
                        fl.contains(&pattern_lower) && fl.contains(&locale_lower)
                    })
                    .collect()
            }
            Err(_) => {
                // No local install — search embedded fallbacks only.
                fallbacks::search(&pattern, locale)
                    .into_iter()
                    .map(|s| s.to_owned())
                    .collect()
            }
        }
    } else if online {
        // When fetching from the CDN, paths that lack a locale prefix won't resolve.
        // Automatically prepend `locales\<locale>\` to any path that doesn't already
        // start with that prefix.
        file_paths
            .into_iter()
            .map(|p| {
                let pl = p.to_lowercase();
                if pl.starts_with("locales/") || pl.starts_with("locales\\") {
                    p
                } else {
                    // Use backslash convention (matches SC:R CASC paths).
                    format!("locales\\{}\\{}", locale, p)
                }
            })
            .collect()
    } else {
        file_paths
    };

    if paths.is_empty() {
        println!("No files matched the request.");
        return Ok(());
    }

    let mut extracted = 0usize;
    let mut from_fallback = 0usize;
    let mut skipped = 0usize;

    for casc_path in &paths {
        // Resolve data: live archive first, then embedded fallback.
        let resolved: Option<(Vec<u8>, bool)> = match archive.extract_file(casc_path) {
            Ok(data) => Some((data, false)),
            Err(_) => {
                fallbacks::get(casc_path).map(|embedded| (embedded.to_vec(), true))
            }
        };

        match resolved {
            Some((data, is_fallback)) => {
                // Determine the local output path.
                let local_path = if preserve_path {
                    // Normalise CASC backslashes to the OS separator, then join.
                    let normalised = casc_path.replace('\\', std::path::MAIN_SEPARATOR_STR);
                    let rel = PathBuf::from(normalised);
                    // Strip a leading separator if present (makes join work correctly).
                    let rel = rel.strip_prefix(std::path::MAIN_SEPARATOR_STR).unwrap_or(&rel).to_path_buf();
                    output.join(rel)
                } else {
                    let filename = casc_path
                        .rsplit(['\\', '/'])
                        .next()
                        .unwrap_or(casc_path);
                    output.join(filename)
                };

                // Create any intermediate directories when --preserve-path is set.
                if let Some(parent) = local_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                fs::write(&local_path, &data)
                    .with_context(|| format!("Failed to write {:?}", local_path))?;

                if is_fallback {
                    println!(
                        "fallback: {} -> {} ({} bytes) [embedded]",
                        casc_path,
                        local_path.display(),
                        data.len()
                    );
                    from_fallback += 1;
                } else {
                    println!(
                        "extracted: {} -> {} ({} bytes)",
                        casc_path,
                        local_path.display(),
                        data.len()
                    );
                    extracted += 1;
                }
            }
            None => {
                println!("skipped (not in local install): {}", casc_path);
                skipped += 1;
            }
        }
    }

    println!(
        "\n{} extracted, {} from embedded fallback, {} skipped (of {} requested).",
        extracted,
        from_fallback,
        skipped,
        paths.len()
    );

    if extracted == 0 && from_fallback == 0 && skipped > 0 {
        anyhow::bail!("All {} requested file(s) were skipped — none present in local install or embedded fallbacks.", skipped);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load config file (if provided), then apply CLI overrides on top.
    let config = load_config(cli.config.as_deref());

    // Logging: verbose flag or config beats default warn-only.
    let log_level = if cli.verbose || config.feedback_settings.verbose_logging {
        "debug"
    } else {
        "warn"
    };
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", log_level);
    }
    env_logger::init();

    // Resolve output directory: CLI flag > config file > hard-coded default.
    let output = cli.output
        .unwrap_or_else(|| config.output_settings.output_directory.clone());

    match cli.command {
        // ------------------------------------------------------------------
        Commands::Extract { target } => {
            // --validate-only: open archive, count files, print first 5, then exit.
            if cli.validate_only {
                let storage = open_casc_storage(cli.install_path.as_deref())?;
                let files = storage
                    .list_files()
                    .map_err(|e| anyhow::anyhow!("list_files failed: {}", e))?;
                println!("Archive opened successfully. {} files found.", files.len());
                for f in files.iter().take(5) {
                    println!("  {}", f);
                }
                return Ok(());
            }

            let archive = open_casc_archive(cli.install_path.as_deref())?;
            match target {
                ExtractCommands::Anim {
                    quality,
                    ids,
                    raw,
                    name_map,
                    convert_to_png,
                    team_color_mask,
                    layers,
                    save_dds,
                } => {
                    let convert_to_png = convert_to_png
                        || matches!(
                            config.quality_settings.format_filter,
                            FormatFilterOption::Png | FormatFilterOption::Images
                        );
                    cmd_extract_anim(
                        &archive,
                        &output,
                        quality,
                        ids,
                        raw,
                        name_map,
                        convert_to_png,
                        team_color_mask,
                        &config,
                        layers,
                        save_dds,
                    )?;
                }
                ExtractCommands::Tileset {
                    quality,
                    convert_to_png,
                } => {
                    let convert_to_png = convert_to_png
                        || matches!(
                            config.quality_settings.format_filter,
                            FormatFilterOption::Png | FormatFilterOption::Images
                        );
                    cmd_extract_tileset(&archive, &output.join("tilesets"), quality, convert_to_png, &config)?;
                }
                ExtractCommands::Effect {
                    quality,
                    convert_to_png,
                } => {
                    let convert_to_png = convert_to_png
                        || matches!(
                            config.quality_settings.format_filter,
                            FormatFilterOption::Png | FormatFilterOption::Images
                        );
                    cmd_extract_effect(&archive, &output.join("effects"), quality, convert_to_png, &config)?;
                }
                ExtractCommands::TilesetAtlas {
                    tileset,
                    quality,
                    cols,
                    max_tiles,
                    tile_size,
                    output: atlas_output,
                } => {
                    let out = atlas_output.unwrap_or_else(|| output.join("tileset-atlas"));
                    cmd_extract_tileset_atlas(&archive, &tileset, quality, cols, max_tiles, tile_size, &out)?;
                }
                ExtractCommands::TilesetMegatile {
                    tileset,
                    quality,
                    max_tiles,
                    tile_size,
                    cols,
                    output: mega_output,
                } => {
                    let out = mega_output.unwrap_or_else(|| output.join("tileset-megatile"));
                    cmd_extract_tileset_megatile(&archive, &tileset, quality, max_tiles, tile_size, cols, &out)?;
                }
                ExtractCommands::MapTerrain { scm, cols, tile_size, output: terrain_output } => {
                    let out = terrain_output.unwrap_or_else(|| output.join("map-terrain"));
                    cmd_extract_map_terrain(&archive, &scm, cols, tile_size, &out)?;
                }
                ExtractCommands::MapTerrainDual { scm, cols, tile_size, output: terrain_output } => {
                    let out = terrain_output.unwrap_or_else(|| output.join("map-terrain"));
                    cmd_extract_map_terrain_dual(&archive, &scm, cols, tile_size, &out)?;
                }
                ExtractCommands::Mainmenu { output: mm_output, convert_dds } => {
                    let out = mm_output.unwrap_or_else(|| output.join("ui/mainmenu"));
                    cmd_extract_mainmenu(&archive, &out, convert_dds)?;
                }
                ExtractCommands::Dat { output: dat_output } => {
                    let out = dat_output.unwrap_or_else(|| output.join("dat"));
                    cmd_extract_dat(&archive, &out)?;
                }
                ExtractCommands::Cmdicons { output: cmd_output, tint, save_raw } => {
                    let out = cmd_output.unwrap_or_else(|| output.join("ui/cmdicons"));
                    let tint_rgb = [tint.first().copied().unwrap_or(255),
                                    tint.get(1).copied().unwrap_or(254),
                                    tint.get(2).copied().unwrap_or(84)];
                    cmd_extract_cmdicons(&archive, &out, tint_rgb, save_raw)?;
                }
                ExtractCommands::Raw {
                    file_paths,
                    search,
                    locale,
                    preserve_path,
                    online,
                } => {
                    if online {
                        let install_path = locate_starcraft(cli.install_path.as_deref())
                            .map_err(|e| anyhow::anyhow!("Could not locate StarCraft install path: {}\n{}", e, INSTALL_PATH_HINT))?
                            .into_os_string()
                            .into_string()
                            .map_err(|p| anyhow::anyhow!("Install path is not valid UTF-8: {:?}", p))?;
                        let online_archive = CascArchive::open_online(&install_path)
                            .map_err(|e| anyhow::anyhow!("Failed to open CASC archive in online mode: {}", e))?;
                        cmd_extract_raw(
                            &online_archive,
                            cli.install_path.as_deref(),
                            file_paths,
                            search,
                            &locale,
                            &output,
                            preserve_path,
                            online,
                        )?;
                    } else {
                        cmd_extract_raw(
                            &archive,
                            cli.install_path.as_deref(),
                            file_paths,
                            search,
                            &locale,
                            &output,
                            preserve_path,
                            online,
                        )?;
                    }
                }
            }
        }

        // ------------------------------------------------------------------
        Commands::Sounds { action } => match action {
            SoundsCommands::Extract { sounds_output, targets } => {
                let out = sounds_output.unwrap_or_else(|| output.join("sounds"));
                let archive = open_casc_archive(cli.install_path.as_deref())?;
                let storage = open_casc_storage(cli.install_path.as_deref())?;
                cmd_sounds_extract(&archive, &storage, &out, targets.as_deref())?;
            }
            SoundsCommands::List { search } => {
                cmd_sounds_list(cli.install_path.as_deref(), search)?;
            }
            SoundsCommands::Probe { filter } => {
                cmd_sounds_probe(cli.install_path.as_deref(), filter.as_deref())?;
            }
            SoundsCommands::ExportTargets { output: targets_output } => {
                cmd_sounds_export_targets(&targets_output)?;
            }
        },

        // ------------------------------------------------------------------
        Commands::Inspect { target } => match target {
            InspectCommands::Sprites { quality, max_id } => {
                let archive = open_casc_archive(cli.install_path.as_deref())?;
                cmd_inspect_sprites(&archive, quality, max_id)?;
            }
            InspectCommands::Files { search, sizes } => {
                cmd_inspect_files(cli.install_path.as_deref(), search.as_deref(), sizes)?;
            }
            InspectCommands::Archive => {
                cmd_inspect_archive(cli.install_path.as_deref())?;
            }
        },

        // ------------------------------------------------------------------
        Commands::Config { action } => match action {
            ConfigCommands::Init { output: cfg_output } => {
                cmd_config_init(&cfg_output)?;
            }
        },

        // ------------------------------------------------------------------
        Commands::Validate { action } => match action {
            ValidateCommands::Register { file, suite } => {
                cmd_validate_register(&file, &suite)?;
            }
            ValidateCommands::Run { dir, suite, json } => {
                cmd_validate_run(&dir, &suite, json)?;
            }
        },
    }

    Ok(())
}

// ─── extract cmdicons ──────────────────────────────────────────────────────

/// Extract `unit\cmdicons\cmdicons.dds.grp` and write a colour-tinted PNG atlas.
///
/// ## File format (discovered by analysis)
/// The dds.grp starts with a 20-byte pre-header:
///   u32 total_file_size | u16 frame_count (390) | u16 flags | u32 zero |
///   u16 max_width (128) | u16 max_height (128) | u32 frame_alloc (8332)
///
/// Each of the 390 frames is a standalone DDS file prefixed by its own `DDS ` magic.
/// Frame sizes vary (128×128 = 8332 bytes most common; blank/portrait frames smaller).
/// Blank/placeholder frames use reduced dimensions (e.g. 116×36 for BLANK text).
///
/// ## Slot layout
/// Frames are in units.dat unit-ID order for the portrait section, then command buttons:
///   Slots   0–227: unit wireframe portraits  (frame == units.dat unit ID)
///   Slots 228–389: command button icons      (Move=228, Stop=229, …)
///
/// ## Tinting
/// Icons are stored as greyscale DXT1 (disabled/neutral state).  The game tints them
/// at runtime using the player colour.  We apply a static yellow tint:
///   out.rgb = tint_rgb * (grey / 255)
fn cmd_extract_cmdicons(
    archive: &casc_extractor::casc::casclib_ffi::CascArchive,
    output: &std::path::Path,
    tint: [u8; 3],
    save_raw: bool,
) -> anyhow::Result<()> {
    use texture2ddecoder::decode_bc1;

    let raw = archive.extract_file("unit\\cmdicons\\cmdicons.dds.grp")
        .map_err(|e| anyhow::anyhow!("cmdicons.dds.grp not found: {}", e))?;

    let total    = u16::from_le_bytes([raw[4], raw[5]]) as usize;
    let max_w    = u16::from_le_bytes([raw[12], raw[13]]) as usize;
    let max_h    = u16::from_le_bytes([raw[14], raw[15]]) as usize;

    // Find all DDS frame start positions
    let mut positions: Vec<usize> = Vec::new();
    let mut i = 0usize;
    while let Some(rel) = raw[i..].windows(4).position(|w| w == b"DDS ") {
        positions.push(rel + i);
        i = rel + i + 1;
    }
    if positions.len() != total {
        println!("  Warning: found {} DDS frames, expected {}", positions.len(), total);
    }
    println!("  {} frames, max {}x{}, tint {:?}", positions.len(), max_w, max_h, tint);

    let cols  = 17usize;
    let rows  = total.div_ceil(cols);
    let sw    = max_w * cols;
    let sh    = max_h * rows;
    let blank = vec![0u8; sw * sh * 4];

    let mut tinted_sheet = blank.clone();
    let mut raw_sheet    = if save_raw { blank.clone() } else { Vec::new() };

    for (slot, &pos) in positions.iter().enumerate() {
        if slot >= total { break; }
        let end = positions.get(slot + 1).copied().unwrap_or(raw.len());
        let frame_data = &raw[pos..end];

        if frame_data.len() < 128 { continue; }
        let fw = u32::from_le_bytes([frame_data[16],frame_data[17],frame_data[18],frame_data[19]]) as usize;
        let fh = u32::from_le_bytes([frame_data[12],frame_data[13],frame_data[14],frame_data[15]]) as usize;
        if fw == 0 || fh == 0 { continue; }

        let expected = fw.div_ceil(4) * fh.div_ceil(4) * 8;
        if frame_data.len() < 128 + expected { continue; }

        let mut px = vec![0u32; fw * fh];
        if decode_bc1(&frame_data[128..128+expected], fw, fh, &mut px).is_err() { continue; }

        // Centre the (possibly smaller) frame in the max_w × max_h cell
        let col = slot % cols; let row = slot / cols;
        let xo  = col * max_w + (max_w - fw) / 2;
        let yo  = row * max_h + (max_h - fh) / 2;

        for y in 0..fh { for x in 0..fw {
            let p   = px[y * fw + x];
            let r   = (p & 0xFF) as u8;
            let g   = ((p >> 8)  & 0xFF) as u8;
            let b   = ((p >> 16) & 0xFF) as u8;
            let a   = ((p >> 24) & 0xFF) as u8;
            let lum = (r as u32 + g as u32 + b as u32) / 3;
            let di  = ((yo + y) * sw + xo + x) * 4;
            if di + 4 > tinted_sheet.len() { continue; }
            // Tinted
            tinted_sheet[di]   = (tint[0] as u32 * lum / 255) as u8;
            tinted_sheet[di+1] = (tint[1] as u32 * lum / 255) as u8;
            tinted_sheet[di+2] = (tint[2] as u32 * lum / 255) as u8;
            tinted_sheet[di+3] = a;
            // Raw greyscale
            if save_raw && di + 4 <= raw_sheet.len() {
                raw_sheet[di] = r; raw_sheet[di+1] = g; raw_sheet[di+2] = b; raw_sheet[di+3] = a;
            }
        }}
    }

    std::fs::create_dir_all(output)?;
    let write_png = |path: &std::path::Path, data: &[u8]| -> anyhow::Result<()> {
        let f = std::fs::File::create(path)?;
        let mut enc = png::Encoder::new(std::io::BufWriter::new(f), sw as u32, sh as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()?.write_image_data(data)?;
        Ok(())
    };

    let tinted_path = output.join("cmdicons.png");
    write_png(&tinted_path, &tinted_sheet)?;
    println!("  Saved {}  ({}×{}, {} slots, tint {:?})", tinted_path.display(), sw, sh, total, tint);

    if save_raw {
        let raw_path = output.join("cmdicons_raw.png");
        write_png(&raw_path, &raw_sheet)?;
        println!("  Saved {} (greyscale)", raw_path.display());
    }

    // Write metadata JSON
    let meta = serde_json::json!({
        "frameCount": total, "frameWidth": max_w, "frameHeight": max_h,
        "framesPerRow": cols, "rows": rows, "sheetWidth": sw, "sheetHeight": sh,
        "layout": "slots 0-227 = unit portraits (frame == units.dat unit ID); slots 228-389 = command buttons",
        "commandButtons": {
            "Move":228,"Stop":229,"HoldPosition":230,"Patrol":231,"AttackMove":232,
            "Cancel":236,"StimPack":237,"U238Shells":238,"Lockdown":240,"EMP":241,
            "Irradiate":242,"SpiderMines":243,"SiegeTech":245,"SciVesselEnergy":248,
            "OcularImplants":249,"YamatoGun":251,"PersonnelCloak":252,"MoebiusReactor":256,
            "ColossusReactor":285,"IonThrusters":287,"InfantryWeapons":288,"VehicleWeapons":289,
            "ShipWeapons":290,"ShipPlating":291,"InfantryArmor":292,"VehiclePlating":293,
            "Restoration":366,"OpticFlare":373,"CharonBoosters":380,"CaduceusReactor":384
        },
        "frames": (0..total).map(|i| serde_json::json!({
            "index": i,
            "x": (i % cols) * max_w,
            "y": (i / cols) * max_h,
            "width": max_w, "height": max_h
        })).collect::<Vec<_>>()
    });
    std::fs::write(output.join("cmdicons.json"), serde_json::to_string_pretty(&meta)?)?;
    println!("  Saved {}", output.join("cmdicons.json").display());
    Ok(())
}

// ─── extract mainmenu ──────────────────────────────────────────────────────

/// Extract SC:R main-menu assets (backgrounds, logos, music, SFX, button webms).
fn cmd_extract_mainmenu(
    archive: &casc_extractor::casc::casclib_ffi::CascArchive,
    output: &std::path::Path,
    convert_dds: bool,
) -> anyhow::Result<()> {
    use casc_extractor::dds_converter::save_dds_as_png;

    std::fs::create_dir_all(output)?;

    let assets: &[(&str, &str)] = &[
        ("HD2\\glue\\title\\title.DDS",                 "title.dds"),
        ("HD2\\glue\\mainmenu\\titleframe_bg.DDS",      "titleframe_bg.dds"),
        ("HD2\\glue\\mainmenu\\titleframe_overlay.DDS", "titleframe_overlay.dds"),
        ("HD2\\glue\\mainmenu\\etail.DDS",              "etail.dds"),
        ("HD2\\glue\\mainmenu\\pintro.DDS",             "pintro.dds"),
        ("HD2\\glue\\mainmenu\\pcredit.DDS",            "pcredit.dds"),
        ("HD2\\glue\\mainmenu\\Lock.DDS",               "lock.dds"),
        ("HD2\\glue\\mainmenu\\single.webm",            "single.webm"),
        ("HD2\\glue\\mainmenu\\singleon.webm",          "singleon.webm"),
        ("HD2\\glue\\mainmenu\\multi.webm",             "multi.webm"),
        ("HD2\\glue\\mainmenu\\multion.webm",           "multion.webm"),
        ("HD2\\glue\\mainmenu\\exit.webm",              "exit.webm"),
        ("HD2\\glue\\mainmenu\\exiton.webm",            "exiton.webm"),
        ("HD2\\glue\\mainmenu\\editor.webm",            "editor.webm"),
        ("HD2\\glue\\mainmenu\\editoron.webm",          "editoron.webm"),
        ("sound\\glue\\mouseover.wav",                  "sfx_mouseover.wav"),
        ("sound\\glue\\mousedown2.wav",                 "sfx_click.wav"),
        ("sound\\glue\\swishin.wav",                    "sfx_swishin.wav"),
        ("sound\\glue\\swishout.wav",                   "sfx_swishout.wav"),
        ("sound\\glue\\swishlock.wav",                  "sfx_swishlock.wav"),
        ("sound\\glue\\bnetclick.wav",                  "sfx_bnetclick.wav"),
        ("sound\\glue\\countdown.wav",                  "sfx_countdown.wav"),
        ("Music\\terran1.ogg",                          "music_terran.ogg"),
        ("Music\\zerg1.ogg",                            "music_zerg.ogg"),
        ("Music\\protoss1.ogg",                         "music_protoss.ogg"),
    ];

    let (mut ok, mut fail) = (0usize, 0usize);
    for (casc_path, out_name) in assets {
        match archive.extract_file(casc_path) {
            Ok(data) if !data.is_empty() => {
                if convert_dds && out_name.ends_with(".dds") {
                    let png_name = out_name.replace(".dds", ".png");
                    let png_path = output.join(&png_name);
                    match save_dds_as_png(&data, &png_path) {
                        Ok(()) => { println!("  ✓ {} → {}", casc_path, png_name); ok += 1; }
                        Err(e) => {
                            std::fs::write(output.join(out_name), &data)?;
                            println!("  ⚠ {} → {} (PNG failed: {}, saved raw DDS)", casc_path, out_name, e);
                            ok += 1;
                        }
                    }
                } else {
                    std::fs::write(output.join(out_name), &data)?;
                    println!("  ✓ {} → {} ({} bytes)", casc_path, out_name, data.len());
                    ok += 1;
                }
            }
            Ok(_) => { println!("  ⚠ empty: {}", casc_path); fail += 1; }
            Err(_) => { println!("  ✗ not found: {}", casc_path); fail += 1; }
        }
    }
    println!("  {} extracted, {} not found → {:?}", ok, fail, output);
    Ok(())
}

// ─── extract dat ───────────────────────────────────────────────────────────

/// Extract BW data tables (units.dat, upgrades.dat, images.dat, orders.dat, techdata.dat).
fn cmd_extract_dat(
    archive: &casc_extractor::casc::casclib_ffi::CascArchive,
    output: &std::path::Path,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(output)?;

    let files: &[(&str, &[&str])] = &[
        ("units.dat",    &["arr\\units.dat",    "arr/units.dat"]),
        ("upgrades.dat", &["arr\\upgrades.dat", "arr/upgrades.dat"]),
        ("images.dat",   &["arr\\images.dat",   "arr/images.dat"]),
        ("orders.dat",   &["arr\\orders.dat",   "arr/orders.dat"]),
        ("techdata.dat", &["arr\\techdata.dat", "arr/techdata.dat"]),
        ("weapons.dat",  &["arr\\weapons.dat",  "arr/weapons.dat"]),
        ("flingy.dat",   &["arr\\flingy.dat",   "arr/flingy.dat"]),
        ("sprites.dat",  &["arr\\sprites.dat",  "arr/sprites.dat"]),
        ("iscript.bin",  &["scripts\\iscript.bin", "scripts/iscript.bin"]),
        ("images.tbl",   &["arr\\images.tbl",   "arr/images.tbl"]),
        ("stat_txt.tbl", &["rez\\stat_txt.tbl", "rez/stat_txt.tbl"]),
    ];

    let (mut ok, mut fail) = (0usize, 0usize);
    for (name, paths) in files {
        let mut found = false;
        for p in *paths {
            if let Ok(d) = archive.extract_file(p) {
                if !d.is_empty() {
                    std::fs::write(output.join(name), &d)?;
                    println!("  ✓ {} → {} ({} bytes)", p, name, d.len());
                    ok += 1; found = true; break;
                }
            }
        }
        if !found { println!("  ✗ not found: {}", name); fail += 1; }
    }
    println!("  {} extracted, {} not found → {:?}", ok, fail, output);
    Ok(())
}

// ─── inspect files ─────────────────────────────────────────────────────────

/// List or search all files in the CASC archive.
fn cmd_inspect_files(
    install_path: Option<&std::path::Path>,
    search: Option<&str>,
    sizes: bool,
) -> anyhow::Result<()> {
    let storage = open_casc_storage(install_path)?;
    let files = storage.list_files()
        .map_err(|e| anyhow::anyhow!("list_files failed: {}", e))?;

    let pattern = search.map(|s| s.to_lowercase());
    let mut count = 0usize;
    for f in &files {
        let fl = f.to_lowercase();
        if pattern.as_deref().is_none_or(|p| fl.contains(p)) {
            if sizes {
                // Try to get file size via a quick extract probe — skip for speed
                println!("{}", f);
            } else {
                println!("{}", f);
            }
            count += 1;
        }
    }
    eprintln!("({} matches / {} total files)", count, files.len());
    Ok(())
}

// ─── sounds probe ──────────────────────────────────────────────────────────

/// Probe known unit sound paths to verify which ones resolve in the archive.
fn cmd_sounds_probe(
    install_path: Option<&std::path::Path>,
    filter: Option<&str>,
) -> anyhow::Result<()> {
    let archive = open_casc_archive(install_path)?;

    let candidates: &[(&str, &str)] = &[
        ("hydralisk_attack",  "sound\\Zerg\\Hydralisk\\HydAtt00.wav"),
        ("hydralisk_die",     "sound\\Zerg\\Hydralisk\\HydDth00.wav"),
        ("hydralisk_yes",     "sound\\Zerg\\Hydralisk\\HydYes00.wav"),
        ("hydralisk_what",    "sound\\Zerg\\Hydralisk\\HydWht00.wav"),
        ("zealot_attack",     "sound\\Protoss\\Zealot\\ZeaAtt00.wav"),
        ("zealot_die",        "sound\\Protoss\\Zealot\\ZeaDth00.wav"),
        ("zealot_yes",        "sound\\Protoss\\Zealot\\ZeaYes00.wav"),
        ("zealot_what",       "sound\\Protoss\\Zealot\\ZeaWht00.wav"),
        ("ghost_attack",      "sound\\Terran\\Ghost\\TghAtt00.wav"),
        ("ghost_die",         "sound\\Terran\\Ghost\\TghDth00.wav"),
        ("ghost_yes",         "sound\\Terran\\Ghost\\TghYes00.wav"),
        ("ghost_what",        "sound\\Terran\\Ghost\\TghWht00.wav"),
        ("siege_attack",      "sound\\Terran\\Vehicle\\TvhAtt00.wav"),
        ("siege_die",         "sound\\Terran\\Vehicle\\TvhDth00.wav"),
        ("vulture_attack",    "sound\\Terran\\Vulture\\TVuAtt00.wav"),
        ("vulture_yes",       "sound\\Terran\\Vulture\\TVuYes00.wav"),
        ("dragoon_attack",    "sound\\Protoss\\Dragoon\\PDrAtt00.wav"),
        ("dragoon_die",       "sound\\Protoss\\Dragoon\\PDrDth00.wav"),
        ("dragoon_yes",       "sound\\Protoss\\Dragoon\\PDrYes00.wav"),
        ("scv_yes",           "sound\\Terran\\SCV\\TSCYes00.wav"),
        ("scv_attack",        "sound\\Terran\\SCV\\TSCAtt00.wav"),
        ("medic_yes",         "sound\\Terran\\Medic\\TMEYes00.wav"),
        ("medic_what",        "sound\\Terran\\Medic\\TMEWht00.wav"),
        ("mutalisk_attack",   "sound\\Zerg\\Mutalisk\\MutAtt00.wav"),
        ("mutalisk_die",      "sound\\Zerg\\Mutalisk\\MutDth00.wav"),
        ("ultralisk_attack",  "sound\\Zerg\\Ultralisk\\UltAtt00.wav"),
        ("ultralisk_die",     "sound\\Zerg\\Ultralisk\\UltDth00.wav"),
        ("ultralisk_yes",     "sound\\Zerg\\Ultralisk\\UltYes00.wav"),
        ("probe_yes",         "sound\\Protoss\\Probe\\PrbYes00.wav"),
        ("probe_attack",      "sound\\Protoss\\Probe\\PrbAtt00.wav"),
        ("weapon_gauss",      "sound\\Weapons\\Terran\\tgun.wav"),
        ("weapon_tank",       "sound\\Weapons\\Terran\\tTankVulcan.wav"),
        ("weapon_tank2",      "sound\\Weapons\\Terran\\tTankShot.wav"),
        ("weapon_needle",     "sound\\Weapons\\Zerg\\zNeedleSpine.wav"),
        ("weapon_zealot",     "sound\\Weapons\\Protoss\\pZealotHit.wav"),
        ("weapon_phaser",     "sound\\Weapons\\Protoss\\pPhaserCannon.wav"),
    ];

    let filter_lc = filter.map(|s| s.to_lowercase());
    let (mut found, mut checked) = (0usize, 0usize);
    for (name, path) in candidates {
        if let Some(ref f) = filter_lc {
            if !name.contains(f.as_str()) && !path.to_lowercase().contains(f.as_str()) { continue; }
        }
        checked += 1;
        let variants = [
            path.to_string(),
            path.to_lowercase(),
            path.replace('\\', "/"),
            path.to_lowercase().replace('\\', "/"),
        ];
        let mut hit = false;
        for v in &variants {
            if archive.extract_file(v).map(|d| !d.is_empty()).unwrap_or(false) {
                println!("  ✓ {:<25} {}", name, v);
                found += 1; hit = true; break;
            }
        }
        if !hit { println!("  ✗ {:<25} {}", name, path); }
    }
    println!("  {}/{} found", found, checked);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // passes_filter
    // -----------------------------------------------------------------------

    #[test]
    fn passes_filter_empty_patterns_always_passes() {
        assert!(passes_filter("anything/path.anim", &[], &[]));
        assert!(passes_filter("", &[], &[]));
    }

    #[test]
    fn passes_filter_include_match_passes() {
        let include = compile_patterns(&Some(vec!["terran".to_string()]));
        assert!(passes_filter("data/terran/unit.anim", &include, &[]));
    }

    #[test]
    fn passes_filter_include_no_match_fails() {
        let include = compile_patterns(&Some(vec!["terran".to_string()]));
        assert!(!passes_filter("data/protoss/unit.anim", &include, &[]));
    }

    #[test]
    fn passes_filter_exclude_match_blocks() {
        let exclude = compile_patterns(&Some(vec!["ui".to_string()]));
        assert!(!passes_filter("data/ui/button.anim", &[], &exclude));
    }

    #[test]
    fn passes_filter_exclude_no_match_passes() {
        let exclude = compile_patterns(&Some(vec!["ui".to_string()]));
        assert!(passes_filter("data/terran/unit.anim", &[], &exclude));
    }

    #[test]
    fn passes_filter_include_and_exclude_combined() {
        let include = compile_patterns(&Some(vec!["anim".to_string()]));
        let exclude = compile_patterns(&Some(vec!["ui".to_string()]));
        // matches include, not excluded
        assert!(passes_filter("data/terran/unit.anim", &include, &exclude));
        // matches include but also excluded
        assert!(!passes_filter("data/ui/button.anim", &include, &exclude));
        // does not match include
        assert!(!passes_filter("data/terran/unit.png", &include, &exclude));
    }

    // -----------------------------------------------------------------------
    // png_compression
    // -----------------------------------------------------------------------

    #[test]
    fn png_compression_level_0_is_fast() {
        assert!(matches!(png_compression(0), png::Compression::Fast));
    }

    #[test]
    fn png_compression_level_2_is_fast() {
        assert!(matches!(png_compression(2), png::Compression::Fast));
    }

    #[test]
    fn png_compression_level_5_is_default() {
        assert!(matches!(png_compression(5), png::Compression::Default));
    }

    #[test]
    fn png_compression_level_9_is_best() {
        assert!(matches!(png_compression(9), png::Compression::Best));
    }

    #[test]
    fn png_compression_level_7_is_best() {
        assert!(matches!(png_compression(7), png::Compression::Best));
    }

    // -----------------------------------------------------------------------
    // compile_patterns
    // -----------------------------------------------------------------------

    #[test]
    fn compile_patterns_none_returns_empty() {
        let result = compile_patterns(&None);
        assert!(result.is_empty());
    }

    #[test]
    fn compile_patterns_some_empty_vec_returns_empty() {
        let result = compile_patterns(&Some(vec![]));
        assert!(result.is_empty());
    }

    #[test]
    fn compile_patterns_valid_patterns_compile() {
        let result = compile_patterns(&Some(vec![
            "terran".to_string(),
            r".*\.anim$".to_string(),
        ]));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn compile_patterns_invalid_pattern_silently_skipped() {
        // "[invalid" is not a valid regex
        let result = compile_patterns(&Some(vec![
            "valid".to_string(),
            "[invalid".to_string(),
        ]));
        // Only the valid one is compiled
        assert_eq!(result.len(), 1);
        assert!(result[0].is_match("valid_path"));
    }

    #[test]
    fn compile_patterns_all_invalid_returns_empty() {
        let result = compile_patterns(&Some(vec![
            "[bad".to_string(),
            "**broken**(".to_string(),
        ]));
        assert!(result.is_empty());
    }

    // -----------------------------------------------------------------------
    // should_skip
    // -----------------------------------------------------------------------

    #[test]
    fn should_skip_always_returns_false() {
        let dir = tempfile::TempDir::new().unwrap();
        let existing = dir.path().join("file.txt");
        std::fs::write(&existing, b"content").unwrap();
        let t = std::time::SystemTime::now();
        // Always behavior never skips, even for existing files
        assert!(!should_skip(&existing, OverwriteBehavior::Always, t));
        // Also false for non-existent paths
        assert!(!should_skip(&dir.path().join("does_not_exist.txt"), OverwriteBehavior::Always, t));
    }

    #[test]
    fn should_skip_never_with_nonexistent_path_is_false() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.txt");
        let t = std::time::SystemTime::now();
        assert!(!should_skip(&path, OverwriteBehavior::Never, t));
    }

    #[test]
    fn should_skip_never_with_existing_file_is_true() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("existing.txt");
        std::fs::write(&path, b"hello").unwrap();
        let t = std::time::SystemTime::now();
        assert!(should_skip(&path, OverwriteBehavior::Never, t));
    }
}