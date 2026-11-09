<script lang="ts">
	// Squirt's mark: a drop folded in three layers (pink, rose, amber), after
	// the 2025 Microsoft 365 icons: fluid folds, vivid analogous gradients,
	// soft shading where layers overlap. Shading is left out at small sizes.
	// `animate` plays the intro once: the drop falls in as plain amber, lands
	// with a small squash and a ripple, then the rose and pink layers fold out
	// from its left edge.
	let { size = 20, class: className = '', animate = false }: { size?: number; class?: string; animate?: boolean } = $props();
	const uid = $props.id();
	const shaded = $derived(size >= 28);
</script>

<svg width={size * (68 / 94)} height={size} viewBox="16 4 68 94" class="{className} {animate ? 'intro' : ''}" overflow="visible" aria-hidden="true">
	<defs>
		<linearGradient id="{uid}-pink" x1="0.15" y1="0" x2="0.85" y2="1"><stop offset="0" stop-color="#ff5cba" /><stop offset="1" stop-color="#d6007a" /></linearGradient>
		<linearGradient id="{uid}-rose" x1="0.1" y1="0" x2="0.9" y2="1"><stop offset="0" stop-color="#ff3d8a" /><stop offset="1" stop-color="#ff5a4a" /></linearGradient>
		<linearGradient id="{uid}-orange" x1="0.2" y1="0" x2="0.8" y2="1"><stop offset="0" stop-color="#ff6a4a" /><stop offset="1" stop-color="#ffb21e" /></linearGradient>
		<filter id="{uid}-soft" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="3" /></filter>
		<clipPath id="{uid}-drop"><path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 82 64 C82 44 50 6 50 6 Z" /></clipPath>
		<clipPath id="{uid}-mid"><path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 76.6 81.9 C66 72 62 58 66 44 C68 32 60 18 50 6 Z" /></clipPath>
	</defs>
	{#if animate}<ellipse class="ripple" cx="50" cy="96" rx="30" ry="3.5" fill="none" stroke="url(#{uid}-orange)" stroke-width="1.5" />{/if}
	<g class="drop">
	{#if shaded}
	<path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 82 64 C82 44 50 6 50 6 Z" fill="url(#{uid}-orange)" />
	<g class="layer rose">
	<g clip-path="url(#{uid}-drop)"><path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 76.6 81.9 C66 72 62 58 66 44 C68 32 60 18 50 6 Z" transform="translate(4 2)" fill="#5c0030" opacity="0.38" filter="url(#{uid}-soft)" /></g>
	<path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 76.6 81.9 C66 72 62 58 66 44 C68 32 60 18 50 6 Z" fill="url(#{uid}-rose)" />
	</g>
	<g class="layer pink">
	<g clip-path="url(#{uid}-mid)"><path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 66.3 91.6 C52 80 46 62 55 42 C60 31 57 18 50 6 Z" transform="translate(4 2)" fill="#5c0030" opacity="0.38" filter="url(#{uid}-soft)" /></g>
	<path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 66.3 91.6 C52 80 46 62 55 42 C60 31 57 18 50 6 Z" fill="url(#{uid}-pink)" />
	</g>
	{:else}
	<path d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 82 64 C82 44 50 6 50 6 Z" fill="url(#{uid}-orange)" />
	<path class="layer rose" d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 76.6 81.9 C66 72 62 58 66 44 C68 32 60 18 50 6 Z" fill="url(#{uid}-rose)" />
	<path class="layer pink" d="M50 6 C50 6 18 44 18 64 A32 32 0 0 0 66.3 91.6 C52 80 46 62 55 42 C60 31 57 18 50 6 Z" fill="url(#{uid}-pink)" />
	{/if}
	</g>
</svg>

<style>
	/* 0-0.55s fall (speeding up), 0.55-0.9s squash and settle, ripple from
	   0.55s, layers fold out from 0.75s, 80ms apart. */
	.intro .drop {
		transform-box: fill-box;
		transform-origin: 50% 100%;
		animation: squirt-fall 0.9s both;
	}
	/* Each layer unfolds about the drop's tip (x 50), so the tips stay
	   together: 55% of the rose layer's width, 66% of the pink's. */
	.intro .layer {
		transform-box: fill-box;
		transform-origin: 55% 50%;
		animation: squirt-fold 0.7s cubic-bezier(0.1, 0.9, 0.2, 1) 0.75s both;
	}
	.intro .layer.pink {
		transform-origin: 66% 50%;
		animation-delay: 0.83s;
	}
	/* Hidden until the drop lands (forwards, not both, so the delay shows nothing). */
	.intro .ripple {
		opacity: 0;
		transform-box: fill-box;
		transform-origin: 50% 50%;
		animation: squirt-ripple 0.9s cubic-bezier(0.2, 0.7, 0.3, 1) 0.55s forwards;
	}
	@keyframes squirt-fall {
		0% {
			opacity: 0;
			transform: translateY(-140%) scale(0.9, 1.08);
			animation-timing-function: cubic-bezier(0.55, 0, 1, 0.45);
		}
		15% {
			opacity: 1;
		}
		61% {
			transform: translateY(0) scale(0.94, 1.06);
			animation-timing-function: cubic-bezier(0.2, 0.8, 0.3, 1);
		}
		75% {
			transform: scale(1.07, 0.9);
			animation-timing-function: cubic-bezier(0.3, 0, 0.2, 1);
		}
		100% {
			opacity: 1;
			transform: none;
		}
	}
	@keyframes squirt-fold {
		from {
			opacity: 0;
			transform: scaleX(0.2);
		}
		to {
			opacity: 1;
			transform: none;
		}
	}
	@keyframes squirt-ripple {
		from {
			opacity: 0.9;
			transform: scale(0.35, 0.6);
		}
		to {
			opacity: 0;
			transform: scale(1.7, 1.4);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.intro .drop,
		.intro .layer {
			animation: none;
		}
		.intro .ripple {
			display: none;
		}
	}
</style>
