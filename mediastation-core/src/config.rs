//! Configuration module for MediaStation

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// Configuration for Trakt.tv integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraktConfig {
    pub username: String,
    pub client_id: String,
}

impl Default for TraktConfig {
    fn default() -> Self {
        Self {
            username: String::new(),
            client_id: String::new(),
        }
    }
}

/// Configuration for qBittorrent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QBittorrentConfig {
    pub host: String,
    pub username: String,
    pub password: String,
}

impl Default for QBittorrentConfig {
    fn default() -> Self {
        Self {
            host: "http://localhost:8080".to_string(),
            username: String::new(),
            password: String::new(),
        }
    }
}

/// Configuration for Telegram notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
    pub enabled: bool,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            chat_id: String::new(),
            enabled: false,
        }
    }
}

/// Configuration for paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub download_dir: String,
    pub movies_dir: String,
    pub shows_dir: String,
    pub anime_dir: String,
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            download_dir: String::new(),
            movies_dir: String::new(),
            shows_dir: String::new(),
            anime_dir: String::new(),
        }
    }
}

/// General settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub check_interval_minutes: u32,
    pub min_seeds: u32,
    pub preferred_seeds: u32,
    pub quality_priority: Vec<String>,
    pub max_size_gb: u32,
    pub avoid_cam: bool,
    pub stop_seeding_after_seconds: u32,
    pub auto_delete_removed: bool,
    pub stall_threshold_hours: u32,
    #[serde(default = "default_search_indexers")]
    pub search_indexers: Vec<String>,
}

fn default_search_indexers() -> Vec<String> {
    vec!["yts".to_string(), "tpb".to_string(), "1337x".to_string()]
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            check_interval_minutes: 15,
            min_seeds: 5,
            preferred_seeds: 20,
            quality_priority: vec!["2160p".to_string(), "1080p".to_string()],
            max_size_gb: 10,
            avoid_cam: true,
            stop_seeding_after_seconds: 10,
            auto_delete_removed: false,
            stall_threshold_hours: 24,
            search_indexers: default_search_indexers(),
        }
    }
}

/// TV settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TVSettings {
    pub max_episode_size_gb: u32,
    pub max_season_size_gb: u32,
    pub prefer_season_packs: bool,
    pub quality_priority: Vec<String>,
}

impl Default for TVSettings {
    fn default() -> Self {
        Self {
            max_episode_size_gb: 3,
            max_season_size_gb: 50,
            prefer_season_packs: true,
            quality_priority: vec!["2160p".to_string(), "1080p".to_string()],
        }
    }
}

/// Raw configuration structure from JSON
#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(default)]
    trakt: TraktConfig,
    #[serde(default)]
    qbittorrent: QBittorrentConfig,
    #[serde(default)]
    telegram: TelegramConfig,
    #[serde(default)]
    paths: PathsConfig,
    #[serde(default)]
    settings: Settings,
    #[serde(default)]
    tv_settings: TVSettings,
}

/// Application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub trakt: Arc<TraktConfig>,
    pub qbittorrent: Arc<QBittorrentConfig>,
    pub telegram: Arc<TelegramConfig>,
    pub paths: Arc<PathsConfig>,
    pub settings: Arc<Settings>,
    pub tv_settings: Arc<TVSettings>,
    config_path: String,
}

impl AppConfig {
    /// Load configuration from a JSON file
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let config_path = Path::new(path);

        let raw = if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            serde_json::from_str::<RawConfig>(&content)?
        } else {
            log::warn!("Config file not found, using defaults: {}", path);
            RawConfig::default()
        };

        Ok(Self {
            trakt: Arc::new(raw.trakt),
            qbittorrent: Arc::new(raw.qbittorrent),
            telegram: Arc::new(raw.telegram),
            paths: Arc::new(raw.paths),
            settings: Arc::new(raw.settings),
            tv_settings: Arc::new(raw.tv_settings),
            config_path: path.to_string(),
        })
    }

    /// Get the config file path
    pub fn config_path(&self) -> &str {
        &self.config_path
    }

    /// Save configuration to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let raw = serde_json::json!({
            "trakt": *self.trakt,
            "qbittorrent": *self.qbittorrent,
            "telegram": *self.telegram,
            "paths": *self.paths,
            "settings": *self.settings,
            "tv_settings": *self.tv_settings,
        });
        let content = serde_json::to_string_pretty(&raw)?;
        std::fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// Get the base directory for state files
    pub fn base_dir(&self) -> std::path::PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("rustyfin")
    }

    /// Get the processed files path
    pub fn processed_file(&self) -> std::path::PathBuf {
        self.base_dir().join("processed.json")
    }

    pub fn processed_shows_file(&self) -> std::path::PathBuf {
        self.base_dir().join("processed_shows.json")
    }

    pub fn organized_shows_file(&self) -> std::path::PathBuf {
        self.base_dir().join("organized_shows.json")
    }
}

impl Default for RawConfig {
    fn default() -> Self {
        Self {
            trakt: TraktConfig::default(),
            qbittorrent: QBittorrentConfig::default(),
            telegram: TelegramConfig::default(),
            paths: PathsConfig::default(),
            settings: Settings::default(),
            tv_settings: TVSettings::default(),
        }
    }
}
