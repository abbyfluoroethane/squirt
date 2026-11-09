<script lang="ts">
	// A Zune drawn as the model and colorway that's connected. The drawings
	// live in ./art (one module per model); this picks one and keeps its
	// lock-screen clock current.
	import { onMount } from 'svelte';
	import type { ArtOptions, ZuneModel } from './art/types';

	type Mod = { model: ZuneModel; render: (o: ArtOptions) => string };
	const mods: Mod[] = Object.values(import.meta.glob<Mod>('./art/*.ts', { eager: true })).filter((m) => m.model && m.render);

	let {
		kind = 'zunehd',
		finish = null,
		lit = true,
		height = 420
	}: { kind?: string | null; finish?: string | null; lit?: boolean; height?: number } = $props();

	const uid = $props.id();
	let now = $state(new Date());
	onMount(() => {
		const t = setInterval(() => (now = new Date()), 10_000);
		return () => clearInterval(t);
	});

	const mod = $derived(mods.find((m) => m.model.id === kind) ?? mods.find((m) => m.model.id === 'zunehd') ?? mods[0]);
	const svg = $derived(mod ? mod.render({ finish: finish ?? mod.model.finishes[0]?.id ?? '', lit, height, uid, now }) : '');
</script>

{#if mod}
	<div class="drop-shadow-2xl" style:width="{height * mod.model.aspect}px" style:height="{height}px" aria-hidden="true">{@html svg}</div>
{/if}
