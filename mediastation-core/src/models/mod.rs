//! Models module - Data structures for the application

use serde::{Deserialize, Serialize};

/// A torrent from qBittorrent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Torrent {
    pub hash: String,
    pub name: String,
    pub state: String,
    pub progress: f64,
    pub size: u64,
    pub download_speed: u64,
    pub upload_speed: u64,
    pub eta: i64,
    pub seeds: u32,
    pub leechs: u32,
    pub category: String,
    pub added_on: i64,
    /// Full path to the content (file or root folder)
    #[serde(default)]
    pub content_path: String,
    /// Save path (download directory)
    #[serde(default)]
    pub save_path: String,
}

/// Torrent state categories
pub mod torrent_state {
    pub const SEEDING: &[&str] = &["uploading", "stalledUP", "pausedUP", "forcedUP", "queuedUP"];
    pub const DOWNLOADING: &[&str] = &[
        "downloading",
        "stalledDL",
        "metaDL",
        "forcedDL",
        "allocating",
        "queuedDL",
    ];

    pub fn is_seeding(state: &str) -> bool {
        SEEDING.contains(&state)
    }

    pub fn is_downloading(state: &str) -> bool {
        DOWNLOADING.contains(&state)
    }
}

/// Search result from torrent indexers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub name: String,
    pub title: String,
    pub year: Option<i32>,
    pub quality: String,
    pub seeds: u32,
    pub size: String,
    pub size_gb: f64,
    pub magnet: String,
    pub source: String,
}

/// A movie in the library
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Movie {
    pub title: String,
    pub year: Option<i32>,
    pub path: String,
    pub video_file: Option<String>,
    pub resolution: Option<String>,
    pub has_subtitle: bool,
    pub added_at: i64,
}

/// A TV show in the library
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Show {
    pub title: String,
    pub year: Option<i32>,
    pub path: String,
    pub seasons: Vec<Season>,
    pub added_at: i64,
}

/// A season of a TV show
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Season {
    pub number: u32,
    pub episodes: Vec<Episode>,
}

/// An episode of a TV show
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub number: u32,
    pub title: Option<String>,
    pub path: String,
}

/// Anime entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anime {
    pub title: String,
    pub path: String,
    pub seasons: Vec<Season>,
    pub added_at: i64,
}

/// Unified recent item for the Recent panel (movies, shows, anime)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentItem {
    pub title: String,
    pub year: Option<i32>,
    pub path: String,
    pub video_file: Option<String>,
    pub has_subtitle: bool,
    pub media_type: MediaType,
    pub added_at: i64,
}

/// Watchlist item from Trakt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistItem {
    pub title: String,
    pub year: Option<i32>,
    pub media_type: MediaType,
    pub trakt_slug: Option<String>,
    pub poster: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MediaType {
    Movie,
    Show,
    Anime,
}

/// Missing subtitle entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingSubtitle {
    pub title: String,
    pub path: String,
    pub video_file: String,
    pub year: Option<i32>,
}

/// Pending item for organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingItem {
    pub video_file: String,
    pub show_name: Option<String>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub pending_type: PendingType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PendingType {
    Movie,
    Show,
    Anime,
}

/// Transfer info from qBittorrent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferInfo {
    pub download_speed: u64,
    pub upload_speed: u64,
    pub dht_nodes: u32,
}

/// Library statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LibraryStats {
    pub movies_count: u32,
    pub shows_count: u32,
    pub anime_count: u32,
    pub total_size_gb: f64,
    pub missing_subs: u32,
}

/// Trakt season/episode data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraktSeason {
    pub number: u32,
    pub episodes: Vec<TraktEpisode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraktEpisode {
    pub number: u32,
    pub title: Option<String>,
    pub season: u32,
}

/// OMDb response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmdbResponse {
    pub title: String,
    pub year: Option<String>,
    pub rated: Option<String>,
    pub plot: Option<String>,
    pub poster: Option<String>,
}

/// TMDB movie/TV response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbResponse {
    pub id: u64,
    pub name: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
}
