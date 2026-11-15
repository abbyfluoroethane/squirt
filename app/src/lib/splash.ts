// The splash is plain HTML in app.html, so it is on screen before any
// script runs. The page calls these once it has something to show.

/** The splash stays up at least this long, so a fast start doesn't flash it. */
const MINIMUM_MS = 1000;

/** Resolves when the splash has been on screen for its minimum time. */
export function splashMinimum(): Promise<void> {
	const at = (window as { __splashAt?: number }).__splashAt ?? 0;
	const left = MINIMUM_MS - (performance.now() - at);
	return left > 0 ? new Promise((r) => setTimeout(r, left)) : Promise.resolve();
}

/** Fades the splash out and removes it. */
export function hideSplash() {
	const el = document.getElementById('splash');
	if (!el) return;
	el.classList.add('out');
	setTimeout(() => el.remove(), 300);
}
