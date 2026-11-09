<script lang="ts">
	// A seek bar: click to jump, drag to scrub, arrow keys for 5 s steps.
	// While dragging it shows where you are (and reports it through
	// `scrubMs`, for the time label) and seeks once, on release. After that
	// it holds the new spot briefly so it doesn't snap back before the
	// player reports the new position.
	let {
		positionMs,
		durationMs,
		disabled = false,
		onSeek,
		scrubMs = $bindable(null),
		thickness = 3,
		label = 'seek'
	}: {
		positionMs: number;
		durationMs: number;
		disabled?: boolean;
		onSeek: (ms: number) => void;
		scrubMs?: number | null;
		thickness?: number;
		label?: string;
	} = $props();

	let bar: HTMLDivElement;
	let dragging = $state(false);
	let hover = $state(false);
	// Where we last seeked to, until the player reports it (or 1.5 s pass).
	let held = $state<number | null>(null);
	let heldTimer: ReturnType<typeof setTimeout> | undefined;

	const off = $derived(disabled || !durationMs);
	const shownMs = $derived(scrubMs ?? (held !== null && Math.abs(positionMs - held) > 1500 ? held : positionMs));
	$effect(() => {
		if (held !== null && Math.abs(positionMs - held) <= 1500) held = null;
	});
	const pct = $derived(durationMs ? Math.min(100, Math.max(0, (shownMs / durationMs) * 100)) : 0);

	const msAt = (x: number) => {
		const r = bar.getBoundingClientRect();
		return Math.round(Math.max(0, Math.min(1, (x - r.left) / r.width)) * durationMs);
	};

	function commit(ms: number) {
		held = ms;
		clearTimeout(heldTimer);
		heldTimer = setTimeout(() => (held = null), 1500);
		onSeek(ms);
	}

	function down(e: PointerEvent) {
		if (off || e.button !== 0) return;
		e.preventDefault();
		bar.setPointerCapture(e.pointerId);
		dragging = true;
		scrubMs = msAt(e.clientX);
	}
	function move(e: PointerEvent) {
		if (dragging) scrubMs = msAt(e.clientX);
	}
	function up(e: PointerEvent) {
		if (!dragging) return;
		dragging = false;
		const ms = msAt(e.clientX);
		scrubMs = null;
		commit(ms);
	}
	function cancel() {
		dragging = false;
		scrubMs = null;
	}
	function key(e: KeyboardEvent) {
		if (off) return;
		const step = e.key === 'ArrowRight' ? 5000 : e.key === 'ArrowLeft' ? -5000 : 0;
		if (!step) return;
		e.preventDefault();
		e.stopPropagation();
		commit(Math.max(0, Math.min(durationMs, Math.round(shownMs + step))));
	}
</script>

<div
	bind:this={bar}
	class="group relative h-4 flex-1 touch-none select-none {off ? 'cursor-default' : 'cursor-pointer'}"
	role="slider"
	tabindex={off ? -1 : 0}
	aria-label={label}
	aria-disabled={off}
	aria-valuemin={0}
	aria-valuemax={Math.round(durationMs / 1000)}
	aria-valuenow={Math.round(shownMs / 1000)}
	onpointerdown={down}
	onpointermove={move}
	onpointerup={up}
	onpointercancel={cancel}
	onpointerenter={() => (hover = true)}
	onpointerleave={() => (hover = false)}
	onkeydown={key}
>
	<span class="absolute inset-x-0 top-1/2 -translate-y-1/2 bg-muted" style:height="{thickness}px"></span>
	<span
		class="zune-gradient absolute top-1/2 left-0 -translate-y-1/2 {dragging || held !== null ? '' : 'transition-[width] duration-500 ease-linear'}"
		style:height="{thickness}px"
		style:width="{pct}%"
	></span>
	<!-- Square thumb, shown on hover and while dragging. -->
	{#if !off && (hover || dragging)}
		<span
			class="absolute top-1/2 size-2.5 -translate-x-1/2 -translate-y-1/2 bg-foreground {dragging || held !== null ? '' : 'transition-[left] duration-500 ease-linear'}"
			style:left="{pct}%"
		></span>
	{/if}
</div>
