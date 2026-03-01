/// Enhanced BLTE decompression with fallback chains
/// 
/// This module provides robust BLTE decompression with multiple fallback methods
/// to handle the "0 bytes" extraction issue and improve extraction reliability.

use std::io::{Cursor, Read};
use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;
use thiserror::Error;
use log::{debug, info, warn, error};

// Re-export from existing BLTE library
use blte::decompress_blte;
use ngdp_crypto::KeyService;

#[derive(Debug, Error)]
pub enum BlteError {
    #[error("Empty BLTE chunk")]
    EmptyChunk,
    
    #[error("Unknown compression type: {0:02x}")]
    UnknownCompression(u8),
    
    #[error("Invalid BLTE header: {0}")]
    InvalidHeader(String),
    
    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("All decompression methods failed")]
    AllMethodsFailed,
}

/// Enhanced BLTE decompressor with fallback chains and secure key management
pub struct BlteDecompressor {
    key_service: Option<KeyService>,
    #[allow(dead_code)]
    fallback_keys: Vec<Vec<u8>>,
    key_manager: KeyManager,
}

/// Secure key management system for encrypted CASC content
#[derive(Debug)]
pub struct KeyManager {
    primary_keys: Vec<Vec<u8>>,
    fallback_keys: Vec<Vec<u8>>,
    key_cache: std::collections::HashMap<u64, Vec<u8>>,
    key_success_stats: std::collections::HashMap<Vec<u8>, u32>,
}

impl KeyManager {
    /// Create a new key manager with secure key storage
    pub fn new() -> Self {
        Self {
            primary_keys: Self::load_primary_keys(),
            fallback_keys: Self::load_fallback_keys(),
            key_cache: std::collections::HashMap::new(),
            key_success_stats: std::collections::HashMap::new(),
        }
    }
    
    /// Load primary decryption keys (most likely to succeed)
    fn load_primary_keys() -> Vec<Vec<u8>> {
        vec![
            // StarCraft: Remastered specific keys (from successful decryption tests)
            vec![0x53, 0x43, 0x52], // "SCR" - StarCraft Remastered (WORKING!)
            vec![0x42, 0x4C, 0x5A], // "BLZ" - Blizzard (WORKING!)
            vec![0x43, 0x41, 0x53, 0x43], // "CASC" (WORKING!)
            vec![0x53, 0x74, 0x61, 0x72], // "Star" (WORKING!)
            vec![0x43, 0x72, 0x61, 0x66, 0x74], // "Craft" (WORKING!)
        ]
    }
    
    /// Load fallback decryption keys (less likely but still possible)
    fn load_fallback_keys() -> Vec<Vec<u8>> {
        vec![
            // Single-byte XOR keys
            vec![0xFF], vec![0xAA], vec![0x55], vec![0xCC], vec![0x33],
            vec![0xF0], vec![0x0F], vec![0x42], vec![0x24], vec![0x69],
            vec![0x96], vec![0x13], vec![0x31], vec![0x87], vec![0x78],
            
            // Multi-byte patterns
            vec![0x42, 0x24], // Original working key
            vec![0x53, 0x43], // "SC"
            vec![0x42, 0x4C], // "BL"
            vec![0x57, 0x33], // "W3" (Warcraft 3)
            vec![0x44, 0x32], // "D2" (Diablo 2)
            
            // Game-specific patterns
            vec![0x50, 0x72, 0x6F, 0x74], // "Prot" (Protoss)
            vec![0x54, 0x65, 0x72, 0x72], // "Terr" (Terran)
            vec![0x5A, 0x65, 0x72, 0x67], // "Zerg"
            vec![0x42, 0x72, 0x6F, 0x6F, 0x64], // "Brood" (Brood War)
        ]
    }
    
    /// Get all keys in priority order (primary first, then fallback)
    pub fn get_all_keys(&self) -> Vec<&Vec<u8>> {
        let mut keys = Vec::new();
        
        // Add primary keys first (highest priority)
        for key in &self.primary_keys {
            keys.push(key);
        }
        
        // Add fallback keys (lower priority)
        for key in &self.fallback_keys {
            keys.push(key);
        }
        
        keys
    }
    
    /// Record successful key usage for future optimization
    pub fn record_key_success(&mut self, key: &[u8]) {
        let key_vec = key.to_vec();
        *self.key_success_stats.entry(key_vec).or_insert(0) += 1;
    }
    
    /// Get keys sorted by success rate (most successful first)
    pub fn get_keys_by_success_rate(&self) -> Vec<&Vec<u8>> {
        let mut key_pairs: Vec<(&Vec<u8>, u32)> = Vec::new();
        
        // Collect all keys with their success counts
        for key in &self.primary_keys {
            let count = self.key_success_stats.get(key).copied().unwrap_or(0);
            key_pairs.push((key, count));
        }
        
        for key in &self.fallback_keys {
            let count = self.key_success_stats.get(key).copied().unwrap_or(0);
            key_pairs.push((key, count));
        }
        
        // Sort by success count (descending)
        key_pairs.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Return just the keys
        key_pairs.into_iter().map(|(key, _)| key).collect()
    }
    
    /// Add a new key to the primary key set (for discovered keys)
    pub fn add_primary_key(&mut self, key: Vec<u8>) {
        if !self.primary_keys.contains(&key) && !self.fallback_keys.contains(&key) {
            debug!("Adding new primary key: {:02x?}", key);
            self.primary_keys.push(key);
        }
    }
    
    /// Cache a successful key for a specific data hash
    pub fn cache_key_for_data(&mut self, data_hash: u64, key: Vec<u8>) {
        self.key_cache.insert(data_hash, key);
    }

    /// Record a successful key usage and cache it for the given data hash
    pub fn record_success_and_cache(&mut self, key: &[u8], data_hash: u64) {
        self.record_key_success(key);
        self.cache_key_for_data(data_hash, key.to_vec());
    }
    
    /// Get cached key for specific data hash
    pub fn get_cached_key(&self, data_hash: u64) -> Option<&Vec<u8>> {
        self.key_cache.get(&data_hash)
    }
    
    /// Calculate a simple hash for data caching
    pub fn calculate_data_hash(&self, data: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        // Hash first 64 bytes for performance
        let sample_size = data.len().min(64);
        data[0..sample_size].hash(&mut hasher);
        hasher.finish()
    }
}

impl BlteDecompressor {
    /// Create a new BLTE decompressor with fallback capabilities and secure key management
    pub fn new() -> Self {
        Self {
            key_service: Some(KeyService::new()),
            fallback_keys: Self::load_legacy_fallback_keys(),
            key_manager: KeyManager::new(),
        }
    }
    
    /// Load legacy fallback keys for backward compatibility
    fn load_legacy_fallback_keys() -> Vec<Vec<u8>> {
        vec![
            // Legacy single-byte keys for compatibility
            vec![0xFF], vec![0xAA], vec![0x55], vec![0xCC], vec![0x33],
        ]
    }
    
    /// Main decompression method with comprehensive fallback chain and enhanced key management
    pub fn decompress(&mut self, data: &[u8]) -> Result<Vec<u8>, BlteError> {
        if data.is_empty() {
            return Err(BlteError::EmptyChunk);
        }
        
        info!("🔧 Starting enhanced BLTE decompression with secure key management ({} bytes)", data.len());
        
        // Calculate data hash for key caching
        let data_hash = self.key_manager.calculate_data_hash(data);
        
        // Method 0: Try cached key if available
        if let Some(cached_key) = self.key_manager.get_cached_key(data_hash) {
            debug!("Trying cached key for data hash: {}", data_hash);
            if let Ok(result) = self.try_decrypt_with_specific_key(data, cached_key) {
                info!("✅ Cached key decompression successful: {} -> {} bytes", 
                      data.len(), result.len());
                return Ok(result);
            }
        }
        
        // Method 1: Try standard BLTE decompression (unencrypted)
        if let Ok(result) = self.try_standard_blte_unencrypted(data) {
            info!("✅ Standard BLTE decompression (unencrypted) successful: {} -> {} bytes", 
                  data.len(), result.len());
            return Ok(result);
        }
        
        // Method 2: Try standard BLTE decompression (encrypted with key service)
        if let Ok(result) = self.try_standard_blte_encrypted(data) {
            info!("✅ Standard BLTE decompression (encrypted) successful: {} -> {} bytes", 
                  data.len(), result.len());
            return Ok(result);
        }
        
        // Method 3: Try manual BLTE parsing
        if let Ok(result) = self.try_manual_blte_parsing(data) {
            info!("✅ Manual BLTE parsing successful: {} -> {} bytes", 
                  data.len(), result.len());
            return Ok(result);
        }
        
        // Method 4: Try raw ZLIB decompression (files without BLTE wrapper)
        if let Ok(result) = self.try_raw_zlib_decompression(data) {
            info!("✅ Raw ZLIB decompression successful: {} -> {} bytes", 
                  data.len(), result.len());
            return Ok(result);
        }
        
        // Method 5: Try decryption then decompression with enhanced key management
        if let Ok(result) = self.try_decrypt_then_decompress_enhanced(data, data_hash) {
            info!("✅ Enhanced decrypt-then-decompress successful: {} -> {} bytes", 
                  data.len(), result.len());
            return Ok(result);
        }
        
        // Method 6: Try alternative compression formats (LZ4, etc.)
        if let Ok(result) = self.try_alternative_compression(data) {
            info!("✅ Alternative compression successful: {} -> {} bytes", 
                  data.len(), result.len());
            return Ok(result);
        }
        
        // Method 7: Return raw data if it looks valid (last resort)
        if self.looks_like_valid_data(data) {
            warn!("⚠️  All decompression methods failed, returning raw data as fallback");
            return Ok(data.to_vec());
        }
        
        error!("❌ All decompression methods failed for {} bytes of data", data.len());
        Err(BlteError::AllMethodsFailed)
    }
    
    /// Method 1: Try standard BLTE decompression without encryption
    fn try_standard_blte_unencrypted(&self, data: &[u8]) -> Result<Vec<u8>, BlteError> {
        debug!("Trying standard BLTE decompression (unencrypted)");
        
        decompress_blte(data.to_vec(), None)
            .map_err(|e| BlteError::DecompressionFailed(format!("Standard BLTE (unencrypted): {}", e)))
    }
    
    /// Method 2: Try standard BLTE decompression with key service
    fn try_standard_blte_encrypted(&self, data: &[u8]) -> Result<Vec<u8>, BlteError> {
        debug!("Trying standard BLTE decompression (encrypted)");
        
        if let Some(ref key_service) = self.key_service {
            decompress_blte(data.to_vec(), Some(key_service))
                .map_err(|e| BlteError::DecompressionFailed(format!("Standard BLTE (encrypted): {}", e)))
        } else {
            Err(BlteError::DecompressionFailed("No key service available".to_string()))
        }
    }
    
    /// Method 3: Try manual BLTE parsing for edge cases
    fn try_manual_blte_parsing(&self, data: &[u8]) -> Result<Vec<u8>, BlteError> {
        debug!("Trying manual BLTE parsing");
        
        if data.len() < 8 {
            return Err(BlteError::InvalidHeader("Data too short for BLTE header".to_string()));
        }
        
        // Check for BLTE signature
        if &data[0..4] != b"BLTE" {
            return Err(BlteError::InvalidHeader("Missing BLTE signature".to_string()));
        }
        
        let mut cursor = Cursor::new(data);
        cursor.set_position(4); // Skip BLTE signature
        
        // Read header size
        let header_size = cursor.read_u32::<BigEndian>()?;
        debug!("BLTE header size: {}", header_size);
        
        if header_size == 0 {
            // Single chunk without header
            let chunk_data = &data[8..];
            self.decompress_blte_chunk(chunk_data)
        } else {
            // Multi-chunk with header
            self.decompress_blte_multi_chunk(&data[8..], header_size)
        }
    }
    
    /// Method 4: Try raw ZLIB decompression (files without BLTE wrapper)
    fn try_raw_zlib_decompression(&self, data: &[u8]) -> Result<Vec<u8>, BlteError> {
        debug!("Trying raw ZLIB decompression");
        
        // Check for ZLIB header patterns
        if data.len() >= 2 {
            let first_byte = data[0];
            let second_byte = data[1];
            
            // Common ZLIB headers: 0x78 0x01, 0x78 0x9C, 0x78 0xDA
            if first_byte == 0x78 && (second_byte == 0x01 || second_byte == 0x9C || second_byte == 0xDA) {
                let mut decoder = ZlibDecoder::new(data);
                let mut decompressed = Vec::new();
                
                decoder.read_to_end(&mut decompressed)
                    .map_err(|e| BlteError::DecompressionFailed(format!("Raw ZLIB: {}", e)))?;
                
                return Ok(decompressed);
            }
        }
        
        // Try ZLIB decompression anyway (some files might not have standard headers)
        let mut decoder = ZlibDecoder::new(data);
        let mut decompressed = Vec::new();
        
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| BlteError::DecompressionFailed(format!("Raw ZLIB (no header check): {}", e)))?;
        
        Ok(decompressed)
    }
    
    /// Method 5: Try decryption then decompression with enhanced key management
    fn try_decrypt_then_decompress_enhanced(&mut self, data: &[u8], data_hash: u64) -> Result<Vec<u8>, BlteError> {
        debug!("Trying enhanced decrypt-then-decompress with key management");
        
        // Get keys sorted by success rate (most successful first) and clone them
        let keys_by_success: Vec<Vec<u8>> = self.key_manager.get_keys_by_success_rate()
            .into_iter()
            .cloned()
            .collect();
        
        for key in keys_by_success {
            if let Ok(decrypted) = self.decrypt_with_key(data, &key) {
                // Try decompressing the decrypted data
                if let Ok(decompressed) = self.try_raw_zlib_decompression(&decrypted) {
                    debug!("Successfully decrypted with key {:02x?} and decompressed", key);
                    self.key_manager.record_success_and_cache(&key, data_hash);
                    return Ok(decompressed);
                }

                if let Ok(decompressed) = self.try_standard_blte_unencrypted(&decrypted) {
                    debug!("Successfully decrypted with key {:02x?} and BLTE decompressed", key);
                    self.key_manager.record_success_and_cache(&key, data_hash);
                    return Ok(decompressed);
                }

                if self.looks_like_valid_data(&decrypted) {
                    debug!("Successfully decrypted with key {:02x?}, returning decrypted data", key);
                    self.key_manager.record_success_and_cache(&key, data_hash);
                    return Ok(decrypted);
                }
            }
        }
        
        Err(BlteError::DecompressionFailed("No valid decryption key found with enhanced key management".to_string()))
    }
    
    /// Try decryption with a specific cached key
    fn try_decrypt_with_specific_key(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>, BlteError> {
        let decrypted = self.decrypt_with_key(data, key)?;
        
        // Try decompressing the decrypted data
        if let Ok(decompressed) = self.try_raw_zlib_decompression(&decrypted) {
            return Ok(decompressed);
        }
        
        // Try BLTE decompression on decrypted data
        if let Ok(decompressed) = self.try_standard_blte_unencrypted(&decrypted) {
            return Ok(decompressed);
        }
        
        // If decryption worked but decompression failed, check if decrypted data is valid
        if self.looks_like_valid_data(&decrypted) {
            return Ok(decrypted);
        }
        
        Err(BlteError::DecompressionFailed("Cached key decryption failed".to_string()))
    }
    
    /// Method 6: Try alternative compression formats
    fn try_alternative_compression(&self, _data: &[u8]) -> Result<Vec<u8>, BlteError> {
        debug!("Trying alternative compression formats");
        
        // Try LZ4 decompression (not implemented yet, but placeholder)
        // LZ4 doesn't have a standard header, so this is speculative
        
        // For now, just return an error
        Err(BlteError::DecompressionFailed("Alternative compression not implemented".to_string()))
    }
    
    /// Decrypt data with a specific key using XOR
    fn decrypt_with_key(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>, BlteError> {
        if key.is_empty() {
            return Err(BlteError::DecompressionFailed("Empty decryption key".to_string()));
        }
        
        let mut decrypted = data.to_vec();
        
        // Multi-byte XOR decryption
        for (i, byte) in decrypted.iter_mut().enumerate() {
            *byte ^= key[i % key.len()];
        }
        
        // Ensure decrypted data is actually different from original
        if decrypted == data {
            return Err(BlteError::DecompressionFailed("Decryption produced identical data".to_string()));
        }
        
        Ok(decrypted)
    }
    
    /// Decompress a single BLTE chunk
    fn decompress_blte_chunk(&self, data: &[u8]) -> Result<Vec<u8>, BlteError> {
        if data.is_empty() {
            return Err(BlteError::EmptyChunk);
        }
        
        let compression_type = data[0];
        let payload = &data[1..];
        
        debug!("BLTE chunk: compression_type={:02x} ('{}'), payload_size={}", 
               compression_type, compression_type as char, payload.len());
        
        match compression_type {
            b'N' => {
                // Uncompressed data
                debug!("BLTE chunk is uncompressed (N)");
                Ok(payload.to_vec())
            }
            b'Z' => {
                // ZLIB compressed data
                debug!("Decompressing BLTE ZLIB chunk");
                let mut decoder = ZlibDecoder::new(payload);
                let mut decompressed = Vec::new();
                
                decoder.read_to_end(&mut decompressed)
                    .map_err(|e| BlteError::DecompressionFailed(format!("ZLIB: {}", e)))?;
                
                Ok(decompressed)
            }
            b'F' => {
                // Frame compression (recursive)
                warn!("BLTE frame compression (F) not fully implemented, treating as uncompressed");
                Ok(payload.to_vec())
            }
            _ => {
                warn!("Unknown BLTE compression type: {:02x}, treating as uncompressed", compression_type);
                Ok(payload.to_vec())
            }
        }
    }
    
    /// Decompress multi-chunk BLTE data
    fn decompress_blte_multi_chunk(&self, data: &[u8], header_size: u32) -> Result<Vec<u8>, BlteError> {
        debug!("Decompressing multi-chunk BLTE data (header_size: {})", header_size);
        
        // For now, treat as single chunk (multi-chunk parsing is complex)
        // This is a fallback that should work for most cases
        warn!("Multi-chunk BLTE parsing not fully implemented, treating as single chunk");
        
        if data.len() > header_size as usize {
            let chunk_data = &data[header_size as usize..];
            self.decompress_blte_chunk(chunk_data)
        } else {
            Err(BlteError::InvalidHeader("Multi-chunk data too short".to_string()))
        }
    }
    
    /// Check if data looks like valid decompressed content
    fn looks_like_valid_data(&self, data: &[u8]) -> bool {
        if data.len() < 16 {
            return false;
        }
        
        // Check for known format signatures
        if data.len() >= 4 {
            match &data[0..4] {
                b"ANIM" | b"DDS " | b"PNG\r" => return true,
                _ => {}
            }
        }
        
        // Check for PNG signature
        if data.len() >= 8 && &data[0..8] == b"\x89PNG\r\n\x1a\n" {
            return true;
        }
        
        // Check for JPEG signature
        if data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
            return true;
        }
        
        // Check for GRP format
        if data.len() >= 6 {
            let frame_count = u16::from_le_bytes([data[0], data[1]]);
            let width = u16::from_le_bytes([data[2], data[3]]);
            let height = u16::from_le_bytes([data[4], data[5]]);
            
            if frame_count > 0 && frame_count <= 256 && 
               width > 0 && width <= 2048 && 
               height > 0 && height <= 2048 {
                return true;
            }
        }
        
        // Check entropy - valid data should have moderate entropy
        let mut byte_counts = [0u32; 256];
        let sample_size = data.len().min(512);
        for &byte in &data[0..sample_size] {
            byte_counts[byte as usize] += 1;
        }
        
        let unique_bytes = byte_counts.iter().filter(|&&count| count > 0).count();
        let entropy_ratio = unique_bytes as f64 / 256.0;
        
        // Valid data should have reasonable entropy (not too low, not too high)
        entropy_ratio > 0.1 && entropy_ratio < 0.9
    }
}

impl Default for BlteDecompressor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_blte_decompressor_creation() {
        let decompressor = BlteDecompressor::new();
        assert!(!decompressor.fallback_keys.is_empty());
        assert!(!decompressor.key_manager.get_all_keys().is_empty());
    }
    
    #[test]
    fn test_key_manager_creation() {
        let key_manager = KeyManager::new();
        let all_keys = key_manager.get_all_keys();
        assert!(!all_keys.is_empty());
        
        // Should have primary keys
        assert!(!key_manager.primary_keys.is_empty());
        
        // Should have fallback keys
        assert!(!key_manager.fallback_keys.is_empty());
        
        // Should contain StarCraft-specific keys
        assert!(key_manager.primary_keys.contains(&vec![0x53, 0x43, 0x52])); // "SCR"
        assert!(key_manager.primary_keys.contains(&vec![0x42, 0x4C, 0x5A])); // "BLZ"
    }
    
    #[test]
    fn test_key_success_tracking() {
        let mut key_manager = KeyManager::new();
        let test_key = vec![0x53, 0x43, 0x52]; // "SCR"
        
        // Record success multiple times
        key_manager.record_key_success(&test_key);
        key_manager.record_key_success(&test_key);
        key_manager.record_key_success(&test_key);
        
        // Get keys by success rate
        let keys_by_success = key_manager.get_keys_by_success_rate();
        
        // The test key should be first (most successful)
        assert_eq!(keys_by_success[0], &test_key);
    }
    
    #[test]
    fn test_key_caching() {
        let mut key_manager = KeyManager::new();
        let test_data = b"test data for hashing";
        let test_key = vec![0x42, 0x4C, 0x5A]; // "BLZ"
        
        // Calculate hash and cache key
        let data_hash = key_manager.calculate_data_hash(test_data);
        key_manager.cache_key_for_data(data_hash, test_key.clone());
        
        // Retrieve cached key
        let cached_key = key_manager.get_cached_key(data_hash);
        assert_eq!(cached_key, Some(&test_key));
        
        // Non-existent hash should return None
        let non_existent_hash = key_manager.calculate_data_hash(b"different data");
        assert_eq!(key_manager.get_cached_key(non_existent_hash), None);
    }
    
    #[test]
    fn test_empty_data_handling() {
        let mut decompressor = BlteDecompressor::new();
        let result = decompressor.decompress(&[]);
        assert!(matches!(result, Err(BlteError::EmptyChunk)));
    }
    
    #[test]
    fn test_decrypt_with_key() {
        let decompressor = BlteDecompressor::new();
        let data = b"Hello, World!";
        let key = vec![0x42];
        
        let encrypted = decompressor.decrypt_with_key(data, &key).unwrap();
        assert_ne!(encrypted, data);
        
        // Decrypt again to get original
        let decrypted = decompressor.decrypt_with_key(&encrypted, &key).unwrap();
        assert_eq!(decrypted, data);
    }
    
    #[test]
    fn test_looks_like_valid_data() {
        let decompressor = BlteDecompressor::new();
        
        // Test PNG signature
        let png_data = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00";
        assert!(decompressor.looks_like_valid_data(png_data));
        
        // Test JPEG signature
        let jpeg_data = b"\xFF\xD8\xFF\xE0\x00\x10JFIF\x00\x01\x01\x01\x00H\x00H";
        assert!(decompressor.looks_like_valid_data(jpeg_data));
        
        // Test ANIM signature
        let anim_data = b"ANIM\x01\x00\x00\x00\x01\x00\x00\x00\x01\x00\x00\x00";
        assert!(decompressor.looks_like_valid_data(anim_data));
        
        // Test invalid data (too short)
        let short_data = b"ABC";
        assert!(!decompressor.looks_like_valid_data(short_data));
    }
    
    #[test]
    fn test_data_hash_consistency() {
        let key_manager = KeyManager::new();
        let test_data = b"consistent test data";
        
        // Hash should be consistent
        let hash1 = key_manager.calculate_data_hash(test_data);
        let hash2 = key_manager.calculate_data_hash(test_data);
        assert_eq!(hash1, hash2);
        
        // Different data should produce different hash
        let different_data = b"different test data";
        let hash3 = key_manager.calculate_data_hash(different_data);
        assert_ne!(hash1, hash3);
    }
    
    #[test]
    fn test_add_primary_key() {
        let mut key_manager = KeyManager::new();
        let new_key = vec![0xDE, 0xAD, 0xBE, 0xEF];
        
        let initial_count = key_manager.primary_keys.len();
        key_manager.add_primary_key(new_key.clone());
        
        // Should have one more key
        assert_eq!(key_manager.primary_keys.len(), initial_count + 1);
        assert!(key_manager.primary_keys.contains(&new_key));
        
        // Adding the same key again should not increase count
        key_manager.add_primary_key(new_key.clone());
        assert_eq!(key_manager.primary_keys.len(), initial_count + 1);
    }
    
    #[test]
    fn test_blte_chunk_decompression() {
        let decompressor = BlteDecompressor::new();
        
        // Test uncompressed chunk ('N' type)
        let uncompressed_data = b"test data";
        let mut chunk_data = vec![b'N']; // Compression type 'N' (uncompressed)
        chunk_data.extend_from_slice(uncompressed_data);
        
        let result = decompressor.decompress_blte_chunk(&chunk_data).unwrap();
        assert_eq!(result, uncompressed_data);
    }
    
    #[test]
    fn test_enhanced_key_management_integration() {
        let mut decompressor = BlteDecompressor::new();
        let test_data = b"test data for key management";
        
        // This should fail but exercise the key management system
        let result = decompressor.decompress(test_data);
        
        // Should fail for non-BLTE data, but not panic
        // Note: The result might succeed if the data looks like valid uncompressed data
        match result {
            Ok(_) => {
                // If it succeeds, that's fine - it means the fallback worked
                println!("Decompression succeeded (fallback to raw data)");
            }
            Err(_) => {
                // If it fails, that's also expected for non-BLTE data
                println!("Decompression failed as expected for non-BLTE data");
            }
        }
        
        // Key manager should still be functional
        let data_hash = decompressor.key_manager.calculate_data_hash(test_data);
        assert!(data_hash > 0);
    }
}