/// Salsa20 stream cipher implementation for CASC decryption
/// Ported from CascLib's CascDecrypt.cpp
const KEY_CONSTANT_16: &[u8; 16] = b"expand 16-byte k";
const KEY_CONSTANT_32: &[u8; 16] = b"expand 32-byte k";

#[inline]
fn rol32(value: u32, shift: u32) -> u32 {
    value.rotate_left(shift)
}

struct Salsa20State {
    key: [u32; 16],
    rounds: u32,
}

impl Salsa20State {
    fn new(key: &[u8], vector: &[u8]) -> Self {
        let key_len = key.len();
        let constants = if key_len == 32 { KEY_CONSTANT_32 } else { KEY_CONSTANT_16 };
        let key_index = key_len.saturating_sub(0x10);

        let mut state = [0u32; 16];
        state[0] = u32::from_le_bytes([constants[0], constants[1], constants[2], constants[3]]);
        state[1] = u32::from_le_bytes([key[0], key[1], key[2], key[3]]);
        state[2] = u32::from_le_bytes([key[4], key[5], key[6], key[7]]);
        state[3] = u32::from_le_bytes([key[8], key[9], key[10], key[11]]);
        state[4] = u32::from_le_bytes([key[12], key[13], key[14], key[15]]);
        state[5] = u32::from_le_bytes([constants[4], constants[5], constants[6], constants[7]]);
        state[6] = u32::from_le_bytes([vector[0], vector[1], vector[2], vector[3]]);
        state[7] = u32::from_le_bytes([vector[4], vector[5], vector[6], vector[7]]);
        state[8] = 0;
        state[9] = 0;
        state[10] = u32::from_le_bytes([constants[8], constants[9], constants[10], constants[11]]);
        state[11] = u32::from_le_bytes([key[key_index], key[key_index + 1], key[key_index + 2], key[key_index + 3]]);
        state[12] = u32::from_le_bytes([key[key_index + 4], key[key_index + 5], key[key_index + 6], key[key_index + 7]]);
        state[13] = u32::from_le_bytes([key[key_index + 8], key[key_index + 9], key[key_index + 10], key[key_index + 11]]);
        state[14] = u32::from_le_bytes([key[key_index + 12], key[key_index + 13], key[key_index + 14], key[key_index + 15]]);
        state[15] = u32::from_le_bytes([constants[12], constants[13], constants[14], constants[15]]);

        Self {
            key: state,
            rounds: 20,
        }
    }

    fn decrypt(&mut self, output: &mut [u8], input: &[u8]) {
        let mut offset = 0;
        let mut remaining = input.len();

        while remaining > 0 {
            let mut key_mirror = self.key;

            // Shuffle the key (Salsa20 core)
            for _ in (0..self.rounds).step_by(2) {
                key_mirror[0x04] ^= rol32(key_mirror[0x00].wrapping_add(key_mirror[0x0C]), 0x07);
                key_mirror[0x08] ^= rol32(key_mirror[0x04].wrapping_add(key_mirror[0x00]), 0x09);
                key_mirror[0x0C] ^= rol32(key_mirror[0x08].wrapping_add(key_mirror[0x04]), 0x0D);
                key_mirror[0x00] ^= rol32(key_mirror[0x0C].wrapping_add(key_mirror[0x08]), 0x12);

                key_mirror[0x09] ^= rol32(key_mirror[0x05].wrapping_add(key_mirror[0x01]), 0x07);
                key_mirror[0x0D] ^= rol32(key_mirror[0x09].wrapping_add(key_mirror[0x05]), 0x09);
                key_mirror[0x01] ^= rol32(key_mirror[0x0D].wrapping_add(key_mirror[0x09]), 0x0D);
                key_mirror[0x05] ^= rol32(key_mirror[0x01].wrapping_add(key_mirror[0x0D]), 0x12);

                key_mirror[0x0E] ^= rol32(key_mirror[0x0A].wrapping_add(key_mirror[0x06]), 0x07);
                key_mirror[0x02] ^= rol32(key_mirror[0x0E].wrapping_add(key_mirror[0x0A]), 0x09);
                key_mirror[0x06] ^= rol32(key_mirror[0x02].wrapping_add(key_mirror[0x0E]), 0x0D);
                key_mirror[0x0A] ^= rol32(key_mirror[0x06].wrapping_add(key_mirror[0x02]), 0x12);

                key_mirror[0x03] ^= rol32(key_mirror[0x0F].wrapping_add(key_mirror[0x0B]), 0x07);
                key_mirror[0x07] ^= rol32(key_mirror[0x03].wrapping_add(key_mirror[0x0F]), 0x09);
                key_mirror[0x0B] ^= rol32(key_mirror[0x07].wrapping_add(key_mirror[0x03]), 0x0D);
                key_mirror[0x0F] ^= rol32(key_mirror[0x0B].wrapping_add(key_mirror[0x07]), 0x12);

                key_mirror[0x01] ^= rol32(key_mirror[0x00].wrapping_add(key_mirror[0x03]), 0x07);
                key_mirror[0x02] ^= rol32(key_mirror[0x01].wrapping_add(key_mirror[0x00]), 0x09);
                key_mirror[0x03] ^= rol32(key_mirror[0x02].wrapping_add(key_mirror[0x01]), 0x0D);
                key_mirror[0x00] ^= rol32(key_mirror[0x03].wrapping_add(key_mirror[0x02]), 0x12);

                key_mirror[0x06] ^= rol32(key_mirror[0x05].wrapping_add(key_mirror[0x04]), 0x07);
                key_mirror[0x07] ^= rol32(key_mirror[0x06].wrapping_add(key_mirror[0x05]), 0x09);
                key_mirror[0x04] ^= rol32(key_mirror[0x07].wrapping_add(key_mirror[0x06]), 0x0D);
                key_mirror[0x05] ^= rol32(key_mirror[0x04].wrapping_add(key_mirror[0x07]), 0x12);

                key_mirror[0x0B] ^= rol32(key_mirror[0x0A].wrapping_add(key_mirror[0x09]), 0x07);
                key_mirror[0x08] ^= rol32(key_mirror[0x0B].wrapping_add(key_mirror[0x0A]), 0x09);
                key_mirror[0x09] ^= rol32(key_mirror[0x08].wrapping_add(key_mirror[0x0B]), 0x0D);
                key_mirror[0x0A] ^= rol32(key_mirror[0x09].wrapping_add(key_mirror[0x08]), 0x12);

                key_mirror[0x0C] ^= rol32(key_mirror[0x0F].wrapping_add(key_mirror[0x0E]), 0x07);
                key_mirror[0x0D] ^= rol32(key_mirror[0x0C].wrapping_add(key_mirror[0x0F]), 0x09);
                key_mirror[0x0E] ^= rol32(key_mirror[0x0D].wrapping_add(key_mirror[0x0C]), 0x0D);
                key_mirror[0x0F] ^= rol32(key_mirror[0x0E].wrapping_add(key_mirror[0x0D]), 0x12);
            }

            // Generate XOR keystream
            let mut xor_value = [0u32; 16];
            for i in 0..16 {
                xor_value[i] = key_mirror[i].wrapping_add(self.key[i]);
            }

            // Convert to bytes and XOR with input
            let block_size = remaining.min(0x40);
            // SAFETY: xor_value is [u32; 16] = 64 bytes on all platforms. Interpreting as &[u8; 64] is valid.
            let xor_bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(xor_value.as_ptr() as *const u8, 64)
            };

            for i in 0..block_size {
                output[offset + i] = input[offset + i] ^ xor_bytes[i];
            }

            // Increment counter
            self.key[8] = self.key[8].wrapping_add(1);
            if self.key[8] == 0 {
                self.key[9] = self.key[9].wrapping_add(1);
            }

            offset += block_size;
            remaining -= block_size;
        }
    }
}

pub fn decrypt_salsa20(output: &mut [u8], input: &[u8], key: &[u8], vector: &[u8]) {
    let mut state = Salsa20State::new(key, vector);
    state.decrypt(output, input);
}
