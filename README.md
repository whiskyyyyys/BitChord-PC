# BitChord x Whis

Windows x64 portable edition of [BitChord PC](https://github.com/jackychan4891/BitChord-PC-Official), with a W icon, midnight and mint colors, and Discord Rich Presence recovery during playback.

## Download

Open [Windows builds](https://github.com/whiskyyyyys/BitChord-PC/actions/workflows/custom-windows.yml), select a successful run, and download **BitChord-x-Whis-Windows-x64** from Artifacts while signed in to GitHub. Extract the whole ZIP and launch `bitchord.exe`. For Rich Presence, open Discord desktop and enable Discord under BitChord Settings → Connections.

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
