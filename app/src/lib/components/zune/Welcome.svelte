<script lang="ts">
	// First launch: a short setup that covers the window. Steps: welcome,
	// library folders, the Zune (and its key file), social, then ready while
	// the first scan runs. Every step but the library can be skipped.
	import { onMount } from 'svelte';
	import { isTauri, openExternal, pickFile, pickFolder, type Api, type DeviceStatus, type MediaKind, type ScanStatus, type SetupStatus, type SocialStatus } from '$lib/api';
	import { bytes } from '$lib/format';
	import { Button } from '$lib/components/ui/button';
	import { Switch } from '$lib/components/ui/switch';
	import { Check, FolderSimple, Plus } from 'phosphor-svelte';
	import Pivot from 'fluorokit/svelte/Pivot.svelte';
	import SquirtMark from './SquirtMark.svelte';
	import ZuneArt from './ZuneArt.svelte';

	let { api, onDone }: { api: Api; onDone: () => void } = $props();

	const STEPS = ['welcome', 'library', 'zune', 'social', 'ready'] as const;
	type Step = (typeof STEPS)[number];
	let step = $state<Step>('welcome');
	let reached = $state(0); // furthest step so far; later ones are disabled
	let dir = $state(1);
	function go(s: Step) {
		const i = STEPS.indexOf(s);
		dir = i >= STEPS.indexOf(step) ? 1 : -1;
		reached = Math.max(reached, i);
		step = s;
	}
	const next = () => go(STEPS[Math.min(STEPS.indexOf(step) + 1, STEPS.length - 1)]);
	const back = () => go(STEPS[Math.max(STEPS.indexOf(step) - 1, 0)]);

	// ------------------------------------------------------------ library
	type Row = { path: string; kind: MediaKind; count: number | null; more: boolean; on: boolean; watched: boolean; partly?: boolean };
	let rows = $state<Row[]>([]);
	let suggestionsLoaded = $state(false);
	let adding = $state(false);
	let libraryError = $state<string | null>(null);

	const KINDS: MediaKind[] = ['music', 'video', 'picture', 'podcast'];
	const KIND_LABEL: Record<MediaKind, string> = { music: 'music', video: 'videos', picture: 'pictures', podcast: 'podcasts' };
	const NOUN: Record<MediaKind, [string, string]> = {
		music: ['song', 'songs'],
		video: ['video', 'videos'],
		picture: ['picture', 'pictures'],
		podcast: ['episode', 'episodes']
	};
	function countLabel(r: Row) {
		if (r.watched) return 'already in your library';
		if (r.partly && !r.on) return 'part of it is already in your library';
		if (r.count === null) return '';
		if (r.count === 0) return `no ${NOUN[r.kind][1]} yet`;
		return `${r.count.toLocaleString()}${r.more ? '+' : ''} ${NOUN[r.kind][r.count === 1 ? 0 : 1]}`;
	}
	const tilde = (p: string) => p.replace(/^\/Users\/[^/]+|^\/home\/[^/]+|^[A-Z]:\\Users\\[^\\]+/, '~');

	async function addOther() {
		const path = await pickFolder('add a folder');
		if (!path || rows.some((r) => r.path === path)) return;
		rows.push({ path, kind: 'music', count: null, more: false, on: true, watched: false });
	}

	async function saveLibrary() {
		adding = true;
		libraryError = null;
		try {
			for (const r of rows.filter((r) => r.on && !r.watched)) {
				await api.media.addFolder(r.path, r.kind);
				r.watched = true;
			}
			next();
		} catch (e) {
			libraryError = String(e instanceof Error ? e.message : e);
		} finally {
			adding = false;
		}
	}
	const chosen = $derived(rows.filter((r) => r.on || r.watched).length);

	// --------------------------------------------------------------- zune
	let setup = $state<SetupStatus | null>(null);
	let device = $state<DeviceStatus | null>(null);
	let keysError = $state<string | null>(null);

	async function chooseKeys() {
		keysError = null;
		const path = await pickFile('choose your mtpz key file');
		if (!path) return;
		try {
			setup = await api.setup.importKeys(path);
			device = await api.device();
		} catch (e) {
			keysError = String(e instanceof Error ? e.message : e);
		}
	}

	// ------------------------------------------------------------- social
	let social = $state<SocialStatus | null>(null);
	let socialBusy = $state(false);
	let socialError = $state<string | null>(null);
	async function runSocial(f: () => Promise<SocialStatus | void>) {
		socialBusy = true;
		socialError = null;
		try {
			const s = await f();
			social = s ?? (await api.social.status());
		} catch (e) {
			socialError = String(e instanceof Error ? e.message : e);
		} finally {
			socialBusy = false;
		}
	}
	const connectLastfm = () =>
		runSocial(async () => {
			await openExternal(await api.social.beginAuth());
		});

	// -------------------------------------------------------------- ready
	let scan = $state<ScanStatus | null>(null);
	let finishing = $state(false);
	async function finish() {
		finishing = true;
		try {
			await api.setup.finish();
		} finally {
			onDone();
		}
	}

	onMount(() => {
		api.setup.status().then((s) => (setup = s));
		api.setup.suggestions().then((list) => {
			rows = list.map((s) => ({ ...s, count: s.count, on: s.count > 0 && !s.watched && !s.partly }));
			suggestionsLoaded = true;
		});
		api.social.status().then((s) => (social = s));
		const t = setInterval(async () => {
			// Only poll what the current step shows.
			if (step === 'zune') device = await api.device();
			if (step === 'ready') scan = await api.media.scanStatus();
			if (step === 'social' && social?.lastfm.authPending) social = await api.social.status();
		}, 1500);
		return () => clearInterval(t);
	});
	$effect(() => {
		if (step === 'zune' && !device) api.device().then((d) => (device = d));
		if (step === 'ready' && !scan) api.media.scanStatus().then((s) => (scan = s));
	});

	function onKey(e: KeyboardEvent) {
		if (e.key !== 'Enter' || e.target instanceof HTMLButtonElement || e.target instanceof HTMLInputElement) return;
		if (step === 'library') saveLibrary();
		else if (step === 'ready') finish();
		else next();
	}

	const macApp = isTauri && typeof navigator !== 'undefined' && navigator.userAgent.includes('Mac');
	const enter = $derived(`--fk-enter-dir: ${dir}`);
	const progress = $derived(scan && scan.total > 0 ? Math.min(1, scan.done / scan.total) : 0);
</script>

<svelte:window onkeydown={onKey} />

<div class="fixed inset-0 z-50 flex flex-col bg-background" role="dialog" aria-label="set up squirt">
	<header class="flex shrink-0 flex-col gap-3 px-6 {macApp ? 'pt-0' : 'pt-5'}" data-tauri-drag-region>
		<!-- Same brand row as the main window, so nothing jumps when setup closes. -->
		<div class="flex items-center gap-2.5 text-xs font-semibold tracking-wide text-muted-foreground lowercase {macApp ? 'pointer-events-none relative top-[3.5px] -mb-1 h-8 pl-[66px]' : ''}">
			<SquirtMark size={14} /><span class="text-[0.95rem] leading-none">squirt</span>
			<span class="font-normal text-faint">· setup</span>
		</div>
		<div class="flex justify-center"><Pivot items={STEPS} bind:value={() => step, go} disabled={STEPS.filter((_, i) => i > reached)} /></div>
	</header>

	<main class="flex min-h-0 flex-1 overflow-y-auto px-6 py-8">
		{#key step}
			<!-- One centered column; welcome and ready center their text too. -->
			<div class="m-auto flex w-full {step === 'zune' ? 'max-w-3xl' : 'max-w-2xl'} flex-col {step === 'welcome' || step === 'ready' ? 'items-center text-center' : ''}" style={enter}>
				{#if step === 'welcome'}
					<div class="mb-10"><SquirtMark size={128} animate /></div>
					<h1 class="fk-enter text-[4.25rem] leading-[1.05] font-light lowercase" style="--i: 1; animation-delay: 0.9s">welcome to squirt.</h1>
					<p class="fk-enter mt-5 max-w-lg text-lg text-muted-foreground" style="--i: 2; animation-delay: 1s">
						your music, videos and pictures, on your computer and your zune. setup takes about a minute.
					</p>

				{:else if step === 'library'}
					<h1 class="fk-enter text-[3rem] leading-tight font-light lowercase" style="--i: 0">where's your music?</h1>
					<p class="fk-enter mt-3 max-w-xl text-muted-foreground" style="--i: 1">
						squirt watches these folders and picks up anything you add later. it never moves or changes your files.
					</p>
					<div class="fk-enter mt-8 flex flex-col" style="--i: 2">
						{#if !suggestionsLoaded}
							<p class="text-muted-foreground lowercase">looking for your folders…</p>
						{/if}
						{#each rows as r (r.path)}
							<div class="flex items-center gap-4 border-b py-3">
								<button
									class="flex size-5 shrink-0 items-center justify-center border transition-colors {r.on || r.watched ? 'border-primary bg-primary text-primary-foreground' : 'border-input hover:border-muted-foreground'}"
									role="checkbox"
									aria-checked={r.on || r.watched}
									aria-label="watch {r.path}"
									disabled={r.watched}
									onclick={() => (r.on = !r.on)}
								>
									{#if r.on || r.watched}<Check size={13} weight="bold" />{/if}
								</button>
								<FolderSimple size={18} class="shrink-0 text-muted-foreground" />
								<div class="flex min-w-0 flex-1 flex-col">
									<span class="truncate font-mono text-sm">{tilde(r.path)}</span>
									{#if countLabel(r)}<span class="text-xs text-muted-foreground">{countLabel(r)}</span>{/if}
								</div>
								<!-- What the folder holds. -->
								<div class="flex shrink-0 gap-3" role="radiogroup" aria-label="kind">
									{#each KINDS as k (k)}
										<button
											role="radio"
											aria-checked={r.kind === k}
											disabled={r.watched}
											class="text-sm lowercase transition-colors {r.kind === k ? 'text-foreground' : 'text-faint hover:text-muted-foreground'}"
											onclick={() => { r.kind = k; r.count = null; r.on = true; }}
										>{KIND_LABEL[k]}</button>
									{/each}
								</div>
							</div>
						{/each}
						<Button size="sm" variant="outline" class="mt-4 w-fit lowercase" onclick={addOther}><Plus />add another folder</Button>
						{#if libraryError}<p class="mt-3 text-xs text-destructive">{libraryError}</p>{/if}
					</div>

				{:else if step === 'zune'}
					<div class="flex items-center justify-center gap-14">
						<div class="fk-enter shrink-0" style="--i: 0">
							<ZuneArt height={380} lit={device?.connected ?? false} kind={device?.kind} finish={device?.finish} />
						</div>
						<div class="flex min-w-0 max-w-lg flex-col pt-2">
							{#if setup && !setup.keys}
								<h1 class="fk-enter text-[3rem] leading-tight font-light lowercase" style="--i: 1">one file first.</h1>
								<p class="fk-enter mt-3 text-muted-foreground" style="--i: 2">
									a zune only talks to software that signs in with a key. squirt doesn't include one. choose a key file in the
									<code class="font-mono text-foreground">.mtpz-data</code> format and squirt will copy it to
									<code class="font-mono text-foreground">{setup.keysPath}</code>.
								</p>
								<div class="fk-enter mt-6 flex flex-col gap-2" style="--i: 3">
									<Button size="sm" class="w-fit lowercase" onclick={chooseKeys}>choose key file</Button>
									{#if keysError ?? setup.keysError}<p class="text-xs text-destructive">{keysError ?? setup.keysError}</p>{/if}
								</div>
							{:else if device?.connected}
								<h1 class="fk-enter text-[3rem] leading-tight font-light lowercase" style="--i: 1">hi, {device.name || 'zune'}.</h1>
								<p class="fk-enter mt-3 text-muted-foreground lowercase" style="--i: 2">
									{device.finish ? `${device.finish} ` : ''}{device.model} · <span class="tabular">{bytes(device.free)}</span> free of <span class="tabular">{bytes(device.capacity)}</span>
								</p>
								<p class="fk-enter mt-6 text-muted-foreground" style="--i: 3">send albums to it from your collection, and see what's on it under device.</p>
							{:else if device?.problem}
								<h1 class="fk-enter text-[3rem] leading-tight font-light lowercase" style="--i: 1">almost.</h1>
								<p class="fk-enter mt-3 text-muted-foreground" style="--i: 2">your zune is plugged in, but squirt can't use it yet:</p>
								<p class="fk-enter mt-2 font-mono text-xs text-muted-foreground" style="--i: 3">{device.problem}</p>
							{:else}
								<h1 class="fk-enter text-[3rem] leading-tight font-light lowercase" style="--i: 1">plug in your zune.</h1>
								<p class="fk-enter mt-3 text-muted-foreground" style="--i: 2">connect it with a usb cable and unlock it. squirt finds it on its own.</p>
								<p class="fk-enter mt-6 flex items-center gap-2 text-xs text-muted-foreground lowercase" style="--i: 3">
									<span class="inline-block size-1.5 animate-pulse bg-primary"></span>looking for your zune…
								</p>
							{/if}
						</div>
					</div>

				{:else if step === 'social'}
					<h1 class="fk-enter text-[3rem] leading-tight font-light lowercase" style="--i: 0">share what you play.</h1>
					<p class="fk-enter mt-3 max-w-xl text-muted-foreground" style="--i: 1">both are optional. you can change them later under social.</p>
					<div class="mt-8 flex flex-col">
						<div class="fk-enter flex items-center gap-6 border-b py-4" style="--i: 2">
							<div class="flex min-w-0 flex-1 flex-col">
								<span class="subpivot">last.fm</span>
								<span class="text-xs text-muted-foreground">
									{#if !social}…
									{:else if !social.lastfm.available}not available in this build.
									{:else if social.lastfm.connected}signed in as <span class="text-foreground">{social.lastfm.username}</span>. scrobbling what you play.
									{:else if social.lastfm.authPending}approve squirt in your browser, then come back.
									{:else}scrobble what you play to your profile.{/if}
								</span>
							</div>
							{#if social?.lastfm.available && !social.lastfm.connected}
								{#if social.lastfm.authPending}
									<Button size="sm" class="lowercase" disabled={socialBusy} onclick={() => runSocial(() => api.social.finishAuth())}>i've approved it</Button>
								{:else}
									<Button size="sm" variant="outline" class="lowercase" disabled={socialBusy} onclick={connectLastfm}>connect</Button>
								{/if}
							{:else if social?.lastfm.connected}
								<Check size={18} class="text-primary" />
							{/if}
						</div>
						<label class="fk-enter flex items-center gap-6 border-b py-4" style="--i: 3">
							<span class="flex min-w-0 flex-1 flex-col">
								<span class="subpivot">discord</span>
								<span class="text-xs text-muted-foreground">show what you're playing on your discord profile.</span>
							</span>
							<Switch checked={social?.discord.enabled ?? false} disabled={!social || socialBusy} onCheckedChange={(on) => runSocial(() => api.social.setDiscordEnabled(on))} />
						</label>
						{#if socialError}<p class="mt-3 text-xs text-destructive">{socialError}</p>{/if}
					</div>

				{:else}
					<h1 class="fk-enter text-[4.25rem] leading-[1.05] font-light lowercase" style="--i: 0">you're set.</h1>
					<div class="fk-enter mt-8 flex w-full max-w-md flex-col gap-2" style="--i: 1">
						{#if scan?.running}
							<p class="text-muted-foreground lowercase">
								reading your library… <span class="tabular text-foreground">{scan.done.toLocaleString()}</span> of <span class="tabular">{scan.total.toLocaleString()}</span>
							</p>
							<div class="h-1 w-full bg-muted"><div class="h-full transition-[width] duration-500" style="width: {progress * 100}%; background: var(--zune-gradient)"></div></div>
							<p class="text-xs text-muted-foreground">you can start now; the rest shows up as it's read.</p>
						{:else if chosen === 0}
							<p class="text-muted-foreground">no folders yet. add them any time under settings.</p>
						{:else}
							<p class="text-muted-foreground">your library is ready.</p>
						{/if}
					</div>
					<!-- What setup did, in one place. -->
					<dl class="fk-enter mt-10 grid w-full max-w-md grid-cols-[7rem_1fr] items-baseline gap-x-6 gap-y-2.5 text-left" style="--i: 2">
						<dt class="colhead">library</dt>
						<dd class="min-w-0 truncate font-mono text-xs">
							{rows.filter((r) => r.watched).map((r) => tilde(r.path)).join(', ') || '—'}
						</dd>
						<dt class="colhead">zune</dt>
						<dd class="lowercase">{device?.connected ? device.name || device.model : 'not connected'}</dd>
						<dt class="colhead">last.fm</dt>
						<dd class="lowercase">{social?.lastfm.connected ? social.lastfm.username : 'off'}</dd>
						<dt class="colhead">discord</dt>
						<dd class="lowercase">{social?.discord.enabled ? 'on' : 'off'}</dd>
					</dl>
				{/if}
			</div>
		{/key}
	</main>

	<footer class="flex shrink-0 items-center gap-3 border-t px-6 py-4">
		{#if step === 'welcome'}
			<Button variant="ghost" size="sm" class="lowercase text-muted-foreground" disabled={finishing} onclick={finish}>skip setup</Button>
		{:else}
			<Button variant="ghost" size="sm" class="lowercase" onclick={back}>back</Button>
		{/if}
		<span class="ml-auto"></span>
		{#if step === 'welcome'}
			<Button size="sm" class="lowercase" onclick={next}>get started</Button>
		{:else if step === 'library'}
			<Button size="sm" class="lowercase" disabled={adding || !suggestionsLoaded} onclick={saveLibrary}>{chosen ? 'continue' : 'skip for now'}</Button>
		{:else if step === 'zune'}
			<Button size="sm" variant={device?.connected ? 'default' : 'outline'} class="lowercase" onclick={next}>{device?.connected ? 'continue' : 'skip for now'}</Button>
		{:else if step === 'social'}
			<Button size="sm" class="lowercase" onclick={next}>continue</Button>
		{:else}
			<Button size="sm" class="lowercase" disabled={finishing} onclick={finish}>start listening</Button>
		{/if}
	</footer>
</div>
