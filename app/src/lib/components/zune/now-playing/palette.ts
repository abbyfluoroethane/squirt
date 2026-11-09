// The main colors of a cover, for the now-playing color field.

const hsl = (h: number, s: number, l: number) => `hsl(${((h % 360) + 360) % 360} ${s}% ${l}%)`;

/** Soft greys with a hint of the album's hue, for covers with no real color. */
function neutralPalette(hue: number): string[] {
	return [hsl(hue, 12, 42), hsl(hue + 30, 8, 34), hsl(hue - 30, 10, 28)];
}

/** Colors made from the album's placeholder hue, when the cover can't be read. */
export function fallbackPalette(hue: number): string[] {
	return [hsl(hue, 70, 48), hsl(hue + 35, 65, 42), hsl(hue - 35, 60, 38)];
}

function toHsl(r: number, g: number, b: number): [number, number, number] {
	r /= 255;
	g /= 255;
	b /= 255;
	const max = Math.max(r, g, b);
	const min = Math.min(r, g, b);
	const l = (max + min) / 2;
	const d = max - min;
	if (d === 0) return [0, 0, l];
	const s = d / (1 - Math.abs(2 * l - 1));
	let h = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
	h *= 60;
	return [h < 0 ? h + 360 : h, s, l];
}

/**
 * Up to three colors that stand out in the image: vivid hues first, at
 * least 30 degrees apart. Lightness is pulled into a range that glows on
 * black. Falls back to `fallbackPalette(hue)` for grey covers or images
 * the page may not read.
 */
export async function palette(src: string | null | undefined, hue: number): Promise<string[]> {
	if (!src) return fallbackPalette(hue);
	try {
		const img = new Image();
		img.crossOrigin = 'anonymous';
		img.src = src;
		await img.decode();
		const size = 32;
		const c = document.createElement('canvas');
		c.width = c.height = size;
		const ctx = c.getContext('2d', { willReadFrequently: true })!;
		ctx.drawImage(img, 0, 0, size, size);
		const px = ctx.getImageData(0, 0, size, size).data;

		// 24 hue buckets, weighted toward saturated, mid-light pixels.
		const bins = Array.from({ length: 24 }, () => ({ w: 0, h: 0, s: 0, l: 0 }));
		for (let i = 0; i < px.length; i += 4) {
			const [h, s, l] = toHsl(px[i], px[i + 1], px[i + 2]);
			const w = s * s * (1 - Math.abs(2 * l - 1));
			if (w < 0.02) continue;
			const b = bins[Math.floor(h / 15) % 24];
			b.w += w;
			b.h += h * w;
			b.s += s * w;
			b.l += l * w;
		}
		const picked: { h: number; s: number; l: number }[] = [];
		// A hue must carry about 0.5% of the image (at full weight) to count,
		// so a few stray pixels on a grey cover don't color the whole screen.
		const minWeight = (size * size) * 0.005;
		for (const b of [...bins].sort((a, z) => z.w - a.w)) {
			if (b.w < minWeight || picked.length === 3) break;
			const h = b.h / b.w;
			if (picked.some((p) => Math.min(Math.abs(p.h - h), 360 - Math.abs(p.h - h)) < 30)) continue;
			picked.push({ h, s: b.s / b.w, l: b.l / b.w });
		}
		if (!picked.length) return neutralPalette(hue);
		// Fewer than three hues: fill in with neighbors of the strongest.
		while (picked.length < 3) {
			const p = picked[0];
			picked.push({ h: p.h + (picked.length === 1 ? 25 : -25), s: p.s * 0.85, l: p.l * 0.85 });
		}
		return picked.map((p) => hsl(p.h, Math.round(Math.min(90, Math.max(45, p.s * 100))), Math.round(Math.min(58, Math.max(36, p.l * 100)))));
	} catch {
		return fallbackPalette(hue);
	}
}
