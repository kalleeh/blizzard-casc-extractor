/// Quick probe to find working Zerg sound paths
use casc_extractor::casc::casclib_ffi::CascArchive;
use casc_extractor::casc::discovery::locate_starcraft;
use casc_extractor::CascStorage;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "probe-sounds")]
#[command(about = "Probe CASC archive to find working Zerg/UI sound paths", long_about = None)]
struct Args {
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
    println!("  Opened archive\n");

    // Probe known-working pattern variants for Zerg
    let probes = vec![
        // Locale-prefixed Zerg sounds (from listing output)
        "enUS\\Assets\\sound\\Zerg\\Zergling\\ZlDth00.wav",
        "enUS\\Assets\\sound\\zerg\\zergling\\zldth00.wav",
        "enUS\\Assets\\sound\\Zerg\\Zergling\\ZlAtt00.wav",
        "NOLA\\Assets\\sound\\Zerg\\Zergling\\ZlDth00.wav",
        "NOLA\\Assets\\sound\\Zerg\\Zergling\\ZlAtt00.wav",
        // Try the full CASC format
        "Assets\\sound\\Zerg\\Zergling\\ZlDth00.wav",
        "Assets\\sound\\Zerg\\Zergling\\ZlAtt00.wav",
        // Zerg variations
        "sound\\Zerg\\Zergling\\ZlDth00.wav",
        "sound\\zerg\\zergling\\zldth00.wav",
        "sound/Zerg/Zergling/ZlDth00.wav",
        "sound/zerg/zergling/zldth00.wav",
        "SD\\sound\\Zerg\\Zergling\\ZlDth00.wav",
        "SD\\sound\\zerg\\zergling\\zldth00.wav",
        "Assets\\sound\\Zerg\\Zergling\\ZlDth00.wav",
        "NOLA\\sound\\Zerg\\Zergling\\ZlDth00.wav",
        // Select/UI
        "sound\\Misc\\select.wav",
        "sound/Misc/select.wav",
        "SD\\sound\\Misc\\select.wav",
        "sound\\glue\\select.wav",
        "sound\\Glue\\select.wav",
        "sound/Glue/select.wav",
        "sound\\ui\\select.wav",
        "sound\\Misc\\mousedown.wav",
        "sound/misc/mousedown.wav",
        "sound\\Misc\\klink.wav",
        "sound/misc/klink.wav",
    ];

    for p in probes {
        match archive.extract_file(p) {
            Ok(data) if !data.is_empty() => println!("  {} ({} bytes)", p, data.len()),
            _ => println!("  {}", p),
        }
    }

    // Use CascStorage to list Zerg sounds quickly
    println!("\nListing Zerg audio from archive...");
    let storage = CascStorage::open(install_str)
        .map_err(|e| anyhow::anyhow!("CascStorage::open failed: {}", e))?;
    let files = storage.list_files()
        .map_err(|e| anyhow::anyhow!("list_files failed: {}", e))?;
    let zerg_audio: Vec<_> = files.iter().filter(|f| {
        let lower = f.to_lowercase();
        (lower.contains("zerg") || lower.contains("\\zl") || lower.contains("/zl"))
            && (lower.ends_with(".wav") || lower.ends_with(".ogg"))
    }).collect();

    println!("Found {} Zerg audio paths:", zerg_audio.len());
    for f in zerg_audio.iter().take(30) {
        println!("  {}", f);
    }

    println!("\nListing Misc/Glue UI sounds...");
    let ui_audio: Vec<_> = files.iter().filter(|f| {
        let lower = f.to_lowercase();
        (lower.contains("misc") || lower.contains("glue") || lower.contains("\\ui"))
            && (lower.ends_with(".wav") || lower.ends_with(".ogg"))
    }).collect();
    println!("Found {} UI audio paths:", ui_audio.len());
    for f in ui_audio.iter().take(30) {
        println!("  {}", f);
    }

    Ok(())
}
