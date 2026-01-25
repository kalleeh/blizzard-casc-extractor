/// Debug tool to examine raw CASC index file format
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let index_path = "/Applications/StarCraft/Data/data/0000000005.idx";
    
    println!("Examining index file: {}", index_path);
    
    let mut file = File::open(index_path)?;
    
    // Read and display the header
    println!("\n=== HEADER ===");
    
    let header_hash_size = file.read_u32::<LittleEndian>()?;
    println!("header_hash_size: {}", header_hash_size);
    
    let header_hash = file.read_u32::<LittleEndian>()?;
    println!("header_hash: 0x{:08x}", header_hash);
    
    let unk0 = file.read_u16::<LittleEndian>()?;
    println!("unk0: {} (should be 7)", unk0);
    
    let bucket_index = file.read_u8()?;
    println!("bucket_index: {}", bucket_index);
    
    let unk1 = file.read_u8()?;
    println!("unk1: {}", unk1);
    
    let entry_size_bytes = file.read_u8()?;
    println!("entry_size_bytes: {}", entry_size_bytes);
    
    let entry_offset_bytes = file.read_u8()?;
    println!("entry_offset_bytes: {}", entry_offset_bytes);
    
    let entry_key_bytes = file.read_u8()?;
    println!("entry_key_bytes: {}", entry_key_bytes);
    
    let archive_file_header_size = file.read_u8()?;
    println!("archive_file_header_size: {}", archive_file_header_size);
    
    let archive_total_size_maximum = file.read_u64::<LittleEndian>()?;
    println!("archive_total_size_maximum: {}", archive_total_size_maximum);
    
    // Skip any remaining header bytes
    let bytes_read_so_far = 24;
    if archive_file_header_size as usize > bytes_read_so_far {
        let remaining_header_bytes = archive_file_header_size as usize - bytes_read_so_far;
        println!("Skipping {} remaining header bytes", remaining_header_bytes);
        file.seek(SeekFrom::Current(remaining_header_bytes as i64))?;
    }
    
    println!("\n=== ENTRIES ===");
    println!("Entry format: {} key bytes + {} size bytes + {} offset bytes", 
        entry_key_bytes, entry_size_bytes, entry_offset_bytes);
    
    // Read first few entries
    for i in 0..5 {
        println!("\nEntry {}:", i);
        
        // Read key
        let mut key = vec![0u8; entry_key_bytes as usize];
        match file.read_exact(&mut key) {
            Ok(_) => {
                print!("  key: ");
                for &byte in &key {
                    print!("{:02x}", byte);
                }
                println!();
            }
            Err(e) => {
                println!("  Failed to read key: {}", e);
                break;
            }
        }
        
        // Read data file number
        let data_file_number = match entry_size_bytes {
            1 => file.read_u8().map(|v| v as u32),
            2 => file.read_u16::<LittleEndian>().map(|v| v as u32),
            3 => {
                let mut bytes = [0u8; 4];
                file.read_exact(&mut bytes[..3])?;
                Ok(u32::from_le_bytes(bytes))
            },
            4 => file.read_u32::<LittleEndian>(),
            _ => {
                println!("  Unsupported entry_size_bytes: {}", entry_size_bytes);
                break;
            }
        };
        
        match data_file_number {
            Ok(num) => println!("  data_file_number: {}", num),
            Err(e) => {
                println!("  Failed to read data_file_number: {}", e);
                break;
            }
        }
        
        // Read data file offset
        let data_file_offset = match entry_offset_bytes {
            1 => file.read_u8().map(|v| v as u32),
            2 => file.read_u16::<LittleEndian>().map(|v| v as u32),
            3 => {
                let mut bytes = [0u8; 4];
                file.read_exact(&mut bytes[..3])?;
                Ok(u32::from_le_bytes(bytes))
            },
            4 => file.read_u32::<LittleEndian>(),
            5 => {
                let mut bytes = [0u8; 8];
                file.read_exact(&mut bytes[..5])?;
                Ok(u64::from_le_bytes(bytes) as u32)
            },
            6 => {
                let mut bytes = [0u8; 8];
                file.read_exact(&mut bytes[..6])?;
                Ok(u64::from_le_bytes(bytes) as u32)
            },
            8 => file.read_u64::<LittleEndian>().map(|v| v as u32),
            _ => {
                println!("  Unsupported entry_offset_bytes: {}", entry_offset_bytes);
                break;
            }
        };
        
        match data_file_offset {
            Ok(offset) => println!("  data_file_offset: {}", offset),
            Err(e) => {
                println!("  Failed to read data_file_offset: {}", e);
                break;
            }
        }
    }
    
    // Show file size and calculate expected entry count
    let file_size = file.seek(SeekFrom::End(0))?;
    println!("\n=== FILE INFO ===");
    println!("Total file size: {} bytes", file_size);
    
    let header_size = archive_file_header_size as u64;
    let data_size = file_size - header_size;
    let entry_size = entry_key_bytes as u64 + entry_size_bytes as u64 + entry_offset_bytes as u64;
    let expected_entries = data_size / entry_size;
    
    println!("Header size: {} bytes", header_size);
    println!("Data size: {} bytes", data_size);
    println!("Entry size: {} bytes", entry_size);
    println!("Expected entries: {}", expected_entries);
    
    Ok(())
}