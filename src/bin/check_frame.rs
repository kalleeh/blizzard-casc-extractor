use casc_extractor::casc::casclib_ffi::CascArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive = CascArchive::open("/Applications/StarCraft")?;
    let data = archive.extract_file("unit\\protoss\\templar.grp")?;
    
    let frame_count = u16::from_le_bytes([data[0], data[1]]);
    
    // Check frames around 220
    for i in 218..223.min(frame_count as usize) {
        let offset_pos = 6 + (i * 8);
        let file_offset = u32::from_le_bytes([
            data[offset_pos + 4],
            data[offset_pos + 5],
            data[offset_pos + 6],
            data[offset_pos + 7],
        ]);
        
        // Get next frame offset
        let next_offset = if i + 1 < frame_count as usize {
            let next_pos = 6 + ((i + 1) * 8);
            u32::from_le_bytes([
                data[next_pos + 4],
                data[next_pos + 5],
                data[next_pos + 6],
                data[next_pos + 7],
            ])
        } else {
            data.len() as u32
        };
        
        let size = next_offset.saturating_sub(file_offset);
        println!("Frame {}: offset={}, next_offset={}, size={}", i, file_offset, next_offset, size);
    }
    
    println!("\nFile size: {}", data.len());
    
    Ok(())
}
