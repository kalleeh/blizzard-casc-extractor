use casc_extractor::casc::casclib_ffi::CascArchive;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let filename = if args.len() > 1 {
        &args[1]
    } else {
        "unit\\terran\\marine.grp"
    };
    
    let archive = CascArchive::open("/Applications/StarCraft")?;
    let data = archive.extract_file(filename)?;
    
    println!("{}: {} bytes", filename, data.len());
    println!("First 64 bytes: {:02x?}", &data[0..64.min(data.len())]);
    
    let frame_count = u16::from_le_bytes([data[0], data[1]]);
    let width = u16::from_le_bytes([data[2], data[3]]);
    let height = u16::from_le_bytes([data[4], data[5]]);
    
    println!("\nHeader:");
    println!("  Frame count: {}", frame_count);
    println!("  Width: {}", width);
    println!("  Height: {}", height);
    
    println!("\nFirst 10 frame table entries:");
    for i in 0..10.min(frame_count as usize) {
        let offset_pos = 6 + (i * 8);
        if offset_pos + 8 > data.len() { break; }
        
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
    
    // Check last few frames
    println!("\nLast 5 frame table entries:");
    for i in (frame_count as usize).saturating_sub(5)..frame_count as usize {
        let offset_pos = 6 + (i * 8);
        if offset_pos + 8 > data.len() { break; }
        
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
    
    println!("\nFile size: {} bytes", data.len());
    println!("Frame table ends at: {} bytes", 6 + (frame_count as usize * 8));
    
    Ok(())
}
