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

use anyhow::Result;
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
    /// Extract HD animations to ANIM (optionally converted to PNG + JSON metadata)
    Anim {
        /// Quality level
        #[arg(long, value_enum, default_value = "hd4")]
        quality: QualityLevel,

        /// Specific animation IDs to extract, comma-separated (e.g. 0,1,7)
        #[arg(long, value_delimiter = ',')]
        ids: Option<Vec<u16>>,

        /// Convert ANIM to PNG (extracts diffuse layer)
        #[arg(long)]
        convert_to_png: bool,

        /// Export team-color mask alongside diffuse PNG (requires --convert-to-png)
        #[arg(long)]
        team_color_mask: bool,

        /// JSON file mapping anim IDs to unit names (e.g. {"0": "marine", "1": "ghost"})
        #[arg(long)]
        name_map: Option<PathBuf>,

        /// Comma-separated list of ANIM layers to export (requires --convert-to-png).
        /// Valid: diffuse,teamcolor,normal,specular,emissive,ao  (default: diffuse)
        #[arg(long, value_delimiter = ',', default_values = ["diffuse"])]
        layers: Vec<String>,

        /// Write the raw diffuse DDS file alongside the PNG
        #[arg(long)]
        save_dds: bool,
    },

    /// Extract HD tilesets
    Tileset {
        /// Quality level
        #[arg(long, value_enum, default_value = "hd4")]
        quality: QualityLevel,

        /// Convert extracted DDS to PNG
        #[arg(long)]
        convert_to_png: bool,
    },

    /// Extract HD effects
    Effect {
        /// Quality level
        #[arg(long, value_enum, default_value = "hd4")]
        quality: QualityLevel,

        /// Convert extracted DDS/GRP to PNG
        #[arg(long)]
        convert_to_png: bool,
    },

    /// Extract sprites via YAML mapping file
    Organized {
        /// Path to the YAML sprite mapping file
        #[arg(long, default_value = "mappings/starcraft-remastered.yaml")]
        mapping: PathBuf,

        /// Quality level: sd extracts GRP spritesheets, hd4/hd2 extract raw ANIM files
        #[arg(long, value_enum, default_value = "sd")]
        quality: QualityLevel,

        /// Convert extracted ANIM files to PNG (only applies to hd4/hd2 quality)
        #[arg(long)]
        convert_to_png: bool,

        /// Export team-color mask alongside diffuse PNG (requires --convert-to-png, hd4/hd2 only)
        #[arg(long)]
        team_color_mask: bool,

        /// Comma-separated list of ANIM layers to export (requires --convert-to-png, hd4/hd2 only).
        /// Valid: diffuse,teamcolor,normal,specular,emissive,ao  (default: diffuse)
        #[arg(long, value_delimiter = ',', default_values = ["diffuse"])]
        layers: Vec<String>,

        /// Write the raw diffuse DDS file alongside the PNG (hd4/hd2 only)
        #[arg(long)]
        save_dds: bool,
    },
}

#[derive(Subcommand)]
enum SoundsCommands {
    /// Extract known unit and UI sounds from the archive
    Extract {
        /// Output directory for extracted sounds (overrides global --output)
        #[arg(long)]
        sounds_output: Option<PathBuf>,

        /// Path to a JSON targets file produced by `sounds export-targets`.
        /// When supplied, replaces the built-in target list.
        #[arg(long)]
        targets: Option<PathBuf>,
    },

    /// List available audio files in the archive (Zerg + UI).
    /// Use --search to filter all archive paths by a pattern.
    List {
        /// Case-insensitive substring to filter archive paths (e.g. "zergling").
        /// When provided, all matches are printed and the 30-result cap is lifted.
        #[arg(long)]
        search: Option<String>,
    },

    /// Write the built-in sound targets to a JSON file for customisation
    ExportTargets {
        /// Path to write the JSON targets file
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
    convert_to_png: bool,
    team_color_mask: bool,
    name_map: Option<PathBuf>,
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

    // Load optional name map (ID -> unit name).
    let id_to_name: HashMap<String, String> = match name_map {
        Some(ref p) => {
            let raw = fs::read_to_string(p)
                .map_err(|e| anyhow::anyhow!("Failed to read name map {:?}: {}", p, e))?;
            serde_json::from_str::<HashMap<String, String>>(&raw)
                .map_err(|e| anyhow::anyhow!("Failed to parse name map {:?}: {}", p, e))?
        }
        None => HashMap::new(),
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
                let output_name = format!("main_{:03}.anim", id);
                let mapped_name = id_to_name_ref.get(&id.to_string()).cloned();

                if let Some(ref name) = mapped_name {
                    let named_path = output_ref.join(format!("{}.anim", name));
                    if let Err(e) = fs::copy(output_path, &named_path) {
                        eprintln!("Warning: could not write named copy {:?}: {}", named_path, e);
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
                                        output_name,
                                        result.frame_count,
                                        data.len() as f64 / 1_000_000.0,
                                        result.tc_mask_written
                                    );
                                } else {
                                    println!(
                                        "  {} ({} frames, {:.1} MB) tc={}",
                                        output_name,
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
                    Err(e) => Err(anyhow::anyhow!("  {} - Parse error: {}", output_name, e)),
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
                        output_name,
                        data.len() as f64 / 1_000_000.0
                    );
                } else {
                    println!(
                        "  {} ({:.1} MB)",
                        output_name,
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
            println!("  {} (skipped)", output_name);
            continue;
        }
        let path = format!("{}tileset/{}.dds.vr4", prefix, tileset);
        match archive.extract_file(&path) {
            Ok(data) => extracted.push((output_name, output_path, data)),
            Err(e) => println!("  {} - {}", tileset, e),
        }
    }

    // Phase 2 — Parallel: write files to disk (I/O-bound, no archive access).
    let results: Vec<anyhow::Result<()>> = extracted
        .par_iter()
        .map(|(output_name, output_path, data)| -> anyhow::Result<()> {
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
            println!("  {} ({:.1} MB){}", output_name, data.len() as f64 / 1_000_000.0, note);
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
        .map(|(output_name, output_path, data)| -> anyhow::Result<()> {
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
            println!("  {} ({:.1} MB){}", output_name, data.len() as f64 / 1_000_000.0, note);
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

/// Write .txt metadata for a GRP spritesheet.
fn write_text_metadata(grp: &GrpFile, path: &Path) -> Result<()> {
    let frames_per_row = 17usize;
    let rows = (grp.frame_count as usize).div_ceil(frames_per_row);
    let sheet_width = grp.width as u32 * frames_per_row as u32;
    let sheet_height = grp.height as u32 * rows as u32;

    let meta_txt = format!(
        "frames: {}\nframe_size: {}x{}\nsheet_size: {}x{}\nlayout: {}x{}\n",
        grp.frame_count, grp.width, grp.height, sheet_width, sheet_height, frames_per_row, rows
    );
    fs::write(path, meta_txt)?;
    Ok(())
}

/// Write .json (Unity) metadata for a GRP spritesheet.
fn write_json_metadata(grp: &GrpFile, path: &Path) -> Result<()> {
    let frames_per_row = 17usize;
    let rows = (grp.frame_count as usize).div_ceil(frames_per_row);
    let sheet_width = grp.width as u32 * frames_per_row as u32;
    let sheet_height = grp.height as u32 * rows as u32;

    let unity_meta = format!(
        r#"{{
  "frameCount": {},
  "frameWidth": {},
  "frameHeight": {},
  "framesPerRow": {},
  "rows": {},
  "sheetWidth": {},
  "sheetHeight": {},
  "frames": [{}
  ]
}}"#,
        grp.frame_count,
        grp.width,
        grp.height,
        frames_per_row,
        rows,
        sheet_width,
        sheet_height,
        (0..grp.frame_count)
            .map(|i| {
                let col = i as usize % frames_per_row;
                let row_i = i as usize / frames_per_row;
                let x = col * grp.width as usize;
                let y = row_i * grp.height as usize;
                format!(
                    "\n    {{\"index\": {}, \"x\": {}, \"y\": {}, \"width\": {}, \"height\": {}}}",
                    i, x, y, grp.width, grp.height
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    );
    fs::write(path, unity_meta)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_extract_organized(
    archive: &CascArchive,
    output: &Path,
    mapping_file: &Path,
    quality: QualityLevel,
    convert_to_png: bool,
    team_color_mask: bool,
    layers: Vec<String>,
    save_dds: bool,
    config: &ExtractionConfig,
) -> Result<()> {
    let session_start = std::time::SystemTime::now();

    let mapping = SpriteMapping::load(mapping_file).map_err(|e| {
        anyhow::anyhow!("Failed to load mapping file '{:?}': {}", mapping_file, e)
    })?;

    println!("StarCraft Sprite Extraction (Organized)");
    println!("Quality:  {}", quality.description());
    println!("Using mapping: {:?}", mapping_file);
    println!("--------------------------------------------------------\n");

    let overwrite   = config.output_settings.overwrite_behavior;
    let gen_meta    = config.output_settings.metadata_options.generate_json;
    let compression = png_compression(config.quality_settings.png_compression_level);
    let pixels_per_unit = config.output_settings.unity_settings.pixels_per_unit;
    let include     = compile_patterns(&config.filter_settings.include_patterns);
    let exclude     = compile_patterns(&config.filter_settings.exclude_patterns);

    let mut stats: HashMap<String, usize> = HashMap::new();
    let mut total_success = 0usize;
    let mut total_failed = 0usize;

    let mut progress = casc_extractor::ProgressReporter::new(
        mapping.entries.len() as u64,
        config.feedback_settings.verbose_logging,
    );

    for (category_path, file_path) in &mapping.entries {
        progress.update_current_file(category_path);

        if !passes_filter(category_path, &include, &exclude) {
            progress.increment();
            continue;
        }

        let output_path = output.join(category_path);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        match quality {
            QualityLevel::Sd => {
                if should_skip(&output_path.with_extension("png"), overwrite, session_start) {
                    progress.increment();
                    continue;
                }

                match archive.extract_file(file_path) {
                    Ok(data) => match GrpFile::parse(&data) {
                        Ok(grp) => {
                            build_spritesheet(&grp, &output_path.with_extension("png"), compression)?;
                            if gen_meta {
                                write_text_metadata(&grp, &output_path.with_extension("txt"))?;
                                write_json_metadata(&grp, &output_path.with_extension("json"))?;
                            }

                            println!("  {}", category_path);
                            let category = category_path
                                .split('/')
                                .next()
                                .unwrap_or(category_path)
                                .to_string();
                            *stats.entry(category).or_insert(0) += 1;
                            total_success += 1;
                        }
                        Err(e) => {
                            println!("  {}: Parse error - {}", category_path, e);
                            total_failed += 1;
                        }
                    },
                    Err(_) => {
                        total_failed += 1;
                    }
                }
            }

            QualityLevel::Hd4 | QualityLevel::Hd2 => {
                let anim_path = output_path.with_extension("anim");
                if should_skip(&anim_path, overwrite, session_start) {
                    progress.increment();
                    continue;
                }

                match archive.extract_file(file_path) {
                    Ok(data) => {
                        if let Err(e) = File::create(&anim_path).and_then(|mut f| f.write_all(&data)) {
                            eprintln!("Warning: could not write {:?}: {}", anim_path, e);
                            total_failed += 1;
                        } else {
                            println!("  {} ({:.1} MB)", category_path, data.len() as f64 / 1_000_000.0);

                            if convert_to_png {
                                match HdAnimFile::parse(&data) {
                                    Ok(anim) => {
                                        let export_cfg = ExportConfig {
                                            convert_to_png: true,
                                            team_color_mask,
                                            save_dds,
                                            generate_metadata: gen_meta,
                                            pixels_per_unit,
                                            layers: layers.clone(),
                                        };
                                        match export_anim(&anim, &output_path, &export_cfg) {
                                            Ok(result) => println!(
                                                "    -> {} frames exported as PNG",
                                                result.frame_count
                                            ),
                                            Err(e) => println!("    -> ANIM export failed: {}", e),
                                        }
                                    }
                                    Err(e) => println!("    -> ANIM parse failed: {}", e),
                                }
                            }

                            let category = category_path
                                .split('/')
                                .next()
                                .unwrap_or(category_path)
                                .to_string();
                            *stats.entry(category).or_insert(0) += 1;
                            total_success += 1;
                        }
                    }
                    Err(_) => {
                        total_failed += 1;
                    }
                }
            }
        }

        progress.increment();
    }

    progress.finish(total_success as u64, total_failed as u64);

    println!("\n--------------------------------------------------------");
    println!("Statistics:");
    for (category, count) in &stats {
        println!("   {}: {} sprites", category, count);
    }
    println!("\nSuccess: {} | Failed: {}", total_success, total_failed);
    println!("Output: {:?}", output);
    Ok(())
}

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
                    convert_to_png,
                    team_color_mask,
                    name_map,
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
                        convert_to_png,
                        team_color_mask,
                        name_map,
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
                    cmd_extract_tileset(&archive, &output, quality, convert_to_png, &config)?;
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
                    cmd_extract_effect(&archive, &output, quality, convert_to_png, &config)?;
                }
                ExtractCommands::Organized { mapping, quality, convert_to_png, team_color_mask, layers, save_dds } => {
                    let convert_to_png = convert_to_png
                        || matches!(
                            config.quality_settings.format_filter,
                            FormatFilterOption::Png | FormatFilterOption::Images
                        );
                    cmd_extract_organized(&archive, &output, &mapping, quality, convert_to_png, team_color_mask, layers, save_dds, &config)?;
                }
            }
        }

        // ------------------------------------------------------------------
        Commands::Sounds { action } => match action {
            SoundsCommands::Extract { sounds_output, targets } => {
                let out = sounds_output.unwrap_or(output);
                let archive = open_casc_archive(cli.install_path.as_deref())?;
                let storage = open_casc_storage(cli.install_path.as_deref())?;
                cmd_sounds_extract(&archive, &storage, &out, targets.as_deref())?;
            }
            SoundsCommands::List { search } => {
                cmd_sounds_list(cli.install_path.as_deref(), search)?;
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
