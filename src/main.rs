//! casc-extractor — CLI entry point for StarCraft: Remastered asset extraction.
//!
//! This file holds the clap definitions and dispatch only; the handlers live in
//! `commands::*`, grouped by subject:
//!   support     — path safety, overwrite policy, config, archive opening
//!   sprites     — extract anim / tileset / effect
//!   terrain     — tileset atlases, map-terrain, VR4 and CHK parsing
//!   assets      — cmdicons, mainmenu, dat, raw
//!   sounds      — sounds extract / list / probe / export-targets
//!   inspect     — inspect sprites / archive / files
//!   maintenance — config init, validate register / run
mod commands;
mod fallbacks;

use commands::assets::*;
use commands::inspect::*;
use commands::maintenance::*;
use commands::sounds::*;
use commands::sprites::*;
use commands::support::*;
use commands::terrain::*;

use anyhow::{Context as _, Result};
use casc_extractor::casc::casclib_ffi::CascArchive;
use casc_extractor::config::FormatFilterOption;
use clap::{Parser, Subcommand, ValueEnum};
use std::fs::{self};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Quality level (shared by extract anim/tileset/effect + inspect sprites)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum QualityLevel {
    /// Original SD quality (GRP format)
    Sd,
    /// 2x HD quality
    Hd2,
    /// 4x Ultra HD quality (default)
    Hd4,
}

impl QualityLevel {
    pub fn path_prefix(self) -> &'static str {
        match self {
            QualityLevel::Sd => "SD/",
            QualityLevel::Hd2 => "HD2/",
            QualityLevel::Hd4 => "",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            QualityLevel::Sd => "SD (Original)",
            QualityLevel::Hd2 => "2x HD",
            QualityLevel::Hd4 => "4x Ultra HD",
        }
    }
}

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

    /// Output directory (overrides config file). Accepted before or after the
    /// subcommand; when given, it is used verbatim, so subcommands that would
    /// otherwise nest their output (e.g. `<output>/map-terrain/`) write directly
    /// into it.
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

    /// Run SHA256 regression checks only against a directory of extracted files
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
        #[arg(long, default_value_t = 32, value_parser = clap::value_parser!(u32).range(1..=4096))]
        cols: u32,

        /// Max tiles to extract (0 = all)
        #[arg(long, default_value_t = 512)]
        max_tiles: u32,

        /// Output tile size in pixels (tiles are scaled to this size)
        #[arg(long, default_value_t = 32, value_parser = clap::value_parser!(u32).range(1..=4096))]
        tile_size: u32,

        /// Output directory
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Build a megatile atlas from VX4EX + VR4 data (game-ready terrain tiles).
    /// Always uses full-resolution art: VX4EX indices address the HD4 tile array,
    /// so there is no quality option here.
    TilesetMegatile {
        /// Which tileset to extract
        #[arg(long, default_value = "jungle")]
        tileset: String,

        /// Max megatiles to extract (0 = all; typical map uses ~2000 unique megatiles)
        #[arg(long, default_value_t = 1024)]
        max_tiles: u32,

        /// Output tile size per megatile in atlas (32 = game tile size)
        #[arg(long, default_value_t = 32, value_parser = clap::value_parser!(u32).range(1..=4096))]
        tile_size: u32,

        /// Atlas columns
        #[arg(long, default_value_t = 32, value_parser = clap::value_parser!(u32).range(1..=4096))]
        cols: u32,

        /// Output directory
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Extract the exact terrain tiles used by a specific BW map (.scm/.scx),
    /// or a raw scenario.chk file (via --chk, e.g. an already-extracted
    /// campaign mission that isn't MPQ-wrapped).
    /// Output: `<output>/map-terrain/` by default.
    MapTerrain {
        /// Path to the .scm or .scx map file (MPQ-wrapped). Mutually
        /// exclusive with --chk.
        #[arg(long, required_unless_present = "chk", conflicts_with = "chk")]
        scm: Option<PathBuf>,

        /// Path to a raw (non-MPQ) scenario.chk file. Requires --name for the
        /// output filename stem, since a raw campaign chk is always literally
        /// "scenario.chk" and its own file stem is useless.
        #[arg(long, requires = "name")]
        chk: Option<PathBuf>,

        /// Output filename stem when using --chk (e.g. "Terran01" →
        /// Terran01_terrain.{png,json} + Terran01_minimap.png).
        #[arg(long)]
        name: Option<String>,

        /// Columns in output atlas
        #[arg(long, default_value_t = 32, value_parser = clap::value_parser!(u32).range(1..=4096))]
        cols: u32,

        /// Tile size in output atlas (pixels per megatile)
        #[arg(long, default_value_t = 32, value_parser = clap::value_parser!(u32).range(1..=4096))]
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
        #[arg(long, default_value_t = 32, value_parser = clap::value_parser!(u32).range(1..=4096))]
        cols: u32,

        /// Tile size in output atlas (pixels per megatile)
        #[arg(long, default_value_t = 32, value_parser = clap::value_parser!(u32).range(1..=4096))]
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
        /// Output directory for extracted sounds (overrides the global --output).
        /// Default: `<output>/sounds/`
        #[arg(long)]
        sounds_output: Option<PathBuf>,

        /// Path to a custom JSON targets file (produced by export-targets)
        #[arg(long)]
        targets: Option<PathBuf>,

        /// Exit with a non-zero status if not all sound targets were extracted
        #[arg(long)]
        fail_on_incomplete: bool,
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

        /// Exit with a non-zero status if not all probed sound paths resolved
        #[arg(long)]
        fail_on_incomplete: bool,
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
    let output = cli
        .output
        .unwrap_or_else(|| config.output_settings.output_directory.clone());

    // --validate-only is a global dry run: confirm the archive opens and files are
    // visible, then exit before any command can write. It used to be checked only
    // inside `extract`, so `sounds extract --validate-only` happily wrote files.
    if cli.validate_only {
        let storage = open_casc_storage(cli.install_path.as_deref())?;
        let files = storage
            .list_files()
            .map_err(|e| anyhow::anyhow!("list_files failed: {}", e))?;
        println!("Archive opened successfully. {} files found.", files.len());
        for f in files.iter().take(5) {
            println!("  {}", f);
        }
        println!("(--validate-only: no output written)");
        return Ok(());
    }

    match cli.command {
        // ------------------------------------------------------------------
        Commands::Extract { target } => {
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
                    cmd_extract_tileset(
                        &archive,
                        &output.join("tilesets"),
                        quality,
                        convert_to_png,
                        &config,
                    )?;
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
                    cmd_extract_effect(
                        &archive,
                        &output.join("effects"),
                        quality,
                        convert_to_png,
                        &config,
                    )?;
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
                    cmd_extract_tileset_atlas(
                        &archive, &tileset, quality, cols, max_tiles, tile_size, &out,
                    )?;
                }
                ExtractCommands::TilesetMegatile {
                    tileset,
                    max_tiles,
                    tile_size,
                    cols,
                    output: mega_output,
                } => {
                    let out = mega_output.unwrap_or_else(|| output.join("tileset-megatile"));
                    cmd_extract_tileset_megatile(
                        &archive, &tileset, max_tiles, tile_size, cols, &out,
                    )?;
                }
                ExtractCommands::MapTerrain {
                    scm,
                    chk,
                    name,
                    cols,
                    tile_size,
                    output: terrain_output,
                } => {
                    let out = terrain_output.unwrap_or_else(|| output.join("map-terrain"));
                    cmd_extract_map_terrain(
                        &archive,
                        scm.as_deref(),
                        chk.as_deref(),
                        name.as_deref(),
                        cols,
                        tile_size,
                        &out,
                    )?;
                }
                ExtractCommands::MapTerrainDual {
                    scm,
                    cols,
                    tile_size,
                    output: terrain_output,
                } => {
                    let out = terrain_output.unwrap_or_else(|| output.join("map-terrain"));
                    cmd_extract_map_terrain_dual(&archive, &scm, cols, tile_size, &out)?;
                }
                ExtractCommands::Mainmenu {
                    output: mm_output,
                    convert_dds,
                } => {
                    let out = mm_output.unwrap_or_else(|| output.join("ui/mainmenu"));
                    cmd_extract_mainmenu(&archive, &out, convert_dds)?;
                }
                ExtractCommands::Dat { output: dat_output } => {
                    let out = dat_output.unwrap_or_else(|| output.join("dat"));
                    cmd_extract_dat(&archive, &out)?;
                }
                ExtractCommands::Cmdicons {
                    output: cmd_output,
                    tint,
                    save_raw,
                } => {
                    let out = cmd_output.unwrap_or_else(|| output.join("ui/cmdicons"));
                    let tint_rgb = [
                        tint.first().copied().unwrap_or(255),
                        tint.get(1).copied().unwrap_or(254),
                        tint.get(2).copied().unwrap_or(84),
                    ];
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
                        // CascOpenOnlineStorage wants `cache_path:product:region`,
                        // not an install directory: the cache is where CDN-fetched
                        // data lands, and the product code selects the game (`s1`
                        // is StarCraft: Remastered).
                        let cache_dir = dirs::cache_dir()
                            .unwrap_or_else(std::env::temp_dir)
                            .join("casc-extractor-online");
                        fs::create_dir_all(&cache_dir).with_context(|| {
                            format!("creating online cache directory {}", cache_dir.display())
                        })?;
                        let cache_str = cache_dir.to_str().ok_or_else(|| {
                            anyhow::anyhow!("Cache path is not valid UTF-8: {:?}", cache_dir)
                        })?;
                        let region = detect_cdn_region(cli.install_path.as_deref())
                            .unwrap_or_else(|| ONLINE_REGION_FALLBACK.to_string());
                        // Fields are separated by CASC_PARAM_SEPARATOR ('*'), not ':'
                        // — a colon would be swallowed by Windows drive-letter paths.
                        let params = format!("{}*{}*{}", cache_str, ONLINE_PRODUCT, region);
                        let online_archive = CascArchive::open_online(&params)
                            .map_err(|e| anyhow::anyhow!(
                                "Failed to open online CASC storage ({}): {}\n\
                                 Online mode downloads from Blizzard's CDN and needs network access.",
                                params, e
                            ))?;
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
            SoundsCommands::Extract {
                sounds_output,
                targets,
                fail_on_incomplete,
            } => {
                let out = sounds_output.unwrap_or_else(|| output.join("sounds"));
                let archive = open_casc_archive(cli.install_path.as_deref())?;
                let storage = open_casc_storage(cli.install_path.as_deref())?;
                cmd_sounds_extract(
                    &archive,
                    &storage,
                    &out,
                    targets.as_deref(),
                    fail_on_incomplete,
                )?;
            }
            SoundsCommands::List { search } => {
                cmd_sounds_list(cli.install_path.as_deref(), search)?;
            }
            SoundsCommands::Probe {
                filter,
                fail_on_incomplete,
            } => {
                cmd_sounds_probe(
                    cli.install_path.as_deref(),
                    filter.as_deref(),
                    fail_on_incomplete,
                )?;
            }
            SoundsCommands::ExportTargets {
                output: targets_output,
            } => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use casc_extractor::config::OverwriteBehavior;
    use std::path::Path;

    // -----------------------------------------------------------------------
    // join_within
    // -----------------------------------------------------------------------

    #[test]
    fn join_within_allows_plain_and_nested_names() {
        let base = Path::new("/out");
        assert_eq!(
            join_within(base, "marine.anim").unwrap(),
            base.join("marine.anim")
        );
        assert_eq!(
            join_within(base, "terran/marine.anim").unwrap(),
            base.join("terran").join("marine.anim")
        );
        // Archive-style backslashes are normalised, not treated as one filename.
        assert_eq!(
            join_within(base, "terran\\marine.anim").unwrap(),
            base.join("terran").join("marine.anim")
        );
    }

    #[test]
    fn join_within_rejects_escapes() {
        let base = Path::new("/out");
        for bad in [
            "../escaped.anim",
            "terran/../../escaped.anim",
            "/etc/passwd",
            "",
            "..",
        ] {
            assert!(
                join_within(base, bad).is_err(),
                "{:?} should be rejected as an output name",
                bad
            );
        }
    }

    // -----------------------------------------------------------------------
    // tileset_name_from_era
    // -----------------------------------------------------------------------

    #[test]
    fn tileset_name_from_era_matches_bw_ids() {
        // Canonical Brood War tileset ids; 3 is ashworld, not jungle.
        assert_eq!(tileset_name_from_era(0), "badlands");
        assert_eq!(tileset_name_from_era(1), "platform");
        assert_eq!(tileset_name_from_era(2), "install");
        assert_eq!(tileset_name_from_era(3), "ashworld");
        assert_eq!(tileset_name_from_era(4), "jungle");
        assert_eq!(tileset_name_from_era(5), "desert");
        assert_eq!(tileset_name_from_era(6), "ice");
        assert_eq!(tileset_name_from_era(7), "twilight");
        // CHK stores the tileset in the low 3 bits, so higher bits wrap.
        assert_eq!(tileset_name_from_era(11), "ashworld");
    }

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
        let result = compile_patterns(&Some(vec!["terran".to_string(), r".*\.anim$".to_string()]));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn compile_patterns_invalid_pattern_silently_skipped() {
        // "[invalid" is not a valid regex
        let result = compile_patterns(&Some(vec!["valid".to_string(), "[invalid".to_string()]));
        // Only the valid one is compiled
        assert_eq!(result.len(), 1);
        assert!(result[0].is_match("valid_path"));
    }

    #[test]
    fn compile_patterns_all_invalid_returns_empty() {
        let result = compile_patterns(&Some(vec!["[bad".to_string(), "**broken**(".to_string()]));
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
        assert!(!should_skip(
            &dir.path().join("does_not_exist.txt"),
            OverwriteBehavior::Always,
            t
        ));
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
