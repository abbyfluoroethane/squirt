#!/usr/bin/env node
// Renders every Zune illustration (src/lib/components/zune/art/*.ts) into
// one HTML page, and optionally screenshots it.
//
//   node scripts/art-gallery.mjs                       -> build/art-gallery.html
//   node scripts/art-gallery.mjs --out page.html
//   node scripts/art-gallery.mjs --only zune30 --png /tmp/zune30.png
//
// The page is self-contained (fonts inlined), so it can be opened or
// published anywhere. Screenshots take a lock so only one headless Chrome
// runs at a time, even with several people (or agents) rendering.

import { readFileSync, writeFileSync, readdirSync, mkdirSync, rmdirSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..');
const artDir = join(root, 'src/lib/components/zune/art');

const args = process.argv.slice(2);
const opt = (name) => {
	const i = args.indexOf(`--${name}`);
	return i >= 0 ? args[i + 1] : undefined;
};
const only = opt('only');
const png = opt('png');
// --artifact: write just the page content (the host adds the document
// wrapper), with side gutters that clear a phone's system bars.
const artifact = args.includes('--artifact');
const out = resolve(opt('out') ?? (png ? png.replace(/\.png$/, '.html') : join(root, 'build/art-gallery.html')));

// A fixed clock so renders are repeatable.
const now = new Date(2026, 10, 9, 12, 15);

const files = readdirSync(artDir)
	.filter((f) => f.endsWith('.ts') && !['types.ts', 'index.ts', 'classic.ts'].includes(f))
	.sort();

const sections = [];
const problems = [];
const ORDER = ['zune30', 'zuneflash', 'zunehdd', 'zunehd'];
const mods = [];
for (const f of files) {
	try {
		// Cache-bust so repeated renders pick up edits.
		const m = await import(`${pathToFileURL(join(artDir, f)).href}?t=${Date.now()}`);
		if (!m.model || !m.render) throw new Error('missing `model` or `render` export');
		if (only && m.model.id !== only) continue;
		mods.push(m);
	} catch (e) {
		problems.push(`${f}: ${e.message}`);
	}
}
mods.sort((a, b) => (ORDER.indexOf(a.model.id) + 99) % 99 - (ORDER.indexOf(b.model.id) + 99) % 99);

const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
let n = 0;
for (const { model, render } of mods) {
	const cards = model.finishes
		.map((fin) => {
			const pair = [true, false]
				.map((lit) => {
					try {
						return render({ finish: fin.id, lit, height: 300, uid: `a${n++}`, now });
					} catch (e) {
						problems.push(`${model.id}/${fin.id}: ${e.message}`);
						return `<div class="err">render failed: ${esc(e.message)}</div>`;
					}
				})
				.join('');
			const src = fin.source ? `<a href="${esc(fin.source)}" target="_blank" rel="noopener">source</a>` : '';
			return `<figure><div class="pair">${pair}</div><figcaption><span class="fin">${esc(fin.name)}</span>${fin.note ? `<span class="note">${esc(fin.note)}</span>` : ''}${src}</figcaption></figure>`;
		})
		.join('');
	sections.push(
		`<section><header><h2>${esc(model.name)}</h2><span class="meta">${model.year} · ${model.mm.join(' × ')} mm · ${model.finishes.length} colorway${model.finishes.length === 1 ? '' : 's'}</span></header><div class="grid">${cards}</div></section>`
	);
}

const font = (file, weight) => {
	const b64 = readFileSync(join(root, 'static/fonts/selawik', file)).toString('base64');
	return `@font-face{font-family:Selawik;font-weight:${weight};src:url(data:font/woff2;base64,${b64}) format('woff2')}`;
};

const html = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Zune Colorways</title>
<style>
${font('selawkl.woff2', 300)}${font('selawksl.woff2', 350)}${font('selawk.woff2', 400)}${font('selawksb.woff2', 600)}
:root{--bg:#f7f7f7;--fg:#333;--muted:#8a8a8a;--line:#e3e3e3;--card:#fff;--accent:#e5007e}
@media (prefers-color-scheme:dark){:root:not([data-theme="light"]){--bg:#111;--fg:#f2f2f2;--muted:#8a8a8a;--line:#262626;--card:#181818;color-scheme:dark}}
:root[data-theme="dark"]{--bg:#111;--fg:#f2f2f2;--muted:#8a8a8a;--line:#262626;--card:#181818;color-scheme:dark}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--fg);font-family:Selawik,'Segoe UI',system-ui,sans-serif;font-weight:350}
main{max-width:1400px;margin:0 auto;padding-block:48px 80px;padding-left:max(16px,env(safe-area-inset-left,0px));padding-right:max(16px,env(safe-area-inset-right,0px))}
h1{font-weight:300;font-size:clamp(2.4rem,6vw,4.2rem);margin:0;text-transform:lowercase;line-height:1}
.lede{color:var(--muted);margin:12px 0 0;max-width:42rem}
.rule{height:3px;width:120px;margin:28px 0 8px;background:linear-gradient(90deg,#e5007e,#f7941d)}
section{margin-top:56px}
section header{display:flex;flex-wrap:wrap;align-items:baseline;gap:6px 16px;border-bottom:1px solid var(--line);padding-bottom:10px;margin-bottom:24px}
h2{font-weight:300;font-size:2.2rem;margin:0;text-transform:lowercase}
.meta{color:var(--muted);font-size:.9rem}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(min(300px,100%),1fr));gap:20px}
figure{margin:0;background:var(--card);border:1px solid var(--line);padding:20px 16px 14px}
.pair{display:flex;justify-content:center;align-items:flex-end;gap:18px;min-height:300px}
.pair svg{max-width:46%;height:auto}
figcaption{display:flex;flex-wrap:wrap;align-items:baseline;gap:4px 12px;margin-top:14px;text-transform:lowercase}
.fin{font-size:1.15rem}
.note{color:var(--muted);font-size:.8rem;text-transform:none}
figcaption a{margin-left:auto;color:var(--muted);font-size:.8rem}
figcaption a:hover{color:var(--accent)}
.err{color:#d33;font-size:.8rem}
.problems{margin-top:40px;color:#d33;font-family:ui-monospace,monospace;font-size:.8rem;white-space:pre-wrap}
</style></head>
<body><main>
<h1>zune colorways</h1>
<p class="lede">every zune model in every colorway it shipped in, drawn for squirt's device page. each pair shows the screen lit and off.</p>
<div class="rule"></div>
${sections.join('\n') || '<p class="lede">no models yet.</p>'}
${problems.length ? `<div class="problems">${esc(problems.join('\n'))}</div>` : ''}
</main></body></html>`;

const page = artifact
	? html
			.replace(/^<!doctype html>\n<html lang="en"><head><meta charset="utf-8"><meta name="viewport"[^>]*>\n/, '')
			.replace('<title>Zune Colorways</title>', '<title>Zune Colorways</title>\n<meta name="claude:viewport-fit-sides" content="cover">')
			.replace('</style></head>\n<body>', '</style>\n')
			.replace('</main></body></html>', '</main>')
	: html;
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, page);
console.log(`wrote ${out} (${mods.length} model${mods.length === 1 ? '' : 's'})`);
if (problems.length) console.log('problems:\n  ' + problems.join('\n  '));

if (png) {
	// One Chrome at a time across processes: a lock directory, stale after 2 min.
	const lock = '/tmp/squirt-art-render.lock';
	for (;;) {
		try {
			mkdirSync(lock);
			break;
		} catch {
			try {
				if (Date.now() - statSync(lock).mtimeMs > 120_000) rmdirSync(lock);
			} catch {}
			await new Promise((r) => setTimeout(r, 500));
		}
	}
	try {
		// puppeteer-core lives with the marketing scripts; resolve it from there.
		const { createRequire } = await import('node:module');
		const req = createRequire(resolve(root, '../marketing/ad2/package.json'));
		const { default: puppeteer } = await import(pathToFileURL(req.resolve('puppeteer-core')).href);
		const browser = await puppeteer.launch({
			executablePath: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
			headless: 'new',
			args: ['--disable-gpu', '--no-first-run']
		});
		const page = await browser.newPage();
		await page.setViewport({ width: 1400, height: 900, deviceScaleFactor: 1 });
		await page.goto(pathToFileURL(out).href, { waitUntil: 'load' });
		await page.evaluate(() => document.fonts.ready);
		await page.screenshot({ path: resolve(png), fullPage: true });
		await browser.close();
		console.log(`wrote ${resolve(png)}`);
	} finally {
		rmdirSync(lock);
	}
}
