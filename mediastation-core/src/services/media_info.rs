//! Media Info service - OMDb and TMDB integration

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Media info service for fetching metadata
pub struct MediaInfoService {
    config: Arc<crate::config::AppConfig>,
    client: reqwest::Client,
    cache: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl MediaInfoService {
    pub fn new(config: Arc<crate::config::AppConfig>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("RustyFin/1.0")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            config,
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get movie info from OMDb
    pub async fn get_omdb_movie(&self, title: &str, year: Option<i32>) -> Option<crate::models::OmdbResponse> {
        let cache_key = format!("omdb:{}:{:?}", title, year);
        
        // Check cache
        if let Some(cached) = self.cache.read().get(&cache_key) {
            return serde_json::from_value(cached.clone()).ok();
        }

        let api_key = &self.config.omdb.api_key;
        if api_key.is_empty() {
            log::debug!("OMDb API key not configured");
            return None;
        }

        let mut url = format!("https://www.omdbapi.com/?t={}&apikey={}", urlencoding::encode(title), api_key);
        if let Some(y) = year {
            url.push_str(&format!("&y={}", y));
        }

        match self.client.get(&url).send().await {
            Ok(response) => {
                if let Ok(data) = response.json::<serde_json::Value>().await {
                    if data.get("Response").and_then(|v| v.as_str()) == Some("True") {
                        // Cache the result
                        self.cache.write().insert(cache_key, data.clone());
                        
                        return Some(crate::models::OmdbResponse {
                            title: data.get("Title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            year: data.get("Year").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()),
                            rated: data.get("Rated").and_then(|v| v.as_str()).map(String::from),
                            plot: data.get("Plot").and_then(|v| v.as_str()).map(String::from),
                            poster: data.get("Poster").and_then(|v| v.as_str()).map(String::from),
                        });
                    }
                }
            }
            Err(e) => {
                log::warn!("OMDb request failed: {}", e);
            }
        }

        None
    }

    /// Get movie from TMDB
    pub async fn get_tmdb_movie(&self, title: &str, year: Option<i32>) -> Option<crate::models::TmdbResponse> {
        let cache_key = format!("tmdb:{}:{:?}", title, year);
        
        if let Some(cached) = self.cache.read().get(&cache_key) {
            return serde_json::from_value(cached.clone()).ok();
        }

        // TMDB requires API key - placeholder
        None
    }

    /// Get show from TMDB
    pub async fn get_tmdb_show(&self, title: &str) -> Option<crate::models::TmdbResponse> {
        let cache_key = format!("tmdb_show:{}", title);
        
        if let Some(cached) = self.cache.read().get(&cache_key) {
            return serde_json::from_value(cached.clone()).ok();
        }

        None
    }

    /// Search for movie/show and return best match
    pub async fn search(&self, title: &str, media_type: &str) -> Option<String> {
        match media_type {
            "movie" => {
                let omdb = self.get_omdb_movie(title, None).await;
                omdb.map(|m| m.title)
            }
            "show" => {
                let tmdb = self.get_tmdb_show(title).await;
                tmdb.map(|t| t.name)
            }
            _ => None,
        }
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
}