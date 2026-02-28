//! Feature testing for i18n crate

use dx_i18n::locale::{GoogleTranslator, Translator};
use dx_i18n::tts::{GoogleTTS, TextToSpeech};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing i18n Features\n");

    // Test 1: Auto language detection
    println!("Test 1: Auto-detect language");
    let translator = GoogleTranslator::new("auto", "en")?;
    let result = translator.translate("Bonjour le monde").await?;
    println!("✓ French → English: {}\n", result);

    // Test 2: Multiple languages
    println!("Test 2: Multiple target languages");
    let langs = vec![("fr", "French"), ("de", "German"), ("ja", "Japanese")];
    for (code, name) in langs {
        let t = GoogleTranslator::new("en", code)?;
        let result = t.translate("Hello").await?;
        println!("✓ English → {}: {}", name, result);
    }

    // Test 3: Batch translation
    println!("\nTest 3: Batch translation");
    let translator = GoogleTranslator::new("en", "es")?;
    let texts = vec!["Good morning", "Good night", "Thank you"];
    let results = translator.translate_batch(&texts).await?;
    for (orig, trans) in texts.iter().zip(results.iter()) {
        println!("✓ {} → {}", orig, trans);
    }

    // Test 4: Empty text handling
    println!("\nTest 4: Edge cases");
    let result = translator.translate("").await?;
    println!("✓ Empty string: '{}'", result);

    // Test 5: Language support check
    println!("\nTest 5: Language support");
    println!("✓ Supports 'es': {}", translator.is_language_supported("es"));
    println!("✓ Supports 'xyz': {}", translator.is_language_supported("xyz"));

    // Test 6: TTS with different text
    println!("\nTest 6: TTS synthesis");
    let tts = GoogleTTS::new("en");
    let audio = tts.synthesize("Testing one two three").await?;
    println!("✓ Generated {} bytes of audio", audio.len());

    println!("\n✅ All tests passed!");
    Ok(())
}
