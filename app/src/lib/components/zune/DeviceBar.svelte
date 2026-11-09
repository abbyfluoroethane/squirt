<script lang="ts">
	// Compact device summary for the footer: icon, name, storage meter.
	import type { DeviceStatus } from '$lib/api';
	import { bytes } from '$lib/format';
	import ZuneHD from './ZuneHD.svelte';
	import * as Tooltip from '$lib/components/ui/tooltip';

	let { device }: { device: DeviceStatus | null } = $props();

	const segments = $derived(
		device
			? [
					{ label: 'music', value: device.usage.music, color: 'var(--zune-magenta)' },
					{ label: 'videos', value: device.usage.videos, color: 'var(--chart-3)' },
					{ label: 'pictures', value: device.usage.pictures, color: 'var(--zune-orange)' },
					{ label: 'other', value: device.usage.other, color: 'var(--chart-5)' }
				].filter((s) => s.value > 0)
			: []
	);
</script>

<div class="flex min-w-0 items-center gap-4">
	<ZuneHD size={48} lit={device?.connected} class="shrink-0 {device?.connected ? 'text-foreground' : 'text-faint'}" />
	{#if device?.connected}
		<div class="flex min-w-0 flex-col gap-1.5">
			<span class="truncate text-[1.05rem] leading-none font-[350] lowercase">{device.name}</span>
			<Tooltip.Root>
				<Tooltip.Trigger>
					{#snippet child({ props })}
						<div {...props} class="flex h-1.5 w-44 overflow-hidden bg-muted" role="meter" aria-label="storage used"
							aria-valuemin={0} aria-valuemax={device.capacity} aria-valuenow={device.capacity - device.free}>
							{#each segments as s (s.label)}
								<div style:width="{(s.value / device.capacity) * 100}%" style:background={s.color}></div>
							{/each}
						</div>
					{/snippet}
				</Tooltip.Trigger>
				<Tooltip.Content class="lowercase">
					{#each segments as s (s.label)}<div>{s.label} <span class="tabular">{bytes(s.value)}</span></div>{/each}
				</Tooltip.Content>
			</Tooltip.Root>
			<span class="text-xs leading-none text-muted-foreground lowercase"><span class="tabular">{bytes(device.free)}</span> free of <span class="tabular">{bytes(device.capacity)}</span></span>
		</div>
	{:else}
		<span class="text-[1.05rem] font-[350] text-muted-foreground lowercase">no device connected</span>
	{/if}
</div>
