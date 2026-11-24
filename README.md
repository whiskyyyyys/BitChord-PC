# WhisMuno

**Whis** **Mu**sic **No**tch — a modern YouTube Music desktop player for Windows x64, featuring a midnight-blue & mint interface, glowing **W** icon, persistent Discord Rich Presence, mandatory auto-updater, and a smooth top-docked Dynamic Island lyrics notch.

**[Download Latest Release (Windows x64)](https://github.com/whiskyyyyys/BitChord-PC/releases/latest)** · [All Releases](https://github.com/whiskyyyyys/BitChord-PC/releases) · [Actions Builds](https://github.com/whiskyyyyys/BitChord-PC/actions)

## Screenshots

<p align="center">
  <img src="docs/readme/hero.jpg" alt="WhisMuno Player" width="100%" />
</p>

| Home & Library | Search & Browse |
| :---: | :---: |
| ![Home](docs/readme/home.jpg) | ![Search](docs/readme/search.jpg) |

| Themes & Customization | Artist View |
| :---: | :---: |
| ![Themes](docs/readme/settings-themes.jpg) | ![Artist](docs/readme/artist.jpg) |

## Features

- **Dynamic Island Lyrics Notch** — open **Settings → Notch** to enable the top-docked OLED Dynamic Island bar.
  - **Compact mode**: mini album cover, auto-width centered lyrics with smooth S-curve glide, rising & word-synced highlight, 4-bar animated equalizer, and a vector **Tuck** button (`⌃`) to collapse into a clean mini-capsule.
  - **Expanded mode** (click the notch): album artwork, track & artist info, synced lyric line, seekable progress bar, crisp vector transport controls (**Previous / Play-Pause / Next**), custom mint volume bar + mute button, mini-capsule tuck, collapse, and off buttons.
- **Dual-Mode Lyrics (Side Panel & Full Screen)** — click the **Lyrics** button in the player bar to dock synced lyrics on the right side (Spotify-style) while browsing and picking songs, or switch anytime to **Full Lyrics** (and back via **Side Lyrics**).
- **Music and lyrics** — browse your library, play tracks, and follow word/line-synced lyrics when available.
- **Discord Rich Presence** — displays *"Listening to WhisMuno"* with the current song and artist on your profile (including paused status when playback pauses) and recovers automatically if Discord restarts.
- **Automatic Mandatory Updates** — checks **[WhisMuno Releases](https://github.com/whiskyyyyys/BitChord-PC/releases/latest)** automatically every 60 seconds and prompts to update & restart when a newer build is released.

## Getting started (Windows x64)

Download from **[Releases](https://github.com/whiskyyyyys/BitChord-PC/releases/latest)**:

- **Windows Installer (`.exe`)**: download and run `WhisMuno-2.0.8-Windows-x64-Setup.exe`.
- **Windows Portable (`.zip`)**: download `WhisMuno-2.0.8-Windows-x64-Portable.zip`, extract all files into one folder, and run `bitchord.exe`.

For Discord Rich Presence, open Discord desktop, enable activity sharing in Discord, then turn on **Settings → Connections → Discord** in WhisMuno.

## Build from source

Requirements on Windows: **Rust stable** and **MSVC Build Tools**.

```powershell
cargo build --locked --release
```

Every push to `main` triggers `.github/workflows/custom-windows.yml`, which compiles WhisMuno for Windows x64 and publishes both the Setup `.exe` and Portable `.zip` to **GitHub Releases**.

Based on [BitChord PC](https://github.com/jackychan4891/BitChord-PC-Official) under [GPL-3.0-only](LICENSE). WhisMuno modifications are distributed under the same license.
