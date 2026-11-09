<script lang="ts" generics="T extends string">
	// A row of lowercase pivot headers, Zune-style: the selected one is solid,
	// the rest are faint. `size` picks the top-level or second-level style.
	let {
		items,
		value = $bindable(),
		size = 'pivot',
		disabled = []
	}: { items: readonly T[]; value: T; size?: 'pivot' | 'subpivot'; disabled?: readonly T[] } = $props();
</script>

<div class="flex items-baseline gap-6" role="tablist">
	{#each items as item (item)}
		{@const off = disabled.includes(item)}
		<button
			role="tab"
			aria-selected={value === item}
			disabled={off}
			class="{size} transition-colors duration-150 {value === item
				? 'text-foreground'
				: off
					? 'text-faint/60 cursor-default'
					: 'text-faint hover:text-muted-foreground'}"
			onclick={() => (value = item)}
		>
			{item}
		</button>
	{/each}
</div>
