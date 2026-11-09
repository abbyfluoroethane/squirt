<script lang="ts">
	// The social pivot: your Last.fm profile and recent listening, what this
	// session scrobbled, and the Last.fm / Discord connections.
	import { onMount } from 'svelte';
	import { openExternal, type LastFmProfile, type SocialApi, type SocialStatus } from '$lib/api';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { ArrowSquareOut, MusicNotesSimple } from 'phosphor-svelte';

	let { social }: { social: SocialApi } = $props();

	let status = $state<SocialStatus | null>(null);
	let profile = $state<LastFmProfile | null>(null);
	let profileError = $state<string | null>(null);
	let busy = $state(false);
	let actionError = $state<string | null>(null);

	let keyInput = $state('');
	let secretInput = $state('');
	let discordInput = $state('');

	async function refreshProfile() {
		if (!status?.lastfm.connected) {
			profile = null;
			return;
		}
		try {
			profile = await social.profile();
			profileError = null;
		} catch (e) {
			profileError = String(e instanceof Error ? e.message : e);
		}
	}

	async function refreshStatus() {
		const before = status?.scrobbles.length ?? 0;
		status = await social.status();
		if (!keyInput) keyInput = status.lastfm.apiKey;
		if (!discordInput) discordInput = status.discord.clientId;
		// A new scrobble: refresh recent tracks so it shows up.
		if (status.scrobbles.length !== before) refreshProfile();
	}

	onMount(() => {
		refreshStatus().then(refreshProfile);
		const a = setInterval(refreshStatus, 5000);
		const b = setInterval(refreshProfile, 30000);
		return () => {
			clearInterval(a);
			clearInterval(b);
		};
	});

	async function run(f: () => Promise<SocialStatus | void>) {
		busy = true;
		actionError = null;
		try {
			const s = await f();
			if (s) status = s;
		} catch (e) {
			actionError = String(e instanceof Error ? e.message : e);
		} finally {
			busy = false;
		}
	}

	const saveKeys = () => run(async () => {
		const s = await social.setLastfmKeys(keyInput, secretInput);
		secretInput = '';
		return s;
	});
	const connect = () => run(async () => {
		const url = await social.beginAuth();
		await openExternal(url);
		return social.status();
	});
	const finish = () => run(async () => {
		const s = await social.finishAuth();
		status = s;
		await refreshProfile();
		return s;
	});

	function ago(unix: number | null): string {
		if (!unix) return 'now';
		const s = Math.max(0, Date.now() / 1000 - unix);
		if (s < 60) return 'just now';
		if (s < 3600) return `${Math.floor(s / 60)} min ago`;
		if (s < 86400) return `${Math.floor(s / 3600)} hr ago`;
		return new Date(unix * 1000).toLocaleDateString(undefined, { month: 'short', day: 'numeric' }).toLowerCase();
	}
	const fmt = (n: number) => n.toLocaleString();
</script>

<div class="flex min-h-0 flex-1 gap-8">
	<!-- you -->
	<section class="flex w-64 shrink-0 flex-col gap-6">
		{#if status?.lastfm.connected}
			<div class="flex flex-col gap-1">
				<h3 class="colhead">you</h3>
				<div class="flex items-center gap-3">
					{#if profile?.user.image}
						<img src={profile.user.image} alt="" class="size-14 object-cover" draggable="false" />
					{:else}
						<div class="zune-gradient size-14"></div>
					{/if}
					<div class="flex min-w-0 flex-col">
						<span class="truncate text-[1.6rem] leading-tight font-light lowercase">{status.lastfm.username}</span>
						{#if profile}
							<span class="text-xs text-muted-foreground"><span class="tabular">{fmt(profile.user.playcount)}</span> scrobbles</span>
						{/if}
					</div>
				</div>
				{#if profile}
					<button class="mt-1 flex w-fit items-center gap-1 text-xs text-muted-foreground hover:text-foreground" onclick={() => openExternal(profile!.user.url)}>
						profile on last.fm <ArrowSquareOut size={12} />
					</button>
				{/if}
			</div>
		{/if}

		<div class="flex min-h-0 flex-1 flex-col">
			<h3 class="colhead mb-2">scrobbled this session</h3>
			{#if status?.scrobbles.length}
				<ScrollArea class="min-h-0 flex-1">
					{#each status.scrobbles as s (s.at + s.title)}
						<div class="flex flex-col py-1">
							<span class="truncate">{s.title}</span>
							<span class="truncate text-xs {s.error ? 'text-destructive' : 'text-muted-foreground'}">
								{s.artist} · {s.error ? 'waiting to retry' : ago(s.at)}
							</span>
						</div>
					{/each}
				</ScrollArea>
			{:else}
				<p class="text-muted-foreground">
					{status?.lastfm.connected && status.lastfm.enabled ? 'songs you play here are scrobbled once you have heard half of them.' : 'nothing yet.'}
				</p>
			{/if}
		</div>
	</section>

	<!-- recently played -->
	<section class="flex min-w-0 flex-1 flex-col border-l pl-6">
		<h3 class="colhead mb-2">recently played</h3>
		{#if !status?.lastfm.connected}
			<p class="max-w-md text-muted-foreground">connect last.fm to see what you have been listening to, here and everywhere else you scrobble.</p>
		{:else if profileError}
			<p class="text-destructive">{profileError}</p>
		{:else if profile}
			<ScrollArea class="min-h-0 flex-1 pr-3">
				{#each profile.recent as t, i (i)}
					<button class="flex w-full items-center gap-3 p-1.5 text-left hover:bg-accent" onclick={() => openExternal(t.url)}>
						{#if t.image}
							<img src={t.image} alt="" class="size-11 shrink-0 object-cover" draggable="false" />
						{:else}
							<div class="flex size-11 shrink-0 items-center justify-center bg-muted"><MusicNotesSimple size={16} class="text-muted-foreground" /></div>
						{/if}
						<span class="flex min-w-0 flex-1 flex-col">
							<span class="truncate {t.nowPlaying ? 'text-magenta' : ''}">{t.title}</span>
							<span class="truncate text-xs text-muted-foreground">{t.artist}{t.album ? ` · ${t.album}` : ''}</span>
						</span>
						<span class="shrink-0 text-xs lowercase {t.nowPlaying ? 'text-magenta' : 'text-muted-foreground'}">{t.nowPlaying ? 'now' : ago(t.playedAt)}</span>
					</button>
				{/each}
			</ScrollArea>
		{:else}
			<p class="text-muted-foreground">loading…</p>
		{/if}
	</section>

	<!-- connections -->
	<section class="flex w-80 shrink-0 flex-col gap-8 border-l pl-6">
		{#if status}
			<div class="flex flex-col gap-3">
				<h2 class="subpivot">last.fm</h2>
				{#if !status.lastfm.apiKey || !status.lastfm.hasSecret}
					<p class="text-xs text-muted-foreground">
						squirt needs a last.fm api key and shared secret.
						<button class="underline underline-offset-2 hover:text-foreground" onclick={() => openExternal('https://www.last.fm/api/account/create')}>get them here</button>.
					</p>
					<Input placeholder="api key" bind:value={keyInput} class="font-mono text-xs select-text" />
					<Input placeholder="shared secret" type="password" bind:value={secretInput} class="font-mono text-xs" />
					<Button size="sm" class="w-fit lowercase" disabled={busy || !keyInput || (!secretInput && !status.lastfm.hasSecret)} onclick={saveKeys}>save</Button>
				{:else if !status.lastfm.connected}
					{#if status.lastfm.authPending}
						<p class="text-xs text-muted-foreground">approve squirt on the last.fm page that opened in your browser, then come back here.</p>
						<div class="flex gap-2">
							<Button size="sm" class="lowercase" disabled={busy} onclick={finish}>i've approved it</Button>
							<Button size="sm" variant="outline" class="lowercase" disabled={busy} onclick={connect}>open again</Button>
						</div>
					{:else}
						<p class="text-xs text-muted-foreground">sign in to scrobble what you play and show your listening history.</p>
						<Button size="sm" class="w-fit lowercase" disabled={busy} onclick={connect}>connect last.fm</Button>
					{/if}
					<button class="w-fit text-xs text-muted-foreground underline underline-offset-2 hover:text-foreground" onclick={() => status && (status.lastfm.hasSecret = false)}>change api key</button>
				{:else}
					<p class="text-xs text-muted-foreground">signed in as <span class="text-foreground">{status.lastfm.username}</span></p>
					<label class="flex items-center justify-between gap-3">
						<span class="lowercase">scrobble what i play</span>
						<Switch checked={status.lastfm.enabled} disabled={busy} onCheckedChange={(on) => run(() => social.setLastfmEnabled(on))} />
					</label>
					<Button size="sm" variant="outline" class="w-fit lowercase" disabled={busy} onclick={() => run(() => social.disconnect())}>disconnect</Button>
				{/if}
				{#if status.lastfm.error}<p class="text-xs text-destructive">{status.lastfm.error}</p>{/if}
			</div>

			<div class="flex flex-col gap-3">
				<h2 class="subpivot">discord</h2>
				<p class="text-xs text-muted-foreground">show "listening to squirt" with the song and a progress bar on your discord profile.</p>
				<div class="flex gap-2">
					<Input placeholder="application id" bind:value={discordInput} class="font-mono text-xs" />
					<Button size="sm" variant="outline" class="lowercase" disabled={busy || discordInput === status.discord.clientId} onclick={() => run(() => social.setDiscordClientId(discordInput))}>save</Button>
				</div>
				<label class="flex items-center justify-between gap-3">
					<span class="lowercase">show what i'm listening to</span>
					<Switch checked={status.discord.enabled} disabled={busy || !status.discord.clientId} onCheckedChange={(on) => run(() => social.setDiscordEnabled(on))} />
				</label>
				{#if status.discord.enabled}
					<p class="text-xs {status.discord.error ? 'text-destructive' : 'text-muted-foreground'}">
						{status.discord.error ?? (status.discord.connected ? 'connected to discord' : 'shows up when you play something')}
					</p>
				{/if}
			</div>

			{#if actionError}<p class="text-xs text-destructive">{actionError}</p>{/if}
		{/if}
	</section>
</div>
