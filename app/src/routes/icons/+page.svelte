<script lang="ts">
	// Icon workshop: candidate marks compared at real sizes on dark and light.
	// Each candidate keeps the gradient to a small accent.
	const concepts = [
		{ id: 'beam', name: 'beam', note: 'zune hd sending: three gradient arcs off its corner' },
		{ id: 'handoff', name: 'handoff', note: 'two zunes facing, a song passing between them' },
		{ id: 'droplet', name: 'droplet', note: 'a squirt as a metro drop, with a song "screen" inside' },
		{ id: 's', name: 's', note: 'lowercase selawik light "s" and a thin power bar' },
		{ id: 'sbeam', name: 's beam', note: 'the "s" sending: arcs off its shoulder' },
		{ id: 'wordmark', name: 'wordmark', note: '"squirt" lowercase on black, power bar under it' },
		{ id: 'flight', name: 'in flight', note: 'a music note trailing gradient motion arcs' }
	] as const;
	const sizes = [20, 48, 128];
	const finalists = ['sbeam', 'beam', 's', 'droplet'] as const;
	const names: Record<string, string> = { sbeam: 's beam', beam: 'beam', s: 's', droplet: 'droplet' };
	const ladder = [20, 32, 48, 128, 256];
</script>

<svelte:head><title>Squirt icon workshop</title></svelte:head>

{#snippet grad(id: string)}
	<linearGradient id={id} x1="0" y1="0" x2="1" y2="1">
		<stop offset="0" stop-color="#ec008c" />
		<stop offset="1" stop-color="#f7941d" />
	</linearGradient>
{/snippet}

<!-- Zune HD body in a 53x102 box; `screen` fills the screen, `glass` the face. -->
{#snippet zune(x: number, y: number, s: number, screen: string, rot = 0)}
	<g transform="translate({x} {y}) rotate({rot} {26.5 * s} {51 * s}) scale({s})">
		<rect x="0.75" y="0.75" width="51.5" height="100.5" rx="3.4" fill="#c9c1b3" />
		<path d="M0.75 84.5 V4.15 A3.4 3.4 0 0 1 4.15 0.75 H48.85 A3.4 3.4 0 0 1 52.25 4.15 V84.5 H35.6 L33.4 88.6 H19.6 L17.4 84.5 Z" fill="#0d0d0d" />
		<rect x="6" y="7" width="41" height="72.9" fill={screen} />
		<rect x="0.75" y="0.75" width="51.5" height="100.5" rx="3.4" fill="none" stroke="#888" stroke-opacity="0.45" stroke-width="1.5" />
	</g>
{/snippet}

{#snippet icon(id: string, size: number, key: string)}
	<svg width={size} height={size} viewBox="0 0 100 100" aria-hidden="true">
		<defs>{@render grad(`${key}-g`)}</defs>
		{#if id === 'beam'}
			{@render zune(12, 36, 0.6, '#1c1c1c')}
			<!-- Concentric arcs centred on the device's top-right corner (44, 36). -->
			<g fill="none" stroke="url(#{key}-g)" stroke-linecap="round" stroke-width="5.5">
				<path d="M44 22 A14 14 0 0 1 58 36" />
				<path d="M44 8 A28 28 0 0 1 72 36" opacity="0.75" />
			</g>
		{:else if id === 'handoff'}
			{@render zune(8, 26, 0.5, '#1c1c1c', -12)}
			{@render zune(66, 26, 0.5, '#1c1c1c', 12)}
			<g fill="none" stroke="url(#{key}-g)" stroke-linecap="round" stroke-width="5">
				<path d="M42 42 Q50 34 58 42" />
				<path d="M38 52 Q50 40 62 52" opacity="0.75" />
			</g>
		{:else if id === 'droplet'}
			<path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 82 64 C82 44 50 6 50 6 Z" fill="#0d0d0d" stroke="#888" stroke-opacity="0.45" stroke-width="2" />
			<path d="M50 30 C50 30 32 52 32 64 A18 18 0 0 0 68 64 C68 52 50 30 50 30 Z" fill="url(#{key}-g)" />
		{:else if id === 's'}
			<rect width="100" height="100" rx="18" fill="#0d0d0d" />
			<text x="50" y="70" text-anchor="middle" font-family="Selawik" font-weight="300" font-size="84" fill="#fff">s</text>
			<rect x="30" y="82" width="40" height="5" fill="url(#{key}-g)" />
		{:else if id === 'sbeam'}
			<rect width="100" height="100" rx="18" fill="#0d0d0d" />
			<text x="42" y="76" text-anchor="middle" font-family="Selawik" font-weight="300" font-size="84" fill="#fff">s</text>
			<g fill="none" stroke="url(#{key}-g)" stroke-linecap="round" stroke-width="5">
				<path d="M64 28 A10 10 0 0 1 74 38" />
				<path d="M64 16 A22 22 0 0 1 86 38" opacity="0.75" />
			</g>
		{:else if id === 'wordmark'}
			<rect width="100" height="100" rx="18" fill="#0d0d0d" />
			<text x="50" y="58" text-anchor="middle" font-family="Selawik" font-weight="300" font-size="30" fill="#fff">squirt</text>
			<rect x="22" y="68" width="56" height="4" fill="url(#{key}-g)" />
		{:else if id === 'flight'}
			<g fill="none" stroke="url(#{key}-g)" stroke-linecap="round" stroke-width="5">
				<path d="M8 66 Q20 52 34 58" opacity="0.5" />
				<path d="M6 80 Q24 62 40 70" />
			</g>
			<g fill="currentColor">
				<rect x="56" y="18" width="6" height="56" />
				<path d="M62 18 C72 22 84 26 86 42 C80 32 72 32 62 32 Z" />
				<ellipse cx="50" cy="76" rx="14" ry="10" transform="rotate(-20 50 76)" />
			</g>
		{/if}
	</svg>
{/snippet}

<main class="min-h-dvh bg-background p-10 text-foreground">
	<h1 class="pivot mb-2">icon workshop</h1>
	<p class="mb-8 text-muted-foreground">finalists side by side, then every concept below.</p>

	<section class="mb-14 flex flex-col gap-6">
		<h2 class="subpivot">side by side</h2>
		<div class="grid grid-cols-[6rem_repeat(4,1fr)] items-end gap-x-6 gap-y-4">
			<span></span>
			{#each finalists as f (f)}<span class="subpivot text-center">{names[f]}</span>{/each}
			{#each ladder as size (size)}
				<span class="tabular self-center text-xs text-muted-foreground">{size}px</span>
				{#each finalists as f (f)}
					<div class="flex justify-center">{@render icon(f, size, `ladder-${f}-${size}`)}</div>
				{/each}
			{/each}

			<span class="self-center text-xs text-muted-foreground lowercase">title bar</span>
			{#each finalists as f (f)}
				<div class="flex items-center gap-2.5 bg-[#0b0b0b] px-3 py-2 text-[#999]">
					<span class="flex gap-1.5"><span class="size-3 rounded-full bg-[#ff5f57]"></span><span class="size-3 rounded-full bg-[#febc2e]"></span><span class="size-3 rounded-full bg-[#28c840]"></span></span>
					<span class="ml-3 flex items-center gap-2">{@render icon(f, 20, `tb-${f}`)}<span class="text-[0.95rem] font-semibold lowercase">squirt</span></span>
				</div>
			{/each}

			<span class="self-center text-xs text-muted-foreground lowercase">dock</span>
			{#each finalists as f (f)}
				<div class="flex items-end justify-center gap-2 rounded-2xl bg-gradient-to-b from-[#3b2a4a] to-[#1d1427] p-3">
					<div class="size-12 rounded-xl bg-[#2f6fde]"></div>
					<div class="flex size-12 items-center justify-center">{@render icon(f, 48, `dock-${f}`)}</div>
					<div class="size-12 rounded-xl bg-[#e8e8e8]"></div>
				</div>
			{/each}

			<span class="self-center text-xs text-muted-foreground lowercase">light</span>
			{#each finalists as f (f)}
				<div class="flex items-center justify-center gap-4 bg-[#f3f3f3] p-3 text-[#222]">{@render icon(f, 20, `l20-${f}`)}{@render icon(f, 64, `l64-${f}`)}</div>
			{/each}
		</div>
	</section>

	<h2 class="subpivot mb-4">all concepts</h2>
	<div class="grid grid-cols-2 gap-6 xl:grid-cols-3">
		{#each concepts as c (c.id)}
			<section class="flex min-w-0 flex-col gap-3 border p-5">
				<div class="flex items-baseline justify-between">
					<h2 class="subpivot">{c.name}</h2>
					<span class="text-xs text-muted-foreground lowercase">{c.note}</span>
				</div>
				<div class="flex">
					{#each ['dark', 'light'] as bg (bg)}
						<div class="flex min-w-0 flex-1 items-center gap-4 overflow-hidden p-4 {bg === 'dark' ? 'bg-[#0b0b0b] text-[#e8e8e8]' : 'bg-[#f3f3f3] text-[#222]'}">
							{#each sizes as s (s)}
								{@render icon(c.id, s, `${c.id}-${bg}-${s}`)}
							{/each}
						</div>
					{/each}
				</div>
				<div class="flex items-center gap-2.5 bg-[#0b0b0b] px-3 py-2 text-[#999]">
					{@render icon(c.id, 20, `${c.id}-bar`)}<span class="text-[0.95rem] font-semibold lowercase">squirt</span>
					<span class="ml-auto text-xs">title bar</span>
				</div>
			</section>
		{/each}
	</div>
</main>
