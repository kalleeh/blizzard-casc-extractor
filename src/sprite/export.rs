use crate::anim::HdAnimFile;
use crate::dds_converter::save_dds_as_png;
use anyhow::{Context, Result};
/// Reusable HD anim export logic extracted from the `extract_hd` binary.
///
/// `SpriteExporter` consolidates DDS saving, PNG conversion (with optional
/// team-color-mask stripping), and Unity-compatible JSON metadata generation
/// into a single, testable unit that multiple binaries can share.
use std::fs::File;
use std::io::Write;
use std::path::Path;

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
    /// Write the Unity-compatible `.json` metadata file alongside the PNG.
    pub generate_metadata: bool,
    /// Pixels per unit for Unity sprite import.
    pub pixels_per_unit: f32,
    /// Which ANIM layers to export when convert_to_png is true.
    /// Valid values: "diffuse", "teamcolor", "normal", "specular", "emissive", "ao"
    /// Default: ["diffuse"] — same as current behaviour.
    pub layers: Vec<String>,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            convert_to_png: true,
            team_color_mask: false,
            save_dds: false,
            generate_metadata: true,
            pixels_per_unit: 100.0,
            layers: vec!["diffuse".to_string()],
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
                // The diffuse is written unmodified: SC:R's shader multiplies
                // the raw diffuse by the player colour where the mask is set,
                // so altering the diffuse here would make the documented
                // formula reproduce something the game never renders.
                save_dds_as_png(diffuse, &png_path)
                    .with_context(|| format!("writing diffuse PNG {}", png_path.display()))?;

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
                            tc_img.save(&tc_png_path).with_context(|| {
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
                save_dds_as_png(diffuse, &png_path)
                    .with_context(|| format!("writing diffuse PNG {}", png_path.display()))?;
            }
        }

        // -----------------------------------------------------------------
        // Export additional requested layers (normal, specular, emissive, ao …)
        // -----------------------------------------------------------------
        // Conventional layer order for HD ANIM files, used only when a file's
        // header does not name its layers. See docs/team-color.md.
        let conventional_index: &[(&str, usize)] = &[
            ("diffuse", 0),
            ("bright", 1),
            ("teamcolor", 2),
            ("emissive", 3),
            ("normal", 4),
            ("specular", 5),
            ("ao", 6),
        ];

        for layer_name in &config.layers {
            let lname = layer_name.to_lowercase();
            // "diffuse" is already handled above; skip it here.
            if lname == "diffuse" {
                continue;
            }

            // Resolve the layer index: try matching against the file's own
            // layer_names first, then fall back to the conventional table.
            // The header spells the last layer "ao_depth", so accept the "ao"
            // CLI alias as a prefix match.
            let layer_index = anim
                .header
                .layer_names
                .iter()
                .position(|n| {
                    let n = n.trim().to_lowercase();
                    n == lname || (lname == "ao" && n.starts_with("ao"))
                })
                .or_else(|| {
                    conventional_index
                        .iter()
                        .find(|(k, _)| *k == lname.as_str())
                        .map(|(_, idx)| *idx)
                });

            if let Some(idx) = layer_index {
                if let Some(layer_dds) = anim.get_layer(idx) {
                    let stem = output_base
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("anim");
                    let layer_png_path =
                        output_base.with_file_name(format!("{}_{}.png", stem, lname));

                    if let Err(e) = save_dds_as_png(layer_dds, &layer_png_path) {
                        eprintln!("     Warning: could not export {} layer: {}", lname, e);
                    }
                }
                // If get_layer returns None the layer simply isn't in this file — skip silently.
            }
        }
    }

    // Write Unity-compatible JSON metadata (skipped when generate_metadata is false).
    if config.generate_metadata {
        let json_path = output_base.with_extension("json");
        let metadata = generate_metadata_with_config(anim, &name, config.pixels_per_unit);
        File::create(&json_path)
            .with_context(|| format!("creating metadata file {}", json_path.display()))?
            .write_all(metadata.as_bytes())
            .with_context(|| format!("writing metadata file {}", json_path.display()))?;
    }

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
    generate_metadata_with_config(anim, name, 100.0)
}

/// Generate Unity-compatible JSON metadata, including `pixelsPerUnit`.
pub fn generate_metadata_with_config(
    anim: &HdAnimFile,
    name: &str,
    pixels_per_unit: f32,
) -> String {
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
  "pixelsPerUnit": {},
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
        pixels_per_unit,
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
