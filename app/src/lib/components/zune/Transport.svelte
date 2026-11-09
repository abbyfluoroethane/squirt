<script lang="ts">
	// Playback controls: round outline buttons, now-playing line, seek bar.
	import type { PlayerState } from '$lib/api';
	import { duration } from '$lib/format';
	import { SkipBack, SkipForward, Play, Pause } from 'phosphor-svelte';
	import AlbumArt from './AlbumArt.svelte';
	import SeekBar from './SeekBar.svelte';

	let {
		state: playback,
		title,
		artist,
		album,
		art,
		hue = 0,
		onArtist,
		onAlbum,
		onArt,
		onToggle,
		onPrev,
		onNext,
		onSeek
	}: {
		state: PlayerState;
		title?: string;
		artist?: string;
		album?: string;
		art?: string;
		hue?: number;
		onArtist: () => void;
		onAlbum: () => void;
		onArt: () => void;
		onToggle: () => void;
		onPrev: () => void;
		onNext: () => void;
		onSeek: (ms: number) => void;
	} = $props();

	const active = $derived(playback.trackId !== null);
	let scrubMs = $state<number | null>(null);
</script>

<div class="flex items-center gap-5">
	<div class="flex items-center gap-2.5">
		<button class="round size-8" onclick={onPrev} disabled={!active} aria-label="previous"><SkipBack size={14} weight="fill" /></button>
		<button class="round size-11 border-2" onclick={onToggle} disabled={!active} aria-label={playback.status === 'playing' ? 'pause' : 'play'}>
			{#if playback.status === 'playing'}<Pause size={18} weight="fill" />{:else}<Play size={18} weight="fill" class="translate-x-px" />{/if}
		</button>
		<button class="round size-8" onclick={onNext} disabled={!active} aria-label="next"><SkipForward size={14} weight="fill" /></button>
	</div>
	{#if active}
		<button class="shrink-0 transition-opacity hover:opacity-80" onclick={onArt} aria-label="open now playing" title="now playing">
			<AlbumArt {hue} size={56} src={art} />
		</button>
	{:else}
		<div class="size-14 shrink-0 bg-muted"></div>
	{/if}
	<div class="-ml-1 flex w-[26rem] max-w-[34vw] flex-col gap-1">
		<!-- Song name, then artist · album beneath it. -->
		<div class="flex min-w-0 flex-col leading-tight">
			{#if active}
				<span class="truncate">{title}</span>
				<span class="flex min-w-0 items-baseline gap-1 text-xs text-muted-foreground">
					<button class="link truncate" onclick={onArtist}>{artist}</button>·<button class="link truncate" onclick={onAlbum}>{album}</button>
				</span>
			{:else}
				<span class="text-muted-foreground lowercase">nothing playing</span>
			{/if}
		</div>
		<div class="flex items-center gap-2.5">
			<span class="tabular w-9 text-[0.7rem] text-muted-foreground">{active ? duration(scrubMs ?? playback.positionMs) : ''}</span>
			<SeekBar positionMs={playback.positionMs} durationMs={playback.durationMs} disabled={!active} {onSeek} bind:scrubMs />
			<span class="tabular w-9 text-right text-[0.7rem] text-muted-foreground">{active ? duration(playback.durationMs) : ''}</span>
		</div>
	</div>
</div>

<style>
	.round {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border-radius: 9999px;
		border-width: 1.5px;
		border-color: var(--foreground);
		color: var(--foreground);
		transition: background-color 120ms, color 120ms, opacity 120ms;
	}
	.round:hover:not(:disabled) {
		background: var(--foreground);
		color: var(--background);
	}
	.round:disabled {
		opacity: 0.3;
	}
	.link:hover {
		color: var(--foreground);
		text-decoration: underline;
		text-underline-offset: 2px;
	}
</style>
