// DDS to PNG converter for StarCraft HD assets

use ddsfile::{Dds, DxgiFormat, FourCC};
use image::{ImageBuffer, RgbaImage};
use std::io;

fn invalid(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

/// Decode DDS bytes to raw RGBA pixels, returning (pixels, width, height).
///
/// SC:R HD textures are BC1 (DXT1) or BC3 (DXT5). Both are decoded in-process;
/// this used to shell out to ImageMagick per file, which cost a process spawn
/// plus two temp files and measured ~62x slower than decoding directly.
pub fn dds_to_rgba_pixels(dds_data: &[u8]) -> io::Result<(Vec<u8>, u32, u32)> {
    let dds = Dds::read(&mut std::io::Cursor::new(dds_data))
        .map_err(|e| invalid(format!("DDS parse error: {}", e)))?;

    let width = dds.get_width();
    let height = dds.get_height();
    if width == 0 || height == 0 {
        return Err(invalid(format!(
            "DDS has zero extent ({}×{})",
            width, height
        )));
    }
    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| invalid(format!("DDS dimensions overflow ({}×{})", width, height)))?;

    let raw = dds
        .get_data(0)
        .map_err(|e| invalid(format!("DDS data error: {}", e)))?;

    // Uncompressed straight RGBA needs no block decode.
    if raw.len() == pixel_count * 4 && block_format(&dds).is_none() {
        return Ok((raw.to_vec(), width, height));
    }

    let format = block_format(&dds).ok_or_else(|| {
        invalid(format!(
            "unsupported DDS format ({}×{}, {} bytes); expected BC1/DXT1, BC3/DXT5 or uncompressed RGBA",
            width, height, raw.len()
        ))
    })?;

    // texture2ddecoder emits one BGRA u32 per pixel.
    let mut argb = vec![0u32; pixel_count];
    let w = width as usize;
    let h = height as usize;
    match format {
        BlockFormat::Bc1 => texture2ddecoder::decode_bc1(raw, w, h, &mut argb),
        BlockFormat::Bc3 => texture2ddecoder::decode_bc3(raw, w, h, &mut argb),
    }
    .map_err(|e| invalid(format!("{:?} decode failed: {}", format, e)))?;

    let mut rgba = vec![0u8; pixel_count * 4];
    for (px, out) in argb.iter().zip(rgba.chunks_exact_mut(4)) {
        let [b, g, r, a] = px.to_le_bytes();
        out.copy_from_slice(&[r, g, b, a]);
    }

    if matches!(format, BlockFormat::Bc1) {
        apply_bc1_punchthrough_alpha(raw, w, h, &mut rgba);
    }

    Ok((rgba, width, height))
}

/// Restore BC1 one-bit alpha, which `texture2ddecoder::decode_bc1` does not emit.
///
/// When a BC1 block has `color0 <= color1` it is in punchthrough mode and colour
/// index 3 means "transparent black". Without this the `teamcolor` layer — a BC1
/// mask whose blocks are 100% punchthrough — decodes fully opaque, so every pixel
/// reads as team-coloured.
fn apply_bc1_punchthrough_alpha(raw: &[u8], width: usize, height: usize, rgba: &mut [u8]) {
    let blocks_x = width.div_ceil(4);
    let blocks_y = height.div_ceil(4);

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let base = (by * blocks_x + bx) * 8;
            let Some(block) = raw.get(base..base + 8) else {
                return;
            };

            let c0 = u16::from_le_bytes([block[0], block[1]]);
            let c1 = u16::from_le_bytes([block[2], block[3]]);
            if c0 > c1 {
                continue; // opaque mode — all four indices are colours
            }
            let bits = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);

            for py in 0..4 {
                let y = by * 4 + py;
                if y >= height {
                    break;
                }
                for px in 0..4 {
                    let x = bx * 4 + px;
                    if x >= width {
                        break;
                    }
                    if (bits >> ((py * 4 + px) * 2)) & 0x3 == 3 {
                        let dst = (y * width + x) * 4;
                        rgba[dst..dst + 4].copy_from_slice(&[0, 0, 0, 0]);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum BlockFormat {
    Bc1,
    Bc3,
}

fn block_format(dds: &Dds) -> Option<BlockFormat> {
    if let Some(four_cc) = dds.header.spf.fourcc.as_ref() {
        return match four_cc.0 {
            FourCC::DXT1 => Some(BlockFormat::Bc1),
            FourCC::DXT4 | FourCC::DXT5 => Some(BlockFormat::Bc3),
            _ => None,
        };
    }
    match dds.get_dxgi_format() {
        Some(DxgiFormat::BC1_UNorm)
        | Some(DxgiFormat::BC1_UNorm_sRGB)
        | Some(DxgiFormat::BC1_Typeless) => Some(BlockFormat::Bc1),
        Some(DxgiFormat::BC3_UNorm)
        | Some(DxgiFormat::BC3_UNorm_sRGB)
        | Some(DxgiFormat::BC3_Typeless) => Some(BlockFormat::Bc3),
        _ => None,
    }
}

pub fn dds_to_png(dds_data: &[u8]) -> io::Result<RgbaImage> {
    let (pixels, width, height) = dds_to_rgba_pixels(dds_data)?;
    ImageBuffer::from_raw(width, height, pixels)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Failed to create image buffer"))
}

pub fn save_dds_as_png(dds_data: &[u8], output_path: &std::path::Path) -> io::Result<()> {
    let img = dds_to_png(dds_data)?;
    img.save(output_path)
        .map_err(|e| io::Error::other(format!("PNG save error: {}", e)))?;
    Ok(())
}

/// Build the team-colour mask PNG from the raw DDS bytes of the `teamcolor` layer.
///
/// The mask is emitted as-is: opaque white where the pixel is team-coloured,
/// fully transparent elsewhere. Consumers apply
/// `out.rgb = diffuse.rgb * player_colour / 255` wherever alpha > 0, per
/// `docs/team-color.md`.
pub fn build_tc_mask_png(tc_dds: &[u8]) -> io::Result<RgbaImage> {
    let (pixels, width, height) = dds_to_rgba_pixels(tc_dds)?;
    let pixel_count = (width as usize) * (height as usize);

    if pixels.len() < pixel_count * 4 {
        return Err(invalid(format!(
            "TC layer pixel buffer too small: expected {} bytes, got {}",
            pixel_count * 4,
            pixels.len()
        )));
    }

    ImageBuffer::from_raw(width, height, pixels)
        .ok_or_else(|| invalid("Failed to create TC mask image buffer"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal DDS container around a single 4x4 block.
    fn dds_with_block(fourcc: &[u8; 4], block: &[u8]) -> Vec<u8> {
        let mut d = vec![0u8; 128];
        d[0..4].copy_from_slice(b"DDS ");
        d[4..8].copy_from_slice(&124u32.to_le_bytes()); // header size
        d[8..12].copy_from_slice(&0x1007u32.to_le_bytes()); // caps|height|width|pixelformat
        d[12..16].copy_from_slice(&4u32.to_le_bytes()); // height
        d[16..20].copy_from_slice(&4u32.to_le_bytes()); // width
        d[76..80].copy_from_slice(&32u32.to_le_bytes()); // pixelformat size
        d[80..84].copy_from_slice(&0x4u32.to_le_bytes()); // DDPF_FOURCC
        d[84..88].copy_from_slice(fourcc);
        d.extend_from_slice(block);
        d
    }

    #[test]
    fn bc1_punchthrough_index3_decodes_transparent() {
        // color0 <= color1 selects punchthrough mode, where index 3 is
        // transparent black. Indices: pixel 0 = 0 (white), pixel 1 = 3.
        let mut block = Vec::new();
        block.extend_from_slice(&0x0000u16.to_le_bytes()); // color0 (black)
        block.extend_from_slice(&0xFFFFu16.to_le_bytes()); // color1 (white) => c0 <= c1
                                                           // 2 bits per pixel, little-endian: pixel0=0, pixel1=3, rest 0
        block.extend_from_slice(&0b1100u32.to_le_bytes());

        let dds = dds_with_block(b"DXT1", &block);
        let (rgba, w, h) = dds_to_rgba_pixels(&dds).expect("BC1 decode");
        assert_eq!((w, h), (4, 4));
        assert_eq!(rgba[3], 255, "index 0 must stay opaque");
        assert_eq!(
            &rgba[4..8],
            &[0, 0, 0, 0],
            "index 3 must be transparent black"
        );
    }

    #[test]
    fn bc1_opaque_mode_keeps_all_pixels_opaque() {
        // color0 > color1 is the 4-colour opaque mode: no pixel may become
        // transparent, including index 3.
        let mut block = Vec::new();
        block.extend_from_slice(&0xFFFFu16.to_le_bytes()); // color0 (white)
        block.extend_from_slice(&0x0000u16.to_le_bytes()); // color1 (black) => c0 > c1
        block.extend_from_slice(&0b1100u32.to_le_bytes()); // pixel1 = index 3

        let dds = dds_with_block(b"DXT1", &block);
        let (rgba, _, _) = dds_to_rgba_pixels(&dds).expect("BC1 decode");
        for (i, px) in rgba.chunks_exact(4).enumerate() {
            assert_eq!(px[3], 255, "pixel {} must be opaque in 4-colour mode", i);
        }
    }

    #[test]
    fn unsupported_fourcc_is_an_error_not_a_panic() {
        let dds = dds_with_block(b"DXT2", &[0u8; 16]);
        assert!(dds_to_rgba_pixels(&dds).is_err());
    }
}
