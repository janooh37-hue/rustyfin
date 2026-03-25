<p align="center">
  <img src="https://raw.githubusercontent.com/janooh37-hue/rustyfin/main/.github/logo.png" alt="RustyFin" width="200"/>
</p>

<h1 align="center">RustyFin</h1>

<p align="center">
  <b>Terminal-first power for your Jellyfin empire</b><br>
  Browse. Search. Organize. All from the comfort of your terminal.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-DEA584?style=flat&logo=rust&logoColor=white" alt="Rust"/>
  <img src="https://img.shields.io/badge/Jellyfin-00A4EF?style=flat&logo=jellyfin&logoColor=white" alt="Jellyfin"/>
  <img src="https://img.shields.io/badge/License-MIT-green?style=flat" alt="License"/>
</p>

---

## Why RustyFin?

Because your media server deserves better than a browser tab. RustyFin brings the full power of Jellyfin to your terminal with blazing fast performance and zero bloat.

### Features

| Feature | What it does |
|---------|--------------|
| **Library Browser** | Navigate your Jellyfin libraries without leaving the terminal |
| **Smart Search** | Find movies, shows, and media across your entire library |
| **qBittorrent Integration** | Download torrents directly from the TUI |
| **Subtitle Manager** | Search & download subtitles for any media |
| **Trakt.tv Sync** | Track watched status across devices |
| **Auto-Organize** | Keep your media collection tidy automatically |
| **Rich Media Info** | Detailed info about your files - codecs, quality, size |

---

## Quick Start

```bash
# Clone & build
git clone https://github.com/janooh37-hue/rustyfin.git
cd rustyfin
cargo build --release

# Run it
cargo run --package mediastation-cli
```

### First Run

Create a `config.yaml` in `~/.config/rustyfin/`:

```yaml
jellyfin:
  url: "http://localhost:8096"
  api_key: "your-api-key-here"
  user_id: "your-user-id"

qbittorrent:
  url: "http://localhost:8080"
  username: "admin"
  password: "adminadmin"

trakt:
  client_id: "your-trakt-client-id"
  client_secret: "your-trakt-secret"
```

---

## Controls

| Key | Action |
|-----|--------|
| `↑↓` | Navigate |
| `←→` | Move between panels |
| `Enter` | Select / Open |
| `Esc` | Go back / Close |
| `/` | Search |
| `d` | Download (qBittorrent) |
| `s` | Search subtitles |
| `w` | Mark as watched (Trakt) |
| `r` | Refresh |
| `q` | Quit |

---

## Architecture

```
mediastation-cli     → The TUI entrypoint
mediastation-core    → The brain:
  ├── services/      → API integrations (Jellyfin, qBittorrent, Trakt)
  ├── ui/            → TUI components & rendering
  ├── models/        → Data structures
  └── config/        → Configuration handling
```

---

## Tech Stack

- **Rust** - Memory-safe, blazing fast
- **Ratatui** - Terminal UI framework
- **Reqwest** - HTTP client
- **Tokio** - Async runtime
- **Serde** - Serialization

---

## Contributing

Pull requests welcome. Found a bug? Open an issue. Want a feature? Tell us.

---

<p align="center">
  <sub>Built with ☕ and pure determination by MediaStation Team</sub>
</p>
