//! Microsoft Translator implementation

use crate::error::{I18nError, Result};
use crate::locale::base::Translator;
use crate::locale::constants::MICROSOFT_TRANSLATE_URL;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Serialize)]
struct TranslationRequest {
    #[serde(rename = "Text")]
    text: String,
}

#[derive(Deserialize)]
struct TranslationResponse {
    translations: Vec<Translation>,
}

#[derive(Deserialize)]
struct Translation {
    text: String,
}

/// Microsoft Translator
pub struct MicrosoftTranslator {
    client: Client,
    api_key: String,
    region: Option<String>,
    source: String,
    target: String,
}

impl MicrosoftTranslator {
    /// Create a new Microsoft Translator
    ///
    /// # Arguments
    /// * `source` - Source language (use "auto" for auto-detection)
    /// * `target` - Target language
    /// * `api_key` - Optional API key (will use MICROSOFT_API_KEY env var if not provided)
    /// * `region` - Optional Azure region
    ///
    /// # Example
    /// ```no_run
    /// use i18n::locale::MicrosoftTranslator;
    ///
    /// let translator = MicrosoftTranslator::new("en", "es", None, None).unwrap();
    /// ```
    pub fn new(
        source: &str,
        target: &str,
        api_key: Option<String>,
        region: Option<String>,
    ) -> Result<Self> {
        let api_key = api_key.or_else(|| env::var("MICROSOFT_API_KEY").ok()).ok_or_else(|| {
            I18nError::ApiKeyRequired(
                "Microsoft Translator".to_string(),
                "MICROSOFT_API_KEY".to_string(),
            )
        })?;

        Ok(Self {
            client: Client::new(),
            api_key,
            region,
            source: source.to_string(),
            target: target.to_string(),
        })
    }
}

#[async_trait]
impl Translator for MicrosoftTranslator {
    async fn translate(&self, text: &str) -> Result<String> {
        let text = text.trim();

        if text.is_empty() {
            return Ok(text.to_string());
        }

        let mut headers = reqwest::header::HeaderMap::new();
        // Note: These header value parses should never fail for valid ASCII strings
        // Using map_err to convert parse errors to I18nError for proper error handling
        headers.insert(
            "Ocp-Apim-Subscription-Key",
            self.api_key.parse().map_err(|_| I18nError::ServerError {
                code: 0,
                message: "Invalid API key format for header".to_string(),
            })?,
        );
        headers.insert(
            "Content-Type",
            "application/json".parse().expect("static Content-Type header value is valid"),
        );

        if let Some(ref region) = self.region {
            headers.insert(
                "Ocp-Apim-Subscription-Region",
                region.parse().map_err(|_| I18nError::ServerError {
                    code: 0,
                    message: "Invalid region format for header".to_string(),
                })?,
            );
        }

        let body = vec![TranslationRequest {
            text: text.to_string(),
        }];

        let response = self
            .client
            .post(MICROSOFT_TRANSLATE_URL)
            .headers(headers)
            .query(&[("from", self.source.as_str()), ("to", self.target.as_str())])
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status_code = response.status().as_u16();
            let error_text = response.text().await?;
            return Err(I18nError::ServerError {
                code: status_code,
                message: error_text,
            });
        }

        let translations: Vec<TranslationResponse> = response.json().await?;

        if let Some(first) = translations.first()
            && let Some(translation) = first.translations.first()
        {
            return Ok(translation.text.clone());
        }

        Err(I18nError::TranslationNotFound(text.to_string()))
    }

    fn get_supported_languages(&self) -> Vec<&'static str> {
        // Microsoft supports many languages - returning a subset
        vec![
            "en", "es", "fr", "de", "it", "ja", "ko", "pt", "ru", "zh-Hans", "zh-Hant",
        ]
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn target(&self) -> &str {
        &self.target
    }
}
