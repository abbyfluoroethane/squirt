// Zune 4 / 8 (2007) and Zune 16 (2008): one body, front-on illustration.
//
// Measurements (mm), from the Wikipedia infobox (91.5 x 41.4 x 8.5 mm,
// 1.8" 240x320 glass screen) and the front-on Commons photos Zune4gig.jpg
// (red) and Zune80and4.jpg (black), scaled at ~6.04 px/mm:
// - body 41.4 x 91.5, corner radius ~3.2;
// - black glass panel runs nearly edge to edge (0.5 mm of body colour at the
//   sides and top) and ends 50.8 mm down;
// - 27.4 x 36.6 screen (1.8" 3:4), centred, 7.1 mm below the top;
// - Zune Pad: chrome ring ~27 mm across, centre 71.2 mm down, with a
//   body-coloured squircle pad ~23 mm inside it;
// - Back and Play/Pause: ~6.6 mm squircle keys, 58 mm down, +-13.7 mm from
//   the centre line.
import type { ArtOptions, ZuneModel } from './types.ts';
import { classicScreen } from './classic.ts';

const W = 41.4;
const H = 91.5;

export const model: ZuneModel = {
	id: 'zuneflash',
	name: 'Zune 4 / 8 / 16',
	year: 2007,
	aspect: W / H,
	mm: [41.4, 91.5, 8.5],
	finishes: [
		{
			id: 'black',
			name: 'black',
			source: 'https://en.wikipedia.org/wiki/Zune',
			note: '4 and 8 GB: matte black; 16 GB: glossy black (the only retail 16 GB colour)'
		},
		{
			id: 'red',
			name: 'red',
			source: 'https://news.microsoft.com/2007/11/12/microsoft-unveils-device-customization-via-new-zune-originals-store-new-zune-players-available-tomorrow/',
			note: '4 and 8 GB at launch (2007); 16 GB via Zune Originals (2008)'
		},
		{
			id: 'pink',
			name: 'pink',
			source: 'https://news.microsoft.com/2007/11/12/microsoft-unveils-device-customization-via-new-zune-originals-store-new-zune-players-available-tomorrow/',
			note: '4 and 8 GB at launch; 16 GB via Zune Originals only'
		},
		{
			id: 'green',
			name: 'green',
			source: 'https://news.microsoft.com/2007/11/12/microsoft-unveils-device-customization-via-new-zune-originals-store-new-zune-players-available-tomorrow/',
			note: 'Olive green. 4 and 8 GB at launch; 16 GB via Zune Originals only'
		},
		{
			id: 'blue',
			name: 'blue',
			source: 'https://techcrunch.com/2008/09/07/blue-8gb-zune-available-at-frys',
			note: '2008: 8 GB at retail, 16 GB via Zune Originals. Wikipedia also lists 4 GB; unconfirmed'
		},
		{
			id: 'citron',
			name: 'citron',
			source: 'https://istartedsomething.com/20071114/zune-citron-color',
			note: 'Never sold: Microsoft staff gift, black pad. Sources disagree on 4/8 vs 16 GB'
		}
	]
};

// Front colour [light, dark], pad colour [light, dark], and button tint.
const COLORS: Record<string, { body: [string, string]; pad: [string, string]; ring?: string }> = {
	black: { body: ['#2a2a2d', '#141416'], pad: ['#2e2e31', '#121214'] },
	red: { body: ['#8d1a22', '#5e0d14'], pad: ['#932029', '#611018'] },
	pink: { body: ['#e0679c', '#c23c78'], pad: ['#e472a4', '#c74580'] },
	green: { body: ['#7b8a3a', '#56622a'], pad: ['#808f3f', '#5a672c'] },
	blue: { body: ['#3d70c0', '#1f4a94'], pad: ['#4575c4', '#244f99'] },
	citron: { body: ['#e4dc3a', '#c2b81c'], pad: ['#2e2e31', '#121214'] }
};

/** Rounded square ("squircle"-ish) centred on cx, cy. */
function squircle(cx: number, cy: number, w: number, h: number, r: number): string {
	const x = cx - w / 2;
	const y = cy - h / 2;
	return `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${r}"`;
}

export function render(o: ArtOptions): string {
	const id = (s: string) => `${o.uid}-${s}`;
	const c = COLORS[o.finish] ?? COLORS.black;
	const glossy = o.finish !== 'black';

	const cx = W / 2;
	const SCREEN = { x: 7, y: 7.1, w: 27.4, h: 36.6 };
	const GLASS = { x: 0.55, y: 0.55, w: W - 1.1, bottom: 50.8 };
	const PAD = { cy: 71.2, ring: 27, inner: 23.2 };
	const KEY = { cy: 58, dx: 13.7, s: 6.6 };


	const glyph = (gx: number, gy: number, back: boolean) =>
		back
			? `<path d="M${gx + 1.4} ${gy} H${gx - 1.2} M${gx - 0.2} ${gy - 1.1} L${gx - 1.4} ${gy} L${gx - 0.2} ${gy + 1.1}" fill="none" stroke="#fff" stroke-opacity="0.75" stroke-width="0.35" stroke-linecap="round" stroke-linejoin="round"/>`
			: `<path d="M${gx - 1.5} ${gy - 1.1} L${gx - 0.1} ${gy} L${gx - 1.5} ${gy + 1.1} Z M${gx + 0.5} ${gy - 1.1} V${gy + 1.1} M${gx + 1.4} ${gy - 1.1} V${gy + 1.1}" fill="#fff" fill-opacity="0.75" stroke="#fff" stroke-opacity="0.75" stroke-width="0.3" stroke-linejoin="round"/>`;

	const key = (kx: number, back: boolean) =>
		`${squircle(kx, KEY.cy, KEY.s, KEY.s, 2.3)} fill="url(#${id('key')})" stroke="#000" stroke-opacity="0.45" stroke-width="0.2"/>` +
		`${squircle(kx, KEY.cy, KEY.s - 0.5, KEY.s - 0.5, 2.0)} fill="none" stroke="#fff" stroke-opacity="${glossy ? 0.18 : 0.08}" stroke-width="0.15"/>` +
		glyph(kx, KEY.cy, back);

	return `<svg xmlns="http://www.w3.org/2000/svg" width="${o.height * model.aspect}" height="${o.height}" viewBox="0 0 ${W} ${H}" aria-hidden="true">
<defs>
<linearGradient id="${id('body')}" x1="0" y1="0" x2="0.15" y2="1"><stop offset="0" stop-color="${c.body[0]}"/><stop offset="1" stop-color="${c.body[1]}"/></linearGradient>
<linearGradient id="${id('key')}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${c.pad[0]}"/><stop offset="1" stop-color="${c.pad[1]}"/></linearGradient>
<linearGradient id="${id('pad')}" x1="0.2" y1="0" x2="0.8" y2="1"><stop offset="0" stop-color="${c.pad[0]}"/><stop offset="1" stop-color="${c.pad[1]}"/></linearGradient>
<linearGradient id="${id('ring')}" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#f2f2f2"/><stop offset="0.35" stop-color="#9c9ea2"/><stop offset="0.7" stop-color="#e4e5e7"/><stop offset="1" stop-color="#6d6f73"/></linearGradient>
<linearGradient id="${id('glass')}" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#34363a"/><stop offset="0.46" stop-color="#1b1c1f"/><stop offset="0.5" stop-color="#0a0a0b"/><stop offset="1" stop-color="#050506"/></linearGradient>
<linearGradient id="${id('shine')}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff" stop-opacity="${glossy ? 0.3 : 0.12}"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></linearGradient>
<clipPath id="${id('scr')}"><rect x="${SCREEN.x}" y="${SCREEN.y}" width="${SCREEN.w}" height="${SCREEN.h}"/></clipPath>
<clipPath id="${id('face')}"><rect x="0" y="0" width="${W}" height="${H}" rx="3.2"/></clipPath>
</defs>
<rect x="0" y="0" width="${W}" height="${H}" rx="3.2" fill="url(#${id('body')})"/>
<g clip-path="url(#${id('face')})"><rect x="0" y="${GLASS.bottom}" width="${W}" height="${H - GLASS.bottom}" fill="url(#${id('shine')})" opacity="0.5"/></g>
<rect x="0.3" y="0.3" width="${W - 0.6}" height="${H - 0.6}" rx="2.9" fill="none" stroke="#fff" stroke-opacity="${glossy ? 0.3 : 0.14}" stroke-width="0.25"/>
<path d="M${GLASS.x} ${GLASS.bottom - 0.8} V3 A2.6 2.6 0 0 1 3.15 ${GLASS.y} H${W - 3.15} A2.6 2.6 0 0 1 ${W - GLASS.x} 3 V${GLASS.bottom - 0.8} Q${W - GLASS.x} ${GLASS.bottom} ${W - GLASS.x - 0.8} ${GLASS.bottom} H${GLASS.x + 0.8} Q${GLASS.x} ${GLASS.bottom} ${GLASS.x} ${GLASS.bottom - 0.8} Z" fill="url(#${id('glass')})"/>
<rect x="${SCREEN.x}" y="${SCREEN.y}" width="${SCREEN.w}" height="${SCREEN.h}" fill="#000"/>
${
	o.lit
		? `<g clip-path="url(#${id('scr')})" font-family="Selawik, 'Segoe UI', sans-serif">${classicScreen(SCREEN, o.uid, 'amber')}</g>`
		: `<path d="M${SCREEN.x} ${SCREEN.y + SCREEN.h} L${SCREEN.x + SCREEN.w} ${SCREEN.y} V${SCREEN.y + 6} L${SCREEN.x} ${SCREEN.y + 22} Z" fill="#fff" fill-opacity="0.04"/>`
}
<rect x="${SCREEN.x}" y="${SCREEN.y}" width="${SCREEN.w}" height="${SCREEN.h}" fill="none" stroke="#000" stroke-opacity="0.6" stroke-width="0.2"/>
${key(cx - KEY.dx, true)}
${key(cx + KEY.dx, false)}
${squircle(cx, PAD.cy, PAD.ring, PAD.ring - 0.3, 8.8)} fill="url(#${id('ring')})" stroke="#000" stroke-opacity="0.4" stroke-width="0.2"/>
${squircle(cx, PAD.cy, PAD.ring - 2.2, PAD.ring - 2.5, 7.8)} fill="#000" fill-opacity="0.35"/>
${squircle(cx, PAD.cy, PAD.inner, PAD.inner - 0.3, 7.0)} fill="url(#${id('pad')})"/>
${squircle(cx, PAD.cy - 0.6, PAD.inner - 1.2, PAD.inner - 2.2, 6.8)} fill="none" stroke="#fff" stroke-opacity="${glossy ? 0.14 : 0.07}" stroke-width="0.2"/>
<rect x="0.1" y="0.1" width="${W - 0.2}" height="${H - 0.2}" rx="3.1" fill="none" stroke="#000" stroke-opacity="0.45" stroke-width="0.25"/>
</svg>`;
}
