//! Simple stream-based XOR cipher used for lightweight obfuscation
//!
//! This module provides `XorCipher`, a small stateful XOR stream cipher
//! intended for demo/lightweight use between the agent and hub. It is
//! NOT cryptographically secure and should not be relied upon for
//! protecting sensitive data in adversarial environments.
//!
/// Shared secret key for XOR encryption
const KEY: &[u8] = b"@la_meva_clau_secreta_666!";

/// Stream cipher using XOR with position-based key rotation
pub struct XorCipher {
    /// Encryption key bytes
    key: &'static [u8],
    /// Current position in key stream for stateful encryption
    cursor: usize,
}

impl XorCipher {
    /// Create a new XOR cipher instance
    pub fn new() -> Self {
        Self {
            key: KEY,
            cursor: 0,
        }
    }

    /// Apply XOR encryption/decryption to data
    ///
    /// Encrypts or decrypts data in-place using XOR with the shared key.
    /// The cipher maintains position state to create a continuous stream.
    ///
    /// # Arguments
    /// * `data` - Byte slice to encrypt/decrypt in-place
    pub fn apply(&mut self, data: &mut [u8]) {
        // XOR each byte with corresponding key byte, advancing cursor
        for byte in data.iter_mut() {
            *byte ^= self.key[self.cursor % self.key.len()];
            self.cursor += 1;
        }
    }
}



