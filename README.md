# BitChord x Whis

A portable YouTube Music desktop player for Windows x64, rebuilt from [BitChord PC](https://github.com/jackychan4891/BitChord-PC-Official) with the **Whis** midnight-blue & mint interface, mint **W** app icon, and a **Coucou-style Dynamic Island notch**.

**[Download Latest Release (Windows x64)](https://github.com/whiskyyyyys/BitChord-PC/releases/latest)** · [All Releases](https://github.com/whiskyyyyys/BitChord-PC/releases) · [Actions Builds](https://github.com/whiskyyyyys/BitChord-PC/actions)

## Screenshots

<p align="center">
  <img src="docs/readme/hero.jpg" alt="BitChord x Whis Player" width="100%" />
</p>

| Home & Library | Search & Browse |
| :---: | :---: |
| ![Home](docs/readme/home.jpg) | ![Search](docs/readme/search.jpg) |

| Themes & Customization | Artist View |
| :---: | :---: |
| ![Themes](docs/readme/settings-themes.jpg) | ![Artist](docs/readme/artist.jpg) |

## Features

- **Coucou & Nusa-style Dynamic Island Notch** — open **Settings → Notch** to enable the top-docked OLED Dynamic Island bar.
  - **Compact mode**: mini album cover, auto-width centered lyrics with smooth rising & word-synced highlight, 4-bar animated equalizer, and a quick **Tuck (`⌃`)** button to hide into a slim top-edge pill.
  - **Expanded mode** (click the notch): album artwork, track & artist info, synced lyric line, seekable progress bar, **Previous / Play-Pause / Next** controls, volume slider, Tuck (`⌃`), Collapse (`−`), and Turn off (`×`).
- **Music and lyrics** — browse your library, play tracks, and follow word/line-synced lyrics when available.
- **Discord Rich Presence** — shares the current song and artist on your profile (including paused status when playback pauses) and recovers automatically if Discord restarts.
- **Portable & Installer builds** — available as Windows `.exe` installer, Windows `.zip` portable, macOS `.dmg`, and Linux `.tar.gz`.

## Getting started

Download the build for your OS from **[Releases](https://github.com/whiskyyyyys/BitChord-PC/releases/latest)**:

- **Windows (Installer)**: download and run `BitChord-x-Whis-2.0.3-Windows-x64-Setup.exe`.
- **Windows (Portable)**: download `BitChord-x-Whis-2.0.3-Windows-x64-Portable.zip`, extract all files into one folder, and run `bitchord.exe`.
- **macOS**: open `BitChord-x-Whis-2.0.3-macOS.dmg` and drag `bitchord.app` to **Applications** (first launch: right-click → **Open**).
- **Linux x64**: extract `BitChord-x-Whis-2.0.3-Linux-x64.tar.gz` and run `./install.sh` (or run `./bitchord` directly).

For Discord Rich Presence, open Discord desktop, enable activity sharing in Discord, then turn on **Settings → Connections → Discord** in BitChord.

## Build from source

Requirements on Windows: **Rust stable** and **MSVC Build Tools**.

```powershell
cargo build --locked --release
```

Every push to `main` also triggers `.github/workflows/custom-windows.yml`, which compiles `bitchord.exe`, packages the portable ZIP, and publishes it to **GitHub Releases**.

Based on BitChord PC under [GPL-3.0-only](LICENSE). Whis modifications are distributed under the same license.
