<script lang="ts">
	// Line drawing of a Zune HD, from its real proportions: 52.7 x 102 mm body,
	// a 3.3" 16:9 screen (41 x 73 mm) set high on the face, and the small
	// home-button bar centred below it. Strokes stay the same width at any size.
	let {
		size = 16,
		weight = 1.25,
		lit = false,
		class: className = ''
	}: { size?: number; weight?: number; lit?: boolean; class?: string } = $props();

	const uid = $props.id();
	// Below ~20px the screen outline turns to mush, so drop it and keep the
	// silhouette and home button, which are what make it read as a Zune HD.
	const detailed = $derived(size >= 20);
</script>

<svg
	width={size * (53 / 102)}
	height={size}
	viewBox="0 0 53 102"
	fill="none"
	stroke="currentColor"
	class={className}
	aria-hidden="true"
>
	{#if lit}
		<defs>
			<linearGradient id="{uid}-g" x1="0" y1="0" x2="1" y2="1">
				<stop offset="0" stop-color="var(--zune-magenta)" stop-opacity="0.85" />
				<stop offset="1" stop-color="var(--zune-orange)" stop-opacity="0.85" />
			</linearGradient>
		</defs>
	{/if}
	<rect x="1" y="1" width="51" height="100" rx="6.5" stroke-width={weight} vector-effect="non-scaling-stroke" />
	{#if detailed || lit}
		<rect
			x="6"
			y="9"
			width="41"
			height="73"
			rx="0.5"
			stroke-width={weight}
			vector-effect="non-scaling-stroke"
			fill={lit ? `url(#${uid}-g)` : 'none'}
			stroke-opacity={lit ? 0 : 0.7}
		/>
	{/if}
	<!-- Home button. Exaggerated at small sizes so it survives rasterising. -->
	{#if detailed}
		<rect x="21" y="89" width="11" height="2.5" rx="1.25" stroke="none" fill="currentColor" />
	{:else}
		<rect x="15" y="82" width="23" height="8" rx="2" stroke="none" fill="currentColor" />
	{/if}
</svg>
