<script lang="ts">
	// A large Zune HD illustration for the device page, measured from photos
	// of the real thing (52.7 x 102 mm, drawn in a 53 x 102 box):
	// - a thin champagne metal rim all the way round;
	// - dark slate glass inset in it, ending at ~78% of the height;
	// - a metal chin below, with the trapezoid home-button notch (~40% of the
	//   width across the top, angled sides) where the glass dips into it;
	// - the screen inset in the glass, ending at ~69%, leaving a band of glass
	//   beneath it. When lit it shows the time and date, as the lock screen does.
	import { onMount } from 'svelte';

	let { height = 420, lit = true }: { height?: number; lit?: boolean } = $props();
	const uid = $props.id();

	let now = $state(new Date());
	onMount(() => {
		const t = setInterval(() => (now = new Date()), 10_000);
		return () => clearInterval(t);
	});
	const time = $derived(now.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' }).replace(/\s?[AP]M$/i, ''));
	const day = $derived(now.toLocaleDateString(undefined, { weekday: 'long' }).toLowerCase());
	const date = $derived(now.toLocaleDateString(undefined, { month: 'long', day: 'numeric' }).toLowerCase());

	// Geometry (units = mm).
	const RIM = 1.7; // metal rim width
	const GLASS_BOTTOM = 79; // where the glass meets the chin
	const NOTCH = { topHalf: 10.6, bottomHalf: 8.2, depth: 3.4 };
	const SCREEN = { x: 5.6, y: 7.2, w: 41.8, h: 63.4 };
	const cx = 26.5;
	const r = 2.4; // glass corner radius
	const glassPath = [
		`M${RIM} ${GLASS_BOTTOM}`,
		`V${RIM + r}`,
		`A${r} ${r} 0 0 1 ${RIM + r} ${RIM}`,
		`H${53 - RIM - r}`,
		`A${r} ${r} 0 0 1 ${53 - RIM} ${RIM + r}`,
		`V${GLASS_BOTTOM}`,
		`H${cx + NOTCH.topHalf}`,
		`L${cx + NOTCH.bottomHalf} ${GLASS_BOTTOM + NOTCH.depth}`,
		`H${cx - NOTCH.bottomHalf}`,
		`L${cx - NOTCH.topHalf} ${GLASS_BOTTOM}`,
		'Z'
	].join(' ');
</script>

<svg width={height * (53 / 102)} height={height} viewBox="0 0 53 102" aria-hidden="true" class="drop-shadow-2xl">
	<defs>
		<linearGradient id="{uid}-screen" x1="0" y1="0" x2="1" y2="1">
			<stop offset="0" stop-color="var(--zune-magenta)" />
			<stop offset="1" stop-color="var(--zune-orange)" />
		</linearGradient>
		<!-- Brushed champagne metal, lit from above. -->
		<linearGradient id="{uid}-metal" x1="0" y1="0" x2="0.35" y2="1">
			<stop offset="0" stop-color="#ddd6c9" />
			<stop offset="0.6" stop-color="#cbc3b4" />
			<stop offset="1" stop-color="#b3ab9c" />
		</linearGradient>
		<linearGradient id="{uid}-glass" x1="0" y1="0" x2="0" y2="1">
			<stop offset="0" stop-color="#2a2f36" />
			<stop offset="1" stop-color="#1d2127" />
		</linearGradient>
		<linearGradient id="{uid}-sheen" x1="0" y1="0" x2="1" y2="0.7">
			<stop offset="0" stop-color="#fff" stop-opacity="0.13" />
			<stop offset="0.45" stop-color="#fff" stop-opacity="0" />
		</linearGradient>
	</defs>

	<!-- Body: metal rim and chin. -->
	<rect x="0" y="0" width="53" height="102" rx="4.2" fill="url(#{uid}-metal)" />
	<!-- Chamfer highlight just inside the outer edge. -->
	<rect x="0.45" y="0.45" width="52.1" height="101.1" rx="3.8" fill="none" stroke="#fff" stroke-opacity="0.35" stroke-width="0.35" />

	<!-- Glass with the home-button notch. -->
	<path d={glassPath} fill="url(#{uid}-glass)" />

	<!-- Screen. -->
	<rect x={SCREEN.x} y={SCREEN.y} width={SCREEN.w} height={SCREEN.h} fill={lit ? `url(#${uid}-screen)` : '#121417'} />
	{#if lit}
		<!-- Lock screen: big light time, day and date beneath, bottom-left. -->
		<g fill="#fff" font-family="Selawik, 'Segoe UI', sans-serif">
			<text x={SCREEN.x + 2.6} y={SCREEN.y + SCREEN.h - 10.6} font-size="10.5" font-weight="300">{time}</text>
			<text x={SCREEN.x + 2.7} y={SCREEN.y + SCREEN.h - 6.2} font-size="3" font-weight="350" opacity="0.92">{day}</text>
			<text x={SCREEN.x + 2.7} y={SCREEN.y + SCREEN.h - 2.6} font-size="3" font-weight="350" opacity="0.92">{date}</text>
		</g>
	{/if}

	<!-- Home button: a glint along the notch's lower edge. -->
	<line x1={cx - NOTCH.bottomHalf + 0.6} y1={GLASS_BOTTOM + NOTCH.depth - 0.35} x2={cx + NOTCH.bottomHalf - 0.6} y2={GLASS_BOTTOM + NOTCH.depth - 0.35} stroke="#fff" stroke-opacity="0.18" stroke-width="0.3" />

	<!-- Glass sheen. -->
	<path d={glassPath} fill="url(#{uid}-sheen)" />
	<!-- Outer edge. -->
	<rect x="0.15" y="0.15" width="52.7" height="101.7" rx="4.1" fill="none" stroke="#000" stroke-opacity="0.35" stroke-width="0.3" />
</svg>
