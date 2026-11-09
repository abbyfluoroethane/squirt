# squirt: 30 s ad script and shot list

1920x1080, 30 fps, 30.0 s. All times are absolute seconds from the start of the ad.

## 1. Concept
The software went quiet years ago, but the zune still works. We open on a dark, silent slate, then a droplet lands and the screen lights up: library, sync, now playing, one calm Metro pivot after another. Dry and warm, never loud.

**Tagline:** `your zune, back on the air.`

**Global style:** bg #0b0b0b, white Selawik Light lowercase, labels 22 px, hero text 96-120 px, numbers IBM Plex Mono. Margins 120 px. Gradient (#ec008c to #f7941d) is accent only: progress bars, 2-4 px rules, device screen, small mark; under 8% of the frame. Default entrance: fade in plus slide 80 px horizontally, 0.7 s, cubic-bezier(.1,.9,.2,1), stagger 60 ms. Default exit: fade out 0.3 s, no slide. Pivot moves: slide 80 px left.

**Overlaps:** s2 starts at 5.6, s3 at 11.6, s4 at 17.6 (0.4 s crossfade each, text of the outgoing scene is gone by the boundary). s4 to s5 is a hard cut at 24.0 on the music lift (s4 fades its content to black 23.7-24.0).

## 2. Scenes

### s1 (0.0-6.0) "still here"
- Black frame. Zune HD slate illustration, screen OFF, centered at x 1180, y 540, height 640 px, tilted upright, slightly right of center. Dim rim light only, no gradient.
- Device fades in 0.2-1.2 (fade only, no slide).
- Text, left-aligned at x 120, baseline y 470, 112 px:
  - 0.8-3.0 `the zune is still here.` (enters from left, slide 80 px)
  - 3.2-5.6 `its software isn't.` (replaces line 1: line 1 exits 3.0, line 2 enters 3.2)
- 4.6: a 3 px magenta-to-orange rule draws on the device screen's bottom edge, left to right, 0.8 s (first color on screen). Small label at x 120, y 560, 22 px, #8a8a8a, 4.2-5.6: `no signal`.
- Out: crossfade into s2 5.6-6.0; device slides out to the left 80 px with the fade.

### s2 (5.6-12.0) "the library"
- 6.0: droplet logo (three folded layers: pink, rose, amber) 96 px at x 120, y 120, scales 0.8 to 1.0 and fades in 0.5 s. Wordmark `squirt` 40 px beside it at x 236, appears 6.1.
- Text at x 120, y 300, 96 px:
  - 6.4-9.0 `so we made squirt.`
  - 9.2-11.6 `your whole library. gapless.`
- App window mockup, 1500x640 at x 300, y 400, square corners, bg #0b0b0b, 1 px #2a2a2a border. Enters 6.6 sliding from the right 80 px.
  - Pivot header 64 px: `collection` (white, active) `device` `social` `settings` (#666). Sub-pivot labels 22 px: `music videos pictures podcasts` with `music` active.
  - 3 columns (artists / albums / songs) at 7.2, columns stagger 60 ms.
  - Artists: `marlowe & the tides`, `paper lanterns`, `kestrel hush`, `ola vance`. Selected at 8.0: `kestrel hush`.
  - Albums: `low season`, `soft machines`. Selected at 8.4: `low season`.
  - Songs: `glass hour 3:42`, `harbor lights 4:05`, `slow weather 3:18`, `ninety nine 2:56` (durations in Plex Mono).
  - 9.4: bottom bar slides up: 3 round outline transport buttons, cover (flat magenta/slate abstract square, no real art), 4 px gradient progress bar filling 0 to 22% by 11.6.
- Format chips, Plex Mono 22 px, appear at 10.2-11.6 staggered 60 ms under the window: `mp3 aac alac flac vorbis wav`.

### s3 (11.6-18.0) "the device"
- Pivot slide: app window slides left off-frame 11.6-12.3. `device` pivot title 120 px enters at x 120, y 140 at 12.0.
- Zune HD illustration (slate) at x 1250, y 540, height 700, screen showing a gradient-filled panel with `now playing` text. Enters from right 12.2.
- Left column, x 120, y 400, 28 px labels over Plex Mono 56 px values, stagger 60 ms, appear 12.6:
  - `model` / `zune hd`; `color` / `slate`; `storage` / `12.6 gb free`; `battery` / `82%`.
- Text at x 120, y 820, 80 px:
  - 13.4-15.6 `artist, album, cover art.`
  - 15.8-17.6 `the way it used to.`
- Sync strip across x 120-1000, y 940: 6 px gradient progress bar fills 13.8-17.4, label Plex Mono 22 px `copying 14 of 40 songs` ticking to `40 of 40` at 17.4.
- 17.4-17.6: all text fades; s4 crossfades in.

### s4 (17.6-24.0) "now playing"
- Full-screen now playing, full black. Artist name `kestrel hush` at 520 px, color #161616 (barely visible), drifting right to left across the background, x 300 to x -900 over 17.8-23.7, linear, can be cropped by frame.
- Album art square 520x520 at x 200, y 280 (abstract slate/rose geometric, no real art). Enters 18.0.
- Song title at x 780, y 400, 96 px: `glass hour`. Artist 36 px `kestrel hush`, album 24 px `low season` (#8a8a8a).
- Gradient progress bar x 780-1720, y 640, 4 px, runs 31% to 64% across scene. Times Plex Mono 22 px.
- Two small badges bottom-left, x 120, y 920, appear 20.0 (stagger 60 ms): `scrobbling to last.fm`, `listening to squirt on discord` (22 px white text on a 1 px #333 outlined box; no logos of those services).
- Text, x 780, y 780, 72 px: 20.6-22.8 `keep your history.` 22.9-23.7 `keep your status.`
- 23.7-24.0 fade to black. Hard cut.

### s5 (24.0-30.0) "end card"
- 24.0: black; droplet logo 220 px centered x 960, y 330, three layers slide in from left, 60 ms stagger (pink, rose, amber), 24.0-24.9.
- 24.5 wordmark `squirt` 140 px centered y 560, fade/slide 80 px.
- 25.3 tagline 56 px centered y 690: `your zune, back on the air.`
- 26.2 gradient rule 3 px, 480 px wide, centered y 770, draws left to right 0.8 s.
- 26.6 `free · open source · macOS windows linux` 28 px centered y 840 (#cfcfcf).
- 27.4 `github.com/…/squirt` Plex Mono 24 px centered y 920 (#8a8a8a).
- Hold to 30.0. At 29.4-30.0 nothing fades except audio; end on a still frame.

## 3. Music brief
Tempo **120 BPM**, 4/4 (1 bar = 2.0 s, 1 beat = 0.5 s; every scene is exactly 3 bars). Key **D minor**, lifts to **D major** at 24.0. Mood: warm, wistful, quietly hopeful; cool indie-electronic, no drops.
Instrumentation (all synthesizable): soft sine/triangle pad (slow attack), felt-like pluck (short decay, light delay), round sine bass, soft kick (low, no click), closed hats (quiet, off-beat), one airy lead (triangle with vibrato), sparkle bell (sine, high).
Structure:
- **0.0-6.0** pad only, D minor (Dm, Bb), sparse high pluck every 2 beats from 1.0. No drums. Quiet.
- **6.0** soft accent: kick plus bass note D2, pad swell. Soft kick on beats 1 and 3, hats start at 8.0. Pluck arpeggio Dm-Bb-F-C, 1 chord per bar.
- **12.0** accent: bass drops to G2, add hats on every off-beat, add lead melody (D-F-A-G) from 13.0.
- **17.6-18.0** 1-beat riser (filtered noise). **18.0** accent. Fullest bar: kick on 1-2-3-4 at quarter volume, bass moving, bell on beat 1 of each bar.
- **23.0-24.0** drums drop out at 23.0, pad holds Bb, one-bar filter sweep up.
- **24.0** lift: full D major chord (D, A, F#) bloom, kick plus bell, bass D2.
- **24.0-26.0** pluck arpeggio D-F#-A-D; **26.0-28.0** pad plus lead sustaining A.
- **28.0** drums out; **28.0-30.0** pad and one bell strike on D5 at 28.0, ringing out; reverb tail to 30.0, master fade last 0.3 s only.
Hit-point list: 0.0 start; 6.0 accent; 12.0 accent; 18.0 accent; 24.0 lift; 28.0 final bell; 30.0 silence. Master peak at -3 dBFS.

## 4. Do not
- Do not use the Zune logo, Microsoft logo or fonts, real artists, real art, or any claim of endorsement.
- Do not use uppercase text (except `macOS`), exclamation marks, or clichés (seamless, powerful, revolutionary).
- Do not exceed 2 text lines on screen at once or 8 words per line.
- Do not use a full-screen gradient or any gradient beyond the listed accents.
- Do not use rounded corners on UI (transport buttons are the only circles), drop shadows, or bounce/overshoot easing.
- Do not add features not listed (no streaming, no video editing, no shopping).
- Do not let text from one scene remain past its stated disappear time.
