//! Subtitle generation module.
//!
//! This module provides utilities for generating subtitles from TTS metadata.

pub mod srt;
pub mod submaker;

pub use srt::Subtitle;
pub use submaker::SubMaker;
