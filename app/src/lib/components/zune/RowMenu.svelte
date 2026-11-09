<script lang="ts">
	// A right-click menu around any row. `items` is called when the menu
	// opens, so it can read the latest state.
	import type { Snippet } from 'svelte';
	import * as ContextMenu from '$lib/components/ui/context-menu';
	import type { MenuEntry } from './menu';

	let { items, children }: { items: () => MenuEntry[]; children: Snippet } = $props();
</script>

<ContextMenu.Root>
	<ContextMenu.Trigger class="contents">{@render children()}</ContextMenu.Trigger>
	<ContextMenu.Content class="min-w-52">
		{#each items() as it, i (i)}
			{#if it === 'separator'}
				<ContextMenu.Separator />
			{:else}
				<ContextMenu.Item class="lowercase" disabled={it.disabled} variant={it.destructive ? 'destructive' : 'default'} onSelect={it.onSelect}>
					{it.label}
					{#if it.shortcut}<ContextMenu.Shortcut>{it.shortcut}</ContextMenu.Shortcut>{/if}
				</ContextMenu.Item>
			{/if}
		{/each}
	</ContextMenu.Content>
</ContextMenu.Root>
