<script lang="ts">
	// Squirt's mark: a black drop (a "squirt") with a gradient core, like a
	// song glowing inside it. The edge is a fixed 1.25px on screen (not scaled
	// with the icon) so the black drop stays outlined at title-bar sizes.
	let { size = 20, class: className = '' }: { size?: number; class?: string } = $props();
	const uid = $props.id();
</script>

<svg width={size * (68 / 94)} height={size} viewBox="16 4 68 94" class={className} aria-hidden="true">
	<defs>
		<linearGradient id="{uid}-g" x1="0" y1="0" x2="1" y2="1">
			<stop offset="0" stop-color="var(--zune-magenta)" />
			<stop offset="1" stop-color="var(--zune-orange)" />
		</linearGradient>
	</defs>
	<path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 82 64 C82 44 50 6 50 6 Z" fill="#0d0d0d" stroke="currentColor" stroke-opacity="0.7" stroke-width="1.25" vector-effect="non-scaling-stroke" />
	<path d="M50 30 C50 30 32 52 32 64 A18 18 0 0 0 68 64 C68 52 50 30 50 30 Z" fill="url(#{uid}-g)" />
</svg>
