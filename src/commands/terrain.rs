//! Tileset and map-terrain rendering: VR4 geometry, DXT1 decode, CHK parsing.

use crate::QualityLevel;
use anyhow::{Context as _, Result};
use casc_extractor::casc::casclib_ffi::CascArchive;
use std::fs;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub fn cmd_extract_tileset_megatile(
    archive: &CascArchive,
    tileset: &str,
    max_tiles: u32,
    tile_size: u32,
    cols: u32,
    output: &Path,
) -> Result<()> {
    use image::{ImageBuffer, RgbaImage};

    // 1. Load VR4 pixel data — always use HD4 (no prefix) because VX4EX
    //    stores indices into the full HD4 tile array (up to 14316 tiles).
    //    Using HD2 (~3759 tiles) would leave most indices out of range.
    let vr4_path = format!("tileset/{}.dds.vr4", tileset);
    let vr4_raw = archive
        .extract_file(&vr4_path)
        .with_context(|| format!("VR4 not found: {}", vr4_path))?;

    // Each VR4 entry IS a full megatile.
    let layout = vr4_layout(&vr4_raw)?;
    let (tile_w, tile_h) = (layout.tile_w, layout.tile_h);
    let vr4_tile_count = layout.tile_count;
    println!("VR4 tiles: {} ({}×{})", vr4_tile_count, tile_w, tile_h);

    // 2. Load VX4EX — 1 entry per megatile (4 bytes): vr4_idx = entry>>1, flip_x = entry&1
    let vx4_path = format!("tileset/{}.vx4ex", tileset);
    let vx4_raw = archive
        .extract_file(&vx4_path)
        .with_context(|| format!("VX4EX not found: {}", vx4_path))?;

    let n_megatiles_vx4 = vx4_raw.len() / 4; // 4 bytes per megatile entry
    println!("VX4EX megatiles: {}", n_megatiles_vx4);

    let n_megatiles = if max_tiles == 0 {
        n_megatiles_vx4 as u32
    } else {
        max_tiles.min(n_megatiles_vx4 as u32)
    };

    let rows = n_megatiles.div_ceil(cols);
    let atlas_w = cols * tile_size;
    let atlas_h = rows * tile_size;
    let mut atlas: RgbaImage = ImageBuffer::new(atlas_w, atlas_h);

    println!(
        "Building megatile atlas: {}×{} ({} megatiles)",
        atlas_w, atlas_h, n_megatiles
    );

    for mega_idx in 0..n_megatiles as usize {
        let vx4_offset = mega_idx * 4;
        if vx4_offset + 4 > vx4_raw.len() {
            continue;
        }
        let entry = u32::from_le_bytes([
            vx4_raw[vx4_offset],
            vx4_raw[vx4_offset + 1],
            vx4_raw[vx4_offset + 2],
            vx4_raw[vx4_offset + 3],
        ]);
        let vr4_idx = (entry >> 1) as usize;
        let flip_x = (entry & 1) == 1;

        if vr4_idx >= vr4_tile_count {
            continue;
        }

        let Some(tile_bytes) = layout.tile_bytes(&vr4_raw, vr4_idx) else {
            continue;
        };
        let decoded = decode_dxt1_nxn(tile_bytes, tile_w as usize, tile_h as usize);

        let mut mini: RgbaImage = ImageBuffer::from_raw(tile_w, tile_h, decoded)
            .unwrap_or_else(|| ImageBuffer::new(tile_w, tile_h));
        if flip_x {
            mini = image::imageops::flip_horizontal(&mini);
        }

        // Scale the native tile → tile_size×tile_size for the final atlas
        let mega_scaled = if tile_size == tile_w {
            mini
        } else {
            image::imageops::resize(
                &mini,
                tile_size,
                tile_size,
                image::imageops::FilterType::Lanczos3,
            )
        };

        let ax = ((mega_idx as u32 % cols) * tile_size) as i64;
        let ay = ((mega_idx as u32 / cols) * tile_size) as i64;
        image::imageops::overlay(&mut atlas, &mega_scaled, ax, ay);
    }

    fs::create_dir_all(output)?;
    let png_name = format!("{}_megatile_{}.png", tileset, tile_size);
    let png_path = output.join(&png_name);
    atlas.save(&png_path)?;
    println!("Saved: {:?}", png_path);

    let meta = serde_json::json!({
        "tileset": tileset,
        "tile_size": tile_size,
        "cols": cols,
        "rows": rows,
        "n_megatiles": n_megatiles,
        "atlas_width": atlas_w,
        "atlas_height": atlas_h,
    });
    let json_path = output.join(format!("{}_megatile_{}.json", tileset, tile_size));
    std::fs::write(&json_path, serde_json::to_string_pretty(&meta)?)?;

    Ok(())
}

/// Geometry of a `tileset/*.dds.vr4` container.
///
/// A VR4 is not one big DDS: it is a 20-byte outer header followed by N
/// self-contained DDS files, each `stride` bytes apart (128-byte DDS header +
/// BC1 payload + 12 bytes of padding). Tile dimensions differ per quality —
/// 128×128 at full res, 64×64 under HD2 — so both stride and size must be read
/// from the first embedded header rather than hardcoded.
pub struct Vr4Layout {
    first_dds: usize,
    stride: usize,
    tile_w: u32,
    tile_h: u32,
    tile_count: usize,
}

pub fn vr4_layout(vr4_raw: &[u8]) -> Result<Vr4Layout> {
    const DDS_MAGIC: &[u8; 4] = b"DDS ";
    const DDS_HEADER: usize = 128;
    const PADDING: usize = 12;

    let first_dds = vr4_raw
        .windows(4)
        .position(|w| w == DDS_MAGIC)
        .ok_or_else(|| anyhow::anyhow!("VR4 contains no DDS tile (not a .dds.vr4 container?)"))?;

    if vr4_raw.len() < first_dds + DDS_HEADER {
        anyhow::bail!(
            "VR4 too short: {} bytes, need at least {} for the first DDS header",
            vr4_raw.len(),
            first_dds + DDS_HEADER
        );
    }

    let rd = |off: usize| -> u32 {
        u32::from_le_bytes([
            vr4_raw[off],
            vr4_raw[off + 1],
            vr4_raw[off + 2],
            vr4_raw[off + 3],
        ])
    };
    let tile_h = rd(first_dds + 12);
    let tile_w = rd(first_dds + 16);
    if tile_w == 0 || tile_h == 0 || tile_w > 4096 || tile_h > 4096 {
        anyhow::bail!("VR4 tile has implausible dimensions {}×{}", tile_w, tile_h);
    }

    // BC1: 8 bytes per 4×4 block.
    let payload = (tile_w.div_ceil(4) as usize) * (tile_h.div_ceil(4) as usize) * 8;
    let stride = DDS_HEADER + payload + PADDING;
    let tile_count = vr4_raw.len().saturating_sub(first_dds) / stride;

    Ok(Vr4Layout {
        first_dds,
        stride,
        tile_w,
        tile_h,
        tile_count,
    })
}

impl Vr4Layout {
    /// BC1 payload bytes of tile `idx`, or `None` if it lies outside the file.
    fn tile_bytes<'a>(&self, vr4_raw: &'a [u8], idx: usize) -> Option<&'a [u8]> {
        let start = self.first_dds + idx * self.stride + 128;
        let end = start + self.stride - 128 - 12;
        vr4_raw.get(start..end)
    }
}

/// Decode a 64×64 DXT1 (BC1) compressed tile to raw RGBA pixels.
/// Layer 0 of each VR4 stride slot = DXT1 diffuse color data.
pub fn decode_dxt1_nxn(data: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut pixels = vec![0u8; w * h * 4];
    let mut block_idx = 0usize;
    for by in (0..h).step_by(4) {
        for bx in (0..w).step_by(4) {
            let base = block_idx * 8;
            if base + 8 > data.len() {
                break;
            }
            let c0_raw = u16::from_le_bytes([data[base], data[base + 1]]);
            let c1_raw = u16::from_le_bytes([data[base + 2], data[base + 3]]);
            let bits = u32::from_le_bytes([
                data[base + 4],
                data[base + 5],
                data[base + 6],
                data[base + 7],
            ]);
            let c0 = rgb565_to_rgba(c0_raw);
            let c1 = rgb565_to_rgba(c1_raw);
            let colors: [[u8; 4]; 4] = if c0_raw > c1_raw {
                [c0, c1, lerp_color(c0, c1, 1, 3), lerp_color(c0, c1, 2, 3)]
            } else {
                [c0, c1, lerp_color(c0, c1, 1, 2), [0, 0, 0, 0]]
            };
            for py in 0..4 {
                for px in 0..4 {
                    let bit_shift = (py * 4 + px) * 2;
                    let color_idx = ((bits >> bit_shift) & 0x3) as usize;
                    let dst = ((by + py) * w + (bx + px)) * 4;
                    if dst + 4 <= pixels.len() {
                        pixels[dst..dst + 4].copy_from_slice(&colors[color_idx]);
                    }
                }
            }
            block_idx += 1;
        }
    }
    pixels
}

#[inline]
pub fn rgb565_to_rgba(c: u16) -> [u8; 4] {
    let r = ((c >> 11) & 0x1f) as u8;
    let g = ((c >> 5) & 0x3f) as u8;
    let b = (c & 0x1f) as u8;
    [
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
        255,
    ]
}

#[inline]
pub fn lerp_color(a: [u8; 4], b: [u8; 4], num: u32, den: u32) -> [u8; 4] {
    [
        ((a[0] as u32 * (den - num) + b[0] as u32 * num) / den) as u8,
        ((a[1] as u32 * (den - num) + b[1] as u32 * num) / den) as u8,
        ((a[2] as u32 * (den - num) + b[2] as u32 * num) / den) as u8,
        255,
    ]
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_extract_tileset_atlas(
    archive: &CascArchive,
    tileset: &str,
    quality: QualityLevel,
    cols: u32,
    max_tiles: u32,
    tile_size: u32,
    output: &Path,
) -> Result<()> {
    use image::{ImageBuffer, RgbaImage};

    let prefix = quality.path_prefix();
    let path = format!("{}tileset/{}.dds.vr4", prefix, tileset);
    let data = archive
        .extract_file(&path)
        .with_context(|| format!("Failed to extract {}", path))?;

    let layout = vr4_layout(&data)?;
    let (tile_w, tile_h) = (layout.tile_w, layout.tile_h);

    let total_tiles = layout.tile_count as u32;
    let n_tiles = if max_tiles == 0 {
        total_tiles
    } else {
        max_tiles.min(total_tiles)
    };

    println!(
        "Tileset: {} ({} total {}×{} tiles, extracting {})",
        tileset, total_tiles, tile_w, tile_h, n_tiles
    );

    let rows = n_tiles.div_ceil(cols);
    let atlas_w = cols * tile_size;
    let atlas_h = rows * tile_size;

    let mut atlas: RgbaImage = ImageBuffer::new(atlas_w, atlas_h);

    for i in 0..n_tiles {
        let Some(tile_bytes) = layout.tile_bytes(&data, i as usize) else {
            break;
        };

        let decoded = decode_dxt1_nxn(tile_bytes, tile_w as usize, tile_h as usize);

        let tile_img: RgbaImage = if tile_size == tile_w {
            ImageBuffer::from_raw(tile_w, tile_h, decoded)
                .ok_or_else(|| anyhow::anyhow!("Failed to create tile image"))?
        } else {
            let src: RgbaImage = ImageBuffer::from_raw(tile_w, tile_h, decoded)
                .ok_or_else(|| anyhow::anyhow!("Failed to create tile image"))?;
            image::imageops::resize(
                &src,
                tile_size,
                tile_size,
                image::imageops::FilterType::Lanczos3,
            )
        };

        let ax = (i % cols) * tile_size;
        let ay = (i / cols) * tile_size;
        image::imageops::overlay(&mut atlas, &tile_img, ax as i64, ay as i64);
    }

    fs::create_dir_all(output)?;

    let filename = format!("{}_{}_atlas.png", tileset, tile_size);
    let out_path = output.join(&filename);
    atlas.save(&out_path)?;
    println!(
        "Saved: {:?} ({}x{}, {} tiles)",
        out_path, atlas_w, atlas_h, n_tiles
    );

    let meta = serde_json::json!({
        "tileset": tileset,
        "tile_size": tile_size,
        "cols": cols,
        "rows": rows,
        "total_tiles": n_tiles,
        "atlas_width": atlas_w,
        "atlas_height": atlas_h,
    });
    let meta_path = output.join(format!("{}_{}_atlas.json", tileset, tile_size));
    fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;

    Ok(())
}

/// Map a CHK `ERA ` tileset id to its archive tileset name.
///
/// These are Brood War's canonical tileset indices; `install` at 2 is easy to
/// miss because no ladder map ships with it.
pub fn tileset_name_from_era(era: u16) -> &'static str {
    match era & 0x7 {
        0 => "badlands",
        1 => "platform",
        2 => "install",
        3 => "ashworld",
        4 => "jungle",
        5 => "desert",
        6 => "ice",
        _ => "twilight",
    }
}

/// The parts of a scenario.chk this tool renders terrain from.
pub struct ChkTerrain {
    era: u16,
    dim_w: u16,
    dim_h: u16,
    mtxm: Vec<u16>,
}

/// Parse `ERA `, `DIM ` and `MTXM` out of a scenario.chk.
///
/// Kept in one place deliberately: this logic used to be copy-pasted between
/// `map-terrain` and `map-terrain-dual`, which is why the ERA table bug had to be
/// fixed twice.
pub fn parse_chk_terrain(chk_bytes: &[u8]) -> ChkTerrain {
    let mut out = ChkTerrain {
        era: 4, // jungle
        dim_w: 128,
        dim_h: 128,
        mtxm: Vec::new(),
    };

    let mut pos = 0;
    while pos + 8 <= chk_bytes.len() {
        let tag = &chk_bytes[pos..pos + 4];
        let size = u32::from_le_bytes(chk_bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        pos += 8;
        if pos + size > chk_bytes.len() {
            break;
        }
        let data = &chk_bytes[pos..pos + size];

        match tag {
            b"ERA " if size >= 2 => {
                out.era = u16::from_le_bytes(data[0..2].try_into().unwrap()) & 7;
            }
            b"DIM " if size >= 4 => {
                out.dim_w = u16::from_le_bytes(data[0..2].try_into().unwrap());
                out.dim_h = u16::from_le_bytes(data[2..4].try_into().unwrap());
            }
            // A CHK may carry several MTXM chunks (map protection, overlays);
            // BW applies them in order with the last writer winning, so restart
            // the grid rather than appending onto the previous one.
            b"MTXM" => {
                out.mtxm.clear();
                for i in (0..size).step_by(2) {
                    if i + 2 <= size {
                        out.mtxm
                            .push(u16::from_le_bytes(data[i..i + 2].try_into().unwrap()));
                    }
                }
            }
            _ => {}
        }
        pos += size;
    }

    out
}

/// Read a scenario.chk either from an MPQ-wrapped .scm/.scx (`scm_path`) or
/// directly from a raw .chk file (`chk_path`); exactly one must be `Some`.
/// Returns the CHK bytes and the output filename stem.
pub fn acquire_chk_bytes(
    scm_path: Option<&Path>,
    chk_path: Option<&Path>,
    name: Option<&str>,
) -> Result<(Vec<u8>, String)> {
    use mpq::Archive;
    if let Some(chk_path) = chk_path {
        let bytes = fs::read(chk_path)
            .with_context(|| format!("Failed to read raw CHK: {:?}", chk_path))?;
        let stem = name
            .map(|s| s.to_string())
            .expect("clap requires --name alongside --chk");
        Ok((bytes, stem))
    } else {
        let scm_path = scm_path.expect("clap requires --scm when --chk absent");
        let mut archive_mpq = Archive::open(scm_path)
            .with_context(|| format!("Failed to open MPQ: {:?}", scm_path))?;
        let chk_file = archive_mpq
            .open_file("staredit\\scenario.chk")
            .or_else(|_| archive_mpq.open_file("staredit/scenario.chk"))
            .or_else(|_| archive_mpq.open_file("scenario.chk"))
            .with_context(|| "CHK not found in MPQ")?;
        let chk_size = chk_file.size() as usize;
        let mut chk_bytes = vec![0u8; chk_size];
        chk_file
            .read(&mut archive_mpq, &mut chk_bytes)
            .context("Failed to read CHK")?;
        let stem = scm_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        Ok((chk_bytes, stem))
    }
}

pub fn cmd_extract_map_terrain(
    archive: &CascArchive,
    scm_path: Option<&Path>,
    chk_path: Option<&Path>,
    name: Option<&str>,
    cols: u32,
    tile_size: u32,
    output: &Path,
) -> Result<()> {
    use image::{ImageBuffer, RgbaImage};

    fs::create_dir_all(output)?;

    // 1. Parse SCM/CHK to get ERA (tileset) and MTXM (megatile IDs)
    let (chk_bytes, map_stem) = acquire_chk_bytes(scm_path, chk_path, name)?;

    let ChkTerrain {
        era,
        dim_w,
        dim_h,
        mtxm,
    } = parse_chk_terrain(&chk_bytes);

    println!(
        "Map: {}×{} tiles, ERA={} (tileset), {} MTXM entries",
        dim_w,
        dim_h,
        era,
        mtxm.len()
    );

    // A truncated MTXM cannot describe the full dim_w×dim_h grid; the walk-map and
    // minimap loops below index mtxm[ty*dim_w + tx] over that full grid, so bail out
    // here rather than panic on a corrupt map.
    let expected_tiles = dim_w as usize * dim_h as usize;
    if mtxm.len() < expected_tiles {
        return Err(anyhow::anyhow!(
            "corrupt map: MTXM has {} entries, expected {} for {}×{} grid",
            mtxm.len(),
            expected_tiles,
            dim_w,
            dim_h
        ));
    }

    let tileset_name = tileset_name_from_era(era);

    // 2. Collect all unique megatile IDs. No cap needed — we use a single atlas PNG,
    // not a GPU texture array, so there is no hardware layer limit.
    let mut unique_ids: Vec<u16> = mtxm
        .iter()
        .copied()
        .collect::<std::collections::HashSet<u16>>()
        .into_iter()
        .collect();
    unique_ids.sort_unstable(); // stable atlas order
    let n_tiles = unique_ids.len() as u32;
    println!("Unique megatile IDs: {}", n_tiles);

    // 3. Load CV5, VX4EX, classic VR4 (8×8 palette tiles) + WPE palette from CASC
    // CV5: 52 bytes/group. MTXM → (group=id>>4, tile=id&0xf) → CV5[group].tiles[tile] = vx4_idx
    let cv5_data = archive
        .extract_file(&format!("tileset/{}.cv5", tileset_name))
        .with_context(|| format!("CV5 not found for tileset {}", tileset_name))?;
    const CV5_ENTRY_BYTES: usize = 52;
    const CV5_TILES_OFFSET: usize = 20;
    println!("CV5 groups: {}", cv5_data.len() / CV5_ENTRY_BYTES);

    let mtxm_to_vx4 = |mega_id: u16| -> Option<usize> {
        let group = (mega_id >> 4) as usize;
        let tile = (mega_id & 0xf) as usize;
        let off = group * CV5_ENTRY_BYTES + CV5_TILES_OFFSET + tile * 2;
        if off + 2 > cv5_data.len() {
            return None;
        }
        Some(u16::from_le_bytes(cv5_data[off..off + 2].try_into().unwrap()) as usize)
    };

    let vx4_data = archive
        .extract_file(&format!("tileset/{}.vx4ex", tileset_name))
        .with_context(|| format!("VX4EX not found for tileset {}", tileset_name))?;

    // Classic VR4: 64 bytes/tile = 8×8 palette index pixels. WPE: 256×4 bytes BGR0 palette.
    let classic_vr4 = archive
        .extract_file(&format!("tileset/{}.vr4", tileset_name))
        .with_context(|| format!("Classic VR4 not found for tileset {}", tileset_name))?;
    let wpe_data = archive
        .extract_file(&format!("tileset/{}.wpe", tileset_name))
        .with_context(|| format!("WPE palette not found for tileset {}", tileset_name))?;

    // Build RGBA palette lookup from WPE (RGB0 format — 3 colour bytes + 1 pad)
    let palette: Vec<[u8; 4]> = (0..256usize)
        .map(|i| {
            let base = i * 4;
            if base + 4 <= wpe_data.len() {
                [wpe_data[base], wpe_data[base + 1], wpe_data[base + 2], 255]
            } else {
                [0, 0, 0, 255]
            }
        })
        .collect();
    let classic_tile_count = classic_vr4.len() / 64;
    println!(
        "Classic VR4: {} tiles. WPE: {} palette entries.",
        classic_tile_count,
        wpe_data.len() / 4
    );

    // Decode one 8×8 classic VR4 tile → 32-byte RGBA pixels
    let decode_mini = |vr4_idx: usize| -> Vec<u8> {
        let mut out = vec![0u8; 8 * 8 * 4];
        let base = vr4_idx * 64;
        if base + 64 > classic_vr4.len() {
            return out;
        }
        for i in 0..64 {
            let c = palette[classic_vr4[base + i] as usize];
            out[i * 4..i * 4 + 4].copy_from_slice(&c);
        }
        out
    };

    // Load VF4 (walkability flags): 16 × u16 per megatile, bit 0x0001 = walkable mini-tile.
    // A megatile is walkable if ANY of its 16 mini-tiles has bit 0x0001 set.
    let vf4_data = archive
        .extract_file(&format!("tileset/{}.vf4", tileset_name))
        .with_context(|| format!("VF4 not found for tileset {}", tileset_name))?;
    const VF4_ENTRY_BYTES: usize = 32; // 16 × u16

    let is_walkable = |mega_id: u16| -> bool {
        let vx4_idx = match mtxm_to_vx4(mega_id) {
            Some(idx) => idx,
            None => return false,
        };
        let vf4_off = vx4_idx * VF4_ENTRY_BYTES;
        if vf4_off + VF4_ENTRY_BYTES > vf4_data.len() {
            return false;
        }
        // Any mini-tile walkable (bit 0x0001) → megatile is walkable
        for i in 0..16 {
            let flags = u16::from_le_bytes(
                vf4_data[vf4_off + i * 2..vf4_off + i * 2 + 2]
                    .try_into()
                    .unwrap(),
            );
            if flags & 0x0001 != 0 {
                return true;
            }
        }
        false
    };

    // Build walkable set for all MTXM values in this map
    let walkable_ids: Vec<u32> = mtxm
        .iter()
        .copied()
        .collect::<std::collections::HashSet<u16>>()
        .into_iter()
        .filter(|&id| is_walkable(id))
        .map(|id| id as u32)
        .collect();
    println!(
        "Walkable megatile IDs: {}/{}",
        walkable_ids.len(),
        mtxm.iter().collect::<std::collections::HashSet<_>>().len()
    );

    // Build per-mini-tile walk map at 4× tile resolution (WalkPosition grid).
    // Each 32×32 tile is divided into 4×4 mini-tiles of 8×8px each.
    // walk_map[(ty*4 + mini_row) * (dim_w*4) + (tx*4 + mini_col)] = 1 if walkable.
    // VF4 entry layout: 16 × u16, indexed as mini_row*4 + mini_col (row-major, top-to-bottom).
    let walk_w = dim_w as usize * 4;
    let walk_h = dim_h as usize * 4;
    let mut walk_map: Vec<u8> = vec![0u8; walk_w * walk_h];
    for ty in 0..dim_h as usize {
        for tx in 0..dim_w as usize {
            let mega_id = mtxm[ty * dim_w as usize + tx];
            let vx4_idx = match mtxm_to_vx4(mega_id) {
                Some(idx) => idx,
                None => continue,
            };
            let vf4_off = vx4_idx * VF4_ENTRY_BYTES;
            if vf4_off + VF4_ENTRY_BYTES > vf4_data.len() {
                continue;
            }
            for mini_row in 0..4usize {
                for mini_col in 0..4usize {
                    let i = mini_row * 4 + mini_col;
                    let flags = u16::from_le_bytes(
                        vf4_data[vf4_off + i * 2..vf4_off + i * 2 + 2]
                            .try_into()
                            .unwrap(),
                    );
                    if flags & 0x0001 != 0 {
                        let wx = tx * 4 + mini_col;
                        let wy = ty * 4 + mini_row;
                        walk_map[wy * walk_w + wx] = 1;
                    }
                }
            }
        }
    }

    // Build per-tile ground level (0 = low, 1 = mid, 2 = high) from the VF4
    // elevation bits: flag_middle 0x0002, flag_high 0x0004, flag_very_high 0x0008.
    //
    // BW derives a megatile's elevation from its 16 mini-tiles by COUNT, not by
    // "any bit set" (bwgame.h:21097-21099 in tiles_flags_and, which builds
    // tile_t::flag_middle / flag_high):
    //     high >= 12                        -> high  (2)
    //     high < 12 && middle + high >= 12  -> mid   (1)
    //     otherwise                          -> low   (0)
    // A separate flag_very_high (any mini-tile) marks fully sight-blocking
    // terrain and is emitted as level 3 so the consumer can treat it like BW's
    // flag_very_high, which blocks vision from every ground elevation.
    // Emitted id-keyed (like `walkable_ids`) rather than as a per-tile grid:
    // elevation is a pure function of the megatile id, so a compact
    // [id, level] table lets a consumer derive any map's heights from the
    // MTXM it already holds, and stays valid if the map is re-cropped.
    let ground_level = |mega_id: u16| -> u8 {
        let Some(vx4_idx) = mtxm_to_vx4(mega_id) else {
            return 0;
        };
        let vf4_off = vx4_idx * VF4_ENTRY_BYTES;
        if vf4_off + VF4_ENTRY_BYTES > vf4_data.len() {
            return 0;
        }
        let (mut middle_count, mut high_count, mut very_high_count) = (0u32, 0u32, 0u32);
        for i in 0..16usize {
            let flags = u16::from_le_bytes(
                vf4_data[vf4_off + i * 2..vf4_off + i * 2 + 2]
                    .try_into()
                    .unwrap(),
            );
            if flags & 0x0002 != 0 {
                middle_count += 1;
            }
            if flags & 0x0004 != 0 {
                high_count += 1;
            }
            if flags & 0x0008 != 0 {
                very_high_count += 1;
            }
        }
        if very_high_count > 0 {
            3
        } else if high_count >= 12 {
            2
        } else if middle_count + high_count >= 12 {
            1
        } else {
            0
        }
    };
    let mut height_levels: Vec<[u32; 2]> = unique_ids
        .iter()
        .map(|&id| [id as u32, ground_level(id) as u32])
        .filter(|&[_, lvl]| lvl != 0) // level 0 is the default; omit for compactness
        .collect();
    height_levels.sort_unstable();
    let level_hist = unique_ids.iter().fold([0u32; 4], |mut acc, &id| {
        acc[ground_level(id) as usize] += 1;
        acc
    });
    println!(
        "Ground levels over {} unique megatiles: low={} mid={} high={} very_high={}",
        unique_ids.len(),
        level_hist[0],
        level_hist[1],
        level_hist[2],
        level_hist[3]
    );

    // Pipeline: MTXM → CV5 → VX4EX → 16 classic VR4 mini-tiles (8×8) → 32×32 megatile.
    // 4. Build atlas: 4×4 classic 8×8 mini-tiles → 32×32 megatile → scaled to tile_size
    const MEGA_W: u32 = 32; // 4 mini-tiles × 8px = 32px megatile
    const MEGA_H: u32 = 32;

    let rows = n_tiles.div_ceil(cols);
    let atlas_w = cols * tile_size;
    let atlas_h = rows * tile_size;
    let mut atlas: RgbaImage = ImageBuffer::new(atlas_w, atlas_h);

    let max_id = *unique_ids.iter().max().unwrap_or(&0) as usize;
    let mut id_to_atlas = vec![u32::MAX; max_id + 1];

    for (atlas_idx, &mega_id) in unique_ids.iter().enumerate() {
        id_to_atlas[mega_id as usize] = atlas_idx as u32;

        let vx4_idx = match mtxm_to_vx4(mega_id) {
            Some(idx) => idx,
            None => continue,
        };
        let vx4_offset = vx4_idx * 64; // 16 × 4-byte entries
        let mut mega_img: RgbaImage = ImageBuffer::new(MEGA_W, MEGA_H);

        for mini_row in 0..4usize {
            for mini_col in 0..4usize {
                let entry_offset = vx4_offset + (mini_row * 4 + mini_col) * 4;
                if entry_offset + 4 > vx4_data.len() {
                    continue;
                }
                let entry = u32::from_le_bytes(
                    vx4_data[entry_offset..entry_offset + 4].try_into().unwrap(),
                );
                let vr4_idx = (entry >> 1) as usize;
                let flip_x = (entry & 1) == 1;
                if vr4_idx >= classic_tile_count {
                    continue;
                }

                let pixels = decode_mini(vr4_idx);
                let mut mini: RgbaImage =
                    ImageBuffer::from_raw(8, 8, pixels).unwrap_or_else(|| ImageBuffer::new(8, 8));
                if flip_x {
                    mini = image::imageops::flip_horizontal(&mini);
                }
                // Place at correct 8×8 position within the 32×32 megatile
                image::imageops::overlay(
                    &mut mega_img,
                    &mini,
                    (mini_col * 8) as i64,
                    (mini_row * 8) as i64,
                );
            }
        }

        let scaled = if tile_size == MEGA_W {
            mega_img
        } else {
            image::imageops::resize(
                &mega_img,
                tile_size,
                tile_size,
                image::imageops::FilterType::Nearest,
            )
        };

        let ax = (atlas_idx as u32 % cols) * tile_size;
        let ay = (atlas_idx as u32 / cols) * tile_size;
        image::imageops::overlay(&mut atlas, &scaled, ax as i64, ay as i64);
    }

    // 5. Save atlas and mapping
    let safe_name: String = map_stem
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let png_path = output.join(format!("{}_terrain.png", safe_name));
    atlas.save(&png_path)?;

    // 6. Generate minimap thumbnail: 1 pixel per tile, sampled from the atlas.
    // This matches BW's minimap which shows terrain colors at low resolution.
    let walkable_set_u16: std::collections::HashSet<u16> =
        walkable_ids.iter().map(|&x| x as u16).collect();
    let mut minimap: RgbaImage = ImageBuffer::new(dim_w as u32, dim_h as u32);
    for ty in 0..dim_h as u32 {
        for tx in 0..dim_w as u32 {
            let mega_id = mtxm[(ty * dim_w as u32 + tx) as usize];
            let pixel = if let Some(&atlas_idx) = id_to_atlas
                .get(mega_id as usize)
                .filter(|&&v| v != u32::MAX)
            {
                let ac = atlas_idx % cols;
                let ar = atlas_idx / cols;
                // Sample centre pixel of the atlas tile
                let px = ac * tile_size + tile_size / 2;
                let py = ar * tile_size + tile_size / 2;
                *atlas.get_pixel(px.min(atlas_w - 1), py.min(atlas_h - 1))
            } else if walkable_set_u16.contains(&mega_id) {
                image::Rgba([30u8, 60, 30, 255]) // walkable fallback: dark green
            } else {
                image::Rgba([40u8, 35, 30, 255]) // cliff fallback: dark brown
            };
            minimap.put_pixel(tx, ty, pixel);
        }
    }
    let mm_path = output.join(format!("{}_minimap.png", safe_name));
    minimap.save(&mm_path)?;

    let meta = serde_json::json!({
        "map": map_stem.as_str(),
        "tileset": tileset_name,
        "era": era,
        "tile_size": tile_size,
        "cols": cols,
        "atlas_width": atlas_w,
        "atlas_height": atlas_h,
        "map_width": dim_w,
        "map_height": dim_h,
        "n_unique_tiles": n_tiles,
        "id_map": unique_ids.iter().enumerate().map(|(i, &id)| [id as u32, i as u32]).collect::<Vec<_>>(),
        "walkable_ids": walkable_ids,
        "walk_map": walk_map,
        "walk_map_width": walk_w,
        "walk_map_height": walk_h,
        // VF4-derived ground level per megatile id: [megatile_id, level] pairs,
        // level 1 = mid, 2 = high, 3 = very high (fully sight-blocking).
        // Ids absent from this table are level 0 (low).
        "height_levels": height_levels,
    });
    let json_path = output.join(format!("{}_terrain.json", safe_name));
    fs::write(&json_path, serde_json::to_string_pretty(&meta)?)?;

    println!(
        "Saved: {:?} ({}×{}, {} tiles)",
        png_path, atlas_w, atlas_h, n_tiles
    );
    println!("Saved minimap: {:?} ({}×{})", mm_path, dim_w, dim_h);
    println!("Saved: {:?}", json_path);
    Ok(())
}

pub fn cmd_extract_map_terrain_dual(
    archive: &CascArchive,
    scm_path: &Path,
    cols: u32,
    tile_size: u32,
    output: &Path,
) -> Result<()> {
    use image::{ImageBuffer, RgbaImage};
    use mpq::Archive;

    fs::create_dir_all(output)?;

    // 1. Parse SCM to get ERA (tileset) and MTXM (megatile IDs)
    let mut archive_mpq =
        Archive::open(scm_path).with_context(|| format!("Failed to open MPQ: {:?}", scm_path))?;

    let chk_file = archive_mpq
        .open_file("staredit\\scenario.chk")
        .or_else(|_| archive_mpq.open_file("staredit/scenario.chk"))
        .or_else(|_| archive_mpq.open_file("scenario.chk"))
        .with_context(|| "CHK not found in MPQ")?;

    let chk_size = chk_file.size() as usize;
    let mut chk_bytes = vec![0u8; chk_size];
    chk_file
        .read(&mut archive_mpq, &mut chk_bytes)
        .context("Failed to read CHK")?;

    let ChkTerrain {
        era,
        dim_w,
        dim_h,
        mtxm,
    } = parse_chk_terrain(&chk_bytes);

    println!(
        "Map: {}×{} tiles, ERA={} (tileset), {} MTXM entries",
        dim_w,
        dim_h,
        era,
        mtxm.len()
    );

    let tileset_name = tileset_name_from_era(era);

    // 2. Collect unique megatile IDs
    let mut unique_ids: Vec<u16> = mtxm
        .iter()
        .copied()
        .collect::<std::collections::HashSet<u16>>()
        .into_iter()
        .collect();
    unique_ids.sort_unstable();
    let n_tiles = unique_ids.len() as u32;
    println!("Unique megatile IDs: {}", n_tiles);

    // 3. Load CV5, VX4EX and VR4 from CASC.
    // A MTXM id is not a VX4EX index: it resolves through CV5 first, as
    // MTXM → (group = id >> 4, tile = id & 0xf) → CV5[group].tiles[tile].
    // Indexing VX4EX directly leaves ~70% of lookups out of range.
    let cv5_data = archive
        .extract_file(&format!("tileset/{}.cv5", tileset_name))
        .with_context(|| format!("CV5 not found for tileset {}", tileset_name))?;
    const CV5_ENTRY_BYTES: usize = 52;
    const CV5_TILES_OFFSET: usize = 20;
    let cv5_lookup = |mega_id: u16| -> Option<usize> {
        let group = (mega_id >> 4) as usize;
        let tile = (mega_id & 0xf) as usize;
        let off = group * CV5_ENTRY_BYTES + CV5_TILES_OFFSET + tile * 2;
        if off + 2 > cv5_data.len() {
            return None;
        }
        Some(u16::from_le_bytes(cv5_data[off..off + 2].try_into().unwrap()) as usize)
    };

    let vr4_raw = archive
        .extract_file(&format!("tileset/{}.dds.vr4", tileset_name))
        .with_context(|| format!("VR4 not found for tileset {}", tileset_name))?;

    let layout = vr4_layout(&vr4_raw)?;
    let (vr4_w, vr4_h) = (layout.tile_w, layout.tile_h);
    let vr4_tile_count = layout.tile_count;
    println!("VR4 tiles: {} ({}×{})", vr4_tile_count, vr4_w, vr4_h);

    // 4. Build atlas
    let rows = n_tiles.div_ceil(cols);
    let atlas_w = cols * tile_size;
    let atlas_h = rows * tile_size;
    let mut atlas_diffuse: RgbaImage = ImageBuffer::new(atlas_w, atlas_h);
    let mut atlas_base: RgbaImage = ImageBuffer::new(atlas_w, atlas_h);

    let max_id = *unique_ids.iter().max().unwrap_or(&0) as usize;
    let mut id_to_atlas = vec![u32::MAX; max_id + 1];

    for (atlas_idx, &mega_id) in unique_ids.iter().enumerate() {
        id_to_atlas[mega_id as usize] = atlas_idx as u32;

        // In the HD tilesets each VR4 entry is already a whole 128×128 megatile,
        // addressed straight by the CV5-resolved index — there is no 4×4 mini-tile
        // assembly step (that is the classic 8×8-per-mini-tile SD layout).
        let Some(vr4_idx) = cv5_lookup(mega_id) else {
            continue;
        };
        if vr4_idx >= vr4_tile_count {
            continue;
        }
        let Some(tile_bytes) = layout.tile_bytes(&vr4_raw, vr4_idx) else {
            continue;
        };
        let decoded = decode_dxt1_nxn(tile_bytes, vr4_w as usize, vr4_h as usize);
        let mega_img: RgbaImage = ImageBuffer::from_raw(vr4_w, vr4_h, decoded)
            .unwrap_or_else(|| ImageBuffer::new(vr4_w, vr4_h));

        let scaled = if tile_size == vr4_w {
            mega_img
        } else {
            image::imageops::resize(
                &mega_img,
                tile_size,
                tile_size,
                image::imageops::FilterType::Lanczos3,
            )
        };
        let mut base = scaled.clone();
        for px in base.pixels_mut() {
            px[3] = 255;
        }

        let ax = (atlas_idx as u32 % cols) * tile_size;
        let ay = (atlas_idx as u32 / cols) * tile_size;
        image::imageops::overlay(&mut atlas_diffuse, &scaled, ax as i64, ay as i64);
        image::imageops::overlay(&mut atlas_base, &base, ax as i64, ay as i64);
    }

    // 5. Compute alpha stats for diffuse
    let total_pixels = (atlas_w * atlas_h) as u64;
    let transparent_pixels = atlas_diffuse.pixels().filter(|p| p[3] == 0).count() as u64;
    let semi_pixels = atlas_diffuse
        .pixels()
        .filter(|p| p[3] > 0 && p[3] < 255)
        .count() as u64;

    // 6. Save outputs
    let map_stem = scm_path.file_stem().unwrap_or_default().to_string_lossy();
    let safe_name: String = map_stem
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let diffuse_path = output.join(format!("{}_diffuse.png", safe_name));
    let base_path = output.join(format!("{}_base.png", safe_name));
    let json_path = output.join(format!("{}_terrain_dual.json", safe_name));

    atlas_diffuse.save(&diffuse_path)?;
    atlas_base.save(&base_path)?;

    let meta = serde_json::json!({
        "map": map_stem.as_ref(),
        "tileset": tileset_name,
        "era": era,
        "tile_size": tile_size,
        "cols": cols,
        "atlas_width": atlas_w,
        "atlas_height": atlas_h,
        "n_unique_tiles": n_tiles,
        "has_dual": true,
        "diffuse_alpha_stats": {
            "total_pixels": total_pixels,
            "transparent_pixels": transparent_pixels,
            "semi_transparent_pixels": semi_pixels,
            "opaque_pixels": total_pixels - transparent_pixels - semi_pixels,
        },
        "id_map": unique_ids.iter().enumerate().map(|(i, &id)| [id as u32, i as u32]).collect::<Vec<_>>(),
    });
    fs::write(&json_path, serde_json::to_string_pretty(&meta)?)?;

    println!(
        "Saved diffuse: {:?} ({}×{}, {} tiles)",
        diffuse_path, atlas_w, atlas_h, n_tiles
    );
    println!("Saved base:    {:?}", base_path);
    println!("Saved JSON:    {:?}", json_path);
    println!(
        "Diffuse alpha: {} transparent, {} semi, {} opaque (of {} total pixels)",
        transparent_pixels,
        semi_pixels,
        total_pixels - transparent_pixels - semi_pixels,
        total_pixels
    );
    Ok(())
}
