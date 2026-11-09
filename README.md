# Squirt

A music player, library manager and sync app for the Microsoft Zune, for macOS,
Windows and Linux. Named after the Zune's wireless song sharing.

Status: early. Works with the Zune HD. Flash and hard-drive Zunes come later.

## What it does

- Plays your library: MP3, AAC, ALAC, FLAC, Vorbis and WAV, gapless.
- Indexes watched folders of music, videos, pictures and podcasts.
- Copies music to a Zune with artist, album and cover art, as the Zune
  software did.
- Shows what is on the Zune, its storage, battery and color.
- Scrobbles to Last.fm and shows what you play on Discord.

## Layout

| Path | What |
|---|---|
| `crates/squirt-mtp` | MTP and MTPZ in pure Rust |
| `crates/squirt-library` | SQLite index of watched folders |
| `crates/squirt-player` | Audio playback (symphonia + cpal) |
| `crates/squirt-social` | Last.fm and Discord |
| `crates/squirt-service` | The backend the app and dev API share |
| `crates/squirt-cli` | `squirt`, a command-line tool for testing |
| `app/` | Tauri + Svelte desktop app |

## MTPZ keys

Zunes require an MTPZ handshake before they accept transfers. The keys are not
in the repository. Release builds compile them in from the `SQUIRT_MTPZ_DATA`
CI secret. For local builds, set the same variable in `.cargo/config.toml`, or
put a libmtp-format key file at `~/.mtpz-data` (it takes precedence). Check with:

    cargo run -- keys

## Running

    cd app && npm install && npx tauri dev    # the app
    cargo run -- serve                        # dev API for the browser preview at :5180

Release builds bundle a minimal LGPL ffmpeg. Build it with
`scripts/build-ffmpeg.sh`, then run
`npx tauri build --config src-tauri/tauri.release.conf.json`.

## License

MIT. The MTPZ handshake is a clean-room implementation of
`docs/mtpz-protocol.md`.
