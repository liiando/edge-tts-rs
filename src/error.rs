//! Error types for the edge-tts library.

use thiserror::Error;

/// Main error type for edge-tts operations
#[derive(Error, Debug)]
pub enum Error {
    /// Unknown response from server
    #[error("Unknown response from server: {0}")]
    UnknownResponse(String),

    /// Unexpected response from server
    #[error("Unexpected response from server: {0}")]
    UnexpectedResponse(String),

    /// No audio received from server
    #[error("No audio received from server")]
    NoAudioReceived,

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Clock skew adjustment error
    #[error("Clock skew adjustment error: {0}")]
    SkewAdjustment(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Stream already consumed
    #[error("Stream can only be consumed once")]
    StreamConsumed,
}

/// Result type alias for edge-tts operations
pub type Result<T> = std::result::Result<T, Error>;
