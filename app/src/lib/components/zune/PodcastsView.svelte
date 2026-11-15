<script lang="ts">
	// Collection > podcasts: shows on the left, episodes newest first.
	import type { Album, Track } from '$lib/api';
	import { duration } from '$lib/format';
	import AlbumArt from './AlbumArt.svelte';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { SpeakerHigh } from 'phosphor-svelte';

	let { shows, playingId, onPlay }: { shows: Album[]; playingId: number | null; onPlay: (show: Album, index: number) => void } = $props();

	let showId = $state<string | null>(null);
	const show = $derived(shows.find((s) => s.id === showId) ?? shows[0] ?? null);
	const isPlaying = (t: Track) => t.id !== undefined && t.id === playingId;
	const when = (d?: string | null) =>
		d && d.length >= 10 ? new Date(d + 'T00:00:00').toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' }).toLowerCase() : (d ?? '');
</script>

{#if shows.length === 0}
	<p class="fk-enter text-muted-foreground lowercase" style="--i: 1">no podcasts yet. add a podcast folder in settings.</p>
{:else}
	<section class="fk-enter flex w-72 shrink-0 flex-col" style="--i: 1">
		<h3 class="colhead mb-2">show</h3>
		<ScrollArea class="min-h-0 flex-1 pr-3">
			{#each shows as s (s.id)}
				<button class="flex w-full items-center gap-3 p-1.5 text-left transition-colors {show?.id === s.id ? 'bg-selection' : 'hover:bg-accent'}" onclick={() => (showId = s.id)}>
					<AlbumArt hue={s.hue} size={52} src={s.art ?? undefined} />
					<span class="flex min-w-0 flex-col">
						<span class="truncate text-[0.95rem]">{s.title}</span>
						<span class="truncate text-xs text-muted-foreground">{s.artist} · <span class="tabular">{s.tracks.length}</span> episodes</span>
					</span>
				</button>
			{/each}
		</ScrollArea>
	</section>
	<section class="fk-enter flex min-w-0 flex-1 flex-col border-l pl-4" style="--i: 2">
		{#if show}
			<div class="mb-4 flex items-end gap-4">
				<AlbumArt hue={show.hue} size={96} src={show.art ?? undefined} />
				<div class="flex min-w-0 flex-col gap-0.5">
					<span class="truncate text-[1.35rem] leading-tight font-light">{show.title}</span>
					<span class="truncate text-muted-foreground">{show.artist}</span>
				</div>
			</div>
			<div class="grid grid-cols-[1.25rem_1fr_7rem_3.5rem] gap-x-3 px-1.5 pb-1.5">
				<span></span><span class="colhead">episode</span><span class="colhead">date</span><span class="colhead text-right">length</span>
			</div>
			<ScrollArea class="min-h-0 flex-1">
				{#each show.tracks as t, i (t.id)}
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div class="grid grid-cols-[1.25rem_1fr_7rem_3.5rem] items-center gap-x-3 px-1.5 py-1.5 hover:bg-accent {isPlaying(t) ? 'text-magenta' : ''}" ondblclick={() => onPlay(show!, i)}>
						<span>{#if isPlaying(t)}<SpeakerHigh size={13} weight="fill" />{/if}</span>
						<span class="truncate">{t.title}</span>
						<span class="truncate text-xs text-muted-foreground lowercase">{when(t.date)}</span>
						<span class="tabular text-right text-xs text-muted-foreground">{duration(t.durationMs)}</span>
					</div>
				{/each}
			</ScrollArea>
		{/if}
	</section>
{/if}
