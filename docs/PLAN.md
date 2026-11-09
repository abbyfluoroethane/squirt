# Plan

## Decisions

- **MTP in pure Rust, over `nusb`.** Homebrew's libmtp is built without MTPZ,
  and libmtp is hard to build on Windows.
- **MIT, with a clean-room MTPZ.** The handshake is written only from
  `docs/mtpz-protocol.md`. Do not read libmtp's `mtpz.c` or other GPL/LGPL
  code when working on it.
- **Users supply MTPZ keys** (`~/.mtpz-data`). Squirt never ships them.
- **No "Zune" in the product name.** It is a Microsoft trademark.
- **Built-in service credentials.** The Last.fm key and Discord app ID are in
  the source. The Last.fm secret comes from `SQUIRT_LASTFM_API_SECRET` at
  build time and is not committed.
- **Minimal LGPL ffmpeg as a sidecar.** H.264 uses the OS encoder, so no GPL
  code ships.

## Done

- MTP transport, MTPZ handshake, storage, object transfer. Tested on a Zune HD,
  firmware 4.5.114.
- Copying music: artist and album objects, cover art, AAC, multi-disc albums,
  skipping tracks already present, recovering from disconnects. About 6 MB/s.
- Library: watched folders per media type, incremental scans, thumbnails.
- Player: gapless, seeking, media keys and the OS now-playing panel.
- App: collection, device, social and settings pages; full-screen now playing.
- Device page: info, music, rename, refresh, disconnect, slate or platinum.
- Last.fm scrobbling and Discord Rich Presence.

## Next

1. List the Zune's own videos, pictures and podcasts.
2. Copy pictures and podcasts to the Zune.
3. Convert and copy videos (H.264 Baseline, 480×272, AAC).
4. Sync from the app: drag to device, sync lists, play counts.
5. Library tools: tag editing, playlists, optional file organizing.
6. Flash and hard-drive Zunes.
7. Maybe: apps (XNA titles, OpenZDK homebrew).

## Zune HD quirks

Details are in `docs/mtpz-protocol.md` §11a and `crates/squirt-mtp`.

- macOS can leave the device unconfigured. Squirt selects configuration 1.
- Data containers must send the 12-byte header as its own USB transfer.
- A track's Artist and Album properties are ignored unless it links to an
  artist object (0xDAB9) and an album object (0xDABB).
- The device renames `.m4a` files, so tracks are matched by title and track
  number.
- Disc number only accepts 0 or 1. Multi-disc albums use track 203 for disc 2,
  track 3.
- Depth-1 property queries work only for the file name or a whole property
  group.

## Platform notes

- macOS: Image Capture can grab the device first.
- Linux: needs udev rules for 045e:063e (HD) and 045e:0710 (others).
- Windows: the Zune driver owns the device. Users must switch it to WinUSB
  (Zadig).
