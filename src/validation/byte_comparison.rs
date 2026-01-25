// Byte-level comparison system with SHA256 hashing and hex dump generation
//
// This module provides 100% byte-level accuracy validation with detailed
// diagnostic information for any mismatches.

use super::ValidationError;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use sha2::{Sha256, Digest};

/// Result of a byte-level comparison
#[derive(Debug, Clone)]
pub struct ByteComparisonResult {
    /// Whether the files match byte-for-byte
    pub matches: bool,
    
    /// SHA256 hash of the first file
    pub hash1: String,
    
    /// SHA256 hash of the second file
    pub hash2: String,
    
    /// Size of the first file in bytes
    pub size1: u64,
    
    /// Size of the second file in bytes
    pub size2: u64,
    
    /// First byte offset where files differ (if they differ)
    pub first_diff_offset: Option<usize>,
    
    /// Path to hex dump file (if generated)
    pub hex_dump_path: Option<String>,
    
    /// Detailed diagnostic message
    pub diagnostic: String,
}

impl ByteComparisonResult {
    /// Create a successful comparison result
    pub fn success(hash: String, size: u64) -> Self {
        Self {
            matches: true,
            hash1: hash.clone(),
            hash2: hash,
            size1: size,
            size2: size,
            first_diff_offset: None,
            hex_dump_path: None,
            diagnostic: "Files match byte-for-byte".to_string(),
        }
    }

    /// Create a failed comparison result
    pub fn failure(
        hash1: String,
        hash2: String,
        size1: u64,
        size2: u64,
        first_diff_offset: Option<usize>,
        diagnostic: String,
    ) -> Self {
        Self {
            matches: false,
            hash1,
            hash2,
            size1,
            size2,
            first_diff_offset,
            hex_dump_path: None,
            diagnostic,
        }
    }
}

/// Byte-level comparison utilities
pub struct ByteComparison;

impl ByteComparison {
    /// Compare two files byte-by-byte with detailed diagnostics
    ///
    /// This performs:
    /// - SHA256 hash comparison
    /// - Byte-by-byte comparison with offset tracking
    /// - Hex dump generation for mismatches
    ///
    /// # Arguments
    /// * `file1` - Path to the first file
    /// * `file2` - Path to the second file
    /// * `generate_hex_dump` - Whether to generate hex dump on mismatch
    ///
    /// # Returns
    /// Detailed comparison result with diagnostic information
    pub fn compare_files(
        file1: &Path,
        file2: &Path,
        generate_hex_dump: bool,
    ) -> Result<ByteComparisonResult, ValidationError> {
        // Calculate SHA256 hashes
        let hash1 = Self::calculate_sha256(file1)?;
        let hash2 = Self::calculate_sha256(file2)?;

        // Get file sizes
        let size1 = std::fs::metadata(file1)?.len();
        let size2 = std::fs::metadata(file2)?.len();

        // Quick check: if hashes match, files are identical
        if hash1 == hash2 {
            return Ok(ByteComparisonResult::success(hash1, size1));
        }

        // Hashes don't match - find first difference
        let first_diff = Self::find_first_difference(file1, file2)?;

        let diagnostic = if size1 != size2 {
            format!(
                "File size mismatch: {} bytes vs {} bytes (difference: {} bytes)",
                size1,
                size2,
                (size1 as i64 - size2 as i64).abs()
            )
        } else if let Some(offset) = first_diff {
            format!(
                "Files differ at byte offset 0x{:08X} ({} bytes from start)",
                offset, offset
            )
        } else {
            "Files differ but no specific offset found".to_string()
        };

        let mut result = ByteComparisonResult::failure(
            hash1,
            hash2,
            size1,
            size2,
            first_diff,
            diagnostic,
        );

        // Generate hex dump if requested and we found a difference
        if generate_hex_dump && first_diff.is_some() {
            if let Ok(hex_dump_path) = Self::generate_hex_dump(file1, file2, first_diff.unwrap()) {
                result.hex_dump_path = Some(hex_dump_path);
            }
        }

        Ok(result)
    }

    /// Calculate SHA256 hash of a file
    pub fn calculate_sha256(file_path: &Path) -> Result<String, ValidationError> {
        let mut file = File::open(file_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    /// Find the first byte offset where two files differ
    fn find_first_difference(file1: &Path, file2: &Path) -> Result<Option<usize>, ValidationError> {
        let mut f1 = File::open(file1)?;
        let mut f2 = File::open(file2)?;

        let mut buf1 = [0u8; 8192];
        let mut buf2 = [0u8; 8192];
        let mut offset = 0;

        loop {
            let bytes1 = f1.read(&mut buf1)?;
            let bytes2 = f2.read(&mut buf2)?;

            if bytes1 == 0 && bytes2 == 0 {
                // End of both files
                return Ok(None);
            }

            let min_bytes = bytes1.min(bytes2);
            for i in 0..min_bytes {
                if buf1[i] != buf2[i] {
                    return Ok(Some(offset + i));
                }
            }

            // If one file is longer, that's where they differ
            if bytes1 != bytes2 {
                return Ok(Some(offset + min_bytes));
            }

            offset += bytes1;
        }
    }

    /// Generate a hex dump showing the difference between two files
    ///
    /// Creates a side-by-side hex dump showing the context around the first difference
    fn generate_hex_dump(
        file1: &Path,
        file2: &Path,
        diff_offset: usize,
    ) -> Result<String, ValidationError> {
        let output_dir = std::env::temp_dir().join("casc_validation");
        std::fs::create_dir_all(&output_dir)?;

        let dump_file = output_dir.join(format!(
            "hex_dump_{}.txt",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ));

        let mut output = File::create(&dump_file)?;

        // Read context around the difference (64 bytes before and after)
        let context_size = 64;
        let start_offset = diff_offset.saturating_sub(context_size);
        let end_offset = diff_offset + context_size;

        let data1 = Self::read_range(file1, start_offset, end_offset)?;
        let data2 = Self::read_range(file2, start_offset, end_offset)?;

        writeln!(output, "Byte-level comparison hex dump")?;
        writeln!(output, "================================")?;
        writeln!(output, "File 1: {:?}", file1)?;
        writeln!(output, "File 2: {:?}", file2)?;
        writeln!(output, "First difference at offset: 0x{:08X}", diff_offset)?;
        writeln!(output, "Context: {} bytes before and after", context_size)?;
        writeln!(output)?;

        writeln!(output, "Offset    | File 1 Hex                                      | File 2 Hex                                      | ASCII")?;
        writeln!(output, "----------|------------------------------------------------|------------------------------------------------|------")?;

        let max_len = data1.len().max(data2.len());
        for chunk_start in (0..max_len).step_by(16) {
            let offset = start_offset + chunk_start;
            write!(output, "0x{:08X} | ", offset)?;

            // File 1 hex
            for i in 0..16 {
                let idx = chunk_start + i;
                if idx < data1.len() {
                    write!(output, "{:02X} ", data1[idx])?;
                } else {
                    write!(output, "   ")?;
                }
            }

            write!(output, "| ")?;

            // File 2 hex
            for i in 0..16 {
                let idx = chunk_start + i;
                if idx < data2.len() {
                    write!(output, "{:02X} ", data2[idx])?;
                } else {
                    write!(output, "   ")?;
                }
            }

            write!(output, "| ")?;

            // ASCII representation
            for i in 0..16 {
                let idx = chunk_start + i;
                if idx < data1.len() {
                    let c = data1[idx];
                    if c >= 32 && c <= 126 {
                        write!(output, "{}", c as char)?;
                    } else {
                        write!(output, ".")?;
                    }
                } else {
                    write!(output, " ")?;
                }
            }

            writeln!(output)?;

            // Highlight the difference
            if offset <= diff_offset && diff_offset < offset + 16 {
                let diff_col = diff_offset - offset;
                write!(output, "          | ")?;
                for i in 0..16 {
                    if i == diff_col {
                        write!(output, "^^ ")?;
                    } else {
                        write!(output, "   ")?;
                    }
                }
                writeln!(output, "| <-- FIRST DIFFERENCE")?;
            }
        }

        Ok(dump_file.to_string_lossy().to_string())
    }

    /// Read a range of bytes from a file
    fn read_range(file: &Path, start: usize, end: usize) -> Result<Vec<u8>, ValidationError> {
        use std::io::Seek;

        let mut f = File::open(file)?;
        f.seek(std::io::SeekFrom::Start(start as u64))?;

        let mut buffer = vec![0u8; end - start];
        let bytes_read = f.read(&mut buffer)?;
        buffer.truncate(bytes_read);

        Ok(buffer)
    }

    /// Generate a detailed byte-level report comparing two files
    pub fn generate_report(
        file1: &Path,
        file2: &Path,
        result: &ByteComparisonResult,
    ) -> String {
        let mut report = String::new();

        report.push_str("=== Byte-Level Comparison Report ===\n\n");
        report.push_str(&format!("File 1: {:?}\n", file1));
        report.push_str(&format!("File 2: {:?}\n\n", file2));

        report.push_str(&format!("SHA256 Hash 1: {}\n", result.hash1));
        report.push_str(&format!("SHA256 Hash 2: {}\n\n", result.hash2));

        report.push_str(&format!("Size 1: {} bytes\n", result.size1));
        report.push_str(&format!("Size 2: {} bytes\n\n", result.size2));

        if result.matches {
            report.push_str("✅ FILES MATCH BYTE-FOR-BYTE\n");
        } else {
            report.push_str("❌ FILES DIFFER\n\n");
            report.push_str(&format!("Diagnostic: {}\n", result.diagnostic));

            if let Some(offset) = result.first_diff_offset {
                report.push_str(&format!("\nFirst difference at offset: 0x{:08X} ({} bytes)\n", offset, offset));
            }

            if let Some(hex_dump) = &result.hex_dump_path {
                report.push_str(&format!("\nHex dump generated: {}\n", hex_dump));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_sha256_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("test.bin");
        
        let mut f = File::create(&file).unwrap();
        f.write_all(b"test data").unwrap();
        drop(f);

        let hash = ByteComparison::calculate_sha256(&file).unwrap();
        assert_eq!(hash.len(), 64); // SHA256 produces 64 hex characters
    }

    #[test]
    fn test_identical_files() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.bin");
        let file2 = temp_dir.path().join("file2.bin");

        let data = b"identical data";
        File::create(&file1).unwrap().write_all(data).unwrap();
        File::create(&file2).unwrap().write_all(data).unwrap();

        let result = ByteComparison::compare_files(&file1, &file2, false).unwrap();
        assert!(result.matches);
        assert_eq!(result.hash1, result.hash2);
    }

    #[test]
    fn test_different_files() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.bin");
        let file2 = temp_dir.path().join("file2.bin");

        File::create(&file1).unwrap().write_all(b"data one").unwrap();
        File::create(&file2).unwrap().write_all(b"data two").unwrap();

        let result = ByteComparison::compare_files(&file1, &file2, false).unwrap();
        assert!(!result.matches);
        assert_ne!(result.hash1, result.hash2);
        assert!(result.first_diff_offset.is_some());
    }

    #[test]
    fn test_find_first_difference() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.bin");
        let file2 = temp_dir.path().join("file2.bin");

        File::create(&file1).unwrap().write_all(b"abcdefgh").unwrap();
        File::create(&file2).unwrap().write_all(b"abcdXfgh").unwrap();

        let diff = ByteComparison::find_first_difference(&file1, &file2).unwrap();
        assert_eq!(diff, Some(4)); // 'e' vs 'X' at offset 4
    }
}
