<script lang="ts">
	import './layout.css';
	import favicon from '#lib/assets/favicon.svg';
	import type { LayoutProps } from './$types';
	import { onMount } from 'svelte';
	import { theme } from '$lib/theme.svelte';
	import * as Tooltip from '$lib/components/ui/tooltip';

	onMount(() => theme.init());

	let { children }: LayoutProps = $props();
</script>

<svelte:head>
	<title>Squirt</title>
	<link rel="icon" href={favicon} />
</svelte:head>

<Tooltip.Provider>
	{@render children()}
</Tooltip.Provider>
