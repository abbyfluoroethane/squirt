<script lang="ts">
	// Compact device summary for the footer: icon, name, storage meter. While
	// songs are being sent it shows the sync instead, then the result for a
	// few seconds.
	import type { DeviceStatus, SyncStatus } from '$lib/api';
	import { bytes } from '$lib/format';
	import ZuneHD from './ZuneHD.svelte';
	import * as Tooltip from '$lib/components/ui/tooltip';

	let { device, sync = null, showResult = false }: { device: DeviceStatus | null; sync?: SyncStatus | null; showResult?: boolean } = $props();

	const syncing = $derived(!!sync?.active);
	const syncPct = $derived(sync && sync.bytesTotal ? Math.min(100, (sync.bytesDone / sync.bytesTotal) * 100) : 0);

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
			{#if syncing && sync}
				<!-- Sending songs: progress by bytes, then what's being sent. -->
				<div class="h-1.5 w-44 bg-muted" role="progressbar" aria-label="sending to zune" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(syncPct)}>
					<div class="zune-gradient h-full transition-[width] duration-300 ease-out" style:width="{syncPct}%"></div>
				</div>
				<span class="flex max-w-56 min-w-0 gap-1 text-xs leading-none text-muted-foreground lowercase">
					<span class="shrink-0">sending <span class="tabular text-foreground">{Math.min(sync.done + 1, sync.total)}</span> of <span class="tabular">{sync.total}</span></span>
					{#if sync.current}<span class="truncate normal-case">· {sync.current}</span>{/if}
				</span>
			{:else if showResult && sync?.last}
				<div class="zune-gradient h-1.5 w-44"></div>
				<Tooltip.Root>
					<Tooltip.Trigger>
						{#snippet child({ props })}
							<span {...props} class="text-xs leading-none lowercase {sync.errors.length ? 'text-destructive' : 'text-muted-foreground'}">{sync.last}</span>
						{/snippet}
					</Tooltip.Trigger>
					{#if sync.errors.length}
						<Tooltip.Content class="max-w-96">
							{#each sync.errors.slice(0, 8) as e (e)}<div class="truncate">{e}</div>{/each}
							{#if sync.errors.length > 8}<div>and {sync.errors.length - 8} more</div>{/if}
						</Tooltip.Content>
					{/if}
				</Tooltip.Root>
			{:else}
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
			{/if}
		</div>
	{:else}
		<span class="text-[1.05rem] font-[350] text-muted-foreground lowercase">no device connected</span>
	{/if}
</div>
