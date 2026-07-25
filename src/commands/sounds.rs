//! `sounds` subcommands: the built-in target table, extraction, listing and probing.

use super::support::*;
use anyhow::Result;
use casc_extractor::casc::casclib_ffi::CascArchive;
use casc_extractor::CascStorage;
use std::fs;
use std::path::Path;

/// JSON-serialisable representation of a single sound target entry.
/// Used by `sounds export-targets` (write) and `sounds extract --targets` (read).
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SoundTargetEntry {
    output: String,
    candidates: Vec<String>,
}

/// (output_filename, list of candidate CASC paths to try in order)
pub const SOUND_TARGETS: &[(&str, &[&str])] = &[
    (
        "marine_yes1.ogg",
        &[
            "sound\\Terran\\marine\\tmayes00.wav",
            "sound\\terran\\marine\\tmayes00.wav",
            "sound\\Terran\\Marine\\tmayes00.wav",
        ],
    ),
    (
        "marine_move1.ogg",
        &[
            "sound\\Terran\\marine\\tmamov00.wav",
            "sound\\terran\\marine\\tmamov00.wav",
            "sound\\Terran\\marine\\tmardy00.wav",
            "sound\\terran\\marine\\tmardy00.wav",
        ],
    ),
    (
        "marine_die.ogg",
        &[
            "sound\\Terran\\marine\\tmadth00.wav",
            "sound\\terran\\marine\\tmadth00.wav",
            "sound\\Terran\\Marine\\tmadth00.wav",
        ],
    ),
    (
        "marine_attack.ogg",
        &[
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
        ],
    ),
    (
        "zergling_attack.ogg",
        &[
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
        ],
    ),
    (
        "zergling_die.ogg",
        &[
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
        ],
    ),
    (
        "button.ogg",
        &[
            "sound\\Misc\\button.wav",
            "sound\\misc\\button.wav",
            "sound\\Glue\\button.wav",
            "sound\\UI\\button.wav",
            "sound\\misc\\buttonclk.wav",
            "sound\\Misc\\Klink.wav",
            "sound\\misc\\klink.wav",
        ],
    ),
    (
        "mouseover.ogg",
        &[
            "sound\\Glue\\mouseover.wav",
            "sound\\Glue\\swishlock.wav",
            "sound\\glue\\mouseover.wav",
            "sound\\glue\\swishlock.wav",
            "\\glue\\mouseover.wav",
            "glue\\mouseover.wav",
        ],
    ),
    (
        "hydralisk_yes1.ogg",
        &[
            "sound\\Zerg\\hydra\\zhyyes00.wav",
            "sound\\zerg\\hydra\\zhyyes00.wav",
            "sound\\Zerg\\hydra\\zhyyes01.wav",
        ],
    ),
    (
        "hydralisk_ready.ogg",
        &[
            "sound\\Zerg\\hydra\\zhyrdy00.wav",
            "sound\\zerg\\hydra\\zhyrdy00.wav",
        ],
    ),
    (
        "hydralisk_attack.ogg",
        &[
            "sound\\Zerg\\hydra\\spifir00.wav",
            "sound\\zerg\\hydra\\spifir00.wav",
        ],
    ),
    (
        "ghost_yes1.ogg",
        &[
            "sound\\Terran\\ghost\\tghyes01.wav",
            "sound\\terran\\ghost\\tghyes01.wav",
            "sound\\Terran\\ghost\\tghyes00.wav",
        ],
    ),
    (
        "ghost_ready.ogg",
        &[
            "sound\\Terran\\ghost\\tghrdy00.wav",
            "sound\\terran\\ghost\\tghrdy00.wav",
        ],
    ),
    (
        "scv_yes1.ogg",
        &[
            "sound\\Terran\\scv\\tscpss00.wav",
            "sound\\terran\\scv\\tscpss00.wav",
            "sound\\Terran\\scv\\tscyes00.wav",
        ],
    ),
    (
        "scv_die.ogg",
        &[
            "sound\\Terran\\scv\\tscdth00.wav",
            "sound\\terran\\scv\\tscdth00.wav",
        ],
    ),
];

/// Scan `files` for an audio entry whose name contains all words in `stem_hint`
/// (case-insensitive), and try to extract the first match.
///
/// Takes the already-enumerated listing rather than a storage handle: this is
/// called once per unresolved target, and re-walking all ~21k entries each time
/// made a targets file with many misses O(targets x archive).
pub fn discover_sound(archive: &CascArchive, files: &[String], stem_hint: &str) -> Option<Vec<u8>> {
    let hint_lower = stem_hint.to_lowercase();
    // Split hint on '_' to get keywords (e.g. "zergling_attack" -> ["zergling", "att"])
    let keywords: Vec<String> = hint_lower
        .split('_')
        .map(|w| w.chars().take(4).collect::<String>())
        .collect();

    let candidate = files.iter().find(|f| {
        let fl = f.to_lowercase();
        (fl.ends_with(".wav") || fl.ends_with(".ogg"))
            && keywords.iter().all(|kw| fl.contains(kw.as_str()))
    })?;

    archive
        .extract_file(candidate)
        .ok()
        .filter(|d| !d.is_empty())
}

pub fn cmd_sounds_extract(
    archive: &CascArchive,
    storage: &CascStorage,
    output: &Path,
    targets_path: Option<&Path>,
    fail_on_incomplete: bool,
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
    let mut pre_existing = 0usize;
    // Filled in on first use by the discovery fallback below.
    let mut listing: Option<Vec<String>> = None;

    for (out_name, candidates) in &targets {
        let dest = join_within(output, out_name)?;
        if dest.exists() {
            println!("  {} already exists, skipping", out_name);
            pre_existing += 1;
            continue;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut found = false;
        for casc_path in candidates {
            let variants = [casc_path.to_string(), casc_path.replace('\\', "/")];
            for variant in &variants {
                match archive.extract_file(variant) {
                    Ok(data) if !data.is_empty() => {
                        write_atomic(&dest, &data)?;
                        println!("  {:>7} bytes  {}  ->  {}", data.len(), variant, out_name);
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
            // Fall back to dynamic discovery via the file listing, enumerating the
            // archive at most once for the whole run and only if something missed.
            if listing.is_none() {
                listing = Some(
                    storage
                        .list_files()
                        .map_err(|e| anyhow::anyhow!("list_files failed: {}", e))?,
                );
            }
            let files = listing.as_deref().unwrap_or(&[]);
            let stem = out_name.trim_end_matches(".ogg").trim_end_matches(".wav");
            if let Some(data) = discover_sound(archive, files, stem) {
                write_atomic(&dest, &data)?;
                println!("  {:>7} bytes  (discovered)  ->  {}", data.len(), out_name);
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

    // `resolved` counts targets that are present on disk; `extracted` counts only
    // what this run actually pulled from the archive, so a re-run cannot report
    // progress it did not make.
    let resolved = extracted + pre_existing;
    println!(
        "\n== Result ==================================================\n  \
         {}/{} sounds available in {} ({} newly extracted, {} already present)",
        resolved,
        total,
        output.display(),
        extracted,
        pre_existing
    );
    println!(
        "\nIf any are missing, check exact paths with:\n  \
         casc-extractor sounds list"
    );

    if fail_on_incomplete && resolved < total {
        return Err(anyhow::anyhow!(
            "sound extraction incomplete: {}/{} targets resolved",
            resolved,
            total
        ));
    }
    Ok(())
}

pub fn cmd_sounds_export_targets(output: &Path) -> Result<()> {
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
    println!(
        "Edit the file then use:  casc-extractor sounds extract --targets {:?}",
        output
    );
    Ok(())
}

pub fn cmd_sounds_list(install_path: Option<&Path>, search: Option<String>) -> Result<()> {
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
            (lower.contains("zerg") || lower.contains("\\zl") || lower.contains("/zl"))
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
            (lower.contains("misc") || lower.contains("glue") || lower.contains("\\ui"))
                && (lower.ends_with(".wav") || lower.ends_with(".ogg"))
        })
        .collect();
    println!("Found {} UI audio paths:", ui_audio.len());
    for f in ui_audio.iter().take(30) {
        println!("  {}", f);
    }

    Ok(())
}

/// Probe known unit sound paths to verify which ones resolve in the archive.
pub fn cmd_sounds_probe(
    install_path: Option<&std::path::Path>,
    filter: Option<&str>,
    fail_on_incomplete: bool,
) -> anyhow::Result<()> {
    let archive = open_casc_archive(install_path)?;

    let candidates: &[(&str, &str)] = &[
        ("hydralisk_attack", "sound\\Zerg\\Hydralisk\\HydAtt00.wav"),
        ("hydralisk_die", "sound\\Zerg\\Hydralisk\\HydDth00.wav"),
        ("hydralisk_yes", "sound\\Zerg\\Hydralisk\\HydYes00.wav"),
        ("hydralisk_what", "sound\\Zerg\\Hydralisk\\HydWht00.wav"),
        ("zealot_attack", "sound\\Protoss\\Zealot\\ZeaAtt00.wav"),
        ("zealot_die", "sound\\Protoss\\Zealot\\ZeaDth00.wav"),
        ("zealot_yes", "sound\\Protoss\\Zealot\\ZeaYes00.wav"),
        ("zealot_what", "sound\\Protoss\\Zealot\\ZeaWht00.wav"),
        ("ghost_attack", "sound\\Terran\\Ghost\\TghAtt00.wav"),
        ("ghost_die", "sound\\Terran\\Ghost\\TghDth00.wav"),
        ("ghost_yes", "sound\\Terran\\Ghost\\TghYes00.wav"),
        ("ghost_what", "sound\\Terran\\Ghost\\TghWht00.wav"),
        ("siege_attack", "sound\\Terran\\Vehicle\\TvhAtt00.wav"),
        ("siege_die", "sound\\Terran\\Vehicle\\TvhDth00.wav"),
        ("vulture_attack", "sound\\Terran\\Vulture\\TVuAtt00.wav"),
        ("vulture_yes", "sound\\Terran\\Vulture\\TVuYes00.wav"),
        ("dragoon_attack", "sound\\Protoss\\Dragoon\\PDrAtt00.wav"),
        ("dragoon_die", "sound\\Protoss\\Dragoon\\PDrDth00.wav"),
        ("dragoon_yes", "sound\\Protoss\\Dragoon\\PDrYes00.wav"),
        ("scv_yes", "sound\\Terran\\SCV\\TSCYes00.wav"),
        ("scv_attack", "sound\\Terran\\SCV\\TSCAtt00.wav"),
        ("medic_yes", "sound\\Terran\\Medic\\TMEYes00.wav"),
        ("medic_what", "sound\\Terran\\Medic\\TMEWht00.wav"),
        ("mutalisk_attack", "sound\\Zerg\\Mutalisk\\MutAtt00.wav"),
        ("mutalisk_die", "sound\\Zerg\\Mutalisk\\MutDth00.wav"),
        ("ultralisk_attack", "sound\\Zerg\\Ultralisk\\UltAtt00.wav"),
        ("ultralisk_die", "sound\\Zerg\\Ultralisk\\UltDth00.wav"),
        ("ultralisk_yes", "sound\\Zerg\\Ultralisk\\UltYes00.wav"),
        ("probe_yes", "sound\\Protoss\\Probe\\PrbYes00.wav"),
        ("probe_attack", "sound\\Protoss\\Probe\\PrbAtt00.wav"),
        ("weapon_gauss", "sound\\Weapons\\Terran\\tgun.wav"),
        ("weapon_tank", "sound\\Weapons\\Terran\\tTankVulcan.wav"),
        ("weapon_tank2", "sound\\Weapons\\Terran\\tTankShot.wav"),
        ("weapon_needle", "sound\\Weapons\\Zerg\\zNeedleSpine.wav"),
        ("weapon_zealot", "sound\\Weapons\\Protoss\\pZealotHit.wav"),
        (
            "weapon_phaser",
            "sound\\Weapons\\Protoss\\pPhaserCannon.wav",
        ),
    ];

    let filter_lc = filter.map(|s| s.to_lowercase());
    let (mut found, mut checked) = (0usize, 0usize);
    for (name, path) in candidates {
        if let Some(ref f) = filter_lc {
            if !name.contains(f.as_str()) && !path.to_lowercase().contains(f.as_str()) {
                continue;
            }
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
            if archive
                .extract_file(v)
                .map(|d| !d.is_empty())
                .unwrap_or(false)
            {
                println!("  ✓ {:<25} {}", name, v);
                found += 1;
                hit = true;
                break;
            }
        }
        if !hit {
            println!("  ✗ {:<25} {}", name, path);
        }
    }
    println!("  {}/{} found", found, checked);

    if fail_on_incomplete && found < checked {
        return Err(anyhow::anyhow!(
            "sound probe incomplete: {}/{} targets resolved",
            found,
            checked
        ));
    }
    Ok(())
}
