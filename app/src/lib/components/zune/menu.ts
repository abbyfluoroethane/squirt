// Entries for right-click menus (see RowMenu.svelte).
export type MenuEntry =
	| { label: string; onSelect: () => void; shortcut?: string; disabled?: boolean; destructive?: boolean }
	| 'separator';

/** What "show in finder" is called on this system. */
export const revealLabel =
	typeof navigator === 'undefined'
		? 'show in folder'
		: navigator.userAgent.includes('Mac')
			? 'show in finder'
			: navigator.userAgent.includes('Windows')
				? 'show in explorer'
				: 'show in folder';
