// Records the real Squirt UI (browser preview at :5180, backed by `squirt
// serve` with the real library and Zune) while clicking through it.
// Writes ui.webm plus marks.json: the time each beat of the tour starts.
import puppeteer from 'puppeteer-core';
import { writeFileSync } from 'node:fs';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
process.env.PATH = `/opt/homebrew/bin:${process.env.PATH}`; // screencast needs ffmpeg

const browser = await puppeteer.launch({
  executablePath: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  headless: 'new',
  args: ['--hide-scrollbars', '--force-color-profile=srgb', '--force-dark-mode']
});
const page = await browser.newPage();
// 1920x1080 viewport with the page zoomed 1.5x: the app lays out at its
// natural 1280x720 but the screencast (which records CSS pixels) is 1080p.
await page.setViewport({ width: 1920, height: 1080, deviceScaleFactor: 1 });
await page.emulateMediaFeatures([{ name: 'prefers-color-scheme', value: 'dark' }]);
// The slider only reads the volume at page load: load at 0.7 so it looks
// normal, then mute the real player so nothing plays aloud.
const api = (path) => fetch(`http://127.0.0.1:5190/api/player/${path}`, { method: 'POST' });
await api('stop');
await api('volume?v=0.7');
await page.goto('http://localhost:5180/', { waitUntil: 'domcontentloaded' });
await page.waitForSelector('::-p-text(your library)', { timeout: 20000 });
await api('volume?v=0');
await page.evaluate(() => {
  document.documentElement.style.zoom = '1.5';
  // Viewport units ignore CSS zoom, so pin the shell to the zoomed height.
  const st = document.createElement('style');
  st.textContent = '.h-dvh { height: 720px !important; }';
  document.head.appendChild(st);
  // Hide the dev-only "browser preview" note next to the title.
  for (const el of document.querySelectorAll('span')) {
    if (el.textContent?.includes('browser preview')) el.style.display = 'none';
  }
});
await sleep(2500); // device poll + art loaded

const click = async (sel) => { const el = await page.waitForSelector(sel, { timeout: 8000 }); await el.click(); };
const marks = {};
const t0 = Date.now();
const mark = (k) => (marks[k] = (Date.now() - t0) / 1000);

const rec = await page.screencast({ path: 'ui.webm', format: 'webm' });
await sleep(600);

mark('collection');
await sleep(1200);
await click('button ::-p-text(Porter Robinson)');
await sleep(900);
await click('button ::-p-text(SMILE! :D plus+)');
await sleep(1100);
const song = await page.waitForSelector('div ::-p-text(feat. Hatsune Miku)');
await song.click({ count: 2 });
mark('playing');
await sleep(2600);

await click('button[aria-label="open now playing"]');
mark('nowplaying');
await sleep(4200);
await page.keyboard.press('Escape');
await sleep(500);

await click('button[role=tab] ::-p-text(device)');
mark('device');
await sleep(4200);
await click('button[role=tab] ::-p-text(music)');
mark('devicemusic');
await sleep(3200);

await click('button[role=tab] ::-p-text(social)');
mark('social');
await sleep(4000);
mark('end');

await rec.stop();
writeFileSync('marks.json', JSON.stringify(marks, null, 2));
await browser.close();
console.log(marks);
