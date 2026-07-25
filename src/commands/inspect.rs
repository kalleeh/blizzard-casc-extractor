//! `inspect` subcommands.

use super::support::*;
use crate::QualityLevel;
use anyhow::Result;
use casc_extractor::casc::casclib_ffi::CascArchive;
use casc_extractor::casc::discovery::locate_starcraft;
use casc_extractor::CascStorage;
use std::path::Path;

pub fn cmd_inspect_sprites(
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
            Ok(data) => println!(
                "  found  mainSD.anim  ({:.1} MB)",
                data.len() as f64 / 1_000_000.0
            ),
            Err(_) => println!("  not found: {}", path),
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

    println!("\nFound {} anim files out of {} IDs probed.", found, max_id);
    Ok(())
}

pub fn cmd_inspect_archive(install_path: Option<&Path>) -> Result<()> {
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

/// List or search all files in the CASC archive.
pub fn cmd_inspect_files(
    install_path: Option<&std::path::Path>,
    search: Option<&str>,
    sizes: bool,
) -> anyhow::Result<()> {
    let storage = open_casc_storage(install_path)?;
    let files = storage
        .list_files_with_sizes()
        .map_err(|e| anyhow::anyhow!("list_files failed: {}", e))?;

    let pattern = search.map(|s| s.to_lowercase());
    let mut count = 0usize;
    for (f, size) in &files {
        let fl = f.to_lowercase();
        if pattern.as_deref().is_none_or(|p| fl.contains(p)) {
            if sizes {
                println!("{:>12}  {}", size, f);
            } else {
                println!("{}", f);
            }
            count += 1;
        }
    }
    eprintln!("({} matches / {} total files)", count, files.len());
    Ok(())
}
