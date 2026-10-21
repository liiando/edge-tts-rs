# edge-tts

A Rust library for Microsoft Edge's online text-to-speech service. This is a port of the Python [edge-tts](https://github.com/rany2/edge-tts) library.

## Features

- **Async/await support** - Built on Tokio for efficient async I/O
- **322+ voices** - Multiple languages and voice styles available
- **Adjustable parameters** - Control rate, volume, and pitch
- **SRT subtitle generation** - Create subtitles from word/sentence boundaries
- **Streaming API** - Process audio chunks as they arrive
- **CLI tool included** - Command-line interface for quick TTS generation

## Installation

The crate is not published on crates.io. Add it to your `Cargo.toml` from Git:

```toml
[dependencies]
edge-tts = { git = "https://github.com/liiando/edge-tts-rs" }
tokio = { version = "1", features = ["full"] }
```

## Library Usage

### Basic Example

Generate speech and save to a file:

```rust
use edge_tts::Communicate;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = "Hello, this is a test of the Rust edge-tts library.";
    
    let mut communicate = Communicate::builder(text)
        .voice("en-US-AvaNeural")
        .build();
    
    communicate.save("output.mp3", None).await?;
    
    Ok(())
}
```

### Streaming Example

Process audio chunks as they arrive:

```rust
use edge_tts::{Communicate, TTSChunk};
use futures_util::StreamExt;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = "Streaming audio example with real-time processing.";
    
    let mut communicate = Communicate::builder(text)
        .voice("en-US-EmmaMultilingualNeural")
        .rate("+0%")
        .volume("+0%")
        .pitch("+0Hz")
        .build();
    
    let mut output_file = File::create("streaming_output.mp3").await?;
    let mut stream = std::pin::pin!(communicate.stream().await?);
    
    while let Some(chunk) = stream.next().await {
        match chunk? {
            TTSChunk::Audio { data } => {
                output_file.write_all(&data).await?;
                println!("Received {} bytes of audio", data.len());
            }
            TTSChunk::WordBoundary { offset, duration, text } => {
                println!("Word: '{}' at {:.2}s", text, offset / 10_000_000.0);
            }
            TTSChunk::SentenceBoundary { offset, duration, text } => {
                println!("Sentence: '{}' at {:.2}s", text, offset / 10_000_000.0);
            }
        }
    }
    
    output_file.flush().await?;
    Ok(())
}
```

### Listing Available Voices

```rust
use edge_tts::{list_voices, VoicesManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple list
    let voices = list_voices(None).await?;
    println!("Total voices: {}", voices.len());
    
    // Using VoicesManager for filtering
    let manager = VoicesManager::new().await?;
    
    // Find English female voices
    let english_female = manager.find(Some("Female"), None, Some("en"));
    for voice in english_female {
        println!("{} - {}", voice.ShortName, voice.FriendlyName);
    }
    
    Ok(())
}
```

## CLI Usage

The library includes a command-line tool for quick TTS generation:

```bash
# Generate audio from text
edge-tts --text "Hello, world!" --write-media output.mp3

# List all available voices
edge-tts --list-voices

# Specify voice and adjust parameters
edge-tts --text "Faster and louder" \
    --voice en-US-AvaNeural \
    --rate "+50%" \
    --volume "+30%" \
    --write-media output.mp3
```

## Available Voices

The library provides access to 322+ voices in multiple languages:

| Language | Example Voice | Gender |
|----------|---------------|--------|
| English (US) | `en-US-AvaNeural` | Female |
| English (US) | `en-US-AndrewNeural` | Male |
| English (GB) | `en-GB-SoniaNeural` | Female |
| French | `fr-FR-DeniseNeural` | Female |
| German | `de-DE-KatjaNeural` | Female |
| Spanish | `es-ES-ElviraNeural` | Female |
| Japanese | `ja-JP-NanamiNeural` | Female |
| Chinese | `zh-CN-XiaoxiaoNeural` | Female |

Use `edge-tts --list-voices` to see all available voices.

## License

This library is a Rust port of [edge-tts](https://github.com/rany2/edge-tts) by rany2, and is distributed under the same license: the GNU Lesser General Public License v3.0 (LGPL-3.0). The LGPL-3.0 supplements the GNU General Public License v3.0, included in [gpl-3.0.txt](gpl-3.0.txt).

`src/subtitle/srt.rs` is ported from `srt_composer.py` of the original project and is under the MIT license (Copyright (c) 2014-2023 Christopher Down, Copyright (c) 2025- rany).

See [LICENSE](LICENSE) for the full texts.

## Author

Liando

## Acknowledgments

- [edge-tts](https://github.com/rany2/edge-tts) - Original Python library
- Microsoft Edge TTS service
