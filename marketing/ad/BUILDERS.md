# Scene builder contract

The ad is one HTML page (`index.html`) on a 1920×1080 stage. It loads five
scenes, `scenes/s1` to `scenes/s5`, in order. `render.mjs` captures it at
30 fps by pausing every animation and setting `currentTime` for each frame.

## Your files

You own exactly two files: `scenes/sN.html` and `scenes/sN.css`. Do not edit
anything else.

- `sN.html` is one element: `<section class="scene sN"> … </section>`.
- `sN.css` holds all of your styles. Prefix every selector with `.sN` and
  every `@keyframes` name with `sN-` so scenes can't collide.

## Timing rules (the renderer depends on these)

- **All motion is CSS animation.** No JavaScript, no `<script>`, no
  `transition`, no `:hover`, no video.
- **All times are absolute, in seconds from the start of the ad.** An element
  that enters at 7.2 s uses `animation-delay: 7.2s`. Every animation starts at
  page load, and the renderer seeks them all together.
- Use `animation-fill-mode: both` (or `backwards`/`forwards` as needed) so
  every element is correct before and after its animation.
- **Your section controls its own visibility.** Give `.sN` a keyframe
  animation lasting 30 s that keeps it at `opacity: 0` outside your window and
  fades it in and out at your transitions. For example, for 6.0–12.0 s:

  ```css
  .sN { animation: sN-vis 30s linear both; }
  @keyframes sN-vis {
    0%, 19.9%   { opacity: 0; }   /* before 5.97 s */
    20%, 39.9%  { opacity: 1; }   /* 6.0–11.97 s */
    40%, 100%   { opacity: 0; }   /* from 12.0 s */
  }
  ```

  Percentages are of 30 s, so 1% = 0.3 s. Fade over a few percent if the
  script calls for a crossfade.
- Elements inside may have their own animations with absolute delays. To loop
  something (a drifting background, a moving progress bar), give it a
  duration and delay that cover your window.

## Brand

- Load nothing external. `assets/brand.css` is already loaded. It provides the
  fonts (Selawik 300/350/400/600/700, IBM Plex Mono 400/500), the colour
  variables (`--bg`, `--fg`, `--muted`, `--faint`, `--panel`, `--line`,
  `--magenta`, `--orange`, `--gradient`), `--ease-metro`, and the
  `metro-enter` and `metro-exit` keyframes.
- Metro entrance: 0.7 s, `var(--ease-metro)`, fade in while sliding 80 px,
  60 ms stagger between siblings.
- Type: Selawik Light (300) lowercase for big headings, 350/400 for body,
  600 for small column labels. Plex Mono for numbers and times.
- The gradient is an accent: progress bars, thin rules, a device screen, the
  logo. Never more than about 8% of the frame.
- Square corners everywhere, except the round transport buttons.
- Assets you may reference with `<img src="assets/…">`:
  - `assets/squirt-mark.svg`: the droplet logo (aspect 68:94).
  - `assets/zune-hd-slate.svg`: a slate Zune HD (aspect 53:102). Its screen
    is the rectangle x 6, y 6.3, w 41, h 72.9 in the SVG's 53×102 units. To
    put content on the screen, absolutely position a div over that rectangle
    (scale the numbers by your rendered size), with `overflow: hidden`.
- Fictional content only (names from SCRIPT.md). No Zune or Microsoft logos.
  "zune" in words is fine.

## Quality bar

- Pixel-precise layout at 1920×1080. Generous margins: content starts at about
  x = 160 px.
- Text must be readable on screen for at least 1.2 s after it finishes
  entering.
- Keep it calm. No bounces, no spins, no more than two things moving at once.
- Do not open a browser or render anything; the lead renders and sends you
  stills. Check your CSS by reading it: timings, prefixes, fill modes.
