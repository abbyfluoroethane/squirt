# Squirt

A cross-platform app (macOS, Windows, Linux) that loads music, video, photos
and podcasts onto a Microsoft Zune. It is named after the Zune's wireless
song-sharing feature.

Status: early. The Zune HD is the first target. Flash models come later.

## Layout

- `crates/squirt-mtp`: pure-Rust MTP and MTPZ device layer.
- `crates/squirt-cli`: the `squirt` command-line tool, used to test the device layer.
- `app/` (planned): Tauri + Svelte desktop app.

## MTPZ keys

Zunes need an MTPZ handshake before they accept transfers. Squirt does
**not** ship the keys. Put a libmtp-format key file at `~/.mtpz-data`, then
check it with:

    cargo run -- keys

## Try it

    cargo run -- detect
