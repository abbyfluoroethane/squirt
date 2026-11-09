// Zune 80 (2007) and Zune 120 (2008): one shared body, front view.
// Measured from front-on photos (Wikimedia Commons "Zune80and4.jpg", black;
// "Zune red.jpg" and "Microsoft Zune 80 in Red", red) and the spec
// (61.1 x 108.2 x 12.9 mm, 3.2" 240x320 screen). Drawn in a 61.1 x 108.2 mm box.
// - Screen panel (black glass with a thin silver edge): 55.7 x 71.3 mm at
//   (2.7, 2.9); lit area 48.75 x 65 mm (3:4) at (6.35, 5.6).
// - Zune Pad: squircle centred at (30.55, 89.2); silver ring 26 mm across,
//   textured pad 21 mm inside it.
// - Back (left) and Play/Pause (right): 9 mm round-cornered keys at the same
//   height as the pad, slightly above its centre (y 85.9), 9.0 mm from each edge.
// - Front is glossy plastic (glossy red/blue, or glossy near-black); a thin
//   silver metal rim shows around the edge.
import type { ArtOptions, ZuneModel } from './types.ts';
import { classicScreen } from './classic.ts';

export const model: ZuneModel = {
	id: 'zunehdd',
	name: 'Zune 80 / 120',
	year: 2007,
	aspect: 61.1 / 108.2,
	mm: [61.1, 108.2, 12.9],
	finishes: [
		{
			id: 'black',
			name: 'black',
			source: 'https://en.wikipedia.org/wiki/Zune_80,_120',
			note: '80 GB (2007) and 120 GB (2008); glossy black front, matte black back'
		},
		{
			id: 'red',
			name: 'red',
			source: 'https://news.microsoft.com/source/2008/01/15/red-zune-80gb-and-new-collection-of-artwork-debut-on-zune-originals/',
			note: '80 GB from Jan 2008 (Valentine promotion, then retail); 120 GB red announced Sept 2008 via Zune Originals (slashgear.com/blue-red-zune-120-available-0825531/)'
		},
		{
			id: 'blue',
			name: 'blue',
			source: 'https://www.slashgear.com/blue-red-zune-120-available-0825531/',
			note: '120 GB only, Zune Originals. Blue front shade estimated; no photo opened. TechCrunch (2008-11-08) called it unannounced, sources differ on timing'
		}
	]
};

const W = 61.1;
const H = 108.2;

const FACE: Record<string, { hi: string; mid: string; lo: string; pad: string; padLo: string; ring: string }> = {
	black: { hi: '#3a3a3d', mid: '#222224', lo: '#111112', pad: '#242426', padLo: '#121213', ring: '#b9bbbf' },
	red: { hi: '#e0313a', mid: '#c4101c', lo: '#8e0812', pad: '#b01f30', padLo: '#8a1020', ring: '#d9dadd' },
	blue: { hi: '#3f86d8', mid: '#1f5fb5', lo: '#123c82', pad: '#2f68b4', padLo: '#1d4688', ring: '#d9dadd' }
};

/** Squircle (superellipse, n = 3.2) path of half-size r centred at (cx, cy). */
function squircle(cx: number, cy: number, r: number): string {
	const pts: string[] = [];
	const n = 3.2;
	for (let i = 0; i < 64; i++) {
		const t = (i / 64) * Math.PI * 2;
		const c = Math.cos(t);
		const s = Math.sin(t);
		const x = cx + r * Math.sign(c) * Math.abs(c) ** (2 / n);
		const y = cy + r * Math.sign(s) * Math.abs(s) ** (2 / n);
		pts.push(`${i ? 'L' : 'M'}${x.toFixed(2)} ${y.toFixed(2)}`);
	}
	return pts.join('') + 'Z';
}

export function render(o: ArtOptions): string {
	const f = FACE[o.finish] ?? FACE.black;
	const u = o.uid;
	const PAD = { cx: 30.55, cy: 89.2 };
	const KEY_Y = 85.9;
	const KEY_X = [4.5, 47.3];
	const LIT = { x: 6.35, y: 5.6, w: 48.75, h: 65 };


	const screen = o.lit
		? `<g clip-path="url(#${u}-clip)">
			${classicScreen(LIT, u, 'violet')}
		</g>`
		: '';

	// Back arrow and play/pause glyphs, thin light strokes.
	const glyphStroke = o.finish === 'black' ? '#d4d6da' : '#f4e6e6';
	const back = `<path d="M${KEY_X[0] + 3.3} ${KEY_Y} h2.6 M${KEY_X[0] + 3.3} ${KEY_Y} l1.5 -1.5 v3 z" fill="none" stroke="${glyphStroke}" stroke-opacity="0.85" stroke-width="0.4" stroke-linejoin="round" transform="translate(0.1 0)"/>`;
	const play = `<path d="M${KEY_X[1] + 2.6} ${KEY_Y - 1.4} l2 1.4 l-2 1.4 z M${KEY_X[1] + 5.4} ${KEY_Y - 1.4} v2.8 M${KEY_X[1] + 6.4} ${KEY_Y - 1.4} v2.8" fill="none" stroke="${glyphStroke}" stroke-opacity="0.85" stroke-width="0.4" stroke-linejoin="round"/>`;
	const key = (x: number) =>
		`<rect x="${x}" y="${KEY_Y - 4.5}" width="9" height="9" rx="3.6" fill="url(#${u}-key)" stroke="#000" stroke-opacity="0.35" stroke-width="0.25"/>
		<rect x="${x + 0.3}" y="${KEY_Y - 4.2}" width="8.4" height="8.4" rx="3.3" fill="none" stroke="#fff" stroke-opacity="0.22" stroke-width="0.25"/>`;

	return `<svg xmlns="http://www.w3.org/2000/svg" width="${(o.height * model.aspect).toFixed(2)}" height="${o.height}" viewBox="0 0 ${W} ${H}" aria-hidden="true">
	<defs>
		<linearGradient id="${u}-rim" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#e6e7ea"/><stop offset="1" stop-color="#8d9096"/></linearGradient>
		<linearGradient id="${u}-face" x1="0" y1="0" x2="0.35" y2="1"><stop offset="0" stop-color="${f.hi}"/><stop offset="0.35" stop-color="${f.mid}"/><stop offset="1" stop-color="${f.lo}"/></linearGradient>
		<linearGradient id="${u}-key" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff" stop-opacity="0.12"/><stop offset="1" stop-color="#000" stop-opacity="0.18"/></linearGradient>
		<linearGradient id="${u}-glass" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#2a2c30"/><stop offset="0.4" stop-color="#15161a"/><stop offset="0.45" stop-color="#060608"/><stop offset="1" stop-color="#020203"/></linearGradient>
		<linearGradient id="${u}-ring" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#fff"/><stop offset="0.5" stop-color="${f.ring}"/><stop offset="1" stop-color="#7d8086"/></linearGradient>
		<radialGradient id="${u}-pad" cx="0.4" cy="0.3" r="0.9"><stop offset="0" stop-color="${f.pad}"/><stop offset="1" stop-color="${f.padLo}"/></radialGradient>
		<linearGradient id="${u}-shine" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff" stop-opacity="0.22"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></linearGradient>
		<clipPath id="${u}-clip"><rect x="${LIT.x}" y="${LIT.y}" width="${LIT.w}" height="${LIT.h}"/></clipPath>
	</defs>
	<!-- Metal rim, then glossy front plastic inset by 0.5 mm. -->
	<rect width="${W}" height="${H}" rx="3.3" fill="url(#${u}-rim)"/>
	<rect x="0.5" y="0.5" width="${W - 1}" height="${H - 1}" rx="2.9" fill="url(#${u}-face)"/>
	<rect x="0.5" y="0.5" width="${W - 1}" height="${H * 0.45}" rx="2.9" fill="url(#${u}-shine)" opacity="0.5"/>
	<!-- Screen panel: black glass, thin silver edge. -->
	<rect x="2.7" y="2.9" width="55.7" height="71.3" rx="1.3" fill="url(#${u}-glass)" stroke="#9a9ca2" stroke-opacity="0.7" stroke-width="0.25"/>
	<rect x="${LIT.x}" y="${LIT.y}" width="${LIT.w}" height="${LIT.h}" fill="#030304"/>
	${screen}
	<!-- Keys. -->
	${key(KEY_X[0])}${back}${key(KEY_X[1])}${play}
	<!-- Zune Pad: silver ring, textured pad. -->
	<path d="${squircle(PAD.cx, PAD.cy, 13)}" fill="url(#${u}-ring)" stroke="#000" stroke-opacity="0.3" stroke-width="0.2"/>
	<path d="${squircle(PAD.cx, PAD.cy, 12.1)}" fill="#000" fill-opacity="0.3"/>
	<path d="${squircle(PAD.cx, PAD.cy, 11.7)}" fill="url(#${u}-pad)"/>
	<!-- Outer edge. -->
	<rect x="0.1" y="0.1" width="${W - 0.2}" height="${H - 0.2}" rx="3.2" fill="none" stroke="#000" stroke-opacity="0.45" stroke-width="0.25"/>
</svg>`;
}
