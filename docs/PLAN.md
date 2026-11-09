# Squirt plan

## Decisions

- **Pure-Rust MTP/MTPZ over `nusb`, not libmtp.** Homebrew builds libmtp with
  `--disable-mtpz`, and Windows builds of libmtp are painful. Use a libmtp
  built from source with MTPZ turned on only as a reference to test against.
- **License: MIT.** MTPZ is implemented clean-room: a spec is written from
  public sources (`docs/mtpz-protocol.md`), and the code is written only
  from that spec. Implementers must not read libmtp `mtpz.c` or other
  GPL/LGPL sources.
- **Keys are user-supplied** in libmtp's `.mtpz-data` format. We never ship
  or redistribute them.
- **UI: Tauri + Svelte.** Ship ffmpeg as a sidecar for video transcoding.
- **Device order:** Zune HD (045e:063e) first, then classic/flash (045e:0710).
- **Name:** no "Zune" in the product name (Microsoft trademark).

## Milestones

1. **CLI spike, Zune HD.**
   - [x] Detect the device.
   - [x] PTP container codec.
   - [x] Key file loader.
   - [x] Bulk transport: claim the interface, then send command, data and
     response containers. Handles the Zune HD quirks: unconfigured device,
     split data headers, and stale replies left by an aborted run.
   - [x] GetDeviceInfo and OpenSession (`squirt info`).
   - [x] MTPZ handshake, written clean-room from `docs/mtpz-protocol.md`
     (`squirt auth`). **Works on a Zune HD, firmware 4.5.114.**
   - [x] GetStorageIDs and GetStorageInfo (`squirt ls`).
   - [x] Push one MP3 with metadata (`squirt push`), into
     `Music/<Album Artist>/<Album>/NN Title.mp3`, matching the Zune software layout.
   - [x] Artist and album objects with cover art (`squirt push`). The Zune
     ignores a track's Artist/Album properties unless the track links to an
     artist object (0xDAB9) and an album object (0xDABB). See
     `crates/squirt-mtp/src/library.rs`.
2. **Library sync:**
   - [x] Push many files and folders in one connection (`squirt push DIR...`),
     with a progress bar, skipping tracks already on the device. A cache of
     folder contents makes this fast. About 6 MB/s on a Zune HD; the device is
     the bottleneck.
   - [x] Delete albums with their tracks and emptied folders (`squirt albums --delete-matching`).
   - [x] AAC `.m4a` (format 0xB215); ALAC is refused with a message. Art falls
     back to cover.jpg/folder.jpg. Multi-disc albums encode the disc in the
     track number (disc 2 track 3 = 203), because the Zune HD's disc
     property only accepts 0 or 1.
   - [x] Skip detection by (title, track number), because the device renames
     .m4a files. It uses a group-3 GetObjectPropList; the Zune refuses
     single-property depth-1 queries except for the file name.
   - [ ] Playlists. Single-track delete. Transcode FLAC/ALAC/OGG.
3. **Video:** ffmpeg presets. Zune HD: H.264 Baseline at 480x272 (720p for
   dock/TV out; check this). Classic: 320x240.
4. **Desktop app: player, library manager and sync** (Tauri + Svelte).
   Decided 2026-11-08: Squirt is also a music player and library manager.
   - Library (`crates/squirt-library`): a SQLite index of watched folders,
     with incremental rescans and an art thumbnail cache. Files are never
     moved unless the user runs the opt-in "organize files" action. Tag edits
     are written into the files.
     - [x] Index, scanner, art cache, album/track queries (`squirt library`).
     - [ ] Folder watching, tag editing, playlists, organize action.
   - Player (`crates/squirt-player`): symphonia + cpal, so formats behave the
     same on every OS (no webview audio). Queue, gapless, media keys
     (souvlaki), play counts.
   - Sync: manual drag-to-device always works. An optional sync list keeps
     chosen artists, albums and playlists matched on the Zune. FLAC/ALAC are
     converted when synced.
   - UI: Zune-desktop look (Selawik, lowercase pivots). In a plain browser it
     uses a local dev API, so the preview shows the real library.
5. **Classic/flash Zunes:** quirks and format presets.
6. **Long shot: apps.** XNA titles. Look into how the Zune software deployed
   them, and OpenZDK homebrew.

## Platform notes

- macOS: libusb-style access works, but Image Capture can grab the device first.
- Linux: needs udev rules for 045e:063e and 045e:0710.
- Windows: the Zune driver owns the device. Users must bind it to WinUSB
  (Zadig), or we document a driver swap.

## Open questions

- Exact MTPZ opcodes and flow. The values in `container::op` come from libmtp's
  `ptp.h` and still need checking on a real device.
