/// Scan data files directly for ANIM magic numbers
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = "/Applications/StarCraft/Data/data";
    
    println!("Scanning data files for ANIM magic numbers...");
    
    // Check each data file
    for i in 0..=5 {
        let data_path = format!("{}/data.{:03}", data_dir, i);
        println!("\nScanning {}...", data_path);
        
        let mut file = File::open(&data_path)?;
        let file_size = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(0))?;
        
        println!("File size: {} bytes", file_size);
        
        // Scan through the file looking for ANIM magic (0x4D494E41)
        // Only scan first 100MB of each file to speed things up
        let mut anim_count = 0;
        let chunk_size = 1024 * 1024; // 1MB chunks
        let max_scan_size = 100 * 1024 * 1024; // 100MB max
        let scan_size = std::cmp::min(file_size, max_scan_size);
        let mut position = 0u64;
        
        while position < scan_size {
            let read_size = std::cmp::min(chunk_size, (scan_size - position) as usize);
            let mut buffer = vec![0u8; read_size];
            
            file.seek(SeekFrom::Start(position))?;
            file.read_exact(&mut buffer)?;
            
            // Look for ANIM magic in this chunk
            for (offset, window) in buffer.windows(4).enumerate() {
                if window == [0x41, 0x4E, 0x49, 0x4D] { // "ANIM" in little endian
                    let file_position = position + offset as u64;
                    println!("  Found ANIM magic at offset: 0x{:x} ({})", file_position, file_position);
                    anim_count += 1;
                    
                    // Try to read some more data to see if it's a valid anim file
                    if offset + 16 < buffer.len() {
                        let anim_data = &buffer[offset..offset + 16];
                        print!("    Next 16 bytes: ");
                        for &byte in anim_data {
                            print!("{:02x} ", byte);
                        }
                        println!();
                        
                        // Try to parse as anim header
                        let mut cursor = Cursor::new(anim_data);
                        if let Ok(magic) = cursor.read_u32::<LittleEndian>() {
                            if magic == 0x4D494E41 {
                                if let Ok(scale) = cursor.read_u8() {
                                    println!("    Scale: {}", scale);
                                }
                            }
                        }
                    }
                    
                    // Only show first few matches per file to avoid spam
                    if anim_count >= 5 {
                        println!("  ... (stopping after 5 matches)");
                        break;
                    }
                }
            }
            
            position += chunk_size as u64;
            
            // Progress indicator for large files
            if position % (50 * 1024 * 1024) == 0 {
                println!("  Progress: {} MB / {} MB", position / (1024 * 1024), scan_size / (1024 * 1024));
            }
        }
        
        println!("  Total ANIM magic numbers found: {}", anim_count);
    }
    
    Ok(())
}