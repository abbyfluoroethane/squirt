<script lang="ts">
	// Fluid workshop: the droplet in the style of the 2025 Microsoft 365 icons.
	// Folded, overlapping layers; vivid analogous gradients (pink, magenta,
	// coral, orange); soft shading where layers overlap; no gloss. Shading
	// drops away below ~28px, as the Office icons simplify when small.
	const variants = [
		{ id: 'fold3', name: 'fold 3', note: 'three folded layers: pink, rose, amber' },
		{ id: 'fold', name: 'fold', note: 'drop folded on an s-curve: pink front over coral back' },
		{ id: 'layers', name: 'layers', note: 'three stacked, offset drops: a splash' },
		{ id: 'wrap', name: 'wrap', note: 'a magenta band curling round an orange core' },
		{ id: 'splash', name: 'splash', note: 'a droplet breaking off the shoulder' },
		{ id: 'wave', name: 'wave', note: 'a magenta wave folding across a coral drop' }
	] as const;
	const ladder = [20, 32, 48, 128, 256];

	// Drop centred in a 100x100 box: tip (50,6), round bottom centre (50,64) r32.
	const DROP = 'M50 6 C50 6 18 44 18 64 A32 32 0 0 0 82 64 C82 44 50 6 50 6 Z';
	// Drop shape scaled by s about (cx, cy).
	const dropAt = (s: number, cx: number, cy: number) =>
		`translate(${cx - 50 * s} ${cy - 52 * s}) scale(${s})`;
</script>

<svelte:head><title>Squirt fluid workshop</title></svelte:head>

{#snippet icon(id: string, size: number, k: string)}
	{@const small = size < 28}
	<svg width={size} height={size} viewBox="0 0 100 100" aria-hidden="true">
		<defs>
			<!-- Front layers: hot pink to magenta. -->
			<linearGradient id="{k}-pink" x1="0.15" y1="0" x2="0.85" y2="1">
				<stop offset="0" stop-color="#ff5cba" />
				<stop offset="1" stop-color="#d6007a" />
			</linearGradient>
			<!-- Middle: rose to coral. -->
			<linearGradient id="{k}-rose" x1="0.1" y1="0" x2="0.9" y2="1">
				<stop offset="0" stop-color="#ff3d8a" />
				<stop offset="1" stop-color="#ff5a4a" />
			</linearGradient>
			<!-- Back layers: coral to amber. -->
			<linearGradient id="{k}-orange" x1="0.2" y1="0" x2="0.8" y2="1">
				<stop offset="0" stop-color="#ff6a4a" />
				<stop offset="1" stop-color="#ffb21e" />
			</linearGradient>
			<filter id="{k}-soft" x="-20%" y="-20%" width="140%" height="140%">
				<feGaussianBlur stdDeviation="3" />
			</filter>
			<clipPath id="{k}-drop"><path d={DROP} /></clipPath>
		</defs>

		<!-- Soft shading cast by `shape` onto whatever is beneath it, clipped to `clip`. -->
		{#snippet shade(shape: string, transform: string, dx: number, dy: number, clip: string)}
			{#if !small}
				<g clip-path="url(#{clip})">
					<g transform="translate({dx} {dy})" filter="url(#{k}-soft)">
						<path d={shape} {transform} fill="#5c0030" opacity="0.38" />
					</g>
				</g>
			{/if}
		{/snippet}

		{#if id === 'fold3'}
			{@const FRONT = 'M50 6 C50 6 18 44 18 64 A32 32 0 0 0 66.3 91.6 C52 80 46 62 55 42 C60 31 57 18 50 6 Z'}
			{@const MID = 'M50 6 C50 6 18 44 18 64 A32 32 0 0 0 76.6 81.9 C66 72 62 58 66 44 C68 32 60 18 50 6 Z'}
			<clipPath id="{k}-mid"><path d={MID} /></clipPath>
			<path d={DROP} fill="url(#{k}-orange)" />
			{@render shade(MID, '', 4, 2, `${k}-drop`)}
			<path d={MID} fill="url(#{k}-rose)" />
			{@render shade(FRONT, '', 4, 2, `${k}-mid`)}
			<path d={FRONT} fill="url(#{k}-pink)" />
		{:else if id === 'fold'}
			{@const FRONT = 'M50 6 C50 6 18 44 18 64 A32 32 0 0 0 66.3 91.6 C52 80 46 62 55 42 C60 31 57 18 50 6 Z'}
			<path d={DROP} fill="url(#{k}-orange)" />
			{@render shade(FRONT, '', 4, 2, `${k}-drop`)}
			<path d={FRONT} fill="url(#{k}-pink)" />
		{:else if id === 'layers'}
			<clipPath id="{k}-back"><path d={DROP} transform={dropAt(0.78, 60, 42)} /></clipPath>
			<clipPath id="{k}-mid"><path d={DROP} transform={dropAt(0.78, 50, 52)} /></clipPath>
			<path d={DROP} transform={dropAt(0.78, 60, 42)} fill="url(#{k}-orange)" />
			{@render shade(DROP, dropAt(0.78, 50, 52), 3, 2, `${k}-back`)}
			<path d={DROP} transform={dropAt(0.78, 50, 52)} fill="url(#{k}-rose)" />
			{@render shade(DROP, dropAt(0.78, 40, 62), 3, 2, `${k}-mid`)}
			<path d={DROP} transform={dropAt(0.78, 40, 62)} fill="url(#{k}-pink)" />
		{:else if id === 'wrap'}
			{@const CORE = 'M50 34 C50 34 34 54 34 66 A16 16 0 0 0 66 66 C66 54 50 34 50 34 Z'}
			<!-- Band: the drop with a hole, open on the right where it curls over the core. -->
			{@const BAND = 'M50 6 C50 6 18 44 18 64 A32 32 0 0 0 82 64 C82 52 72 36 62 22 C64 34 70 46 70 60 C70 54 64 46 58 40 Z'}
			<path d={DROP} fill="url(#{k}-orange)" />
			<path d={CORE} fill="#ffd27a" opacity="0.0" />
			{@render shade(BAND, '', 2, 3, `${k}-drop`)}
			<path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 82 64 C82 58 80 52 77 46 C74 60 64 74 50 74 A12 12 0 0 1 38 62 C38 50 50 30 50 30 Z" fill="url(#{k}-pink)" />
		{:else if id === 'splash'}
			<clipPath id="{k}-main"><path d={DROP} transform={dropAt(0.88, 44, 56)} /></clipPath>
			<path d={DROP} transform={dropAt(0.88, 44, 56)} fill="url(#{k}-pink)" />
			{@render shade('M78 26 A14 14 0 1 1 77.99 26 Z', '', 2, 3, `${k}-main`)}
			<circle cx="78" cy="26" r="13" fill="url(#{k}-orange)" />
		{:else if id === 'wave'}
			{@const WAVE = 'M10 60 C26 48 40 70 56 58 C66 50 76 50 90 56 V100 H10 Z'}
			<path d={DROP} fill="url(#{k}-orange)" />
			<g clip-path="url(#{k}-drop)">
				{@render shade(WAVE, '', 0, -3, `${k}-drop`)}
				<path d={WAVE} fill="url(#{k}-pink)" />
			</g>
		{/if}
	</svg>
{/snippet}

<main class="min-h-dvh bg-background p-10 text-foreground">
	<h1 class="pivot mb-2">fluid workshop</h1>
	<p class="mb-8 max-w-3xl text-muted-foreground">
		the droplet after the 2025 microsoft 365 icons: folded, overlapping layers, vivid analogous gradients, soft shading where layers meet, no gloss.
	</p>

	<div class="grid grid-cols-[6rem_repeat(6,1fr)] items-end gap-x-6 gap-y-4">
		<span></span>
		{#each variants as v (v.id)}<span class="subpivot text-center">{v.name}</span>{/each}
		<span></span>
		{#each variants as v (v.id)}<span class="text-center text-[0.7rem] text-muted-foreground lowercase">{v.note}</span>{/each}

		{#each ladder as size (size)}
			<span class="tabular self-center text-xs text-muted-foreground">{size}px</span>
			{#each variants as v (v.id)}
				<div class="flex justify-center">{@render icon(v.id, size, `l-${v.id}-${size}`)}</div>
			{/each}
		{/each}

		<span class="self-center text-xs text-muted-foreground lowercase">title bar</span>
		{#each variants as v (v.id)}
			<div class="flex items-center gap-2 bg-[#0b0b0b] px-3 py-2 text-[#999]">
				<span class="flex gap-1"><span class="size-2.5 rounded-full bg-[#ff5f57]"></span><span class="size-2.5 rounded-full bg-[#febc2e]"></span><span class="size-2.5 rounded-full bg-[#28c840]"></span></span>
				<span class="ml-2 flex items-center gap-1.5">{@render icon(v.id, 20, `tb-${v.id}`)}<span class="text-[0.9rem] font-semibold lowercase">squirt</span></span>
			</div>
		{/each}

		<span class="self-center text-xs text-muted-foreground lowercase">dock</span>
		{#each variants as v (v.id)}
			<div class="flex items-end justify-center gap-1.5 rounded-2xl bg-gradient-to-b from-[#3b2a4a] to-[#1d1427] p-2.5">
				<div class="size-10 rounded-xl bg-[#2f6fde]"></div>
				<div class="flex size-10 items-center justify-center">{@render icon(v.id, 44, `dk-${v.id}`)}</div>
				<div class="size-10 rounded-xl bg-[#e8e8e8]"></div>
			</div>
		{/each}

		<span class="self-center text-xs text-muted-foreground lowercase">light</span>
		{#each variants as v (v.id)}
			<div class="flex items-center justify-center gap-4 bg-[#f3f3f3] p-3">{@render icon(v.id, 20, `lt20-${v.id}`)}{@render icon(v.id, 64, `lt64-${v.id}`)}</div>
		{/each}
	</div>
</main>
