//! Main application runner

use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ratatui::backend::CrosstermBackend;
use ratatui::{Terminal, TerminalOptions, Viewport};

use crate::config::AppConfig;
use crate::models::{PendingItem, PendingType};
use crate::services::{
    library::LibraryService, organize::OrganizeService, qbittorrent::QBittorrentService,
    search::TorrentSearchService, subtitle::SubtitleService, trakt::TraktService,
};
use crate::ui::render::render;
use crate::ui::state::{AppMode, AppState, Focus, FocusedPanel, LibraryView, SettingsEntry};
use crate::ui::theme::Theme;

const THEME_NAMES: &[&str] = &["catppuccin", "dracula", "gruvbox", "nord", "rosepine"];

/// Run the TUI application
pub fn run_app(config: AppConfig, theme_name: &str) -> anyhow::Result<()> {
    crossterm::terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(io::stdout());
    let options = TerminalOptions {
        viewport: Viewport::Fullscreen,
        ..Default::default()
    };
    let mut terminal = Terminal::with_options(backend, options)?;

    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::Hide
    )?;
    terminal.clear()?;

    let mut state = AppState::new();
    state.theme_name = theme_name.to_string();
    state.search_indexers = config.settings.search_indexers.clone();
    let mut theme = Theme::from_name(theme_name);

    let config = Arc::new(config);
    let mut qbit_service = QBittorrentService::new(&config.qbittorrent);
    let library_service = LibraryService::new(config.as_ref().clone());
    let mut search_service = TorrentSearchService::new(config.as_ref().clone());
    let organize_service = OrganizeService::new(config.as_ref().clone());
    let trakt_service = TraktService::new(config.as_ref().clone());
    let subtitle_service = SubtitleService::new(config.as_ref().clone());

    // Load initial data
    load_all_data(&mut state, &config, &mut qbit_service, &library_service, &organize_service, &trakt_service);
    build_settings_entries(&mut state, &config);
    state.settings_select_first();

    let result = run_loop(
        &mut terminal,
        &mut state,
        &mut theme,
        config,
        &mut qbit_service,
        library_service,
        &mut search_service,
        organize_service,
        trakt_service,
        subtitle_service,
    );

    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;
    crossterm::terminal::disable_raw_mode()?;
    println!("\nGoodbye!");

    if let Err(e) = &result {
        if e.to_string() != "quit" {
            return result;
        }
    }
    Ok(())
}

/// Main event loop
fn run_loop<B: ratatui::backend::Backend + io::Write>(
    terminal: &mut Terminal<B>,
    state: &mut AppState,
    theme: &mut Theme,
    config: Arc<AppConfig>,
    qbit_service: &mut QBittorrentService,
    library_service: LibraryService,
    search_service: &mut TorrentSearchService,
    organize_service: OrganizeService,
    trakt_service: TraktService,
    subtitle_service: SubtitleService,
) -> anyhow::Result<()> {
    use crossterm::event::{self, Event, KeyEventKind};

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    // Initial qBittorrent load (may trigger login)
    load_downloads(state, qbit_service, &rt);

    // Load watchlist async
    let watchlist = rt.block_on(trakt_service.get_watchlist());
    state.watchlist.update(watchlist);

    let mut last_refresh = Instant::now();
    let refresh_interval = Duration::from_secs(5);

    loop {
        if state.needs_clear {
            terminal.clear()?;
            state.needs_clear = false;
        }

        terminal.draw(|f| render(f, state, theme))?;

        // Auto-refresh downloads
        if last_refresh.elapsed() >= refresh_interval {
            load_downloads(state, qbit_service, &rt);
            last_refresh = Instant::now();
        }

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let should_quit = handle_key_event(
                        key, state, &config, qbit_service, &library_service,
                        search_service, &organize_service, &trakt_service,
                        &subtitle_service, &rt, terminal, theme,
                    );
                    if should_quit {
                        return Ok(());
                    }
                }
                Event::Resize(_, _) => {
                    state.needs_clear = true;
                }
                _ => {}
            }
        }
    }
}

/// Load all data on startup or full refresh
fn load_all_data(
    state: &mut AppState,
    config: &Arc<AppConfig>,
    qbit: &mut QBittorrentService,
    library: &LibraryService,
    organize: &OrganizeService,
    _trakt: &TraktService,
) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime");

    // Downloads
    load_downloads(state, qbit, &rt);

    // Library data (movies, shows, anime)
    load_library_data(state, library);

    // Pending items: combine download dir scan + completed torrents
    load_pending_items(state, qbit, organize, config, &rt);

    // Library stats
    state.library_stats = library.get_stats();
}

/// Load downloads from QBittorrent
fn load_downloads(
    state: &mut AppState,
    qbit: &mut QBittorrentService,
    rt: &tokio::runtime::Runtime,
) {
    let torrents = rt.block_on(qbit.get_torrents(None));
    state.torrents.update(torrents);
    let info = rt.block_on(qbit.get_transfer_info());
    state.transfer_info = info;
}

/// Load pending items from both the download directory and completed torrents
fn load_pending_items(
    state: &mut AppState,
    qbit: &mut QBittorrentService,
    organize: &OrganizeService,
    config: &Arc<AppConfig>,
    rt: &tokio::runtime::Runtime,
) {
    let mut items: Vec<PendingItem> = Vec::new();

    // 1. Scan download directory for video files
    let dir_items = organize.get_pending();
    items.extend(dir_items);

    // 2. Get completed/seeding torrents from qBittorrent
    let completed = rt.block_on(qbit.get_completed(None));
    for torrent in completed {
        // Check if already in pending list (by name match)
        let already_listed = items.iter().any(|p| {
            p.video_file.contains(&torrent.name)
                || torrent.name.contains(
                    std::path::Path::new(&p.video_file)
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or(""),
                )
        });
        if !already_listed {
            // Use content_path from qBittorrent API (full path to file/folder)
            let content = if !torrent.content_path.is_empty() {
                std::path::PathBuf::from(&torrent.content_path)
            } else {
                // Fallback: construct from save_path or download_dir
                let base = if !torrent.save_path.is_empty() {
                    &torrent.save_path
                } else {
                    config.paths.download_dir.as_str()
                };
                std::path::Path::new(base).join(&torrent.name)
            };

            let video_file = if content.is_file() && is_video_path(&content) {
                // Direct video file
                content.to_string_lossy().to_string()
            } else if content.is_dir() {
                // Folder - find the largest video file inside
                find_largest_video_in_dir(&content)
                    .unwrap_or_else(|| content.to_string_lossy().to_string())
            } else if content.is_file() {
                // Non-video file (e.g. .rar), skip
                continue;
            } else {
                // Path doesn't exist on disk, skip
                log::warn!("Torrent content path not found: {}", content.display());
                continue;
            };
            items.push(PendingItem {
                video_file,
                show_name: None,
                season: None,
                episode: None,
                pending_type: detect_torrent_type(&torrent.name),
            });
        }
    }

    state.pending_items.update(items);
}

/// Find the largest video file in a directory (recursive one level)
fn find_largest_video_in_dir(dir: &std::path::Path) -> Option<String> {
    let video_extensions = ["mkv", "mp4", "avi", "mov", "wmv", "flv", "webm", "m4v"];
    let mut best: Option<(u64, String)> = None;

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if video_extensions.contains(&ext.to_lowercase().as_str()) {
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        if best.as_ref().map(|(s, _)| size > *s).unwrap_or(true) {
                            best = Some((size, path.to_string_lossy().to_string()));
                        }
                    }
                }
            }
        }
    }

    best.map(|(_, path)| path)
}

fn is_video_path(path: &std::path::Path) -> bool {
    let video_extensions = ["mkv", "mp4", "avi", "mov", "wmv", "flv", "webm", "m4v"];
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| video_extensions.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Load library data (movies, shows, anime) and update the library list
fn load_library_data(state: &mut AppState, library: &LibraryService) {
    state.library_movies = library.get_movies();
    state.library_shows = library.get_shows();
    state.library_anime = library.get_anime();
    // Reset to categories view and rebuild display list
    state.library_view = LibraryView::Categories;
    refresh_library_list(state);
}

/// Refresh the library_list display strings based on current library_view
fn refresh_library_list(state: &mut AppState) {
    let items: Vec<String> = match &state.library_view {
        LibraryView::Categories => {
            vec![
                format!("Movies ({})", state.library_movies.len()),
                format!("Shows ({})", state.library_shows.len()),
                format!("Anime ({})", state.library_anime.len()),
            ]
        }
        LibraryView::Movies => {
            state.library_movies.iter().map(|m| {
                let year = m.year.map(|y| format!(" ({})", y)).unwrap_or_default();
                let subs = if m.has_subtitle { " [subs]" } else { "" };
                format!("{}{}{}", m.title, year, subs)
            }).collect()
        }
        LibraryView::Shows => {
            state.library_shows.iter().map(|s| {
                let year = s.year.map(|y| format!(" ({})", y)).unwrap_or_default();
                let seasons = s.seasons.len();
                format!("{}{} - {} season(s)", s.title, year, seasons)
            }).collect()
        }
        LibraryView::Anime => {
            state.library_anime.iter().map(|a| {
                let seasons = a.seasons.len();
                format!("{} - {} season(s)", a.title, seasons)
            }).collect()
        }
        LibraryView::ShowDetail(idx) => {
            if let Some(show) = state.library_shows.get(*idx) {
                show.seasons.iter().map(|s| {
                    format!("Season {} ({} episodes)", s.number, s.episodes.len())
                }).collect()
            } else {
                vec![]
            }
        }
        LibraryView::AnimeDetail(idx) => {
            if let Some(anime) = state.library_anime.get(*idx) {
                anime.seasons.iter().map(|s| {
                    format!("Season {} ({} episodes)", s.number, s.episodes.len())
                }).collect()
            } else {
                vec![]
            }
        }
        LibraryView::SeasonEpisodes { is_anime, parent_idx, season_idx } => {
            let seasons = if *is_anime {
                state.library_anime.get(*parent_idx).map(|a| &a.seasons)
            } else {
                state.library_shows.get(*parent_idx).map(|s| &s.seasons)
            };
            if let Some(seasons) = seasons {
                if let Some(season) = seasons.get(*season_idx) {
                    season.episodes.iter().map(|e| {
                        let title = e.title.as_deref().unwrap_or("");
                        if title.is_empty() {
                            format!("E{:02} - {}", e.number, std::path::Path::new(&e.path).file_name().and_then(|n| n.to_str()).unwrap_or(&e.path))
                        } else {
                            format!("E{:02} - {}", e.number, title)
                        }
                    }).collect()
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        }
    };
    state.library_list.update(items);
}

/// Detect media type from torrent name
fn detect_torrent_type(name: &str) -> PendingType {
    let lower = name.to_lowercase();
    // Anime: brackets in name
    if lower.contains('[') && lower.contains(']') {
        return PendingType::Anime;
    }
    // Show: S01E02 pattern
    if regex::Regex::new(r"(?i)[sS]\d+[eE]\d+").unwrap().is_match(&lower) {
        return PendingType::Show;
    }
    PendingType::Movie
}

/// Build settings entries from config
fn build_settings_entries(state: &mut AppState, config: &Arc<AppConfig>) {
    let mut entries = Vec::new();

    // Theme section
    entries.push(SettingsEntry {
        key: "── Appearance ──".into(),
        value: String::new(),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "Theme".into(),
        value: format!("{} (Enter to change)", state.theme_name),
        editable: true,
    });

    // Library section
    entries.push(SettingsEntry {
        key: "── Library ──".into(),
        value: String::new(),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "Movies".into(),
        value: format!("{} items", state.library_stats.movies_count),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "Shows".into(),
        value: format!("{} items", state.library_stats.shows_count),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "Anime".into(),
        value: format!("{} items", state.library_stats.anime_count),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "Total Size".into(),
        value: format!("{:.1} GB", state.library_stats.total_size_gb),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "Missing Subs".into(),
        value: format!("{}", state.library_stats.missing_subs),
        editable: false,
    });

    // Paths section
    entries.push(SettingsEntry {
        key: "── Paths ──".into(),
        value: String::new(),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "Downloads".into(),
        value: config.paths.download_dir.clone(),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "Movies Dir".into(),
        value: config.paths.movies_dir.clone(),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "Shows Dir".into(),
        value: config.paths.shows_dir.clone(),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "Anime Dir".into(),
        value: config.paths.anime_dir.clone(),
        editable: false,
    });

    // Quality section
    entries.push(SettingsEntry {
        key: "── Quality ──".into(),
        value: String::new(),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "Priority".into(),
        value: config.settings.quality_priority.join(", "),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "Max Size".into(),
        value: format!("{} GB", config.settings.max_size_gb),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "Min Seeds".into(),
        value: format!("{}", config.settings.min_seeds),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "Avoid CAM".into(),
        value: if config.settings.avoid_cam { "Yes" } else { "No" }.into(),
        editable: false,
    });

    // Search Indexers section
    entries.push(SettingsEntry {
        key: "── Search Indexers ──".into(),
        value: String::new(),
        editable: false,
    });
    let available_indexers = ["yts", "tpb", "1337x"];
    for indexer in &available_indexers {
        let enabled = state.search_indexers.iter().any(|i| i == indexer);
        entries.push(SettingsEntry {
            key: indexer.to_string(),
            value: if enabled { "Enabled (Enter to toggle)".into() } else { "Disabled (Enter to toggle)".into() },
            editable: true,
        });
    }

    // Connection section
    entries.push(SettingsEntry {
        key: "── Connection ──".into(),
        value: String::new(),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "qBittorrent".into(),
        value: config.qbittorrent.host.to_string(),
        editable: false,
    });
    entries.push(SettingsEntry {
        key: "Trakt User".into(),
        value: if config.trakt.username.is_empty() {
            "Not configured".into()
        } else {
            config.trakt.username.to_string()
        },
        editable: false,
    });

    state.settings_entries.update(entries);
}

/// Handle key events - dispatch based on current mode
fn handle_key_event<B: ratatui::backend::Backend + io::Write>(
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    config: &Arc<AppConfig>,
    qbit: &mut QBittorrentService,
    library: &LibraryService,
    search: &mut TorrentSearchService,
    organize: &OrganizeService,
    trakt: &TraktService,
    subtitle: &SubtitleService,
    rt: &tokio::runtime::Runtime,
    terminal: &mut Terminal<B>,
    theme: &mut Theme,
) -> bool {
    match state.mode {
        AppMode::Search => handle_search_mode(key, state, qbit, search, rt),
        AppMode::Help => handle_help_mode(key, state),
        AppMode::Normal => handle_normal_mode(key, state, config, qbit, library, search, organize, trakt, subtitle, rt, terminal, theme),
        AppMode::Confirm => { handle_confirm_mode(key, state, qbit, rt); false }
        _ => false,
    }
}

/// Handle keys in Search mode
fn handle_search_mode(
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    qbit: &mut QBittorrentService,
    search: &mut TorrentSearchService,
    rt: &tokio::runtime::Runtime,
) -> bool {
    use crossterm::event::KeyCode;
    match key.code {
        KeyCode::Esc => { state.set_mode(AppMode::Normal); }
        KeyCode::Enter => {
            if !state.search_results.items.is_empty() {
                if let Some(result) = state.search_results.selected() {
                    let name = result.name.clone();
                    let magnet = result.magnet.clone();
                    state.set_status(format!("Adding: {}", truncate_name(&name, 50)));
                    let _ = rt.block_on(qbit.add_torrent(&magnet, ""));
                    load_downloads(state, qbit, rt);
                    state.set_status(format!("Download started: {}", truncate_name(&name, 50)));
                    state.set_mode(AppMode::Normal);
                    state.focus = Focus::Content;
                    state.focused_panel = FocusedPanel::Downloads;
                    state.needs_clear = true;
                }
            } else if !state.search_query.is_empty() {
                let query = state.search_query.clone();
                state.set_status(format!("Searching '{}'...", query));
                state.is_searching = true;
                let results = rt.block_on(search.search(&query, "movie", &state.search_indexers));
                state.is_searching = false;
                let count = results.len();
                state.search_results.update(results);
                state.set_status(format!("Found {} results for '{}'", count, query));
                state.needs_clear = true;
            }
        }
        KeyCode::Up => { state.search_results.select_previous(); }
        KeyCode::Down => { state.search_results.select_next(); }
        KeyCode::Backspace => {
            state.search_query.pop();
            if state.search_query.is_empty() {
                state.search_results.update(Vec::new());
            }
        }
        KeyCode::Char(c) => { state.search_query.push(c); }
        _ => {}
    }
    false
}

/// Handle keys in Help mode
fn handle_help_mode(key: crossterm::event::KeyEvent, state: &mut AppState) -> bool {
    use crossterm::event::KeyCode;
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
            state.set_mode(AppMode::Normal);
            state.needs_clear = true;
        }
        _ => {}
    }
    false
}

/// Handle keys in Confirm mode
fn handle_confirm_mode(
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    qbit: &mut QBittorrentService,
    rt: &tokio::runtime::Runtime,
) {
    use crossterm::event::KeyCode;
    match key.code {
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
            state.set_mode(AppMode::Normal);
            state.confirm_callback = None;
            state.confirm_data = None;
            state.set_status("Cancelled");
        }
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            // Handle delete confirmation
            if let Some(hash) = state.confirm_data.take() {
                if rt.block_on(qbit.delete_torrent(&hash, false)) {
                    state.set_status("Deleted successfully");
                    load_downloads(state, qbit, rt);
                } else {
                    state.set_status("Delete failed");
                }
                state.needs_clear = true;
            }
            if let Some(cb) = state.confirm_callback.take() { cb(); }
            state.set_mode(AppMode::Normal);
        }
        _ => {}
    }
}

/// Handle keys in Normal mode
fn handle_normal_mode<B: ratatui::backend::Backend + io::Write>(
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    config: &Arc<AppConfig>,
    qbit: &mut QBittorrentService,
    library: &LibraryService,
    search: &mut TorrentSearchService,
    organize: &OrganizeService,
    trakt: &TraktService,
    subtitle: &SubtitleService,
    rt: &tokio::runtime::Runtime,
    terminal: &mut Terminal<B>,
    theme: &mut Theme,
) -> bool {
    use crossterm::event::KeyCode;

    // Global keys
    match key.code {
        KeyCode::Char('q') => return true,
        KeyCode::Char('?') => { state.set_mode(AppMode::Help); return false; }
        KeyCode::Char('/') => {
            state.focused_panel = FocusedPanel::Search;
            state.focus = Focus::Content;
            state.set_mode(AppMode::Search);
            return false;
        }
        _ => {}
    }

    match state.focus {
        Focus::Sidebar => {
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => { state.next_panel(); }
                KeyCode::Up | KeyCode::Char('k') => { state.prev_panel(); }
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => { state.enter_content(); }
                KeyCode::Char('g') => { state.focused_panel = FocusedPanel::all()[0]; }
                KeyCode::Char('G') => {
                    let p = FocusedPanel::all();
                    state.focused_panel = p[p.len() - 1];
                }
                KeyCode::Char('r') => {
                    refresh_current_panel(state, config, qbit, library, search, organize, trakt, rt);
                }
                KeyCode::Char('R') => {
                    refresh_all(state, config, qbit, library, organize, trakt, rt);
                }
                _ => {}
            }
        }
        Focus::Content => {
            // Library panel has special navigation for drill-down
            if state.focused_panel == FocusedPanel::Library {
                match key.code {
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Esc => {
                        handle_library_back(state);
                    }
                    KeyCode::Down | KeyCode::Char('j') => { state.navigate_down(); }
                    KeyCode::Up | KeyCode::Char('k') => { state.navigate_up(); }
                    KeyCode::Char('g') => { state.navigate_first(); }
                    KeyCode::Char('G') => { state.navigate_last(); }
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
                        handle_library_enter(state, terminal);
                    }
                    KeyCode::Char('p') => {
                        handle_library_play(state, terminal);
                    }
                    KeyCode::Char('S') => {
                        handle_library_subtitle(state, subtitle, rt);
                    }
                    KeyCode::Char('i') => {
                        handle_library_info(state);
                    }
                    KeyCode::Char('r') => {
                        refresh_current_panel(state, config, qbit, library, search, organize, trakt, rt);
                    }
                    KeyCode::Char('R') => {
                        refresh_all(state, config, qbit, library, organize, trakt, rt);
                    }
                    _ => {}
                }
                return false;
            }

            match key.code {
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Esc => {
                    state.return_to_sidebar();
                }
                KeyCode::Down | KeyCode::Char('j') => { state.navigate_down(); }
                KeyCode::Up | KeyCode::Char('k') => { state.navigate_up(); }
                KeyCode::Char('g') => { state.navigate_first(); }
                KeyCode::Char('G') => { state.navigate_last(); }

                // Enter = primary action
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
                    handle_enter_action(state, config, qbit, library, search, organize, subtitle, rt, terminal, theme);
                }

                // Panel-specific actions
                KeyCode::Char('d') => {
                    if state.focused_panel == FocusedPanel::Downloads {
                        if let Some(t) = state.torrents.selected() {
                            let hash = t.hash.clone();
                            let name = t.name.clone();
                            state.set_status(format!("Deleting: {}", truncate_name(&name, 40)));
                            if rt.block_on(qbit.delete_torrent(&hash, true)) {
                                state.set_status(format!("Deleted: {}", truncate_name(&name, 40)));
                                load_downloads(state, qbit, rt);
                            } else {
                                state.set_status("Delete failed");
                            }
                            state.needs_clear = true;
                        }
                    }
                }
                KeyCode::Char('o') => {
                    if state.focused_panel == FocusedPanel::Organize {
                        handle_organize(state, organize, library, subtitle, config, qbit, rt);
                    }
                }
                KeyCode::Char('S') => {
                    if state.focused_panel == FocusedPanel::Organize {
                        // Download subtitle for selected pending item
                        if let Some(item) = state.pending_items.selected() {
                            let video = item.video_file.clone();
                            let title = std::path::Path::new(&video)
                                .file_stem()
                                .and_then(|n| n.to_str())
                                .unwrap_or(&video)
                                .to_string();
                            state.set_status(format!("Downloading subtitle: {}", truncate_name(&title, 40)));
                            let (ok, msg) = rt.block_on(subtitle.download_subtitle(&video, Some(&title)));
                            if ok {
                                state.set_status(format!("Subtitle downloaded: {}", truncate_name(&title, 40)));
                            } else {
                                state.set_status(format!("Subtitle failed: {}", msg));
                            }
                        }
                    }
                }
                KeyCode::Char('P') => {
                    if state.focused_panel == FocusedPanel::Downloads {
                        if let Some(t) = state.torrents.selected() {
                            let hash = t.hash.clone();
                            let name = t.name.clone();
                            if rt.block_on(qbit.pause_torrent(&hash)) {
                                state.set_status(format!("Paused: {}", truncate_name(&name, 40)));
                                load_downloads(state, qbit, rt);
                            } else {
                                state.set_status("Pause failed");
                            }
                        }
                    }
                }
                KeyCode::Char('u') => {
                    if state.focused_panel == FocusedPanel::Downloads {
                        if let Some(t) = state.torrents.selected() {
                            let hash = t.hash.clone();
                            let name = t.name.clone();
                            if rt.block_on(qbit.resume_torrent(&hash)) {
                                state.set_status(format!("Resumed: {}", truncate_name(&name, 40)));
                                load_downloads(state, qbit, rt);
                            } else {
                                state.set_status("Resume failed");
                            }
                        }
                    }
                }
                KeyCode::Char('a') => {
                    if state.focused_panel == FocusedPanel::Watchlist {
                        if let Some(item) = state.watchlist.selected() {
                            let query = item.title.clone();
                            let media_type = match item.media_type {
                                crate::models::MediaType::Show => "tv",
                                _ => "movie",
                            };
                            state.focused_panel = FocusedPanel::Search;
                            state.search_query = query.clone();
                            state.set_mode(AppMode::Search);
                            state.set_status(format!("Searching: {}", query));
                            let results = rt.block_on(search.search(&query, media_type, &state.search_indexers));
                            let count = results.len();
                            state.search_results.update(results);
                            state.set_status(format!("Found {} results for '{}'", count, query));
                            state.needs_clear = true;
                        }
                    }
                }
                KeyCode::Char('r') => {
                    refresh_current_panel(state, config, qbit, library, search, organize, trakt, rt);
                }
                KeyCode::Char('R') => {
                    refresh_all(state, config, qbit, library, organize, trakt, rt);
                }
                _ => {}
            }
        }
    }
    false
}

/// Handle Enter key - primary action for current panel
fn handle_enter_action<B: ratatui::backend::Backend + io::Write>(
    state: &mut AppState,
    config: &Arc<AppConfig>,
    qbit: &mut QBittorrentService,
    library: &LibraryService,
    _search: &mut TorrentSearchService,
    organize: &OrganizeService,
    subtitle: &SubtitleService,
    rt: &tokio::runtime::Runtime,
    _terminal: &mut Terminal<B>,
    theme: &mut Theme,
) {
    match state.focused_panel {
        FocusedPanel::Downloads => {
            if let Some(t) = state.torrents.selected() {
                let name = truncate_name(&t.name, 40);
                let progress = t.progress * 100.0;
                let st = t.state.clone();
                state.set_status(format!("{} - {:.1}% [{}]", name, progress, st));
            }
        }
        FocusedPanel::Search => {
            if !state.search_results.items.is_empty() {
                if let Some(result) = state.search_results.selected() {
                    let name = result.name.clone();
                    let magnet = result.magnet.clone();
                    state.set_status(format!("Adding: {}", truncate_name(&name, 50)));
                    let _ = rt.block_on(qbit.add_torrent(&magnet, ""));
                    load_downloads(state, qbit, rt);
                    state.set_status(format!("Download started: {}", truncate_name(&name, 50)));
                    state.needs_clear = true;
                }
            } else {
                state.set_mode(AppMode::Search);
            }
        }
        FocusedPanel::Library => {
            // Handled by handle_library_enter in the Library-specific key handler
        }
        FocusedPanel::Organize => {
            handle_organize(state, organize, library, subtitle, config, qbit, rt);
        }
        FocusedPanel::Watchlist => {
            if let Some(item) = state.watchlist.selected() {
                let query = item.title.clone();
                state.focused_panel = FocusedPanel::Search;
                state.search_query = query;
                state.set_mode(AppMode::Search);
                state.needs_clear = true;
            }
        }
        FocusedPanel::Settings => {
            if let Some(entry) = state.settings_entries.selected() {
                if entry.editable && entry.key == "Theme" {
                    // Cycle to next theme
                    let current_idx = THEME_NAMES.iter()
                        .position(|n| *n == state.theme_name)
                        .unwrap_or(0);
                    let next_idx = (current_idx + 1) % THEME_NAMES.len();
                    let new_theme_name = THEME_NAMES[next_idx];
                    state.theme_name = new_theme_name.to_string();
                    *theme = Theme::from_name(new_theme_name);
                    state.set_status(format!("Theme changed to: {}", new_theme_name));
                    build_settings_entries(state, config);
                    state.needs_clear = true;
                } else if entry.editable {
                    // Check if it's a search indexer toggle
                    let available_indexers = ["yts", "tpb", "1337x"];
                    let key = entry.key.clone();
                    if available_indexers.contains(&key.as_str()) {
                        toggle_search_indexer(state, config, &key);
                        state.set_status(format!("Toggled indexer: {}", key));
                        build_settings_entries(state, config);
                        state.needs_clear = true;
                    }
                }
            }
        }
    }
}

/// Toggle a search indexer on/off and save to disk
fn toggle_search_indexer(state: &mut AppState, config: &Arc<AppConfig>, indexer: &str) {
    if let Some(pos) = state.search_indexers.iter().position(|i| i == indexer) {
        // Don't allow removing the last indexer
        if state.search_indexers.len() > 1 {
            state.search_indexers.remove(pos);
        }
    } else {
        state.search_indexers.push(indexer.to_string());
    }

    // Save to disk by reading, modifying, writing config
    let config_path = config.config_path();
    if let Ok(content) = std::fs::read_to_string(config_path) {
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(settings) = json.get_mut("settings") {
                settings["search_indexers"] = serde_json::json!(state.search_indexers);
            }
            let _ = std::fs::write(config_path, serde_json::to_string_pretty(&json).unwrap_or_default());
        }
    }
}

/// Handle organize action for the selected pending item
fn handle_organize(
    state: &mut AppState,
    organize: &OrganizeService,
    library: &LibraryService,
    subtitle: &SubtitleService,
    config: &Arc<AppConfig>,
    qbit: &mut QBittorrentService,
    rt: &tokio::runtime::Runtime,
) {
    // Clone all needed data upfront to avoid borrow issues
    let selected = state.pending_items.selected().map(|item| {
        (
            item.video_file.clone(),
            item.pending_type.clone(),
            item.show_name.clone(),
            item.season,
        )
    });

    if let Some((video, ptype, show_name, season)) = selected {
        state.set_status(format!("Organizing: {}", truncate_name(&video, 40)));

        let (ok, msg, dest_path) = match ptype {
            PendingType::Movie => organize.organize_movie(&video),
            PendingType::Show => {
                let (ok, msg) = organize.organize_episode(&video);
                (ok, msg, None)
            }
            PendingType::Anime => {
                let (count, msgs) = organize.organize_anime(
                    vec![video.clone()],
                    show_name.as_deref().unwrap_or("Unknown"),
                    season.unwrap_or(1),
                );
                if count > 0 {
                    (true, format!("Organized {} anime file(s)", count), None)
                } else {
                    (false, msgs.join("; "), None)
                }
            }
        };

        if ok {
            let title = std::path::Path::new(&video)
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or(&video)
                .to_string();
            state.set_status(format!("Organized: {}", truncate_name(&title, 40)));

            // Download subtitle to the destination path (best effort)
            if ptype == PendingType::Movie {
                if let Some(ref dest) = dest_path {
                    state.set_status(format!("Downloading subtitle: {}", truncate_name(&title, 40)));
                    let _ = rt.block_on(subtitle.download_subtitle(dest, Some(&title)));
                }
            }

            // Refresh pending items and library data after successful organize
            load_pending_items(state, qbit, organize, config, rt);
            load_library_data(state, library);
            state.library_stats = library.get_stats();
            state.set_status(format!("Organized: {}", msg));
        } else {
            state.set_error(format!("Organize failed: {}", msg));
        }
        state.needs_clear = true;
    }
}

/// Play a video file using mpv (leaves TUI, runs mpv, returns to TUI)
fn play_video<B: ratatui::backend::Backend + io::Write>(
    state: &mut AppState,
    terminal: &mut Terminal<B>,
    video_path: &str,
    title: &str,
) {
    if !std::path::Path::new(video_path).exists() {
        state.set_status(format!("File not found: {}", truncate_name(video_path, 50)));
        return;
    }

    state.set_status(format!("Playing: {}", title));

    // Leave TUI for mpv
    let _ = crossterm::execute!(
        io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    );
    let _ = crossterm::terminal::disable_raw_mode();

    let result = std::process::Command::new("mpv")
        .arg(video_path)
        .status();

    // Re-enter TUI
    let _ = crossterm::terminal::enable_raw_mode();
    let _ = crossterm::execute!(
        io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::Hide
    );
    let _ = terminal.clear();
    state.needs_clear = true;

    match result {
        Ok(s) if s.success() => state.set_status(format!("Finished: {}", title)),
        Ok(s) => state.set_status(format!("mpv exited: {}", s)),
        Err(e) => state.set_status(format!("mpv failed: {}", e)),
    }
}

/// Handle Enter/Right in Library panel - drill down
fn handle_library_enter<B: ratatui::backend::Backend + io::Write>(
    state: &mut AppState,
    terminal: &mut Terminal<B>,
) {
    let selected_idx = match state.library_list.state.selected() {
        Some(i) => i,
        None => return,
    };

    match state.library_view.clone() {
        LibraryView::Categories => {
            match selected_idx {
                0 => state.library_view = LibraryView::Movies,
                1 => state.library_view = LibraryView::Shows,
                2 => state.library_view = LibraryView::Anime,
                _ => return,
            }
            refresh_library_list(state);
            state.needs_clear = true;
        }
        LibraryView::Movies => {
            // Movie is a leaf - play it
            if let Some(movie) = state.library_movies.get(selected_idx) {
                if let Some(ref vf) = movie.video_file {
                    let video_path = vf.clone();
                    let title = movie.title.clone();
                    play_video(state, terminal, &video_path, &title);
                } else {
                    state.set_status(format!("No video file for: {}", movie.title));
                }
            }
        }
        LibraryView::Shows => {
            state.library_view = LibraryView::ShowDetail(selected_idx);
            refresh_library_list(state);
            state.needs_clear = true;
        }
        LibraryView::Anime => {
            state.library_view = LibraryView::AnimeDetail(selected_idx);
            refresh_library_list(state);
            state.needs_clear = true;
        }
        LibraryView::ShowDetail(parent_idx) => {
            state.library_view = LibraryView::SeasonEpisodes {
                is_anime: false,
                parent_idx,
                season_idx: selected_idx,
            };
            refresh_library_list(state);
            state.needs_clear = true;
        }
        LibraryView::AnimeDetail(parent_idx) => {
            state.library_view = LibraryView::SeasonEpisodes {
                is_anime: true,
                parent_idx,
                season_idx: selected_idx,
            };
            refresh_library_list(state);
            state.needs_clear = true;
        }
        LibraryView::SeasonEpisodes { is_anime, parent_idx, season_idx } => {
            // Episode is a leaf - play it
            let episode_path = if is_anime {
                state.library_anime.get(parent_idx)
                    .and_then(|a| a.seasons.get(season_idx))
                    .and_then(|s| s.episodes.get(selected_idx))
                    .map(|e| e.path.clone())
            } else {
                state.library_shows.get(parent_idx)
                    .and_then(|s| s.seasons.get(season_idx))
                    .and_then(|s| s.episodes.get(selected_idx))
                    .map(|e| e.path.clone())
            };
            if let Some(path) = episode_path {
                let title = state.library_list.items.get(selected_idx)
                    .cloned()
                    .unwrap_or_else(|| "Episode".to_string());
                play_video(state, terminal, &path, &title);
            }
        }
    }
}

/// Handle Left/Esc in Library panel - go back one level
fn handle_library_back(state: &mut AppState) {
    match state.library_view.clone() {
        LibraryView::Categories => {
            // At top level, return to sidebar
            state.return_to_sidebar();
            return;
        }
        LibraryView::Movies | LibraryView::Shows | LibraryView::Anime => {
            state.library_view = LibraryView::Categories;
        }
        LibraryView::ShowDetail(_) => {
            state.library_view = LibraryView::Shows;
        }
        LibraryView::AnimeDetail(_) => {
            state.library_view = LibraryView::Anime;
        }
        LibraryView::SeasonEpisodes { is_anime, parent_idx, .. } => {
            if is_anime {
                state.library_view = LibraryView::AnimeDetail(parent_idx);
            } else {
                state.library_view = LibraryView::ShowDetail(parent_idx);
            }
        }
    }
    refresh_library_list(state);
    state.needs_clear = true;
}

/// Handle 'p' key in Library panel - play selected item
fn handle_library_play<B: ratatui::backend::Backend + io::Write>(
    state: &mut AppState,
    terminal: &mut Terminal<B>,
) {
    let selected_idx = match state.library_list.state.selected() {
        Some(i) => i,
        None => { state.set_status("No item selected"); return; }
    };

    match &state.library_view {
        LibraryView::Movies => {
            if let Some(movie) = state.library_movies.get(selected_idx) {
                if let Some(ref vf) = movie.video_file {
                    let video_path = vf.clone();
                    let title = movie.title.clone();
                    play_video(state, terminal, &video_path, &title);
                } else {
                    state.set_status(format!("No video file for: {}", movie.title));
                }
            }
        }
        LibraryView::SeasonEpisodes { is_anime, parent_idx, season_idx } => {
            let episode_path = if *is_anime {
                state.library_anime.get(*parent_idx)
                    .and_then(|a| a.seasons.get(*season_idx))
                    .and_then(|s| s.episodes.get(selected_idx))
                    .map(|e| e.path.clone())
            } else {
                state.library_shows.get(*parent_idx)
                    .and_then(|s| s.seasons.get(*season_idx))
                    .and_then(|s| s.episodes.get(selected_idx))
                    .map(|e| e.path.clone())
            };
            if let Some(path) = episode_path {
                let title = state.library_list.items.get(selected_idx)
                    .cloned()
                    .unwrap_or_else(|| "Episode".to_string());
                play_video(state, terminal, &path, &title);
            } else {
                state.set_status("No video file for this episode");
            }
        }
        _ => {
            state.set_status("Select a movie or episode to play");
        }
    }
}

/// Handle 'S' key in Library panel - download subtitle
fn handle_library_subtitle(
    state: &mut AppState,
    subtitle: &SubtitleService,
    rt: &tokio::runtime::Runtime,
) {
    let selected_idx = match state.library_list.state.selected() {
        Some(i) => i,
        None => { state.set_status("No item selected"); return; }
    };

    match &state.library_view {
        LibraryView::Movies => {
            if let Some(movie) = state.library_movies.get(selected_idx) {
                if let Some(ref vf) = movie.video_file {
                    let video_path = vf.clone();
                    let title = movie.title.clone();
                    state.set_status(format!("Downloading subtitle: {}", truncate_name(&title, 40)));
                    let (ok, msg) = rt.block_on(subtitle.download_subtitle(&video_path, Some(&title)));
                    if ok {
                        state.set_status(format!("Subtitle downloaded: {}", truncate_name(&title, 40)));
                    } else {
                        state.set_status(format!("Subtitle failed: {}", msg));
                    }
                } else {
                    state.set_status(format!("No video file for: {}", movie.title));
                }
            }
        }
        LibraryView::SeasonEpisodes { is_anime, parent_idx, season_idx } => {
            let episode = if *is_anime {
                state.library_anime.get(*parent_idx)
                    .and_then(|a| a.seasons.get(*season_idx))
                    .and_then(|s| s.episodes.get(selected_idx))
                    .cloned()
            } else {
                state.library_shows.get(*parent_idx)
                    .and_then(|s| s.seasons.get(*season_idx))
                    .and_then(|s| s.episodes.get(selected_idx))
                    .cloned()
            };
            if let Some(ep) = episode {
                let title = ep.title.as_deref().unwrap_or("episode").to_string();
                state.set_status(format!("Downloading subtitle: {}", truncate_name(&title, 40)));
                let (ok, msg) = rt.block_on(subtitle.download_subtitle(&ep.path, Some(&title)));
                if ok {
                    state.set_status(format!("Subtitle downloaded: {}", truncate_name(&title, 40)));
                } else {
                    state.set_status(format!("Subtitle failed: {}", msg));
                }
            }
        }
        _ => {
            state.set_status("Select a movie or episode for subtitle download");
        }
    }
}

/// Handle 'i' key in Library panel - show file/folder location
fn handle_library_info(state: &mut AppState) {
    let selected_idx = match state.library_list.state.selected() {
        Some(i) => i,
        None => { state.set_status("No item selected"); return; }
    };

    match &state.library_view {
        LibraryView::Movies => {
            if let Some(movie) = state.library_movies.get(selected_idx) {
                state.set_status(format!("Location: {}", movie.path));
            }
        }
        LibraryView::Shows => {
            if let Some(show) = state.library_shows.get(selected_idx) {
                state.set_status(format!("Location: {}", show.path));
            }
        }
        LibraryView::Anime => {
            if let Some(anime) = state.library_anime.get(selected_idx) {
                state.set_status(format!("Location: {}", anime.path));
            }
        }
        LibraryView::SeasonEpisodes { is_anime, parent_idx, season_idx } => {
            let episode = if *is_anime {
                state.library_anime.get(*parent_idx)
                    .and_then(|a| a.seasons.get(*season_idx))
                    .and_then(|s| s.episodes.get(selected_idx))
            } else {
                state.library_shows.get(*parent_idx)
                    .and_then(|s| s.seasons.get(*season_idx))
                    .and_then(|s| s.episodes.get(selected_idx))
            };
            if let Some(ep) = episode {
                state.set_status(format!("Location: {}", ep.path));
            }
        }
        _ => {
            state.set_status("Select an item to see its location");
        }
    }
}

/// Refresh current panel
fn refresh_current_panel(
    state: &mut AppState,
    config: &Arc<AppConfig>,
    qbit: &mut QBittorrentService,
    library: &LibraryService,
    search: &mut TorrentSearchService,
    organize: &OrganizeService,
    trakt: &TraktService,
    rt: &tokio::runtime::Runtime,
) {
    match state.focused_panel {
        FocusedPanel::Downloads => {
            load_downloads(state, qbit, rt);
            state.set_status("Downloads refreshed");
        }
        FocusedPanel::Library => {
            load_library_data(state, library);
            state.set_status("Library refreshed");
        }
        FocusedPanel::Organize => {
            load_pending_items(state, qbit, organize, config, rt);
            state.set_status("Organize refreshed");
        }
        FocusedPanel::Search => {
            if !state.search_query.is_empty() {
                let query = state.search_query.clone();
                let results = rt.block_on(search.search(&query, "movie", &state.search_indexers));
                let count = results.len();
                state.search_results.update(results);
                state.set_status(format!("Found {} results", count));
            }
        }
        FocusedPanel::Settings => {
            state.library_stats = library.get_stats();
            build_settings_entries(state, config);
            state.set_status("Settings refreshed");
        }
        FocusedPanel::Watchlist => {
            let watchlist = rt.block_on(trakt.get_watchlist());
            state.watchlist.update(watchlist);
            state.set_status("Watchlist refreshed");
        }
    }
    state.needs_clear = true;
}

/// Refresh all panels
fn refresh_all(
    state: &mut AppState,
    config: &Arc<AppConfig>,
    qbit: &mut QBittorrentService,
    library: &LibraryService,
    organize: &OrganizeService,
    trakt: &TraktService,
    rt: &tokio::runtime::Runtime,
) {
    state.set_status("Refreshing all...");
    load_downloads(state, qbit, rt);
    load_library_data(state, library);
    load_pending_items(state, qbit, organize, config, rt);
    let watchlist = rt.block_on(trakt.get_watchlist());
    state.watchlist.update(watchlist);
    state.library_stats = library.get_stats();
    build_settings_entries(state, config);
    state.set_status("All panels refreshed");
    state.needs_clear = true;
}

/// Truncate a name for display
fn truncate_name(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len { s.to_string() }
    else if max_len > 3 {
        let t: String = s.chars().take(max_len - 3).collect();
        format!("{}...", t)
    } else {
        s.chars().take(max_len).collect()
    }
}
