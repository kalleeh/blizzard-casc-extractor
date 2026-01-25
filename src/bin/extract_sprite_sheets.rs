use casc_extractor::casc::casclib_ffi::CascArchive;
use casc_extractor::grp::GrpFile;
use std::path::Path;
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive = CascArchive::open("/Applications/StarCraft")?;
    let output_dir = Path::new("extracted/sprite_sheets");
    std::fs::create_dir_all(output_dir)?;
    
    let units = vec![
        ("Marine", "unit\\terran\\marine.grp"),
        ("Zealot", "unit\\protoss\\zealot.grp"),
        ("Zergling", "unit\\zerg\\zergling.grp"),
    ];
    
    for (name, filename) in units {
        let data = archive.extract_file(filename)?;
        let grp = GrpFile::parse(&data)?;
        
        println!("📦 {}: {}x{}, {} frames", name, grp.width, grp.height, grp.frame_count);
        
        // Create sprite sheet: 17 frames per row (standard SC animation)
        let frames_per_row = 17;
        let rows = (grp.frame_count as usize + frames_per_row - 1) / frames_per_row;
        let sheet_width = grp.width as u32 * frames_per_row as u32;
        let sheet_height = grp.height as u32 * rows as u32;
        
        let mut sheet_data = vec![0u8; (sheet_width * sheet_height * 4) as usize];
        
        // Copy each frame to sprite sheet
        for (idx, frame) in grp.frames.iter().enumerate() {
            let rgba = frame.to_rgba()?;
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
        
        // Save sprite sheet
        let png_path = output_dir.join(format!("{}_spritesheet.png", name));
        let file = File::create(&png_path)?;
        let w = BufWriter::new(file);
        let mut encoder = png::Encoder::new(w, sheet_width, sheet_height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&sheet_data)?;
        
        println!("   💾 Saved {}x{} sprite sheet ({} rows)", sheet_width, sheet_height, rows);
    }
    
    println!("\n📁 Saved to: {:?}", output_dir);
    Ok(())
}
