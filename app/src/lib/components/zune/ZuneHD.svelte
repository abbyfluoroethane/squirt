<script lang="ts">
	// Line drawing of a Zune HD (52.7 x 102 mm): one outline with tight
	// corners, glass edge to edge over the top ~83% of the face, and a metal
	// chin across the full width below it. The trapezoid notch where the glass
	// dips into the chin (the home button) is what makes it read as a Zune HD.
	let {
		size = 16,
		weight = 1.25,
		lit = false,
		class: className = ''
	}: { size?: number; weight?: number; lit?: boolean; class?: string } = $props();

	const uid = $props.id();
	// Small icons fill the glass solid so the notch survives rasterising.
	const detailed = $derived(size >= 24);

	// Body outline, and the glass: the body's top part down to the notched line.
	const BODY = { x: 0.75, y: 0.75, w: 51.5, h: 100.5, r: 3.2 };
	const GLASS =
		'M0.75 84.5 V3.95 A3.2 3.2 0 0 1 3.95 0.75 H49.05 A3.2 3.2 0 0 1 52.25 3.95 V84.5 H35.6 L33.4 88.6 H19.6 L17.4 84.5 Z';
</script>

<svg
	width={size * (53 / 102)}
	height={size}
	viewBox="0 0 53 102"
	fill="none"
	stroke="currentColor"
	stroke-linejoin="round"
	class={className}
	aria-hidden="true"
>
	{#if lit}
		<defs>
			<linearGradient id="{uid}-g" x1="0" y1="0" x2="1" y2="1">
				<stop offset="0" stop-color="var(--zune-magenta)" />
				<stop offset="1" stop-color="var(--zune-orange)" />
			</linearGradient>
		</defs>
	{/if}

	<!-- Metal chin: a faint fill over the whole body; the glass covers the top. -->
	<rect x={BODY.x} y={BODY.y} width={BODY.w} height={BODY.h} rx={BODY.r} stroke="none"
		fill="currentColor" fill-opacity={detailed ? 0.22 : 0.3} />

	{#if detailed}
		<path d={GLASS} stroke-width={weight * 0.8} vector-effect="non-scaling-stroke" fill="var(--background)" />
		<!-- Screen, 16:9. -->
		<rect
			x="6"
			y="7"
			width="41"
			height="72.9"
			stroke-width={weight * 0.8}
			vector-effect="non-scaling-stroke"
			stroke-opacity={lit ? 0 : 0.45}
			fill={lit ? `url(#${uid}-g)` : 'none'}
			fill-opacity={lit ? 0.9 : 0}
		/>
	{:else}
		<path d={GLASS} stroke="none" fill="currentColor" />
	{/if}

	<!-- Outline on top, so it stays crisp over the fills. -->
	<rect x={BODY.x} y={BODY.y} width={BODY.w} height={BODY.h} rx={BODY.r} stroke-width={weight} vector-effect="non-scaling-stroke" />
</svg>
