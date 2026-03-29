//! RustyFin CLI - Entry point for the TUI application

use clap::Parser;
use log::info;

/// Get the default config path: ~/.config/rustyfin/config.json
fn default_config_path() -> String {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
        .join("rustyfin")
        .join("config.json")
        .to_string_lossy()
        .to_string()
}

/// Get the default log directory: ~/.config/rustyfin/
fn default_log_dir() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
        .join("rustyfin")
}

/// CLI arguments for RustyFin
#[derive(Parser, Debug)]
#[command(name = "rustyfin")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "RustyFin - A TUI application for managing your Jellyfin media server", long_about = None)]
pub struct Args {
    /// Config file path
    #[arg(short, long, default_value_t = default_config_path())]
    pub config: String,

    /// Enable verbose logging
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Theme to use (catppuccin, dracula, gruvbox, nord, rosepine)
    #[arg(short, long, default_value = "gruvbox")]
    pub theme: String,
}

fn main() -> anyhow::Result<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // Initialize logger - write to file to keep terminal clean for TUI
    let log_dir = default_log_dir();
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("rustyfin.log"))
        .unwrap_or_else(|_| {
            #[cfg(windows)]
            {
                std::fs::File::create("NUL").unwrap()
            }
            #[cfg(not(windows))]
            {
                std::fs::File::create("/dev/null").unwrap()
            }
        });

    let log_level = match args.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();

    info!("Starting RustyFin v{}", env!("CARGO_PKG_VERSION"));
    info!("Using config: {}", args.config);
    info!("Theme: {}", args.theme);

    // Run the TUI application
    mediastation_core::run_tui(&args.config, &args.theme)?;

    Ok(())
}
