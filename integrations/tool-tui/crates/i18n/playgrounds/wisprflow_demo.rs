//! Wispr Flow demo - record 10 seconds and enhance

use dx_i18n::wisprflow::WisprFlow;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎙️  Wispr Flow Demo\n");
    println!("Recording for 10 seconds...\n");

    // Create Wispr Flow processor
    let mut flow = WisprFlow::new(None, None)?;

    // Add custom sound mappings
    flow.add_sound_mapping(" um ", " ");
    flow.add_sound_mapping(" uh ", " ");
    flow.add_sound_mapping(" like ", " ");
    flow.add_sound_mapping(" you know ", " ");

    // Process microphone input
    let result = flow.process_microphone(10).await?;

    // Display results
    println!("📝 Raw Transcript:");
    println!("{}\n", result.raw_transcript);

    println!("✨ Enhanced Text:");
    println!("{}\n", result.enhanced_text);

    println!("⏱️  Performance:");
    println!("  STT Time: {}ms", result.stt_time_ms);
    println!("  Enhancement Time: {}ms", result.enhancement_time_ms);
    println!("  Total Time: {}ms", result.total_time_ms);

    Ok(())
}
