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

    // Load configuration
    let config = AppConfig::load(config_path)?;

    // Run the TUI
    run_app(config, theme_name)
}