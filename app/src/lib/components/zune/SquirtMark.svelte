<script lang="ts">
	// Squirt's mark: the app icon in miniature. A rounded magenta-to-orange
	// tile with a white Zune HD (glass, notched chin). Simplified for small
	// sizes: the glass is solid and the notch slightly exaggerated.
	let { size = 18, class: className = '' }: { size?: number; class?: string } = $props();
	const uid = $props.id();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" class={className} aria-hidden="true">
	<defs>
		<linearGradient id="{uid}-g" x1="0" y1="0" x2="1" y2="1">
			<stop offset="0" stop-color="var(--zune-magenta)" />
			<stop offset="1" stop-color="var(--zune-orange)" />
		</linearGradient>
	</defs>
	<rect width="24" height="24" rx="5.5" fill="url(#{uid}-g)" />
	<!-- Device body (chin shows through as translucent white). -->
	<rect x="7.5" y="3.5" width="9" height="17" rx="1.3" fill="white" fill-opacity="0.45" />
	<!-- Glass with the home-button notch. -->
	<path d="M7.5 4.8 A1.3 1.3 0 0 1 8.8 3.5 H15.2 A1.3 1.3 0 0 1 16.5 4.8 V17 H13.4 L12.9 18.2 H11.1 L10.6 17 H7.5 Z" fill="white" />
</svg>
