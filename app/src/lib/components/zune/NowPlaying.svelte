<script lang="ts">
	// Full-screen now playing, Zune-desktop style: blurred art behind, the
	// artist's name drifting huge and faint across the back, big art and type.
	import type { Album, PlayerState, Track } from '$lib/api';
	import { duration } from '$lib/format';
	import AlbumArt from './AlbumArt.svelte';
	import { Slider } from '$lib/components/ui/slider';
	import { ArrowLeft, Pause, Play, SkipBack, SkipForward, SpeakerHigh, SpeakerSimpleNone } from 'phosphor-svelte';
	import { fade } from 'svelte/transition';

	let {
		state,
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
		onClose
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
	} = $props();

	const pct = $derived(state.durationMs ? Math.min(100, (state.positionMs / state.durationMs) * 100) : 0);

	function seekFrom(e: MouseEvent) {
		if (!state.durationMs) return;
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		onSeek(Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)) * state.durationMs);
	}
</script>

<!-- Always dark, like the device. -->
<div class="dark fixed inset-0 z-50 overflow-hidden bg-background text-foreground" transition:fade={{ duration: 180 }} role="dialog" aria-label="now playing">
	{#if album.art}
		<img src={album.art} alt="" class="pointer-events-none absolute inset-0 size-full scale-125 object-cover opacity-40 blur-3xl" />
	{/if}
	<div class="absolute inset-0 bg-gradient-to-t from-background via-background/70 to-background/30"></div>

	<!-- The artist's name, huge and faint, drifting. -->
	<div class="drift pointer-events-none absolute top-[6%] left-0 text-[13rem] leading-none font-light whitespace-nowrap lowercase opacity-[0.07]" aria-hidden="true">
		{track.artist} &nbsp; {track.artist} &nbsp; {track.artist}
	</div>

	<div class="relative flex h-full flex-col px-12 pt-8 pb-10">
		<button class="flex w-fit items-center gap-2 text-muted-foreground transition-colors hover:text-foreground" onclick={onClose}>
			<ArrowLeft size={18} /><span class="subpivot">back</span>
		</button>

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
					<button class="relative h-4 w-full cursor-pointer" onclick={seekFrom} aria-label="seek">
						<span class="absolute inset-x-0 top-1/2 h-1 -translate-y-1/2 bg-white/15"></span>
						<span class="zune-gradient absolute top-1/2 left-0 h-1 -translate-y-1/2 transition-[width] duration-500 ease-linear" style:width="{pct}%"></span>
					</button>
					<div class="flex justify-between text-sm text-muted-foreground">
						<span class="tabular">{duration(state.positionMs)}</span>
						<span class="tabular">{duration(state.durationMs)}</span>
					</div>
				</div>

				<div class="flex items-center gap-4">
					<button class="round size-12" onclick={onPrev} aria-label="previous"><SkipBack size={20} weight="fill" /></button>
					<button class="round size-16 border-2" onclick={onToggle} aria-label={state.status === 'playing' ? 'pause' : 'play'}>
						{#if state.status === 'playing'}<Pause size={26} weight="fill" />{:else}<Play size={26} weight="fill" class="translate-x-0.5" />{/if}
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
	.drift {
		animation: drift 90s linear infinite alternate;
	}
	@keyframes drift {
		from {
			transform: translateX(0);
		}
		to {
			transform: translateX(-45%);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.drift {
			animation: none;
		}
	}
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
