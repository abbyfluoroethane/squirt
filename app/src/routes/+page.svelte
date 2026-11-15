<script lang="ts">
	import { onMount } from 'svelte';
	import { connect, isTauri, openExternal, pickFolder, revealFile, type PictureAlbum, type VideoItem, type Album, type Api, type Backend, type DeviceStatus, type PlayerState, type SyncStatus, type Track } from '$lib/api';
	import { bytes, duration } from '$lib/format';
	import { theme, type Theme } from '$lib/theme.svelte';
	import Pivot from 'fluorokit/svelte/Pivot.svelte';
	import { setEnterDirection } from 'fluorokit/enter';
	import AlbumArt from '$lib/components/zune/AlbumArt.svelte';
	import DeviceBar from '$lib/components/zune/DeviceBar.svelte';
	import Transport from '$lib/components/zune/Transport.svelte';
	import { Slider } from '$lib/components/ui/slider';
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Check, Play, SpeakerHigh, SpeakerSimpleNone } from 'phosphor-svelte';
	import ZuneHD from '$lib/components/zune/ZuneHD.svelte';
	import SquirtMark from '$lib/components/zune/SquirtMark.svelte';
	import RowMenu from '$lib/components/zune/RowMenu.svelte';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { revealLabel, type MenuEntry } from '$lib/components/zune/menu';
	import { tick } from 'svelte';
	import { hideSplash, splashMinimum } from '$lib/splash';

	// Views that the first screen doesn't show load when they are first
	// opened, so the app starts with less script to read. `preloadViews`
	// fetches them all once the window is up, so opening one never waits.
	const views = {
		NowPlaying: () => import('$lib/components/zune/NowPlaying.svelte'),
		Social: () => import('$lib/components/zune/Social.svelte'),
		DeviceInfo: () => import('$lib/components/zune/DeviceInfo.svelte'),
		LibraryFolders: () => import('$lib/components/zune/LibraryFolders.svelte'),
		Welcome: () => import('$lib/components/zune/Welcome.svelte'),
		VideosView: () => import('$lib/components/zune/VideosView.svelte'),
		PicturesView: () => import('$lib/components/zune/PicturesView.svelte'),
		PodcastsView: () => import('$lib/components/zune/PodcastsView.svelte')
	};
	const preloadViews = () => Object.values(views).forEach((load) => load().catch(() => {}));

	const PIVOTS = ['collection', 'device', 'social', 'settings'] as const;
	const SUBPIVOTS = ['music', 'videos', 'pictures', 'podcasts'] as const;
	const DEVICE_SUBPIVOTS = ['info', ...SUBPIVOTS] as const;

	type PivotName = (typeof PIVOTS)[number];
	let pivot = $state<PivotName>('collection');
	// Pages slide in from the side their pivot lies on. The kit's Pivot sets
	// that at a click; this does it when a menu or a shortcut changes the pivot.
	function setPivot(p: PivotName) {
		if (p === pivot) return;
		setEnterDirection(PIVOTS, pivot, p);
		// The device pivot opens on its info page; info only exists there.
		if (p === 'device') subpivot = 'info';
		else if (subpivot === 'info') subpivot = 'music';
		pivot = p;
	}
	let subpivot = $state<(typeof DEVICE_SUBPIVOTS)[number]>('music');

	let device = $state<DeviceStatus | null>(null);
	let albums = $state<Album[]>([]);
	let deviceAlbums = $state<Album[]>([]);
	let videos = $state<VideoItem[]>([]);
	let pictureAlbums = $state<PictureAlbum[]>([]);
	let podcasts = $state<Album[]>([]);
	let fullscreen = $state(false);
	// Sending songs to the Zune.
	let sync = $state<SyncStatus | null>(null);
	let syncError = $state<string | null>(null);
	// Show the result in the device bar for a few seconds after a run.
	let syncResultUntil = $state(0);
	let clock = $state(Date.now());
	// First launch: setup covers the window until it is finished or skipped.
	let showSetup = $state(false);
	let artist = $state<string | null>(null);
	let selectedId = $state<string | null>(null);

	let backend = $state<Backend>('mock');
	let api = $state.raw<Api | null>(null);
	let player = $state<PlayerState>({ status: 'stopped', trackId: null, positionMs: 0, durationMs: 0, volume: 0.7, queue: [] });

	onMount(() => {
		const timers: ReturnType<typeof setInterval>[] = [];
		(async () => {
			const c = await connect();
			api = c.api;
			backend = c.backend;
			// The library is local and answers at once. The device can take
			// seconds to connect, so nothing waits on it.
			c.api.player.state().then((p) => {
				player = p;
				volume = p.volume;
			});
			const library = reloadLibrary(c.api);
			const setup = c.api.setup
				.status()
				.then((s) => (showSetup = !s.done || location.hash === '#welcome'))
				.catch((e) => console.error('setup', e));
			(async () => {
				device = await c.api.device();
				if (device.connected) await reloadDevice(c.api);
				// Notice the Zune being plugged in or out; reload what's on it when it changes.
				timers.push(
					setInterval(async () => {
						const was = device?.connected;
						device = await c.api.device();
						if (device.connected !== was) await reload(c.api);
					}, 3000)
				);
				// Sync progress: quick while sending, slow otherwise.
				let syncTick = 0;
				timers.push(
					setInterval(async () => {
						clock = Date.now();
						if (!sync?.active && syncTick++ % 5 !== 0) return;
						const was = sync?.active;
						sync = await c.api.devices.syncStatus();
						if (was && !sync.active) {
							syncResultUntil = Date.now() + 6000;
							device = await c.api.device();
							await reload(c.api);
						}
					}, 600)
				);
				// Folders are watched; when a scan finishes, show what it found.
				let scanning = false;
				timers.push(
					setInterval(async () => {
						const s = await c.api.media.scanStatus();
						if (scanning && !s.running) await reloadLibrary(c.api);
						scanning = s.running;
					}, 2000)
				);
			})();
			try {
				await Promise.all([library.catch((e) => console.error('library', e)), setup]);
				// Deep links for development: #device, #now-playing.
				if (location.hash === '#device') setPivot('device');
				if (location.hash === '#social') setPivot('social');
				if (location.hash === '#settings') setPivot('settings');
				if (location.hash === '#videos') subpivot = 'videos';
				if (location.hash === '#pictures') subpivot = 'pictures';
				if (location.hash === '#now-playing') fullscreen = true;
			} finally {
				await reveal();
			}
			timers.push(setInterval(async () => (player = await c.api.player.state()), 500));
		})();
		// The macOS menu bar sends its items here.
		let unlisten: (() => void) | undefined;
		let gone = false;
		if (isTauri) {
			import('@tauri-apps/api/event').then(async ({ listen }) => {
				const u = await listen<string>('menu', (e) => onMenu(e.payload));
				if (gone) u();
				else unlisten = u;
			});
		}
		return () => {
			gone = true;
			unlisten?.();
			timers.forEach(clearInterval);
		};
	});

	async function onMenu(id: string) {
		if (!api) return;
		const a = api;
		if (id === 'setup') {
			fullscreen = false;
			showSetup = true;
			return;
		}
		// Everything else waits until setup is done.
		if (showSetup) return;
		switch (id) {
			case 'settings':
				fullscreen = false;
				setPivot('settings');
				break;
			case 'view-collection':
			case 'view-device':
			case 'view-social':
				fullscreen = false;
				setPivot(id.slice(5) as PivotName);
				break;
			case 'add-folder': {
				const path = await pickFolder('add a music folder');
				if (!path) return;
				await a.media.addFolder(path, 'music');
				fullscreen = false;
				setPivot('settings');
				break;
			}
			case 'rescan':
				await a.media.rescan();
				break;
			case 'now-playing':
				fullscreen = !fullscreen && !!nowPlaying;
				break;
			case 'play-pause':
				await act((x) => x.player.toggle())();
				break;
			case 'next':
				await act((x) => x.player.next())();
				break;
			case 'previous':
				await act((x) => x.player.prev())();
				break;
			case 'volume-up':
			case 'volume-down':
				volume = Math.round(Math.max(0, Math.min(1, volume + (id === 'volume-up' ? 0.1 : -0.1))) * 100) / 100;
				setVolume(volume);
				break;
			case 'go-to-current':
				if (nowPlaying) await show(nowPlaying.a.artist, nowPlaying.a.id);
				break;
		}
	}

	// The splash (app.html) covers the window until the first page is drawn.
	let ready = $state(false);
	/** Draws the first page under the splash, then lifts the splash once the
	 *  page's fonts are in, so the app never shows blank or half styled. */
	async function reveal() {
		// First launch opens on setup; have it loaded before the splash lifts.
		if (showSetup) await views.Welcome().catch(() => {});
		// The page waits for the splash's minimum time, so its entrance
		// plays as the splash lifts and not unseen behind it.
		await splashMinimum();
		ready = true;
		await tick();
		await Promise.race([document.fonts.ready, new Promise((r) => setTimeout(r, 300))]);
		hideSplash();
		// app.html shows the window when the splash is up. Again here, in case that failed.
		if (isTauri) import('@tauri-apps/api/window').then(({ getCurrentWindow }) => getCurrentWindow().show()).catch(() => {});
		setTimeout(preloadViews, 800);
	}

	// Each list is shown as soon as it arrives.
	function reloadLibrary(a: Api) {
		return Promise.all([
			a.collection().then((v) => (albums = v)),
			a.media.videos().then((v) => (videos = v)),
			a.media.pictures().then((v) => (pictureAlbums = v)),
			a.media.podcasts().then((v) => (podcasts = v))
		]);
	}
	async function reloadDevice(a: Api) {
		deviceAlbums = await a.deviceCollection();
	}
	async function reload(a: Api) {
		await Promise.all([reloadLibrary(a), reloadDevice(a)]);
	}

	/** Shows an artist (and optionally one of their albums) in the collection. */
	async function show(artistName: string, albumId?: string) {
		fullscreen = false;
		setPivot('collection');
		subpivot = 'music';
		artist = artistName;
		selectedId = albumId ?? null;
		await tick();
		if (albumId) document.getElementById(`album-${albumId}`)?.scrollIntoView({ block: 'nearest' });
	}

	// Track id -> track and album, for the now-playing line.
	const byId = $derived(new Map([...albums, ...podcasts].flatMap((a) => a.tracks.map((t) => [t.id, { t, a }] as const))));
	const nowPlaying = $derived(player.trackId !== null ? byId.get(player.trackId) : undefined);

	async function playAlbum(al: Album, start = 0) {
		// Device-only tracks have negative ids and are not on this computer.
		const ids = al.tracks.map((t) => t.id).filter((x): x is number => x !== undefined && x > 0);
		if (api && ids.length) player = await api.player.play(ids, start);
	}
	// ------------------------------------------------- sending to the zune
	async function addToDevice(tracks: Track[]) {
		if (!api) return;
		syncError = null;
		try {
			sync = await api.devices.sync(localIds(tracks));
		} catch (e) {
			syncError = String(e instanceof Error ? e.message : e);
		}
	}
	const pendingSet = $derived(new Set(sync?.active ? sync.pending : []));
	const sendingId = $derived(sync?.active ? (sync.pending[0] ?? null) : null);
	/** How far along an album is, if any of it is queued. */
	function albumSync(al: Album): { done: number; total: number } | null {
		const ids = localIds(al.tracks);
		const left = ids.filter((id) => pendingSet.has(id)).length;
		if (!left) return null;
		const notOnDevice = al.tracks.filter((t) => t.id !== undefined && t.id > 0 && !t.onDevice).length;
		return { done: Math.max(0, notOnDevice - left), total: notOnDevice };
	}
	const canSend = $derived(!!device?.connected);

	// ------------------------------------------------ removing from the zune
	// Every removal asks first, in one dialog.
	let removal = $state<{ ids: number[]; what: string; count: number } | null>(null);
	let removing = $state(false);
	/** Songs of `tracks` that are on the Zune (device rows always are). */
	const onZune = (tracks: Track[]) => tracks.filter((t) => t.id !== undefined && (t.id < 0 || t.onDevice));
	function askRemove(tracks: Track[], what: string) {
		const t = onZune(tracks);
		if (t.length) removal = { ids: t.map((x) => x.id!), what, count: t.length };
	}
	async function confirmRemove() {
		if (!api || !removal) return;
		removing = true;
		syncError = null;
		try {
			await api.devices.remove(removal.ids);
			removal = null;
			device = await api.device();
			await reload(api);
		} catch (e) {
			syncError = String(e instanceof Error ? e.message : e);
			removal = null;
		} finally {
			removing = false;
		}
	}
	const busy = $derived(!!sync?.active || removing);

	// ------------------------------------------------- right-click menus
	const localIds = (tracks: Track[]) => tracks.map((t) => t.id).filter((x): x is number => x !== undefined && x > 0);
	async function playIds(ids: number[], start = 0) {
		if (api && ids.length) player = await api.player.play(ids, start);
	}
	function shuffled<T>(a: T[]): T[] {
		const b = [...a];
		for (let i = b.length - 1; i > 0; i--) {
			const j = Math.floor(Math.random() * (i + 1));
			[b[i], b[j]] = [b[j], b[i]];
		}
		return b;
	}
	const copy = (text: string) => navigator.clipboard?.writeText(text);
	const firstPath = (tracks: Track[]) => tracks.find((t) => t.path)?.path ?? null;

	function artistMenu(name: string): MenuEntry[] {
		const ids = localIds(visible.filter((a) => a.artist === name).flatMap((a) => a.tracks));
		return [
			{ label: 'play', onSelect: () => playIds(ids), disabled: !ids.length },
			{ label: 'shuffle', onSelect: () => playIds(shuffled(ids)), disabled: !ids.length },
			'separator',
			{ label: 'add to device', onSelect: () => addToDevice(visible.filter((a) => a.artist === name).flatMap((a) => a.tracks)), disabled: !ids.length || !canSend || pivot === 'device' },
			{ label: 'remove from device', destructive: true, onSelect: () => askRemove(visible.filter((a) => a.artist === name).flatMap((a) => a.tracks), name), disabled: !canSend || busy || !onZune(visible.filter((a) => a.artist === name).flatMap((a) => a.tracks)).length },
			'separator',
			{ label: 'copy name', onSelect: () => copy(name) }
		];
	}
	function albumMenu(al: Album): MenuEntry[] {
		const ids = localIds(al.tracks);
		const path = firstPath(al.tracks);
		return [
			{ label: 'play', onSelect: () => playAlbum(al), disabled: !ids.length },
			{ label: 'shuffle', onSelect: () => playIds(shuffled(ids)), disabled: !ids.length },
			'separator',
			{ label: 'add to device', onSelect: () => addToDevice(al.tracks), disabled: !ids.length || !canSend || al.tracks.every((t) => t.onDevice) || pivot === 'device' },
			{ label: 'remove from device', destructive: true, onSelect: () => askRemove(al.tracks, al.title), disabled: !canSend || busy || !onZune(al.tracks).length },
			'separator',
			{ label: 'go to artist', onSelect: () => show(al.artist) },
			{ label: revealLabel, onSelect: () => path && revealFile(path), disabled: !path || !isTauri },
			'separator',
			{ label: 'copy title', onSelect: () => copy(`${al.artist} — ${al.title}`) }
		];
	}
	function trackMenu(al: Album, i: number): MenuEntry[] {
		const t = al.tracks[i];
		const local = t.id !== undefined && t.id > 0;
		return [
			{ label: 'play from here', onSelect: () => playAlbum(al, i), disabled: !local },
			{ label: 'play only this song', onSelect: () => playIds([t.id!]), disabled: !local },
			{ label: 'add to device', onSelect: () => addToDevice([t]), disabled: !local || !canSend || t.onDevice || pivot === 'device' },
			{ label: 'remove from device', destructive: true, onSelect: () => askRemove([t], t.title), disabled: !canSend || busy || !onZune([t]).length },
			'separator',
			{ label: 'go to artist', onSelect: () => show(al.artist) },
			{ label: revealLabel, onSelect: () => t.path && revealFile(t.path), disabled: !t.path || !isTauri },
			'separator',
			{ label: 'copy title', onSelect: () => copy(`${t.artist} — ${t.title}`) }
		];
	}

	const act = (f: (a: Api) => Promise<PlayerState>) => async () => {
		if (api) player = await f(api);
	};
	// The slider owns the volume while you use it; it starts from the player's.
	let volume = $state(0.7);
	// Send the newest value as fast as the backend takes it: one request in
	// flight at a time, always finishing on the latest value. (A debounce here
	// made the volume lag behind the slider.)
	let volumeBusy = false;
	let volumeNext: number | null = null;
	async function setVolume(v: number) {
		volumeNext = v;
		if (volumeBusy) return;
		volumeBusy = true;
		try {
			while (volumeNext !== null) {
				const x = volumeNext;
				volumeNext = null;
				await api?.player.volume(x);
			}
		} finally {
			volumeBusy = false;
		}
	}

	function onKey(e: KeyboardEvent) {
		if (showSetup) return;
		// A video or picture viewer handles its own keys.
		if (document.querySelector('[aria-label="video player"], [aria-label="picture viewer"]')) return;
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

<!-- Drawn once the library has answered, so the page enters complete as the splash lifts. -->
{#if ready}
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
			<SquirtMark size={14} /><span class="text-[0.95rem] leading-none">squirt</span>
			{#if backend === 'mock'}<span class="font-normal text-faint">· browser preview, mock data (run <code class="font-mono">squirt serve</code> for your library)</span>
			{:else if backend === 'dev-api'}<span class="font-normal text-faint">· browser preview, your library</span>{/if}
		</div>
		<Pivot items={PIVOTS} bind:value={() => pivot, setPivot} />
		{#key pivot}
			{#if pivot === 'collection' || pivot === 'device'}
				<div class="fk-enter">
					<Pivot items={pivot === 'device' ? DEVICE_SUBPIVOTS : SUBPIVOTS} bind:value={subpivot} size="subpivot" disabled={pivot === 'device' ? ['videos', 'pictures', 'podcasts'] : []} />
				</div>
			{/if}
		{/key}
	</header>

	<main class="flex min-h-0 flex-1 px-6 pt-5 pb-3">
		{#key `${pivot}/${subpivot}`}
		<!-- Each page slides in from the side its pivot or subpivot lies on. -->
		<div class="flex min-h-0 min-w-0 flex-1 gap-0">
		{#if pivot === 'social'}
			{#if api}{#await views.Social() then { default: Social }}<Social social={api.social} />{/await}{/if}
		{:else if pivot === 'settings'}
			<section class="flex min-w-0 flex-1 gap-14 overflow-y-auto pr-2">
			<div class="flex w-[34rem] max-w-full shrink-0 flex-col gap-3">
				<h2 class="subpivot fk-enter" style="--i: 0">library folders</h2>
				{#if api}{#await views.LibraryFolders() then { default: LibraryFolders }}<LibraryFolders media={api.media} onScanned={() => api && reload(api)} />{/await}{/if}
			</div>
			<div class="flex max-w-md flex-col gap-8">
				<div class="fk-enter flex flex-col gap-3" style="--i: 1">
					<h2 class="subpivot">theme</h2>
					<div class="flex gap-2">
						{#each THEMES as t (t)}
							<Button variant={theme.value === t ? 'default' : 'outline'} size="sm" class="lowercase" onclick={() => theme.set(t)}>{t}</Button>
						{/each}
					</div>
				</div>
				<div class="fk-enter flex flex-col gap-2" style="--i: 2">
					<h2 class="subpivot">mtpz keys</h2>
					<p class="text-muted-foreground">read from <code class="font-mono text-foreground">~/.mtpz-data</code>. squirt doesn't include keys.</p>
				</div>
				<div class="fk-enter flex flex-col gap-3" style="--i: 2">
					<h2 class="subpivot">setup</h2>
					<Button size="sm" variant="outline" class="w-fit lowercase" onclick={() => (showSetup = true)}>run setup again</Button>
				</div>
				<div class="fk-enter flex flex-col gap-2" style="--i: 3">
					<h2 class="subpivot">about</h2>
					<p class="text-muted-foreground">squirt 0.1 · mit license</p>
					<p class="text-xs leading-relaxed text-muted-foreground">
						includes <button class="underline underline-offset-2 hover:text-foreground" onclick={() => openExternal('https://ffmpeg.org')}>ffmpeg</button>
						(<button class="underline underline-offset-2 hover:text-foreground" onclick={() => openExternal('https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html')}>lgpl 2.1</button>,
						<button class="underline underline-offset-2 hover:text-foreground" onclick={() => openExternal('https://ffmpeg.org/releases/ffmpeg-7.1.2.tar.xz')}>source</button>)
						and the selawik and ibm plex mono fonts (ofl).
					</p>
				</div>
			</div>
			</section>
		{:else if pivot === 'collection' && subpivot === 'videos'}
			{#if api}{#await views.VideosView() then { default: VideosView }}<VideosView media={api.media} {videos} onPlay={() => { if (player.status === 'playing') act((a) => a.player.toggle())(); }} />{/await}{/if}
		{:else if pivot === 'collection' && subpivot === 'pictures'}
			{#if api}{#await views.PicturesView() then { default: PicturesView }}<PicturesView media={api.media} albums={pictureAlbums} />{/await}{/if}
		{:else if pivot === 'collection' && subpivot === 'podcasts'}
			{#await views.PodcastsView() then { default: PodcastsView }}<PodcastsView shows={podcasts} playingId={player.trackId} onPlay={(s, i) => playAlbum(s, i)} />{/await}
		{:else if pivot === 'device' && subpivot === 'info'}
			{#if api}{#await views.DeviceInfo() then { default: DeviceInfo }}<DeviceInfo devices={api.devices} onChange={async () => { if (api) { device = await api.device(); await reload(api); } }} />{/await}{/if}
		{:else if pivot === 'device' && !device?.connected}
			<section class="fk-enter flex flex-col gap-2" style="--i: 1">
				<p class="subpivot text-muted-foreground">{device?.problem ? 'your zune is connected but squirt cannot use it' : 'connect your zune'}</p>
				{#if device?.problem}<p class="text-muted-foreground">{device.problem}</p>{/if}
			</section>
		{:else}
			<!-- artists -->
			<section class="fk-enter flex w-52 shrink-0 flex-col" style="--i: 1">
				<h3 class="colhead mb-2">artist</h3>
				<ScrollArea class="min-h-0 flex-1 pr-3">
					<button class="row {artist === null ? 'row-on' : ''}" onclick={() => (artist = null)}>all</button>
					{#each artists as a (a)}
						<RowMenu items={() => artistMenu(a)}>
							<button class="row {artist === a ? 'row-on' : ''}" onclick={() => { artist = a; selectedId = null; }}>{a}</button>
						</RowMenu>
					{/each}
				</ScrollArea>
			</section>

			<!-- albums -->
			<section class="fk-enter flex min-w-0 flex-1 flex-col border-l pl-4" style="--i: 2">
				<h3 class="colhead mb-2">album</h3>
				<ScrollArea class="min-h-0 flex-1 pr-3">
					{#each shownAlbums as al (al.id)}
						<RowMenu items={() => albumMenu(al)}>
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
						</RowMenu>
					{/each}
				</ScrollArea>
			</section>

			<!-- songs -->
			<section class="fk-enter flex w-[44%] shrink-0 flex-col border-l pl-4" style="--i: 3">
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
							{@const progress = albumSync(selected)}
							{#if selectedOnDevice}
								<span class="flex items-center gap-1.5 pb-1 text-xs text-muted-foreground lowercase"><Check size={14} />on device</span>
								<Button size="sm" variant="ghost" class="lowercase text-muted-foreground hover:text-destructive" disabled={busy} onclick={() => askRemove(selected!.tracks, selected!.title)}>remove</Button>
							{:else if progress}
								<!-- This album is being sent: count and a thin bar. -->
								<div class="flex w-32 flex-col gap-1 pb-1">
									<span class="text-xs lowercase">adding <span class="tabular">{progress.done}</span> of <span class="tabular">{progress.total}</span>…</span>
									<div class="h-1 bg-muted"><div class="zune-gradient h-full transition-[width] duration-300" style:width="{progress.total ? (progress.done / progress.total) * 100 : 0}%"></div></div>
								</div>
							{:else}
								<Button size="sm" class="lowercase" disabled={!canSend} title={canSend ? undefined : 'connect your zune first'} onclick={() => addToDevice(selected!.tracks)}>add to device</Button>
							{/if}
						{:else if pivot === 'device'}
							<Button size="sm" variant="outline" class="lowercase hover:text-destructive" disabled={busy} onclick={() => askRemove(selected!.tracks, selected!.title)}>remove from device</Button>
						{/if}
					</div>
					{#if syncError}<p class="-mt-2 mb-3 text-xs text-destructive">{syncError}</p>{/if}
					<div class="grid grid-cols-[2.5rem_1fr_3.5rem_1.25rem] gap-x-3 px-1.5 pb-1.5">
						<span class="colhead">#</span><span class="colhead">song</span><span class="colhead text-right">length</span><span></span>
					</div>
					<ScrollArea class="min-h-0 flex-1">
						{#each selected.tracks as t, i (i)}
							{#if multiDisc && (i === 0 || selected.tracks[i - 1].disc !== t.disc)}
								<div class="colhead px-1.5 pt-3 pb-1">disc {t.disc}</div>
							{/if}
							<RowMenu items={() => trackMenu(selected!, i)}>
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div class="grid grid-cols-[2.5rem_1fr_3.5rem_1.25rem] items-center gap-x-3 px-1.5 py-1 hover:bg-accent {isPlaying(t) ? 'text-magenta' : ''}"
								ondblclick={() => playAlbum(selected!, i)}>
								<span class="tabular text-xs {isPlaying(t) ? '' : 'text-muted-foreground'}">
									{#if isPlaying(t)}<SpeakerHigh size={13} weight="fill" class="inline" />{:else}{t.n}{/if}
								</span>
								<span class="truncate">{t.title}</span>
								<span class="tabular text-right text-xs text-muted-foreground">{duration(t.durationMs)}</span>
								{#if pivot === 'collection' && t.id !== undefined && t.id === sendingId}
									<span class="mx-auto size-2 animate-pulse bg-primary" title="sending"></span>
								{:else if pivot === 'collection' && t.id !== undefined && pendingSet.has(t.id)}
									<span class="mx-auto size-1.5 bg-faint" title="waiting to send"></span>
								{:else if pivot === 'collection' && t.onDevice}<ZuneHD size={14} class="text-muted-foreground" />{:else}<span></span>{/if}
							</div>
							</RowMenu>
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
		</div>
		{/key}
	</main>

	<footer class="grid h-24 shrink-0 grid-cols-[1fr_auto_1fr] items-center gap-6 border-t px-6">
		<DeviceBar {device} {sync} showResult={clock < syncResultUntil} />
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
{/if}

<!-- Confirms removing songs from the Zune. -->
<AlertDialog.Root open={removal !== null} onOpenChange={(o) => { if (!o && !removing) removal = null; }}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title class="lowercase">remove from your zune?</AlertDialog.Title>
			<AlertDialog.Description>
				{#if removal}
					{removal.count === 1 ? 'this song' : `${removal.count} songs`} from <span class="text-foreground">{removal.what}</span> will be deleted from the zune. files on this computer aren't touched.
				{/if}
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel class="lowercase" disabled={removing}>cancel</AlertDialog.Cancel>
			<AlertDialog.Action class="lowercase bg-destructive text-white hover:bg-destructive/90" disabled={removing} onclick={confirmRemove}>{removing ? 'removing…' : 'remove'}</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>

{#if showSetup && api}
	{#await views.Welcome() then { default: Welcome }}<Welcome {api} onDone={() => { showSetup = false; setPivot('collection'); subpivot = 'music'; if (api) reload(api); }} />{/await}
{/if}

{#if fullscreen && nowPlaying}
	{#await views.NowPlaying() then { default: NowPlaying }}
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
		spectrum={(n) => api?.player.spectrum(n) ?? Promise.resolve([])}
	/>
	{/await}
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
