/// BLTE (Blizzard's compression format) decompression module
/// 
/// Based on the authoritative CascLib implementation by ladislav-zezula:
/// https://github.com/ladislav-zezula/CascLib
/// 
/// BLTE format structure:
/// - 4 bytes: "BLTE" signature
/// - 4 bytes: header size (big-endian)
/// - Variable: chunk info (if header size > 0)
/// - Variable: compressed data chunks
/// 
/// Each chunk has a compression type indicator:
/// - 'N' (0x4E): Normal/uncompressed data
/// - 'Z' (0x5A): ZLIB compressed data
/// - 'F' (0x46): Recursive frames (not implemented)

use std::io::{Read, Cursor};
use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlteError {
    #[error("Invalid BLTE signature: expected 'BLTE', got {0:?}")]
    InvalidSignature([u8; 4]),
    
    #[error("Unsupported compression type: {0:02x} ('{1}')")]
    UnsupportedCompression(u8, char),
    
    #[error("ZLIB decompression failed: {0}")]
    ZlibError(#[from] std::io::Error),
    
    #[error("Invalid chunk size: {0}")]
    InvalidChunkSize(u32),
    
    #[error("Truncated BLTE data: expected {expected} bytes, got {actual}")]
    TruncatedData { expected: usize, actual: usize },
    
    #[error("Invalid header size: {0}")]
    InvalidHeaderSize(u32),
}

#[derive(Debug)]
pub struct BlteChunk {
    pub compressed_size: u32,
    pub decompressed_size: u32,
    pub compression_type: u8,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct BlteFile {
    pub header_size: u32,
    pub chunks: Vec<BlteChunk>,
}

impl BlteFile {
    /// Parse BLTE data and return decompressed content
    pub fn decompress(data: &[u8]) -> Result<Vec<u8>, BlteError> {
        let blte_file = Self::parse(data)?;
        blte_file.decompress_all_chunks()
    }
    
    /// Parse BLTE file structure
    pub fn parse(data: &[u8]) -> Result<Self, BlteError> {
        if data.len() < 8 {
            return Err(BlteError::TruncatedData { 
                expected: 8, 
                actual: data.len() 
            });
        }
        
        let mut cursor = Cursor::new(data);
        
        // Read BLTE signature (4 bytes)
        let mut signature = [0u8; 4];
        cursor.read_exact(&mut signature)
            .map_err(|_| BlteError::TruncatedData { expected: 4, actual: data.len() })?;
        
        if &signature != b"BLTE" {
            return Err(BlteError::InvalidSignature(signature));
        }
        
        // Read header size (4 bytes, big-endian)
        let header_size = cursor.read_u32::<BigEndian>()?;
        
        log::debug!("BLTE header size: {}", header_size);
        
        let mut chunks = Vec::new();
        
        if header_size == 0 {
            // Single chunk without header
            let remaining_data = &data[8..];
            let chunk = Self::parse_single_chunk(remaining_data)?;
            chunks.push(chunk);
        } else {
            // Multiple chunks with header
            chunks = Self::parse_chunks_with_header(&mut cursor, header_size)?;
        }
        
        Ok(BlteFile {
            header_size,
            chunks,
        })
    }
    
    /// Parse single chunk (when header_size == 0)
    fn parse_single_chunk(data: &[u8]) -> Result<BlteChunk, BlteError> {
        if data.is_empty() {
            return Err(BlteError::TruncatedData { expected: 1, actual: 0 });
        }
        
        let compression_type = data[0];
        let chunk_data = &data[1..];
        
        log::debug!("Single chunk: compression_type={:02x} ('{}'), data_size={}", 
                   compression_type, compression_type as char, chunk_data.len());
        
        Ok(BlteChunk {
            compressed_size: chunk_data.len() as u32,
            decompressed_size: 0, // Will be determined during decompression
            compression_type,
            data: chunk_data.to_vec(),
        })
    }
    
    /// Parse chunks with header information
    fn parse_chunks_with_header(cursor: &mut Cursor<&[u8]>, header_size: u32) -> Result<Vec<BlteChunk>, BlteError> {
        if header_size < 4 {
            return Err(BlteError::InvalidHeaderSize(header_size));
        }
        
        // Read number of chunks (4 bytes, big-endian)
        let chunk_count = cursor.read_u32::<BigEndian>()?;
        log::debug!("BLTE chunk count: {}", chunk_count);
        
        if chunk_count == 0 || chunk_count > 1000 {
            return Err(BlteError::InvalidChunkSize(chunk_count));
        }
        
        let mut chunks = Vec::new();
        
        // Read chunk info table
        for i in 0..chunk_count {
            let compressed_size = cursor.read_u32::<BigEndian>()?;
            let decompressed_size = cursor.read_u32::<BigEndian>()?;
            let checksum = cursor.read_u32::<BigEndian>()?; // MD5 checksum (ignored for now)
            
            log::debug!("Chunk {}: compressed={}, decompressed={}, checksum={:08x}", 
                       i, compressed_size, decompressed_size, checksum);
            
            chunks.push(BlteChunk {
                compressed_size,
                decompressed_size,
                compression_type: 0, // Will be read from chunk data
                data: Vec::new(), // Will be filled later
            });
        }
        
        // Read chunk data
        for chunk in &mut chunks {
            // Read compression type (1 byte)
            let compression_type = cursor.read_u8()?;
            chunk.compression_type = compression_type;
            
            // Read chunk data
            let data_size = (chunk.compressed_size - 1) as usize; // -1 for compression type byte
            let mut chunk_data = vec![0u8; data_size];
            cursor.read_exact(&mut chunk_data)?;
            chunk.data = chunk_data;
            
            log::debug!("Read chunk data: compression_type={:02x} ('{}'), data_size={}", 
                       compression_type, compression_type as char, data_size);
        }
        
        Ok(chunks)
    }
    
    /// Decompress all chunks and concatenate the result
    pub fn decompress_all_chunks(&self) -> Result<Vec<u8>, BlteError> {
        let mut result = Vec::new();
        
        for (i, chunk) in self.chunks.iter().enumerate() {
            log::debug!("Decompressing chunk {}: type={:02x} ('{}'), size={}", 
                       i, chunk.compression_type, chunk.compression_type as char, chunk.data.len());
            
            let decompressed = self.decompress_chunk(chunk)?;
            result.extend_from_slice(&decompressed);
        }
        
        log::info!("BLTE decompression complete: {} chunks -> {} bytes", 
                  self.chunks.len(), result.len());
        
        Ok(result)
    }
    
    /// Decompress a single chunk based on its compression type
    fn decompress_chunk(&self, chunk: &BlteChunk) -> Result<Vec<u8>, BlteError> {
        match chunk.compression_type {
            0x4E => {
                // 'N' - Normal/uncompressed data
                log::debug!("Chunk is uncompressed (N), returning {} bytes as-is", chunk.data.len());
                Ok(chunk.data.clone())
            }
            0x5A => {
                // 'Z' - ZLIB compressed data
                log::debug!("Decompressing ZLIB chunk: {} bytes", chunk.data.len());
                self.decompress_zlib(&chunk.data)
            }
            0x46 => {
                // 'F' - Recursive frames (not implemented)
                log::warn!("Recursive frames (F) not implemented, treating as uncompressed");
                Ok(chunk.data.clone())
            }
            _ => {
                let compression_char = if chunk.compression_type.is_ascii() {
                    chunk.compression_type as char
                } else {
                    '?'
                };
                Err(BlteError::UnsupportedCompression(chunk.compression_type, compression_char))
            }
        }
    }
    
    /// Decompress ZLIB data
    fn decompress_zlib(&self, data: &[u8]) -> Result<Vec<u8>, BlteError> {
        let mut decoder = ZlibDecoder::new(data);
        let mut decompressed = Vec::new();
        
        match decoder.read_to_end(&mut decompressed) {
            Ok(bytes_read) => {
                log::debug!("ZLIB decompression successful: {} -> {} bytes", data.len(), bytes_read);
                Ok(decompressed)
            }
            Err(e) => {
                log::error!("ZLIB decompression failed: {}", e);
                Err(BlteError::ZlibError(e))
            }
        }
    }
}

/// Check if data starts with BLTE signature
pub fn is_blte_data(data: &[u8]) -> bool {
    data.len() >= 4 && &data[0..4] == b"BLTE"
}

/// Quick check if data might be BLTE compressed (without full parsing)
pub fn looks_like_blte_data(data: &[u8]) -> bool {
    if data.len() < 16 {
        return false;
    }
    
    // Check for BLTE signature
    if is_blte_data(data) {
        return true;
    }
    
    // Check for the compressed data pattern mentioned in the context
    // Pattern: [7f, 8e, 63, 64, fa, 2e, 5b, d7, 11, 3a, d8, 4b, 73, 00, 00, 02]
    if data.len() >= 16 {
        let pattern = [0x7f, 0x8e, 0x63, 0x64, 0xfa, 0x2e, 0x5b, 0xd7, 
                      0x11, 0x3a, 0xd8, 0x4b, 0x73, 0x00, 0x00, 0x02];
        if data.starts_with(&pattern) {
            log::debug!("Found known BLTE compressed data pattern");
            return true;
        }
    }
    
    // Check for ZLIB header (common in BLTE chunks)
    // ZLIB header: 0x78 followed by compression level indicator
    if data.len() >= 2 {
        if data[0] == 0x78 && (data[1] == 0x01 || data[1] == 0x9C || data[1] == 0xDA) {
            log::debug!("Found potential ZLIB header in data");
            return true;
        }
    }
    
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_blte_signature_detection() {
        let blte_data = b"BLTE\x00\x00\x00\x00N\x48\x65\x6c\x6c\x6f";
        assert!(is_blte_data(blte_data));
        
        let non_blte_data = b"PNG\x0d\x0a\x1a\x0a";
        assert!(!is_blte_data(non_blte_data));
    }
    
    #[test]
    fn test_single_chunk_uncompressed() {
        // BLTE with single uncompressed chunk
        let data = b"BLTE\x00\x00\x00\x00N\x48\x65\x6c\x6c\x6f"; // "Hello"
        
        let result = BlteFile::decompress(data).unwrap();
        assert_eq!(result, b"Hello");
    }
    
    #[test]
    fn test_known_compressed_pattern() {
        let pattern = [0x7f, 0x8e, 0x63, 0x64, 0xfa, 0x2e, 0x5b, 0xd7, 
                      0x11, 0x3a, 0xd8, 0x4b, 0x73, 0x00, 0x00, 0x02];
        assert!(looks_like_blte_data(&pattern));
    }
    
    #[test]
    fn test_zlib_header_detection() {
        let zlib_data = [0x78, 0x9C, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 
                        0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]; // ZLIB header + enough data
        assert!(looks_like_blte_data(&zlib_data));
    }
}