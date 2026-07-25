//! `config` and `validate` subcommands.

use super::support::write_atomic;
use anyhow::Result;
use casc_extractor::config::ExtractionConfig;
use casc_extractor::validation::regression_suite::KnownGoodExtraction;
use casc_extractor::validation::regression_suite::SpriteMetadata as RegressionSpriteMetadata;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn cmd_config_init(output: &Path) -> Result<()> {
    let cfg = ExtractionConfig::default();
    let json = serde_json::to_string_pretty(&cfg)
        .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;
    write_atomic(output, json.as_bytes())
        .map_err(|e| anyhow::anyhow!("Failed to write config to {:?}: {}", output, e))?;
    println!("Config written to {:?}", output);
    Ok(())
}

pub fn cmd_validate_register(file: &Path, suite_path: &Path) -> Result<()> {
    // Compute SHA256 of the file
    let mut f =
        File::open(file).map_err(|e| anyhow::anyhow!("Cannot open file {:?}: {}", file, e))?;
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

    // Record real dimensions where the file is a readable image, so a committed
    // suite documents what it is guarding rather than a row of zeroes. `validate
    // run` matches on the file name alone, so store that instead of the absolute
    // path the baseline happened to be created from — otherwise a suite is only
    // meaningful on the machine that produced it.
    let (width, height, format) = match image::open(file) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            (
                rgba.width(),
                rgba.height(),
                file.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_uppercase(),
            )
        }
        Err(_) => (0, 0, String::new()),
    };

    let entry = KnownGoodExtraction {
        sprite_name: sprite_name.clone(),
        source_file: PathBuf::from(&sprite_name),
        expected_output: PathBuf::from(&sprite_name),
        expected_metadata: RegressionSpriteMetadata {
            width,
            height,
            frame_count: 0,
            format,
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
    // Atomic in particular here: a half-written suite would corrupt the very
    // baselines that guard pixel output, and `validate run` would then fail to parse.
    write_atomic(suite_path, json.as_bytes())
        .map_err(|e| anyhow::anyhow!("Cannot write suite {:?}: {}", suite_path, e))?;

    println!(
        "Registered {} (SHA256: {}) in {}",
        sprite_name,
        hash,
        suite_path.display()
    );
    Ok(())
}

pub fn cmd_validate_run(dir: &Path, suite_path: &Path, json_output: bool) -> Result<()> {
    if !suite_path.exists() {
        if json_output {
            println!(
                "{}",
                serde_json::json!({"total": 0, "passed": 0, "results": []})
            );
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
        let filename = entry
            .expected_output
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

    // A regression check that reports failures but exits 0 cannot gate CI.
    if passed < total {
        anyhow::bail!("{} of {} validation check(s) failed", total - passed, total);
    }
    Ok(())
}
