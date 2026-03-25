//! Organize service - Move downloads into library structure

use std::path::Path;

use crate::config::AppConfig;
use crate::models::PendingItem;

/// Organize service for moving completed downloads
pub struct OrganizeService {
    config: AppConfig,
}

impl OrganizeService {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    /// Get pending items from download directory
    pub fn get_pending(&self) -> Vec<PendingItem> {
        let mut items = Vec::new();
        let download_dir = Path::new(&self.config.paths.download_dir);

        if !download_dir.exists() {
            return items;
        }

        if let Ok(entries) = std::fs::read_dir(download_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && is_video_file(&path) {
                    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if let Some(item) = self.detect_type(filename, &path.to_string_lossy()) {
                        items.push(item);
                    }
                } else if path.is_dir() {
                    // Scan subdirectories for video files (torrent folders)
                    if let Ok(sub_entries) = std::fs::read_dir(&path) {
                        for sub_entry in sub_entries.flatten() {
                            let sub_path = sub_entry.path();
                            if sub_path.is_file() && is_video_file(&sub_path) {
                                let filename = sub_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                                if let Some(item) = self.detect_type(filename, &sub_path.to_string_lossy()) {
                                    items.push(item);
                                }
                            }
                        }
                    }
                }
            }
        }

        items
    }

    /// Detect item type from filename
    fn detect_type(&self, filename: &str, path: &str) -> Option<PendingItem> {
        let name_lower = filename.to_lowercase();

        // Check for anime patterns (abbreviations)
        if is_anime_filename(&name_lower) {
            let (show_name, season) = parse_anime_name(&name_lower);
            return Some(PendingItem {
                video_file: path.to_string(),
                show_name: Some(show_name),
                season: Some(season),
                episode: None,
                pending_type: crate::models::PendingType::Anime,
            });
        }

        // Check for TV show patterns (S01E02)
        if let Some((show_name, season, episode)) = parse_show_episode(filename) {
            return Some(PendingItem {
                video_file: path.to_string(),
                show_name: Some(show_name),
                season: Some(season),
                episode: Some(episode),
                pending_type: crate::models::PendingType::Show,
            });
        }

        // Check for movie (has year or resolution)
        if is_movie_filename(&name_lower) {
            return Some(PendingItem {
                video_file: path.to_string(),
                show_name: None,
                season: None,
                episode: None,
                pending_type: crate::models::PendingType::Movie,
            });
        }

        // Default to movie
        Some(PendingItem {
            video_file: path.to_string(),
            show_name: None,
            season: None,
            episode: None,
            pending_type: crate::models::PendingType::Movie,
        })
    }

    /// Organize a movie - returns (success, message, destination_path)
    pub fn organize_movie(&self, video_file: &str) -> (bool, String, Option<String>) {
        let path = Path::new(video_file);
        if !path.exists() {
            return (false, format!("File not found: {}", video_file), None);
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown");

        // Extract title and year
        let (title, year) = extract_title_year(filename);

        // Create destination directory
        let dest_dir =
            Path::new(&self.config.paths.movies_dir).join(format!("{} ({})", title, year));

        if let Err(e) = std::fs::create_dir_all(&dest_dir) {
            return (false, format!("Failed to create directory: {}", e), None);
        }

        // Move file
        let dest_path = dest_dir.join(path.file_name().unwrap());
        if let Err(e) = std::fs::copy(path, &dest_path) {
            return (false, format!("Failed to copy: {}", e), None);
        }

        // Remove original
        let _ = std::fs::remove_file(path);

        let dest_str = dest_path.to_string_lossy().to_string();
        (true, format!("Organized: {}", title), Some(dest_str))
    }

    /// Organize a TV episode
    pub fn organize_episode(&self, video_file: &str) -> (bool, String) {
        let path = Path::new(video_file);
        if !path.exists() {
            return (false, "File not found".to_string());
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown");

        // Parse S01E02 pattern
        let (show_name, season, episode) = match parse_show_episode(filename) {
            Some((s, se, e)) => (s, se, e),
            None => return (false, "Could not parse filename".to_string()),
        };

        // Create destination directory: Show Name/Season X/
        let season_dir = format!("Season {}", season);
        let dest_dir = Path::new(&self.config.paths.shows_dir)
            .join(&show_name)
            .join(season_dir);

        if let Err(e) = std::fs::create_dir_all(&dest_dir) {
            return (false, format!("Failed to create directory: {}", e));
        }

        // Move file
        let dest_path = dest_dir.join(path.file_name().unwrap());
        if let Err(e) = std::fs::copy(path, &dest_path) {
            return (false, format!("Failed to copy: {}", e));
        }

        // Remove original
        let _ = std::fs::remove_file(path);

        (
            true,
            format!("Organized: {} S{:02}E{:02}", show_name, season, episode),
        )
    }

    /// Organize anime
    pub fn organize_anime(
        &self,
        files: Vec<String>,
        show_name: &str,
        season: u32,
    ) -> (u32, Vec<String>) {
        let mut count = 0;
        let mut messages = Vec::new();

        let dest_dir = Path::new(&self.config.paths.anime_dir)
            .join(show_name)
            .join(format!("Season {}", season));

        if let Err(e) = std::fs::create_dir_all(&dest_dir) {
            messages.push(format!("Failed to create directory: {}", e));
            return (0, messages);
        }

        for file in files {
            let path = Path::new(&file);
            if !path.exists() {
                continue;
            }

            let dest_path = dest_dir.join(path.file_name().unwrap());
            match std::fs::copy(path, &dest_path) {
                Ok(_) => {
                    let _ = std::fs::remove_file(path);
                    count += 1;
                }
                Err(e) => {
                    messages.push(format!("Failed to copy {}: {}", path.display(), e));
                }
            }
        }

        (count, messages)
    }
}

/// Check if filename is a video file
fn is_video_file(path: &Path) -> bool {
    let video_extensions = ["mkv", "mp4", "avi", "mov", "wmv", "flv", "webm", "m4v"];
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| video_extensions.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Check if filename is anime
fn is_anime_filename(name: &str) -> bool {
    // Common anime release group patterns
    name.contains("[") && name.contains("]")
        || name.contains("E субтитры")
        || name.contains("anime")
}

/// Parse anime name (simplified)
fn parse_anime_name(name: &str) -> (String, u32) {
    // Remove everything in brackets and after
    let name = regex::Regex::new(r"\[.*?\]").unwrap().replace(name, "");
    let name = regex::Regex::new(r"\s*сезон\s*\d+")
        .unwrap()
        .replace(&name, "");
    let name = name.trim().to_string();

    (name, 1)
}

/// Parse show episode (S01E02 or Season 1 Episode 2)
fn parse_show_episode(name: &str) -> Option<(String, u32, u32)> {
    // Try S01E02 pattern
    let re = regex::Regex::new(r"(?i)(.+?)[.\s_-]*[sS](\d+)[eE](\d+)").unwrap();
    if let Some(caps) = re.captures(name) {
        let title = caps.get(1)?.as_str().trim().to_string();
        let season: u32 = caps.get(2)?.as_str().parse().ok()?;
        let episode: u32 = caps.get(3)?.as_str().parse().ok()?;
        return Some((title, season, episode));
    }

    None
}

/// Extract title and year from filename
fn extract_title_year(name: &str) -> (String, String) {
    let re = regex::Regex::new(r"[\(\[\s]?((?:19|20)\d{2})[\)\]\s]?").unwrap();
    if let Some(m) = re.find(name) {
        let title = name[..m.start()].trim();
        let year = m
            .as_str()
            .trim_matches(|c| c == '(' || c == ')' || c == '[' || c == ']');
        (title.to_string(), year.to_string())
    } else {
        (name.trim().to_string(), "Unknown".to_string())
    }
}

/// Check if filename looks like a movie
fn is_movie_filename(name: &str) -> bool {
    name.contains("1080p")
        || name.contains("720p")
        || name.contains("2160p")
        || name.contains("4k")
        || name.contains("bluray")
        || name.contains("bdrip")
        || name.contains("webrip")
}
