//! DRM token generation for Microsoft Edge TTS authentication.

use chrono::{DateTime, TimeZone, Utc};
use sha2::{Digest, Sha256};

/// DRM token generator for Microsoft Edge TTS authentication.
pub struct DRM;

impl DRM {
    /// Generate the Sec-MS-GEC token.
    ///
    /// This token is required for authentication with the Microsoft Edge TTS service.
    /// It's a SHA256 hash of the current Unix timestamp (with clock skew adjustment)
    /// concatenated with the trusted client token.
    ///
    /// # Arguments
    ///
    /// * `clock_skew_seconds` - Optional clock skew adjustment in seconds
    ///
    /// # Returns
    ///
    /// A tuple containing (timestamp, hash) where:
    /// - timestamp is the Unix timestamp used (with skew adjustment)
    /// - hash is the SHA256 hash in lowercase hex format
    pub fn generate_sec_ms_gec(clock_skew_seconds: Option<i64>) -> (i64, String) {
        let skew = clock_skew_seconds.unwrap_or(0);
        let timestamp = Utc::now().timestamp() + skew;
        
        let data = format!(
            "{}{}",
            timestamp,
            crate::constants::TRUSTED_CLIENT_TOKEN
        );
        
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let hash = hasher.finalize();
        let hash_hex = hex::encode(hash);
        
        (timestamp, hash_hex)
    }

    /// Generate a random MUID (Machine User ID).
    ///
    /// This is used for tracking purposes in the request headers.
    ///
    /// # Returns
    ///
    /// A random UUID string without hyphens
    pub fn generate_muid() -> String {
        uuid::Uuid::new_v4()
            .to_string()
            .replace("-", "")
    }

    /// Generate a date string for the X-Ms-Date header.
    ///
    /// # Arguments
    ///
    /// * `clock_skew_seconds` - Optional clock skew adjustment in seconds
    ///
    /// # Returns
    ///
    /// A date string in RFC 1123 format (e.g., "Mon, 01 Jan 2024 12:00:00 GMT")
    pub fn generate_date(clock_skew_seconds: Option<i64>) -> String {
        let skew = clock_skew_seconds.unwrap_or(0);
        let timestamp = Utc::now().timestamp() + skew;
        
        // RFC 1123 format
        let dt: DateTime<Utc> = Utc.timestamp_opt(timestamp, 0).single().unwrap_or_else(Utc::now);
        dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sec_ms_gec() {
        let (timestamp, hash) = DRM::generate_sec_ms_gec(None);
        
        // Hash should be 64 characters (SHA256 in hex)
        assert_eq!(hash.len(), 64);
        
        // Hash should be lowercase hex
        assert!(hash.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
        
        // Timestamp should be recent (within last minute)
        let now = Utc::now().timestamp();
        assert!((now - timestamp).abs() < 60);
    }

    #[test]
    fn test_generate_muid() {
        let muid = DRM::generate_muid();
        
        // MUID should be 32 characters (UUID without hyphens)
        assert_eq!(muid.len(), 32);
        
        // MUID should be lowercase hex
        assert!(muid.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }

    #[test]
    fn test_generate_date() {
        let date = DRM::generate_date(None);
        
        // Date should end with GMT
        assert!(date.ends_with("GMT"));
    }
}
