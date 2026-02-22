//! Voice management for Microsoft Edge TTS.

use crate::constants::{voice_headers, voice_list_url};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// Voice information from Microsoft Edge TTS service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voice {
    /// Voice name in Microsoft Server Speech format
    #[serde(rename = "Name")]
    pub Name: String,
    /// Display name for the voice
    #[serde(rename = "DisplayName")]
    pub DisplayName: String,
    /// Localized name (if applicable)
    #[serde(rename = "LocalName")]
    pub LocalName: String,
    /// Short name (e.g., "en-US-AvaNeural")
    #[serde(rename = "ShortName")]
    pub ShortName: String,
    /// Gender ("Male" or "Female")
    #[serde(rename = "Gender")]
    pub Gender: String,
    /// Locale (e.g., "en-US")
    #[serde(rename = "Locale")]
    pub Locale: String,
    /// Whether the voice is neural
    #[serde(rename = "SuggestedCodec")]
    pub SuggestedCodec: String,
    /// Friendly name with locale
    #[serde(rename = "FriendlyName")]
    pub FriendlyName: String,
    /// Status (usually "Standard")
    #[serde(rename = "Status")]
    pub Status: String,
    /// Voice tags
    #[serde(rename = "VoiceTag")]
    pub VoiceTag: VoiceTag,
}

/// Voice tag information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceTag {
    /// List of content categories the voice is suitable for
    #[serde(rename = "ContentCategories")]
    pub ContentCategories: Vec<String>,
    /// List of voice personalities
    #[serde(rename = "VoicePersonalities")]
    pub VoicePersonalities: Vec<String>,
}

/// Manager for filtering and searching voices.
pub struct VoicesManager {
    voices: Vec<Voice>,
}

impl VoicesManager {
    /// Create a new VoicesManager by fetching voices from the server.
    pub async fn new() -> Result<Self> {
        let voices = list_voices(None).await?;
        Ok(Self { voices })
    }

    /// Create a VoicesManager from a pre-fetched list of voices.
    pub fn from_voices(voices: Vec<Voice>) -> Self {
        Self { voices }
    }

    /// Find voices matching the given criteria.
    ///
    /// # Arguments
    ///
    /// * `gender` - Filter by gender ("Male" or "Female")
    /// * `locale` - Filter by locale (e.g., "en-US")
    /// * `language` - Filter by language code (e.g., "en")
    ///
    /// # Returns
    ///
    /// A vector of voices matching all specified criteria
    pub fn find(
        &self,
        gender: Option<&str>,
        locale: Option<&str>,
        language: Option<&str>,
    ) -> Vec<&Voice> {
        self.voices
            .iter()
            .filter(|voice| {
                if let Some(g) = gender {
                    if voice.Gender != g {
                        return false;
                    }
                }
                if let Some(l) = locale {
                    if !voice.Locale.eq_ignore_ascii_case(l) {
                        return false;
                    }
                }
                if let Some(lang) = language {
                    if !voice.Locale.to_lowercase().starts_with(&lang.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    /// Get all voices.
    pub fn all(&self) -> &[Voice] {
        &self.voices
    }

    /// Get the number of voices.
    pub fn len(&self) -> usize {
        self.voices.len()
    }

    /// Check if there are no voices.
    pub fn is_empty(&self) -> bool {
        self.voices.is_empty()
    }
}

/// Fetch the list of available voices from Microsoft Edge TTS service.
///
/// # Arguments
///
/// * `proxy` - Optional proxy URL (not currently implemented)
///
/// # Returns
///
/// A vector of Voice structs containing voice information
///
/// # Example
///
/// ```rust,no_run
/// use edge_tts::list_voices;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let voices = list_voices(None).await?;
///     println!("Found {} voices", voices.len());
///     Ok(())
/// }
/// ```
pub async fn list_voices(_proxy: Option<&str>) -> Result<Vec<Voice>> {
    let client = reqwest::Client::new();
    let url = voice_list_url();
    
    let response = client
        .get(&url)
        .headers(voice_headers())
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(Error::Http(response.error_for_status().unwrap_err()));
    }
    
    let text = response.text().await?;
    let voices: Vec<Voice> = serde_json::from_str(&text)?;
    
    Ok(voices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_voices() {
        let voices = list_voices(None).await.unwrap();
        assert!(!voices.is_empty());
        
        // Check that we have at least some expected voices
        let has_english = voices.iter().any(|v| v.Locale.starts_with("en"));
        assert!(has_english);
    }
}
