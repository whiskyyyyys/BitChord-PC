# BitChord x Whis

A portable YouTube Music desktop player for Windows x64, based on [BitChord PC](https://github.com/jackychan4891/BitChord-PC-Official). The Whis edition pairs a midnight-blue interface with mint accents and a W app icon.

**[Download for Windows x64](https://github.com/whiskyyyyys/BitChord-PC/releases/download/v2.0.1-whis.2/BitChord-x-Whis-Notch-Windows-x64.zip)** · [Release notes](https://github.com/whiskyyyyys/BitChord-PC/releases/latest)

[Windows redesign build](https://github.com/whiskyyyyys/BitChord-PC/actions/runs/45101931335) · Separate Songs, Playlists and Albums in navigation and search, grouped settings, and a mint W icon.

## Features

- **Lyrics notch** — in the redesign build, open **Settings → Notch**. Click the floating lyric pill to expand playback and volume controls, or use **Hide** to dismiss it. The earlier stable release keeps the switch in Settings → Appearance.

- **Music and lyrics** — browse your library, play tracks, and follow synced lyrics when available.
- **Discord Rich Presence** — share the current song and artist on your profile. While music plays, BitChord periodically refreshes the card so it can recover after Discord restarts.
- **Portable Windows build** — extract the ZIP to a folder and launch the app. No installer or Python setup required.

## Getting started

1. Download the Windows x64 ZIP under **Assets** on the release page.
2. Extract the entire ZIP and open `bitchord.exe`. Keep the accompanying files together.
3. For Rich Presence, open Discord desktop, enable activity sharing in Discord, then turn on **Settings → Connections → Discord** in BitChord and play a track.

## Source and build

The workflow checks out [the exact upstream source](https://github.com/jackychan4891/BitChord-PC-Official/tree/d7d71a128c53a2352be8ea081f9a0a423b96a9e0) and extracts the [Whis source and icon overlay](customization/whis-overlay.zip.b64). To reproduce on Windows with Git, Rust stable and MSVC Build Tools, run from this repository:

```powershell
git clone https://github.com/jackychan4891/BitChord-PC-Official.git app
git -C app checkout d7d71a128c53a2352be8ea081f9a0a423b96a9e0
[IO.File]::WriteAllBytes("$env:TEMP/whis-overlay.zip", [Convert]::FromBase64String((Get-Content customization/whis-overlay.zip.b64 -Raw)))
Expand-Archive "$env:TEMP/whis-overlay.zip" app -Force
Set-Location app
cargo build --locked --release
```

Based on BitChord PC under [GPL-3.0-only](https://github.com/jackychan4891/BitChord-PC-Official/blob/d7d71a128c53a2352be8ea081f9a0a423b96a9e0/LICENSE). Whis modifications are distributed under the same license.
