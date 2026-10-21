//! DRM token generation for Microsoft Edge TTS authentication.

use chrono::{DateTime, TimeZone, Utc};
use sha2::{Digest, Sha256};

/// DRM token generator for Microsoft Edge TTS authentication.
pub struct DRM;

impl DRM {
    /// Generate the Sec-MS-GEC token.
    ///
    /// Microsoft expects a SHA-256 of the current time expressed as a Windows
    /// FILETIME (100-nanosecond intervals since 1601-01-01), floored to the
    /// nearest 5-minute boundary, concatenated with the trusted client token,
    /// returned as UPPERCASE hex.  This matches the Python edge-tts reference.
    ///
    /// # Arguments
    ///
    /// * `clock_skew_seconds` - Optional clock skew adjustment in seconds
    ///
    /// # Returns
    ///
    /// A tuple containing (win_ts_100ns, hash) where:
    /// - win_ts_100ns is the Windows FILETIME value used
    /// - hash is the SHA256 hash in UPPERCASE hex format
    pub fn generate_sec_ms_gec(clock_skew_seconds: Option<i64>) -> (i64, String) {
        // Seconds between Windows epoch (1601-01-01) and Unix epoch (1970-01-01)
        const WIN_EPOCH: i64 = 11_644_473_600;

        let skew = clock_skew_seconds.unwrap_or(0);
        let unix_ts = Utc::now().timestamp() + skew;

        // Convert to Windows FILETIME seconds, then floor to 5-minute boundary
        let win_ts_s = unix_ts + WIN_EPOCH;
        let win_ts_s = win_ts_s - (win_ts_s % 300);

        // Convert to 100-nanosecond intervals (Windows FILETIME unit)
        let win_ts_100ns = win_ts_s * 10_000_000;

        let data = format!(
            "{}{}",
            win_ts_100ns,
            crate::constants::TRUSTED_CLIENT_TOKEN
        );

        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let hash = hasher.finalize();
        let hash_hex = hex::encode(hash).to_uppercase();

        (win_ts_100ns, hash_hex)
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
        let (win_ts_100ns, hash) = DRM::generate_sec_ms_gec(None);

        // Hash should be 64 characters (SHA256 in hex)
        assert_eq!(hash.len(), 64);

        // Hash should be UPPERCASE hex
        assert!(hash.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));

        // Timestamp should be divisible by 300 * 10_000_000 (5-minute boundary)
        assert_eq!(win_ts_100ns % (300 * 10_000_000), 0);

        // Timestamp must be positive and after 2020-01-01 in Windows FILETIME
        assert!(win_ts_100ns > 132_225_408_000_000_000);
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
