<script lang="ts">
	import type { DeviceStatus } from '$lib/api';
	import { bytes } from '$lib/format';
	import ZuneHD from './ZuneHD.svelte';

	let { device }: { device: DeviceStatus | null } = $props();

	const segments = $derived(
		device
			? [
					{ label: 'music', value: device.usage.music, color: 'var(--zune-magenta)' },
					{ label: 'videos', value: device.usage.videos, color: 'var(--chart-3)' },
					{ label: 'pictures', value: device.usage.pictures, color: 'var(--zune-orange)' },
					{ label: 'other', value: device.usage.other, color: 'var(--chart-5)' }
				]
			: []
	);
</script>

<footer class="flex h-20 shrink-0 items-center gap-5 border-t px-6">
	<ZuneHD size={52} weight={1.25} lit={device?.connected} class={device?.connected ? 'text-foreground' : 'text-faint'} />
	{#if device?.connected}
		<div class="flex min-w-0 flex-col gap-1">
			<span class="truncate text-[1.05rem] font-[350] lowercase">{device.name}</span>
			<span class="text-xs text-muted-foreground lowercase">{device.model} · firmware {device.firmware}</span>
		</div>
		<div class="ml-auto flex w-[28rem] max-w-[45%] flex-col gap-1.5">
			<div class="flex h-2 w-full overflow-hidden bg-muted" role="meter" aria-label="storage used"
				aria-valuemin={0} aria-valuemax={device.capacity} aria-valuenow={device.capacity - device.free}>
				{#each segments as s (s.label)}
					<div style:width="{(s.value / device.capacity) * 100}%" style:background={s.color}></div>
				{/each}
			</div>
			<div class="flex gap-4 text-xs text-muted-foreground lowercase">
				{#each segments.filter((s) => s.value > 0) as s (s.label)}
					<span class="flex items-center gap-1.5">
						<span class="inline-block size-2" style:background={s.color}></span>{s.label}
						<span class="tabular">{bytes(s.value)}</span>
					</span>
				{/each}
				<span class="ml-auto"><span class="tabular">{bytes(device.free)}</span> free</span>
			</div>
		</div>
	{:else}
		<span class="text-[1.05rem] font-[350] text-muted-foreground lowercase">no device connected</span>
	{/if}
</footer>
