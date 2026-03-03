// DDS to PNG converter for StarCraft HD assets

use ddsfile::Dds;
use image::{ImageBuffer, RgbaImage};
use std::io;

/// Decode DDS bytes to raw RGBA pixels via ImageMagick, returning (pixels, width, height).
///
/// ImageMagick is used because `image::codecs::dxt` v0.24 only extracts the
/// alpha/luma channel for DXT5 (BC3), losing the colour information. ImageMagick
/// correctly decompresses all DXT variants used in SC:R HD anim files.
///
/// Falls back to the ddsfile crate for uncompressed RGBA textures.
pub fn dds_to_rgba_pixels(dds_data: &[u8]) -> io::Result<(Vec<u8>, u32, u32)> {
    // Write to a temp file so ImageMagick can read it
    use std::io::Write;
    let tmp = std::env::temp_dir().join(format!("sc_dds_{}.dds", std::process::id()));
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(dds_data)?;
    }

    // Convert to PNG via ImageMagick
    let png_tmp = tmp.with_extension("png");
    let status = std::process::Command::new("magick")
        .args([tmp.to_str().unwrap(), png_tmp.to_str().unwrap()])
        .status();

    let _ = std::fs::remove_file(&tmp);

    match status {
        Ok(s) if s.success() => {
            // Load the decoded PNG with the image crate
            let img = image::open(&png_tmp)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData,
                    format!("PNG load error: {}", e)))?
                .to_rgba8();
            let _ = std::fs::remove_file(&png_tmp);
            let (w, h) = img.dimensions();
            Ok((img.into_raw(), w, h))
        }
        _ => {
            let _ = std::fs::remove_file(&png_tmp);
            // ImageMagick unavailable — fall back to ddsfile raw read.
            // Only works for uncompressed RGBA textures.
            dds_to_rgba_pixels_fallback(dds_data)
        }
    }
}

/// ddsfile-based fallback for uncompressed RGBA DDS only.
fn dds_to_rgba_pixels_fallback(dds_data: &[u8]) -> io::Result<(Vec<u8>, u32, u32)> {
    let dds = Dds::read(&mut std::io::Cursor::new(dds_data))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("DDS parse error: {}", e)))?;

    let width  = dds.get_width();
    let height = dds.get_height();
    let rgba_size = (width * height * 4) as usize;

    let raw = dds.get_data(0)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("DDS data error: {}", e)))?;

    if raw.len() == rgba_size {
        return Ok((raw.to_vec(), width, height));
    }

    Err(io::Error::new(io::ErrorKind::InvalidData,
        format!("ImageMagick unavailable and DDS is compressed ({}×{}, {} bytes). \
                 Install ImageMagick to decode DXT5/BC3 textures.", width, height, raw.len())))
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

/// BT.601 luminance from an RGB triple (inputs and output are 0–255 u8).
#[inline]
pub fn bt601_luminance(r: u8, g: u8, b: u8) -> u8 {
    // Use integer arithmetic to avoid floating-point in hot paths.
    // Coefficients scaled by 1000: R*299 + G*587 + B*114, then / 1000.
    let lum = (r as u32 * 299 + g as u32 * 587 + b as u32 * 114 + 500) / 1000;
    lum.min(255) as u8
}

/// Build a team-color mask PNG from the raw DDS bytes of layer 1.
///
/// The output is an RGBA image where:
///   - R = G = B = BT.601 luminance of the TC layer pixel
///   - A = original alpha of the TC layer pixel
///
/// Pixels with alpha == 0 are written as (0, 0, 0, 0) — fully transparent.
pub fn build_tc_mask_png(tc_dds: &[u8]) -> io::Result<RgbaImage> {
    let (pixels, width, height) = dds_to_rgba_pixels(tc_dds)?;
    let pixel_count = (width * height) as usize;

    if pixels.len() < pixel_count * 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "TC layer pixel buffer too small: expected {} bytes, got {}",
                pixel_count * 4,
                pixels.len()
            ),
        ));
    }

    let mut out: Vec<u8> = Vec::with_capacity(pixel_count * 4);
    for i in 0..pixel_count {
        let base = i * 4;
        let r = pixels[base];
        let g = pixels[base + 1];
        let b = pixels[base + 2];
        let a = pixels[base + 3];

        if a == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            let lum = bt601_luminance(r, g, b);
            out.extend_from_slice(&[lum, lum, lum, a]);
        }
    }

    ImageBuffer::from_raw(width, height, out)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Failed to create TC mask image buffer"))
}

/// Build a diffuse PNG with hue stripped from team-color pixels.
///
/// For every pixel where the corresponding TC layer alpha is non-zero,
/// the diffuse R/G/B are replaced with their BT.601 luminance, keeping
/// the diffuse alpha unchanged.  Pixels where TC alpha == 0 are copied
/// unchanged from the diffuse layer.
///
/// Both `diffuse_dds` and `tc_dds` must decode to the same (width, height).
pub fn build_diffuse_tc_stripped_png(diffuse_dds: &[u8], tc_dds: &[u8]) -> io::Result<RgbaImage> {
    let (mut diff_pixels, dw, dh) = dds_to_rgba_pixels(diffuse_dds)?;
    let (tc_pixels, tw, th) = dds_to_rgba_pixels(tc_dds)?;

    if dw != tw || dh != th {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Diffuse ({}x{}) and TC layer ({}x{}) have different dimensions",
                dw, dh, tw, th
            ),
        ));
    }

    let pixel_count = (dw * dh) as usize;
    if diff_pixels.len() < pixel_count * 4 || tc_pixels.len() < pixel_count * 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Pixel buffer too small for declared dimensions",
        ));
    }

    for i in 0..pixel_count {
        let base = i * 4;
        let tc_alpha = tc_pixels[base + 3];

        if tc_alpha != 0 {
            // Replace RGB with luminance; keep diffuse alpha.
            let lum = bt601_luminance(diff_pixels[base], diff_pixels[base + 1], diff_pixels[base + 2]);
            diff_pixels[base]     = lum;
            diff_pixels[base + 1] = lum;
            diff_pixels[base + 2] = lum;
            // diff_pixels[base + 3] unchanged (diffuse alpha)
        }
    }

    ImageBuffer::from_raw(dw, dh, diff_pixels)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Failed to create stripped diffuse image buffer"))
}
