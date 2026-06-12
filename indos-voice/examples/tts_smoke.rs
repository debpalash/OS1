//! TTS smoke test: voice resolution + piper invocation + WAV output.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut t = indos_voice::TtsEngine::new("en_US-lessac-medium", 1.0);
    t.init().await?;
    t.synthesize_to_file(
        "Good evening. I'm OS 1. It's good to finally have a voice.",
        "/tmp/os1-tts.wav",
    )
    .await?;
    println!("synthesized ok");
    Ok(())
}
