//! MediaStation Core - TUI application and business logic

pub mod config;
pub mod models;
pub mod services;
pub mod setup;
pub mod ui;

use std::panic;

use crate::config::AppConfig;
use crate::setup::run_setup;
use crate::ui::app::run_app;

/// Run the TUI application
pub fn run_tui(config_path: &str, theme_name: &str) -> anyhow::Result<()> {
    // Set up panic hook for logging
    let default_panic = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        log::error!("Application panic: {}", panic_info);
        default_panic(panic_info);
    }));

    // Run first-run setup if no config file exists
    if let Err(e) = run_setup(config_path) {
        eprintln!("Setup wizard error: {}. Creating default config...", e);
        // Create a minimal default config so the app can still launch
        let config = AppConfig::default_with_path(config_path);
        if let Err(e2) = config.save() {
            eprintln!("Warning: Could not save default config: {}", e2);
        }
    }

    // Load configuration - fall back to defaults if file is corrupt/missing
    let config = match AppConfig::load(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Config error: {}. Starting with defaults...", e);
            eprintln!("Use Settings panel to configure the app.");
            let config = AppConfig::default_with_path(config_path);
            // Try to save the default config for next time
            let _ = config.save();
            config
        }
    };

    // Run the TUI
    run_app(config, theme_name)
}
