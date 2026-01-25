/// Debug tool to examine raw bytes in CASC index file
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let index_path = "/Applications/StarCraft/Data/data/0000000005.idx";
    
    println!("Examining raw bytes in: {}", index_path);
    
    let mut file = File::open(index_path)?;
    
    // Skip to after header (30 bytes)
    file.seek(SeekFrom::Start(30))?;
    
    // Read first entry raw bytes
    let mut entry_bytes = vec![0u8; 18]; // 9 key + 4 size + 5 offset
    file.read_exact(&mut entry_bytes)?;
    
    println!("\nFirst entry raw bytes:");
    print_hex_dump(&entry_bytes);
    
    println!("\nBreaking down the entry:");
    
    // Key (9 bytes)
    let key = &entry_bytes[0..9];
    print!("Key (9 bytes): ");
    for &byte in key {
        print!("{:02x}", byte);
    }
    println!();
    
    // Data file number (4 bytes)
    let data_file_bytes = &entry_bytes[9..13];
    print!("Data file number bytes: ");
    for &byte in data_file_bytes {
        print!("{:02x} ", byte);
    }
    println!();
    
    // Try different interpretations
    let as_u32_le = u32::from_le_bytes([data_file_bytes[0], data_file_bytes[1], data_file_bytes[2], data_file_bytes[3]]);
    let as_u32_be = u32::from_be_bytes([data_file_bytes[0], data_file_bytes[1], data_file_bytes[2], data_file_bytes[3]]);
    let as_u16_le = u16::from_le_bytes([data_file_bytes[0], data_file_bytes[1]]);
    let as_u8 = data_file_bytes[0];
    
    println!("  As u32 LE: {}", as_u32_le);
    println!("  As u32 BE: {}", as_u32_be);
    println!("  As u16 LE: {}", as_u16_le);
    println!("  As u8: {}", as_u8);
    
    // Offset (5 bytes)
    let offset_bytes = &entry_bytes[13..18];
    print!("Offset bytes: ");
    for &byte in offset_bytes {
        print!("{:02x} ", byte);
    }
    println!();
    
    // Try different interpretations for offset
    let mut offset_u64_bytes = [0u8; 8];
    offset_u64_bytes[..5].copy_from_slice(offset_bytes);
    let as_u64_le = u64::from_le_bytes(offset_u64_bytes);
    let as_u32_le_offset = u32::from_le_bytes([offset_bytes[0], offset_bytes[1], offset_bytes[2], offset_bytes[3]]);
    
    println!("  As u64 LE (5 bytes): {}", as_u64_le);
    println!("  As u32 LE (first 4 bytes): {}", as_u32_le_offset);
    
    // Read a few more entries to see patterns
    println!("\n=== Next few entries ===");
    for i in 1..5 {
        let mut entry_bytes = vec![0u8; 18];
        file.read_exact(&mut entry_bytes)?;
        
        let data_file_bytes = &entry_bytes[9..13];
        let as_u8 = data_file_bytes[0];
        let as_u16_le = u16::from_le_bytes([data_file_bytes[0], data_file_bytes[1]]);
        
        println!("Entry {}: data_file as u8={}, as u16={}", i, as_u8, as_u16_le);
    }
    
    Ok(())
}

fn print_hex_dump(data: &[u8]) {
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("{:04x}: ", i * 16);
        
        // Hex bytes
        for (j, &byte) in chunk.iter().enumerate() {
            print!("{:02x} ", byte);
            if j == 7 { print!(" "); }
        }
        
        // Padding for incomplete lines
        for _ in chunk.len()..16 {
            print!("   ");
            if chunk.len() <= 8 { print!(" "); }
        }
        
        print!(" |");
        
        // ASCII representation
        for &byte in chunk {
            if byte.is_ascii_graphic() {
                print!("{}", byte as char);
            } else {
                print!(".");
            }
        }
        
        println!("|");
    }
}