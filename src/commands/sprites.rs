//! `extract anim`, `extract tileset` and `extract effect`.

use super::support::*;
use crate::QualityLevel;
use anyhow::{Context as _, Result};
use casc_extractor::anim::HdAnimFile;
use casc_extractor::casc::casclib_ffi::CascArchive;
use casc_extractor::config::ExtractionConfig;
use casc_extractor::grp::GrpFile;
use casc_extractor::mapping::SpriteMapping;
use casc_extractor::{export_anim, ExportConfig};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

#[allow(clippy::too_many_arguments)]
pub fn cmd_extract_anim(
    archive: &CascArchive,
    output: &Path,
    quality: QualityLevel,
    ids: Option<Vec<u16>>,
    raw: bool,
    name_map: Option<PathBuf>,
    convert_to_png: bool,
    team_color_mask: bool,
    config: &ExtractionConfig,
    layers: Vec<String>,
    save_dds: bool,
) -> Result<()> {
    let session_start = std::time::SystemTime::now();

    fs::create_dir_all(output)?;
    println!("Extracting Animations...");
    println!("Quality:  {}", quality.description());
    println!("Output:   {:?}\n", output);

    let overwrite = config.output_settings.overwrite_behavior;
    // quality_settings.png_compression_level previously only reached the
    // spritesheet writer; every per-sprite PNG used the image crate default.
    let sprite_png_compression = png_compression(config.quality_settings.png_compression_level);
    let gen_meta = config.output_settings.metadata_options.generate_json;
    let pixels_per_unit = config.output_settings.unity_settings.pixels_per_unit;

    // SD quality uses a single file rather than the per-ID loop.
    if matches!(quality, QualityLevel::Sd) {
        let casc_path = "SD/mainSD.anim";
        let output_path = output.join("mainSD.anim");
        if !should_skip(&output_path, overwrite, session_start) {
            match archive.extract_file(casc_path) {
                Ok(data) => {
                    write_atomic(&output_path, &data)?;
                    println!("  mainSD.anim ({:.1} MB)", data.len() as f64 / 1_000_000.0);

                    if convert_to_png {
                        let has_anim_magic = data.starts_with(b"ANIM");
                        if has_anim_magic {
                            // mainSD.anim uses the SD ANIM variant (version 0x0101).
                            // The HD parser (HdAnimFile) expects version 0x0202 or 0x0204 and
                            // explicitly rejects 0x0101 — the SD format uses a different
                            // per-sprite structure with palettised pixel data rather than
                            // DDS texture slabs, and is not yet implemented.
                            match HdAnimFile::parse(&data) {
                                Ok(anim) => {
                                    let export_cfg = ExportConfig {
                                        convert_to_png,
                                        team_color_mask,
                                        save_dds,
                                        generate_metadata: gen_meta,
                                        pixels_per_unit,
                                        layers: layers.clone(),
                                        png_compression: sprite_png_compression,
                                    };
                                    match export_anim(
                                        &anim,
                                        &output_path.with_extension(""),
                                        &export_cfg,
                                    ) {
                                        Ok(result) => println!(
                                            "  mainSD ({} frames) exported as PNG",
                                            result.frame_count
                                        ),
                                        Err(e) => println!("  mainSD ANIM export failed: {}", e),
                                    }
                                }
                                Err(_) => {
                                    // HdAnimFile failed — try the SD ANIM v0x0101 parser.
                                    match casc_extractor::anim::SdAnimFile::parse(&data) {
                                        Ok(sd) => {
                                            println!(
                                                "  mainSD parsed: {} sprites (SD ANIM v0x0101)",
                                                sd.sprite_count
                                            );
                                            // Build a name map from the sprite mapping if available:
                                            // "terran/marine" -> "anim/main_000.anim" => id 0 -> "terran/marine"
                                            let mapping_path =
                                                PathBuf::from("mappings/starcraft-remastered.yaml");
                                            let id_to_name: std::collections::HashMap<
                                                usize,
                                                String,
                                            > = if mapping_path.exists() {
                                                SpriteMapping::load(&mapping_path)
                                                    .map(|m| {
                                                        m.entries
                                                            .into_iter()
                                                            .filter_map(|(name, casc_path)| {
                                                                // Extract numeric ID from "anim/main_NNN.anim"
                                                                casc_path
                                                                    .trim_end_matches(".anim")
                                                                    .rsplit('_')
                                                                    .next()
                                                                    .and_then(|s| {
                                                                        s.parse::<usize>().ok()
                                                                    })
                                                                    .map(|id| {
                                                                        (id, name.replace('/', "_"))
                                                                    })
                                                            })
                                                            .collect()
                                                    })
                                                    .unwrap_or_default()
                                            } else {
                                                std::collections::HashMap::new()
                                            };

                                            let export_teamcolor = team_color_mask;
                                            let mut exported = 0usize;
                                            for (idx, sprite) in sd.sprites.iter().enumerate() {
                                                if sprite.dds1_data.is_empty() {
                                                    continue;
                                                }
                                                let stem =
                                                    id_to_name.get(&idx).cloned().unwrap_or_else(
                                                        || format!("mainSD_sprite_{:03}", idx),
                                                    );
                                                let sprite_path =
                                                    output.join(format!("{}.png", stem));
                                                if let Err(e) =
                                                    casc_extractor::dds_converter::save_dds_as_png(
                                                        &sprite.dds1_data,
                                                        &sprite_path,
                                                    )
                                                {
                                                    log::debug!("SD sprite export failed: {}", e);
                                                } else {
                                                    exported += 1;
                                                    // Export team-color layer if requested and available
                                                    if export_teamcolor
                                                        && !sprite.layer2_data.is_empty()
                                                    {
                                                        let tc_path =
                                                            output.join(format!("{}_tc.png", stem));
                                                        if let Err(e) = casc_extractor::dds_converter::save_dds_as_png(
                                                            &sprite.layer2_data, &tc_path
                                                        ) {
                                                            log::debug!("SD TC export failed: {}", e);
                                                        }
                                                    }
                                                }
                                            }
                                            println!("  Exported {} SD sprite PNGs", exported);
                                        }
                                        Err(e) => {
                                            let version = if data.len() >= 6 {
                                                u16::from_le_bytes([data[4], data[5]])
                                            } else {
                                                0
                                            };
                                            println!("  SD ANIM parse error: {}", e);
                                            println!("  SD ANIM version: 0x{:04X}", version);
                                        }
                                    }
                                }
                            }
                        } else if let Ok(grp) = GrpFile::parse(&data) {
                            let png_path = output.join("mainSD.png");
                            let compression =
                                png_compression(config.quality_settings.png_compression_level);
                            match build_spritesheet(&grp, &png_path, compression) {
                                Ok(()) => println!(
                                    "  mainSD spritesheet PNG ({} frames)",
                                    grp.frame_count
                                ),
                                Err(e) => println!("  mainSD spritesheet failed: {}", e),
                            }
                        } else {
                            let hex: Vec<String> =
                                data.iter().take(16).map(|b| format!("{:02X}", b)).collect();
                            println!(
                                "  SD PNG conversion: unknown format (first 16 bytes: {})",
                                hex.join(" ")
                            );
                        }
                    }
                }
                Err(e) => println!("  mainSD.anim - {}", e),
            }
        }
        println!("\nExtraction complete!");
        println!("Files saved to: {:?}", output);
        return Ok(());
    }

    let prefix = quality.path_prefix();

    // Build ID → output name map.
    // Priority: explicit --name-map JSON > built-in YAML mapping > raw numbered names.
    let id_to_name: HashMap<String, String> = if raw {
        HashMap::new()
    } else if let Some(ref p) = name_map {
        let json = fs::read_to_string(p)
            .map_err(|e| anyhow::anyhow!("Failed to read name map {:?}: {}", p, e))?;
        serde_json::from_str::<HashMap<String, String>>(&json)
            .map_err(|e| anyhow::anyhow!("Failed to parse name map {:?}: {}", p, e))?
    } else {
        // Default: parse built-in YAML mapping (animations/category/name: anim/main_NNN.anim)
        // and invert it to produce NNN → category/name.
        let yaml = include_str!("../../mappings/starcraft-remastered.yaml");
        let mut map = HashMap::new();
        for line in yaml.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.split('#').next().unwrap_or("").trim();
                // Only animation entries: key = "animations/category/name"
                // value = "anim/main_NNN.anim" (possibly with HD2/ prefix)
                if key.starts_with("animations/") {
                    let name = key.trim_start_matches("animations/");
                    // Extract the numeric ID from the CASC path
                    let casc = value.trim_start_matches("HD2/").trim_start_matches("SD/");
                    if let Some(stem) = casc
                        .strip_prefix("anim/main_")
                        .and_then(|s| s.strip_suffix(".anim"))
                    {
                        if let Ok(id) = stem
                            .trim_start_matches('0')
                            .parse::<u16>()
                            .or_else(|_| stem.parse::<u16>())
                        {
                            map.insert(id.to_string(), name.to_string());
                        }
                    }
                }
            }
        }
        println!(
            "  Using built-in mapping ({} named entries). Pass --raw for numbered output.",
            map.len()
        );
        map
    };

    // Build id list, then cap to max_files if set.
    let mut id_list: Vec<u16> = ids.unwrap_or_else(|| (0..1000).collect());
    if let Some(max) = config.filter_settings.max_files {
        id_list.truncate(max as usize);
    }

    // Compile include/exclude regex patterns once upfront.
    let include = compile_patterns(&config.filter_settings.include_patterns);
    let exclude = compile_patterns(&config.filter_settings.exclude_patterns);

    if convert_to_png {
        // -----------------------------------------------------------------------
        // Phase 1 — Sequential: extract from the CASC archive (FFI, non-Send).
        // -----------------------------------------------------------------------
        let mut progress = casc_extractor::ProgressReporter::new(
            id_list.len() as u64,
            config.feedback_settings.verbose_logging,
        );

        // Only (id, path, size) is carried into phase 2 — the bytes are already on
        // disk and get re-read per item there. Holding every payload here peaked
        // near 1 GB at 150 sprites and would reach multiple GB for a full 1000-ID
        // run, where an allocation failure aborts the whole extraction.
        let mut extracted: Vec<(u16, PathBuf)> = Vec::new();

        for &id in &id_list {
            let casc_path = format!("{}anim/main_{:03}.anim", prefix, id);

            if !passes_filter(&casc_path, &include, &exclude) {
                progress.increment();
                continue;
            }

            progress.update_current_file(&casc_path);

            if let Ok(data) = archive.extract_file(&casc_path) {
                let output_path = output.join(format!("main_{:03}.anim", id));
                if !should_skip(&output_path, overwrite, session_start) {
                    if let Err(e) = write_atomic(&output_path, &data) {
                        eprintln!("Warning: could not write {:?}: {}", output_path, e);
                    } else {
                        extracted.push((id, output_path));
                    }
                }
            }
            // Silently skip missing anim IDs — not every ID exists.

            progress.increment();
        }

        progress.finish(0, 0);

        // -----------------------------------------------------------------------
        // Phase 2 — Parallel: pure-Rust PNG conversion (no archive access).
        // -----------------------------------------------------------------------
        let id_to_name_ref = &id_to_name;
        let output_ref = output;
        let layers_ref = &layers;

        let results: Vec<anyhow::Result<()>> = extracted
            .par_iter()
            .map(|(id, output_path)| -> anyhow::Result<()> {
                let mapped_name = id_to_name_ref.get(&id.to_string()).cloned();
                // Tracks where the bytes actually live: the rename below may move
                // the file before we read it back.
                let mut source_path = output_path.clone();
                // When a mapping exists, the named path IS the primary output.
                // Create subdirectories as needed (e.g. terran/marine.anim).
                if let Some(ref name) = mapped_name {
                    let named_path = join_within(output_ref, &format!("{}.anim", name))?;
                    if let Some(parent) = named_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if named_path != *output_path {
                        if let Err(e) = fs::rename(output_path, &named_path) {
                            // rename may fail across filesystems; fall back to copy+delete
                            if fs::copy(output_path, &named_path).is_ok() {
                                let _ = fs::remove_file(output_path);
                                source_path = named_path.clone();
                            } else {
                                eprintln!(
                                    "Warning: could not move to named path {:?}: {}",
                                    named_path, e
                                );
                            }
                        } else {
                            source_path = named_path.clone();
                        }
                    }
                }

                let data = fs::read(&source_path)
                    .with_context(|| format!("re-reading {}", source_path.display()))?;

                match HdAnimFile::parse(&data) {
                    Ok(anim) => {
                        let export_cfg = ExportConfig {
                            convert_to_png: true,
                            team_color_mask,
                            save_dds,
                            generate_metadata: gen_meta,
                            pixels_per_unit,
                            layers: layers_ref.clone(),
                            png_compression: sprite_png_compression,
                        };
                        match export_anim(&anim, &output_path.with_extension(""), &export_cfg) {
                            Ok(result) => {
                                if let Some(ref name) = mapped_name {
                                    println!(
                                        "  {}.anim ({}, {} frames, {:.1} MB) tc={}",
                                        name,
                                        output_path.display(),
                                        result.frame_count,
                                        data.len() as f64 / 1_000_000.0,
                                        result.tc_mask_written
                                    );
                                } else {
                                    println!(
                                        "  {} ({} frames, {:.1} MB) tc={}",
                                        output_path.display(),
                                        result.frame_count,
                                        data.len() as f64 / 1_000_000.0,
                                        result.tc_mask_written
                                    );
                                }
                                Ok(())
                            }
                            Err(e) => Err(anyhow::anyhow!("  Export failed: {}", e)),
                        }
                    }
                    Err(e) => Err(anyhow::anyhow!(
                        "  {} - Parse error: {}",
                        output_path.display(),
                        e
                    )),
                }
            })
            .collect();

        for result in results {
            if let Err(e) = result {
                println!("  Export failed: {}", e);
            }
        }
    } else {
        // -----------------------------------------------------------------------
        // Non-PNG path: sequential write, fast enough without parallelism.
        // -----------------------------------------------------------------------
        let mut progress = casc_extractor::ProgressReporter::new(
            id_list.len() as u64,
            config.feedback_settings.verbose_logging,
        );

        for &id in &id_list {
            let casc_path = format!("{}anim/main_{:03}.anim", prefix, id);

            if !passes_filter(&casc_path, &include, &exclude) {
                progress.increment();
                continue;
            }

            progress.update_current_file(&casc_path);

            if let Ok(data) = archive.extract_file(&casc_path) {
                let output_name = format!("main_{:03}.anim", id);
                let output_path = output.join(&output_name);

                if should_skip(&output_path, overwrite, session_start) {
                    progress.increment();
                    continue;
                }

                write_atomic(&output_path, &data)?;

                let mapped_name = id_to_name.get(&id.to_string()).cloned();
                if let Some(ref name) = mapped_name {
                    let named_path = join_within(output, &format!("{}.anim", name))?;
                    if let Some(parent) = named_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let mut final_path = output_path.clone();
                    if named_path != output_path {
                        match fs::rename(&output_path, &named_path) {
                            Ok(()) => final_path = named_path.clone(),
                            Err(e) => {
                                // rename can fail across filesystems; fall back to copy+delete
                                if fs::copy(&output_path, &named_path).is_ok() {
                                    let _ = fs::remove_file(&output_path);
                                    final_path = named_path.clone();
                                } else {
                                    eprintln!(
                                        "Warning: could not move to named path {:?}: {}",
                                        named_path, e
                                    );
                                }
                            }
                        }
                    }
                    println!(
                        "  {}.anim ({}, {:.1} MB)",
                        name,
                        final_path.display(),
                        data.len() as f64 / 1_000_000.0
                    );
                } else {
                    println!(
                        "  {} ({:.1} MB)",
                        output_path.display(),
                        data.len() as f64 / 1_000_000.0
                    );
                }
            }
            // Silently skip missing anim IDs — not every ID exists.

            progress.increment();
        }

        progress.finish(0, 0);
    }

    println!("\nExtraction complete!");
    println!("Files saved to: {:?}", output);
    Ok(())
}

pub fn cmd_extract_tileset(
    archive: &CascArchive,
    output: &Path,
    quality: QualityLevel,
    convert_to_png: bool,
    config: &ExtractionConfig,
) -> Result<()> {
    let session_start = std::time::SystemTime::now();

    fs::create_dir_all(output)?;
    println!("Extracting Tilesets...");
    println!("Quality: {}", quality.description());
    println!("Output:  {:?}\n", output);

    let prefix = quality.path_prefix();
    let overwrite = config.output_settings.overwrite_behavior;
    let tilesets = [
        "badlands", "platform", "ashworld", "jungle", "desert", "ice", "twilight", "install",
    ];

    // Phase 1 — Sequential: extract raw bytes from the archive (FFI, non-Send).
    let mut extracted: Vec<(String, PathBuf, Vec<u8>)> = Vec::new();
    for tileset in &tilesets {
        let output_name = format!("{}.dds.vr4", tileset);
        let output_path = output.join(&output_name);
        if should_skip(&output_path, overwrite, session_start) {
            println!("  {} (skipped)", output_path.display());
            continue;
        }
        let path = format!("{}tileset/{}.dds.vr4", prefix, tileset);
        match archive.extract_file(&path) {
            Ok(data) => extracted.push((output_name.clone(), output_path, data)),
            Err(e) => println!("  {} - {}", tileset, e),
        }
    }

    // Phase 2 — Parallel: write files to disk (I/O-bound, no archive access).
    let results: Vec<anyhow::Result<()>> = extracted
        .par_iter()
        .map(|(_output_name, output_path, data)| -> anyhow::Result<()> {
            write_atomic(output_path, data)?;
            let mut note = String::new();
            if convert_to_png {
                // .dds.vr4 files have a 20-byte VR4 header before the DDS data.
                // Search within the first 64 bytes for the DDS magic.
                let dds_offset = data.windows(4).take(64).position(|w| w == b"DDS ");
                if let Some(offset) = dds_offset {
                    let png_path = output_path.with_extension("png");
                    match casc_extractor::dds_converter::save_dds_as_png(&data[offset..], &png_path)
                    {
                        Ok(()) => note = " → PNG".to_string(),
                        Err(e) => note = format!(" (PNG failed: {})", e),
                    }
                }
            }
            println!(
                "  {} ({:.1} MB){}",
                output_path.display(),
                data.len() as f64 / 1_000_000.0,
                note
            );
            Ok(())
        })
        .collect();

    for result in results {
        if let Err(e) = result {
            println!("  Write failed: {}", e);
        }
    }

    println!("\nDone. Files saved to: {:?}", output);
    Ok(())
}

pub fn cmd_extract_effect(
    archive: &CascArchive,
    output: &Path,
    quality: QualityLevel,
    convert_to_png: bool,
    config: &ExtractionConfig,
) -> Result<()> {
    let session_start = std::time::SystemTime::now();

    fs::create_dir_all(output)?;
    println!("Extracting Effects...");
    println!("Quality: {}", quality.description());
    println!("Output:  {:?}\n", output);

    let prefix = quality.path_prefix();
    let overwrite = config.output_settings.overwrite_behavior;
    let effects = ["water_normal_1.dds.grp", "water_normal_2.dds.grp"];

    // Phase 1 — Sequential: extract raw bytes from the archive (FFI, non-Send).
    let mut extracted: Vec<(String, PathBuf, Vec<u8>)> = Vec::new();
    for effect in &effects {
        let output_path = output.join(effect);
        if should_skip(&output_path, overwrite, session_start) {
            println!("  {} (skipped)", effect);
            continue;
        }
        let path = format!("{}effect/{}", prefix, effect);
        match archive.extract_file(&path) {
            Ok(data) => extracted.push((effect.to_string(), output_path, data)),
            Err(e) => println!("  {} - {}", effect, e),
        }
    }

    let compression = png_compression(config.quality_settings.png_compression_level);

    // Phase 2 — Parallel: write files to disk (I/O-bound, no archive access).
    let results: Vec<anyhow::Result<()>> = extracted
        .par_iter()
        .map(|(_output_name, output_path, data)| -> anyhow::Result<()> {
            write_atomic(output_path, data)?;
            let mut note = String::new();
            if convert_to_png {
                let png_path = output_path.with_extension("png");
                if data.starts_with(b"DDS ") {
                    match casc_extractor::dds_converter::save_dds_as_png(data, &png_path) {
                        Ok(()) => note = " → PNG".to_string(),
                        Err(e) => note = format!(" (PNG failed: {})", e),
                    }
                } else if let Ok(grp) = GrpFile::parse(data) {
                    match build_spritesheet(&grp, &png_path, compression) {
                        Ok(()) => note = format!(" → PNG spritesheet ({} frames)", grp.frame_count),
                        Err(e) => note = format!(" (PNG failed: {})", e),
                    }
                }
            }
            println!(
                "  {} ({:.1} MB){}",
                output_path.display(),
                data.len() as f64 / 1_000_000.0,
                note
            );
            Ok(())
        })
        .collect();

    for result in results {
        if let Err(e) = result {
            println!("  Write failed: {}", e);
        }
    }

    println!("\nDone. Files saved to: {:?}", output);
    Ok(())
}

/// Build a spritesheet PNG from GRP frames and write it to `output_path`.
pub fn build_spritesheet(
    grp: &GrpFile,
    output_path: &Path,
    compression: png::Compression,
) -> Result<()> {
    let frames_per_row = 17usize;
    let rows = (grp.frame_count as usize).div_ceil(frames_per_row);
    let sheet_width = grp.width as u32 * frames_per_row as u32;
    let sheet_height = grp.height as u32 * rows as u32;

    let mut sheet_data = vec![0u8; (sheet_width * sheet_height * 4) as usize];

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
                    let dst_idx = (dst_y * sheet_width as usize + dst_x) * 4;
                    sheet_data[dst_idx..dst_idx + 4].copy_from_slice(&rgba[src_idx..src_idx + 4]);
                }
            }
        }
    }

    let file = File::create(output_path)?;
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, sheet_width, sheet_height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(compression);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&sheet_data)?;
    Ok(())
}
