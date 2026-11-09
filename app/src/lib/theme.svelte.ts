// Light/dark theme: follows the system unless the user picks one.
export type Theme = 'system' | 'light' | 'dark';

const KEY = 'squirt.theme';

function read(): Theme {
	try {
		const v = localStorage.getItem(KEY);
		if (v === 'light' || v === 'dark' || v === 'system') return v;
	} catch {
		// Storage can be unavailable; fall back to the system setting.
	}
	return 'system';
}

function apply(t: Theme) {
	const dark = t === 'dark' || (t === 'system' && matchMedia('(prefers-color-scheme: dark)').matches);
	document.documentElement.classList.toggle('dark', dark);
}

class ThemeState {
	value = $state<Theme>('system');

	init() {
		this.value = read();
		apply(this.value);
		matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => apply(this.value));
	}

	set(t: Theme) {
		this.value = t;
		try {
			localStorage.setItem(KEY, t);
		} catch {
			// Not remembered, but still applied.
		}
		apply(t);
	}
}

export const theme = new ThemeState();
