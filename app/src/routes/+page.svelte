<script lang="ts">
	import { onMount } from 'svelte';
	import { connect, type Album, type Api, type Backend, type DeviceStatus } from '$lib/api';
	import { bytes, duration } from '$lib/format';
	import { theme, type Theme } from '$lib/theme.svelte';
	import Pivot from '$lib/components/zune/Pivot.svelte';
	import AlbumArt from '$lib/components/zune/AlbumArt.svelte';
	import DeviceBar from '$lib/components/zune/DeviceBar.svelte';
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Check } from 'phosphor-svelte';
	import ZuneHD from '$lib/components/zune/ZuneHD.svelte';

	const PIVOTS = ['collection', 'device', 'settings'] as const;
	const SUBPIVOTS = ['music', 'videos', 'pictures', 'podcasts'] as const;

	let pivot = $state<(typeof PIVOTS)[number]>('collection');
	let subpivot = $state<(typeof SUBPIVOTS)[number]>('music');

	let device = $state<DeviceStatus | null>(null);
	let albums = $state<Album[]>([]);
	let artist = $state<string | null>(null);
	let selectedId = $state<string | null>(null);

	let backend = $state<Backend>('mock');

	onMount(() => {
		let api: Api;
		let timer: ReturnType<typeof setInterval>;
		(async () => {
			({ api, backend } = await connect());
			[device, albums] = await Promise.all([api.device(), api.collection()]);
			// Notice the Zune being plugged in or out.
			timer = setInterval(async () => (device = await api.device()), 5000);
		})();
		return () => clearInterval(timer);
	});

	// "device" shows only what is on the Zune; "collection" shows everything.
	const visible = $derived(
		pivot === 'device' ? albums.filter((a) => a.tracks.some((t) => t.onDevice)) : albums
	);
	const artists = $derived([...new Set(visible.map((a) => a.artist))].sort((a, b) => a.localeCompare(b)));
	const shownAlbums = $derived(artist ? visible.filter((a) => a.artist === artist) : visible);
	const selected = $derived(shownAlbums.find((a) => a.id === selectedId) ?? shownAlbums[0] ?? null);
	const multiDisc = $derived(selected ? new Set(selected.tracks.map((t) => t.disc)).size > 1 : false);
	const selectedOnDevice = $derived(selected?.tracks.every((t) => t.onDevice) ?? false);

	const THEMES: Theme[] = ['system', 'light', 'dark'];
</script>

<div class="flex h-dvh flex-col overflow-hidden">
	<header class="flex shrink-0 flex-col gap-3 px-6 pt-5">
		<div class="flex items-center gap-3 text-xs font-semibold tracking-wide text-muted-foreground lowercase">
			<span class="zune-gradient inline-block size-2.5"></span>squirt
			{#if backend === 'mock'}<span class="font-normal text-faint">· browser preview, mock data (run <code class="font-mono">squirt serve</code> for your library)</span>
			{:else if backend === 'dev-api'}<span class="font-normal text-faint">· browser preview, your library</span>{/if}
		</div>
		<Pivot items={PIVOTS} bind:value={pivot} />
		{#if pivot !== 'settings'}
			<Pivot items={SUBPIVOTS} bind:value={subpivot} size="subpivot" disabled={['videos', 'pictures', 'podcasts']} />
		{/if}
	</header>

	<main class="flex min-h-0 flex-1 gap-0 px-6 pt-5 pb-3">
		{#if pivot === 'settings'}
			<section class="flex max-w-xl flex-col gap-8">
				<div class="flex flex-col gap-3">
					<h2 class="subpivot">theme</h2>
					<div class="flex gap-2">
						{#each THEMES as t (t)}
							<Button variant={theme.value === t ? 'default' : 'outline'} size="sm" class="lowercase" onclick={() => theme.set(t)}>{t}</Button>
						{/each}
					</div>
				</div>
				<div class="flex flex-col gap-2">
					<h2 class="subpivot">mtpz keys</h2>
					<p class="text-muted-foreground">Read from <code class="font-mono text-foreground">~/.mtpz-data</code>. Squirt does not include keys.</p>
				</div>
			</section>
		{:else}
			<!-- artists -->
			<section class="flex w-52 shrink-0 flex-col">
				<h3 class="colhead mb-2">artist</h3>
				<ScrollArea class="min-h-0 flex-1 pr-3">
					<button class="row {artist === null ? 'row-on' : ''}" onclick={() => (artist = null)}>all</button>
					{#each artists as a (a)}
						<button class="row {artist === a ? 'row-on' : ''}" onclick={() => { artist = a; selectedId = null; }}>{a}</button>
					{/each}
				</ScrollArea>
			</section>

			<!-- albums -->
			<section class="flex min-w-0 flex-1 flex-col border-l pl-4">
				<h3 class="colhead mb-2">album</h3>
				<ScrollArea class="min-h-0 flex-1 pr-3">
					{#each shownAlbums as al (al.id)}
						<button class="flex w-full items-center gap-3 p-1.5 text-left transition-colors {selected?.id === al.id ? 'bg-selection' : 'hover:bg-accent'}"
							onclick={() => (selectedId = al.id)}>
							<AlbumArt hue={al.hue} size={52} src={al.art ?? undefined} />
							<span class="flex min-w-0 flex-col">
								<span class="truncate text-[0.95rem]">{al.title}</span>
								<span class="truncate text-xs text-muted-foreground">{al.artist}{al.year ? ` · ${al.year}` : ''}</span>
							</span>
							{#if al.tracks.every((t) => t.onDevice)}
								<ZuneHD class="ml-auto shrink-0 text-muted-foreground" size={16} />
							{/if}
						</button>
					{/each}
				</ScrollArea>
			</section>

			<!-- songs -->
			<section class="flex w-[44%] shrink-0 flex-col border-l pl-4">
				{#if selected}
					<div class="mb-4 flex items-end gap-4">
						<AlbumArt hue={selected.hue} size={96} src={selected.art ?? undefined} />
						<div class="flex min-w-0 flex-1 flex-col gap-0.5">
							<span class="truncate text-[1.35rem] leading-tight font-light">{selected.title}</span>
							<span class="truncate text-muted-foreground">{selected.artist}</span>
							<span class="text-xs text-muted-foreground lowercase">
								{[selected.genre, selected.year, `${selected.tracks.length} songs`].filter(Boolean).join(' · ')}
							</span>
						</div>
						{#if pivot === 'collection'}
							{#if selectedOnDevice}
								<span class="flex items-center gap-1.5 pb-1 text-xs text-muted-foreground lowercase"><Check size={14} />on device</span>
							{:else}
								<Button size="sm" class="lowercase">add to device</Button>
							{/if}
						{/if}
					</div>
					<div class="grid grid-cols-[2.5rem_1fr_3.5rem_1.25rem] gap-x-3 px-1.5 pb-1.5">
						<span class="colhead">#</span><span class="colhead">song</span><span class="colhead text-right">length</span><span></span>
					</div>
					<ScrollArea class="min-h-0 flex-1">
						{#each selected.tracks as t, i (i)}
							{#if multiDisc && (i === 0 || selected.tracks[i - 1].disc !== t.disc)}
								<div class="colhead px-1.5 pt-3 pb-1">disc {t.disc}</div>
							{/if}
							<div class="grid grid-cols-[2.5rem_1fr_3.5rem_1.25rem] items-center gap-x-3 px-1.5 py-1 hover:bg-accent">
								<span class="tabular text-xs text-muted-foreground">{t.n}</span>
								<span class="truncate">{t.title}</span>
								<span class="tabular text-right text-xs text-muted-foreground">{duration(t.durationMs)}</span>
								{#if t.onDevice}<ZuneHD size={14} class="text-muted-foreground" />{:else}<span></span>{/if}
							</div>
						{/each}
					</ScrollArea>
					<div class="pt-2 text-xs text-muted-foreground lowercase">
						<span class="tabular">{bytes(selected.tracks.reduce((s, t) => s + t.size, 0))}</span> ·
						<span class="tabular">{duration(selected.tracks.reduce((s, t) => s + t.durationMs, 0))}</span>
					</div>
				{:else}
					<p class="text-muted-foreground lowercase">nothing here yet</p>
				{/if}
			</section>
		{/if}
	</main>

	<DeviceBar {device} />
</div>

<style>
	.row {
		display: block;
		width: 100%;
		padding: 0.3rem 0.4rem;
		text-align: left;
		font-size: 0.95rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		transition: background-color 120ms;
	}
	.row:hover {
		background: var(--accent);
	}
	.row-on,
	.row-on:hover {
		background: var(--selection);
		color: var(--foreground);
	}
</style>
