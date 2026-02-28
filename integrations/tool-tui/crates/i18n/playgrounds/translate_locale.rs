//! Example: Translating JSON localization files

use i18n::locale::{GoogleTranslator, Translator};
use serde_json::{json, Value};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample i18n JSON structure (English)
    let en_locale = json!({
        "app": {
            "title": "My Application",
            "welcome": "Welcome to our app!",
            "logout": "Logout",
            "settings": "Settings"
        },
        "messages": {
            "success": "Operation completed successfully",
            "error": "An error occurred",
            "confirm": "Are you sure?"
        },
        "forms": {
            "name": "Name",
            "email": "Email",
            "submit": "Submit",
            "cancel": "Cancel"
        }
    });

    println!("Original English locale:");
    println!("{}\n", serde_json::to_string_pretty(&en_locale)?);

    // Translate to multiple languages
    let target_languages = vec![
        ("es", "Spanish"),
        ("fr", "French"),
        ("de", "German"),
    ];

    for (lang_code, lang_name) in target_languages {
        println!("Translating to {}...", lang_name);
        let translator = GoogleTranslator::new("en", lang_code)?;
        
        let translated = translate_json_values(&en_locale, &translator).await?;
        
        let output_file = format!("locale_{}.json", lang_code);
        std::fs::write(&output_file, serde_json::to_string_pretty(&translated)?)?;
        
        println!("✓ Saved to: {}\n", output_file);
    }

    Ok(())
}

async fn translate_json_values(
    value: &Value,
    translator: &GoogleTranslator,
) -> Result<Value, Box<dyn std::error::Error>> {
    match value {
        Value::Object(map) => {
            let mut result = serde_json::Map::new();
            for (key, val) in map {
                result.insert(
                    key.clone(),
                    translate_json_values(val, translator).await?,
                );
            }
            Ok(Value::Object(result))
        }
        Value::String(s) => {
            let translated = translator.translate(s).await?;
            Ok(Value::String(translated))
        }
        Value::Array(arr) => {
            let mut result = Vec::new();
            for item in arr {
                result.push(translate_json_values(item, translator).await?);
            }
            Ok(Value::Array(result))
        }
        other => Ok(other.clone()),
    }
}
