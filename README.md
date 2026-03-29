
<h1 align="center">RustyFin</h1>

<p align="center">
  <b>Terminal-first power for your Jellyfin empire</b><br>
  Browse. Search. Download. Organize. All from the comfort of your terminal.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-DEA584?style=flat&logo=rust&logoColor=white" alt="Rust"/>
  <img src="https://img.shields.io/badge/Jellyfin-00A4EF?style=flat&logo=jellyfin&logoColor=white" alt="Jellyfin"/>
  <img src="https://img.shields.io/badge/License-MIT-green?style=flat" alt="License"/>
</p>

---

## Why RustyFin?

Because your media server deserves better than a browser tab. RustyFin brings full Jellyfin media management to your terminal with blazing fast performance and zero bloat.

### Features

| Feature | What it does |
|---------|--------------|
| **Library Browser** | Drill-down navigation: Movies / Shows / Anime > Seasons > Episodes |
| **Torrent Search** | Search across 7 indexers: YTS, TPB, 1337x, EZTV, Nyaa, TorrentGalaxy, LimeTorrents |
| **qBittorrent Integration** | Download, pause, resume, delete torrents directly from the TUI |
| **Auto-Organize** | Move completed downloads into your Jellyfin library structure |
| **Subtitle Manager** | Download Arabic subtitles via SubDL chain |
| **Trakt.tv Sync** | Movie + Show + Anime watchlists, search directly from watchlist |
| **mpv Playback** | Play any file with mpv directly from the TUI |
| **Editable Settings** | Configure everything in-app: paths, credentials, quality, indexers |
| **Themes** | 5 built-in themes: Catppuccin, Dracula, Gruvbox, Nord, Rose Pine |

---

## Installation

### Requirements

- **Rust** (1.70+) - [Install Rust](https://rustup.rs/)
- **qBittorrent** with Web UI enabled
- **mpv** (optional, for playback)

### Quick Install

```bash
git clone https://github.com/janooh37-hue/rustyfin.git
cd rustyfin
./install.sh
```

This builds the release binary, installs it to `~/.cargo/bin/`, and creates a shorthand symlink.

After installation:

```bash
rustyfin    # full name
rf          # shorthand
```

### Manual Install

```bash
git clone https://github.com/janooh37-hue/rustyfin.git
cd rustyfin
cargo build --release

# Copy binary to your PATH
cp target/release/rustyfin ~/.cargo/bin/
ln -s ~/.cargo/bin/rustyfin ~/.cargo/bin/rf
```

### Updating

```bash
cd rustyfin
git pull
./install.sh
```

---

## First Run

On first launch, RustyFin opens directly with sensible defaults. A config file is automatically created at `~/.config/rustyfin/config.json`.

To configure the app, go to the **Settings** panel and edit:
- Media paths (movies, shows, anime, downloads)
- qBittorrent connection (host, username, password)
- Trakt.tv credentials (username, client ID)
- Quality preferences, search indexers, and more

All changes save instantly to the config file. Passwords are masked in the UI.

---

## Controls

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

## Configuration

Config file: `~/.config/rustyfin/config.json`

```json
{
  "paths": {
    "download_dir": "/home/user/Downloads",
    "movies_dir": "/home/user/media/movies",
    "shows_dir": "/home/user/media/tv",
    "anime_dir": "/home/user/media/anime"
  },
  "qbittorrent": {
    "host": "http://localhost:8080",
    "username": "admin",
    "password": "your-password"
  },
  "trakt": {
    "username": "your-trakt-username",
    "client_id": "your-trakt-client-id"
  },
  "settings": {
    "quality_priority": ["2160p", "1080p"],
    "max_size_gb": 10,
    "min_seeds": 5,
    "avoid_cam": true,
    "search_indexers": ["yts", "tpb", "1337x"]
  }
}
```

You can edit this file directly or use the Settings panel in the TUI.

---

## CLI Options

```
rf [OPTIONS]

Options:
  -c, --config <PATH>   Config file path [default: ~/.config/rustyfin/config.json]
  -t, --theme <THEME>   Theme: catppuccin, dracula, gruvbox, nord, rosepine [default: gruvbox]
  -v, --verbose         Enable verbose logging (writes to ~/.config/rustyfin/rustyfin.log)
  -h, --help            Show help
  -V, --version         Show version
```

---

## Architecture

```
rustyfin/
  mediastation-cli/     -> CLI entrypoint (binary: rustyfin)
  mediastation-core/    -> Core library:
    ├── services/       -> qBittorrent, Trakt, Search, Organize, Subtitle, Library
    ├── ui/             -> TUI app loop, rendering, state management
    ├── models/         -> Data structures
    └── config.rs       -> Configuration loading/saving
```

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

<p align="center">
  <sub>Built with Rust and determination</sub>
</p>
