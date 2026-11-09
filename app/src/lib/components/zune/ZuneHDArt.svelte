<script lang="ts">
	// A large Zune HD illustration for the device page: black glass edge to
	// edge, champagne chin with the home-button notch, and a lit screen showing
	// the time and date the way the Zune HD's lock screen does.
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
</script>

<svg width={height * (53 / 102)} height={height} viewBox="0 0 53 102" aria-hidden="true" class="drop-shadow-2xl">
	<defs>
		<linearGradient id="{uid}-screen" x1="0" y1="0" x2="1" y2="1">
			<stop offset="0" stop-color="var(--zune-magenta)" />
			<stop offset="1" stop-color="var(--zune-orange)" />
		</linearGradient>
		<linearGradient id="{uid}-metal" x1="0" y1="0" x2="0" y2="1">
			<stop offset="0" stop-color="#d8d1c4" />
			<stop offset="1" stop-color="#b4ac9e" />
		</linearGradient>
		<linearGradient id="{uid}-sheen" x1="0" y1="0" x2="1" y2="0.6">
			<stop offset="0" stop-color="#fff" stop-opacity="0.12" />
			<stop offset="0.5" stop-color="#fff" stop-opacity="0" />
		</linearGradient>
	</defs>
	<rect x="0.5" y="0.5" width="52" height="101" rx="3.4" fill="url(#{uid}-metal)" />
	<path d="M0.5 84.5 V3.9 A3.4 3.4 0 0 1 3.9 0.5 H49.1 A3.4 3.4 0 0 1 52.5 3.9 V84.5 H35.6 L33.4 88.6 H19.6 L17.4 84.5 Z" fill="#0b0b0b" />
	<rect x="6" y="7" width="41" height="72.9" fill={lit ? `url(#${uid}-screen)` : '#141414'} />
	{#if lit}
		<!-- Lock screen: big light time, day and date beneath, bottom-left. -->
		<g fill="#fff" font-family="Selawik, 'Segoe UI', sans-serif">
			<text x="8.5" y="66" font-size="11" font-weight="300">{time}</text>
			<text x="8.6" y="71.5" font-size="3.2" font-weight="350" opacity="0.9">{day}</text>
			<text x="8.6" y="75.5" font-size="3.2" font-weight="350" opacity="0.9">{date}</text>
		</g>
	{/if}
	<!-- Glass sheen across the face. -->
	<path d="M0.5 84.5 V3.9 A3.4 3.4 0 0 1 3.9 0.5 H49.1 A3.4 3.4 0 0 1 52.5 3.9 V84.5 Z" fill="url(#{uid}-sheen)" />
	<rect x="0.5" y="0.5" width="52" height="101" rx="3.4" fill="none" stroke="#000" stroke-opacity="0.35" stroke-width="0.4" />
</svg>
