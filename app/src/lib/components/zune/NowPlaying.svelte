<script lang="ts">
	// Full-screen now playing: big art and type over one of two backgrounds,
	// glows in the cover's colors or a wave that follows the music.
	import type { Album, PlayerState, Track } from '$lib/api';
	import { duration } from '$lib/format';
	import AlbumArt from './AlbumArt.svelte';
	import { Slider } from '$lib/components/ui/slider';
	import { ArrowLeft, Pause, Play, SkipBack, SkipForward, SpeakerHigh, SpeakerSimpleNone } from 'phosphor-svelte';
	import { fade } from 'svelte/transition';
	import ColorField from './now-playing/ColorField.svelte';
	import Visualizer from './now-playing/Visualizer.svelte';
	import SeekBar from './SeekBar.svelte';

	let {
		state: playback,
		track,
		album,
		upNext,
		volume = $bindable(),
		onVolume,
		onToggle,
		onPrev,
		onNext,
		onSeek,
		onArtist,
		onAlbum,
		onClose,
		spectrum
	}: {
		state: PlayerState;
		track: Track;
		album: Album;
		upNext: Track[];
		volume: number;
		onVolume: (v: number) => void;
		onToggle: () => void;
		onPrev: () => void;
		onNext: () => void;
		onSeek: (ms: number) => void;
		onArtist: () => void;
		onAlbum: () => void;
		onClose: () => void;
		spectrum: (bands: number) => Promise<number[]>;
	} = $props();

	// Background: glows in the cover's colors, or a spectrum wave over the
	// blurred cover. Remembered between launches.
	const BACKGROUNDS = ['color', 'wave'] as const;
	type Background = (typeof BACKGROUNDS)[number];
	const BG_KEY = 'squirt.nowPlayingBackground';
	let background = $state<Background>(
		(() => {
			try {
				const v = localStorage.getItem(BG_KEY);
				if (v && (BACKGROUNDS as readonly string[]).includes(v)) return v as Background;
			} catch {
				// No storage; use the default.
			}
			return 'color';
		})()
	);
	function setBackground(b: Background) {
		background = b;
		try {
			localStorage.setItem(BG_KEY, b);
		} catch {
			// Not remembered.
		}
	}

	let scrubMs = $state<number | null>(null);
</script>

<!-- Always dark, like the device. -->
<div class="dark fixed inset-0 z-50 overflow-hidden bg-background text-foreground" transition:fade={{ duration: 180 }} role="dialog" aria-label="now playing">
	{#if background === 'color'}
		<ColorField src={album.art} hue={album.hue} />
		<div class="absolute inset-0 bg-gradient-to-t from-background/90 via-background/30 to-transparent"></div>
	{:else}
		{#if album.art}
			<img src={album.art} alt="" class="pointer-events-none absolute inset-0 size-full scale-125 object-cover opacity-40 blur-3xl" />
		{/if}
		<div class="absolute inset-0 bg-gradient-to-t from-background via-background/70 to-background/30"></div>
	{/if}

	{#if background === 'wave'}
		<Visualizer {spectrum} />
	{/if}

	<div class="relative flex h-full flex-col px-12 pt-12 pb-10">
		<div class="flex items-center">
			<button class="flex w-fit items-center gap-2 text-muted-foreground transition-colors hover:text-foreground" onclick={onClose}>
				<ArrowLeft size={18} /><span class="subpivot">back</span>
			</button>
			<!-- Background picker. -->
			<div class="ml-auto flex items-baseline gap-4 text-sm lowercase" role="radiogroup" aria-label="background">
				<span class="text-xs text-faint">background</span>
				{#each BACKGROUNDS as b (b)}
					<button role="radio" aria-checked={background === b} class="transition-colors {background === b ? 'text-foreground' : 'text-faint hover:text-muted-foreground'}" onclick={() => setBackground(b)}>{b}</button>
				{/each}
			</div>
		</div>

		<div class="flex min-h-0 flex-1 items-end gap-12">
			<div class="shadow-2xl shadow-black/60">
				<AlbumArt hue={album.hue} size={400} src={album.art ?? undefined} />
			</div>

			<div class="flex min-w-0 flex-1 flex-col gap-6 pb-1">
				<div class="flex min-w-0 flex-col gap-1">
					<span class="truncate text-[3.2rem] leading-tight font-light">{track.title}</span>
					<button class="link w-fit truncate text-left text-[1.6rem] font-[350] text-muted-foreground" onclick={onArtist}>{track.artist}</button>
					<button class="link w-fit truncate text-left text-[1.1rem] text-muted-foreground" onclick={onAlbum}>
						{album.title}{album.year ? ` · ${album.year}` : ''}
					</button>
				</div>

				<div class="flex max-w-3xl flex-col gap-2">
					<div class="flex"><SeekBar positionMs={playback.positionMs} durationMs={playback.durationMs} {onSeek} bind:scrubMs thickness={4} /></div>
					<div class="flex justify-between text-sm text-muted-foreground">
						<span class="tabular">{duration(scrubMs ?? playback.positionMs)}</span>
						<span class="tabular">{duration(playback.durationMs)}</span>
					</div>
				</div>

				<div class="flex items-center gap-4">
					<button class="round size-12" onclick={onPrev} aria-label="previous"><SkipBack size={20} weight="fill" /></button>
					<button class="round size-16 border-2" onclick={onToggle} aria-label={playback.status === 'playing' ? 'pause' : 'play'}>
						{#if playback.status === 'playing'}<Pause size={26} weight="fill" />{:else}<Play size={26} weight="fill" class="translate-x-0.5" />{/if}
					</button>
					<button class="round size-12" onclick={onNext} aria-label="next"><SkipForward size={20} weight="fill" /></button>
					<div class="ml-6 flex items-center gap-2.5 text-muted-foreground">
						<SpeakerSimpleNone size={16} />
						<Slider type="single" bind:value={volume} min={0} max={1} step={0.01} class="w-32" onValueChange={onVolume} aria-label="volume" />
						<SpeakerHigh size={16} />
					</div>
				</div>
			</div>

			{#if upNext.length}
				<div class="hidden w-64 shrink-0 flex-col gap-1.5 pb-1 xl:flex">
					<span class="colhead">up next</span>
					{#each upNext as t (t.id)}
						<div class="flex min-w-0 flex-col">
							<span class="truncate">{t.title}</span>
							<span class="truncate text-xs text-muted-foreground">{t.artist} · <span class="tabular">{duration(t.durationMs)}</span></span>
						</div>
					{/each}
				</div>
			{/if}
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
		transition: background-color 120ms, color 120ms;
	}
	.round:hover {
		background: var(--foreground);
		color: var(--background);
	}
	.link:hover {
		color: var(--foreground);
		text-decoration: underline;
		text-underline-offset: 3px;
	}
</style>
