//! Subtitle service - Download subtitles for movies

use std::path::Path;

use crate::config::AppConfig;

/// Subtitle service for downloading Arabic subtitles
pub struct SubtitleService {
    config: AppConfig,
    client: reqwest::Client,
}

impl SubtitleService {
    pub fn new(config: AppConfig) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    /// Download subtitle for a video file
    pub async fn download_subtitle(&self, video_file: &str, title: Option<&str>) -> (bool, String) {
        let path = Path::new(video_file);
        let directory = path.parent().unwrap_or(path);

        // Try SubDL API first
        if let Some(t) = title {
            match self.download_subdl(t, directory).await {
                (true, msg) => return (true, msg),
                (false, _) => {}
            }
        }

        // Try OpenSubtitles
        if let Some(t) = title {
            match self.download_opensubtitles(t, directory).await {
                (true, msg) => return (true, msg),
                (false, _) => {}
            }
        }

        // Try YIFY subtitles
        if let Some(t) = title {
            match self.download_yify(t, directory).await {
                (true, msg) => return (true, msg),
                (false, _) => {}
            }
        }

        // Try Subscene
        if let Some(t) = title {
            match self.download_subscene(t, directory).await {
                (true, msg) => return (true, msg),
                (false, _) => {}
            }
        }

        (false, "No subtitle found".to_string())
    }

    /// Download from SubDL API
    async fn download_subdl(&self, title: &str, _directory: &Path) -> (bool, String) {
        // SubDL API (simplified - would need actual API key)
        let url = format!("https://api.subdl.com/api/v1/subtitles?query={}", urlencoding::encode(title));
        
        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    // Process response and download
                    (true, "SubDL subtitle found".to_string())
                } else {
                    (false, "SubDL not available".to_string())
                }
            }
            Err(e) => (false, format!("SubDL error: {}", e)),
        }
    }

    /// Download from OpenSubtitles
    async fn download_opensubtitles(&self, title: &str, _directory: &Path) -> (bool, String) {
        // OpenSubtitles (simplified)
        let _url = format!("https://www.opensubtitles.com/en/search/subs/{}?language=ar", urlencoding::encode(title));
        
        (false, "OpenSubtitles not implemented".to_string())
    }

    /// Download from YIFY
    async fn download_yify(&self, title: &str, _directory: &Path) -> (bool, String) {
        // YIFY subtitles
        let _url = format!("https://www.yifysubtitles.com/search?q={}", urlencoding::encode(title));
        
        (false, "YIFY not implemented".to_string())
    }

    /// Download from Subscene
    async fn download_subscene(&self, title: &str, _directory: &Path) -> (bool, String) {
        // Subscene scraping
        let _url = format!("https://subscene.com/subtitles/search?q={}", urlencoding::encode(title));
        
        (false, "Subscene not implemented".to_string())
    }

    /// Get missing subtitles (movies without Arabic subs)
    pub async fn get_missing_subs(&self) -> Vec<crate::models::MissingSubtitle> {
        // This would need to scan the library
        Vec::new()
    }
}