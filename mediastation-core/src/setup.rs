//! First-run setup wizard for RustyFin
//!
//! Runs an interactive terminal prompt (stdin/stdout, not TUI) to collect
//! configuration when no config file exists yet.

use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::config::{
    PathsConfig, QBittorrentConfig, Settings, TVSettings, TelegramConfig, TraktConfig,
};

/// Run the first-run setup wizard if no config file exists.
///
/// If the config file already exists at `config_path`, this returns
/// immediately. Otherwise it prompts the user through an interactive
/// setup, writes the resulting JSON config, and returns.
pub fn run_setup(config_path: &str) -> anyhow::Result<()> {
    if Path::new(config_path).exists() {
        return Ok(());
    }

    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "/home/user".to_string());

    let stdin = io::stdin();
    let mut reader = stdin.lock();

    println!();
    println!("=== RustyFin First-Run Setup ===");
    println!("Press Enter to accept defaults shown in [brackets].");
    println!("You can change all settings later from the Settings panel.");
    println!();

    // -- Media Paths --
    println!("-- Media Paths --");

    let default_movies = format!("{}/media/movies", home);
    let movies_dir = prompt(&mut reader, "Movies directory", &default_movies)?;

    let default_shows = format!("{}/media/tv", home);
    let shows_dir = prompt(&mut reader, "TV Shows directory", &default_shows)?;

    let default_anime = format!("{}/media/anime", home);
    let anime_dir = prompt(&mut reader, "Anime directory", &default_anime)?;

    let default_downloads = format!("{}/Downloads", home);
    let download_dir = prompt(&mut reader, "Downloads directory", &default_downloads)?;

    // Create all media directories
    for (label, dir) in [
        ("Movies", &movies_dir),
        ("Shows", &shows_dir),
        ("Anime", &anime_dir),
        ("Downloads", &download_dir),
    ] {
        match std::fs::create_dir_all(dir) {
            Ok(_) => println!("  Created: {}", dir),
            Err(e) => println!("  Warning: Could not create {} dir ({}): {}", label, dir, e),
        }
    }

    println!();

    // -- qBittorrent Connection --
    println!("-- qBittorrent Connection --");

    let qbt_host = prompt(&mut reader, "qBittorrent host", "http://localhost:8080")?;
    let qbt_username = prompt(&mut reader, "qBittorrent username", "admin")?;
    let qbt_password = prompt(&mut reader, "qBittorrent password", "")?;

    println!();

    // -- Trakt.tv (optional) --
    println!("-- Trakt.tv (optional, press Enter to skip) --");

    let trakt_username = prompt_optional(&mut reader, "Trakt username")?;
    let trakt_client_id = prompt_optional(&mut reader, "Trakt client ID")?;

    println!();

    // -- Quality Preferences --
    println!("-- Quality Preferences --");

    let quality_str = prompt(
        &mut reader,
        "Quality priority (comma-separated)",
        "2160p,1080p",
    )?;
    let quality_priority: Vec<String> = quality_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let max_size_str = prompt(&mut reader, "Max file size in GB", "10")?;
    let max_size_gb: u32 = max_size_str.parse().unwrap_or(10);

    let avoid_cam_str = prompt(&mut reader, "Avoid CAM releases? (y/n)", "y")?;
    let avoid_cam = !avoid_cam_str.eq_ignore_ascii_case("n");

    println!();

    // Build the config structures
    let paths = PathsConfig {
        download_dir,
        movies_dir,
        shows_dir,
        anime_dir,
    };

    let qbittorrent = QBittorrentConfig {
        host: qbt_host,
        username: qbt_username,
        password: qbt_password,
    };

    let trakt = TraktConfig {
        username: trakt_username,
        client_id: trakt_client_id,
    };

    let settings = Settings {
        quality_priority,
        max_size_gb,
        avoid_cam,
        ..Settings::default()
    };

    let tv_settings = TVSettings::default();
    let telegram = TelegramConfig::default();

    // Serialize to JSON
    let config_json = serde_json::json!({
        "trakt": trakt,
        "qbittorrent": qbittorrent,
        "telegram": telegram,
        "paths": paths,
        "settings": settings,
        "tv_settings": tv_settings,
    });

    let content = serde_json::to_string_pretty(&config_json)?;

    // Create the config directory if it doesn't exist
    if let Some(parent) = Path::new(config_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write the config file
    std::fs::write(config_path, content)?;

    println!("Config saved to: {}", config_path);
    println!("Setup complete! Starting RustyFin...");
    println!();

    Ok(())
}

/// Prompt the user for a value, showing a default in brackets.
/// If the user presses Enter without typing anything, the default is returned.
fn prompt(reader: &mut impl BufRead, label: &str, default: &str) -> io::Result<String> {
    if default.is_empty() {
        print!("{}: ", label);
    } else {
        print!("{} [{}]: ", label, default);
    }
    io::stdout().flush()?;

    let mut input = String::new();
    reader.read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(input.to_string())
    }
}

/// Prompt for an optional value (no default). Returns an empty string if skipped.
fn prompt_optional(reader: &mut impl BufRead, label: &str) -> io::Result<String> {
    print!("{}: ", label);
    io::stdout().flush()?;

    let mut input = String::new();
    reader.read_line(&mut input)?;
    Ok(input.trim().to_string())
}
