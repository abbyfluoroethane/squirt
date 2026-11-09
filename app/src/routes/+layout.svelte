<script lang="ts">
	import './layout.css';
	import favicon from '#lib/assets/favicon.svg';
	import type { LayoutProps } from './$types';
	import { onMount } from 'svelte';
	import { theme } from '$lib/theme.svelte';
	import * as Tooltip from '$lib/components/ui/tooltip';

	onMount(() => theme.init());

	// No browser menu (reload, inspect element) on right-click: rows that
	// have actions show their own menu. Text fields and selectable text keep
	// the system one for copy and paste.
	function onContextMenu(e: MouseEvent) {
		const t = e.target as HTMLElement | null;
		if (t?.closest('input, textarea, [contenteditable], .selectable')) return;
		e.preventDefault();
	}

	let { children }: LayoutProps = $props();
</script>

<svelte:head>
	<title>Squirt</title>
	<link rel="icon" href={favicon} />
</svelte:head>

<svelte:window oncontextmenu={onContextMenu} />

<Tooltip.Provider>
	{@render children()}
</Tooltip.Provider>
