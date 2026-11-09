# UI cut of the ad (internal)

Records the real Squirt UI and cuts it to a song from the local library.
The output contains personal library data and copyrighted music, so it is
not committed.

1. Run `squirt serve` and the app's Vite dev server (:5180), with the Zune
   connected and the desktop app closed.
2. `node capture.mjs` records `ui.webm` and `marks.json`. The player is muted.
3. In `../ad`: `node render.mjs --page cut2.html --alpha --out out/cut2`
   renders the caption, intro and end-card overlay.
4. Cut the segments from `ui.webm` and composite with the overlay and song.
   The ffmpeg commands are in the session log; segment times come from
   `marks.json`.

Stop the player afterwards: it keeps playing (silently) and scrobbling.
