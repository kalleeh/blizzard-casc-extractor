//! Shared helpers used across the command handlers: path safety, overwrite
//! policy, config loading, and archive opening.

use anyhow::Result;
use casc_extractor::casc::casclib_ffi::CascArchive;
use casc_extractor::casc::discovery::locate_starcraft;
use casc_extractor::config::{ExtractionConfig, OverwriteBehavior};
use casc_extractor::CascStorage;
use regex::Regex;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub use casc_extractor::dds_converter::write_atomic;

/// Join a caller-supplied relative name onto `base`, guaranteeing the result
/// stays inside `base`.
///
/// Names reach us from user-controlled JSON (`--name-map`, `--targets`), so a
/// `..` component or an absolute path would otherwise write outside the chosen
/// output directory. Subdirectories are still allowed, since mappings legitimately
/// use them (`terran/marine`).
pub fn join_within(base: &Path, name: &str) -> Result<PathBuf> {
    use std::path::Component;

    let rel = PathBuf::from(name.replace('\\', "/"));
    let mut safe = PathBuf::new();
    for comp in rel.components() {
        match comp {
            Component::Normal(part) => safe.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                anyhow::bail!(
                    "refusing unsafe output name {:?}: must be a relative path without `..`",
                    name
                );
            }
        }
    }
    if safe.as_os_str().is_empty() {
        anyhow::bail!("refusing empty output name");
    }
    Ok(base.join(safe))
}

/// Returns `true` when the file should be skipped based on overwrite policy.
///
/// `session_start` is used only by `IfNewer`: skip the file if it already
/// exists and its mtime is at or after the session start time (i.e., it was
/// written during this extraction session and need not be re-extracted).
pub fn should_skip(
    path: &Path,
    behavior: OverwriteBehavior,
    session_start: std::time::SystemTime,
) -> bool {
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
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                // Pick the first unused suffix so repeated runs keep every
                // generation; a fixed `.bak` silently destroyed the previous
                // backup on each run, leaving only one.
                let mut bak = path.with_extension(format!("{}.bak", ext));
                let mut generation = 1u32;
                while bak.exists() && generation <= 1000 {
                    bak = path.with_extension(format!("{}.bak.{}", ext, generation));
                    generation += 1;
                }
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
pub fn png_compression(level: u32) -> png::Compression {
    match level {
        0..=2 => png::Compression::Fast,
        7..=9 => png::Compression::High,
        _ => png::Compression::Balanced,
    }
}

/// Compile a list of regex pattern strings, silently skipping invalid ones.
pub fn compile_patterns(patterns: &Option<Vec<String>>) -> Vec<Regex> {
    patterns
        .as_ref()
        .map(|pats| {
            pats.iter()
                .filter_map(|p| {
                    Regex::new(p)
                        .map_err(|e| {
                            eprintln!("Warning: invalid filter pattern {:?}: {}", p, e);
                        })
                        .ok()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Returns `true` if the path passes the include/exclude filter.
/// An empty include list means "allow everything".
pub fn passes_filter(path: &str, include: &[Regex], exclude: &[Regex]) -> bool {
    if !include.is_empty() && !include.iter().any(|r| r.is_match(path)) {
        return false;
    }
    !exclude.iter().any(|r| r.is_match(path))
}

/// Load ExtractionConfig from a JSON file, falling back to defaults on any error.
pub fn load_config(path: Option<&Path>) -> ExtractionConfig {
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

/// CascLib product code for StarCraft: Remastered (its CDN path is `tpr/sc1live`).
pub const ONLINE_PRODUCT: &str = "s1";

/// Region used when the local install does not tell us one.
pub const ONLINE_REGION_FALLBACK: &str = "us";

/// Read the active CDN region (`Branch`) from a local `.build.info`.
///
/// The region is part of CascLib's online params string, and picking the wrong
/// one points at a CDN that may not carry this build. `.build.info` is a
/// pipe-delimited table whose header names the columns, so find `Branch` by name
/// rather than by position, and take the row whose `Active` column is 1.
pub fn detect_cdn_region(install_path: Option<&Path>) -> Option<String> {
    let dir = locate_starcraft(install_path).ok()?;
    let text = fs::read_to_string(dir.join(".build.info")).ok()?;
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next()?.split('|').collect();
    // Column names carry a `!TYPE:SIZE` suffix, e.g. `Branch!STRING:0`.
    let col = |name: &str| {
        header
            .iter()
            .position(|h| h.split('!').next().map(str::trim) == Some(name))
    };
    let branch_idx = col("Branch")?;
    let active_idx = col("Active");

    for line in lines {
        let fields: Vec<&str> = line.split('|').collect();
        let is_active = active_idx
            .and_then(|i| fields.get(i))
            .map(|v| v.trim() == "1")
            .unwrap_or(true);
        if !is_active {
            continue;
        }
        if let Some(branch) = fields.get(branch_idx) {
            let branch = branch.trim();
            if !branch.is_empty() {
                return Some(branch.to_string());
            }
        }
    }
    None
}

pub const INSTALL_PATH_HINT: &str =
    "Hint: Use --install-path to specify the StarCraft directory.\n\
     Common paths: /Applications/StarCraft (macOS), ~/.local/share/StarCraft or ~/.wine/drive_c/Program Files/StarCraft (Linux)";

/// Resolve the StarCraft install directory to a UTF-8 string.
pub fn resolve_install_str(install_path: Option<&Path>) -> Result<String> {
    let install_dir = locate_starcraft(install_path)
        .map_err(|e| anyhow::anyhow!("{}\n{}", e, INSTALL_PATH_HINT))?;
    install_dir
        .into_os_string()
        .into_string()
        .map_err(|p| anyhow::anyhow!("Install path is not valid UTF-8: {:?}", p))
}

/// Open the CascLib-backed archive, resolving the install path.
pub fn open_casc_archive(install_path: Option<&Path>) -> Result<CascArchive> {
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
pub fn open_casc_storage(install_path: Option<&Path>) -> Result<CascStorage> {
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
