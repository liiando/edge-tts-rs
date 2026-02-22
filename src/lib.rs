//! Edge-TTS - Rust port of the Python edge-tts library
//!
//! A Microsoft Edge TTS client library for Rust.

pub mod communicate;
pub mod config;
pub mod constants;
pub mod drm;
pub mod error;
pub mod voices;
pub mod subtitle;

pub use communicate::{Communicate, SubMaker, TTSChunk};
pub use config::{TTSConfig, BoundaryType};
pub use error::{Error, Result};
pub use voices::{Voice, VoiceTag, VoicesManager, list_voices};
pub use subtitle::{Subtitle, SubMaker as SubtitleMaker};

/// Library version
pub const VERSION: &str = "0.1.0";
