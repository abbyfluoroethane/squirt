export function bytes(n: number): string {
	if (n >= 1e9) return `${(n / 1e9).toFixed(1)} gb`;
	if (n >= 1e6) return `${(n / 1e6).toFixed(1)} mb`;
	if (n >= 1e3) return `${Math.round(n / 1e3)} kb`;
	return `${n} b`;
}

export function duration(ms: number): string {
	const s = Math.round(ms / 1000);
	return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}
