<script lang="ts">
	// Collection > pictures: albums on the left, a thumbnail grid, and a viewer.
	import type { MediaApi, PictureAlbum, PictureItem } from '$lib/api';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { ArrowLeft, CaretLeft, CaretRight, ImageSquare } from 'phosphor-svelte';
	import { fade } from 'svelte/transition';

	let { media, albums }: { media: MediaApi; albums: PictureAlbum[] } = $props();

	let albumName = $state<string | null>(null);
	const album = $derived(albums.find((a) => a.name === albumName) ?? albums[0] ?? null);

	let viewing = $state<{ index: number; url: string } | null>(null);

	async function open(i: number) {
		if (!album) return;
		viewing = { index: i, url: await media.fileUrl('picture', album.pictures[i]) };
	}
	function step(d: number) {
		if (!viewing || !album) return;
		const n = album.pictures.length;
		open((viewing.index + d + n) % n);
	}
	function onKey(e: KeyboardEvent) {
		if (!viewing) return;
		if (e.code === 'Escape') {
			e.stopPropagation();
			viewing = null;
		} else if (e.code === 'ArrowRight') step(1);
		else if (e.code === 'ArrowLeft') step(-1);
	}
	const current = $derived<PictureItem | null>(viewing && album ? album.pictures[viewing.index] : null);
</script>

<svelte:window onkeydown={onKey} />

{#if albums.length === 0}
	<p class="metro-enter text-muted-foreground lowercase" style="--i: 1">no pictures yet. add a picture folder in settings.</p>
{:else}
	<section class="metro-enter flex w-56 shrink-0 flex-col" style="--i: 1">
		<h3 class="colhead mb-2">album</h3>
		<ScrollArea class="min-h-0 flex-1 pr-3">
			{#each albums as a (a.name)}
				<button class="flex w-full items-center gap-3 p-1.5 text-left transition-colors {album?.name === a.name ? 'bg-selection' : 'hover:bg-accent'}" onclick={() => (albumName = a.name)}>
					{#if a.cover}
						<img src={a.cover} alt="" class="size-11 shrink-0 object-cover" draggable="false" />
					{:else}
						<div class="flex size-11 shrink-0 items-center justify-center bg-muted"><ImageSquare size={16} class="text-muted-foreground" /></div>
					{/if}
					<span class="flex min-w-0 flex-col">
						<span class="truncate">{a.name}</span>
						<span class="text-xs text-muted-foreground"><span class="tabular">{a.count}</span> pictures</span>
					</span>
				</button>
			{/each}
		</ScrollArea>
	</section>
	<section class="metro-enter flex min-w-0 flex-1 flex-col border-l pl-4" style="--i: 2">
		<h3 class="colhead mb-2">{album?.name ?? ''}</h3>
		<ScrollArea class="min-h-0 flex-1 pr-3">
			<div class="grid grid-cols-[repeat(auto-fill,minmax(8.5rem,1fr))] gap-1.5">
				{#each album?.pictures ?? [] as p, i (p.id)}
					<button class="group aspect-square overflow-hidden bg-muted" onclick={() => open(i)}>
						{#if p.thumb}<img src={p.thumb} alt="" class="size-full object-cover transition-transform duration-300 group-hover:scale-105" loading="lazy" draggable="false" />{/if}
					</button>
				{/each}
			</div>
		</ScrollArea>
	</section>
{/if}

{#if viewing && current}
	<div class="dark fixed inset-0 z-50 flex flex-col bg-black text-foreground" transition:fade={{ duration: 150 }} role="dialog" aria-label="picture viewer">
		<div class="flex items-center gap-4 px-12 pt-12 pb-4">
			<button class="flex items-center gap-2 text-muted-foreground transition-colors hover:text-foreground" onclick={() => (viewing = null)}>
				<ArrowLeft size={18} /><span class="subpivot">back</span>
			</button>
			<span class="text-muted-foreground lowercase">{current.album} · <span class="tabular">{viewing.index + 1}</span> of <span class="tabular">{album?.pictures.length}</span></span>
		</div>
		<div class="relative flex min-h-0 flex-1 items-center justify-center px-16 pb-10">
			<img src={viewing.url} alt="" class="max-h-full max-w-full object-contain" draggable="false" />
			<button class="absolute top-1/2 left-4 -translate-y-1/2 p-2 text-white/60 hover:text-white" onclick={() => step(-1)} aria-label="previous"><CaretLeft size={32} weight="light" /></button>
			<button class="absolute top-1/2 right-4 -translate-y-1/2 p-2 text-white/60 hover:text-white" onclick={() => step(1)} aria-label="next"><CaretRight size={32} weight="light" /></button>
		</div>
	</div>
{/if}
