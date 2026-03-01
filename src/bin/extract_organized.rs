use casc_extractor::casc::casclib_ffi::CascArchive;
use casc_extractor::casc::discovery::locate_starcraft;
use casc_extractor::grp::GrpFile;
use casc_extractor::mapping::SpriteMapping;
use clap::Parser;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "extract-organized")]
#[command(about = "Extract StarCraft sprites organized by mapping file", long_about = None)]
struct Args {
    /// Path to the YAML sprite mapping file
    #[arg(default_value = "mappings/starcraft-remastered.yaml")]
    mapping_file: String,

    /// StarCraft installation directory (auto-detected if omitted)
    #[arg(long)]
    install_path: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mapping = SpriteMapping::load(Path::new(&args.mapping_file))
        .map_err(|e| anyhow::anyhow!("Failed to load mapping file '{}': {}", args.mapping_file, e))?;

    let install_dir = locate_starcraft(args.install_path.as_deref())?;
    let install_str = install_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Install path is not valid UTF-8: {:?}", install_dir))?;
    let archive = CascArchive::open(install_str)
        .map_err(|e| anyhow::anyhow!("Failed to open CASC archive at {}: {}", install_str, e))?;

    let base_output = Path::new("output");

    println!("StarCraft Sprite Extraction (Organized)");
    println!("Using mapping: {}", args.mapping_file);
    println!("--------------------------------------------------------\n");

    let mut stats: HashMap<&str, usize> = HashMap::new();
    let mut total_success = 0usize;
    let mut total_failed = 0usize;

    for (category_path, file_path) in &mapping.entries {
        // Create output directory structure
        let output_path = base_output.join(category_path);
        fs::create_dir_all(output_path.parent().unwrap())?;

        match archive.extract_file(file_path) {
            Ok(data) => {
                match GrpFile::parse(&data) {
                    Ok(grp) => {
                        // Create sprite sheet
                        let frames_per_row = 17;
                        let rows = (grp.frame_count as usize + frames_per_row - 1) / frames_per_row;
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

                        // Save PNG
                        let png_path = output_path.with_extension("png");
                        let file = File::create(&png_path)?;
                        let w = BufWriter::new(file);
                        let mut encoder = png::Encoder::new(w, sheet_width, sheet_height);
                        encoder.set_color(png::ColorType::Rgba);
                        encoder.set_depth(png::BitDepth::Eight);
                        let mut writer = encoder.write_header()?;
                        writer.write_image_data(&sheet_data)?;

                        // Save metadata
                        let meta_txt = format!(
                            "frames: {}\nframe_size: {}x{}\nsheet_size: {}x{}\nlayout: {}x{}\n",
                            grp.frame_count, grp.width, grp.height,
                            sheet_width, sheet_height, frames_per_row, rows
                        );
                        fs::write(output_path.with_extension("txt"), meta_txt)?;

                        // Save Unity JSON metadata
                        let unity_meta = format!(
                            r#"{{
  "frameCount": {},
  "frameWidth": {},
  "frameHeight": {},
  "framesPerRow": {},
  "rows": {},
  "sheetWidth": {},
  "sheetHeight": {},
  "frames": [{}
  ]
}}"#,
                            grp.frame_count,
                            grp.width,
                            grp.height,
                            frames_per_row,
                            rows,
                            sheet_width,
                            sheet_height,
                            (0..grp.frame_count)
                                .map(|i| {
                                    let col = i as usize % frames_per_row;
                                    let row = i as usize / frames_per_row;
                                    let x = col * grp.width as usize;
                                    let y = row * grp.height as usize;
                                    format!(
                                        "\n    {{\"index\": {}, \"x\": {}, \"y\": {}, \"width\": {}, \"height\": {}}}",
                                        i, x, y, grp.width, grp.height
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(",")
                        );
                        fs::write(output_path.with_extension("json"), unity_meta)?;

                        println!("  {}", category_path);

                        let category = category_path.split('/').next().unwrap();
                        *stats.entry(category).or_insert(0) += 1;
                        total_success += 1;
                    }
                    Err(e) => {
                        println!("  {}: Parse error - {}", category_path, e);
                        total_failed += 1;
                    }
                }
            }
            Err(_) => {
                total_failed += 1;
            }
        }
    }

    println!("\n--------------------------------------------------------");
    println!("Statistics:");
    for (category, count) in stats {
        println!("   {}: {} sprites", category, count);
    }
    println!("\nSuccess: {} | Failed: {}", total_success, total_failed);
    println!("Output: {:?}", base_output);

    Ok(())
}
