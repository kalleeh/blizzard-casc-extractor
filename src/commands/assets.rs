//! Remaining asset extraction: command icons, main-menu art, DAT tables, raw files.

use super::support::*;
use crate::fallbacks;
use anyhow::{Context as _, Result};
use casc_extractor::casc::casclib_ffi::CascArchive;
use std::fs;
use std::path::{Path, PathBuf};

/// Extract `unit\cmdicons\cmdicons.dds.grp` and write a colour-tinted PNG atlas.
///
/// ## File format (discovered by analysis)
/// The dds.grp starts with a 20-byte pre-header:
///   u32 total_file_size | u16 frame_count (390) | u16 flags | u32 zero |
///   u16 max_width (128) | u16 max_height (128) | u32 frame_alloc (8332)
///
/// Each of the 390 frames is a standalone DDS file prefixed by its own `DDS ` magic.
/// Frame sizes vary (128×128 = 8332 bytes most common; blank/portrait frames smaller).
/// Blank/placeholder frames use reduced dimensions (e.g. 116×36 for BLANK text).
///
/// ## Slot layout
/// Frames are in units.dat unit-ID order for the portrait section, then command buttons:
///   Slots   0–227: unit wireframe portraits  (frame == units.dat unit ID)
///   Slots 228–389: command button icons      (Move=228, Stop=229, …)
///
/// ## Tinting
/// Icons are stored as greyscale DXT1 (disabled/neutral state).  The game tints them
/// at runtime using the player colour.  We apply a static yellow tint:
///   out.rgb = tint_rgb * (grey / 255)
pub fn cmd_extract_cmdicons(
    archive: &casc_extractor::casc::casclib_ffi::CascArchive,
    output: &std::path::Path,
    tint: [u8; 3],
    save_raw: bool,
) -> anyhow::Result<()> {
    use texture2ddecoder::decode_bc1;

    let raw = archive
        .extract_file("unit\\cmdicons\\cmdicons.dds.grp")
        .map_err(|e| anyhow::anyhow!("cmdicons.dds.grp not found: {}", e))?;

    let total = u16::from_le_bytes([raw[4], raw[5]]) as usize;
    let max_w = u16::from_le_bytes([raw[12], raw[13]]) as usize;
    let max_h = u16::from_le_bytes([raw[14], raw[15]]) as usize;

    // Find all DDS frame start positions
    let mut positions: Vec<usize> = Vec::new();
    let mut i = 0usize;
    while let Some(rel) = raw[i..].windows(4).position(|w| w == b"DDS ") {
        positions.push(rel + i);
        i = rel + i + 1;
    }
    if positions.len() != total {
        println!(
            "  Warning: found {} DDS frames, expected {}",
            positions.len(),
            total
        );
    }
    println!(
        "  {} frames, max {}x{}, tint {:?}",
        positions.len(),
        max_w,
        max_h,
        tint
    );

    let cols = 17usize;
    let rows = total.div_ceil(cols);
    let sw = max_w * cols;
    let sh = max_h * rows;
    let blank = vec![0u8; sw * sh * 4];

    let mut tinted_sheet = blank.clone();
    let mut raw_sheet = if save_raw { blank.clone() } else { Vec::new() };

    for (slot, &pos) in positions.iter().enumerate() {
        if slot >= total {
            break;
        }
        let end = positions.get(slot + 1).copied().unwrap_or(raw.len());
        let frame_data = &raw[pos..end];

        if frame_data.len() < 128 {
            continue;
        }
        let fw = u32::from_le_bytes([
            frame_data[16],
            frame_data[17],
            frame_data[18],
            frame_data[19],
        ]) as usize;
        let fh = u32::from_le_bytes([
            frame_data[12],
            frame_data[13],
            frame_data[14],
            frame_data[15],
        ]) as usize;
        if fw == 0 || fh == 0 {
            continue;
        }

        let expected = fw.div_ceil(4) * fh.div_ceil(4) * 8;
        if frame_data.len() < 128 + expected {
            continue;
        }

        let mut px = vec![0u32; fw * fh];
        if decode_bc1(&frame_data[128..128 + expected], fw, fh, &mut px).is_err() {
            continue;
        }

        // Centre the (possibly smaller) frame in the max_w × max_h cell
        let col = slot % cols;
        let row = slot / cols;
        let xo = col * max_w + (max_w - fw) / 2;
        let yo = row * max_h + (max_h - fh) / 2;

        for y in 0..fh {
            for x in 0..fw {
                let p = px[y * fw + x];
                let r = (p & 0xFF) as u8;
                let g = ((p >> 8) & 0xFF) as u8;
                let b = ((p >> 16) & 0xFF) as u8;
                let a = ((p >> 24) & 0xFF) as u8;
                let lum = (r as u32 + g as u32 + b as u32) / 3;
                let di = ((yo + y) * sw + xo + x) * 4;
                if di + 4 > tinted_sheet.len() {
                    continue;
                }
                // Tinted
                tinted_sheet[di] = (tint[0] as u32 * lum / 255) as u8;
                tinted_sheet[di + 1] = (tint[1] as u32 * lum / 255) as u8;
                tinted_sheet[di + 2] = (tint[2] as u32 * lum / 255) as u8;
                tinted_sheet[di + 3] = a;
                // Raw greyscale
                if save_raw && di + 4 <= raw_sheet.len() {
                    raw_sheet[di] = r;
                    raw_sheet[di + 1] = g;
                    raw_sheet[di + 2] = b;
                    raw_sheet[di + 3] = a;
                }
            }
        }
    }

    std::fs::create_dir_all(output)?;
    let write_png = |path: &std::path::Path, data: &[u8]| -> anyhow::Result<()> {
        let img = image::RgbaImage::from_raw(sw as u32, sh as u32, data.to_vec())
            .ok_or_else(|| anyhow::anyhow!("icon buffer does not match {}x{}", sw, sh))?;
        casc_extractor::dds_converter::save_rgba_as_png(&img, path, png::Compression::Fast)?;
        Ok(())
    };

    let tinted_path = output.join("cmdicons.png");
    write_png(&tinted_path, &tinted_sheet)?;
    println!(
        "  Saved {}  ({}×{}, {} slots, tint {:?})",
        tinted_path.display(),
        sw,
        sh,
        total,
        tint
    );

    if save_raw {
        let raw_path = output.join("cmdicons_raw.png");
        write_png(&raw_path, &raw_sheet)?;
        println!("  Saved {} (greyscale)", raw_path.display());
    }

    // Write metadata JSON
    let meta = serde_json::json!({
        "frameCount": total, "frameWidth": max_w, "frameHeight": max_h,
        "framesPerRow": cols, "rows": rows, "sheetWidth": sw, "sheetHeight": sh,
        "layout": "slots 0-227 = unit portraits (frame == units.dat unit ID); slots 228-389 = command buttons",
        "commandButtons": {
            "Move":228,"Stop":229,"HoldPosition":230,"Patrol":231,"AttackMove":232,
            "Cancel":236,"StimPack":237,"U238Shells":238,"Lockdown":240,"EMP":241,
            "Irradiate":242,"SpiderMines":243,"SiegeTech":245,"SciVesselEnergy":248,
            "OcularImplants":249,"YamatoGun":251,"PersonnelCloak":252,"MoebiusReactor":256,
            "ColossusReactor":285,"IonThrusters":287,"InfantryWeapons":288,"VehicleWeapons":289,
            "ShipWeapons":290,"ShipPlating":291,"InfantryArmor":292,"VehiclePlating":293,
            "Restoration":366,"OpticFlare":373,"CharonBoosters":380,"CaduceusReactor":384
        },
        "frames": (0..total).map(|i| serde_json::json!({
            "index": i,
            "x": (i % cols) * max_w,
            "y": (i / cols) * max_h,
            "width": max_w, "height": max_h
        })).collect::<Vec<_>>()
    });
    write_atomic(
        &output.join("cmdicons.json"),
        serde_json::to_string_pretty(&meta)?.as_bytes(),
    )?;
    println!("  Saved {}", output.join("cmdicons.json").display());
    Ok(())
}

/// The in-game mouse cursors, as `(archive stem, output stem)`.
///
/// BW's full set. `arrow` is the default pointer; the `targ*`/`mag*` triples are the
/// targeting reticles and magnifier variants in green/yellow/red (valid / neutral /
/// hostile target); `drag` is the drag-select cursor, `illegal` the "cannot do that"
/// cross, and the eight `scroll*` are the map-edge scroll arrows.
const CURSOR_STEMS: &[&str] = &[
    "arrow", "drag", "illegal", "time",
    "targg", "targy", "targr",
    "magg", "magy", "magr",
    "scrollu", "scrolld", "scrolll", "scrollr",
    "scrollul", "scrollur", "scrolldl", "scrolldr",
];

/// Extract SC:R's mouse cursors (`cursor\*.dds.grp`) as one PNG strip each.
///
/// ## File format
/// Identical container to `cmdicons.dds.grp` (see [`cmd_extract_cmdicons`]): a
/// 20-byte pre-header —
///   `u32 total_file_size | u16 frame_count | u16 flags | u32 zero |
///    u16 max_width | u16 max_height | u32 frame_alloc`
/// — followed by `frame_count` standalone DDS files, each with its own `DDS ` magic.
///
/// ## But the pixels are NOT DXT-compressed
/// Unlike cmdicons' greyscale BC1, cursor frames are **uncompressed 32-bit RGBA**
/// (`DDSPF_RGB | DDSPF_ALPHAPIXELS`, i.e. pixel-format flags `0x41`, bit count 32,
/// masks `R=0x000000ff G=0x0000ff00 B=0x00ff0000 A=0xff000000`). So the payload is
/// copied straight out at 4 bytes per pixel and needs no block decode — and the
/// masks say the byte order really is R,G,B,A, not the BGRA that `glue\*.dds`
/// backgrounds use.
///
/// ## Output
/// One horizontal strip per cursor (`frame_count` cells of `max_width × max_height`),
/// plus a `cursors.json` manifest giving each cursor's frame count and cell size.
/// Cursors are animated in BW — `magg` has 14 frames — so a strip keeps the frames
/// addressable without inventing a packing scheme.
pub fn cmd_extract_cursors(
    archive: &casc_extractor::casc::casclib_ffi::CascArchive,
    output: &std::path::Path,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(output)?;
    let mut manifest = Vec::new();
    let mut extracted = 0usize;
    let mut skipped = 0usize;

    for stem in CURSOR_STEMS {
        let casc_path = format!("cursor\\{stem}.dds.grp");
        let Ok(raw) = archive.extract_file(&casc_path) else {
            println!("  skipped (not in local install): {casc_path}");
            skipped += 1;
            continue;
        };
        if raw.len() < 20 {
            println!("  skipped (truncated): {casc_path}");
            skipped += 1;
            continue;
        }

        let frames = u16::from_le_bytes([raw[4], raw[5]]) as usize;
        let max_w = u16::from_le_bytes([raw[12], raw[13]]) as usize;
        let max_h = u16::from_le_bytes([raw[14], raw[15]]) as usize;
        if frames == 0 || max_w == 0 || max_h == 0 {
            println!("  skipped (empty header): {casc_path}");
            skipped += 1;
            continue;
        }

        // Locate each embedded DDS the same way cmdicons does.
        let mut positions: Vec<usize> = Vec::new();
        let mut i = 0usize;
        while let Some(rel) = raw[i..].windows(4).position(|w| w == b"DDS ") {
            positions.push(rel + i);
            i = rel + i + 1;
        }
        if positions.len() != frames {
            println!(
                "  Warning: {stem} declares {frames} frames but holds {} DDS blocks",
                positions.len()
            );
        }

        let sw = max_w * frames;
        let mut strip = vec![0u8; sw * max_h * 4];

        for (slot, &pos) in positions.iter().enumerate().take(frames) {
            let end = positions.get(slot + 1).copied().unwrap_or(raw.len());
            let frame = &raw[pos..end];
            if frame.len() < 128 {
                continue;
            }
            // DDS header: height at +12, width at +16.
            let fh = u32::from_le_bytes([frame[12], frame[13], frame[14], frame[15]]) as usize;
            let fw = u32::from_le_bytes([frame[16], frame[17], frame[18], frame[19]]) as usize;
            if fw == 0 || fh == 0 || fw > max_w || fh > max_h {
                continue;
            }
            let need = fw * fh * 4;
            if frame.len() < 128 + need {
                continue;
            }
            let px = &frame[128..128 + need];

            // Centre a smaller frame in its cell, matching cmdicons' behaviour, so
            // every cell shares one hotspot-relative origin.
            let xo = slot * max_w + (max_w - fw) / 2;
            let yo = (max_h - fh) / 2;
            for y in 0..fh {
                for x in 0..fw {
                    let si = (y * fw + x) * 4;
                    let di = ((yo + y) * sw + xo + x) * 4;
                    if di + 4 > strip.len() {
                        continue;
                    }
                    strip[di..di + 4].copy_from_slice(&px[si..si + 4]);
                }
            }
        }

        let img = image::RgbaImage::from_raw(sw as u32, max_h as u32, strip)
            .ok_or_else(|| anyhow::anyhow!("{stem}: buffer does not match {sw}x{max_h}"))?;
        let png_path = output.join(format!("{stem}.png"));
        casc_extractor::dds_converter::save_rgba_as_png(&img, &png_path, png::Compression::Fast)?;
        println!("  Saved {} ({frames} frames, {max_w}x{max_h} each)", png_path.display());

        manifest.push(serde_json::json!({
            "name": stem,
            "frameCount": frames,
            "frameWidth": max_w,
            "frameHeight": max_h,
            "sheetWidth": sw,
            "sheetHeight": max_h,
        }));
        extracted += 1;
    }

    if extracted == 0 {
        anyhow::bail!("No cursors extracted — none present in the local install.");
    }

    let meta = serde_json::json!({
        "layout": "one horizontal strip per cursor; frame N occupies x = N * frameWidth",
        "pixelFormat": "uncompressed 32-bit RGBA (DDS pf_flags 0x41, no fourcc)",
        "cursors": manifest,
    });
    write_atomic(
        &output.join("cursors.json"),
        serde_json::to_string_pretty(&meta)?.as_bytes(),
    )?;
    println!("  Saved {}", output.join("cursors.json").display());
    println!("\n{extracted} cursor(s) extracted, {skipped} skipped.");
    Ok(())
}

/// Extract SC:R main-menu assets (backgrounds, logos, music, SFX, button webms).
pub fn cmd_extract_mainmenu(
    archive: &casc_extractor::casc::casclib_ffi::CascArchive,
    output: &std::path::Path,
    convert_dds: bool,
) -> anyhow::Result<()> {
    use casc_extractor::dds_converter::save_dds_as_png;

    std::fs::create_dir_all(output)?;

    let assets: &[(&str, &str)] = &[
        ("HD2\\glue\\title\\title.DDS", "title.dds"),
        (
            "HD2\\glue\\mainmenu\\titleframe_bg.DDS",
            "titleframe_bg.dds",
        ),
        (
            "HD2\\glue\\mainmenu\\titleframe_overlay.DDS",
            "titleframe_overlay.dds",
        ),
        ("HD2\\glue\\mainmenu\\etail.DDS", "etail.dds"),
        ("HD2\\glue\\mainmenu\\pintro.DDS", "pintro.dds"),
        ("HD2\\glue\\mainmenu\\pcredit.DDS", "pcredit.dds"),
        ("HD2\\glue\\mainmenu\\Lock.DDS", "lock.dds"),
        ("HD2\\glue\\mainmenu\\single.webm", "single.webm"),
        ("HD2\\glue\\mainmenu\\singleon.webm", "singleon.webm"),
        ("HD2\\glue\\mainmenu\\multi.webm", "multi.webm"),
        ("HD2\\glue\\mainmenu\\multion.webm", "multion.webm"),
        ("HD2\\glue\\mainmenu\\exit.webm", "exit.webm"),
        ("HD2\\glue\\mainmenu\\exiton.webm", "exiton.webm"),
        ("HD2\\glue\\mainmenu\\editor.webm", "editor.webm"),
        ("HD2\\glue\\mainmenu\\editoron.webm", "editoron.webm"),
        ("sound\\glue\\mouseover.wav", "sfx_mouseover.wav"),
        ("sound\\glue\\mousedown2.wav", "sfx_click.wav"),
        ("sound\\glue\\swishin.wav", "sfx_swishin.wav"),
        ("sound\\glue\\swishout.wav", "sfx_swishout.wav"),
        ("sound\\glue\\swishlock.wav", "sfx_swishlock.wav"),
        ("sound\\glue\\bnetclick.wav", "sfx_bnetclick.wav"),
        ("sound\\glue\\countdown.wav", "sfx_countdown.wav"),
        ("Music\\terran1.ogg", "music_terran.ogg"),
        ("Music\\zerg1.ogg", "music_zerg.ogg"),
        ("Music\\protoss1.ogg", "music_protoss.ogg"),
    ];

    let (mut ok, mut fail) = (0usize, 0usize);
    for (casc_path, out_name) in assets {
        match archive.extract_file(casc_path) {
            Ok(data) if !data.is_empty() => {
                if convert_dds && out_name.ends_with(".dds") {
                    let png_name = out_name.replace(".dds", ".png");
                    let png_path = output.join(&png_name);
                    match save_dds_as_png(&data, &png_path) {
                        Ok(()) => {
                            println!("  ✓ {} → {}", casc_path, png_name);
                            ok += 1;
                        }
                        Err(e) => {
                            write_atomic(&output.join(out_name), &data)?;
                            println!(
                                "  ⚠ {} → {} (PNG failed: {}, saved raw DDS)",
                                casc_path, out_name, e
                            );
                            ok += 1;
                        }
                    }
                } else {
                    write_atomic(&output.join(out_name), &data)?;
                    println!("  ✓ {} → {} ({} bytes)", casc_path, out_name, data.len());
                    ok += 1;
                }
            }
            Ok(_) => {
                println!("  ⚠ empty: {}", casc_path);
                fail += 1;
            }
            Err(_) => {
                println!("  ✗ not found: {}", casc_path);
                fail += 1;
            }
        }
    }
    println!("  {} extracted, {} not found → {:?}", ok, fail, output);
    Ok(())
}

/// Extract BW data tables (units.dat, upgrades.dat, images.dat, orders.dat, techdata.dat).
pub fn cmd_extract_dat(
    archive: &casc_extractor::casc::casclib_ffi::CascArchive,
    output: &std::path::Path,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(output)?;

    let files: &[(&str, &[&str])] = &[
        ("units.dat", &["arr\\units.dat", "arr/units.dat"]),
        ("upgrades.dat", &["arr\\upgrades.dat", "arr/upgrades.dat"]),
        ("images.dat", &["arr\\images.dat", "arr/images.dat"]),
        ("orders.dat", &["arr\\orders.dat", "arr/orders.dat"]),
        ("techdata.dat", &["arr\\techdata.dat", "arr/techdata.dat"]),
        ("weapons.dat", &["arr\\weapons.dat", "arr/weapons.dat"]),
        ("flingy.dat", &["arr\\flingy.dat", "arr/flingy.dat"]),
        ("sprites.dat", &["arr\\sprites.dat", "arr/sprites.dat"]),
        (
            "iscript.bin",
            &["scripts\\iscript.bin", "scripts/iscript.bin"],
        ),
        ("images.tbl", &["arr\\images.tbl", "arr/images.tbl"]),
        ("stat_txt.tbl", &["rez\\stat_txt.tbl", "rez/stat_txt.tbl"]),
    ];

    let (mut ok, mut fail) = (0usize, 0usize);
    for (name, paths) in files {
        let mut found = false;
        for p in *paths {
            if let Ok(d) = archive.extract_file(p) {
                if !d.is_empty() {
                    write_atomic(&output.join(name), &d)?;
                    println!("  ✓ {} → {} ({} bytes)", p, name, d.len());
                    ok += 1;
                    found = true;
                    break;
                }
            }
        }
        if !found {
            println!("  ✗ not found: {}", name);
            fail += 1;
        }
    }
    println!("  {} extracted, {} not found → {:?}", ok, fail, output);
    Ok(())
}

/// Extract raw bytes from the CASC archive without any format conversion.
///
/// Returns `Ok(())` as long as at least one file was extracted successfully.
/// Returns `Err` only when every requested file failed.
#[allow(clippy::too_many_arguments)]
pub fn cmd_extract_raw(
    archive: &CascArchive,
    install_path: Option<&Path>,
    file_paths: Vec<String>,
    search: Option<String>,
    locale: &str,
    output: &Path,
    preserve_path: bool,
    online: bool,
) -> Result<()> {
    fs::create_dir_all(output)?;

    // Collect the list of CASC paths to attempt.
    let paths: Vec<String> = if let Some(pattern) = search {
        // Use CascStorage to enumerate all archive paths, then filter by pattern + locale.
        // If the storage cannot be opened (no local install), fall back to searching
        // the embedded fallback table.
        match open_casc_storage(install_path) {
            Ok(storage) => {
                let all_files = storage
                    .list_files()
                    .map_err(|e| anyhow::anyhow!("list_files failed: {}", e))?;

                let pattern_lower = pattern.to_lowercase();
                let locale_lower = locale.to_lowercase();

                // Only localised assets live under `locales\<locale>\`; the vast
                // majority of the archive is locale-neutral. Applying the locale
                // as an unconditional AND would hide everything else, so it only
                // narrows results that are themselves locale-scoped.
                all_files
                    .into_iter()
                    .filter(|f| {
                        let fl = f.to_lowercase();
                        if !fl.contains(&pattern_lower) {
                            return false;
                        }
                        let is_localised =
                            fl.starts_with("locales\\") || fl.starts_with("locales/");
                        !is_localised || fl.contains(&locale_lower)
                    })
                    .collect()
            }
            Err(_) => {
                // No local install — search embedded fallbacks only.
                fallbacks::search(&pattern, locale)
                    .into_iter()
                    .map(|s| s.to_owned())
                    .collect()
            }
        }
    } else if online {
        // When fetching from the CDN, paths that lack a locale prefix won't resolve.
        // Automatically prepend `locales\<locale>\` to any path that doesn't already
        // start with that prefix.
        file_paths
            .into_iter()
            .map(|p| {
                let pl = p.to_lowercase();
                if pl.starts_with("locales/") || pl.starts_with("locales\\") {
                    p
                } else {
                    // Use backslash convention (matches SC:R CASC paths).
                    format!("locales\\{}\\{}", locale, p)
                }
            })
            .collect()
    } else {
        file_paths
    };

    if paths.is_empty() {
        println!("No files matched the request.");
        return Ok(());
    }

    let mut extracted = 0usize;
    let mut from_fallback = 0usize;
    let mut skipped = 0usize;
    // Without --preserve-path, distinct archive paths can share a basename and
    // silently overwrite each other (e.g. the same campaign .ogg under several
    // locales), so a plain "N extracted" would overstate what landed on disk.
    let mut written: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    let mut collisions = 0usize;

    for casc_path in &paths {
        // Resolve data: live archive first, then embedded fallback.
        let resolved: Option<(Vec<u8>, bool)> = match archive.extract_file(casc_path) {
            Ok(data) => Some((data, false)),
            Err(_) => fallbacks::get(casc_path).map(|embedded| (embedded.to_vec(), true)),
        };

        match resolved {
            Some((data, is_fallback)) => {
                // Determine the local output path.
                let local_path = if preserve_path {
                    // Normalise CASC backslashes to the OS separator, then join.
                    let normalised = casc_path.replace('\\', std::path::MAIN_SEPARATOR_STR);
                    let rel = PathBuf::from(normalised);
                    // Strip a leading separator if present (makes join work correctly).
                    let rel = rel
                        .strip_prefix(std::path::MAIN_SEPARATOR_STR)
                        .unwrap_or(&rel)
                        .to_path_buf();
                    output.join(rel)
                } else {
                    let filename = casc_path.rsplit(['\\', '/']).next().unwrap_or(casc_path);
                    output.join(filename)
                };

                // Create any intermediate directories when --preserve-path is set.
                if let Some(parent) = local_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                write_atomic(&local_path, &data)
                    .with_context(|| format!("Failed to write {:?}", local_path))?;

                if !written.insert(local_path.clone()) {
                    collisions += 1;
                    eprintln!(
                        "Warning: {} overwrote an earlier file at {} (use --preserve-path to keep both)",
                        casc_path,
                        local_path.display()
                    );
                }

                if is_fallback {
                    println!(
                        "fallback: {} -> {} ({} bytes) [embedded]",
                        casc_path,
                        local_path.display(),
                        data.len()
                    );
                    from_fallback += 1;
                } else {
                    println!(
                        "extracted: {} -> {} ({} bytes)",
                        casc_path,
                        local_path.display(),
                        data.len()
                    );
                    extracted += 1;
                }
            }
            None => {
                println!("skipped (not in local install): {}", casc_path);
                skipped += 1;
            }
        }
    }

    println!(
        "\n{} extracted, {} from embedded fallback, {} skipped (of {} requested).",
        extracted,
        from_fallback,
        skipped,
        paths.len()
    );
    if collisions > 0 {
        println!(
            "{} file(s) shared an output name and were overwritten — {} distinct file(s) on disk. Re-run with --preserve-path to keep all of them.",
            collisions,
            written.len()
        );
    }

    if extracted == 0 && from_fallback == 0 && skipped > 0 {
        anyhow::bail!("All {} requested file(s) were skipped — none present in local install or embedded fallbacks.", skipped);
    }

    Ok(())
}
