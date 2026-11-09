<script lang="ts">
	// Now-playing background: the spectrum as an abstract, mirrored shape
	// across the full width. Bass sits in the middle and higher bands spread
	// to both edges; the shape is mirrored above and below a center line.
	// Three translucent layers in the Zune gradient, each a little smaller
	// and wobbling on its own slow phase, add up where they overlap. When
	// nothing plays it rests as a thin glowing line.
	//
	// Levels come from the player about 30 times a second; each frame eases
	// toward them (fast up, slow down).
	import { onMount } from 'svelte';

	let { spectrum, bands = 40 }: { spectrum: (bands: number) => Promise<number[]>; bands?: number } = $props();

	let canvas: HTMLCanvasElement;
	let box: HTMLDivElement;

	// Layers, outermost first: size, opacity, wobble speed and phase.
	const LAYERS = [
		{ scale: 1, alpha: 0.2, speed: 0.7, phase: 0 },
		{ scale: 0.68, alpha: 0.3, speed: 1.1, phase: 2.1 },
		{ scale: 0.4, alpha: 0.45, speed: 1.6, phase: 4.2 }
	];

	onMount(() => {
		const target = new Float32Array(bands);
		const level = new Float32Array(bands);
		let alive = true;
		let busy = false;

		const poll = setInterval(async () => {
			if (busy) return;
			busy = true;
			try {
				const v = await spectrum(bands);
				for (let i = 0; i < bands; i++) target[i] = v[i] ?? 0;
			} catch {
				target.fill(0);
			} finally {
				busy = false;
			}
		}, 33);

		const ctx = canvas.getContext('2d')!;
		let last = performance.now();
		const draw = (now: number) => {
			if (!alive) return;
			const dt = Math.min(0.1, (now - last) / 1000);
			last = now;
			const t = now / 1000;
			const dpr = devicePixelRatio || 1;
			// Size from the wrapper, not the canvas: a canvas's own size can
			// follow its pixel buffer.
			const w = box.clientWidth;
			const h = box.clientHeight;
			if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) {
				canvas.width = Math.round(w * dpr);
				canvas.height = Math.round(h * dpr);
			}
			ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
			ctx.clearRect(0, 0, w, h);

			for (let i = 0; i < bands; i++) {
				const k = 1 - Math.exp(-dt * (target[i] > level[i] ? 24 : 5));
				level[i] += (target[i] - level[i]) * k;
			}

			// Bands mirrored left-right: highs at the edges, bass in the middle.
			// Lows tend to be louder, so taper them a little to keep the middle
			// from always filling the whole height.
			const n = bands * 2;
			const amp = (j: number) => {
				const i = j < bands ? bands - 1 - j : j - bands;
				const tilt = 0.75 + 0.25 * (i / (bands - 1));
				return Math.pow(level[i], 1.3) * tilt;
			};

			// --zune-magenta and --zune-orange, in hex (canvas may not parse
			// oklch): orange in the middle, magenta at the edges.
			const grad = ctx.createLinearGradient(0, 0, w, 0);
			grad.addColorStop(0, '#de007d');
			grad.addColorStop(0.5, '#f78c28');
			grad.addColorStop(1, '#de007d');

			const mid = h / 2;
			ctx.globalCompositeOperation = 'lighter';
			for (const L of LAYERS) {
				// Points across the full width (one past each edge so the
				// curve leaves the frame cleanly).
				const pts: [number, number][] = [];
				for (let j = -1; j <= n; j++) {
					const jj = Math.min(n - 1, Math.max(0, j));
					const wobble = 0.85 + 0.15 * Math.sin(t * L.speed + jj * 0.35 + L.phase);
					const a = Math.max(0.004, amp(jj) * wobble * L.scale);
					pts.push([((j + 0.5) / n) * w, a * mid]);
				}
				ctx.beginPath();
				// Top edge left to right, then the mirrored bottom edge back,
				// smoothed by curving through the midpoints between samples.
				ctx.moveTo(pts[0][0], mid - pts[0][1]);
				for (let p = 1; p < pts.length - 1; p++) {
					const [x, y] = pts[p];
					const [nx, ny] = pts[p + 1];
					ctx.quadraticCurveTo(x, mid - y, (x + nx) / 2, mid - (y + ny) / 2);
				}
				ctx.lineTo(pts[pts.length - 1][0], mid - pts[pts.length - 1][1]);
				ctx.lineTo(pts[pts.length - 1][0], mid + pts[pts.length - 1][1]);
				for (let p = pts.length - 2; p > 0; p--) {
					const [x, y] = pts[p];
					const [px, py] = pts[p - 1];
					ctx.quadraticCurveTo(x, mid + y, (x + px) / 2, mid + (y + py) / 2);
				}
				ctx.lineTo(pts[0][0], mid + pts[0][1]);
				ctx.closePath();
				ctx.globalAlpha = L.alpha;
				ctx.fillStyle = grad;
				ctx.fill();
			}
			ctx.globalAlpha = 1;
			ctx.globalCompositeOperation = 'source-over';
			requestAnimationFrame(draw);
		};
		requestAnimationFrame(draw);

		return () => {
			alive = false;
			clearInterval(poll);
		};
	});
</script>

<!-- Edge to edge, centered in the open space above the art; slightly
     blurred so it reads as light rather than a graph. (A canvas needs an
     explicit width: left/right alone don't stretch it, so it fills a
     wrapper that does.) -->
<div bind:this={box} class="pointer-events-none absolute inset-x-0 top-[6%] h-[46%]" aria-hidden="true">
	<canvas bind:this={canvas} class="absolute inset-0 blur-[3px]" style="width: 100%; height: 100%"></canvas>
</div>
