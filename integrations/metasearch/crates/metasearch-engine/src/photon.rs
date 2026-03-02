//! Photon — geocoding / map search via Komoot's Photon API.

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};
use reqwest::Client;
use serde::Deserialize;
use smallvec::smallvec;

pub struct Photon {
    metadata: EngineMetadata,
    client: Client,
}

impl Photon {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "photon".to_string().into(),
                display_name: "Photon".to_string().into(),
                homepage: "https://photon.komoot.io".to_string().into(),
                categories: smallvec![SearchCategory::Maps],
                enabled: true,
                timeout_ms: 5000,
                weight: 1.0,
            },
            client,
        }
    }
}

#[derive(Deserialize)]
struct GeoJsonResponse {
    features: Option<Vec<Feature>>,
}

#[derive(Deserialize)]
struct Feature {
    properties: Option<FeatureProperties>,
    geometry: Option<Geometry>,
}

#[derive(Deserialize)]
struct FeatureProperties {
    name: Option<String>,
    city: Option<String>,
    state: Option<String>,
    country: Option<String>,
    #[serde(rename = "type")]
    osm_type: Option<String>,
    osm_id: Option<u64>,
    osm_key: Option<String>,
}

#[derive(Deserialize)]
struct Geometry {
    coordinates: Option<Vec<f64>>,
}

#[async_trait]
impl SearchEngine for Photon {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        let url = format!(
            "https://photon.komoot.io/api/?q={}&limit=10",
            urlencoding::encode(&query.query)
        );

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Photon request failed: {e}")))?;

        let geo: GeoJsonResponse = resp
            .json()
            .await
            .map_err(|e| MetasearchError::Engine(format!("Photon parse failed: {e}")))?;

        let results = geo
            .features
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .filter_map(|(i, feature)| {
                let props = feature.properties?;
                let name = props.name.filter(|n| !n.is_empty())?;

                // Build location string
                let mut parts: Vec<String> = Vec::new();
                if let Some(city) = &props.city {
                    if !city.is_empty() && *city != name {
                        parts.push(city.clone());
                    }
                }
                if let Some(state) = &props.state {
                    if !state.is_empty() {
                        parts.push(state.clone());
                    }
                }
                if let Some(country) = &props.country {
                    if !country.is_empty() {
                        parts.push(country.clone());
                    }
                }
                let snippet = parts.join(", ");

                // Build OSM URL
                let result_url = if let Some(ref osm_type) = props.osm_type {
                    if let Some(osm_id) = props.osm_id {
                        let osm_letter = match osm_type.as_str() {
                            "N" => "node",
                            "W" => "way",
                            "R" => "relation",
                            _ => "node",
                        };
                        format!("https://www.openstreetmap.org/{}/{}", osm_letter, osm_id)
                    } else if let Some(coords) = &feature.geometry.and_then(|g| g.coordinates) {
                        if coords.len() >= 2 {
                            format!(
                                "https://www.openstreetmap.org/?mlat={}&mlon={}#map=15/{}/{}",
                                coords[1], coords[0], coords[1], coords[0]
                            )
                        } else {
                            return None;
                        }
                    } else {
                        return None;
                    }
                } else if let Some(coords) = &feature.geometry.and_then(|g| g.coordinates) {
                    if coords.len() >= 2 {
                        format!(
                            "https://www.openstreetmap.org/?mlat={}&mlon={}#map=15/{}/{}",
                            coords[1], coords[0], coords[1], coords[0]
                        )
                    } else {
                        return None;
                    }
                } else {
                    return None;
                };

                let title = if let Some(key) = &props.osm_key {
                    format!("{} ({})", name, key)
                } else {
                    name
                };

                let mut result = SearchResult::new(
                    title,
                    result_url,
                    snippet,
                    "photon",
                );
                result.engine_rank = (i + 1) as u32;
                Some(result)
            })
            .collect();

        Ok(results)
    }
}
