//! TTS Configuration types and validation.

use crate::error::{Error, Result};

/// Boundary type for TTS metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryType {
    /// Word boundary events
    WordBoundary,
    /// Sentence boundary events
    SentenceBoundary,
}

impl Default for BoundaryType {
    fn default() -> Self {
        Self::WordBoundary
    }
}

/// TTS Configuration for the Communicate class
#[derive(Debug, Clone)]
pub struct TTSConfig {
    /// Voice name (Microsoft Server Speech format)
    pub voice: String,
    /// Speech rate (e.g., "+0%", "+50%", "-20%")
    pub rate: String,
    /// Volume (e.g., "+0%", "+50%", "-20%")
    pub volume: String,
    /// Pitch (e.g., "+0Hz", "+50Hz", "-20Hz")
    pub pitch: String,
    /// Boundary type for metadata events
    pub boundary: BoundaryType,
}

impl TTSConfig {
    /// Create a new TTSConfig with validation
    pub fn new(
        voice: impl Into<String>,
        rate: impl Into<String>,
        volume: impl Into<String>,
        pitch: impl Into<String>,
        boundary: BoundaryType,
    ) -> Result<Self> {
        let voice = Self::normalize_voice(voice.into())?;
        let rate = Self::validate_param("rate", &rate.into(), r"^[+-]\d+%$")?;
        let volume = Self::validate_param("volume", &volume.into(), r"^[+-]\d+%$")?;
        let pitch = Self::validate_param("pitch", &pitch.into(), r"^[+-]\d+Hz$")?;

        Ok(Self {
            voice,
            rate,
            volume,
            pitch,
            boundary,
        })
    }

    /// Create a TTSConfig with default values
    pub fn with_voice(voice: impl Into<String>) -> Result<Self> {
        Self::new(
            voice,
            "+0%",
            "+0%",
            "+0Hz",
            BoundaryType::default(),
        )
    }

    /// Normalize voice name to Microsoft Server Speech format
    fn normalize_voice(voice: String) -> Result<String> {
        // Check if already in Microsoft Server Speech format - extract short name
        let ms_pattern = regex::Regex::new(r"^Microsoft Server Speech Text to Speech Voice \((.+), (.+)\)$").unwrap();
        if let Some(caps) = ms_pattern.captures(&voice) {
            let locale = caps.get(1).unwrap().as_str();
            let name = caps.get(2).unwrap().as_str();
            return Ok(format!("{}-{}", locale, name));
        }

        // Check if it's already in short form (e.g., "en-US-EmmaMultilingualNeural")
        let short_pattern = regex::Regex::new(r"^[a-z]{2,}-[A-Z]{2,}-.+$").unwrap();
        if short_pattern.is_match(&voice) {
            return Ok(voice);
        }

        // If it doesn't match either pattern, return an error
        Err(Error::InvalidConfig(format!("Invalid voice name format: {}", voice)))
    }

    /// Validate a parameter against a regex pattern
    fn validate_param(name: &str, value: &str, pattern: &str) -> Result<String> {
        let re = regex::Regex::new(pattern).unwrap();
        if !re.is_match(value) {
            return Err(Error::InvalidConfig(format!(
                "Invalid {} '{}'. Expected pattern: {}",
                name, value, pattern
            )));
        }
        Ok(value.to_string())
    }
}

impl Default for TTSConfig {
    fn default() -> Self {
        Self::with_voice(crate::constants::DEFAULT_VOICE)
            .expect("Default config should be valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TTSConfig::default();
        assert!(config.voice.contains("EmmaMultilingualNeural"));
        assert_eq!(config.rate, "+0%");
        assert_eq!(config.volume, "+0%");
        assert_eq!(config.pitch, "+0Hz");
    }

    #[test]
    fn test_voice_normalization() {
        let config = TTSConfig::with_voice("en-US-EmmaMultilingualNeural").unwrap();
        assert_eq!(config.voice, "en-US-EmmaMultilingualNeural");
    }

    #[test]
    fn test_invalid_rate() {
        let result = TTSConfig::new(
            "en-US-EmmaMultilingualNeural",
            "invalid",
            "+0%",
            "+0Hz",
            BoundaryType::WordBoundary,
        );
        assert!(result.is_err());
    }
}
