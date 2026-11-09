<script lang="ts">
	import { onMount } from 'svelte';
	import { connect, isTauri, type Album, type Api, type Backend, type DeviceStatus, type PlayerState, type Track } from '$lib/api';
	import { bytes, duration } from '$lib/format';
	import { theme, type Theme } from '$lib/theme.svelte';
	import Pivot from '$lib/components/zune/Pivot.svelte';
	import AlbumArt from '$lib/components/zune/AlbumArt.svelte';
	import DeviceBar from '$lib/components/zune/DeviceBar.svelte';
	import Transport from '$lib/components/zune/Transport.svelte';
	import { Slider } from '$lib/components/ui/slider';
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Check, Play, SpeakerHigh, SpeakerSimpleNone } from 'phosphor-svelte';
	import ZuneHD from '$lib/components/zune/ZuneHD.svelte';
	import NowPlaying from '$lib/components/zune/NowPlaying.svelte';
	import { tick } from 'svelte';

	const PIVOTS = ['collection', 'device', 'settings'] as const;
	const SUBPIVOTS = ['music', 'videos', 'pictures', 'podcasts'] as const;

	let pivot = $state<(typeof PIVOTS)[number]>('collection');
	let subpivot = $state<(typeof SUBPIVOTS)[number]>('music');

	let device = $state<DeviceStatus | null>(null);
	let albums = $state<Album[]>([]);
	let deviceAlbums = $state<Album[]>([]);
	let fullscreen = $state(false);
	let artist = $state<string | null>(null);
	let selectedId = $state<string | null>(null);

	let backend = $state<Backend>('mock');
	let api: Api | null = null;
	let player = $state<PlayerState>({ status: 'stopped', trackId: null, positionMs: 0, durationMs: 0, volume: 0.7, queue: [] });

	onMount(() => {
		const timers: ReturnType<typeof setInterval>[] = [];
		(async () => {
			const c = await connect();
			api = c.api;
			backend = c.backend;
			[device, player] = await Promise.all([c.api.device(), c.api.player.state()]);
			volume = player.volume;
			await reload(c.api);
			// Deep links for development: #device, #now-playing.
			if (location.hash === '#device') pivot = 'device';
			if (location.hash === '#now-playing') fullscreen = true;
			// Notice the Zune being plugged in or out; reload what's on it when it changes.
			timers.push(
				setInterval(async () => {
					const was = device?.connected;
					device = await c.api.device();
					if (device.connected !== was) await reload(c.api);
				}, 3000)
			);
			timers.push(setInterval(async () => (player = await c.api.player.state()), 500));
		})();
		return () => timers.forEach(clearInterval);
	});

	async function reload(a: Api) {
		[albums, deviceAlbums] = await Promise.all([a.collection(), a.deviceCollection()]);
	}

	/** Shows an artist (and optionally one of their albums) in the collection. */
	async function show(artistName: string, albumId?: string) {
		fullscreen = false;
		pivot = 'collection';
		subpivot = 'music';
		artist = artistName;
		selectedId = albumId ?? null;
		await tick();
		if (albumId) document.getElementById(`album-${albumId}`)?.scrollIntoView({ block: 'nearest' });
	}

	// Track id -> track and album, for the now-playing line.
	const byId = $derived(new Map(albums.flatMap((a) => a.tracks.map((t) => [t.id, { t, a }] as const))));
	const nowPlaying = $derived(player.trackId !== null ? byId.get(player.trackId) : undefined);

	async function playAlbum(al: Album, start = 0) {
		// Device-only tracks have negative ids and are not on this computer.
		const ids = al.tracks.map((t) => t.id).filter((x): x is number => x !== undefined && x > 0);
		if (api && ids.length) player = await api.player.play(ids, start);
	}
	const act = (f: (a: Api) => Promise<PlayerState>) => async () => {
		if (api) player = await f(api);
	};
	// The slider owns the volume while you use it; it starts from the player's.
	let volume = $state(0.7);
	let volumeTimer: ReturnType<typeof setTimeout>;
	function setVolume(v: number) {
		clearTimeout(volumeTimer);
		volumeTimer = setTimeout(() => api?.player.volume(v), 60);
	}

	function onKey(e: KeyboardEvent) {
		if (e.code === 'Escape' && fullscreen) {
			fullscreen = false;
			return;
		}
		if (e.code === 'Space' && !(e.target instanceof HTMLInputElement)) {
			e.preventDefault();
			act((a) => a.player.toggle())();
		}
	}
	const isPlaying = (t: Track) => t.id !== undefined && t.id === player.trackId;

	// "device" shows what is on the Zune itself; "collection" shows the library.
	const visible = $derived(pivot === 'device' ? deviceAlbums : albums);
	const artists = $derived([...new Set(visible.map((a) => a.artist))].sort((a, b) => a.localeCompare(b)));
	const shownAlbums = $derived(artist ? visible.filter((a) => a.artist === artist) : visible);
	const selected = $derived(shownAlbums.find((a) => a.id === selectedId) ?? shownAlbums[0] ?? null);
	const multiDisc = $derived(selected ? new Set(selected.tracks.map((t) => t.disc)).size > 1 : false);
	const selectedOnDevice = $derived(selected?.tracks.every((t) => t.onDevice) ?? false);

	const THEMES: Theme[] = ['system', 'light', 'dark'];
	const macApp = isTauri && typeof navigator !== 'undefined' && navigator.userAgent.includes('Mac');
</script>

<svelte:window onkeydown={onKey} />

<div class="flex h-dvh flex-col overflow-hidden">
	<!-- In the app, the header doubles as the window's title bar (macOS draws
	     the traffic lights over its top-left corner). -->
	<header class="flex shrink-0 flex-col gap-3 px-6 {macApp ? 'pt-0' : 'pt-5'}" data-tauri-drag-region>
		<!-- On macOS the brand row shares the title bar with the traffic lights.
		     Positions measured from screenshots: the lights are centred ~15.5pt
		     down, and the row needs a 3.5pt nudge to meet them; the left padding
		     leaves a gap after the green light about as wide as the gaps between
		     the lights. -->
		<div class="flex items-center gap-2.5 text-xs font-semibold tracking-wide text-muted-foreground lowercase {macApp ? 'pointer-events-none relative top-[3.5px] -mb-1 h-8 pl-[66px]' : ''}">
			<span class="zune-gradient inline-block size-3.5"></span><span class="text-[0.95rem] leading-none">squirt</span>
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
		{:else if pivot === 'device' && !device?.connected}
			<section class="flex flex-col gap-2">
				<p class="subpivot text-muted-foreground">{device?.problem ? 'your zune is connected but squirt cannot use it' : 'connect your zune'}</p>
				{#if device?.problem}<p class="text-muted-foreground">{device.problem}</p>{/if}
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
						<button id="album-{al.id}" class="flex w-full items-center gap-3 p-1.5 text-left transition-colors {selected?.id === al.id ? 'bg-selection' : 'hover:bg-accent'}"
							onclick={() => (selectedId = al.id)}>
							<AlbumArt hue={al.hue} size={52} src={al.art ?? undefined} />
							<span class="flex min-w-0 flex-col">
								<span class="truncate text-[0.95rem]">{al.title}</span>
								<span class="truncate text-xs text-muted-foreground">{al.artist}{al.year ? ` · ${al.year}` : ''}</span>
							</span>
							{#if pivot === 'collection' && al.tracks.every((t) => t.onDevice)}
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
							<Button size="sm" variant="outline" class="lowercase" onclick={() => playAlbum(selected!)}><Play weight="fill" />play</Button>
						{/if}
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
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div class="grid grid-cols-[2.5rem_1fr_3.5rem_1.25rem] items-center gap-x-3 px-1.5 py-1 hover:bg-accent {isPlaying(t) ? 'text-magenta' : ''}"
								ondblclick={() => playAlbum(selected!, i)}>
								<span class="tabular text-xs {isPlaying(t) ? '' : 'text-muted-foreground'}">
									{#if isPlaying(t)}<SpeakerHigh size={13} weight="fill" class="inline" />{:else}{t.n}{/if}
								</span>
								<span class="truncate">{t.title}</span>
								<span class="tabular text-right text-xs text-muted-foreground">{duration(t.durationMs)}</span>
								{#if pivot === 'collection' && t.onDevice}<ZuneHD size={14} class="text-muted-foreground" />{:else}<span></span>{/if}
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

	<footer class="grid h-24 shrink-0 grid-cols-[1fr_auto_1fr] items-center gap-6 border-t px-6">
		<DeviceBar {device} />
		<Transport
			state={player}
			title={nowPlaying?.t.title}
			artist={nowPlaying?.t.artist}
			album={nowPlaying?.a.title}
			art={nowPlaying?.a.art ?? undefined}
			hue={nowPlaying?.a.hue}
			onArtist={() => nowPlaying && show(nowPlaying.a.artist)}
			onAlbum={() => nowPlaying && show(nowPlaying.a.artist, nowPlaying.a.id)}
			onArt={() => (fullscreen = true)}
			onToggle={act((a) => a.player.toggle())}
			onPrev={act((a) => a.player.prev())}
			onNext={act((a) => a.player.next())}
			onSeek={(ms) => act((a) => a.player.seek(ms))()}
		/>
		<div class="flex items-center justify-end gap-2.5 text-muted-foreground">
			<SpeakerSimpleNone size={16} />
			<Slider type="single" bind:value={volume} min={0} max={1} step={0.01} class="w-28" onValueChange={setVolume} aria-label="volume" />
			<SpeakerHigh size={16} />
		</div>
	</footer>
</div>

{#if fullscreen && nowPlaying}
	<NowPlaying
		state={player}
		track={nowPlaying.t}
		album={nowPlaying.a}
		upNext={player.queue.slice(player.queue.indexOf(player.trackId ?? -1) + 1, player.queue.indexOf(player.trackId ?? -1) + 5).map((id) => byId.get(id)?.t).filter((t) => t !== undefined)}
		bind:volume
		onVolume={setVolume}
		onToggle={act((a) => a.player.toggle())}
		onPrev={act((a) => a.player.prev())}
		onNext={act((a) => a.player.next())}
		onSeek={(ms) => act((a) => a.player.seek(ms))()}
		onArtist={() => show(nowPlaying!.a.artist)}
		onAlbum={() => show(nowPlaying!.a.artist, nowPlaying!.a.id)}
		onClose={() => (fullscreen = false)}
	/>
{/if}

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
