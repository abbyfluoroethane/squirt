// Zune 30 (2006), front view. viewBox in mm of the real body.
// Measurements (Wikipedia: 111.8 x 61.0 x 14.7 mm; 3" 320x240 screen, shown
// portrait so 45.7 x 61 mm active area), cross-checked on a front-on photo
// (Wikimedia Commons "Zune-brown.jpg"):
// - dark glass panel with a thin silver trim, ~5.5 mm in from each side,
//   top at 2.9 mm, bottom at ~70 mm, wrapped around the screen;
// - 46 x 61 mm screen at y 6.3;
// - Zune Pad centred at y 87.5: silver ring 30 mm across, black disc 26 mm,
//   centre select button ~12.5 mm;
// - round Back (left) and Play/Pause (right) buttons ~8.2 mm across, 21.8 mm
//   either side of centre, level with the pad (y ~86.7);
// - body corners ~5 mm, a translucent coloured edge (the "doubleshot") ~0.9 mm.
import type { ArtOptions, ZuneModel } from './types.ts';
import { classicScreen } from './classic.ts';

const W = 61;
const H = 112;

export const model: ZuneModel = {
	id: 'zune30',
	name: 'Zune 30',
	year: 2006,
	aspect: W / H,
	mm: [61, 111.8, 14.7],
	finishes: [
		{
			id: 'black',
			name: 'black',
			source: 'https://en.wikipedia.org/wiki/Zune_30',
			note: 'Launch colour, Nov 2006. Doubleshot edge glows blue.'
		},
		{
			id: 'brown',
			name: 'brown',
			source: 'https://gizmodo.com/zunes-brown-color-sucks-analyst-says-218274',
			note: 'Launch colour, Nov 2006. Translucent painted shell, green edge.'
		},
		{
			id: 'white',
			name: 'white',
			source: 'https://en.wikipedia.org/wiki/Zune_30',
			note: 'Launch colour ("pearl white"), Nov 2006. Clear edge.'
		},
		{
			id: 'pink',
			name: 'pink',
			source: 'https://gizmodo.com/pink-zune-looks-girly-but-in-a-good-way-252722',
			note: 'Limited run of 100,000, spring 2007; soft baby pink. Run size and date per Wikipedia and Engadget, sources differ slightly on dates.'
		},
		{
			id: 'red',
			name: 'red',
			source: 'https://techcrunch.com/2007/06/04/hey-its-a-zune-and-its-red/',
			note: 'Watermelon red, Target and Amazon.com exclusive, June 2007 (100,000 per Wikipedia).'
		},
		{
			id: 'magenta',
			name: 'magenta',
			source: 'https://en.wikipedia.org/wiki/Zune_30',
			note: 'About 100 made, randomly inserted into launch boxes; often called "pink" at the time. Shade approximated, no photo checked.'
		},
		{
			id: 'orange',
			name: 'orange',
			source: 'https://en.wikipedia.org/wiki/Zune_30',
			note: 'About 100 made, randomly inserted into launch boxes. Shade approximated, no photo checked.'
		}
	]
};

// face: front plastic, edge: translucent rim, ink: glyph colour on the buttons.
const COLORS: Record<string, { face: string; faceLo: string; edge: string; edgeLo: string; ink: string }> = {
	black: { face: '#2a2b2f', faceLo: '#18191b', edge: '#3a5fb0', edgeLo: '#1a2f66', ink: '#d8d8dc' },
	brown: { face: '#7d5a4d', faceLo: '#5e4037', edge: '#86b83c', edgeLo: '#4f7a2a', ink: '#e8d9d2' },
	white: { face: '#f1efea', faceLo: '#d9d6cf', edge: '#e4e6e8', edgeLo: '#b9bcc0', ink: '#55575b' },
	pink: { face: '#f4bccd', faceLo: '#e49fb5', edge: '#f7cfdb', edgeLo: '#d78ba3', ink: '#6e3446' },
	red: { face: '#cf2f3d', faceLo: '#a51f2e', edge: '#e0646d', edgeLo: '#8c1826', ink: '#f6dcdd' },
	magenta: { face: '#cc2a86', faceLo: '#a01a66', edge: '#e064ac', edgeLo: '#7e1450', ink: '#f6d9ea' },
	orange: { face: '#f47a24', faceLo: '#d05c10', edge: '#f9a463', edgeLo: '#a8460a', ink: '#fbe7d6' }
};

export function render(o: ArtOptions): string {
	const c = COLORS[o.finish] ?? COLORS[model.finishes[0].id];
	const u = o.uid;
	const cx = W / 2;
	const PAD_Y = 87.5;
	const BTN_Y = 86.7;
	const BTN_DX = 21.8;
	const screen = { x: 7.5, y: 6.3, w: 46, h: 61 };
	const panel = { x: 5.4, y: 2.9, w: 50.2, h: 67.2 };

	const menu = o.lit ? classicScreen(screen, u, 'violet') : '';

	// Back arrow and play/pause glyphs.
	const back = `<path d="M${-1.7} 0 L${0.4} ${-1.9} V${-0.6} H${1.9} V${0.6} H${0.4} V${1.9} Z" fill="${c.ink}"/>`;
	const play = `<path d="M${-2} ${-1.8} L${0} 0 L${-2} ${1.8} Z M${0.6} ${-1.8} H${1.1} V${1.8} H${0.6} Z M${1.6} ${-1.8} H${2.1} V${1.8} H${1.6} Z" fill="${c.ink}"/>`;

	const button = (x: number, glyph: string) => `
<g transform="translate(${x} ${BTN_Y})">
<circle r="4.3" fill="#000" fill-opacity="0.22"/>
<circle r="4.1" cy="-0.1" fill="url(#${u}-btn)"/>
<circle r="4.1" cy="-0.1" fill="none" stroke="#fff" stroke-opacity="0.22" stroke-width="0.2"/>
<g>${glyph}</g></g>`;

	return `<svg xmlns="http://www.w3.org/2000/svg" width="${o.height * model.aspect}" height="${o.height}" viewBox="0 0 ${W} ${H}" role="img" aria-label="Zune 30, ${o.finish}">
<defs>
<linearGradient id="${u}-edge" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${c.edge}"/><stop offset="1" stop-color="${c.edgeLo}"/></linearGradient>
<linearGradient id="${u}-face" x1="0" y1="0" x2="0.15" y2="1"><stop offset="0" stop-color="${c.face}"/><stop offset="1" stop-color="${c.faceLo}"/></linearGradient>
<linearGradient id="${u}-btn" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${c.face}"/><stop offset="1" stop-color="${c.faceLo}"/></linearGradient>
<linearGradient id="${u}-gloss" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#fff" stop-opacity="0.22"/><stop offset="0.4" stop-color="#fff" stop-opacity="0.04"/><stop offset="0.5" stop-color="#fff" stop-opacity="0"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></linearGradient>
<linearGradient id="${u}-glass" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#34363b"/><stop offset="0.5" stop-color="#1d1e21"/><stop offset="1" stop-color="#0c0d0e"/></linearGradient>
<linearGradient id="${u}-ring" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#f4f5f6"/><stop offset="0.35" stop-color="#a9acb0"/><stop offset="0.6" stop-color="#e3e4e6"/><stop offset="1" stop-color="#7d8085"/></linearGradient>
<linearGradient id="${u}-trim" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#b9bcc0"/><stop offset="1" stop-color="#5d6065"/></linearGradient>
<radialGradient id="${u}-disc" cx="0.35" cy="0.28" r="0.9"><stop offset="0" stop-color="#2d2e32"/><stop offset="0.6" stop-color="#101113"/><stop offset="1" stop-color="#060607"/></radialGradient>
<linearGradient id="${u}-lcd" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#fff" stop-opacity="0.09"/><stop offset="0.45" stop-color="#fff" stop-opacity="0.02"/><stop offset="0.5" stop-color="#fff" stop-opacity="0"/></linearGradient>
<clipPath id="${u}-scr"><rect x="${screen.x}" y="${screen.y}" width="${screen.w}" height="${screen.h}" rx="0.6"/></clipPath>
</defs>

<!-- Body: translucent edge colour. -->
<rect width="${W}" height="${H}" rx="5.2" fill="url(#${u}-edge)"/>
<!-- Front face. -->
<rect x="0.9" y="0.9" width="${W - 1.8}" height="${H - 1.8}" rx="4.4" fill="url(#${u}-face)"/>
<rect x="0.9" y="0.9" width="${W - 1.8}" height="${H - 1.8}" rx="4.4" fill="url(#${u}-gloss)"/>
<rect x="1.1" y="1.1" width="${W - 2.2}" height="${H - 2.2}" rx="4.2" fill="none" stroke="#fff" stroke-opacity="0.25" stroke-width="0.25"/>

<!-- Glass panel, bulging slightly down around the pad. -->
<path d="M${panel.x + 2.2} ${panel.y} H${panel.x + panel.w - 2.2} Q${panel.x + panel.w} ${panel.y} ${panel.x + panel.w} ${panel.y + 2.2} V${panel.y + panel.h - 2.4} Q${panel.x + panel.w} ${panel.y + panel.h} ${panel.x + panel.w - 2.4} ${panel.y + panel.h} H${cx + 9} C${cx + 6.5} ${panel.y + panel.h} ${cx + 6} ${panel.y + panel.h + 1.2} ${cx + 3} ${panel.y + panel.h + 1.2} H${cx - 3} C${cx - 6} ${panel.y + panel.h + 1.2} ${cx - 6.5} ${panel.y + panel.h} ${cx - 9} ${panel.y + panel.h} H${panel.x + 2.4} Q${panel.x} ${panel.y + panel.h} ${panel.x} ${panel.y + panel.h - 2.4} V${panel.y + 2.2} Q${panel.x} ${panel.y} ${panel.x + 2.2} ${panel.y} Z" fill="url(#${u}-glass)" stroke="url(#${u}-trim)" stroke-width="0.35"/>
<rect x="${screen.x - 0.7}" y="${screen.y - 0.7}" width="${screen.w + 1.4}" height="${screen.h + 1.4}" rx="1" fill="none" stroke="#8d9094" stroke-opacity="0.7" stroke-width="0.35"/>

<!-- Screen. -->
<rect x="${screen.x}" y="${screen.y}" width="${screen.w}" height="${screen.h}" rx="0.6" fill="${o.lit ? '#000' : '#050607'}"/>
<g clip-path="url(#${u}-scr)" font-family="Selawik, 'Segoe UI', sans-serif">${menu}
<rect x="${screen.x}" y="${screen.y}" width="${screen.w}" height="${screen.h}" fill="url(#${u}-lcd)"/></g>

<!-- Zune Pad: silver ring, black disc, select button. -->
<circle cx="${cx}" cy="${PAD_Y + 0.3}" r="15.2" fill="#000" fill-opacity="0.25"/>
<circle cx="${cx}" cy="${PAD_Y}" r="15" fill="url(#${u}-ring)"/>
<circle cx="${cx}" cy="${PAD_Y}" r="13.1" fill="#0a0a0b"/>
<circle cx="${cx}" cy="${PAD_Y}" r="12.8" fill="url(#${u}-disc)"/>
<circle cx="${cx}" cy="${PAD_Y}" r="6.3" fill="#0b0b0c" stroke="#52555a" stroke-width="0.35"/>
<circle cx="${cx}" cy="${PAD_Y}" r="5.7" fill="url(#${u}-disc)"/>
<path d="M${cx - 10.5} ${PAD_Y - 5} A11.6 11.6 0 0 1 ${cx + 2} ${PAD_Y - 11.4}" fill="none" stroke="#fff" stroke-opacity="0.18" stroke-width="0.5" stroke-linecap="round"/>

<!-- Back and Play/Pause. -->
${button(cx - BTN_DX, back)}
${button(cx + BTN_DX, play)}

<!-- Outer edge. -->
<rect x="0.1" y="0.1" width="${W - 0.2}" height="${H - 0.2}" rx="5.1" fill="none" stroke="#000" stroke-opacity="0.4" stroke-width="0.25"/>
</svg>`;
}
