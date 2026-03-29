# RustyFin

<p align="center">
  <img src=".github/logo.png" alt="RustyFin" width="200"/>
</p>

<h1 align="center">RustyFin</h1>

<p align="center">
  <b>Terminal-first power for your media empire</b><br>
  Browse. Search. Download. Organize. All from the comfort of your terminal.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-DEA584?style=flat&logo=rust&logoColor=white" alt="Rust"/>
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-blue?style=flat" alt="Platform"/>
  <img src="https://img.shields.io/badge/License-MIT-green?style=flat" alt="License"/>
</p>

---

## Features

| Feature | Description |
|---------|-------------|
| **Library Browser** | Drill-down navigation: Movies / Shows / Anime > Seasons > Episodes |
| **Torrent Search** | Search across 7 indexers: YTS, TPB, 1337x, EZTV, Nyaa, TorrentGalaxy, LimeTorrents |
| **qBittorrent Integration** | Download, pause, resume, delete torrents directly from the TUI |
| **Auto-Organize** | Move completed downloads into your media library structure |
| **Subtitle Manager** | Download subtitles via SubDL chain |
| **Trakt.tv Sync** | Movie + Show + Anime watchlists, search directly from watchlist |
| **mpv Playback** | Play any file with mpv directly from the TUI |
| **OMDb Integration** | Fetch movie metadata and posters |
| **Editable Settings** | Configure everything in-app: paths, credentials, quality, indexers |
| **Themes** | 5 built-in themes: Catppuccin, Dracula, Gruvbox, Nord, Rose Pine |

---

## Requirements

- **Rust** (1.70+) - [Install Rust](https://rustup.rs/)
- **qBittorrent** with Web UI enabled
- **mpv** (optional, for playback)
- **OMDb API Key** (optional, for movie metadata) - [Get free key](https://www.omdbapi.com/apikey.aspx)

---

## Installation

### Windows

1. Install Rust from [rustup.rs](https://rustup.rs/)

2. Install mpv (recommended):
   - Download from [mpv.io](https://mpv.io/installation/) or via winget:
   ```
   winget install mpv
   ```
   - Or install to `C:\Program Files\mpv\mpv.exe`

3. Clone and build:
   ```powershell
   git clone https://github.com/janooh37-hue/rustyfin.git
   cd rustyfin
   cargo build --release
   ```

4. Run:
   ```
   .\target\release\rustyfin.exe
   ```

### Linux / macOS

```bash
git clone https://github.com/janooh37-hue/rustyfin.git
cd rustyfin
./install.sh
```

Or manual:

```bash
cargo build --release
cp target/release/rustyfin ~/.cargo/bin/
```

---

## Configuration

On first launch, RustyFin creates a config file at:
- **Windows**: `%APPDATA%\rustyfin\config.json`
- **Linux/macOS**: `~/.config/rustyfin/config.json`

### Config File Structure

```json
{
  "qbittorrent": {
    "host": "http://localhost:8080",
    "username": "your-username",
    "password": "your-password"
  },
  "trakt": {
    "username": "your-trakt-username",
    "client_id": "your-trakt-client-id"
  },
  "omdb": {
    "api_key": "your-omdb-api-key"
  },
  "telegram": {
    "bot_token": "",
    "chat_id": "",
    "enabled": false
  },
  "paths": {
    "download_dir": "C:\\Users\\YourUser\\Downloads",
    "movies_dir": "C:\\Users\\YourUser\\Videos\\Movies",
    "shows_dir": "C:\\Users\\YourUser\\Videos\\TV",
    "anime_dir": "C:\\Users\\YourUser\\Videos\\Anime"
  },
  "settings": {
    "check_interval_minutes": 15,
    "min_seeds": 5,
    "preferred_seeds": 20,
    "quality_priority": ["2160p", "1080p"],
    "max_size_gb": 10,
    "avoid_cam": true,
    "search_indexers": ["yts", "tpb", "1337x"]
  },
  "tv_settings": {
    "max_episode_size_gb": 3,
    "max_season_size_gb": 50,
    "prefer_season_packs": true,
    "quality_priority": ["2160p", "1080p"]
  }
}
```

### qBittorrent Setup

1. Enable Web UI in qBittorrent:
   - Tools > Options > Web UI
   - Enable "Web User Interface (Remote access)"
   - Set port (default: 8080)

2. Create a user account if needed:
   - Tools > Options > Web UI > "Use authentication"

### mpv on Windows

RustyFin looks for mpv in these locations on Windows:
1. PATH (recommended)
2. `C:\Program Files\mpv\mpv.exe`
3. `C:\Program Files (x86)\mpv\mpv.exe`
4. `C:\mpv\mpv.exe`

---

## Usage

### CLI Options

```
rustyfin [OPTIONS]

Options:
  -c, --config <PATH>   Config file path
  -t, --theme <THEME>    Theme: catppuccin, dracula, gruvbox, nord, rosepine
  -v, --verbose         Enable verbose logging
  -h, --help            Show help
  -V, --version         Show version
```

### Keyboard Controls

| Key | Action |
|-----|--------|
| `↑` / `k` | Navigate up |
| `↓` / `j` | Navigate down |
| `→` / `Enter` | Enter panel / Select |
| `←` / `Esc` | Back to sidebar |
| `g` / `G` | Jump to top / bottom |
| `/` | Search torrents |
| `d` | Delete torrent |
| `o` | Organize file into library |
| `S` | Download subtitle |
| `p` | Play with mpv |
| `a` | Search watchlist item |
| `P` / `u` | Pause / Resume torrent |
| `r` / `R` | Refresh panel / Refresh all |
| `?` | Help overlay |
| `q` | Quit |

### Settings Panel

- Navigate to an editable field and press **Enter** to edit inline
- Type your value, use **Left/Right** to move cursor
- **Enter** saves, **Esc** cancels
- Search indexers toggle **ON/OFF** with Enter
- Theme cycles through options with Enter

---

## Search Indexers

All indexers are toggleable in Settings:

| Indexer | Category | Type |
|---------|----------|------|
| YTS | Movies | API |
| The Pirate Bay | General | API |
| 1337x | General | Scrape |
| EZTV | TV Shows | API |
| Nyaa | Anime | Scrape |
| TorrentGalaxy | General | Scrape |
| LimeTorrents | General | Scrape |

---

## Architecture

```
rustyfin/
├── mediastation-cli/      CLI entrypoint (binary: rustyfin)
├── mediastation-core/     Core library:
│   ├── services/          qBittorrent, Trakt, Search, Organize, Subtitle, Library, MediaInfo
│   ├── ui/                TUI app loop, rendering, state management
│   ├── models/            Data structures
│   └── config.rs          Configuration loading/saving
└── install.sh             Linux/macOS install script
```

---

## Tech Stack

- **Rust** - Memory-safe, blazing fast
- **Ratatui** - Terminal UI framework
- **Crossterm** - Terminal backend
- **Reqwest** - HTTP client
- **Tokio** - Async runtime
- **Scraper** - HTML parsing for web scrapers

---

## Contributing

Pull requests welcome. Found a bug? Open an issue. Want a feature? Tell us.

---

## License

MIT

---

<p align="center">
  <sub>Built with Rust and determination</sub>
</p>
