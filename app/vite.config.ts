import tailwindcss from '@tailwindcss/vite';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	server: {
		// `squirt serve` provides the real library to the browser preview.
		proxy: { '/api': 'http://127.0.0.1:5190' },
		// Transform the app's entry when the dev server starts, not when the
		// window first asks for it.
		warmup: { clientFiles: ['./src/routes/+layout.svelte', './src/routes/+page.svelte'] }
	},
	plugins: [
		tailwindcss(),
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			adapter: adapter({ fallback: 'index.html' }),
			// shadcn-svelte components import from $lib, which SvelteKit no longer defines by default.
			alias: { $lib: 'src/lib' }
		})
	]
});
