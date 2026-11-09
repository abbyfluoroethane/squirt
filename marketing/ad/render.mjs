// Renders index.html to frames, then encodes ad.mp4 with audio.wav.
//   node render.mjs [--from 0] [--to 30] [--fps 30] [--stills 2,7.5,14] [--out out]
// Every animation must be a CSS animation (or element.animate) so its time
// can be set exactly: each frame sets currentTime on all animations.
import { spawn, execFileSync } from 'node:child_process';
import { mkdirSync, rmSync, existsSync } from 'node:fs';
import puppeteer from 'puppeteer-core';

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const FPS = Number(arg('fps', 30));
const FROM = Number(arg('from', 0));
const TO = Number(arg('to', 30));
const STILLS = arg('stills', null);
const OUT = arg('out', 'out');
// A fresh port per run, so a server left over from the last run can't interfere.
const PORT = 5300 + Math.floor(Math.random() * 600);

const server = spawn('python3', ['-m', 'http.server', String(PORT), '--bind', '127.0.0.1'], { cwd: new URL('.', import.meta.url).pathname, stdio: 'ignore' });
// Wait until the server answers (up to 10 s).
for (let i = 0; ; i++) {
  try { if ((await fetch(`http://127.0.0.1:${PORT}/index.html`)).ok) break; } catch {}
  if (i > 100) throw new Error('static server did not start');
  await new Promise((r) => setTimeout(r, 100));
}
const browser = await puppeteer.launch({
  executablePath: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  headless: 'new',
  args: ['--hide-scrollbars', '--force-color-profile=srgb']
});
try {
  const page = await browser.newPage();
  await page.setViewport({ width: 1920, height: 1080, deviceScaleFactor: 1 });
  await page.goto(`http://127.0.0.1:${PORT}/index.html`, { waitUntil: 'networkidle0' });
  await page.waitForFunction('window.__ready === true', { timeout: 60000 });
  await page.evaluate(() => document.getAnimations().forEach((a) => a.pause()));
  const stage = await page.$('#stage');
  const seek = (t) => page.evaluate((ms) => { for (const a of document.getAnimations()) a.currentTime = ms; }, t * 1000);

  if (STILLS) {
    mkdirSync(OUT, { recursive: true });
    for (const t of STILLS.split(',').map(Number)) {
      await seek(t);
      await stage.screenshot({ path: `${OUT}/still-${t.toFixed(2)}.png` });
    }
  } else {
    const dir = `${OUT}/frames`;
    rmSync(dir, { recursive: true, force: true });
    mkdirSync(dir, { recursive: true });
    const n = Math.round((TO - FROM) * FPS);
    for (let i = 0; i < n; i++) {
      await seek(FROM + i / FPS);
      await stage.screenshot({ path: `${dir}/${String(i).padStart(5, '0')}.png` });
      if (i % 60 === 0) process.stdout.write(`\r${i}/${n}`);
    }
    process.stdout.write(`\r${n}/${n}\n`);
    const audio = existsSync('audio.wav') ? ['-i', 'audio.wav'] : [];
    execFileSync('/opt/homebrew/bin/ffmpeg', [
      '-v', 'error', '-y', '-framerate', String(FPS), '-i', `${dir}/%05d.png`, ...audio,
      '-c:v', 'libx264', '-preset', 'slow', '-crf', '16', '-pix_fmt', 'yuv420p',
      ...(audio.length ? ['-c:a', 'aac', '-b:a', '256k', '-shortest'] : []),
      '-movflags', '+faststart', `${OUT}/ad.mp4`
    ], { stdio: 'inherit' });
    console.log(`wrote ${OUT}/ad.mp4`);
  }
} finally {
  await browser.close();
  server.kill();
}
