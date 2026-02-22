//! Subtitle maker for generating subtitles from TTS boundary events.

use crate::communicate::TTSChunk;
use crate::subtitle::srt::Subtitle;

/// Subtitle maker that collects TTS boundary events and generates SRT output
pub struct SubMaker {
    /// Collected subtitle cues
    cues: Vec<Subtitle>,
    /// Type of boundary being processed
    boundary_type: Option<String>,
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
    ///
    /// Only WordBoundary and SentenceBoundary chunks are processed.
    pub fn feed(&mut self, chunk: &TTSChunk) -> Result<(), String> {
        match chunk {
            TTSChunk::WordBoundary {
                offset,
                duration,
                text,
            } => {
                if self.boundary_type.is_none() {
                    self.boundary_type = Some("WordBoundary".to_string());
                }
                self.cues.push(Subtitle::new(
                    self.cues.len() as u32 + 1,
                    (*offset / 10.0) as i64,
                    ((offset + duration) / 10.0) as i64,
                    text.clone(),
                ));
            }
            TTSChunk::SentenceBoundary {
                offset,
                duration,
                text,
            } => {
                if self.boundary_type.is_none() {
                    self.boundary_type = Some("SentenceBoundary".to_string());
                }
                self.cues.push(Subtitle::new(
                    self.cues.len() as u32 + 1,
                    (*offset / 10.0) as i64,
                    ((offset + duration) / 10.0) as i64,
                    text.clone(),
                ));
            }
            _ => return Err("Invalid chunk type for subtitle generation".to_string()),
        }
        Ok(())
    }

    /// Generate SRT format subtitles
    pub fn get_srt(&self) -> String {
        crate::subtitle::srt::compose(&self.cues)
    }

    /// Get the boundary type being used
    pub fn boundary_type(&self) -> Option<&str> {
        self.boundary_type.as_deref()
    }

    /// Get the number of cues collected
    pub fn len(&self) -> usize {
        self.cues.len()
    }

    /// Check if no cues have been collected
    pub fn is_empty(&self) -> bool {
        self.cues.is_empty()
    }

    /// Clear all collected cues
    pub fn clear(&mut self) {
        self.cues.clear();
        self.boundary_type = None;
    }
}

impl Default for SubMaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submaker_word_boundary() {
        let mut submaker = SubMaker::new();
        
        submaker.feed(&TTSChunk::WordBoundary {
            offset: 0.0,
            duration: 100000.0, // 1 second in 100ns units
            text: "Hello".to_string(),
        }).unwrap();
        
        submaker.feed(&TTSChunk::WordBoundary {
            offset: 100000.0,
            duration: 100000.0,
            text: "world".to_string(),
        }).unwrap();
        
        assert_eq!(submaker.len(), 2);
        assert_eq!(submaker.boundary_type(), Some("WordBoundary"));
        
        let srt = submaker.get_srt();
        assert!(srt.contains("Hello"));
        assert!(srt.contains("world"));
    }

    #[test]
    fn test_submaker_invalid_chunk() {
        let mut submaker = SubMaker::new();
        
        let result = submaker.feed(&TTSChunk::Audio {
            data: vec![0, 1, 2, 3],
        });
        
        assert!(result.is_err());
    }
}
