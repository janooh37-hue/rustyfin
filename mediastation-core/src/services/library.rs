//! Library service - Manage media library (movies, shows, anime)

use std::path::Path;

use crate::config::AppConfig;
use crate::models::{Anime, Episode, Movie, Season, Show};

/// Library service for file system operations
pub struct LibraryService {
    config: AppConfig,
}

impl LibraryService {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    /// Get all movies
    pub fn get_movies(&self) -> Vec<Movie> {
        let mut movies = Vec::new();
        let movies_dir = Path::new(&self.config.paths.movies_dir);

        if !movies_dir.exists() {
            return movies;
        }

        if let Ok(entries) = std::fs::read_dir(movies_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(video_file) = self.find_video_file(&path) {
                        let title = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown")
                            .to_string();

                        let added_at = entry
                            .metadata()
                            .ok()
                            .and_then(|m| m.modified().ok())
                            .map(|t| {
                                t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64
                            })
                            .unwrap_or(0);

                        movies.push(Movie {
                            title: clean_title(&title),
                            year: extract_year(&title),
                            path: path.to_string_lossy().to_string(),
                            video_file: Some(video_file),
                            resolution: None,
                            has_subtitle: self.has_subtitle(&path),
                            added_at,
                        });
                    }
                }
            }
        }

        // Sort by added date (newest first)
        movies.sort_by(|a, b| b.added_at.cmp(&a.added_at));
        movies
    }

    /// Get all TV shows
    pub fn get_shows(&self) -> Vec<Show> {
        let mut shows = Vec::new();
        let shows_dir = Path::new(&self.config.paths.shows_dir);

        if !shows_dir.exists() {
            return shows;
        }

        if let Ok(entries) = std::fs::read_dir(shows_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let title = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    let seasons = self.get_seasons(&path);

                    let added_at = entry
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64)
                        .unwrap_or(0);

                    shows.push(Show {
                        title: clean_title(&title),
                        year: extract_year(&title),
                        path: path.to_string_lossy().to_string(),
                        seasons,
                        added_at,
                    });
                }
            }
        }

        shows.sort_by(|a, b| a.title.cmp(&b.title));
        shows
    }

    /// Get all anime
    pub fn get_anime(&self) -> Vec<Anime> {
        let mut anime = Vec::new();
        let anime_dir = Path::new(&self.config.paths.anime_dir);

        if !anime_dir.exists() {
            return anime;
        }

        if let Ok(entries) = std::fs::read_dir(anime_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let title = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    let seasons = self.get_seasons(&path);

                    let added_at = entry
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64)
                        .unwrap_or(0);

                    anime.push(Anime {
                        title: clean_title(&title),
                        path: path.to_string_lossy().to_string(),
                        seasons,
                        added_at,
                    });
                }
            }
        }

        anime.sort_by(|a, b| a.title.cmp(&b.title));
        anime
    }

    /// Get all recent items (movies, shows, anime) sorted by modification time
    pub fn get_recent_all(&self) -> Vec<crate::models::RecentItem> {
        let mut items: Vec<crate::models::RecentItem> = Vec::new();

        for movie in self.get_movies() {
            items.push(crate::models::RecentItem {
                title: movie.title,
                year: movie.year,
                path: movie.path,
                video_file: movie.video_file,
                has_subtitle: movie.has_subtitle,
                media_type: crate::models::MediaType::Movie,
                added_at: movie.added_at,
            });
        }

        for show in self.get_shows() {
            items.push(crate::models::RecentItem {
                title: show.title,
                year: show.year,
                path: show.path,
                video_file: None,
                has_subtitle: false,
                media_type: crate::models::MediaType::Show,
                added_at: show.added_at,
            });
        }

        for anime in self.get_anime() {
            items.push(crate::models::RecentItem {
                title: anime.title,
                year: None,
                path: anime.path,
                video_file: None,
                has_subtitle: false,
                media_type: crate::models::MediaType::Anime,
                added_at: anime.added_at,
            });
        }

        items.sort_by(|a, b| b.added_at.cmp(&a.added_at));
        items
    }

    /// Get seasons for a show/anime directory
    fn get_seasons(&self, path: &Path) -> Vec<Season> {
        let mut seasons = Vec::new();

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    let name = entry_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");

                    // Match Season X or Season 0X
                    if let Some(season_num) = parse_season_number(name) {
                        let episodes = self.get_episodes(&entry_path, season_num);
                        seasons.push(Season {
                            number: season_num,
                            episodes,
                        });
                    }
                }
            }
        }

        seasons.sort_by(|a, b| a.number.cmp(&b.number));
        seasons
    }

    /// Get episodes in a season directory
    fn get_episodes(&self, path: &Path, _season: u32) -> Vec<Episode> {
        let mut episodes = Vec::new();

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if is_video_file(&entry_path) {
                    let name = entry_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");

                    if let Some(ep_num) = parse_episode_number(name) {
                        episodes.push(Episode {
                            number: ep_num,
                            title: None,
                            path: entry_path.to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }

        episodes.sort_by(|a, b| a.number.cmp(&b.number));
        episodes
    }

    /// Find video file in directory
    fn find_video_file(&self, path: &Path) -> Option<String> {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if is_video_file(&entry_path) {
                    return Some(entry_path.to_string_lossy().to_string());
                }
            }
        }
        None
    }

    /// Check if directory has subtitle files
    fn has_subtitle(&self, path: &Path) -> bool {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "srt" || ext == "ass" || ext == "ssa" || ext == "vtt" {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Delete a movie
    pub fn delete_movie(&self, path: &str) -> (bool, String) {
        let path = Path::new(path);
        if !path.exists() {
            return (false, "Path does not exist".to_string());
        }

        match std::fs::remove_dir_all(path) {
            Ok(_) => (true, format!("Deleted: {}", path.display())),
            Err(e) => (false, format!("Failed to delete: {}", e)),
        }
    }

    /// Delete a show
    pub fn delete_show(&self, path: &str) -> (bool, String) {
        self.delete_movie(path)
    }

    /// Delete anime
    pub fn delete_anime(&self, path: &str) -> (bool, String) {
        self.delete_movie(path)
    }

    /// Get library statistics
    pub fn get_stats(&self) -> crate::models::LibraryStats {
        let movies = self.get_movies();
        let shows = self.get_shows();
        let anime = self.get_anime();

        let total_size = self.calculate_library_size();
        let missing_subs = movies.iter().filter(|m| !m.has_subtitle).count() as u32;

        crate::models::LibraryStats {
            movies_count: movies.len() as u32,
            shows_count: shows.len() as u32,
            anime_count: anime.len() as u32,
            total_size_gb: (total_size / (1024 * 1024 * 1024)) as f64,
            missing_subs,
        }
    }

    fn calculate_library_size(&self) -> u64 {
        let mut total = 0u64;

        for dir in [
            &self.config.paths.movies_dir,
            &self.config.paths.shows_dir,
            &self.config.paths.anime_dir,
        ] {
            let path = Path::new(dir);
            let entries = walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok());

            for entry in entries {
                if entry.file_type().is_file() {
                    total += entry.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
        }

        total
    }
}

/// Check if file is a video
fn is_video_file(path: &Path) -> bool {
    let video_extensions = ["mkv", "mp4", "avi", "mov", "wmv", "flv", "webm", "m4v"];
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| video_extensions.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Clean title (remove year, resolution, etc)
fn clean_title(title: &str) -> String {
    let re = regex::Regex::new(r"[\(\[\s]?((?:19|20)\d{2})[\)\]\s]?").unwrap();
    let title = re.replace(title, "");
    let re2 = regex::Regex::new(
        r"(2160p|1080p|720p|480p|4k|uhd|brip|bdrip|webrip|web-dl|bluray|blu-ray)",
    )
    .unwrap();
    let title = re2.replace(&title, "");
    title.trim().to_string()
}

/// Extract year from title
fn extract_year(title: &str) -> Option<i32> {
    let re = regex::Regex::new(r"(?:19|20)\d{2}").unwrap();
    re.find(title).and_then(|m| m.as_str().parse().ok())
}

/// Parse season number from directory name
fn parse_season_number(name: &str) -> Option<u32> {
    let re = regex::Regex::new(r"(?i)season\s*(\d+)").unwrap();
    re.captures(name)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

/// Parse episode number from filename
fn parse_episode_number(name: &str) -> Option<u32> {
    let re = regex::Regex::new(r"(?i)[sS]\d+[eE](\d+)").unwrap();
    re.captures(name)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}
