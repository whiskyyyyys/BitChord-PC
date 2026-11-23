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

- **Coucou-style Dynamic Island Notch** — open **Settings → Notch** to enable the top-docked Dynamic Island bar.
  - **Compact mode**: mini album cover, live synced lyrics or track title, and animated 3-bar equalizer.
  - **Expanded mode** (click the notch): album artwork, track & artist info, synced lyric line, progress bar, **Previous / Play-Pause / Next** controls, volume slider, Collapse (`⌃`), and Hide (`×`).
- **Music and lyrics** — browse your library, play tracks, and follow word/line-synced lyrics when available.
- **Discord Rich Presence** — share the current song and artist on your profile, with periodic refresh so it recovers cleanly if Discord restarts.
- **Portable Windows build** — download the ZIP from **Releases**, extract it, and run `bitchord.exe`. No installer required.

## Getting started

1. Download `BitChord-x-Whis-Windows-x64.zip` from **[Releases](https://github.com/whiskyyyyys/BitChord-PC/releases/latest)**.
2. Extract the entire ZIP into one folder (keep `bitchord.exe`, `WebView2Loader.dll`, and `portable.txt` together).
3. Run `bitchord.exe`.
4. For Rich Presence, open Discord desktop, enable activity sharing in Discord, then turn on **Settings → Connections → Discord** in BitChord.

## Build from source

Requirements on Windows: **Rust stable** and **MSVC Build Tools**.

```powershell
cargo build --locked --release
```

Every push to `main` also triggers `.github/workflows/custom-windows.yml`, which compiles `bitchord.exe`, packages the portable ZIP, and publishes it to **GitHub Releases**.

Based on BitChord PC under [GPL-3.0-only](LICENSE). Whis modifications are distributed under the same license.
