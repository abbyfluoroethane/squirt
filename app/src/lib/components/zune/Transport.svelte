<script lang="ts">
	// Playback controls: round outline buttons, now-playing line, seek bar.
	import type { PlayerState } from '$lib/api';
	import { duration } from '$lib/format';
	import { SkipBack, SkipForward, Play, Pause } from 'phosphor-svelte';

	let {
		state,
		title,
		subtitle,
		onToggle,
		onPrev,
		onNext,
		onSeek
	}: {
		state: PlayerState;
		title?: string;
		subtitle?: string;
		onToggle: () => void;
		onPrev: () => void;
		onNext: () => void;
		onSeek: (ms: number) => void;
	} = $props();

	const active = $derived(state.trackId !== null);
	const pct = $derived(state.durationMs ? Math.min(100, (state.positionMs / state.durationMs) * 100) : 0);

	function seekFrom(e: MouseEvent) {
		if (!active || !state.durationMs) return;
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		onSeek(Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)) * state.durationMs);
	}
</script>

<div class="flex items-center gap-5">
	<div class="flex items-center gap-2.5">
		<button class="round size-8" onclick={onPrev} disabled={!active} aria-label="previous"><SkipBack size={14} weight="fill" /></button>
		<button class="round size-11 border-2" onclick={onToggle} disabled={!active} aria-label={state.status === 'playing' ? 'pause' : 'play'}>
			{#if state.status === 'playing'}<Pause size={18} weight="fill" />{:else}<Play size={18} weight="fill" class="translate-x-px" />{/if}
		</button>
		<button class="round size-8" onclick={onNext} disabled={!active} aria-label="next"><SkipForward size={14} weight="fill" /></button>
	</div>
	<div class="flex w-[26rem] max-w-[38vw] flex-col gap-1.5">
		<div class="flex min-w-0 items-baseline gap-2">
			{#if active}
				<span class="truncate">{title}</span>
				<span class="truncate text-xs text-muted-foreground">{subtitle}</span>
			{:else}
				<span class="text-muted-foreground lowercase">nothing playing</span>
			{/if}
		</div>
		<div class="flex items-center gap-2.5">
			<span class="tabular w-9 text-[0.7rem] text-muted-foreground">{active ? duration(state.positionMs) : ''}</span>
			<button class="group relative h-3 flex-1 cursor-pointer disabled:cursor-default" onclick={seekFrom} disabled={!active} aria-label="seek">
				<span class="absolute inset-x-0 top-1/2 h-[3px] -translate-y-1/2 bg-muted"></span>
				<span class="zune-gradient absolute top-1/2 left-0 h-[3px] -translate-y-1/2 transition-[width] duration-500 ease-linear" style:width="{pct}%"></span>
			</button>
			<span class="tabular w-9 text-right text-[0.7rem] text-muted-foreground">{active ? duration(state.durationMs) : ''}</span>
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
</style>
