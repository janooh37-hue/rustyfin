//! Trakt service - Trakt.tv API integration

use crate::config::AppConfig;
use crate::models::{MediaType, WatchlistItem};

/// Trakt service for watchlist management
pub struct TraktService {
    config: AppConfig,
    client: reqwest::Client,
}

impl TraktService {
    pub fn new(config: AppConfig) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("RustyFin/1.0")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    /// Get user's movie watchlist
    pub async fn get_movie_watchlist(&self) -> Vec<WatchlistItem> {
        if self.config.trakt.username.is_empty() || self.config.trakt.client_id.is_empty() {
            return Vec::new();
        }

        let url = format!(
            "https://api.trakt.tv/users/{}/watchlist/movies",
            self.config.trakt.username
        );

        match self
            .client
            .get(&url)
            .header("Content-Type", "application/json")
            .header("trakt-api-version", "2")
            .header("trakt-api-key", &*self.config.trakt.client_id)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                match response.json::<serde_json::Value>().await {
                    Ok(serde_json::Value::Array(items)) => {
                        items
                            .iter()
                            .filter_map(|entry| {
                                let movie = entry.get("movie")?;
                                let title = movie.get("title")?.as_str()?.to_string();
                                let year = movie.get("year").and_then(|y| y.as_i64()).map(|y| y as i32);
                                let slug = movie
                                    .get("ids")
                                    .and_then(|ids| ids.get("slug"))
                                    .and_then(|s| s.as_str())
                                    .map(|s| s.to_string());
                                Some(WatchlistItem {
                                    title,
                                    year,
                                    media_type: MediaType::Movie,
                                    trakt_slug: slug,
                                    poster: None,
                                })
                            })
                            .collect()
                    }
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }

    /// Get user's show watchlist
    pub async fn get_show_watchlist(&self) -> Vec<WatchlistItem> {
        if self.config.trakt.username.is_empty() || self.config.trakt.client_id.is_empty() {
            return Vec::new();
        }

        let url = format!(
            "https://api.trakt.tv/users/{}/watchlist/shows",
            self.config.trakt.username
        );

        match self
            .client
            .get(&url)
            .header("Content-Type", "application/json")
            .header("trakt-api-version", "2")
            .header("trakt-api-key", &*self.config.trakt.client_id)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                match response.json::<serde_json::Value>().await {
                    Ok(serde_json::Value::Array(items)) => {
                        items
                            .iter()
                            .filter_map(|entry| {
                                let show = entry.get("show")?;
                                let title = show.get("title")?.as_str()?.to_string();
                                let year = show.get("year").and_then(|y| y.as_i64()).map(|y| y as i32);
                                let slug = show
                                    .get("ids")
                                    .and_then(|ids| ids.get("slug"))
                                    .and_then(|s| s.as_str())
                                    .map(|s| s.to_string());
                                Some(WatchlistItem {
                                    title,
                                    year,
                                    media_type: MediaType::Show,
                                    trakt_slug: slug,
                                    poster: None,
                                })
                            })
                            .collect()
                    }
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }

    /// Get anime watchlist from custom "anime" list
    pub async fn get_anime_watchlist(&self) -> Vec<WatchlistItem> {
        if self.config.trakt.username.is_empty() || self.config.trakt.client_id.is_empty() {
            return Vec::new();
        }

        let url = format!(
            "https://api.trakt.tv/users/{}/lists/anime/items",
            self.config.trakt.username
        );

        match self
            .client
            .get(&url)
            .header("Content-Type", "application/json")
            .header("trakt-api-version", "2")
            .header("trakt-api-key", &*self.config.trakt.client_id)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                match response.json::<serde_json::Value>().await {
                    Ok(serde_json::Value::Array(items)) => {
                        items
                            .iter()
                            .filter_map(|entry| {
                                // Items in anime list can be "show" or "movie" type on Trakt
                                let item_type = entry.get("type")?.as_str()?;
                                let (title, year, slug) = match item_type {
                                    "show" => {
                                        let show = entry.get("show")?;
                                        let title = show.get("title")?.as_str()?.to_string();
                                        let year = show.get("year").and_then(|y| y.as_i64()).map(|y| y as i32);
                                        let slug = show.get("ids")
                                            .and_then(|ids| ids.get("slug"))
                                            .and_then(|s| s.as_str())
                                            .map(|s| s.to_string());
                                        (title, year, slug)
                                    }
                                    "movie" => {
                                        let movie = entry.get("movie")?;
                                        let title = movie.get("title")?.as_str()?.to_string();
                                        let year = movie.get("year").and_then(|y| y.as_i64()).map(|y| y as i32);
                                        let slug = movie.get("ids")
                                            .and_then(|ids| ids.get("slug"))
                                            .and_then(|s| s.as_str())
                                            .map(|s| s.to_string());
                                        (title, year, slug)
                                    }
                                    _ => return None,
                                };
                                Some(WatchlistItem {
                                    title,
                                    year,
                                    media_type: MediaType::Anime,
                                    trakt_slug: slug,
                                    poster: None,
                                })
                            })
                            .collect()
                    }
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }

    /// Get combined watchlist (movies + shows + anime)
    pub async fn get_watchlist(&self) -> Vec<WatchlistItem> {
        let mut items = Vec::new();
        items.extend(self.get_movie_watchlist().await);
        items.extend(self.get_show_watchlist().await);
        items.extend(self.get_anime_watchlist().await);
        items
    }

    /// Get show seasons
    pub async fn get_show_seasons(&self, trakt_slug: &str) -> Vec<crate::models::TraktSeason> {
        if self.config.trakt.client_id.is_empty() {
            return Vec::new();
        }

        let url = format!("https://api.trakt.tv/shows/{}/seasons", trakt_slug);

        match self
            .client
            .get(&url)
            .header("Content-Type", "application/json")
            .header("trakt-api-version", "2")
            .header("trakt-api-key", &*self.config.trakt.client_id)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                Vec::new() // TODO: parse seasons
            }
            _ => Vec::new(),
        }
    }

    /// Get episode details
    pub async fn get_episode(
        &self,
        trakt_slug: &str,
        season: u32,
        episode: u32,
    ) -> Option<crate::models::TraktEpisode> {
        if self.config.trakt.client_id.is_empty() {
            return None;
        }

        let url = format!(
            "https://api.trakt.tv/shows/{}/seasons/{}/episodes/{}",
            trakt_slug, season, episode
        );

        match self
            .client
            .get(&url)
            .header("Content-Type", "application/json")
            .header("trakt-api-version", "2")
            .header("trakt-api-key", &*self.config.trakt.client_id)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                None // TODO: parse episode
            }
            _ => None,
        }
    }
}
