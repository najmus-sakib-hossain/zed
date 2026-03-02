//! DuckDuckGo Weather — weather forecast via DDG's spice API (JSONP)
//!
//! Queries `https://duckduckgo.com/js/spice/forecast/{query}/{lang}`
//! and parses the Apple WeatherKit JSON response.
//!
//! Reference: <https://github.com/searxng/searxng/blob/master/searx/engines/duckduckgo_weather.py>

use async_trait::async_trait;
use reqwest::Client;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

pub struct DuckDuckGoWeather {
    client: Client,
}

impl DuckDuckGoWeather {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

fn condition_label(code: &str) -> &str {
    match code {
        "Clear" | "MostlyClear" => "Clear sky",
        "Cloudy" => "Cloudy",
        "MostlyCloudy" | "PartlyCloudy" => "Partly cloudy",
        "Foggy" | "Haze" | "Smoky" | "BlowingDust" => "Fog",
        "Drizzle" => "Light rain",
        "Rain" | "SunShowers" => "Rain",
        "HeavyRain" | "Hail" => "Heavy rain",
        "IsolatedThunderstorms" | "Thunderstorms" => "Thunderstorms",
        "ScatteredThunderstorms" | "StrongStorms" => "Heavy thunderstorms",
        "Flurries" | "SunFlurries" | "Snow" => "Light snow",
        "HeavySnow" | "Blizzard" | "BlowingSnow" => "Heavy snow",
        "Sleet" | "WintryMix" | "FreezingRain" => "Sleet",
        "FreezingDrizzle" => "Light sleet",
        "Breezy" | "Windy" => "Windy",
        "Hot" | "Frigid" => "Extreme temperature",
        "Hurricane" | "TropicalStorm" => "Storm",
        _ => code,
    }
}

#[async_trait]
impl SearchEngine for DuckDuckGoWeather {
    fn metadata(&self) -> EngineMetadata {
        EngineMetadata {
            name: "DuckDuckGoWeather".to_string().into(),
            display_name: "DuckDuckGoWeather".to_string().into(),
            homepage: "https://duckduckgo.com".to_string().into(),
            categories: smallvec![SearchCategory::General],
            enabled: true,
            timeout_ms: 5000,
            weight: 1.0,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "https://duckduckgo.com/js/spice/forecast/{}/en",
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("DDG Weather request error: {e}")))?;

        let text = resp
            .text()
            .await
            .map_err(|e| MetasearchError::Engine(format!("DDG Weather read error: {e}")))?;

        // Check for empty response
        if text.trim() == "ddg_spice_forecast();" {
            return Ok(vec![]);
        }

        // Parse JSONP: strip first line (callback) and last line (closing)
        let json_str = if let Some(start) = text.find('\n') {
            if let Some(end) = text.rfind('\n') {
                if end > start {
                    &text[start + 1..end - 2]
                } else {
                    return Ok(vec![]);
                }
            } else {
                return Ok(vec![]);
            }
        } else {
            return Ok(vec![]);
        };

        let json: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| MetasearchError::Engine(format!("DDG Weather JSON error: {e}")))?;

        let mut results = Vec::new();

        // Current weather
        if let Some(current) = json.get("currentWeather") {
            let temp = current
                .get("temperature")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let feels_like = current
                .get("temperatureApparent")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let condition_code = current
                .get("conditionCode")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let humidity = current
                .get("humidity")
                .and_then(|v| v.as_f64())
                .map(|h| h * 100.0)
                .unwrap_or(0.0);
            let wind_speed = current
                .get("windSpeed")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let pressure = current
                .get("pressure")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let condition = condition_label(condition_code);

            let mut result = SearchResult::new(
                format!(
                    "Weather for {}: {:.1}°C — {}",
                    query.query, temp, condition
                ),
                format!("https://duckduckgo.com/?q=weather+{}", urlencoding::encode(&query.query)),
                format!(
                    "Temperature: {:.1}°C (feels like {:.1}°C) | Condition: {} | Humidity: {:.0}% | Wind: {:.1} km/h | Pressure: {:.0} hPa",
                    temp, feels_like, condition, humidity, wind_speed, pressure
                ),
                "duckduckgo_weather",
            );
            result.engine_rank = 1;
            results.push(result);
        }

        // Daily forecast (from forecastDaily if available)
        if let Some(daily) = json
            .get("forecastDaily")
            .and_then(|fd| fd.get("days"))
            .and_then(|d| d.as_array())
        {
            for (i, day) in daily.iter().take(5).enumerate() {
                let date = day
                    .get("forecastStart")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let date_short = if date.len() >= 10 { &date[..10] } else { date };

                let temp_max = day
                    .get("temperatureMax")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let temp_min = day
                    .get("temperatureMin")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let condition_code = day
                    .get("conditionCode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                let mut result = SearchResult::new(
                    format!(
                        "{}: {:.0}°C / {:.0}°C — {}",
                        date_short,
                        temp_max,
                        temp_min,
                        condition_label(condition_code)
                    ),
                    format!(
                        "https://duckduckgo.com/?q=weather+{}",
                        urlencoding::encode(&query.query)
                    ),
                    format!(
                        "High: {:.1}°C | Low: {:.1}°C | {}",
                        temp_max,
                        temp_min,
                        condition_label(condition_code)
                    ),
                    "duckduckgo_weather",
                );
                result.engine_rank = (i + 2) as u32;
                results.push(result);
            }
        }

        Ok(results)
    }
}
