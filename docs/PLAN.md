# Squirt plan

## Decisions

- **Pure-Rust MTP/MTPZ over `nusb`, not libmtp.** Homebrew builds libmtp with
  `--disable-mtpz`, and Windows builds of libmtp are painful. Use a libmtp
  built from source with MTPZ turned on only as a reference to test against.
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
   - [ ] Bulk transport: claim the interface, then send command, data and
     response containers.
   - [ ] GetDeviceInfo and OpenSession.
   - [ ] MTPZ handshake (model on libmtp `mtpz.c`; this affects the license choice).
   - [ ] GetStorageIDs and GetStorageInfo.
   - [ ] Push one MP3 with metadata.
   - [ ] Create an album object with cover art.
2. **Library sync:** list, delete and dedupe on-device content; playlists.
3. **Video:** ffmpeg presets. Zune HD: H.264 Baseline at 480x272 (720p for
   dock/TV out; check this). Classic: 320x240.
4. **Desktop app:** Tauri + Svelte. Drag-and-drop, device view, transfer queue.
5. **Classic/flash Zunes:** quirks and format presets.
6. **Long shot: apps.** XNA titles. Look into how the Zune software deployed
   them, and OpenZDK homebrew.

## Platform notes

- macOS: libusb-style access works, but Image Capture can grab the device first.
- Linux: needs udev rules for 045e:063e and 045e:0710.
- Windows: the Zune driver owns the device. Users must bind it to WinUSB
  (Zadig), or we document a driver swap.

## Open questions

- License. Porting libmtp's `mtpz.c` (LGPL-2.1+) makes that code LGPL-derived.
- Exact MTPZ opcodes and flow. The values in `container::op` come from libmtp's
  `ptp.h` and still need checking on a real device.
