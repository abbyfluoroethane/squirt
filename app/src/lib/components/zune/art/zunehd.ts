import type { ArtOptions, ZuneModel } from './types.ts';

// Zune HD (2009), ported from ZuneHDArt.svelte. Measured from the straight-on
// product photo and real dimensions (52.7 x 102 mm body, 3.3" 16:9 screen),
// drawn in a 53 x 102 mm box:
// - glass runs edge to edge, leaving a ~0.9 mm metal hairline at sides and top,
//   and ends at 84% of the height;
// - the metal chin fills the bottom 16%, full width;
// - the home-button notch dips ~2.9 mm into the chin, ~37% of the width across
//   the top, ~28% at the bottom, with rounded inner corners;
// - the 41 x 72.9 mm screen sits high on the face, with glass below it.
// The front glass is black on every model; colorways differ in the metal.

const SRC_ANN = 'https://gizmodo.com/a-tiny-look-at-the-zune-hd-color-options-5336671';
const SRC_64 = 'https://gizmodo.com/64gb-zune-hd-will-cost-350-other-models-slashed-by-2-5506501';
const SRC_PR = 'https://news.microsoft.com/source/2009/08/13/media-alert-zune-hd-available-for-pre-order-today/';

export const model: ZuneModel = {
	id: 'zunehd',
	name: 'Zune HD',
	year: 2009,
	aspect: 53 / 102,
	mm: [52.7, 102, 8.9],
	finishes: [
		{
			id: 'platinum',
			name: 'platinum',
			source: SRC_PR,
			note: 'Retail from 2009-09-15 (32 GB at launch pre-order; also 16 GB and 64 GB). Microsoft calls it Platinum.'
		},
		{
			id: 'slate',
			name: 'black',
			source: SRC_PR,
			note: 'Microsoft named this finish Black (16 GB at launch pre-order; also 32 and 64 GB). "slate" is an app-internal id for the graphite/black-metal model, not a Microsoft name; no Zune HD finish was officially called Slate.'
		},
		{
			id: 'red',
			name: 'red',
			source: SRC_ANN,
			note: 'Zune Originals only (online), from 2009-09-15. Press release: five colors per capacity (16/32 GB); the fan site names them red, green, blue, platinum, black. Metal tint approximated.'
		},
		{
			id: 'green',
			name: 'green',
			source: SRC_ANN,
			note: 'Zune Originals only, from 2009-09-15, 16/32 GB. Metal tint approximated.'
		},
		{
			id: 'blue',
			name: 'blue',
			source: SRC_ANN,
			note: 'Zune Originals only, from 2009-09-15, 16/32 GB. Metal tint approximated.'
		},
		{
			id: 'purple',
			name: 'purple',
			source: SRC_64,
			note: 'Listed with the 64 GB launch (April 2010) as a Zune Originals choice; unclear whether 16/32 GB could also be ordered in it. Metal tint approximated.'
		},
		{
			id: 'magenta',
			name: 'magenta',
			source: SRC_64,
			note: 'Listed with the 64 GB launch (April 2010) as a Zune Originals choice; unclear whether 16/32 GB could also be ordered in it. Metal tint approximated.'
		}
	]
};

// Metal for the chin and edge, lit from above.
const METAL: Record<string, { top: string; bottom: string; highlight: number }> = {
	platinum: { top: '#ddd5c6', bottom: '#b9b0a0', highlight: 0.4 },
	slate: { top: '#47494d', bottom: '#222326', highlight: 0.18 },
	red: { top: '#c5353f', bottom: '#85181f', highlight: 0.3 },
	green: { top: '#9ccb3c', bottom: '#5f8f1c', highlight: 0.3 },
	blue: { top: '#3f94d9', bottom: '#1f5a99', highlight: 0.3 },
	purple: { top: '#8c52bd', bottom: '#552a80', highlight: 0.3 },
	magenta: { top: '#e2358f', bottom: '#a3145c', highlight: 0.3 }
};

const esc = (s: string) => s.replace(/[&<>]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' })[c]!);

export function render(o: ArtOptions): string {
	const metal = METAL[o.finish] ?? METAL.platinum;
	const u = o.uid;
	const h = o.now.getHours() % 12 || 12;
	const time = `${h}:${String(o.now.getMinutes()).padStart(2, '0')}`;
	const day = o.now.toLocaleDateString('en-US', { weekday: 'long' }).toLowerCase();
	const date = o.now.toLocaleDateString('en-US', { month: 'long', day: 'numeric' }).toLowerCase();

	const EDGE = 0.9;
	const R_BODY = 2.4;
	const R_GLASS = 1.6;
	const GLASS_BOTTOM = 85.8;
	const N = { top: 9.8, bottom: 7.4, depth: 2.9, r: 0.9 }; // notch half-widths
	const SCREEN = { x: 6, y: 6.3, w: 41, h: 72.9 };
	const cx = 26.5;
	const L = EDGE;
	const R = 53 - EDGE;
	const nb = GLASS_BOTTOM + N.depth;
	const glassPath = [
		`M${L} ${GLASS_BOTTOM}`,
		`V${EDGE + R_GLASS}`,
		`A${R_GLASS} ${R_GLASS} 0 0 1 ${L + R_GLASS} ${EDGE}`,
		`H${R - R_GLASS}`,
		`A${R_GLASS} ${R_GLASS} 0 0 1 ${R} ${EDGE + R_GLASS}`,
		`V${GLASS_BOTTOM}`,
		`H${cx + N.top}`,
		`L${cx + N.bottom + N.r * 0.7} ${nb - N.r * 0.7}`,
		`Q${cx + N.bottom} ${nb} ${cx + N.bottom - N.r} ${nb}`,
		`H${cx - N.bottom + N.r}`,
		`Q${cx - N.bottom} ${nb} ${cx - N.bottom - N.r * 0.7} ${nb - N.r * 0.7}`,
		`L${cx - N.top} ${GLASS_BOTTOM}`,
		'Z'
	].join(' ');

	// Home button: glossy black key filling 90% of the notch height, with the
	// same angled sides.
	const B = { inset: 0.75, top: GLASS_BOTTOM + N.depth * 0.05, bottom: nb - N.depth * 0.05 };
	const slope = (N.top - N.bottom) / N.depth;
	const halfAt = (y: number) => N.top - slope * (y - GLASS_BOTTOM) - B.inset;
	const buttonPath = [
		`M${cx - halfAt(GLASS_BOTTOM)} ${B.top}`,
		`H${cx + halfAt(GLASS_BOTTOM)}`,
		`L${cx + halfAt(B.bottom)} ${B.bottom - 0.4}`,
		`Q${cx + halfAt(B.bottom) - 0.15} ${B.bottom} ${cx + halfAt(B.bottom) - 0.6} ${B.bottom}`,
		`H${cx - halfAt(B.bottom) + 0.6}`,
		`Q${cx - halfAt(B.bottom) + 0.15} ${B.bottom} ${cx - halfAt(B.bottom)} ${B.bottom - 0.4}`,
		'Z'
	].join(' ');

	const screen = o.lit
		? `<rect x="${SCREEN.x}" y="${SCREEN.y}" width="${SCREEN.w}" height="${SCREEN.h}" fill="url(#${u}-screen)"/>
<g fill="#fff" font-family="Selawik, 'Segoe UI', sans-serif">
<text x="${SCREEN.x + 2.6}" y="${SCREEN.y + SCREEN.h - 10.6}" font-size="10.5" font-weight="300">${esc(time)}</text>
<text x="${SCREEN.x + 2.7}" y="${SCREEN.y + SCREEN.h - 6.2}" font-size="3" font-weight="350" opacity="0.92">${esc(day)}</text>
<text x="${SCREEN.x + 2.7}" y="${SCREEN.y + SCREEN.h - 2.6}" font-size="3" font-weight="350" opacity="0.92">${esc(date)}</text>
</g>`
		: '';

	return `<svg xmlns="http://www.w3.org/2000/svg" width="${o.height * model.aspect}" height="${o.height}" viewBox="0 0 53 102" aria-hidden="true">
<defs>
<linearGradient id="${u}-screen" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="var(--zune-magenta, #e3008c)"/><stop offset="1" stop-color="var(--zune-orange, #ff6a13)"/></linearGradient>
<linearGradient id="${u}-metal" x1="0" y1="0" x2="0.2" y2="1"><stop offset="0" stop-color="${metal.top}"/><stop offset="1" stop-color="${metal.bottom}"/></linearGradient>
<linearGradient id="${u}-button" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2b2d31"/><stop offset="0.45" stop-color="#121315"/><stop offset="1" stop-color="#050506"/></linearGradient>
<linearGradient id="${u}-glass" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#3a3d42"/><stop offset="0.48" stop-color="#26282c"/><stop offset="0.52" stop-color="#0e0f11"/><stop offset="1" stop-color="#08090a"/></linearGradient>
</defs>
<rect x="0" y="0" width="53" height="102" rx="${R_BODY}" fill="url(#${u}-metal)"/>
<rect x="0.3" y="0.3" width="52.4" height="101.4" rx="${R_BODY - 0.3}" fill="none" stroke="#fff" stroke-opacity="${metal.highlight}" stroke-width="0.3"/>
<path d="${glassPath}" fill="url(#${u}-glass)"/>
${screen}
<path d="${buttonPath}" fill="url(#${u}-button)" stroke="#000" stroke-opacity="0.6" stroke-width="0.2"/>
<line x1="${cx - halfAt(GLASS_BOTTOM) + 0.5}" y1="${B.top + 0.25}" x2="${cx + halfAt(GLASS_BOTTOM) - 0.5}" y2="${B.top + 0.25}" stroke="#fff" stroke-opacity="0.22" stroke-width="0.25"/>
<circle cx="${cx}" cy="${B.top + 0.8}" r="0.2" fill="#fff" fill-opacity="0.35"/>
<rect x="0.1" y="0.1" width="52.8" height="101.8" rx="${R_BODY - 0.1}" fill="none" stroke="#000" stroke-opacity="0.4" stroke-width="0.25"/>
</svg>`;
}
