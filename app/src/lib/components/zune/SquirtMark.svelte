<script lang="ts">
	// Squirt's mark: a Zune HD whose screen carries the brand gradient, and
	// nothing else (the gradient appears only on the screen). Black glass edge
	// to edge, a champagne metal chin with the home-button notch.
	let { size = 20, class: className = '' }: { size?: number; class?: string } = $props();
	const uid = $props.id();
</script>

<svg width={size * (53 / 102)} height={size} viewBox="0 0 53 102" class={className} aria-hidden="true">
	<defs>
		<linearGradient id="{uid}-g" x1="0" y1="0" x2="1" y2="1">
			<stop offset="0" stop-color="var(--zune-magenta)" />
			<stop offset="1" stop-color="var(--zune-orange)" />
		</linearGradient>
	</defs>
	<!-- Metal body; the glass covers all but the chin. -->
	<rect x="0.75" y="0.75" width="51.5" height="100.5" rx="3.4" fill="#c9c1b3" />
	<path d="M0.75 84.5 V4.15 A3.4 3.4 0 0 1 4.15 0.75 H48.85 A3.4 3.4 0 0 1 52.25 4.15 V84.5 H35.6 L33.4 88.6 H19.6 L17.4 84.5 Z" fill="#0d0d0d" />
	<rect x="6" y="7" width="41" height="72.9" fill="url(#{uid}-g)" />
	<!-- Hairline edge so the black glass holds its shape on a dark background. -->
	<rect x="0.75" y="0.75" width="51.5" height="100.5" rx="3.4" fill="none" stroke="currentColor" stroke-opacity="0.35" stroke-width="1.5" />
</svg>
