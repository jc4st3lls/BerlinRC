//! OTP (TOTP) utilities used for two-factor authentication in BerlinRC.
//!
//! Provides `MyOtp` — a small wrapper around `totp_rs::TOTP` for generating
//! and verifying 6-digit time-based codes, plus helpers to emit QR codes and
//! to generate new shared secrets. This module centralizes 2FA logic used by
//! the web UI; keep changes here in sync with the `berlinweb` authentication
//! flows.
//!
use totp_rs::{Algorithm, TOTP, Secret};

/// Time-based One-Time Password (TOTP) manager for 2FA
pub struct MyOtp {
    /// TOTP instance for code generation and verification
    totp: TOTP,
}

impl MyOtp {
    /// Create new OTP manager with shared secret
    ///
    /// # Arguments
    /// * `shared_secret` - Base32 encoded shared secret for TOTP
    pub fn new(shared_secret: &str) -> Self {
        let secret = Secret::Encoded(shared_secret.to_string()).to_bytes().unwrap();
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,      // 6-digit OTP code
            1,      // Time step in seconds
            30,     // Time period in seconds     
            secret,
            Some("BerlinRC".to_string()),
            "admin".to_string()
        ).unwrap();
        Self { totp }
    }

    /// Generate current 6-digit OTP code
    pub fn generate_current(&self) -> String {
        self.totp.generate_current().unwrap()
    }

    /// Verify OTP code with time tolerance
    pub fn verify(&self, code: &str) -> bool {
        self.totp.check_current(code).unwrap_or(false)
    }

    /// Get QR code as base64-encoded string
    pub fn get_qr_base64(&self) -> String {
        
        self.totp.get_qr_base64().expect("Error al generar QR")
    }
    
    /// Get QR code as PNG bytes
    pub fn get_qr_png(&self) -> Result<Vec<u8>,String> {
        
        self.totp.get_qr_png()
    }
}

/// Generate a new random OTP secret in base32 format
pub fn generate_otp_secret() -> String{
    let random_bytes = rand::random::<[u8; 20]>();
    

    let secret = Secret::Raw(random_bytes.to_vec());
    let base32_string = secret.to_encoded().to_string();
    base32_string
}