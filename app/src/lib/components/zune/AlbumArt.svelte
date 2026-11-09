<script lang="ts">
	import { MusicNotesSimple } from 'phosphor-svelte';

	let { hue, size = 64, src }: { hue: number; size?: number; src?: string } = $props();
</script>

<div
	class="relative shrink-0 overflow-hidden bg-muted"
	style:width="{size}px"
	style:height="{size}px"
	style:background-image={src
		? undefined
		: `linear-gradient(135deg, oklch(0.55 0.13 ${hue}), oklch(0.32 0.08 ${(hue + 40) % 360}))`}
>
	{#if src}
		<img {src} alt="" class="size-full object-cover" draggable="false" />
	{:else}
		<MusicNotesSimple class="absolute right-1.5 bottom-1.5 text-white/50" size={Math.max(12, size / 5)} weight="light" />
	{/if}
</div>
