//! MediaStation Core - TUI application and business logic

pub mod config;
pub mod models;
pub mod services;
pub mod ui;

use std::panic;

use crate::config::AppConfig;
use crate::ui::app::run_app;

/// Run the TUI application
pub fn run_tui(config_path: &str, theme_name: &str) -> anyhow::Result<()> {
    // Set up panic hook for logging
    let default_panic = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        log::error!("Application panic: {}", panic_info);
        default_panic(panic_info);
    }));

    // Load configuration - fall back to defaults if file is missing/corrupt
    let config = match AppConfig::load(config_path) {
        Ok(c) => c,
        Err(_) => {
            let config = AppConfig::default_with_path(config_path);
            // Save defaults so Settings panel edits persist
            let _ = config.save();
            config
        }
    };

    // Run the TUI
    run_app(config, theme_name)
}
