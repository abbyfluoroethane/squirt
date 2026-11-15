<script lang="ts">
	// Collection > videos: a grid of thumbnails; clicking one plays it.
	import type { MediaApi, VideoItem } from '$lib/api';
	import { bytes, duration } from '$lib/format';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { ArrowLeft, FilmSlate } from 'phosphor-svelte';
	import { fade } from 'svelte/transition';

	let { media, videos, onPlay }: { media: MediaApi; videos: VideoItem[]; onPlay?: () => void } = $props();

	let playing = $state<{ item: VideoItem; url: string } | null>(null);

	async function play(v: VideoItem) {
		onPlay?.();
		playing = { item: v, url: await media.fileUrl('video', v) };
	}

	function onKey(e: KeyboardEvent) {
		if (e.code === 'Escape' && playing) {
			e.stopPropagation();
			playing = null;
		}
	}
	const res = (v: VideoItem) => (v.height ? `${v.height}p` : '');
</script>

<svelte:window onkeydown={onKey} />

{#if videos.length === 0}
	<p class="fk-enter text-muted-foreground lowercase" style="--i: 1">no videos yet. add a video folder in settings.</p>
{:else}
	<ScrollArea class="fk-enter min-h-0 flex-1 pr-3" style="--i: 1">
		<div class="grid grid-cols-[repeat(auto-fill,minmax(15rem,1fr))] gap-x-5 gap-y-6">
			{#each videos as v (v.id)}
				<button class="group flex flex-col gap-2 text-left" onclick={() => play(v)}>
					<div class="relative aspect-video w-full overflow-hidden bg-muted">
						{#if v.thumb}
							<img src={v.thumb} alt="" class="size-full object-cover transition-transform duration-300 group-hover:scale-[1.03]" draggable="false" />
						{:else}
							<FilmSlate size={28} weight="light" class="absolute inset-0 m-auto text-muted-foreground" />
						{/if}
						{#if v.durationMs}
							<span class="tabular absolute right-1.5 bottom-1.5 bg-black/70 px-1.5 py-0.5 text-[0.7rem] text-white">{duration(v.durationMs)}</span>
						{/if}
					</div>
					<span class="flex min-w-0 flex-col">
						<span class="truncate">{v.title}</span>
						<span class="text-xs text-muted-foreground lowercase">{[res(v), v.codec, bytes(v.size)].filter(Boolean).join(' · ')}</span>
					</span>
				</button>
			{/each}
		</div>
	</ScrollArea>
{/if}

{#if playing}
	<div class="dark fixed inset-0 z-50 flex flex-col bg-black text-foreground" transition:fade={{ duration: 150 }} role="dialog" aria-label="video player">
		<div class="flex items-center gap-4 px-12 pt-12 pb-4">
			<button class="flex items-center gap-2 text-muted-foreground transition-colors hover:text-foreground" onclick={() => (playing = null)}>
				<ArrowLeft size={18} /><span class="subpivot">back</span>
			</button>
			<span class="truncate text-[1.4rem] font-light">{playing.item.title}</span>
		</div>
		<!-- svelte-ignore a11y_media_has_caption -->
		<video src={playing.url} controls autoplay class="min-h-0 w-full flex-1 bg-black"></video>
	</div>
{/if}
