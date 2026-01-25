use casc_extractor::casc::casclib_ffi::CascArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive = CascArchive::open("/Applications/StarCraft")?;
    let data = archive.extract_file("unit\\terran\\marine.grp")?;
    
    println!("Marine.grp: {} bytes", data.len());
    println!("First 32 bytes: {:02x?}", &data[0..32]);
    
    // Parse header manually
    let frame_count = u16::from_le_bytes([data[0], data[1]]);
    let width = u16::from_le_bytes([data[2], data[3]]);
    let height = u16::from_le_bytes([data[4], data[5]]);
    
    println!("\nHeader:");
    println!("  Frame count: {}", frame_count);
    println!("  Width: {}", width);
    println!("  Height: {}", height);
    
    // Parse first few offsets
    println!("\nFirst 10 frame offsets:");
    for i in 0..10.min(frame_count as usize) {
        let offset_pos = 6 + (i * 8);
        let x_offset = data[offset_pos];
        let y_offset = data[offset_pos + 1];
        let unknown = u16::from_le_bytes([data[offset_pos + 2], data[offset_pos + 3]]);
        let file_offset = u32::from_le_bytes([
            data[offset_pos + 4],
            data[offset_pos + 5],
            data[offset_pos + 6],
            data[offset_pos + 7],
        ]);
        println!("  Frame {}: x={}, y={}, unk={}, offset={} (0x{:08x})", 
                 i, x_offset, y_offset, unknown, file_offset, file_offset);
    }
    
    Ok(())
}
