<script lang="ts">
	// Now-playing background: three soft glows in the cover's colors,
	// drifting slowly. Crossfades to the new colors when the album changes.
	import { fade } from 'svelte/transition';
	import { fallbackPalette, palette } from './palette';

	let { src, hue }: { src?: string | null; hue: number } = $props();

	let colors = $state<string[]>(fallbackPalette(0));
	$effect(() => {
		const s = src;
		const h = hue;
		let stale = false;
		palette(s, h).then((c) => {
			if (!stale) colors = c;
		});
		return () => (stale = true);
	});
</script>

<div class="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden="true">
	{#key colors.join()}
		<div class="absolute inset-0" transition:fade={{ duration: 1200 }}>
			<div class="glow g1" style="--c: {colors[0]}"></div>
			<div class="glow g2" style="--c: {colors[1]}"></div>
			<div class="glow g3" style="--c: {colors[2]}"></div>
		</div>
	{/key}
</div>

<style>
	.glow {
		position: absolute;
		width: 85vmax;
		height: 85vmax;
		background: radial-gradient(circle, var(--c) 0%, transparent 62%);
		opacity: 0.6;
		will-change: transform;
	}
	/* Three loops of different lengths, so the pattern never visibly repeats. */
	.g1 {
		left: -30vmax;
		top: -35vmax;
		animation: g1 29s ease-in-out infinite alternate;
	}
	.g2 {
		right: -35vmax;
		top: -20vmax;
		animation: g2 37s ease-in-out infinite alternate;
	}
	.g3 {
		left: 10vmax;
		bottom: -50vmax;
		opacity: 0.45;
		animation: g3 43s ease-in-out infinite alternate;
	}
	@keyframes g1 {
		to {
			transform: translate(22vmax, 14vmax) scale(1.15);
		}
	}
	@keyframes g2 {
		to {
			transform: translate(-18vmax, 20vmax) scale(0.9);
		}
	}
	@keyframes g3 {
		to {
			transform: translate(15vmax, -12vmax) scale(1.2);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.glow {
			animation: none;
		}
	}
</style>
