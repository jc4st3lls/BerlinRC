//! BerlinRC protocol utilities crate.
//!
//! This crate contains small helpers used by the BerlinRC agent and web
//! components: a lightweight XOR stream cipher (`xor`), TOTP-based 2FA
//! helpers (`otp`), and the handshake data structures. These modules are
//! intentionally minimal and focus on internal protocol needs rather than
//! being general-purpose libraries.
//!
/// XOR encryption/decryption module
pub mod xor;
/// One-Time Password generation and verification module
pub mod otp;
/// HandShake Info
pub mod handshake;
#[cfg(test)]
mod tests {
    use crate::{otp::MyOtp, xor::XorCipher};
    
    /// Test XOR encryption and decryption symmetry
    #[test]
    fn xor_works() {
       let data = b"Hola, BerlinCypher!";
        let mut data_copy = data.to_vec();
        let mut xorcipher1=XorCipher::new();
        xorcipher1.apply(&mut data_copy);
        let mut xorcipher2=XorCipher::new();
        xorcipher2.apply(&mut data_copy);
        assert_eq!(data_copy, data);
    }
    
    /// Test OTP generation and verification
    #[test]
    fn otp_works() {
       let shared_secret= "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
       let otp_manager = MyOtp::new(shared_secret);
       let current=otp_manager.generate_current();
       let verify=otp_manager.verify(&current);
       assert_eq!(verify,true);

    }
}
