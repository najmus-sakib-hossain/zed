//! Google TTS implementation

use crate::error::{I18nError, Result};
use crate::tts::base::TextToSpeech;
use async_trait::async_trait;

/// Google Text-to-Speech
pub struct GoogleTTS {
    lang: String,
}

impl GoogleTTS {
    /// Create a new Google TTS instance
    ///
    /// # Arguments
    /// * `lang` - Language code (e.g., "en", "es", "fr")
    ///
    /// # Example
    /// ```no_run
    /// use i18n::tts::GoogleTTS;
    ///
    /// let tts = GoogleTTS::new("en");
    /// ```
    pub fn new(lang: &str) -> Self {
        Self {
            lang: lang.to_string(),
        }
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        let text = text.trim();

        if text.len() <= 100 {
            return vec![text.to_string()];
        }

        // Simple tokenization by sentence
        let mut tokens = Vec::new();
        let mut current = String::new();

        for sentence in text.split(&['.', '!', '?'][..]) {
            let sentence = sentence.trim();
            if sentence.is_empty() {
                continue;
            }

            if current.len() + sentence.len() + 1 > 100 && !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }

            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(sentence);
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        tokens
    }

    async fn synthesize_part(&self, text: &str) -> Result<Vec<u8>> {
        // Use Google Translate TTS public API (no authentication required)
        let url = format!(
            "https://translate.google.com/translate_tts?ie=UTF-8&tl={}&client=tw-ob&q={}",
            self.lang,
            urlencoding::encode(text)
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .header("Referer", "https://translate.google.com/")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(I18nError::Other(format!("Google TTS API error: {}", response.status())));
        }

        let audio_data = response.bytes().await?;
        Ok(audio_data.to_vec())
    }
}

#[async_trait]
impl TextToSpeech for GoogleTTS {
    async fn synthesize(&self, text: &str) -> Result<Vec<u8>> {
        if text.trim().is_empty() {
            return Err(I18nError::Other("No text to speak".to_string()));
        }

        let parts = self.tokenize(text);
        let mut audio_data = Vec::new();

        for part in parts {
            let part_audio = self.synthesize_part(&part).await?;
            audio_data.extend(part_audio);
        }

        Ok(audio_data)
    }

    fn get_supported_languages(&self) -> Vec<&'static str> {
        vec![
            "af", "sq", "am", "ar", "hy", "az", "eu", "be", "bn", "bs", "bg", "ca", "ceb", "ny",
            "zh-CN", "zh-TW", "co", "hr", "cs", "da", "nl", "en", "eo", "et", "tl", "fi", "fr",
            "gl", "ka", "de", "el", "gu", "ht", "ha", "haw", "iw", "hi", "hmn", "hu", "is", "ig",
            "id", "ga", "it", "ja", "jw", "kn", "kk", "km", "ko", "ku", "ky", "lo", "la", "lv",
            "lt", "lb", "mk", "mg", "ms", "ml", "mt", "mi", "mr", "mn", "my", "ne", "no", "ps",
            "fa", "pl", "pt", "pa", "ro", "ru", "sm", "gd", "sr", "st", "sn", "sd", "si", "sk",
            "sl", "so", "es", "su", "sw", "sv", "tg", "ta", "te", "th", "tr", "uk", "ur", "uz",
            "vi", "cy", "xh", "yi", "yo", "zu",
            // Additional languages supported by Google TTS
            "fo", "ab", "ace", "ach", "aa", "alz", "as", "av", "awa", "ay", "ban", "bal", "bm",
            "bci", "ba", "btx", "bts", "bbc", "bem", "bew", "bho", "bik", "br", "bua", "yue", "ch",
            "ce", "chk", "cv", "crh", "prs", "dv", "din", "doi", "dov", "dyu", "dz", "ee", "fj",
            "fil", "fon", "fr-CA", "fy", "fur", "ff", "gaa", "gn", "cnh", "hil", "hrx", "iba",
            "ilo", "iu", "jam", "kac", "kl", "kr", "pam", "kha", "cgg", "kg", "rw", "ktu", "trp",
            "kv", "kok", "kri", "ckb", "ltg", "lij", "li", "ln", "lmo", "lg", "luo", "mad", "mai",
            "mak", "mam", "gv", "mh", "mwr", "mfe", "mhr", "mni", "min", "lus", "nhe", "ndc", "nr",
            "new", "nqo", "nus", "oc", "or", "os", "pag", "pap", "pt-BR", "pt-PT", "qu", "kek",
            "rom", "rn", "se", "sg", "sa", "sat", "nso", "crs", "shn", "scn", "szl", "sus", "ss",
            "ty", "ber", "tet", "bo", "tiv", "tpi", "to", "lua", "ts", "tn", "tcy", "tum", "tk",
            "tyv", "tw", "udm", "ug", "ve", "vec", "war", "wo", "sah", "yua", "zap",
        ]
    }
}
