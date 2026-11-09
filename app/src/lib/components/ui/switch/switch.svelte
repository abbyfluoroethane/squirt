<script lang="ts">
	// Metro toggle, after the Zune HD / Windows Phone 7 switch: a rectangular
	// outlined track that fills with the accent when on, a solid bar for the
	// thumb, and an on/off label beside it.
	import { Switch as SwitchPrimitive } from 'bits-ui';
	import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		checked = $bindable(false),
		showLabel = true,
		...restProps
	}: WithoutChildrenOrChild<SwitchPrimitive.RootProps> & { showLabel?: boolean } = $props();
</script>

<span class="inline-flex items-center gap-2.5">
	{#if showLabel}
		<span class="w-5 text-right text-xs lowercase {checked ? 'text-foreground' : 'text-muted-foreground'}" aria-hidden="true">{checked ? 'on' : 'off'}</span>
	{/if}
	<SwitchPrimitive.Root
		bind:ref
		bind:checked
		data-slot="switch"
		class={cn(
			'group/switch peer relative inline-flex h-5 w-9 shrink-0 items-center border-2 p-[2px] transition-colors outline-none',
			'data-unchecked:border-muted-foreground data-checked:border-foreground',
			'focus-visible:ring-2 focus-visible:ring-ring/60 focus-visible:ring-offset-1 focus-visible:ring-offset-background',
			'data-disabled:cursor-not-allowed data-disabled:opacity-40',
			'hover:data-unchecked:border-foreground',
			className
		)}
		{...restProps}
	>
		<!-- Accent fill, growing from the left as the thumb moves right. -->
		<span
			class="pointer-events-none absolute inset-[2px] origin-left bg-magenta transition-transform duration-150 ease-out group-data-unchecked/switch:scale-x-0"
			aria-hidden="true"
		></span>
		<SwitchPrimitive.Thumb
			data-slot="switch-thumb"
			class={cn(
				'pointer-events-none relative block h-full w-2.5 transition-transform duration-150 ease-out',
				'data-unchecked:translate-x-0 data-unchecked:bg-muted-foreground',
				'data-checked:translate-x-[18px] data-checked:bg-foreground'
			)}
		/>
	</SwitchPrimitive.Root>
</span>
