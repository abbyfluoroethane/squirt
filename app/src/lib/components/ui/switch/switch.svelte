<script lang="ts">
	// The stock shadcn-svelte switch, with square corners to match the rest of
	// the UI. Track fills with the primary (magenta) when on.
	import { Switch as SwitchPrimitive } from 'bits-ui';
	import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		checked = $bindable(false),
		...restProps
	}: WithoutChildrenOrChild<SwitchPrimitive.RootProps> = $props();
</script>

<SwitchPrimitive.Root
	bind:ref
	bind:checked
	data-slot="switch"
	class={cn(
		'peer inline-flex h-[18px] w-[32px] shrink-0 items-center rounded-none border border-transparent p-px transition-colors outline-none',
		'data-checked:bg-primary data-unchecked:bg-input',
		'focus-visible:ring-2 focus-visible:ring-ring/50',
		'data-disabled:cursor-not-allowed data-disabled:opacity-50',
		className
	)}
	{...restProps}
>
	<SwitchPrimitive.Thumb
		data-slot="switch-thumb"
		class="pointer-events-none block size-[14px] rounded-none bg-background ring-0 transition-transform data-checked:translate-x-[14px] data-unchecked:translate-x-0 dark:data-checked:bg-primary-foreground dark:data-unchecked:bg-foreground"
	/>
</SwitchPrimitive.Root>
