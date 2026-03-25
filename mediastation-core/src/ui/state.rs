//! Application state and mode management

use ratatui::widgets::ListState;

use crate::models::*;

/// Application mode - determines how keybindings and rendering work
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Normal mode - browse and select items
    Normal,
    /// Search mode - search torrents
    Search,
    /// Help mode - show help overlay
    Help,
    /// Confirm mode - confirmation dialog
    Confirm,
    /// Running mode - executing an action
    Running,
    /// Editing a setting value inline
    EditSetting,
}

/// Two-level focus: sidebar (panel selection) vs content (item navigation)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// Navigating the sidebar panel list
    Sidebar,
    /// Navigating items within the focused panel
    Content,
}

/// Library browser view levels
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryView {
    Categories,
    Movies,
    Shows,
    Anime,
    ShowDetail(usize),     // index into library_shows
    AnimeDetail(usize),    // index into library_anime
    SeasonEpisodes {
        is_anime: bool,
        parent_idx: usize,
        season_idx: usize,
    },
}

/// Focused panel / view in the sidebar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    Downloads,
    Organize,
    Watchlist,
    Library,
    Search,
    Settings,
}

impl FocusedPanel {
    pub fn all() -> [Self; 6] {
        [
            Self::Downloads,
            Self::Organize,
            Self::Watchlist,
            Self::Library,
            Self::Search,
            Self::Settings,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Downloads => "Downloads",
            Self::Organize => "Organize",
            Self::Watchlist => "Watchlist",
            Self::Library => "Library",
            Self::Search => "Search",
            Self::Settings => "Settings",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Downloads => "↓",
            Self::Organize => "⚙",
            Self::Watchlist => "★",
            Self::Library => "◫",
            Self::Search => "⌕",
            Self::Settings => "≡",
        }
    }
}

use ratatui::widgets::TableState;

/// Settings entry for the settings panel
#[derive(Debug, Clone)]
pub struct SettingsEntry {
    pub key: String,
    pub value: String,
    /// If true, user can interact with Enter to change
    pub editable: bool,
}

/// Stateful list wrapper for navigation (for lists)
pub struct StatefulList<T> {
    pub items: Vec<T>,
    pub state: ListState,
}

impl<T> StatefulList<T> {
    pub fn new(items: Vec<T>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self { items, state }
    }

    pub fn select_first(&mut self) {
        if !self.items.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn select_last(&mut self) {
        if !self.items.is_empty() {
            self.state.select(Some(self.items.len() - 1));
        }
    }

    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        if let Some(selected) = self.state.selected() {
            let next = (selected + 1) % self.items.len();
            self.state.select(Some(next));
        } else {
            self.state.select(Some(0));
        }
    }

    pub fn select_previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        if let Some(selected) = self.state.selected() {
            let prev = if selected == 0 {
                self.items.len() - 1
            } else {
                selected - 1
            };
            self.state.select(Some(prev));
        } else {
            self.state.select(Some(0));
        }
    }

    pub fn selected(&self) -> Option<&T> {
        self.state.selected().and_then(|i| self.items.get(i))
    }

    pub fn update(&mut self, items: Vec<T>) {
        let selected = self.state.selected();
        self.items = items;
        if let Some(i) = selected {
            if i >= self.items.len() {
                self.state.select(Some(self.items.len().saturating_sub(1)));
            }
        } else if !self.items.is_empty() {
            self.state.select(Some(0));
        }
    }
}

/// Stateful table wrapper for tables
pub struct StatefulTable<T> {
    pub items: Vec<T>,
    pub state: TableState,
}

impl<T> StatefulTable<T> {
    pub fn new(items: Vec<T>) -> Self {
        let mut state = TableState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self { items, state }
    }

    pub fn select_first(&mut self) {
        if !self.items.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn select_last(&mut self) {
        if !self.items.is_empty() {
            self.state.select(Some(self.items.len() - 1));
        }
    }

    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        if let Some(selected) = self.state.selected() {
            let next = (selected + 1) % self.items.len();
            self.state.select(Some(next));
        } else {
            self.state.select(Some(0));
        }
    }

    pub fn select_previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        if let Some(selected) = self.state.selected() {
            let prev = if selected == 0 {
                self.items.len() - 1
            } else {
                selected - 1
            };
            self.state.select(Some(prev));
        } else {
            self.state.select(Some(0));
        }
    }

    pub fn selected(&self) -> Option<&T> {
        self.state.selected().and_then(|i| self.items.get(i))
    }

    pub fn update(&mut self, items: Vec<T>) {
        let selected = self.state.selected();
        self.items = items;
        if let Some(i) = selected {
            if i >= self.items.len() {
                self.state.select(Some(self.items.len().saturating_sub(1)));
            }
        } else if !self.items.is_empty() {
            self.state.select(Some(0));
        }
    }
}

/// Main application state
pub struct AppState {
    /// Current application mode
    pub mode: AppMode,
    /// Two-level focus: sidebar vs content
    pub focus: Focus,
    /// Currently focused sidebar panel
    pub focused_panel: FocusedPanel,
    /// Theme name
    pub theme_name: String,

    // Panel data
    pub torrents: StatefulTable<Torrent>,
    pub search_results: StatefulTable<SearchResult>,
    pub pending_items: StatefulTable<PendingItem>,
    pub watchlist: StatefulTable<WatchlistItem>,
    pub missing_subs: StatefulTable<MissingSubtitle>,
    pub library_view: LibraryView,
    pub library_list: StatefulList<String>,
    pub library_movies: Vec<Movie>,
    pub library_shows: Vec<Show>,
    pub library_anime: Vec<Anime>,
    pub settings_entries: StatefulTable<SettingsEntry>,

    // Library stats
    pub library_stats: LibraryStats,

    // Transfer info
    pub transfer_info: TransferInfo,

    // Search state
    pub search_query: String,
    pub is_searching: bool,
    pub search_indexers: Vec<String>,

    // Confirmation dialog
    pub confirm_message: String,
    pub confirm_callback: Option<Box<dyn FnOnce() + Send>>,
    /// Generic string data for confirm actions (e.g. torrent hash)
    pub confirm_data: Option<String>,

    // Setting edit state
    pub editing_setting_key: String,
    pub editing_setting_value: String,
    pub editing_setting_cursor: usize,

    // Status messages
    pub status_message: String,
    pub error_message: Option<String>,

    // Flag to force terminal clear on next frame
    pub needs_clear: bool,

    // Refresh timers
    pub last_refresh: chrono::DateTime<chrono::Utc>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Normal,
            focus: Focus::Sidebar,
            focused_panel: FocusedPanel::Downloads,
            theme_name: "catppuccin".to_string(),

            torrents: StatefulTable::new(Vec::new()),
            search_results: StatefulTable::new(Vec::new()),
            pending_items: StatefulTable::new(Vec::new()),
            watchlist: StatefulTable::new(Vec::new()),
            missing_subs: StatefulTable::new(Vec::new()),
            library_view: LibraryView::Categories,
            library_list: StatefulList::new(vec!["Movies".to_string(), "Shows".to_string(), "Anime".to_string()]),
            library_movies: Vec::new(),
            library_shows: Vec::new(),
            library_anime: Vec::new(),
            settings_entries: StatefulTable::new(Vec::new()),

            library_stats: LibraryStats::default(),
            transfer_info: TransferInfo {
                download_speed: 0,
                upload_speed: 0,
                dht_nodes: 0,
            },

            search_query: String::new(),
            is_searching: false,
            search_indexers: vec!["yts".to_string(), "tpb".to_string(), "1337x".to_string()],

            confirm_message: String::new(),
            confirm_callback: None,
            confirm_data: None,

            editing_setting_key: String::new(),
            editing_setting_value: String::new(),
            editing_setting_cursor: 0,

            status_message: "Ready".to_string(),
            error_message: None,

            needs_clear: false,

            last_refresh: chrono::Utc::now(),
        }
    }

    /// Navigate down in current panel's item list
    pub fn navigate_down(&mut self) {
        match self.focused_panel {
            FocusedPanel::Downloads => self.torrents.select_next(),
            FocusedPanel::Search => self.search_results.select_next(),
            FocusedPanel::Organize => self.pending_items.select_next(),
            FocusedPanel::Watchlist => self.watchlist.select_next(),
            FocusedPanel::Library => self.library_list.select_next(),
            FocusedPanel::Settings => self.settings_select_next(),
        }
    }

    /// Navigate up in current panel's item list
    pub fn navigate_up(&mut self) {
        match self.focused_panel {
            FocusedPanel::Downloads => self.torrents.select_previous(),
            FocusedPanel::Search => self.search_results.select_previous(),
            FocusedPanel::Organize => self.pending_items.select_previous(),
            FocusedPanel::Watchlist => self.watchlist.select_previous(),
            FocusedPanel::Library => self.library_list.select_previous(),
            FocusedPanel::Settings => self.settings_select_previous(),
        }
    }

    /// Navigate to first item in current panel
    pub fn navigate_first(&mut self) {
        match self.focused_panel {
            FocusedPanel::Downloads => self.torrents.select_first(),
            FocusedPanel::Search => self.search_results.select_first(),
            FocusedPanel::Organize => self.pending_items.select_first(),
            FocusedPanel::Watchlist => self.watchlist.select_first(),
            FocusedPanel::Library => self.library_list.select_first(),
            FocusedPanel::Settings => self.settings_select_first(),
        }
    }

    /// Navigate to last item in current panel
    pub fn navigate_last(&mut self) {
        match self.focused_panel {
            FocusedPanel::Downloads => self.torrents.select_last(),
            FocusedPanel::Search => self.search_results.select_last(),
            FocusedPanel::Organize => self.pending_items.select_last(),
            FocusedPanel::Watchlist => self.watchlist.select_last(),
            FocusedPanel::Library => self.library_list.select_last(),
            FocusedPanel::Settings => self.settings_select_last(),
        }
    }

    fn is_settings_header(entry: &SettingsEntry) -> bool {
        entry.key.starts_with("──")
    }

    fn settings_select_next(&mut self) {
        let len = self.settings_entries.items.len();
        if len == 0 { return; }
        let current = self.settings_entries.state.selected().unwrap_or(0);
        for offset in 1..len {
            let idx = (current + offset) % len;
            if !Self::is_settings_header(&self.settings_entries.items[idx]) {
                self.settings_entries.state.select(Some(idx));
                return;
            }
        }
    }

    fn settings_select_previous(&mut self) {
        let len = self.settings_entries.items.len();
        if len == 0 { return; }
        let current = self.settings_entries.state.selected().unwrap_or(0);
        for offset in 1..len {
            let idx = (current + len - offset) % len;
            if !Self::is_settings_header(&self.settings_entries.items[idx]) {
                self.settings_entries.state.select(Some(idx));
                return;
            }
        }
    }

    pub fn settings_select_first(&mut self) {
        for (i, entry) in self.settings_entries.items.iter().enumerate() {
            if !Self::is_settings_header(entry) {
                self.settings_entries.state.select(Some(i));
                return;
            }
        }
    }

    fn settings_select_last(&mut self) {
        for (i, entry) in self.settings_entries.items.iter().enumerate().rev() {
            if !Self::is_settings_header(entry) {
                self.settings_entries.state.select(Some(i));
                return;
            }
        }
    }

    /// Move to next panel in sidebar
    pub fn next_panel(&mut self) {
        let panels = FocusedPanel::all();
        let current_idx = panels.iter().position(|p| *p == self.focused_panel).unwrap_or(0);
        let next_idx = (current_idx + 1) % panels.len();
        self.focused_panel = panels[next_idx];
    }

    /// Move to previous panel in sidebar
    pub fn prev_panel(&mut self) {
        let panels = FocusedPanel::all();
        let current_idx = panels.iter().position(|p| *p == self.focused_panel).unwrap_or(0);
        let prev_idx = if current_idx == 0 { panels.len() - 1 } else { current_idx - 1 };
        self.focused_panel = panels[prev_idx];
    }

    pub fn enter_content(&mut self) {
        self.focus = Focus::Content;
    }

    pub fn return_to_sidebar(&mut self) {
        self.focus = Focus::Sidebar;
    }

    pub fn set_mode(&mut self, mode: AppMode) {
        self.mode = mode;
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.error_message = Some(message.into());
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
