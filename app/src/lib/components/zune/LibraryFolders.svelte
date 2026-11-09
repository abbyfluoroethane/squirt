<script lang="ts">
	// Settings: which folders each kind of media comes from, and rescanning.
	import { onMount } from 'svelte';
	import { pickFolder, type Folder, type MediaApi, type MediaKind, type ScanStatus } from '$lib/api';
	import { Button } from '$lib/components/ui/button';
	import { FolderSimple, Plus, X } from 'phosphor-svelte';

	let { media, onScanned }: { media: MediaApi; onScanned?: () => void } = $props();

	const SECTIONS: { kind: MediaKind; title: string; hint: string }[] = [
		{ kind: 'music', title: 'music', hint: 'mp3, aac, flac, alac, ogg, wav' },
		{ kind: 'video', title: 'videos', hint: 'mp4, m4v, mov, wmv, mkv' },
		{ kind: 'picture', title: 'pictures', hint: 'jpg, png, gif, bmp · subfolders become albums' },
		{ kind: 'podcast', title: 'podcasts', hint: 'audio · the album tag names the show' }
	];

	let folders = $state<Folder[]>([]);
	let scan = $state<ScanStatus | null>(null);
	let error = $state<string | null>(null);

	async function poll() {
		const was = scan?.running;
		scan = await media.scanStatus();
		if (was && !scan.running) onScanned?.();
	}

	onMount(() => {
		media.folders().then((f) => (folders = f));
		poll();
		const t = setInterval(poll, 1500);
		return () => clearInterval(t);
	});

	async function add(kind: MediaKind, title: string) {
		error = null;
		const path = await pickFolder(`add a ${title} folder`);
		if (!path) return;
		try {
			folders = await media.addFolder(path, kind);
			poll();
		} catch (e) {
			error = String(e instanceof Error ? e.message : e);
		}
	}

	async function remove(id: number) {
		folders = await media.removeFolder(id);
		onScanned?.();
	}

	const tilde = (p: string) => p.replace(/^\/Users\/[^/]+/, '~');
</script>

<div class="flex flex-col gap-6">
	{#each SECTIONS as s, i (s.kind)}
		{@const list = folders.filter((f) => f.kind === s.kind)}
		<div class="metro-enter flex flex-col gap-2" style="--i: {i + 1}">
			<div class="flex items-baseline gap-3">
				<h3 class="text-[1.05rem] font-[350] lowercase">{s.title}</h3>
				<span class="text-xs text-muted-foreground">{s.hint}</span>
			</div>
			{#each list as f (f.id)}
				<div class="group flex items-center gap-2.5 py-0.5">
					<FolderSimple size={16} class="shrink-0 text-muted-foreground" />
					<span class="selectable min-w-0 flex-1 truncate font-mono text-xs">{tilde(f.path)}</span>
					<button class="text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 hover:text-foreground" onclick={() => remove(f.id)} aria-label="stop watching {f.path}" title="stop watching (files are not deleted)">
						<X size={14} />
					</button>
				</div>
			{:else}
				<p class="text-xs text-muted-foreground">no folders yet.</p>
			{/each}
			<Button size="sm" variant="outline" class="w-fit lowercase" onclick={() => add(s.kind, s.title)}><Plus />add folder</Button>
		</div>
	{/each}

	<div class="metro-enter flex items-center gap-3" style="--i: 5">
		<Button size="sm" variant="outline" class="lowercase" disabled={scan?.running} onclick={async () => { scan = await media.rescan(); }}>rescan now</Button>
		<span class="text-xs text-muted-foreground">
			{#if scan?.running}
				scanning… <span class="tabular">{scan.done.toLocaleString()}</span> of <span class="tabular">{scan.total.toLocaleString()}</span>
			{:else if scan?.last}
				{scan.last}{scan.errors ? ` · ${scan.errors} unreadable` : ''}
			{/if}
		</span>
	</div>
	{#if error}<p class="text-xs text-destructive">{error}</p>{/if}
</div>
