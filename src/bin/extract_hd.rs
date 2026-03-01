// Unified HD extraction tool with quality level support

use anyhow::Result;
use casc_extractor::casc::casclib_ffi::CascArchive;
use casc_extractor::casc::discovery::locate_starcraft;
use casc_extractor::{export_anim, ExportConfig};
use casc_extractor::anim::HdAnimFile;
use clap::{Parser, ValueEnum};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum QualityLevel {
    /// SD - Original quality (GRP format)
    Sd,
    /// HD2 - 2x HD quality (1.1MB animations)
    Hd2,
    /// HD4 - 4x Ultra HD quality (4.5MB animations)
    Hd4,
}

impl QualityLevel {
    fn path_prefix(&self) -> &'static str {
        match self {
            QualityLevel::Sd => "SD/",
            QualityLevel::Hd2 => "HD2/",
            QualityLevel::Hd4 => "",  // No prefix for 4x HD
        }
    }

    fn description(&self) -> &'static str {
        match self {
            QualityLevel::Sd => "SD (Original)",
            QualityLevel::Hd2 => "2x HD",
            QualityLevel::Hd4 => "4x Ultra HD",
        }
    }
}

#[derive(Parser)]
#[command(name = "extract-hd")]
#[command(about = "Extract HD assets from StarCraft: Remastered", long_about = None)]
struct Args {
    /// Quality level to extract
    #[arg(short, long, value_enum, default_value = "hd4")]
    quality: QualityLevel,

    /// Output directory
    #[arg(short, long, default_value = "output/hd")]
    output: PathBuf,

    /// Extract animations
    #[arg(long)]
    animations: bool,

    /// Extract tilesets
    #[arg(long)]
    tilesets: bool,

    /// Extract effects
    #[arg(long)]
    effects: bool,

    /// Extract all (animations, tilesets, effects)
    #[arg(long)]
    all: bool,

    /// Convert ANIM to PNG (extracts diffuse layer)
    #[arg(long)]
    convert_to_png: bool,

    /// Export team-color mask alongside diffuse PNG.
    ///
    /// When set (and --convert-to-png is also active), two extra outputs are
    /// produced next to the normal diffuse PNG:
    ///
    ///   <name>_tc.png   – grayscale+alpha mask (R=G=B=BT.601 luminance of
    ///                      the TC layer pixel, A=TC alpha).
    ///
    /// Additionally the diffuse PNG has its hue stripped for every pixel
    /// where the TC layer is non-zero, replacing R/G/B with their luminance
    /// so the Unity shader can apply per-team colour at runtime.
    ///
    /// If layer 1 (the team-colour layer) is absent in a given .anim file
    /// this flag is silently ignored for that file.
    #[arg(long)]
    team_color_mask: bool,

    /// Specific animation IDs to extract (e.g., 0,1,7 for marine,ghost,scv)
    #[arg(long, value_delimiter = ',')]
    anim_ids: Option<Vec<u16>>,

    /// StarCraft installation directory (auto-detected if omitted)
    #[arg(long)]
    install_path: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let install_dir = locate_starcraft(args.install_path.as_deref())?;
    let install_str = install_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Install path is not valid UTF-8: {:?}", install_dir))?;
    let archive = CascArchive::open(install_str)
        .map_err(|e| anyhow::anyhow!("Failed to open CASC archive at {}: {}", install_str, e))?;

    fs::create_dir_all(&args.output)?;

    println!("StarCraft: Remastered HD Extraction");
    println!("Quality: {}", args.quality.description());
    println!("Output: {:?}\n", args.output);

    let extract_all = args.all;
    let prefix = args.quality.path_prefix();

    // Extract animations
    if args.animations || extract_all {
        println!("Extracting Animations...");
        let ids = args.anim_ids.clone().unwrap_or_else(|| {
            // Extract all possible animations (0-999 covers everything)
            (0..1000).collect()
        });

        for id in ids {
            let path = format!("{}anim/main_{:03}.anim", prefix, id);
            match archive.extract_file(&path) {
                Ok(data) => {
                    let output_name = format!("main_{:03}.anim", id);
                    let output_path = args.output.join(&output_name);

                    // Always save raw ANIM for reference
                    File::create(&output_path)?.write_all(&data)?;

                    if args.convert_to_png {
                        // Parse and convert to PNG using the shared export_anim helper
                        match HdAnimFile::parse(&data) {
                            Ok(anim) => {
                                let config = ExportConfig {
                                    convert_to_png: args.convert_to_png,
                                    team_color_mask: args.team_color_mask,
                                    save_dds: true,
                                };
                                match export_anim(&anim, &output_path.with_extension(""), &config) {
                                    Ok(result) => println!("  {} ({} frames, {:.1} MB) tc={}",
                                        output_name, result.frame_count, data.len() as f64/1_000_000.0, result.tc_mask_written),
                                    Err(e) => println!("  Export failed: {}", e),
                                }
                            }
                            Err(e) => println!("  {} - Parse error: {}", output_name, e),
                        }
                    } else {
                        println!("  {} ({:.1} MB)", output_name, data.len() as f64 / 1_000_000.0);
                    }
                }
                Err(_) => {
                    // Silently skip missing animations
                }
            }
        }
    }

    // Extract tilesets
    if args.tilesets || extract_all {
        println!("\nExtracting Tilesets...");
        let tilesets = vec!["badlands", "platform", "ashworld", "jungle", "desert", "ice", "twilight", "install"];

        for tileset in tilesets {
            let path = format!("{}tileset/{}.dds.vr4", prefix, tileset);
            match archive.extract_file(&path) {
                Ok(data) => {
                    let output_name = format!("{}.dds.vr4", tileset);
                    let output_path = args.output.join(&output_name);
                    File::create(&output_path)?.write_all(&data)?;
                    println!("  {} ({:.1} MB)", output_name, data.len() as f64 / 1_000_000.0);
                }
                Err(e) => println!("  {} - {}", tileset, e),
            }
        }
    }

    // Extract effects
    if args.effects || extract_all {
        println!("\nExtracting Effects...");
        let effects = vec!["water_normal_1.dds.grp", "water_normal_2.dds.grp"];

        for effect in effects {
            let path = format!("{}effect/{}", prefix, effect);
            match archive.extract_file(&path) {
                Ok(data) => {
                    let output_path = args.output.join(effect);
                    File::create(&output_path)?.write_all(&data)?;
                    println!("  {} ({:.1} MB)", effect, data.len() as f64 / 1_000_000.0);
                }
                Err(e) => println!("  {} - {}", effect, e),
            }
        }
    }

    println!("\nExtraction complete!");
    println!("Files saved to: {:?}", args.output);

    if args.convert_to_png {
        println!("\nTip: DDS files can be converted to PNG using ImageMagick:");
        println!("   convert file.dds file.png");
    }

    Ok(())
}
