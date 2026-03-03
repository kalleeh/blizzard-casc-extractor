/// Salsa20 stream cipher implementation for CASC decryption
/// Ported from CascLib's CascDecrypt.cpp
use super::salsa20::decrypt_salsa20;
use std::collections::HashMap;

/// Known encryption keys from CascLib
/// See https://wowdev.wiki/CASC for updates
const ENCRYPTION_KEYS: &[(u64, [u8; 16])] = &[
    // StarCraft II
    (0xD0CAE11366CEEA83, [0x00, 0x41, 0x61, 0x07, 0x8E, 0x5A, 0x61, 0x20, 0x32, 0x1E, 0xA5, 0xFF, 0xE4, 0xDC, 0xD1, 0x26]),
    
    // Warcraft III Reforged
    (0x6E4296823E7D561E, [0xC0, 0xBF, 0xA2, 0x94, 0x3A, 0xC3, 0xE9, 0x22, 0x86, 0xE4, 0x44, 0x3E, 0xE3, 0x56, 0x0D, 0x65]),
    (0xE04D60E31DDEBF63, [0x26, 0x3D, 0xB5, 0xC4, 0x02, 0xDA, 0x8D, 0x4D, 0x68, 0x63, 0x09, 0xCB, 0x2E, 0x32, 0x54, 0xD0]),
];

pub struct CascDecryptor {
    key_map: HashMap<u64, [u8; 16]>,
}

impl CascDecryptor {
    pub fn new() -> Self {
        let mut key_map = HashMap::new();
        for &(key_name, key) in ENCRYPTION_KEYS {
            key_map.insert(key_name, key);
        }
        Self { key_map }
    }

    /// Decrypt CASC encrypted data
    /// Format: [key_name_size][key_name][iv_size][iv][encryption_type][encrypted_data]
    pub fn decrypt(&self, input: &[u8], frame_index: u32) -> Result<Vec<u8>, String> {
        if input.is_empty() {
            return Err("Empty input".to_string());
        }

        let mut offset = 0;

        // Read key name size
        if offset >= input.len() {
            return Err("Truncated: key name size".to_string());
        }
        let key_name_size = input[offset] as usize;
        offset += 1;

        if key_name_size != 0 && key_name_size != 8 {
            return Err(format!("Unsupported key name size: {}", key_name_size));
        }

        // Read key name
        if offset + key_name_size > input.len() {
            return Err("Truncated: key name".to_string());
        }
        let mut key_name = 0u64;
        if key_name_size > 0 {
            let key_name_bytes = &input[offset..offset + key_name_size];
            key_name = u64::from_le_bytes(key_name_bytes.try_into().unwrap());
        }
        offset += key_name_size;

        // Read IV size
        if offset >= input.len() {
            return Err("Truncated: IV size".to_string());
        }
        let iv_size = input[offset] as usize;
        offset += 1;

        if iv_size != 4 && iv_size != 8 {
            return Err(format!("Unsupported IV size: {}", iv_size));
        }

        // Read IV
        if offset + iv_size > input.len() {
            return Err("Truncated: IV".to_string());
        }
        let mut vector = [0u8; 8];
        vector[..iv_size].copy_from_slice(&input[offset..offset + iv_size]);
        offset += iv_size;

        // Read encryption type
        if offset >= input.len() {
            return Err("Truncated: encryption type".to_string());
        }
        let encryption_type = input[offset];
        offset += 1;

        if encryption_type != b'S' && encryption_type != b'A' {
            return Err(format!("Unsupported encryption type: {}", encryption_type as char));
        }

        // Find encryption key
        let key = self.key_map.get(&key_name)
            .ok_or_else(|| format!("Unknown encryption key: 0x{:016X}", key_name))?;

        // XOR vector with frame index
        for (i, byte) in vector[..4].iter_mut().enumerate() {
            *byte ^= ((frame_index >> (i * 8)) & 0xFF) as u8;
        }

        // Decrypt based on type
        let encrypted_data = &input[offset..];
        let mut decrypted = vec![0u8; encrypted_data.len()];

        match encryption_type {
            b'S' => {
                decrypt_salsa20(&mut decrypted, encrypted_data, key, &vector);
                Ok(decrypted)
            }
            b'A' => Err("AES decryption not implemented".to_string()),
            _ => Err(format!("Unknown encryption type: {}", encryption_type as char)),
        }
    }
}

impl Default for CascDecryptor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decryptor_creation() {
        let decryptor = CascDecryptor::new();
        assert!(!decryptor.key_map.is_empty());
    }

    #[test]
    fn test_decrypt_empty() {
        let decryptor = CascDecryptor::new();
        assert!(decryptor.decrypt(&[], 0).is_err());
    }
}
