/// Cross-platform StarCraft: Remastered installation discovery.
///
/// Provides helpers to locate the game directory and open the CASC archive
/// without callers needing to hard-code platform-specific paths.

use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};
use crate::casc::CascArchive;

/// Locate the StarCraft: Remastered installation directory.
///
/// Checks `override_path` first, then platform-specific default locations.
/// Returns an error with helpful install instructions if not found.
pub fn locate_starcraft(override_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = override_path {
        return Ok(p.to_path_buf());
    }

    let candidates: Vec<PathBuf> = platform_candidates();

    for path in &candidates {
        if path.exists() && path.join("Data").exists() {
            return Ok(path.clone());
        }
    }

    Err(anyhow!(
        "StarCraft: Remastered installation not found.\n\
         Checked: {}\n\
         Use --install-path <PATH> to specify the location.",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

#[cfg(target_os = "macos")]
fn platform_candidates() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/Applications/StarCraft"),
        PathBuf::from("/Applications/StarCraft Remastered"),
    ]
}

#[cfg(target_os = "windows")]
fn platform_candidates() -> Vec<PathBuf> {
    let mut paths = vec![
        PathBuf::from("C:\\Program Files\\StarCraft"),
        PathBuf::from("C:\\Program Files (x86)\\StarCraft"),
        PathBuf::from("C:\\Program Files\\Battle.net\\StarCraft"),
    ];
    // Also check PROGRAMFILES env var
    if let Ok(pf) = std::env::var("PROGRAMFILES") {
        paths.push(PathBuf::from(pf).join("StarCraft"));
    }
    if let Ok(pf) = std::env::var("PROGRAMFILES(X86)") {
        paths.push(PathBuf::from(pf).join("StarCraft"));
    }
    paths
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_candidates() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/opt/starcraft"),
        PathBuf::from("/usr/local/games/starcraft"),
        dirs::home_dir()
            .unwrap_or_default()
            .join(".local/share/starcraft"),
    ]
}

/// Convenience: open the CascArchive, auto-detecting or using override_path.
pub fn open_archive(override_path: Option<&Path>) -> Result<CascArchive> {
    let install_dir = locate_starcraft(override_path)?;
    CascArchive::open(&install_dir)
        .map_err(|e| anyhow!("Failed to open CASC archive at {}: {}", install_dir.display(), e))
}
