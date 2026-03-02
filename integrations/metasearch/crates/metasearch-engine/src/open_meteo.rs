//! Open-Meteo weather engine implementation.
//!
//! Two-step JSON API:
//! 1. Geocoding: <https://geocoding-api.open-meteo.com/v1/search>
//! 2. Forecast: <https://api.open-meteo.com/v1/forecast>
//!
//!    Website: https://open-meteo.com
//!    Features: Current weather with geocoding, no pagination

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::{MetasearchError, Result},
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use smallvec::smallvec;

pub struct OpenMeteo {
    metadata: EngineMetadata,
    client: Client,
}

impl OpenMeteo {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "open_meteo".to_string().into(),
                display_name: "Open-Meteo".to_string().into(),
                homepage: "https://open-meteo.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled: true,
                timeout_ms: 6000,
                weight: 1.0,
            },
            client,
        }
    }

    /// Map WMO weather code to a human-readable condition string.
    fn wmo_condition(code: i64) -> &'static str {
        match code {
            0 => "Clear sky",
            1 => "Mainly clear",
            2 => "Partly cloudy",
            3 => "Overcast",
            45 => "Fog",
            48 => "Depositing rime fog",
            51 => "Light drizzle",
            53 => "Moderate drizzle",
            55 => "Dense drizzle",
            56 => "Light freezing drizzle",
            57 => "Dense freezing drizzle",
            61 => "Slight rain",
            63 => "Moderate rain",
            65 => "Heavy rain",
            66 => "Light freezing rain",
            67 => "Heavy freezing rain",
            71 => "Slight snow fall",
            73 => "Moderate snow fall",
            75 => "Heavy snow fall",
            77 => "Snow grains",
            80 => "Slight rain showers",
            81 => "Moderate rain showers",
            82 => "Violent rain showers",
            85 => "Slight snow showers",
            86 => "Heavy snow showers",
            95 => "Thunderstorm",
            96 => "Thunderstorm with slight hail",
            99 => "Thunderstorm with heavy hail",
            _ => "Unknown",
        }
    }
}

#[async_trait]
impl SearchEngine for OpenMeteo {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        // Step 1: Geocode the query to get latitude/longitude
        let geocode_url = format!(
            "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
            urlencoding::encode(&query.query)
        );

        let geocode_resp = self
            .client
            .get(&geocode_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let geocode_data: serde_json::Value = geocode_resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(format!("Geocoding JSON error: {}", e)))?;

        // Extract first geocoding result
        let location = match geocode_data
            .get("results")
            .and_then(|r| r.as_array())
            .and_then(|a| a.first())
        {
            Some(loc) => loc,
            None => return Ok(Vec::new()), // Location not found
        };

        let latitude = location
            .get("latitude")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| MetasearchError::ParseError("Missing latitude".to_string()))?;
        let longitude = location
            .get("longitude")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| MetasearchError::ParseError("Missing longitude".to_string()))?;
        let location_name = location
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&query.query);
        let country = location
            .get("country")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Step 2: Fetch current weather forecast
        let forecast_url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,apparent_temperature,relative_humidity_2m,cloud_cover,pressure_msl,wind_speed_10m,wind_direction_10m,weather_code&timezone=auto&format=json",
            latitude, longitude
        );

        let forecast_resp = self
            .client
            .get(&forecast_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
            .map_err(|e| MetasearchError::HttpError(e.to_string()))?;

        let forecast_data: serde_json::Value = forecast_resp
            .json()
            .await
            .map_err(|e| MetasearchError::ParseError(format!("Forecast JSON error: {}", e)))?;

        let current = match forecast_data.get("current") {
            Some(c) => c,
            None => return Ok(Vec::new()),
        };

        let temperature = current
            .get("temperature_2m")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let apparent_temp = current
            .get("apparent_temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let humidity = current
            .get("relative_humidity_2m")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let cloud_cover = current
            .get("cloud_cover")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let pressure = current
            .get("pressure_msl")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let wind_speed = current
            .get("wind_speed_10m")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let wind_direction = current
            .get("wind_direction_10m")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let weather_code = current
            .get("weather_code")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let condition = Self::wmo_condition(weather_code);

        let display_location = if country.is_empty() {
            location_name.to_string()
        } else {
            format!("{}, {}", location_name, country)
        };

        let title = format!(
            "Weather in {}: {:.1}°C — {}",
            display_location, temperature, condition
        );
        let content = format!(
            "Feels like {:.1}°C | Wind: {:.1} km/h ({}°) | Humidity: {:.0}% | Cloud cover: {:.0}% | Pressure: {:.0} hPa",
            apparent_temp, wind_speed, wind_direction, humidity, cloud_cover, pressure
        );
        let result_url = format!(
            "https://open-meteo.com/en/docs#latitude={}&longitude={}",
            latitude, longitude
        );

        let mut results = Vec::new();
        let mut r = SearchResult::new(&title, &result_url, &content, "open_meteo");
        r.engine_rank = 1;
        r.category = SearchCategory::General.to_string();
        results.push(r);

        Ok(results)
    }
}
