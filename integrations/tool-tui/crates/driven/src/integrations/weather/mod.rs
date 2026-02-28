//! # Weather Integration
//!
//! Weather data via OpenWeatherMap and other providers.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::weather::{WeatherClient, WeatherConfig};
//!
//! let config = WeatherConfig::from_file("~/.dx/config/weather.sr")?;
//! let client = WeatherClient::new(&config)?;
//!
//! // Get current weather
//! let weather = client.current("San Francisco").await?;
//!
//! // Get forecast
//! let forecast = client.forecast("San Francisco", 5).await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};

/// Weather configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConfig {
    /// Whether weather integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// OpenWeatherMap API key
    #[serde(default)]
    pub openweathermap_key: String,
    /// Default location
    pub default_location: Option<String>,
    /// Units (metric, imperial, kelvin)
    #[serde(default)]
    pub units: WeatherUnits,
    /// Language code
    #[serde(default = "default_lang")]
    pub language: String,
}

fn default_true() -> bool {
    true
}

fn default_lang() -> String {
    "en".to_string()
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            openweathermap_key: String::new(),
            default_location: None,
            units: WeatherUnits::Metric,
            language: default_lang(),
        }
    }
}

impl WeatherConfig {
    /// Load from .sr config file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        Self::parse_sr(&content)
    }

    fn parse_sr(_content: &str) -> Result<Self> {
        Ok(Self::default())
    }

    /// Resolve environment variables
    pub fn resolve_env_vars(&mut self) {
        if self.openweathermap_key.is_empty() || self.openweathermap_key.starts_with('$') {
            self.openweathermap_key = std::env::var("OPENWEATHERMAP_API_KEY")
                .or_else(|_| std::env::var("OWM_API_KEY"))
                .unwrap_or_default();
        }
    }
}

/// Weather units
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WeatherUnits {
    #[default]
    Metric,
    Imperial,
    Kelvin,
}

impl WeatherUnits {
    fn to_owm_string(&self) -> &str {
        match self {
            WeatherUnits::Metric => "metric",
            WeatherUnits::Imperial => "imperial",
            WeatherUnits::Kelvin => "standard",
        }
    }

    fn temp_symbol(&self) -> &str {
        match self {
            WeatherUnits::Metric => "°C",
            WeatherUnits::Imperial => "°F",
            WeatherUnits::Kelvin => "K",
        }
    }

    fn speed_unit(&self) -> &str {
        match self {
            WeatherUnits::Metric => "m/s",
            WeatherUnits::Imperial => "mph",
            WeatherUnits::Kelvin => "m/s",
        }
    }
}

/// Current weather data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentWeather {
    /// Location name
    pub location: String,
    /// Country code
    pub country: String,
    /// Coordinates
    pub coords: Coordinates,
    /// Weather condition
    pub condition: WeatherCondition,
    /// Temperature
    pub temperature: Temperature,
    /// Feels like temperature
    pub feels_like: f32,
    /// Humidity percentage
    pub humidity: u8,
    /// Pressure in hPa
    pub pressure: u32,
    /// Wind info
    pub wind: Wind,
    /// Cloudiness percentage
    pub clouds: u8,
    /// Visibility in meters
    pub visibility: u32,
    /// Sunrise time
    pub sunrise: chrono::DateTime<chrono::Utc>,
    /// Sunset time
    pub sunset: chrono::DateTime<chrono::Utc>,
    /// Data timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Units used
    pub units: WeatherUnits,
}

/// Coordinates
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Coordinates {
    pub lat: f64,
    pub lon: f64,
}

/// Weather condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherCondition {
    /// Condition ID
    pub id: u32,
    /// Main condition (Rain, Snow, Clear, etc.)
    pub main: String,
    /// Description
    pub description: String,
    /// Icon code
    pub icon: String,
}

impl WeatherCondition {
    /// Get emoji for condition
    pub fn emoji(&self) -> &str {
        match self.main.as_str() {
            "Clear" => "☀️",
            "Clouds" => "☁️",
            "Rain" | "Drizzle" => "🌧️",
            "Thunderstorm" => "⛈️",
            "Snow" => "🌨️",
            "Mist" | "Fog" | "Haze" => "🌫️",
            "Smoke" | "Dust" | "Sand" | "Ash" => "💨",
            "Squall" | "Tornado" => "🌪️",
            _ => "🌡️",
        }
    }

    /// Get icon URL
    pub fn icon_url(&self) -> String {
        format!("https://openweathermap.org/img/wn/{}@2x.png", self.icon)
    }
}

/// Temperature
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Temperature {
    /// Current temperature
    pub current: f32,
    /// Minimum temperature
    pub min: f32,
    /// Maximum temperature
    pub max: f32,
}

/// Wind information
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Wind {
    /// Speed
    pub speed: f32,
    /// Direction in degrees
    pub deg: u16,
    /// Gust speed
    pub gust: Option<f32>,
}

impl Wind {
    /// Get cardinal direction
    pub fn direction(&self) -> &str {
        match self.deg {
            0..=22 => "N",
            23..=67 => "NE",
            68..=112 => "E",
            113..=157 => "SE",
            158..=202 => "S",
            203..=247 => "SW",
            248..=292 => "W",
            293..=337 => "NW",
            338..=360 => "N",
            _ => "?",
        }
    }
}

/// Weather forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Forecast {
    /// Location name
    pub location: String,
    /// Country code
    pub country: String,
    /// Forecast periods
    pub periods: Vec<ForecastPeriod>,
    /// Units used
    pub units: WeatherUnits,
}

/// Forecast period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPeriod {
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Weather condition
    pub condition: WeatherCondition,
    /// Temperature
    pub temperature: Temperature,
    /// Feels like
    pub feels_like: f32,
    /// Humidity
    pub humidity: u8,
    /// Wind
    pub wind: Wind,
    /// Precipitation probability (0-100)
    pub pop: u8,
    /// Rain volume in mm (3h)
    pub rain: Option<f32>,
    /// Snow volume in mm (3h)
    pub snow: Option<f32>,
}

/// Weather client
pub struct WeatherClient {
    config: WeatherConfig,
}

impl WeatherClient {
    const OWM_BASE: &'static str = "https://api.openweathermap.org/data/2.5";

    /// Create a new weather client
    pub fn new(config: &WeatherConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        Ok(Self { config })
    }

    /// Check if client is configured
    pub fn is_configured(&self) -> bool {
        self.config.enabled && !self.config.openweathermap_key.is_empty()
    }

    /// Get current weather by city name
    pub async fn current(&self, location: &str) -> Result<CurrentWeather> {
        let url = format!(
            "{}/weather?q={}&appid={}&units={}&lang={}",
            Self::OWM_BASE,
            urlencoding::encode(location),
            self.config.openweathermap_key,
            self.config.units.to_owm_string(),
            self.config.language
        );

        let response: serde_json::Value = self.api_get(&url).await?;
        self.parse_current_weather(response)
    }

    /// Get current weather by coordinates
    pub async fn current_by_coords(&self, lat: f64, lon: f64) -> Result<CurrentWeather> {
        let url = format!(
            "{}/weather?lat={}&lon={}&appid={}&units={}&lang={}",
            Self::OWM_BASE,
            lat,
            lon,
            self.config.openweathermap_key,
            self.config.units.to_owm_string(),
            self.config.language
        );

        let response: serde_json::Value = self.api_get(&url).await?;
        self.parse_current_weather(response)
    }

    /// Get weather forecast
    pub async fn forecast(&self, location: &str, days: u8) -> Result<Forecast> {
        // OWM free tier gives 5-day/3-hour forecast
        let cnt = (days as u32 * 8).min(40); // 8 periods per day

        let url = format!(
            "{}/forecast?q={}&cnt={}&appid={}&units={}&lang={}",
            Self::OWM_BASE,
            urlencoding::encode(location),
            cnt,
            self.config.openweathermap_key,
            self.config.units.to_owm_string(),
            self.config.language
        );

        let response: serde_json::Value = self.api_get(&url).await?;
        self.parse_forecast(response)
    }

    /// Get forecast by coordinates
    pub async fn forecast_by_coords(&self, lat: f64, lon: f64, days: u8) -> Result<Forecast> {
        let cnt = (days as u32 * 8).min(40);

        let url = format!(
            "{}/forecast?lat={}&lon={}&cnt={}&appid={}&units={}&lang={}",
            Self::OWM_BASE,
            lat,
            lon,
            cnt,
            self.config.openweathermap_key,
            self.config.units.to_owm_string(),
            self.config.language
        );

        let response: serde_json::Value = self.api_get(&url).await?;
        self.parse_forecast(response)
    }

    /// Get default location weather
    pub async fn current_default(&self) -> Result<CurrentWeather> {
        let location = self
            .config
            .default_location
            .clone()
            .ok_or_else(|| DrivenError::Config("No default location set".into()))?;

        self.current(&location).await
    }

    /// Format temperature with unit
    pub fn format_temp(&self, temp: f32) -> String {
        format!("{:.1}{}", temp, self.config.units.temp_symbol())
    }

    /// Format wind with unit
    pub fn format_wind(&self, wind: &Wind) -> String {
        format!(
            "{:.1} {} {}",
            wind.speed,
            self.config.units.speed_unit(),
            wind.direction()
        )
    }

    fn parse_current_weather(&self, data: serde_json::Value) -> Result<CurrentWeather> {
        let weather = data["weather"][0].clone();
        let main = &data["main"];
        let wind = &data["wind"];
        let sys = &data["sys"];

        Ok(CurrentWeather {
            location: data["name"].as_str().unwrap_or_default().to_string(),
            country: sys["country"].as_str().unwrap_or_default().to_string(),
            coords: Coordinates {
                lat: data["coord"]["lat"].as_f64().unwrap_or(0.0),
                lon: data["coord"]["lon"].as_f64().unwrap_or(0.0),
            },
            condition: WeatherCondition {
                id: weather["id"].as_u64().unwrap_or(0) as u32,
                main: weather["main"].as_str().unwrap_or_default().to_string(),
                description: weather["description"].as_str().unwrap_or_default().to_string(),
                icon: weather["icon"].as_str().unwrap_or_default().to_string(),
            },
            temperature: Temperature {
                current: main["temp"].as_f64().unwrap_or(0.0) as f32,
                min: main["temp_min"].as_f64().unwrap_or(0.0) as f32,
                max: main["temp_max"].as_f64().unwrap_or(0.0) as f32,
            },
            feels_like: main["feels_like"].as_f64().unwrap_or(0.0) as f32,
            humidity: main["humidity"].as_u64().unwrap_or(0) as u8,
            pressure: main["pressure"].as_u64().unwrap_or(0) as u32,
            wind: Wind {
                speed: wind["speed"].as_f64().unwrap_or(0.0) as f32,
                deg: wind["deg"].as_u64().unwrap_or(0) as u16,
                gust: wind["gust"].as_f64().map(|g| g as f32),
            },
            clouds: data["clouds"]["all"].as_u64().unwrap_or(0) as u8,
            visibility: data["visibility"].as_u64().unwrap_or(10000) as u32,
            sunrise: chrono::DateTime::from_timestamp(
                sys["sunrise"].as_i64().unwrap_or(0),
                0,
            )
            .unwrap_or_default(),
            sunset: chrono::DateTime::from_timestamp(
                sys["sunset"].as_i64().unwrap_or(0),
                0,
            )
            .unwrap_or_default(),
            timestamp: chrono::DateTime::from_timestamp(
                data["dt"].as_i64().unwrap_or(0),
                0,
            )
            .unwrap_or_default(),
            units: self.config.units,
        })
    }

    fn parse_forecast(&self, data: serde_json::Value) -> Result<Forecast> {
        let city = &data["city"];
        let list = data["list"]
            .as_array()
            .ok_or_else(|| DrivenError::Parse("Invalid forecast data".into()))?;

        let periods = list
            .iter()
            .map(|p| {
                let weather = &p["weather"][0];
                let main = &p["main"];
                let wind = &p["wind"];

                ForecastPeriod {
                    timestamp: chrono::DateTime::from_timestamp(
                        p["dt"].as_i64().unwrap_or(0),
                        0,
                    )
                    .unwrap_or_default(),
                    condition: WeatherCondition {
                        id: weather["id"].as_u64().unwrap_or(0) as u32,
                        main: weather["main"].as_str().unwrap_or_default().to_string(),
                        description: weather["description"].as_str().unwrap_or_default().to_string(),
                        icon: weather["icon"].as_str().unwrap_or_default().to_string(),
                    },
                    temperature: Temperature {
                        current: main["temp"].as_f64().unwrap_or(0.0) as f32,
                        min: main["temp_min"].as_f64().unwrap_or(0.0) as f32,
                        max: main["temp_max"].as_f64().unwrap_or(0.0) as f32,
                    },
                    feels_like: main["feels_like"].as_f64().unwrap_or(0.0) as f32,
                    humidity: main["humidity"].as_u64().unwrap_or(0) as u8,
                    wind: Wind {
                        speed: wind["speed"].as_f64().unwrap_or(0.0) as f32,
                        deg: wind["deg"].as_u64().unwrap_or(0) as u16,
                        gust: wind["gust"].as_f64().map(|g| g as f32),
                    },
                    pop: (p["pop"].as_f64().unwrap_or(0.0) * 100.0) as u8,
                    rain: p["rain"]["3h"].as_f64().map(|r| r as f32),
                    snow: p["snow"]["3h"].as_f64().map(|s| s as f32),
                }
            })
            .collect();

        Ok(Forecast {
            location: city["name"].as_str().unwrap_or_default().to_string(),
            country: city["country"].as_str().unwrap_or_default().to_string(),
            periods,
            units: self.config.units,
        })
    }

    async fn api_get(&self, url: &str) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!(
                "Weather API error ({}): {}",
                status, error
            )));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WeatherConfig::default();
        assert!(config.enabled);
        assert_eq!(config.units, WeatherUnits::Metric);
    }

    #[test]
    fn test_units() {
        assert_eq!(WeatherUnits::Metric.temp_symbol(), "°C");
        assert_eq!(WeatherUnits::Imperial.temp_symbol(), "°F");
        assert_eq!(WeatherUnits::Kelvin.temp_symbol(), "K");
    }

    #[test]
    fn test_wind_direction() {
        let wind = Wind {
            speed: 5.0,
            deg: 90,
            gust: None,
        };
        assert_eq!(wind.direction(), "E");
    }

    #[test]
    fn test_condition_emoji() {
        let condition = WeatherCondition {
            id: 800,
            main: "Clear".to_string(),
            description: "clear sky".to_string(),
            icon: "01d".to_string(),
        };
        assert_eq!(condition.emoji(), "☀️");
    }
}
