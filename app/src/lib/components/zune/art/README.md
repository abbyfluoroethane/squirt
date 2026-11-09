# Zune illustrations

One module per Zune model, each drawing that model as a flat, front-on SVG
in any of its real colorways. The device page shows the one that matches the
connected Zune; the gallery script shows them all.

## A module

`<id>.ts` in this folder, plain TypeScript with no imports except
`./types.ts` and `./classic.ts` (write the extension: `import type { ... } from './types.ts'`). `classic.ts` draws the lit menu screen shared by the 30, flash and 80/120.

    export const model: ZuneModel = { ... };
    export function render(o: ArtOptions): string; // one complete <svg>…</svg>

- `render` returns markup only: no scripts, no external images, no fonts
  other than `font-family="Selawik, 'Segoe UI', sans-serif"`.
- Size: `width = height * model.aspect`, `height = o.height`. Use a viewBox
  in millimetres of the real body (e.g. `0 0 61.2 102`), so geometry reads
  like the spec sheet.
- Prefix every `id=` with `o.uid` (several instances share a page).
- Unknown `o.finish` falls back to the first finish.
- Keep each file under ~400 lines. Comment the measurements you used.

## Style (match `zunehd.ts`, the Zune HD drawing)

- Front view, upright, no perspective. Soft gradients for material (glossy
  plastic, brushed or anodized metal, rubber), one subtle highlight, a thin
  dark outer edge. No photo realism, no text logos, no Microsoft or Zune
  logos or wordmarks. A plain shape for the Zune Pad / button is fine.
- Proportions from real dimensions and product photos: screen size and
  position, control shape and position, corner radii. This is the part
  people notice; get it right.
- Screen off: near-black glass with a faint reflection.
- Screen lit (classic models: Zune 30, flash, 80/120): black background with
  the classic menu in lowercase Selawik Light, white: `music`, `videos`,
  `pictures`, `social`, `radio`, `podcasts`, `settings`, as many as fit;
  the first item full white and larger, the rest about 45% white, and the
  time (`o.now`, h:mm, no am/pm) small at the top right. Zune HD keeps its
  lock screen (gradient, big clock, day, date).

## Colorways

List every colorway the model actually shipped in, including limited and
regional editions, each with a `source` URL you checked. Do not invent any.
If sources disagree, note it in `note`. Back finishes and engraved
special editions don't count unless the front looked different.

## Check your work

From `app/`:

    node scripts/art-gallery.mjs --only <id> --png /tmp/<you>/<id>.png

renders every finish of your model, lit and off, into one PNG. Look at it
(Read the PNG) and compare with product photos. Iterate until it is right.
Only one render runs at a time across all agents (the script waits for a
lock), so don't run it in a tight loop.
