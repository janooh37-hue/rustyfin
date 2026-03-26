//! Main rendering logic

use ratatui::prelude::Stylize;
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table},
    Frame,
};

use crate::models::*;
use crate::ui::state::*;
use crate::ui::theme::Theme;

/// Main render function
pub fn render(f: &mut Frame, state: &mut AppState, theme: &Theme) {
    // Clear the entire screen with background color
    let bg = Block::default().style(Style::default().bg(theme.background));
    f.render_widget(bg, f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(0),   // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());

    render_header(f, chunks[0], state, theme);
    render_main(f, chunks[1], state, theme);
    render_status_bar(f, chunks[2], state, theme);

    // Overlays
    if state.mode == AppMode::Help {
        render_help_overlay(f, f.area(), theme);
    }
    if state.mode == AppMode::Confirm {
        render_confirm_dialog(f, f.area(), state, theme);
    }
}

/// Render the header bar
fn render_header(f: &mut Frame, area: Rect, _state: &mut AppState, theme: &Theme) {
    let title = " RustyFin ";
    let help = " [?] Help ";
    let pad = area.width.saturating_sub(title.len() as u16 + help.len() as u16);

    let line = Line::from(vec![
        Span::styled(title, Style::default().fg(theme.primary).bold()),
        Span::raw(" ".repeat(pad as usize)),
        Span::styled(help, Style::default().fg(theme.secondary)),
    ]);

    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme.surface)),
        area,
    );
}

/// Render main content area with sidebar and content panel
fn render_main(f: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22), // Sidebar
            Constraint::Min(0),    // Content
        ])
        .split(area);

    render_sidebar(f, chunks[0], state, theme);
    render_content(f, chunks[1], state, theme);
}

/// Render sidebar with panel navigation
fn render_sidebar(f: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let sidebar_active = state.focus == Focus::Sidebar;
    let border_color = if sidebar_active { theme.primary } else { theme.border };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(" Panels ")
        .title_style(Style::default().fg(theme.primary))
        .style(Style::default().bg(theme.background));
    f.render_widget(block, area);

    let inner = area.inner(Margin::new(1, 1));
    let panels = FocusedPanel::all();
    let items: Vec<ListItem> = panels
        .iter()
        .map(|p| {
            let is_current = *p == state.focused_panel;
            let style = if is_current && sidebar_active {
                Style::default().fg(theme.background).bg(theme.primary).bold()
            } else if is_current {
                Style::default().fg(theme.primary).bg(theme.surface).bold()
            } else {
                Style::default().fg(theme.foreground)
            };
            ListItem::new(Line::from(Span::styled(
                format!(" {} {} ", p.icon(), p.label()),
                style,
            )))
        })
        .collect();

    f.render_widget(List::new(items), inner);
}

/// Render content area based on focused panel
fn render_content(f: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    // Fill content area with background to prevent stale artifacts
    f.render_widget(
        Block::default().style(Style::default().bg(theme.background)),
        area,
    );
    match state.focused_panel {
        FocusedPanel::Downloads => render_torrents(f, area, state, theme),
        FocusedPanel::Organize => render_pending(f, area, state, theme),
        FocusedPanel::Watchlist => render_watchlist(f, area, state, theme),
        FocusedPanel::Library => render_library(f, area, state, theme),
        FocusedPanel::Search => render_search_panel(f, area, state, theme),
        FocusedPanel::Settings => render_settings(f, area, state, theme),
    }
}

/// Create content block with border highlight when content is focused
fn content_block<'a>(title: &'a str, state: &'a AppState, theme: &'a Theme) -> Block<'a> {
    let content_active = state.focus == Focus::Content;
    let border_color = if content_active { theme.primary } else { theme.border };

    Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(format!(" {} ", title))
        .title_style(Style::default().fg(theme.primary).bold())
        .style(Style::default().bg(theme.background))
}

/// Render torrents table
fn render_torrents(f: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let block = content_block("Downloads", state, theme);
    f.render_widget(block, area);
    let inner = area.inner(Margin::new(1, 1));

    if state.torrents.items.is_empty() {
        render_empty_message(f, inner, "No downloads", theme);
        return;
    }

    let header = Row::new(vec![
        Cell::from(Span::styled("Name", Style::default().fg(theme.primary).bold())),
        Cell::from(Span::styled("Progress", Style::default().fg(theme.primary).bold())),
        Cell::from(Span::styled("Speed", Style::default().fg(theme.primary).bold())),
        Cell::from(Span::styled("ETA", Style::default().fg(theme.primary).bold())),
        Cell::from(Span::styled("Seeds", Style::default().fg(theme.primary).bold())),
    ]).height(1);

    let name_w = inner.width.saturating_sub(38) as usize;
    let rows: Vec<Row> = state.torrents.items.iter().map(|t| {
        let progress = format!("{:.1}%", t.progress * 100.0);
        let pcol = if t.progress >= 1.0 { theme.success }
                   else if t.progress >= 0.5 { theme.primary }
                   else { theme.warning };
        Row::new(vec![
            Cell::from(Span::styled(truncate(&t.name, name_w), Style::default().fg(theme.foreground))),
            Cell::from(Span::styled(progress, Style::default().fg(pcol))),
            Cell::from(Span::styled(format_speed(t.download_speed), Style::default().fg(theme.success))),
            Cell::from(Span::styled(format_eta(t.eta), Style::default().fg(theme.warning))),
            Cell::from(Span::styled(t.seeds.to_string(), Style::default().fg(theme.secondary))),
        ])
    }).collect();

    let table = Table::new(rows, [
        Constraint::Min(20), Constraint::Length(10), Constraint::Length(12),
        Constraint::Length(8), Constraint::Length(6),
    ])
    .header(header)
    .row_highlight_style(Style::default().bg(theme.surface).fg(theme.primary))
    .highlight_symbol("▶ ");

    f.render_stateful_widget(table, inner, &mut state.torrents.state);
}

/// Render pending items (organize panel)
fn render_pending(f: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let block = content_block("Organize", state, theme);
    f.render_widget(block, area);
    let inner = area.inner(Margin::new(1, 1));

    if state.pending_items.items.is_empty() {
        render_empty_message(f, inner, "No pending items to organize", theme);
        return;
    }

    let header = Row::new(vec![
        Cell::from(Span::styled("Type", Style::default().fg(theme.primary).bold())),
        Cell::from(Span::styled("Name", Style::default().fg(theme.primary).bold())),
    ]).height(1);

    let rows: Vec<Row> = state.pending_items.items.iter().map(|p| {
        let type_str = match p.pending_type {
            PendingType::Movie => "Movie",
            PendingType::Show => "Show",
            PendingType::Anime => "Anime",
        };
        // Show just the filename, not the full path
        let display_name = std::path::Path::new(&p.video_file)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&p.video_file);
        Row::new(vec![
            Cell::from(Span::styled(type_str, Style::default().fg(theme.accent))),
            Cell::from(Span::styled(truncate(display_name, 60), Style::default().fg(theme.foreground))),
        ])
    }).collect();

    let table = Table::new(rows, [Constraint::Length(8), Constraint::Min(20)])
        .header(header)
        .row_highlight_style(Style::default().bg(theme.surface).fg(theme.primary))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(table, inner, &mut state.pending_items.state);
}

/// Render watchlist
fn render_watchlist(f: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let block = content_block("Watchlist", state, theme);
    f.render_widget(block, area);
    let inner = area.inner(Margin::new(1, 1));

    if state.watchlist.items.is_empty() {
        render_empty_message(f, inner, "No watchlist items (check Trakt config)", theme);
        return;
    }

    let header = Row::new(vec![
        Cell::from(Span::styled("Title", Style::default().fg(theme.primary).bold())),
        Cell::from(Span::styled("Year", Style::default().fg(theme.primary).bold())),
        Cell::from(Span::styled("Type", Style::default().fg(theme.primary).bold())),
    ]).height(1);

    let rows: Vec<Row> = state.watchlist.items.iter().map(|w| {
        let type_str = match w.media_type {
            MediaType::Movie => "Movie",
            MediaType::Show => "Show",
            MediaType::Anime => "Anime",
        };
        let year = w.year.map(|y| y.to_string()).unwrap_or_else(|| "N/A".into());
        Row::new(vec![
            Cell::from(Span::styled(truncate(&w.title, 40), Style::default().fg(theme.foreground))),
            Cell::from(Span::styled(year, Style::default().fg(theme.secondary))),
            Cell::from(Span::styled(type_str, Style::default().fg(theme.accent))),
        ])
    }).collect();

    let table = Table::new(rows, [Constraint::Min(20), Constraint::Length(6), Constraint::Length(8)])
        .header(header)
        .row_highlight_style(Style::default().bg(theme.surface).fg(theme.primary))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(table, inner, &mut state.watchlist.state);
}

/// Render library panel with drill-down views
fn render_library(f: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    // Build title with breadcrumb based on current view
    let title = match &state.library_view {
        LibraryView::Categories => "Library".to_string(),
        LibraryView::Movies => "Library > Movies".to_string(),
        LibraryView::Shows => "Library > Shows".to_string(),
        LibraryView::Anime => "Library > Anime".to_string(),
        LibraryView::ShowDetail(idx) => {
            let name = state.library_shows.get(*idx)
                .map(|s| s.title.as_str())
                .unwrap_or("Show");
            format!("Library > Shows > {}", name)
        }
        LibraryView::AnimeDetail(idx) => {
            let name = state.library_anime.get(*idx)
                .map(|a| a.title.as_str())
                .unwrap_or("Anime");
            format!("Library > Anime > {}", name)
        }
        LibraryView::SeasonEpisodes { is_anime, parent_idx, season_idx } => {
            let (parent_name, season_num) = if *is_anime {
                let name = state.library_anime.get(*parent_idx)
                    .map(|a| a.title.as_str())
                    .unwrap_or("Anime");
                let snum = state.library_anime.get(*parent_idx)
                    .and_then(|a| a.seasons.get(*season_idx))
                    .map(|s| s.number)
                    .unwrap_or(0);
                (name, snum)
            } else {
                let name = state.library_shows.get(*parent_idx)
                    .map(|s| s.title.as_str())
                    .unwrap_or("Show");
                let snum = state.library_shows.get(*parent_idx)
                    .and_then(|s| s.seasons.get(*season_idx))
                    .map(|s| s.number)
                    .unwrap_or(0);
                (name, snum)
            };
            format!("Library > {} > Season {}", parent_name, season_num)
        }
    };

    let block = content_block(&title, state, theme);
    f.render_widget(block, area);
    let inner = area.inner(Margin::new(1, 1));

    if state.library_list.items.is_empty() {
        render_empty_message(f, inner, "No items", theme);
        return;
    }

    // Render as a simple list for all views
    let items: Vec<ListItem> = state.library_list.items.iter().enumerate().map(|(_i, item)| {
        ListItem::new(Line::from(Span::styled(
            truncate(item, inner.width.saturating_sub(4) as usize),
            Style::default().fg(theme.foreground),
        )))
    }).collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(theme.surface).fg(theme.primary))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, inner, &mut state.library_list.state);
}

/// Render search panel - integrated input + results
fn render_search_panel(f: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let block = content_block("Search", state, theme);
    f.render_widget(block, area);
    let inner = area.inner(Margin::new(1, 1));

    if inner.height < 3 {
        return;
    }

    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Search input
            Constraint::Length(1), // Separator
            Constraint::Min(0),   // Results
        ])
        .split(inner);

    // Search input line
    let cursor = if state.mode == AppMode::Search { "█" } else { "" };
    let icon_style = if state.mode == AppMode::Search {
        Style::default().fg(theme.primary)
    } else {
        Style::default().fg(theme.secondary)
    };
    let hint = if state.search_query.is_empty() && state.mode != AppMode::Search {
        Span::styled("Press / to search", Style::default().fg(theme.secondary).italic())
    } else {
        Span::raw("")
    };

    let input_line = Line::from(vec![
        Span::styled("⌕ ", icon_style),
        Span::styled(&state.search_query, Style::default().fg(theme.foreground)),
        Span::styled(cursor, Style::default().fg(theme.primary)),
        hint,
    ]);
    f.render_widget(Paragraph::new(input_line), parts[0]);

    // Separator - fill entire width with background to prevent stray chars
    let sep_str: String = "─".repeat(parts[1].width as usize);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(sep_str, Style::default().fg(theme.border))))
            .style(Style::default().bg(theme.background)),
        parts[1],
    );

    // Results
    if state.search_results.items.is_empty() {
        if state.is_searching {
            render_empty_message(f, parts[2], "Searching...", theme);
        } else if !state.search_query.is_empty() {
            render_empty_message(f, parts[2], "No results. Press Enter to search.", theme);
        }
        return;
    }

    let header = Row::new(vec![
        Cell::from(Span::styled("Title", Style::default().fg(theme.primary).bold())),
        Cell::from(Span::styled("Quality", Style::default().fg(theme.primary).bold())),
        Cell::from(Span::styled("Seeds", Style::default().fg(theme.primary).bold())),
        Cell::from(Span::styled("Size", Style::default().fg(theme.primary).bold())),
        Cell::from(Span::styled("Source", Style::default().fg(theme.primary).bold())),
    ]).height(1);

    let rows: Vec<Row> = state.search_results.items.iter().map(|r| {
        let seeds_color = if r.seeds > 50 { theme.success }
                          else if r.seeds > 10 { theme.warning }
                          else { theme.error };
        Row::new(vec![
            Cell::from(Span::styled(truncate(&r.name, 40), Style::default().fg(theme.foreground))),
            Cell::from(Span::styled(&r.quality, Style::default().fg(theme.primary))),
            Cell::from(Span::styled(r.seeds.to_string(), Style::default().fg(seeds_color))),
            Cell::from(Span::styled(&r.size, Style::default().fg(theme.secondary))),
            Cell::from(Span::styled(&r.source, Style::default().fg(theme.accent))),
        ])
    }).collect();

    let table = Table::new(rows, [
        Constraint::Min(20), Constraint::Length(10), Constraint::Length(8),
        Constraint::Length(10), Constraint::Length(8),
    ])
    .header(header)
    .row_highlight_style(Style::default().bg(theme.surface).fg(theme.primary))
    .highlight_symbol("▶ ");

    f.render_stateful_widget(table, parts[2], &mut state.search_results.state);
}

/// Render settings panel with real configuration data
fn render_settings(f: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let block = content_block("Settings", state, theme);
    f.render_widget(block, area);
    let inner = area.inner(Margin::new(1, 1));

    if state.settings_entries.items.is_empty() {
        render_empty_message(f, inner, "Loading settings...", theme);
        return;
    }

    let header = Row::new(vec![
        Cell::from(Span::styled("Setting", Style::default().fg(theme.primary).bold())),
        Cell::from(Span::styled("Value", Style::default().fg(theme.primary).bold())),
    ]).height(1);

    let is_editing = state.mode == AppMode::EditSetting;
    let selected_idx = state.settings_entries.state.selected();

    let rows: Vec<Row> = state.settings_entries.items.iter().enumerate().map(|(i, e)| {
        let key_style = if e.editable {
            Style::default().fg(theme.accent).bold()
        } else {
            Style::default().fg(theme.foreground)
        };
        let val_style = if e.editable {
            Style::default().fg(theme.primary)
        } else {
            Style::default().fg(theme.secondary)
        };
        // Section headers (empty value, non-editable) get special styling
        if e.value.is_empty() && !e.editable {
            Row::new(vec![
                Cell::from(Span::styled(&e.key, Style::default().fg(theme.primary).bold())),
                Cell::from(Span::raw("")),
            ])
        } else if is_editing && selected_idx == Some(i) && e.key == state.editing_setting_key {
            // Show inline editing with cursor
            // Mask password fields
            let is_password = e.key == "qBit Pass";
            let cursor_pos = state.editing_setting_cursor;
            let val = &state.editing_setting_value;
            let display_chars: Vec<char> = if is_password {
                vec!['*'; val.chars().count()]
            } else {
                val.chars().collect()
            };
            let before: String = display_chars[..cursor_pos.min(display_chars.len())].iter().collect();
            let cursor_char = if cursor_pos < display_chars.len() {
                display_chars[cursor_pos].to_string()
            } else {
                " ".to_string()
            };
            let after: String = if cursor_pos < display_chars.len() {
                display_chars[cursor_pos + 1..].iter().collect()
            } else {
                String::new()
            };
            Row::new(vec![
                Cell::from(Span::styled(&e.key, key_style)),
                Cell::from(Line::from(vec![
                    Span::styled(before, Style::default().fg(theme.foreground)),
                    Span::styled(cursor_char, Style::default().fg(theme.background).bg(theme.primary)),
                    Span::styled(after, Style::default().fg(theme.foreground)),
                ])),
            ])
        } else {
            Row::new(vec![
                Cell::from(Span::styled(&e.key, key_style)),
                Cell::from(Span::styled(truncate(&e.value, 50), val_style)),
            ])
        }
    }).collect();

    let table = Table::new(rows, [Constraint::Length(22), Constraint::Min(20)])
        .header(header)
        .row_highlight_style(Style::default().bg(theme.surface).fg(theme.primary))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(table, inner, &mut state.settings_entries.state);
}

/// Render status bar
fn render_status_bar(f: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let panel = state.focused_panel.label();
    let focus_str = match state.focus {
        Focus::Sidebar => "sidebar",
        Focus::Content => "content",
    };
    let mode_str = match state.mode {
        AppMode::Search => " SEARCH ",
        AppMode::Help => " HELP ",
        AppMode::EditSetting => " EDIT ",
        _ => "",
    };
    let hints = if state.mode == AppMode::EditSetting {
        "Enter:save Esc:cancel"
    } else {
        match (state.focus, state.focused_panel) {
            (Focus::Sidebar, _) => "↑↓:panels →:enter",
            (Focus::Content, FocusedPanel::Downloads) => "d:del P:pause u:resume ←:back",
            (Focus::Content, FocusedPanel::Search) => "/:search Enter:download ←:back",
            (Focus::Content, FocusedPanel::Library) => "→:enter p:play S:subs i:info ←:back",
            (Focus::Content, FocusedPanel::Organize) => "o:organize S:subs Enter:organize ←:back",
            (Focus::Content, FocusedPanel::Watchlist) => "a:search Enter:search ←:back",
            (Focus::Content, FocusedPanel::Settings) => "Enter:edit ←:back",
        }
    };

    let dl = format_speed(state.transfer_info.download_speed);
    let ul = format_speed(state.transfer_info.upload_speed);
    let w = area.width as usize;

    let status = if let Some(err) = &state.error_message {
        format!("ERR: {}", err)
    } else {
        state.status_message.clone()
    };
    let left = format!(
        " {} [{}]{} {} ",
        panel,
        focus_str,
        if mode_str.is_empty() { String::new() } else { format!(" [{}]", mode_str.trim()) },
        status,
    );
    let right = format!("{} ↓{} ↑{} ", hints, dl, ul);
    let pad = w.saturating_sub(left.chars().count() + right.chars().count());
    let full = format!("{}{}{}", left, " ".repeat(pad), right);
    let display: String = full.chars().take(w).collect();

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(display, Style::default().fg(theme.foreground))))
            .style(Style::default().bg(theme.surface)),
        area,
    );
}

/// Render empty message centered in area
fn render_empty_message(f: &mut Frame, area: Rect, message: &str, theme: &Theme) {
    if area.height == 0 { return; }
    let y_off = area.height / 2;
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            message,
            Style::default().fg(theme.secondary).italic(),
        )))
        .alignment(ratatui::layout::Alignment::Center),
        Rect::new(area.x, area.y + y_off, area.width, 1),
    );
}

// ── Helpers ──

fn format_speed(bytes: u64) -> String {
    if bytes == 0 { return "0 B/s".into(); }
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB { format!("{:.1}GB/s", bytes as f64 / GB as f64) }
    else if bytes >= MB { format!("{:.1}MB/s", bytes as f64 / MB as f64) }
    else if bytes >= KB { format!("{:.1}KB/s", bytes as f64 / KB as f64) }
    else { format!("{}B/s", bytes) }
}

fn format_eta(seconds: i64) -> String {
    if seconds <= 0 || seconds > 86400 * 365 { return "∞".into(); }
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    if h > 0 { format!("{}h{}m", h, m) }
    else if m > 0 { format!("{}m", m) }
    else { format!("{}s", seconds) }
}

fn truncate(s: &str, max_len: usize) -> String {
    if max_len < 4 { return s.chars().take(max_len).collect(); }
    if s.chars().count() <= max_len { s.to_string() }
    else {
        let t: String = s.chars().take(max_len - 3).collect();
        format!("{}...", t)
    }
}

/// Render help overlay modal
fn render_help_overlay(f: &mut Frame, area: Rect, theme: &Theme) {
    let w = 62u16.min(area.width.saturating_sub(4));
    let h = 30u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(w)) / 2;
    let y = (area.height.saturating_sub(h)) / 2;
    let modal = Rect::new(x, y, w, h);

    f.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.primary))
        .title(" Help ")
        .title_style(Style::default().fg(theme.primary).bold())
        .style(Style::default().bg(theme.surface));
    f.render_widget(block, modal);

    let inner = modal.inner(Margin::new(2, 1));
    let help = vec![
        Line::from(Span::styled("Navigation", Style::default().fg(theme.primary).bold())),
        Line::from("  ↑/↓ or j/k       Navigate panels (sidebar)"),
        Line::from("                    Navigate items  (content)"),
        Line::from("  → or l or Enter   Enter panel / perform action"),
        Line::from("  ← or h or Esc     Return to sidebar"),
        Line::from("  g / G             First / last item"),
        Line::from(""),
        Line::from(Span::styled("Search", Style::default().fg(theme.primary).bold())),
        Line::from("  /                 Enter search mode"),
        Line::from("  type query        Type search terms"),
        Line::from("  Enter             Search / download result"),
        Line::from("  ↑/↓               Navigate results"),
        Line::from("  Esc               Exit search mode"),
        Line::from(""),
        Line::from(Span::styled("Downloads", Style::default().fg(theme.primary).bold())),
        Line::from("  d                 Delete torrent"),
        Line::from("  P                 Pause torrent"),
        Line::from("  u                 Resume torrent"),
        Line::from(""),
        Line::from(Span::styled("Library", Style::default().fg(theme.primary).bold())),
        Line::from("  →/Enter           Drill into category/show"),
        Line::from("  ←/Esc             Go back one level"),
        Line::from("  p / Enter         Play with mpv (movie/episode)"),
        Line::from("  S                 Download subtitle"),
        Line::from("  i                 Show file location"),
        Line::from(""),
        Line::from(Span::styled("Organize", Style::default().fg(theme.primary).bold())),
        Line::from("  o / Enter         Organize selected"),
        Line::from("  S                 Download subtitle"),
        Line::from(""),
        Line::from(Span::styled("General", Style::default().fg(theme.primary).bold())),
        Line::from("  r / R             Refresh current / all"),
        Line::from("  ?                 Toggle this help"),
        Line::from("  q                 Quit"),
    ];

    f.render_widget(
        Paragraph::new(help).style(Style::default().fg(theme.foreground)),
        inner,
    );
}

/// Render confirmation dialog
fn render_confirm_dialog(f: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let w = 50u16.min(area.width.saturating_sub(4));
    let h = 5u16;
    let x = (area.width.saturating_sub(w)) / 2;
    let y = (area.height.saturating_sub(h)) / 2;
    let modal = Rect::new(x, y, w, h);

    f.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.warning))
        .title(" Confirm ")
        .title_style(Style::default().fg(theme.warning).bold())
        .style(Style::default().bg(theme.surface));
    f.render_widget(block, modal);

    let inner = modal.inner(Margin::new(2, 1));
    let lines = vec![
        Line::from(Span::styled(&state.confirm_message, Style::default().fg(theme.foreground))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [y]es  ", Style::default().fg(theme.success).bold()),
            Span::styled("  [n]o  ", Style::default().fg(theme.error).bold()),
        ]),
    ];
    f.render_widget(
        Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center),
        inner,
    );
}
