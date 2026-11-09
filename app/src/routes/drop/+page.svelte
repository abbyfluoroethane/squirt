<script lang="ts">
	// Drop workshop: liquid-glass / Fluent-style takes on the droplet mark.
	const variants = [
		{ id: 'fluent', name: 'fluent', note: 'shade, highlight streak, glint, rim light, caustic' },
		{ id: 'soft', name: 'soft', note: 'gradient body, one big soft highlight' },
		{ id: 'glass', name: 'glass', note: 'tinted glass shell with the gradient liquid inside' },
		{ id: 'flat', name: 'flat fluent', note: 'gradient with a gentle top light only' },
		{ id: 'deep', name: 'deep', note: 'richer gradient, strong highlight, cast shadow' },
		{ id: 'liquid', name: 'liquid', note: 'half-full glass drop with a wavy meniscus' }
	] as const;
	const ladder = [20, 32, 48, 128, 256];

	const DROP = 'M50 6 C50 6 18 44 18 64 A32 32 0 0 0 82 64 C82 44 50 6 50 6 Z';
	const INNER = 'M50 14 C50 14 24 47 24 64 A26 26 0 0 0 76 64 C76 47 50 14 50 14 Z';
</script>

<svelte:head><title>Squirt drop workshop</title></svelte:head>

{#snippet drop(id: string, size: number, k: string)}
	{@const tiny = size < 28}
	<svg width={size * (68 / 94)} height={size} viewBox="16 4 68 94" aria-hidden="true" overflow="visible">
		<defs>
			<linearGradient id="{k}-body" x1="0.2" y1="0" x2="0.8" y2="1">
				<stop offset="0" stop-color="#ff5fb8" />
				<stop offset="0.45" stop-color="#ec008c" />
				<stop offset="1" stop-color="#f7941d" />
			</linearGradient>
			<linearGradient id="{k}-deep" x1="0.2" y1="0" x2="0.8" y2="1">
				<stop offset="0" stop-color="#e0148f" />
				<stop offset="0.55" stop-color="#b0005f" />
				<stop offset="1" stop-color="#f06a10" />
			</linearGradient>
			<radialGradient id="{k}-shade" cx="0.35" cy="0.3" r="0.85">
				<stop offset="0.55" stop-color="#4a0028" stop-opacity="0" />
				<stop offset="1" stop-color="#4a0028" stop-opacity="0.45" />
			</radialGradient>
			<linearGradient id="{k}-hi" x1="0" y1="0" x2="0.4" y2="1">
				<stop offset="0" stop-color="#fff" stop-opacity="0.85" />
				<stop offset="1" stop-color="#fff" stop-opacity="0" />
			</linearGradient>
			<linearGradient id="{k}-top" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="#fff" stop-opacity="0.35" />
				<stop offset="0.6" stop-color="#fff" stop-opacity="0" />
			</linearGradient>
			<linearGradient id="{k}-rim" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="#fff" stop-opacity="0.7" />
				<stop offset="0.5" stop-color="#fff" stop-opacity="0" />
			</linearGradient>
			<linearGradient id="{k}-glass" x1="0" y1="0" x2="1" y2="1">
				<stop offset="0" stop-color="#ffd6ec" stop-opacity="0.55" />
				<stop offset="1" stop-color="#ffb4d8" stop-opacity="0.2" />
			</linearGradient>
			<clipPath id="{k}-clip"><path d={DROP} /></clipPath>
			<filter id="{k}-shadow" x="-40%" y="-30%" width="180%" height="170%">
				<feDropShadow dx="0" dy="4" stdDeviation="4" flood-color="#5a0030" flood-opacity="0.45" />
			</filter>
		</defs>

		{#if id === 'fluent'}
			<path d={DROP} fill="url(#{k}-body)" />
			<path d={DROP} fill="url(#{k}-shade)" />
			<g clip-path="url(#{k}-clip)">
				{#if !tiny}<ellipse cx="57" cy="86" rx="14" ry="5" fill="#fff" opacity="0.18" />{/if}
				<ellipse cx="37" cy="56" rx="6.5" ry="17" transform="rotate(28 37 56)" fill="url(#{k}-hi)" />
				{#if !tiny}<circle cx="46" cy="24" r="2.6" fill="#fff" opacity="0.8" />{/if}
			</g>
			{#if !tiny}<path d={DROP} fill="none" stroke="url(#{k}-rim)" stroke-width="1.6" />{/if}
		{:else if id === 'soft'}
			<path d={DROP} fill="url(#{k}-body)" />
			<g clip-path="url(#{k}-clip)">
				<ellipse cx="40" cy="50" rx="16" ry="26" transform="rotate(20 40 50)" fill="url(#{k}-top)" />
			</g>
		{:else if id === 'glass'}
			<path d={DROP} fill="url(#{k}-glass)" />
			<path d={INNER} fill="url(#{k}-body)" opacity="0.92" />
			<g clip-path="url(#{k}-clip)">
				<ellipse cx="35" cy="54" rx="5" ry="16" transform="rotate(28 35 54)" fill="url(#{k}-hi)" />
			</g>
			<path d={DROP} fill="none" stroke="url(#{k}-rim)" stroke-width={tiny ? 3 : 1.6} />
		{:else if id === 'flat'}
			<path d={DROP} fill="url(#{k}-body)" />
			<path d={DROP} fill="url(#{k}-top)" />
		{:else if id === 'deep'}
			<g filter={tiny ? undefined : `url(#${k}-shadow)`}>
				<path d={DROP} fill="url(#{k}-deep)" />
			</g>
			<path d={DROP} fill="url(#{k}-shade)" />
			<g clip-path="url(#{k}-clip)">
				<ellipse cx="38" cy="54" rx="8" ry="20" transform="rotate(28 38 54)" fill="url(#{k}-hi)" />
				{#if !tiny}<circle cx="47" cy="24" r="3" fill="#fff" opacity="0.9" />{/if}
			</g>
		{:else if id === 'liquid'}
			<path d={DROP} fill="url(#{k}-glass)" />
			<g clip-path="url(#{k}-clip)">
				<path d="M10 58 C24 52 34 64 50 58 C64 52 74 62 90 56 V100 H10 Z" fill="url(#{k}-body)" />
				<path d="M10 58 C24 52 34 64 50 58 C64 52 74 62 90 56" fill="none" stroke="#fff" stroke-opacity="0.55" stroke-width="1.5" />
				<ellipse cx="36" cy="40" rx="4.5" ry="12" transform="rotate(28 36 40)" fill="url(#{k}-hi)" />
			</g>
			<path d={DROP} fill="none" stroke="url(#{k}-rim)" stroke-width={tiny ? 3 : 1.6} />
		{/if}
	</svg>
{/snippet}

<main class="h-dvh overflow-auto bg-background p-10 text-foreground">
	<h1 class="pivot mb-2">drop workshop</h1>
	<p class="mb-8 text-muted-foreground">liquid-glass takes on the droplet, without an outline.</p>

	<div class="grid grid-cols-[6rem_repeat(6,1fr)] items-end gap-x-5 gap-y-4">
		<span></span>
		{#each variants as v (v.id)}<span class="subpivot text-center">{v.name}</span>{/each}
		<span></span>
		{#each variants as v (v.id)}<span class="text-center text-[0.7rem] text-muted-foreground lowercase">{v.note}</span>{/each}

		{#each ladder as size (size)}
			<span class="tabular self-center text-xs text-muted-foreground">{size}px</span>
			{#each variants as v (v.id)}
				<div class="flex justify-center">{@render drop(v.id, size, `l-${v.id}-${size}`)}</div>
			{/each}
		{/each}

		<span class="self-center text-xs text-muted-foreground lowercase">title bar</span>
		{#each variants as v (v.id)}
			<div class="flex items-center gap-2 bg-[#0b0b0b] px-3 py-2 text-[#999]">
				<span class="flex gap-1"><span class="size-2.5 rounded-full bg-[#ff5f57]"></span><span class="size-2.5 rounded-full bg-[#febc2e]"></span><span class="size-2.5 rounded-full bg-[#28c840]"></span></span>
				<span class="ml-2 flex items-center gap-1.5">{@render drop(v.id, 19, `tb-${v.id}`)}<span class="text-[0.9rem] font-semibold lowercase">squirt</span></span>
			</div>
		{/each}

		<span class="self-center text-xs text-muted-foreground lowercase">dock</span>
		{#each variants as v (v.id)}
			<div class="flex items-end justify-center gap-1.5 rounded-2xl bg-gradient-to-b from-[#3b2a4a] to-[#1d1427] p-2.5">
				<div class="size-10 rounded-xl bg-[#2f6fde]"></div>
				<div class="flex size-10 items-center justify-center">{@render drop(v.id, 44, `dk-${v.id}`)}</div>
				<div class="size-10 rounded-xl bg-[#e8e8e8]"></div>
			</div>
		{/each}

		<span class="self-center text-xs text-muted-foreground lowercase">light</span>
		{#each variants as v (v.id)}
			<div class="flex items-center justify-center gap-4 bg-[#f3f3f3] p-3 text-[#222]">{@render drop(v.id, 20, `lt20-${v.id}`)}{@render drop(v.id, 64, `lt64-${v.id}`)}</div>
		{/each}
	</div>
</main>
