//! Edge-TTS CLI - Command-line interface for Microsoft Edge TTS

use edge_tts::{Communicate, SubMaker, list_voices, Voice};
use std::path::PathBuf;

/// Edge-TTS - Text-to-speech using Microsoft Edge TTS
#[derive(clap::Parser)]
#[command(name = "edge-tts")]
#[command(version)]
#[command(about = "Text-to-speech using Microsoft Edge TTS", long_about = None)]
struct Args {
    /// Text to synthesize
    #[arg(short, long, group = "input")]
    text: Option<String>,

    /// File containing text to synthesize
    #[arg(short, long, group = "input")]
    file: Option<PathBuf>,

    /// Voice to use for synthesis
    #[arg(short = 'v', long, default_value = "en-US-EmmaMultilingualNeural")]
    voice: String,

    /// List available voices
    #[arg(short, long)]
    list_voices: bool,

    /// Speech rate (e.g., +0%, +50%, -20%)
    #[arg(long, default_value = "+0%")]
    rate: String,

    /// Volume (e.g., +0%, +50%, -20%)
    #[arg(long, default_value = "+0%")]
    volume: String,

    /// Pitch (e.g., +0Hz, +50Hz, -20Hz)
    #[arg(long, default_value = "+0Hz")]
    pitch: String,

    /// Output media file path
    #[arg(short, long)]
    write_media: Option<PathBuf>,

    /// Output subtitles file path
    #[arg(long)]
    write_subtitles: Option<PathBuf>,

    /// Proxy URL
    #[arg(long)]
    proxy: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = <Args as clap::Parser>::parse();

    // Initialize logging
    tracing_subscriber::fmt::init();

    // List voices mode
    if args.list_voices {
        let voices = list_voices(args.proxy.as_deref()).await?;
        print_voices(&voices);
        return Ok(());
    }

    // Get input text
    let text = match (args.text, args.file) {
        (Some(t), _) => t,
        (None, Some(f)) => std::fs::read_to_string(f)?,
        (None, None) => {
            eprintln!("Error: Either --text or --file is required");
            std::process::exit(1);
        }
    };

    // Build Communicate instance
    let mut builder = Communicate::builder(&text)
        .voice(&args.voice)
        .rate(&args.rate)
        .volume(&args.volume)
        .pitch(&args.pitch);

    if let Some(proxy) = &args.proxy {
        builder = builder.proxy(proxy);
    }

    let mut communicate = builder.build();

    // Handle output
    match (args.write_media, args.write_subtitles) {
        (Some(audio_path), Some(sub_path)) => {
            communicate.save(&audio_path, Some(&sub_path)).await?;
        }
        (Some(audio_path), None) => {
            communicate.save(&audio_path, None::<&PathBuf>).await?;
        }
        (None, Some(sub_path)) => {
            // Stream and collect subtitles
            use futures_util::StreamExt;
            let mut submaker = SubMaker::new();
            let mut stream = std::pin::pin!(communicate.stream().await?);
            
            while let Some(chunk) = stream.next().await {
                if let Ok(chunk) = chunk {
                    if matches!(chunk, edge_tts::TTSChunk::WordBoundary { .. } | 
                                      edge_tts::TTSChunk::SentenceBoundary { .. }) {
                        let _ = submaker.feed(&chunk);
                    }
                }
            }
            
            std::fs::write(&sub_path, submaker.get_srt())?;
        }
        (None, None) => {
            // Stream to stdout
            use futures_util::StreamExt;
            use tokio::io::{AsyncWriteExt, stdout};
            
            let mut stdout = stdout();
            let mut stream = std::pin::pin!(communicate.stream().await?);
            
            while let Some(chunk) = stream.next().await {
                if let Ok(edge_tts::TTSChunk::Audio { data }) = chunk {
                    stdout.write_all(&data).await?;
                }
            }
            stdout.flush().await?;
        }
    }

    Ok(())
}

/// Print voices in a formatted table
fn print_voices(voices: &[Voice]) {
    use tabled::{Table, Tabled, settings::Style};

    #[derive(Tabled)]
    struct VoiceRow {
        #[tabled(rename = "Name")]
        name: String,
        #[tabled(rename = "Gender")]
        gender: String,
        #[tabled(rename = "Locale")]
        locale: String,
    }

    let rows: Vec<VoiceRow> = voices
        .iter()
        .map(|v| VoiceRow {
            name: v.short_name.clone(),
            gender: v.gender.clone(),
            locale: v.locale.clone(),
        })
        .collect();

    let table = Table::new(rows).with(Style::rounded()).to_string();
    println!("{}", table);
}
