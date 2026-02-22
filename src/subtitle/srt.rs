//! SRT (SubRip) subtitle format utilities.

use std::cmp::Ordering;

/// A single subtitle entry
#[derive(Debug, Clone)]
pub struct Subtitle {
    /// Subtitle index (1-based)
    pub index: u32,
    /// Start time in milliseconds
    pub start_ms: i64,
    /// End time in milliseconds
    pub end_ms: i64,
    /// Subtitle text content
    pub content: String,
}

impl Subtitle {
    /// Create a new subtitle
    pub fn new(index: u32, start_ms: i64, end_ms: i64, content: impl Into<String>) -> Self {
        Self {
            index,
            start_ms,
            end_ms,
            content: content.into(),
        }
    }

    /// Convert to SRT format string
    pub fn to_srt(&self) -> String {
        format!(
            "{}\n{} --> {}\n{}\n\n",
            self.index,
            ms_to_srt_timestamp(self.start_ms),
            ms_to_srt_timestamp(self.end_ms),
            self.content
        )
    }
}

impl PartialEq for Subtitle {
    fn eq(&self, other: &Self) -> bool {
        self.start_ms == other.start_ms && self.end_ms == other.end_ms
    }
}

impl Eq for Subtitle {}

impl PartialOrd for Subtitle {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Subtitle {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start_ms
            .cmp(&other.start_ms)
            .then_with(|| self.end_ms.cmp(&other.end_ms))
            .then_with(|| self.index.cmp(&other.index))
    }
}

/// Convert milliseconds to SRT timestamp format (HH:MM:SS,mmm)
fn ms_to_srt_timestamp(ms: i64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    let millis = ms % 1000;

    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
}

/// Compose multiple subtitles into an SRT file
pub fn compose(subtitles: &[Subtitle]) -> String {
    let mut sorted: Vec<&Subtitle> = subtitles.iter().collect();
    sorted.sort();

    sorted
        .iter()
        .enumerate()
        .map(|(i, s)| Subtitle {
            index: i as u32 + 1,
            ..(*s).clone()
        })
        .map(|s| s.to_srt())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ms_to_srt_timestamp() {
        assert_eq!(ms_to_srt_timestamp(0), "00:00:00,000");
        assert_eq!(ms_to_srt_timestamp(1000), "00:00:01,000");
        assert_eq!(ms_to_srt_timestamp(60000), "00:01:00,000");
        assert_eq!(ms_to_srt_timestamp(3600000), "01:00:00,000");
        assert_eq!(ms_to_srt_timestamp(3661500), "01:01:01,500");
    }

    #[test]
    fn test_subtitle_to_srt() {
        let sub = Subtitle::new(1, 0, 1000, "Hello");
        assert_eq!(sub.to_srt(), "1\n00:00:00,000 --> 00:00:01,000\nHello\n\n");
    }

    #[test]
    fn test_compose() {
        let subtitles = vec![
            Subtitle::new(2, 2000, 3000, "World"),
            Subtitle::new(1, 0, 1000, "Hello"),
        ];
        
        let srt = compose(&subtitles);
        assert!(srt.starts_with("1\n00:00:00,000 --> 00:00:01,000\nHello"));
        assert!(srt.contains("2\n00:00:02,000 --> 00:00:03,000\nWorld"));
    }
}
