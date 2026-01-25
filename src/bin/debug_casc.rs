/// Debug tool to examine CASC file contents and identify sprite files
use std::path::Path;
use casc_extractor::casc::CascArchive;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let install_path = Path::new("/Applications/StarCraft");
    
    println!("Opening CASC archive...");
    let archive = CascArchive::open(install_path)?;
    
    println!("Listing files...");
    let files = archive.list_all_files()?;
    
    println!("Found {} files total", files.len());
    
    // Sample first 100 files to examine their magic numbers
    let sample_size = std::cmp::min(100, files.len());
    let mut anim_files = Vec::new();
    let mut magic_counts = std::collections::HashMap::new();
    
    println!("Examining first {} files for magic numbers...", sample_size);
    
    for (i, file) in files.iter().take(sample_size).enumerate() {
        if i % 10 == 0 {
            println!("Progress: {}/{}", i, sample_size);
        }
        
        match archive.extract_file_by_key(&file.key) {
            Ok(data) => {
                if data.len() >= 4 {
                    let mut cursor = Cursor::new(&data);
                    if let Ok(magic) = cursor.read_u32::<LittleEndian>() {
                        *magic_counts.entry(magic).or_insert(0) += 1;
                        
                        // Check for ANIM magic (0x4D494E41)
                        if magic == 0x4D494E41 {
                            anim_files.push((file.name.clone(), file.key, data.len()));
                            println!("Found ANIM file: {} (key: {:02x?}, size: {} bytes)", 
                                file.name, file.key, data.len());
                        }
                        
                        // Also check for other interesting magic numbers
                        match magic {
                            0x4D494E41 => {}, // ANIM - already handled
                            0x47525020 => println!("Found GRP file: {} (key: {:02x?})", file.name, file.key),
                            0x44445320 => println!("Found DDS file: {} (key: {:02x?})", file.name, file.key),
                            _ => {}
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed to extract file {}: {}", file.name, e);
            }
        }
    }
    
    println!("\n=== SUMMARY ===");
    println!("Found {} ANIM files in sample", anim_files.len());
    
    println!("\nMagic number distribution:");
    let mut sorted_magic: Vec<_> = magic_counts.iter().collect();
    sorted_magic.sort_by(|a, b| b.1.cmp(a.1));
    
    for (magic, count) in sorted_magic.iter().take(20) {
        let magic_str = format!("{:08x}", magic);
        let ascii_str = magic_to_ascii(**magic);
        println!("  0x{} ({}): {} files", magic_str, ascii_str, count);
    }
    
    if anim_files.is_empty() {
        println!("\nNo ANIM files found in sample. Let's examine some file contents:");
        
        // Show hex dump of first few files
        for (i, file) in files.iter().take(5).enumerate() {
            if let Ok(data) = archive.extract_file_by_key(&file.key) {
                println!("\nFile {}: {} (key: {:02x?})", i, file.name, file.key);
                print_hex_dump(&data, 64);
            }
        }
    }
    
    Ok(())
}

fn magic_to_ascii(magic: u32) -> String {
    let bytes = magic.to_le_bytes();
    let mut result = String::new();
    for &byte in &bytes {
        if byte.is_ascii_graphic() {
            result.push(byte as char);
        } else {
            result.push('.');
        }
    }
    result
}

fn print_hex_dump(data: &[u8], max_bytes: usize) {
    let len = std::cmp::min(data.len(), max_bytes);
    for (i, chunk) in data[..len].chunks(16).enumerate() {
        print!("  {:04x}: ", i * 16);
        
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
    
    if data.len() > max_bytes {
        println!("  ... ({} more bytes)", data.len() - max_bytes);
    }
}