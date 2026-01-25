use std::io::Cursor;
use anyhow::Result;

fn main() -> Result<()> {
    println!("🎨 Testing Real PNG Generation");
    
    // Create a simple 64x64 test image with a pattern
    let width = 64u32;
    let height = 64u32;
    
    // Create RGBA pixel data (red gradient)
    let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let red = ((x * 255) / width) as u8;
            let green = ((y * 255) / height) as u8;
            let blue = 128u8;
            let alpha = 255u8;
            
            rgba_data.extend_from_slice(&[red, green, blue, alpha]);
        }
    }
    
    // Generate PNG using the png crate
    let mut png_data = Vec::new();
    let mut cursor = Cursor::new(&mut png_data);
    
    let mut encoder = png::Encoder::new(&mut cursor, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&rgba_data)?;
    writer.finish()?;
    
    // Save the PNG file
    let output_path = "extracted/test_real_sprite.png";
    std::fs::create_dir_all("extracted")?;
    std::fs::write(output_path, &png_data)?;
    
    println!("✅ Generated valid PNG: {} ({} bytes)", output_path, png_data.len());
    println!("📏 Dimensions: {}x{} RGBA", width, height);
    println!("🔍 You can now open this file: {}", output_path);
    
    Ok(())
}