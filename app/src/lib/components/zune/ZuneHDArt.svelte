<script lang="ts">
	// A large Zune HD illustration for the device page, measured from the
	// straight-on product photo and the real dimensions (52.7 x 102 mm body,
	// 3.3" 16:9 screen), drawn in a 53 x 102 box:
	// - glass runs edge to edge, leaving only a ~1 mm hairline of metal at the
	//   sides and top, and ends at 84% of the height;
	// - the champagne metal chin fills the bottom 16%, full width;
	// - the home-button notch dips ~3 mm into the chin: ~37% of the width
	//   across the top, ~28% at the bottom, with rounded inner corners;
	// - the 41 x 73 mm screen sits high on the face, with glass below it.
	// When lit the screen shows the time and date, like the lock screen.
	import { onMount } from 'svelte';

	let {
		height = 420,
		lit = true,
		finish = 'platinum'
	}: { height?: number; lit?: boolean; finish?: 'slate' | 'platinum' } = $props();

	// Metal for the chin and edge: champagne for platinum, graphite for slate.
	const METAL = {
		platinum: { top: '#ddd5c6', bottom: '#b9b0a0', highlight: 0.4 },
		slate: { top: '#47494d', bottom: '#222326', highlight: 0.18 }
	} as const;
	const metal = $derived(METAL[finish]);
	const uid = $props.id();

	let now = $state(new Date());
	onMount(() => {
		const t = setInterval(() => (now = new Date()), 10_000);
		return () => clearInterval(t);
	});
	const time = $derived(now.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' }).replace(/\s?[AP]M$/i, ''));
	const day = $derived(now.toLocaleDateString(undefined, { weekday: 'long' }).toLowerCase());
	const date = $derived(now.toLocaleDateString(undefined, { month: 'long', day: 'numeric' }).toLowerCase());

	// Geometry, in mm. (Champagne metal is the platinum model; the slate model
	// has graphite metal.)
	const EDGE = 0.9; // metal hairline at the sides and top
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
		// Angled side down into the chin, rounding into the notch floor.
		`L${cx + N.bottom + N.r * 0.7} ${nb - N.r * 0.7}`,
		`Q${cx + N.bottom} ${nb} ${cx + N.bottom - N.r} ${nb}`,
		`H${cx - N.bottom + N.r}`,
		`Q${cx - N.bottom} ${nb} ${cx - N.bottom - N.r * 0.7} ${nb - N.r * 0.7}`,
		`L${cx - N.top} ${GLASS_BOTTOM}`,
		'Z'
	].join(' ');

	// Home button: a glossy black key filling the notch, inset slightly from
	// its edges, with the same angled sides.
	// The key fills 90% of the notch height, centred in it.
	const B = { inset: 0.75, top: GLASS_BOTTOM + N.depth * 0.05, bottom: nb - N.depth * 0.05 };
	const slope = (N.top - N.bottom) / N.depth; // half-width lost per mm down
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
</script>

<svg width={height * (53 / 102)} height={height} viewBox="0 0 53 102" aria-hidden="true" class="drop-shadow-2xl">
	<defs>
		<linearGradient id="{uid}-screen" x1="0" y1="0" x2="1" y2="1">
			<stop offset="0" stop-color="var(--zune-magenta)" />
			<stop offset="1" stop-color="var(--zune-orange)" />
		</linearGradient>
		<!-- Champagne metal, lit from above. -->
		<linearGradient id="{uid}-metal" x1="0" y1="0" x2="0.2" y2="1">
			<stop offset="0" stop-color={metal.top} />
			<stop offset="1" stop-color={metal.bottom} />
		</linearGradient>
		<!-- The diagonal reflection across the glass in the product shots:
		     lighter top-left, near black bottom-right. -->
		<linearGradient id="{uid}-button" x1="0" y1="0" x2="0" y2="1">
			<stop offset="0" stop-color="#2b2d31" />
			<stop offset="0.45" stop-color="#121315" />
			<stop offset="1" stop-color="#050506" />
		</linearGradient>
		<linearGradient id="{uid}-glass" x1="0" y1="0" x2="1" y2="1">
			<stop offset="0" stop-color="#3a3d42" />
			<stop offset="0.48" stop-color="#26282c" />
			<stop offset="0.52" stop-color="#0e0f11" />
			<stop offset="1" stop-color="#08090a" />
		</linearGradient>
	</defs>

	<!-- Body: metal, visible as the hairline edge and the chin. -->
	<rect x="0" y="0" width="53" height="102" rx={R_BODY} fill="url(#{uid}-metal)" />
	<rect x="0.3" y="0.3" width="52.4" height="101.4" rx={R_BODY - 0.3} fill="none" stroke="#fff" stroke-opacity={metal.highlight} stroke-width="0.3" />

	<!-- Glass with the home-button notch. -->
	<path d={glassPath} fill="url(#{uid}-glass)" />

	<!-- Screen: dark when off (it disappears into the glass), lit otherwise. -->
	{#if lit}
		<rect x={SCREEN.x} y={SCREEN.y} width={SCREEN.w} height={SCREEN.h} fill="url(#{uid}-screen)" />
		<g fill="#fff" font-family="Selawik, 'Segoe UI', sans-serif">
			<text x={SCREEN.x + 2.6} y={SCREEN.y + SCREEN.h - 10.6} font-size="10.5" font-weight="300">{time}</text>
			<text x={SCREEN.x + 2.7} y={SCREEN.y + SCREEN.h - 6.2} font-size="3" font-weight="350" opacity="0.92">{day}</text>
			<text x={SCREEN.x + 2.7} y={SCREEN.y + SCREEN.h - 2.6} font-size="3" font-weight="350" opacity="0.92">{date}</text>
		</g>
	{/if}

	<!-- Home button in the notch: glossy key, highlight along its top edge,
	     and the small dot at its centre. -->
	<path d={buttonPath} fill="url(#{uid}-button)" stroke="#000" stroke-opacity="0.6" stroke-width="0.2" />
	<line x1={cx - halfAt(GLASS_BOTTOM) + 0.5} y1={B.top + 0.25} x2={cx + halfAt(GLASS_BOTTOM) - 0.5} y2={B.top + 0.25} stroke="#fff" stroke-opacity="0.22" stroke-width="0.25" />
	<circle cx={cx} cy={B.top + 0.8} r="0.2" fill="#fff" fill-opacity="0.35" />

	<!-- Outer edge. -->
	<rect x="0.1" y="0.1" width="52.8" height="101.8" rx={R_BODY - 0.1} fill="none" stroke="#000" stroke-opacity="0.4" stroke-width="0.25" />
</svg>
