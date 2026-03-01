/// casc-extractor — unified CLI entry point for StarCraft: Remastered asset extraction
///
/// Exposes all extraction functionality under one binary:
///   casc-extractor extract anim
///   casc-extractor extract tileset
///   casc-extractor extract effect
///   casc-extractor extract organized
///   casc-extractor sounds extract
///   casc-extractor sounds list
///   casc-extractor inspect sprites
///   casc-extractor inspect archive

use anyhow::Result;
use casc_extractor::anim::HdAnimFile;
use casc_extractor::casc::casclib_ffi::CascArchive;
use casc_extractor::casc::discovery::locate_starcraft;
use casc_extractor::grp::GrpFile;
use casc_extractor::mapping::SpriteMapping;
use casc_extractor::{export_anim, CascStorage, ExportConfig};
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
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
    ("select.ogg", &[
        "\\glue\\mouseover.wav",
        "\\glue\\swishlock.wav",
        "\\misc\\button.wav",
        "\\misc\\perror.wav",
        "sound/Misc/select.wav",
        "sound/Glue/select.wav",
        "glue\\mouseover.wav",
        "glue/mouseover.wav",
        "misc\\button.wav",
        "misc/button.wav",
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

    /// Output directory
    #[arg(long, global = true, default_value = "output")]
    output: PathBuf,

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
    },
}

#[derive(Subcommand)]
enum SoundsCommands {
    /// Extract known unit and UI sounds from the archive
    Extract {
        /// Output directory for extracted sounds (overrides global --output)
        #[arg(long)]
        sounds_output: Option<PathBuf>,
    },

    /// List available audio files in the archive (Zerg + UI)
    List,
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

/// Resolve the StarCraft install directory to a UTF-8 string.
fn resolve_install_str(install_path: Option<&Path>) -> Result<String> {
    let install_dir = locate_starcraft(install_path)?;
    install_dir
        .into_os_string()
        .into_string()
        .map_err(|p| anyhow::anyhow!("Install path is not valid UTF-8: {:?}", p))
}

/// Open the CascLib-backed archive, resolving the install path.
fn open_casc_archive(install_path: Option<&Path>) -> Result<CascArchive> {
    let install_str = resolve_install_str(install_path)?;
    CascArchive::open(&install_str)
        .map_err(|e| anyhow::anyhow!("Failed to open CASC archive at {}: {}", install_str, e))
}

/// Open the index-based CascStorage (used for file enumeration).
fn open_casc_storage(install_path: Option<&Path>) -> Result<CascStorage> {
    let install_str = resolve_install_str(install_path)?;
    CascStorage::open(&install_str)
        .map_err(|e| anyhow::anyhow!("Failed to open CascStorage at {}: {}", install_str, e))
}

// ---------------------------------------------------------------------------
// Subcommand handlers
// ---------------------------------------------------------------------------

fn cmd_extract_anim(
    archive: &CascArchive,
    output: &Path,
    quality: QualityLevel,
    ids: Option<Vec<u16>>,
    convert_to_png: bool,
    team_color_mask: bool,
) -> Result<()> {
    fs::create_dir_all(output)?;
    println!("Extracting Animations...");
    println!("Quality:  {}", quality.description());
    println!("Output:   {:?}\n", output);

    let prefix = quality.path_prefix();
    let id_list: Vec<u16> = ids.unwrap_or_else(|| (0..1000).collect());

    for id in id_list {
        let path = format!("{}anim/main_{:03}.anim", prefix, id);
        match archive.extract_file(&path) {
            Ok(data) => {
                let output_name = format!("main_{:03}.anim", id);
                let output_path = output.join(&output_name);
                File::create(&output_path)?.write_all(&data)?;

                if convert_to_png {
                    match HdAnimFile::parse(&data) {
                        Ok(anim) => {
                            let config = ExportConfig {
                                convert_to_png,
                                team_color_mask,
                                save_dds: true,
                            };
                            match export_anim(
                                &anim,
                                &output_path.with_extension(""),
                                &config,
                            ) {
                                Ok(result) => println!(
                                    "  {} ({} frames, {:.1} MB) tc={}",
                                    output_name,
                                    result.frame_count,
                                    data.len() as f64 / 1_000_000.0,
                                    result.tc_mask_written
                                ),
                                Err(e) => println!("  Export failed: {}", e),
                            }
                        }
                        Err(e) => println!("  {} - Parse error: {}", output_name, e),
                    }
                } else {
                    println!(
                        "  {} ({:.1} MB)",
                        output_name,
                        data.len() as f64 / 1_000_000.0
                    );
                }
            }
            Err(_) => {
                // Silently skip missing anim IDs — not every ID exists.
            }
        }
    }

    println!("\nExtraction complete!");
    println!("Files saved to: {:?}", output);
    Ok(())
}

fn cmd_extract_tileset(
    archive: &CascArchive,
    output: &Path,
    quality: QualityLevel,
    _convert_to_png: bool,
) -> Result<()> {
    fs::create_dir_all(output)?;
    println!("Extracting Tilesets...");
    println!("Quality: {}", quality.description());
    println!("Output:  {:?}\n", output);

    let prefix = quality.path_prefix();
    let tilesets = [
        "badlands", "platform", "ashworld", "jungle",
        "desert", "ice", "twilight", "install",
    ];

    for tileset in &tilesets {
        let path = format!("{}tileset/{}.dds.vr4", prefix, tileset);
        match archive.extract_file(&path) {
            Ok(data) => {
                let output_name = format!("{}.dds.vr4", tileset);
                let output_path = output.join(&output_name);
                File::create(&output_path)?.write_all(&data)?;
                println!("  {} ({:.1} MB)", output_name, data.len() as f64 / 1_000_000.0);
            }
            Err(e) => println!("  {} - {}", tileset, e),
        }
    }

    println!("\nDone. Files saved to: {:?}", output);
    Ok(())
}

fn cmd_extract_effect(
    archive: &CascArchive,
    output: &Path,
    quality: QualityLevel,
    _convert_to_png: bool,
) -> Result<()> {
    fs::create_dir_all(output)?;
    println!("Extracting Effects...");
    println!("Quality: {}", quality.description());
    println!("Output:  {:?}\n", output);

    let prefix = quality.path_prefix();
    let effects = ["water_normal_1.dds.grp", "water_normal_2.dds.grp"];

    for effect in &effects {
        let path = format!("{}effect/{}", prefix, effect);
        match archive.extract_file(&path) {
            Ok(data) => {
                let output_path = output.join(effect);
                File::create(&output_path)?.write_all(&data)?;
                println!("  {} ({:.1} MB)", effect, data.len() as f64 / 1_000_000.0);
            }
            Err(e) => println!("  {} - {}", effect, e),
        }
    }

    println!("\nDone. Files saved to: {:?}", output);
    Ok(())
}

fn cmd_extract_organized(
    archive: &CascArchive,
    output: &Path,
    mapping_file: &Path,
) -> Result<()> {
    let mapping = SpriteMapping::load(mapping_file).map_err(|e| {
        anyhow::anyhow!("Failed to load mapping file '{:?}': {}", mapping_file, e)
    })?;

    println!("StarCraft Sprite Extraction (Organized)");
    println!("Using mapping: {:?}", mapping_file);
    println!("--------------------------------------------------------\n");

    let mut stats: HashMap<String, usize> = HashMap::new();
    let mut total_success = 0usize;
    let mut total_failed = 0usize;

    for (category_path, file_path) in &mapping.entries {
        let output_path = output.join(category_path);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        match archive.extract_file(file_path) {
            Ok(data) => match GrpFile::parse(&data) {
                Ok(grp) => {
                    let frames_per_row = 17usize;
                    let rows =
                        (grp.frame_count as usize + frames_per_row - 1) / frames_per_row;
                    let sheet_width = grp.width as u32 * frames_per_row as u32;
                    let sheet_height = grp.height as u32 * rows as u32;

                    let mut sheet_data =
                        vec![0u8; (sheet_width * sheet_height * 4) as usize];

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
                                    let dst_idx =
                                        (dst_y * sheet_width as usize + dst_x) * 4;
                                    sheet_data[dst_idx..dst_idx + 4]
                                        .copy_from_slice(&rgba[src_idx..src_idx + 4]);
                                }
                            }
                        }
                    }

                    let png_path = output_path.with_extension("png");
                    let file = File::create(&png_path)?;
                    let w = BufWriter::new(file);
                    let mut encoder = png::Encoder::new(w, sheet_width, sheet_height);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    let mut writer = encoder.write_header()?;
                    writer.write_image_data(&sheet_data)?;

                    let meta_txt = format!(
                        "frames: {}\nframe_size: {}x{}\nsheet_size: {}x{}\nlayout: {}x{}\n",
                        grp.frame_count,
                        grp.width,
                        grp.height,
                        sheet_width,
                        sheet_height,
                        frames_per_row,
                        rows
                    );
                    fs::write(output_path.with_extension("txt"), meta_txt)?;

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
                                    "\n    {{\"index\": {}, \"x\": {}, \"y\": {}, \
                                     \"width\": {}, \"height\": {}}}",
                                    i, x, y, grp.width, grp.height
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(",")
                    );
                    fs::write(output_path.with_extension("json"), unity_meta)?;

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

    println!("\n--------------------------------------------------------");
    println!("Statistics:");
    for (category, count) in &stats {
        println!("   {}: {} sprites", category, count);
    }
    println!("\nSuccess: {} | Failed: {}", total_success, total_failed);
    println!("Output: {:?}", output);
    Ok(())
}

fn cmd_sounds_extract(archive: &CascArchive, output: &Path) -> Result<()> {
    println!("  StarCraft Sound Extractor");
    println!("==================================================");
    println!("  Opened CASC archive");

    fs::create_dir_all(output)?;

    let mut extracted = 0usize;

    for (out_name, candidates) in SOUND_TARGETS {
        let dest = output.join(out_name);
        if dest.exists() {
            println!("  {} already exists, skipping", out_name);
            extracted += 1;
            continue;
        }

        let mut found = false;
        for casc_path in *candidates {
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
            println!(
                "  {} -- none of {} candidates succeeded",
                out_name,
                candidates.len()
            );
        }
    }

    println!(
        "\n== Result ==================================================\n  \
         {}/{} sounds extracted to {}",
        extracted,
        SOUND_TARGETS.len(),
        output.display()
    );
    println!(
        "\nIf any are missing, check exact paths with:\n  \
         casc-extractor sounds list"
    );
    Ok(())
}

fn cmd_sounds_list(install_path: Option<&Path>) -> Result<()> {
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
        "Scanning anim IDs 0-{} at quality {}...\n",
        max_id - 1,
        quality.description()
    );
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

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // ------------------------------------------------------------------
        Commands::Extract { target } => {
            let archive = open_casc_archive(cli.install_path.as_deref())?;
            match target {
                ExtractCommands::Anim {
                    quality,
                    ids,
                    convert_to_png,
                    team_color_mask,
                } => {
                    cmd_extract_anim(
                        &archive,
                        &cli.output,
                        quality,
                        ids,
                        convert_to_png,
                        team_color_mask,
                    )?;
                }
                ExtractCommands::Tileset {
                    quality,
                    convert_to_png,
                } => {
                    cmd_extract_tileset(&archive, &cli.output, quality, convert_to_png)?;
                }
                ExtractCommands::Effect {
                    quality,
                    convert_to_png,
                } => {
                    cmd_extract_effect(&archive, &cli.output, quality, convert_to_png)?;
                }
                ExtractCommands::Organized { mapping } => {
                    cmd_extract_organized(&archive, &cli.output, &mapping)?;
                }
            }
        }

        // ------------------------------------------------------------------
        Commands::Sounds { action } => match action {
            SoundsCommands::Extract { sounds_output } => {
                let out = sounds_output.unwrap_or_else(|| cli.output.clone());
                let archive = open_casc_archive(cli.install_path.as_deref())?;
                cmd_sounds_extract(&archive, &out)?;
            }
            SoundsCommands::List => {
                cmd_sounds_list(cli.install_path.as_deref())?;
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
    }

    Ok(())
}
