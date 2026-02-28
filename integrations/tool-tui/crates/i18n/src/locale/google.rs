//! Google Translator implementation

use crate::error::{I18nError, Result};
use crate::locale::base::Translator;
use crate::locale::constants::{GOOGLE_LANGUAGES, GOOGLE_TRANSLATE_URL};
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};

/// Google Translator
pub struct GoogleTranslator {
    client: Client,
    source: String,
    target: String,
}

impl GoogleTranslator {
    /// Create a new Google Translator
    ///
    /// # Arguments
    /// * `source` - Source language (use "auto" for auto-detection)
    /// * `target` - Target language
    ///
    /// # Example
    /// ```no_run
    /// use i18n::locale::GoogleTranslator;
    ///
    /// let translator = GoogleTranslator::new("en", "es").unwrap();
    /// ```
    pub fn new(source: &str, target: &str) -> Result<Self> {
        let source_code = Self::map_language_to_code(source)?;
        let target_code = Self::map_language_to_code(target)?;

        Ok(Self {
            client: Client::new(),
            source: source_code,
            target: target_code,
        })
    }

    fn map_language_to_code(language: &str) -> Result<String> {
        if language == "auto" {
            return Ok("auto".to_string());
        }

        // Check if it's already a code
        if GOOGLE_LANGUAGES.values().any(|&v| v == language) {
            return Ok(language.to_string());
        }

        // Check if it's a language name
        if let Some(&code) = GOOGLE_LANGUAGES.get(language.to_lowercase().as_str()) {
            return Ok(code.to_string());
        }

        Err(I18nError::LanguageNotSupported(language.to_string()))
    }

    fn same_source_target(&self) -> bool {
        self.source == self.target
    }
}

#[async_trait]
impl Translator for GoogleTranslator {
    async fn translate(&self, text: &str) -> Result<String> {
        let text = text.trim();

        if text.is_empty() {
            return Ok(text.to_string());
        }

        if self.same_source_target() {
            return Ok(text.to_string());
        }

        if text.len() > 5000 {
            return Err(I18nError::InvalidLength { min: 1, max: 5000 });
        }

        let response = self
            .client
            .get(GOOGLE_TRANSLATE_URL)
            .query(&[
                ("tl", self.target.as_str()),
                ("sl", self.source.as_str()),
                ("q", text),
            ])
            .send()
            .await?;

        if response.status() == 429 {
            return Err(I18nError::TooManyRequests(
                "Too many requests to Google Translate. Please try again later.".to_string(),
            ));
        }

        let html = response.text().await?;
        let document = Html::parse_document(&html);

        // Try primary selector
        // Note: Selector::parse() only fails for invalid CSS selectors, which are compile-time constants here
        let selector = Selector::parse("div.t0").expect("static CSS selector 'div.t0' is valid");
        if let Some(element) = document.select(&selector).next() {
            let translation = element.text().collect::<String>();
            if !translation.is_empty() {
                return Ok(translation);
            }
        }

        // Try alternative selector
        let alt_selector = Selector::parse("div.result-container")
            .expect("static CSS selector 'div.result-container' is valid");
        if let Some(element) = document.select(&alt_selector).next() {
            let translation = element.text().collect::<String>();
            if !translation.is_empty() {
                return Ok(translation);
            }
        }

        Err(I18nError::TranslationNotFound(text.to_string()))
    }

    fn get_supported_languages(&self) -> Vec<&'static str> {
        GOOGLE_LANGUAGES.keys().copied().collect()
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn target(&self) -> &str {
        &self.target
    }
}
