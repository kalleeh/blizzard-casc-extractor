/// extract_sounds — extract StarCraft: Remastered unit sounds from CASC archive
///
/// Uses known BW/Remastered internal sound paths directly — no enumeration needed.
/// Run:  DYLD_FRAMEWORK_PATH=/Library/Frameworks cargo run --bin extract_sounds

use casc_extractor::casc::casclib_ffi::CascArchive;
use casc_extractor::casc::discovery::locate_starcraft;
use clap::Parser;
use std::fs;
use std::path::PathBuf;

const DEFAULT_OUTPUT_DIR: &str = "/Users/wallbomk/Projects.local/starcraft-bw/assets/sounds";

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
        "sound\\Terran\\marine\\tmardy00.wav",  // "ready" as fallback
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
        // Rifle shot weapon sounds
        "sound\\Weapons\\Terran\\tgun.wav",
        "sound\\Weapons\\terran\\tgun.wav",
        "sound\\weapons\\terran\\tgun.wav",
        "sound\\Terran\\Weapons\\tgun.wav",
        "sound\\Terran\\weapons\\tgun.wav",
        // Stim pack as "attack" proxy
        "sound\\Terran\\marine\\tmasti00.wav",
        "sound\\terran\\marine\\tmasti00.wav",
        // What sound as proxy
        "sound\\Terran\\marine\\tmawht00.wav",
        "sound\\terran\\marine\\tmawht00.wav",
    ]),
    ("zergling_attack.ogg", &[
        // Leading-backslash paths (archive native format)
        "\\zerg\\zergling\\zlatt00.wav",
        "\\zerg\\Zergling\\ZlAtt00.wav",
        "\\Zerg\\Zergling\\ZlAtt00.wav",
        "\\Zerg\\zergling\\zlatt00.wav",
        // With sound\ prefix
        "\\sound\\zerg\\zergling\\zlatt00.wav",
        "\\sound\\Zerg\\Zergling\\ZlAtt00.wav",
        // Standard paths without leading slash
        "sound\\Zerg\\Zergling\\ZlAtt00.wav",
        "sound/Zerg/Zergling/ZlAtt00.wav",
        "zerg\\zergling\\zlatt00.wav",
        "zerg/zergling/zlatt00.wav",
        // Pissed/what as proxy
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
        "sound\\Misc\\Klink.wav",   // classic BW select/click sound
        "sound\\misc\\klink.wav",
    ]),
    ("select.ogg", &[
        // Leading-backslash glue/misc UI sounds (found in archive listing)
        "\\glue\\mouseover.wav",
        "\\glue\\swishlock.wav",
        "\\misc\\button.wav",   // reuse button click as select proxy
        "\\misc\\perror.wav",
        // Standard paths
        "sound/Misc/select.wav",
        "sound/Glue/select.wav",
        "glue\\mouseover.wav",
        "glue/mouseover.wav",
        "misc\\button.wav",
        "misc/button.wav",
    ]),
];

#[derive(Parser)]
#[command(name = "extract-sounds")]
#[command(about = "Extract StarCraft: Remastered unit sounds", long_about = None)]
struct Args {
    /// Output directory for extracted sounds
    #[arg(short, long, default_value = DEFAULT_OUTPUT_DIR)]
    output: PathBuf,

    /// StarCraft installation directory (auto-detected if omitted)
    #[arg(long)]
    install_path: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    println!("  StarCraft Sound Extractor");
    println!("==================================================");

    let install_dir = locate_starcraft(args.install_path.as_deref())?;
    let install_str = install_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Install path is not valid UTF-8: {:?}", install_dir))?;
    let archive = CascArchive::open(install_str)
        .map_err(|e| anyhow::anyhow!("Cannot open StarCraft at {}: {}", install_str, e))?;
    println!("  Opened CASC archive");

    fs::create_dir_all(&args.output)?;

    let mut extracted = 0usize;

    for (out_name, candidates) in SOUND_TARGETS {
        let dest = args.output.join(out_name);
        if dest.exists() {
            println!("  {} already exists, skipping", out_name);
            extracted += 1;
            continue;
        }

        let mut found = false;
        for casc_path in *candidates {
            // Try as-is and with forward slashes
            let variants = [
                casc_path.to_string(),
                casc_path.replace("\\", "/"),
            ];
            for variant in &variants {
                match archive.extract_file(variant) {
                    Ok(data) if !data.is_empty() => {
                        fs::write(&dest, &data)?;
                        println!("  {:>7} bytes  {}  ->  {}", data.len(), variant, out_name);
                        found = true;
                        extracted += 1;
                        break;
                    }
                    _ => {}
                }
            }
            if found { break; }
        }

        if !found {
            println!("  {} -- none of {} candidates succeeded", out_name, candidates.len());
        }
    }

    println!("\n== Result ==================================================");
    println!("  {}/{} sounds extracted to {}", extracted, SOUND_TARGETS.len(), args.output.display());
    println!("\nIf any are missing, check the exact paths with:");
    println!("  DYLD_FRAMEWORK_PATH=/Library/Frameworks cargo run --bin list_files -- sound");

    Ok(())
}
