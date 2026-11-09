<script lang="ts">
	// Squirt's mark: a drop folded in three layers (pink, rose, amber), after
	// the 2025 Microsoft 365 icons: fluid folds, vivid analogous gradients,
	// soft shading where layers overlap. Shading is left out at small sizes.
	let { size = 20, class: className = '' }: { size?: number; class?: string } = $props();
	const uid = $props.id();
	const shaded = $derived(size >= 28);
</script>

<svg width={size * (68 / 94)} height={size} viewBox="16 4 68 94" class={className} aria-hidden="true">
	<defs>
		<linearGradient id="{uid}-pink" x1="0.15" y1="0" x2="0.85" y2="1"><stop offset="0" stop-color="#ff5cba" /><stop offset="1" stop-color="#d6007a" /></linearGradient>
		<linearGradient id="{uid}-rose" x1="0.1" y1="0" x2="0.9" y2="1"><stop offset="0" stop-color="#ff3d8a" /><stop offset="1" stop-color="#ff5a4a" /></linearGradient>
		<linearGradient id="{uid}-orange" x1="0.2" y1="0" x2="0.8" y2="1"><stop offset="0" stop-color="#ff6a4a" /><stop offset="1" stop-color="#ffb21e" /></linearGradient>
		<filter id="{uid}-soft" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="3" /></filter>
		<clipPath id="{uid}-drop"><path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 82 64 C82 44 50 6 50 6 Z" /></clipPath>
		<clipPath id="{uid}-mid"><path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 76.6 81.9 C66 72 62 58 66 44 C68 32 60 18 50 6 Z" /></clipPath>
	</defs>
	{#if shaded}
	<path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 82 64 C82 44 50 6 50 6 Z" fill="url(#{uid}-orange)" />
	<g clip-path="url(#{uid}-drop)"><path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 76.6 81.9 C66 72 62 58 66 44 C68 32 60 18 50 6 Z" transform="translate(4 2)" fill="#5c0030" opacity="0.38" filter="url(#{uid}-soft)" /></g>
	<path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 76.6 81.9 C66 72 62 58 66 44 C68 32 60 18 50 6 Z" fill="url(#{uid}-rose)" />
	<g clip-path="url(#{uid}-mid)"><path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 66.3 91.6 C52 80 46 62 55 42 C60 31 57 18 50 6 Z" transform="translate(4 2)" fill="#5c0030" opacity="0.38" filter="url(#{uid}-soft)" /></g>
	<path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 66.3 91.6 C52 80 46 62 55 42 C60 31 57 18 50 6 Z" fill="url(#{uid}-pink)" />
	{:else}
	<path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 82 64 C82 44 50 6 50 6 Z" fill="url(#{uid}-orange)" />
	<path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 76.6 81.9 C66 72 62 58 66 44 C68 32 60 18 50 6 Z" fill="url(#{uid}-rose)" />
	<path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 66.3 91.6 C52 80 46 62 55 42 C60 31 57 18 50 6 Z" fill="url(#{uid}-pink)" />
	{/if}
</svg>
