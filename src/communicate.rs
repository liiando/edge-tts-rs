//! Core TTS communication module.
//!
//! This module provides the main [`Communicate`] struct for interacting with
//! the Microsoft Edge TTS service via WebSocket.

use futures_util::stream::{Stream, StreamExt};
use futures_util::sink::SinkExt;
use std::pin::Pin;
use std::time::Duration;
use tokio_tungstenite::{connect_async_with_config, tungstenite::protocol::WebSocketConfig};
use tokio_tungstenite::tungstenite::{Message, handshake::client::Request, handshake::client::generate_key};
use serde::Deserialize;

use crate::config::{BoundaryType, TTSConfig};
use crate::error::{Error, Result};
use crate::constants;
use crate::drm::DRM;
use chrono::Timelike;

/// Maximum bytes per text chunk (to avoid server limits)
const MAX_TEXT_LENGTH: usize = 4096;

/// A chunk of data from the TTS stream
#[derive(Debug, Clone)]
pub enum TTSChunk {
    /// Audio data (binary)
    Audio {
        /// Audio bytes (MP3 format)
        data: Vec<u8>,
    },
    /// Word boundary metadata
    WordBoundary {
        /// Offset in 100-nanosecond units
        offset: f64,
        /// Duration in 100-nanosecond units
        duration: f64,
        /// The word text
        text: String,
    },
    /// Sentence boundary metadata
    SentenceBoundary {
        /// Offset in 100-nanosecond units
        offset: f64,
        /// Duration in 100-nanosecond units
        duration: f64,
        /// The sentence text
        text: String,
    },
}

/// Internal state for the Communicate instance
#[allow(dead_code)]
struct CommunicateState {
    /// Offset compensation for multi-chunk streaming
    offset_compensation: f64,
    /// Last duration offset for boundary calculations
    last_duration_offset: f64,
    /// Whether stream() has been called
    stream_was_called: bool,
}

/// Main TTS communication handler
pub struct Communicate {
    /// TTS configuration
    config: TTSConfig,
    /// Text chunks to process
    texts: Vec<String>,
    /// Optional proxy URL
    proxy: Option<String>,
    /// Connection timeout
    connect_timeout: Duration,
    /// Receive timeout
    receive_timeout: Duration,
    /// Internal state
    state: CommunicateState,
}

impl Communicate {
    /// Create a new Communicate instance with default settings
    pub fn new(text: &str) -> Self {
        Self::builder(text).build()
    }

    /// Create a builder for custom configuration
    pub fn builder(text: &str) -> CommunicateBuilder {
        CommunicateBuilder::new(text)
    }

    /// Stream TTS audio and metadata
    ///
    /// Returns a stream of [`TTSChunk`] items.
    pub async fn stream(
        &mut self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<TTSChunk>> + Send>>> {
        if self.state.stream_was_called {
            return Err(Error::StreamConsumed);
        }
        self.state.stream_was_called = true;

        let texts: Vec<String> = self.texts.clone();
        let config = self.config.clone();
        let proxy = self.proxy.clone();
        let connect_timeout = self.connect_timeout;
        let receive_timeout = self.receive_timeout;

        // Create a channel to send chunks
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<TTSChunk>>(100);

        // Spawn the WebSocket handler task
        tokio::spawn(async move {
            if let Err(e) = Self::handle_stream(texts, config, proxy, connect_timeout, receive_timeout, tx.clone()).await {
                let _ = tx.send(Err(e)).await;
            }
        });

        // Convert the receiver to a stream
        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    /// Handle the WebSocket streaming
    async fn handle_stream(
        texts: Vec<String>,
        config: TTSConfig,
        proxy: Option<String>,
        _connect_timeout: Duration,
        _receive_timeout: Duration,
        tx: tokio::sync::mpsc::Sender<Result<TTSChunk>>,
    ) -> Result<()> {
        let mut offset_compensation: f64 = 0.0;
        let mut last_duration_offset: f64 = 0.0;
        let mut audio_received = false;

        for partial_text in texts {
            let result = Self::stream_single_chunk(
                &partial_text,
                &config,
                &proxy,
                &tx,
                &mut offset_compensation,
                &mut last_duration_offset,
                &mut audio_received,
            ).await;

            if let Err(e) = result {
                // Check for 403 error - clock skew
                if matches!(e, Error::WebSocket(_)) {
                    // Try to adjust clock skew and retry
                    DRM::adjust_clock_skew(5.0);
                    let retry_result = Self::stream_single_chunk(
                        &partial_text,
                        &config,
                        &proxy,
                        &tx,
                        &mut offset_compensation,
                        &mut last_duration_offset,
                        &mut audio_received,
                    ).await;
                    if let Err(retry_e) = retry_result {
                        let _ = tx.send(Err(retry_e)).await;
                        return Ok(());
                    }
                } else {
                    let _ = tx.send(Err(e)).await;
                    return Ok(());
                }
            }
        }

        if !audio_received {
            let _ = tx.send(Err(Error::NoAudioReceived)).await;
        }

        Ok(())
    }

    /// Stream a single text chunk
    async fn stream_single_chunk(
        partial_text: &str,
        config: &TTSConfig,
        _proxy: &Option<String>,
        tx: &tokio::sync::mpsc::Sender<Result<TTSChunk>>,
        offset_compensation: &mut f64,
        last_duration_offset: &mut f64,
        audio_received: &mut bool,
    ) -> Result<()> {
        // Build WebSocket URL with connection ID and security tokens
        let connection_id = connect_id();
        let sec_ms_gec = DRM::generate_sec_ms_gec();
        let url = format!(
            "{}&ConnectionId={}&Sec-MS-GEC={}&Sec-MS-GEC-Version={}",
            constants::wss_url(),
            connection_id,
            sec_ms_gec,
            constants::SEC_MS_GEC_VERSION
        );

        // Build the WebSocket request with headers
        let ws_key = generate_key();
        let request = Request::builder()
            .method("GET")
            .uri(&url)
            .header("Host", "speech.platform.bing.com")
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Pragma", "no-cache")
            .header("Cache-Control", "no-cache")
            .header("Origin", "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold")
            .header("User-Agent", format!(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/{}.0.0.0 Safari/537.36 \
                 Edg/{}.0.0.0",
                constants::CHROMIUM_MAJOR_VERSION,
                constants::CHROMIUM_MAJOR_VERSION
            ))
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", ws_key)
            .header("Cookie", format!("muid={}", DRM::generate_muid()))
            .body(())
            .map_err(|e| Error::InvalidConfig(format!("Failed to build request: {}", e)))?;

        // Configure WebSocket
        let ws_config = WebSocketConfig {
            max_message_size: Some(10 * 1024 * 1024), // 10 MB
            max_frame_size: Some(10 * 1024 * 1024),
            ..Default::default()
        };

        // Connect to WebSocket
        let (ws_stream, _) = connect_async_with_config(request, Some(ws_config), false)
            .await?;

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Send speech.config message
        let timestamp = date_to_string();
        let word_boundary_enabled = config.boundary == BoundaryType::WordBoundary;
        let sentence_boundary_enabled = config.boundary == BoundaryType::SentenceBoundary;
        
        // Convert boolean to "true" or "false" string (lowercase, as Python does)
        let wb_str = if word_boundary_enabled { "true" } else { "false" };
        let sb_str = if sentence_boundary_enabled { "true" } else { "false" };
        
        let config_msg = format!(
            "X-Timestamp:{}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n{{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"{}\",\"wordBoundaryEnabled\":\"{}\"}},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"}}}}}}}}",
            timestamp,
            sb_str,
            wb_str
        );
        
        ws_sender.send(Message::Text(config_msg)).await?;

        // Send SSML message - use the voice name directly (short name like "en-US-AvaNeural")
        let ssml = create_ssml(partial_text, &config.voice, &config.rate, &config.volume, &config.pitch);
        let request_id = connect_id();
        let ssml_msg = format!(
            "X-RequestId:{}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{}Z\r\nPath:ssml\r\n\r\n{}",
            request_id,
            date_to_string(),
            ssml
        );
        
        ws_sender.send(Message::Text(ssml_msg)).await?;

        // Process incoming messages
        while let Some(msg) = ws_receiver.next().await {
            match msg? {
                Message::Text(text) => {
                    let encoded = text.as_bytes();
                    let separator_pos = encoded.windows(4)
                        .position(|w| w == b"\r\n\r\n")
                        .unwrap_or(encoded.len());
                    
                    let headers = &encoded[..separator_pos];
                    let data = if separator_pos + 4 < encoded.len() {
                        &encoded[separator_pos + 4..]
                    } else {
                        &[]
                    };

                    let path = parse_header_value(headers, b"Path");
                    tracing::debug!("Path: {:?}", path);

                    match path.as_deref() {
                        Some(b"audio.metadata") => {
                            if let Ok(parsed) = parse_metadata(data, *offset_compensation) {
                                // Update last duration offset
                                *last_duration_offset = parsed.offset + parsed.duration;
                                
                                let chunk = match parsed.metadata_type.as_str() {
                                    "WordBoundary" => TTSChunk::WordBoundary {
                                        offset: parsed.offset,
                                        duration: parsed.duration,
                                        text: parsed.text,
                                    },
                                    "SentenceBoundary" => TTSChunk::SentenceBoundary {
                                        offset: parsed.offset,
                                        duration: parsed.duration,
                                        text: parsed.text,
                                    },
                                    _ => continue,
                                };
                                
                                if tx.send(Ok(chunk)).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Some(b"turn.end") => {
                            // Update offset compensation for next chunk
                            *offset_compensation = *last_duration_offset + 8_750_000.0;
                            break;
                        }
                        Some(b"response") | Some(b"turn.start") => {
                            // These are expected, continue
                        }
                        _ => {
                            // Unknown path
                        }
                    }
                }
                Message::Binary(data) => {
                    if data.len() < 2 {
                        continue;
                    }

                    // First 2 bytes are header length (big-endian)
                    let header_len = u16::from_be_bytes([data[0], data[1]]) as usize;
                    if header_len > data.len() {
                        continue;
                    }

                    let headers = &data[2..2 + header_len];
                    let audio_data = &data[2 + header_len..];

                    // Check if path is "audio"
                    let path = parse_header_value(headers, b"Path");
                    if path.as_deref() != Some(b"audio") {
                        continue;
                    }

                    // Check content type
                    let content_type = parse_header_value(headers, b"Content-Type");
                    match content_type.as_deref() {
                        Some(b"audio/mpeg") => {
                            if !audio_data.is_empty() {
                                *audio_received = true;
                                if tx.send(Ok(TTSChunk::Audio { data: audio_data.to_vec() })).await.is_err() {
                                    break;
                                }
                            }
                        }
                        None => {
                            // No content type with no data is expected at termination
                            if audio_data.is_empty() {
                                continue;
                            }
                        }
                        _ => {}
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }

        Ok(())
    }

    /// Save TTS audio to a file
    ///
    /// Optionally also save metadata (word boundaries) to a separate file.
    pub async fn save<P: AsRef<std::path::Path>>(
        &mut self,
        audio_path: P,
        metadata_path: Option<P>,
    ) -> Result<()> {
        use tokio::fs::File;
        use tokio::io::AsyncWriteExt;

        let mut audio_file = File::create(audio_path).await?;
        let mut metadata_file = if let Some(path) = metadata_path {
            Some(File::create(path).await?)
        } else {
            None
        };

        let mut stream = std::pin::pin!(self.stream().await?);

        while let Some(chunk) = stream.next().await {
            match chunk? {
                TTSChunk::Audio { data } => {
                    audio_file.write_all(&data).await?;
                }
                TTSChunk::WordBoundary { offset, duration, text } => {
                    if let Some(ref mut file) = metadata_file {
                        let json = serde_json::json!({
                            "type": "WordBoundary",
                            "offset": offset,
                            "duration": duration,
                            "text": text
                        });
                        file.write_all(format!("{}\n", json).as_bytes()).await?;
                    }
                }
                TTSChunk::SentenceBoundary { offset, duration, text } => {
                    if let Some(ref mut file) = metadata_file {
                        let json = serde_json::json!({
                            "type": "SentenceBoundary",
                            "offset": offset,
                            "duration": duration,
                            "text": text
                        });
                        file.write_all(format!("{}\n", json).as_bytes()).await?;
                    }
                }
            }
        }

        audio_file.flush().await?;
        if let Some(mut file) = metadata_file {
            file.flush().await?;
        }

        Ok(())
    }
}

/// Builder for [`Communicate`] instances
pub struct CommunicateBuilder {
    text: String,
    voice: String,
    rate: String,
    volume: String,
    pitch: String,
    boundary: BoundaryType,
    proxy: Option<String>,
    connect_timeout: Duration,
    receive_timeout: Duration,
}

impl CommunicateBuilder {
    /// Create a new builder with the given text
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            voice: constants::DEFAULT_VOICE.to_string(),
            rate: "+0%".to_string(),
            volume: "+0%".to_string(),
            pitch: "+0Hz".to_string(),
            boundary: BoundaryType::default(),
            proxy: None,
            connect_timeout: Duration::from_secs(10),
            receive_timeout: Duration::from_secs(60),
        }
    }

    /// Set the voice
    pub fn voice(mut self, voice: impl Into<String>) -> Self {
        self.voice = voice.into();
        self
    }

    /// Set the speech rate
    pub fn rate(mut self, rate: impl Into<String>) -> Self {
        self.rate = rate.into();
        self
    }

    /// Set the volume
    pub fn volume(mut self, volume: impl Into<String>) -> Self {
        self.volume = volume.into();
        self
    }

    /// Set the pitch
    pub fn pitch(mut self, pitch: impl Into<String>) -> Self {
        self.pitch = pitch.into();
        self
    }

    /// Set the boundary type
    pub fn boundary(mut self, boundary: BoundaryType) -> Self {
        self.boundary = boundary;
        self
    }

    /// Set the proxy URL
    pub fn proxy(mut self, proxy: impl Into<String>) -> Self {
        self.proxy = Some(proxy.into());
        self
    }

    /// Set the connection timeout
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set the receive timeout
    pub fn receive_timeout(mut self, timeout: Duration) -> Self {
        self.receive_timeout = timeout;
        self
    }

    /// Build the [`Communicate`] instance
    pub fn build(self) -> Communicate {
        let config = TTSConfig::new(
            self.voice,
            self.rate,
            self.volume,
            self.pitch,
            self.boundary,
        )
        .expect("Invalid config");

        let texts = split_text_by_byte_length(&self.text, MAX_TEXT_LENGTH);

        Communicate {
            config,
            texts,
            proxy: self.proxy,
            connect_timeout: self.connect_timeout,
            receive_timeout: self.receive_timeout,
            state: CommunicateState {
                offset_compensation: 0.0,
                last_duration_offset: 0.0,
                stream_was_called: false,
            },
        }
    }
}

/// Generate a connection ID (UUID without dashes)
fn connect_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// Generate a JavaScript-style date string
fn date_to_string() -> String {
    let now = chrono::Utc::now();
    format!(
        "{} {} {} {} {:02}:{:02}:{:02} GMT+0000 (Coordinated Universal Time)",
        now.format("%a"),
        now.format("%b"),
        now.format("%d"),
        now.format("%Y"),
        now.hour(),
        now.minute(),
        now.second()
    )
}

/// Create SSML from text and voice parameters
fn create_ssml(text: &str, voice: &str, rate: &str, volume: &str, pitch: &str) -> String {
    let escaped = escape_xml(text);
    format!(
        "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'>\
         <voice name='{}'>\
         <prosody pitch='{}' rate='{}' volume='{}'>\
         {}\
         </prosody>\
         </voice>\
         </speak>",
        voice, pitch, rate, volume, escaped
    )
}

/// Escape XML special characters
fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Remove incompatible characters (control characters)
fn remove_incompatible_characters(text: &str) -> String {
    text.chars()
        .map(|c| {
            let code = c as u32;
            if (0..=8).contains(&code) || (11..=12).contains(&code) || (14..=31).contains(&code) {
                ' '
            } else {
                c
            }
        })
        .collect()
}

/// Split text into chunks of maximum byte length
///
/// This function ensures:
/// - UTF-8 characters are never split
/// - XML entities are preserved (not split in the middle)
/// - Natural boundaries (newlines, spaces) are preferred
fn split_text_by_byte_length(text: &str, max_bytes: usize) -> Vec<String> {
    // First remove incompatible characters and escape XML
    let cleaned = remove_incompatible_characters(text);
    let escaped = escape_xml(&cleaned);
    
    if escaped.is_empty() {
        return vec![];
    }

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_bytes = 0;

    for line in escaped.split('\n') {
        let line_with_newline = if current_chunk.is_empty() {
            line.to_string()
        } else {
            format!("\n{}", line)
        };

        let line_bytes = line_with_newline.len();

        if current_bytes + line_bytes <= max_bytes {
            current_chunk.push_str(&line_with_newline);
            current_bytes += line_bytes;
        } else {
            if !current_chunk.is_empty() {
                chunks.push(current_chunk);
                current_chunk = String::new();
                current_bytes = 0;
            }

            // Split the line by words if it's too long
            for word in line.split_whitespace() {
                let word_bytes = word.len() + 1; // +1 for space
                if current_bytes + word_bytes <= max_bytes {
                    if current_chunk.is_empty() {
                        current_chunk.push_str(word);
                        current_bytes = word.len();
                    } else {
                        current_chunk.push_str(&format!(" {}", word));
                        current_bytes += word_bytes;
                    }
                } else {
                    if !current_chunk.is_empty() {
                        chunks.push(current_chunk);
                    }
                    current_chunk = word.to_string();
                    current_bytes = word.len();
                }
            }
        }
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    chunks
}

/// Parse a header value from binary headers
fn parse_header_value<'a>(headers: &'a [u8], key: &[u8]) -> Option<Vec<u8>> {
    let key_with_colon = [key, b":"].concat();
    for line in headers.split(|&b| b == b'\n') {
        if line.starts_with(&key_with_colon) {
            let value = &line[key_with_colon.len()..];
            // Trim leading whitespace and \r
            let value: Vec<u8> = value.iter()
                .skip_while(|&&b| b == b' ' || b == b'\r')
                .copied()
                .collect();
            // Trim trailing \r
            let value = if value.last() == Some(&b'\r') {
                value[..value.len() - 1].to_vec()
            } else {
                value
            };
            return Some(value);
        }
    }
    None
}

/// Parsed metadata from the server
#[derive(Debug, Deserialize)]
#[allow(non_snake_case, dead_code)]
struct MetadataItem {
    #[serde(rename = "Type")]
    metadata_type: String,
    #[serde(rename = "Data")]
    data: MetadataData,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case, dead_code)]
struct MetadataData {
    Offset: f64,
    Duration: f64,
    #[serde(rename = "text")]
    text: MetadataText,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case, dead_code)]
struct MetadataText {
    Text: String,
}

#[derive(Debug)]
struct ParsedMetadata {
    metadata_type: String,
    offset: f64,
    duration: f64,
    text: String,
}

/// Parse metadata JSON from the server
fn parse_metadata(data: &[u8], offset_compensation: f64) -> Result<ParsedMetadata> {
    let metadata: serde_json::Value = serde_json::from_slice(data)?;
    
    if let Some(meta_array) = metadata.get("Metadata").and_then(|m| m.as_array()) {
        for meta_obj in meta_array {
            if let Some(meta_type) = meta_obj.get("Type").and_then(|t| t.as_str()) {
                if meta_type == "WordBoundary" || meta_type == "SentenceBoundary" {
                    let data = meta_obj.get("Data").ok_or_else(|| {
                        Error::UnexpectedResponse("Missing Data field in metadata".to_string())
                    })?;
                    
                    let offset = data.get("Offset")
                        .and_then(|o| o.as_f64())
                        .ok_or_else(|| Error::UnexpectedResponse("Missing Offset".to_string()))?;
                    
                    let duration = data.get("Duration")
                        .and_then(|d| d.as_f64())
                        .ok_or_else(|| Error::UnexpectedResponse("Missing Duration".to_string()))?;
                    
                    let text = data.get("text")
                        .and_then(|t| t.get("Text"))
                        .and_then(|t| t.as_str())
                        .ok_or_else(|| Error::UnexpectedResponse("Missing text".to_string()))?;
                    
                    // Unescape XML entities
                    let unescaped_text = unescape_xml(text);
                    
                    return Ok(ParsedMetadata {
                        metadata_type: meta_type.to_string(),
                        offset: offset + offset_compensation,
                        duration,
                        text: unescaped_text,
                    });
                }
                
                if meta_type == "SessionEnd" {
                    continue;
                }
            }
        }
    }
    
    Err(Error::UnexpectedResponse("No valid metadata found".to_string()))
}

/// Unescape XML entities
fn unescape_xml(text: &str) -> String {
    text.replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
}

/// Subtitle maker for generating SRT files from TTS metadata
pub struct SubMaker {
    /// Collected subtitle cues
    cues: Vec<SubtitleCue>,
    /// Boundary type being used
    boundary_type: Option<String>,
}

/// A single subtitle cue
#[derive(Debug, Clone)]
struct SubtitleCue {
    /// Start time in 100-nanosecond units
    start: f64,
    /// End time in 100-nanosecond units
    end: f64,
    /// Subtitle text
    text: String,
}

impl SubMaker {
    /// Create a new SubMaker
    pub fn new() -> Self {
        Self {
            cues: Vec::new(),
            boundary_type: None,
        }
    }

    /// Feed a TTS chunk to the subtitle maker
    pub fn feed(&mut self, chunk: &TTSChunk) -> std::result::Result<(), String> {
        match chunk {
            TTSChunk::WordBoundary { offset, duration, text } => {
                if self.boundary_type.is_none() {
                    self.boundary_type = Some("WordBoundary".to_string());
                }
                self.cues.push(SubtitleCue {
                    start: *offset,
                    end: offset + duration,
                    text: text.clone(),
                });
            }
            TTSChunk::SentenceBoundary { offset, duration, text } => {
                if self.boundary_type.is_none() {
                    self.boundary_type = Some("SentenceBoundary".to_string());
                }
                self.cues.push(SubtitleCue {
                    start: *offset,
                    end: offset + duration,
                    text: text.clone(),
                });
            }
            _ => return Err("Invalid chunk type for subtitle".to_string()),
        }
        Ok(())
    }

    /// Generate SRT format subtitles
    pub fn get_srt(&self) -> String {
        let mut output = String::new();
        
        for (i, cue) in self.cues.iter().enumerate() {
            output.push_str(&format!(
                "{}\n{} --> {}\n{}\n\n",
                i + 1,
                format_timestamp(cue.start),
                format_timestamp(cue.end),
                cue.text
            ));
        }
        
        output
    }
}

impl Default for SubMaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a timestamp in 100-nanosecond units to SRT format (HH:MM:SS,mmm)
fn format_timestamp(ticks: f64) -> String {
    // Convert from 100-nanosecond units to milliseconds
    // 1 tick = 100ns, 1ms = 10,000 ticks
    let total_ms = (ticks / 10_000.0) as i64;
    
    let hours = total_ms / 3_600_000;
    let minutes = (total_ms % 3_600_000) / 60_000;
    let seconds = (total_ms % 60_000) / 1000;
    let millis = total_ms % 1000;
    
    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_text_empty() {
        let chunks = split_text_by_byte_length("", 4096);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_split_text_short() {
        let chunks = split_text_by_byte_length("Hello, world!", 4096);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn test_split_text_long() {
        let long_text = "word ".repeat(1000);
        let chunks = split_text_by_byte_length(&long_text, 100);
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(chunk.len() <= 100);
        }
    }

    #[test]
    fn test_format_timestamp() {
        // 1 second = 10,000,000 ticks (100-nanosecond units)
        assert_eq!(format_timestamp(10_000_000.0), "00:00:01,000");
        assert_eq!(format_timestamp(0.0), "00:00:00,000");
        assert_eq!(format_timestamp(36_000_000_000.0), "01:00:00,000");
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("Hello & Goodbye"), "Hello &amp; Goodbye");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("'quoted'"), "&apos;quoted&apos;");
    }

    #[test]
    fn test_unescape_xml() {
        assert_eq!(unescape_xml("Hello &amp; Goodbye"), "Hello & Goodbye");
        assert_eq!(unescape_xml("&lt;tag&gt;"), "<tag>");
        assert_eq!(unescape_xml("&apos;quoted&apos;"), "'quoted'");
    }

    #[test]
    fn test_create_ssml() {
        let ssml = create_ssml("Hello", "en-US-EmmaNeural", "+0%", "+0%", "+0Hz");
        assert!(ssml.contains("Hello"));
        assert!(ssml.contains("en-US-EmmaNeural"));
        assert!(ssml.contains("<speak"));
        assert!(ssml.contains("</speak>"));
    }

    #[test]
    fn test_remove_incompatible_characters() {
        // Test control characters are replaced
        let result = remove_incompatible_characters("Hello\x00World");
        assert_eq!(result, "Hello World");
        
        let result = remove_incompatible_characters("Hello\x1FWorld");
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_parse_header_value() {
        let headers = b"Path:audio\r\nContent-Type:audio/mpeg\r\n";
        let path = parse_header_value(headers, b"Path");
        assert_eq!(path, Some(b"audio".to_vec()));
        
        let content_type = parse_header_value(headers, b"Content-Type");
        assert_eq!(content_type, Some(b"audio/mpeg".to_vec()));
    }

    #[test]
    fn test_connect_id() {
        let id = connect_id();
        assert_eq!(id.len(), 32); // UUID without dashes
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_sub_maker() {
        let mut sub_maker = SubMaker::new();
        sub_maker.feed(&TTSChunk::WordBoundary {
            offset: 0.0,
            duration: 1_000_000.0,
            text: "Hello".to_string(),
        }).unwrap();
        
        let srt = sub_maker.get_srt();
        assert!(srt.contains("Hello"));
        assert!(srt.contains("00:00:00,000 --> 00:00:00,100"));
    }
}
