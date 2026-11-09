<script lang="ts">
	// Device > info: a picture of the Zune, what it is and what's on it, and
	// the device settings Squirt can change.
	import { onMount } from 'svelte';
	import type { DeviceApi, DeviceInfo } from '$lib/api';
	import { bytes } from '$lib/format';
	import ZuneArt from './ZuneArt.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { BatteryHigh, BatteryMedium, BatteryLow, BatteryWarning, PencilSimple } from 'phosphor-svelte';

	let { devices, onChange }: { devices: DeviceApi; onChange?: () => void } = $props();

	let info = $state<DeviceInfo | null>(null);
	let renaming = $state(false);
	let newName = $state('');
	let busy = $state(false);
	let error = $state<string | null>(null);

	onMount(() => {
		devices.info().then((i) => (info = i));
		const t = setInterval(async () => (info = await devices.info()), 5000);
		return () => clearInterval(t);
	});

	async function run(f: () => Promise<DeviceInfo>) {
		busy = true;
		error = null;
		try {
			info = await f();
			onChange?.();
		} catch (e) {
			error = String(e instanceof Error ? e.message : e);
		} finally {
			busy = false;
		}
	}

	function startRename() {
		newName = info?.name ?? '';
		renaming = true;
	}
	async function saveName() {
		await run(() => devices.rename(newName));
		if (!error) renaming = false;
	}

	const segments = $derived(
		info
			? [
					{ label: 'music', value: info.usage.music, color: 'var(--zune-magenta)' },
					{ label: 'videos', value: info.usage.videos, color: 'var(--chart-3)' },
					{ label: 'pictures', value: info.usage.pictures, color: 'var(--zune-orange)' },
					{ label: 'other', value: info.usage.other, color: 'var(--chart-5)' }
				].filter((s) => s.value > 0)
			: []
	);
	const battery = $derived(info?.battery ?? null);
	const BatteryIcon = $derived(battery === null ? BatteryHigh : battery > 66 ? BatteryHigh : battery > 33 ? BatteryMedium : battery > 12 ? BatteryLow : BatteryWarning);
</script>

<div class="flex min-h-0 flex-1 gap-14">
	<div class="metro-enter flex shrink-0 items-start pt-2" style="--i: 1">
		<ZuneArt height={430} lit={info?.connected ?? false} kind={info?.kind} finish={info?.finish} />
	</div>

	<div class="metro-enter flex min-w-0 max-w-2xl flex-1 flex-col gap-8 overflow-y-auto pr-2" style="--i: 2">
		{#if !info}
			<p class="text-muted-foreground lowercase">looking for your zune…</p>
		{:else if !info.connected}
			<div class="flex flex-col gap-3">
				<span class="text-[2.6rem] leading-none font-light lowercase text-muted-foreground">
					{info.released ? 'disconnected' : 'no zune connected'}
				</span>
				<p class="text-muted-foreground">
					{info.released ? 'other apps can use your zune now.' : 'plug in your zune and unlock it.'}
				</p>
				{#if info.released}
					<Button size="sm" class="w-fit lowercase" disabled={busy} onclick={() => run(() => devices.connect())}>connect</Button>
				{/if}
			</div>
		{:else}
			<!-- name -->
			<div class="flex flex-col gap-1">
				{#if renaming}
					<form class="flex items-center gap-2" onsubmit={(e) => { e.preventDefault(); saveName(); }}>
						<Input bind:value={newName} maxlength={64} class="h-11 max-w-md text-[1.6rem] font-light" autofocus />
						<Button type="submit" size="sm" class="lowercase" disabled={busy || !newName.trim()}>save</Button>
						<Button type="button" size="sm" variant="ghost" class="lowercase" onclick={() => (renaming = false)}>cancel</Button>
					</form>
				{:else}
					<button class="group flex w-fit items-center gap-3 text-left" onclick={startRename} title="rename">
						<span class="text-[2.6rem] leading-none font-light lowercase">{info.name}</span>
						<PencilSimple size={18} class="text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100" />
					</button>
				{/if}
				<span class="text-muted-foreground lowercase">{info.manufacturer} {info.model}</span>
			</div>

			<!-- storage -->
			<div class="flex flex-col gap-2">
				<h3 class="colhead">storage</h3>
				<div class="flex h-3 w-full overflow-hidden bg-muted" role="meter" aria-label="storage used"
					aria-valuemin={0} aria-valuemax={info.capacity} aria-valuenow={info.capacity - info.free}>
					{#each segments as s (s.label)}
						<div style:width="{(s.value / info.capacity) * 100}%" style:background={s.color}></div>
					{/each}
				</div>
				<div class="flex flex-wrap gap-x-5 gap-y-1 text-xs text-muted-foreground lowercase">
					{#each segments as s (s.label)}
						<span class="flex items-center gap-1.5"><span class="inline-block size-2" style:background={s.color}></span>{s.label} <span class="tabular">{bytes(s.value)}</span></span>
					{/each}
					<span class="ml-auto"><span class="tabular text-foreground">{bytes(info.free)}</span> free of <span class="tabular">{bytes(info.capacity)}</span></span>
				</div>
			</div>

			<!-- details -->
			<dl class="grid grid-cols-[8rem_1fr] items-baseline gap-x-6 gap-y-2.5">
				<dt class="colhead">songs</dt><dd class="tabular">{info.tracks.toLocaleString()}</dd>
				<dt class="colhead">albums</dt><dd class="tabular">{info.albums.toLocaleString()}</dd>
				<dt class="colhead">artists</dt><dd class="tabular">{info.artists.toLocaleString()}</dd>
				{#if battery !== null}
					<dt class="colhead">battery</dt>
					<dd class="flex items-center gap-2"><BatteryIcon size={18} class={battery <= 12 ? 'text-destructive' : ''} /><span class="tabular">{battery}%</span></dd>
				{/if}
				{#if info.finish}
					<dt class="colhead">color</dt><dd class="lowercase">{info.finish}</dd>
				{/if}
				<dt class="colhead">firmware</dt><dd class="tabular">{info.firmware}</dd>
				<dt class="colhead">serial</dt><dd class="tabular selectable text-xs leading-5 break-all text-muted-foreground">{info.serial}</dd>
			</dl>

			<!-- settings -->
			<div class="flex flex-col gap-3">
				<h3 class="colhead">device</h3>
				<div class="flex flex-wrap gap-2">
					<Button size="sm" variant="outline" class="lowercase" disabled={busy} onclick={startRename}>rename</Button>
					<Button size="sm" variant="outline" class="lowercase" disabled={busy} onclick={() => run(() => devices.refresh())}>refresh library</Button>
					<Button size="sm" variant="outline" class="lowercase" disabled={busy} onclick={() => run(() => devices.release())}>disconnect</Button>
				</div>
				<p class="text-xs text-muted-foreground">disconnect frees your zune for other apps without unplugging it.</p>
			</div>
		{/if}
		{#if error}<p class="text-xs text-destructive">{error}</p>{/if}
	</div>
</div>
