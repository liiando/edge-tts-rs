//! Basic example demonstrating edge-tts usage.
//!
//! This example tests:
//! 1. Listing available voices from the Microsoft Edge TTS service
//! 2. Generating audio from text and saving to a file

use edge_tts::{Communicate, list_voices};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test 1: List available voices
    println!("Fetching available voices...");
    let voices = list_voices(None).await?;
    println!("{} voices available", voices.len());
    
    if !voices.is_empty() {
        println!("First voice: {} ({})", voices[0].ShortName, voices[0].Locale);
        
        // Show some English voices
        let english_voices: Vec<_> = voices.iter()
            .filter(|v| v.Locale.starts_with("en-"))
            .take(5)
            .collect();
        println!("\nSample English voices:");
        for voice in english_voices {
            println!("  - {} ({})", voice.ShortName, voice.Gender);
        }
    }

    // Test 2: Generate audio
    println!("\nGenerating audio...");
    let text = "Hello, this is a test of the Rust edge-tts library.";
    let mut communicate = Communicate::builder(text)
        .voice("en-US-AvaNeural")
        .build();
    
    // Save to file
    communicate.save("test_output.mp3", None).await?;
    
    // Get file size
    let metadata = std::fs::metadata("test_output.mp3")?;
    println!("Audio generated successfully: {} bytes", metadata.len());
    println!("Saved to: test_output.mp3");
    
    Ok(())
}
