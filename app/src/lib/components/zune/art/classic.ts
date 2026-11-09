// The lit screen of the classic Zunes (30, 4/8/16, 80/120): the main menu
// as the 2007 firmware drew it. Every item is the same size, full white,
// over a colorful background picture, and the list runs off the bottom of
// the screen. A small battery sits at the bottom right. Matches photos of a
// lit Zune 80 and Zune 8 side by side (Wikimedia Commons, Zune80and4.jpg).
//
// Not a model module (no `model`/`render`); the gallery skips it.

export type Backdrop = 'violet' | 'amber';

const BACKDROPS: Record<Backdrop, { base: [string, string]; glow: [string, string] }> = {
	// The Zune 80 photo: deep blue-violet with a magenta burst.
	violet: { base: ['#1b1554', '#3a1f7a'], glow: ['#c03cc8', '#5b3bd6'] },
	// The Zune 8 photo: dark brown with an amber burst.
	amber: { base: ['#1e0c05', '#3a1a08'], glow: ['#f0902a', '#a8431a'] }
};

/**
 * Screen content for a w x h screen at (x, y), in the caller's units (mm).
 * The caller clips it to the screen. `uid` prefixes the gradient ids.
 */
export function classicScreen(s: { x: number; y: number; w: number; h: number }, uid: string, backdrop: Backdrop = 'violet'): string {
	const b = BACKDROPS[backdrop];
	const id = (n: string) => `${uid}-cs-${n}`;
	// Measured off the Zune 80 photo: first baseline ~20% down, lines ~15.5%
	// apart, so "podcasts" is cut by the bottom edge.
	const size = s.h * 0.155;
	const step = s.h * 0.155;
	const x = s.x + s.w * 0.05;
	const items = ['music', 'videos', 'pictures', 'social', 'radio', 'podcasts']
		.map((t, i) => `<text x="${x.toFixed(2)}" y="${(s.y + s.h * 0.2 + i * step).toFixed(2)}" font-size="${size.toFixed(2)}" font-weight="300" fill="#fff" fill-opacity="0.96">${t}</text>`)
		.join('');
	// Battery: a small upright cell at the bottom right.
	const bw = s.w * 0.035;
	const bh = s.h * 0.06;
	const bx = s.x + s.w - bw - s.w * 0.035;
	const by = s.y + s.h - bh - s.h * 0.03;
	const battery = `<rect x="${bx.toFixed(2)}" y="${by.toFixed(2)}" width="${bw.toFixed(2)}" height="${bh.toFixed(2)}" fill="none" stroke="#fff" stroke-width="${(bw * 0.18).toFixed(2)}"/><rect x="${(bx + bw * 0.25).toFixed(2)}" y="${(by + bh * 0.3).toFixed(2)}" width="${(bw * 0.5).toFixed(2)}" height="${(bh * 0.6).toFixed(2)}" fill="#fff"/>`;
	return `<defs>
<linearGradient id="${id('base')}" x1="0" y1="0" x2="0.4" y2="1"><stop offset="0" stop-color="${b.base[0]}"/><stop offset="1" stop-color="${b.base[1]}"/></linearGradient>
<radialGradient id="${id('glow')}" cx="0.62" cy="0.42" r="0.6"><stop offset="0" stop-color="${b.glow[0]}" stop-opacity="0.85"/><stop offset="0.45" stop-color="${b.glow[1]}" stop-opacity="0.45"/><stop offset="1" stop-color="${b.glow[1]}" stop-opacity="0"/></radialGradient>
</defs>
<rect x="${s.x}" y="${s.y}" width="${s.w}" height="${s.h}" fill="url(#${id('base')})"/>
<rect x="${s.x}" y="${s.y}" width="${s.w}" height="${s.h}" fill="url(#${id('glow')})"/>
<g font-family="Selawik, 'Segoe UI', sans-serif">${items}</g>${battery}`;
}
