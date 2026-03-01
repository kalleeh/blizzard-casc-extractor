/// Reusable HD anim export logic extracted from the `extract_hd` binary.
///
/// `SpriteExporter` consolidates DDS saving, PNG conversion (with optional
/// team-color-mask stripping), and Unity-compatible JSON metadata generation
/// into a single, testable unit that multiple binaries can share.

use std::fs::File;
use std::io::Write;
use std::path::Path;
use anyhow::{Context, Result};
use crate::anim::HdAnimFile;
use crate::dds_converter::save_dds_as_png;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Controls which outputs `export_anim` produces.
pub struct ExportConfig {
    /// Convert the diffuse DDS layer to a `.png` file.
    pub convert_to_png: bool,
    /// When `convert_to_png` is `true`, also write a `_tc.png` grayscale+alpha
    /// team-colour mask and strip the hue from the diffuse PNG for pixels
    /// covered by the TC layer.
    pub team_color_mask: bool,
    /// Write the raw diffuse DDS bytes alongside the PNG.
    pub save_dds: bool,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            convert_to_png: true,
            team_color_mask: false,
            save_dds: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

/// Summary information returned after a successful export.
pub struct ExportResult {
    /// Base name used for the output files (without extension).
    pub name: String,
    /// Number of frames in the exported anim.
    pub frame_count: usize,
    /// `true` when a `_tc.png` team-colour mask was written.
    pub tc_mask_written: bool,
}

// ---------------------------------------------------------------------------
// Core export function
// ---------------------------------------------------------------------------

/// Export a single parsed `HdAnimFile` to the given output base path.
///
/// `output_base` should be the path **without extension**, e.g.
/// `output/hd/main_000`.  The function appends `.dds`, `.png`,
/// `_tc.png`, and `.json` as appropriate based on `config`.
///
/// The raw `.anim` data is **not** written here; callers are responsible for
/// saving the original bytes if needed (as the binary does at line 246 of
/// `extract_hd.rs`).
pub fn export_anim(
    anim: &HdAnimFile,
    output_base: &Path,
    config: &ExportConfig,
) -> Result<ExportResult> {
    let name = output_base
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("anim")
        .to_string();

    let mut tc_mask_written = false;

    if config.convert_to_png {
        if let Some(diffuse) = anim.get_diffuse_layer() {
            // Optionally save the raw DDS for reference.
            if config.save_dds {
                let dds_path = output_base.with_extension("dds");
                File::create(&dds_path)
                    .with_context(|| format!("creating DDS file {}", dds_path.display()))?
                    .write_all(diffuse)
                    .with_context(|| format!("writing DDS file {}", dds_path.display()))?;
            }

            let png_path = output_base.with_extension("png");

            if config.team_color_mask {
                // ---------------------------------------------------------
                // Team-colour-aware diffuse export
                // ---------------------------------------------------------
                // If the TC layer is present, strip the hue from pixels that
                // the TC layer covers before writing the diffuse PNG.
                let write_result = if anim.get_team_color_layer().is_some() {
                    match anim.diffuse_tc_stripped_image() {
                        Some(Ok(img)) => img
                            .save(&png_path)
                            .map_err(|e| {
                                std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    format!("PNG save error: {}", e),
                                )
                            }),
                        Some(Err(e)) => {
                            // TC-stripping failed — fall back to the plain diffuse.
                            eprintln!(
                                "     Warning: TC-stripping failed ({}), falling back to plain diffuse",
                                e
                            );
                            save_dds_as_png(diffuse, &png_path)
                        }
                        None => save_dds_as_png(diffuse, &png_path),
                    }
                } else {
                    // No TC layer in this file — write diffuse unchanged.
                    save_dds_as_png(diffuse, &png_path)
                };

                write_result.with_context(|| {
                    format!("writing diffuse PNG {}", png_path.display())
                })?;

                // ---------------------------------------------------------
                // Team-colour mask export (_tc.png)
                // ---------------------------------------------------------
                if anim.get_team_color_layer().is_some() {
                    let tc_png_path = {
                        let stem = output_base
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("anim");
                        output_base.with_file_name(format!("{}_tc.png", stem))
                    };

                    match anim.team_color_mask_image() {
                        Some(Ok(tc_img)) => {
                            tc_img
                                .save(&tc_png_path)
                                .with_context(|| {
                                    format!("saving TC mask PNG {}", tc_png_path.display())
                                })?;
                            tc_mask_written = true;
                        }
                        Some(Err(e)) => {
                            eprintln!("     Warning: TC mask decode error: {}", e);
                        }
                        None => {}
                    }
                }
            } else {
                // ---------------------------------------------------------
                // Plain diffuse export (no TC processing)
                // ---------------------------------------------------------
                // Mirror the binary: try ImageMagick first, fall back to the
                // ddsfile crate if ImageMagick is not available or fails.
                let dds_path = output_base.with_extension("dds");

                // Write DDS to disk temporarily for ImageMagick if it is not
                // already written.
                if !config.save_dds {
                    File::create(&dds_path)
                        .with_context(|| {
                            format!("creating temporary DDS file {}", dds_path.display())
                        })?
                        .write_all(diffuse)
                        .with_context(|| {
                            format!("writing temporary DDS file {}", dds_path.display())
                        })?;
                }

                let magick_ok = std::process::Command::new("magick")
                    .args(&[
                        dds_path.to_str().unwrap_or(""),
                        png_path.to_str().unwrap_or(""),
                    ])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);

                if !magick_ok {
                    save_dds_as_png(diffuse, &png_path).with_context(|| {
                        format!(
                            "PNG conversion failed for {} (install ImageMagick for better DDS support)",
                            png_path.display()
                        )
                    })?;
                }

                // Remove the temporary DDS unless the caller asked to keep it.
                if !config.save_dds {
                    let _ = std::fs::remove_file(&dds_path);
                }
            }
        }
    }

    // Write Unity-compatible JSON metadata.
    let json_path = output_base.with_extension("json");
    let metadata = generate_metadata(anim, &name);
    File::create(&json_path)
        .with_context(|| format!("creating metadata file {}", json_path.display()))?
        .write_all(metadata.as_bytes())
        .with_context(|| format!("writing metadata file {}", json_path.display()))?;

    Ok(ExportResult {
        name,
        frame_count: anim.frames.len(),
        tc_mask_written,
    })
}

// ---------------------------------------------------------------------------
// Metadata generation
// ---------------------------------------------------------------------------

/// Generate Unity-compatible JSON metadata for an HD anim file.
///
/// This is the same logic as the `generate_metadata` function in
/// `src/bin/extract_hd.rs` lines 11–42, lifted here so it can be reused
/// by other callers without depending on the binary.
pub fn generate_metadata(anim: &HdAnimFile, name: &str) -> String {
    // Get PNG dimensions from first image entry.
    let (width, height) = if let Some(img) = anim.entry.images.first() {
        (img.tex_width as u32, img.tex_height as u32)
    } else {
        (0, 0)
    };

    format!(
        r#"{{
  "name": "{}",
  "frameCount": {},
  "grpWidth": {},
  "grpHeight": {},
  "textureWidth": {},
  "textureHeight": {},
  "frames": [
{}
  ]
}}"#,
        name.trim_end_matches(".anim"),
        anim.frames.len(),
        anim.entry.grp_width,
        anim.entry.grp_height,
        width,
        height,
        anim.frames
            .iter()
            .enumerate()
            .map(|(i, f)| format!(
                r#"    {{"index": {}, "x": {}, "y": {}, "width": {}, "height": {}, "offsetX": {}, "offsetY": {}}}"#,
                i, f.x, f.y, f.width, f.height, f.x_offset, f.y_offset
            ))
            .collect::<Vec<_>>()
            .join(",\n")
    )
}
