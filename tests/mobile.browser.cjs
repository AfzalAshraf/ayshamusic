// Real Chromium layout/navigation checks with explicit sample API fixtures, not a live playback test.
// Test-only dependencies are installed outside the repository; see docs/MOBILE_UI.md.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const modules = process.env.BROWSER_TEST_MODULES;
if (!modules) throw new Error('Set BROWSER_TEST_MODULES to a directory containing playwright-core and @sparticuz/chromium');
const { chromium } = require(path.join(modules, 'playwright-core'));
const root = path.resolve(__dirname, '..');
const html = fs.readFileSync(path.join(root, 'src/index.html'), 'utf8');
const icons = fs.readFileSync(path.join(root, 'src/icons.svg'), 'utf8');
const tracks = [
  { id: 'sample1', title: 'Somewhere only we know', artist: 'Evening collection' },
  { id: 'sample2', title: 'A little closer', artist: 'Acoustic sessions' },
  { id: 'sample3', title: 'The sound of a slow Sunday', artist: 'Quiet mornings' },
  { id: 'sample4', title: 'After the rain', artist: 'Late night favorites' },
].map(t => ({ ...t, url: 'https://www.youtube.com/watch?v=' + t.id, thumbnail: 'https://img.youtube.com/vi/' + t.id + '/hqdefault.jpg' }));

(async () => {
  const { default: binary } = await import(pathToFileURL(path.join(modules, '@sparticuz/chromium/build/index.js')).href);
  const browser = await chromium.launch({ executablePath: await binary.executablePath(), args: binary.args, headless: true });
  try {
    const context = await browser.newContext({ viewport: { width: 320, height: 740 }, isMobile: true, hasTouch: true });
    const page = await context.newPage();
    for (const viewport of [{ width: 320, height: 740 }, { width: 360, height: 800 }, { width: 390, height: 844 }, { width: 412, height: 915 }, { width: 768, height: 1024 }, { width: 844, height: 390 }, { width: 1440, height: 900 }]) {
      await page.setViewportSize(viewport);
      page.removeAllListeners('pageerror');
      await page.unroute('**/*');
      const errors = [];
      page.on('pageerror', error => errors.push(error.message));
      await page.route('**/*', async route => {
        const url = new URL(route.request().url());
        if (url.hostname === 'img.youtube.com') {
          const color = ['#725269', '#5b666b', '#926354', '#496059'][Math.max(0, Number(url.pathname.match(/sample(\d)/)?.[1] || 1) - 1)];
          return route.fulfill({ contentType: 'image/svg+xml', body: `<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200"><rect width="200" height="200" fill="${color}"/><circle cx="100" cy="100" r="65" fill="#0003"/><circle cx="100" cy="100" r="25" fill="#ffffff20"/></svg>` });
        }
        if (url.pathname === '/') return route.fulfill({ contentType: 'text/html', body: html });
        if (url.pathname === '/icons.svg') return route.fulfill({ contentType: 'image/svg+xml', body: icons });
        let data;
        if (url.pathname === '/api/search' || url.pathname === '/api/liked') data = tracks;
        else if (url.pathname === '/api/queue') data = [];
        else if (url.pathname === '/api/status') data = { current_track: null, is_playing: false, volume: 0.8, queue_len: 0 };
        else if (url.pathname === '/api/bt/status') data = { connected: false, device: 'JBL Flip Essential', mac: '04:CB:88:92:43:60' };
        else data = { status: 'failed', error: 'Read-only UI test fixture' };
        return route.fulfill({ contentType: 'application/json', body: JSON.stringify(data) });
      });
      await page.goto('https://aysha.test/');
      await page.locator('#quick-picks .song-card').first().waitFor();
      assert.equal((await page.locator('.footer').textContent()).trim(), 'Made with ❤️ for Aysha — with love, from Afzal');
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `Horizontal viewport overflow at ${viewport.width}`);
      const nav = await page.locator('#nav-home').boundingBox();
      assert.ok(nav.height >= 44 && nav.width >= 44, 'Navigation target must be at least 44px');
      if (viewport.width === 390 && process.env.UI_SCREENSHOT_DIR) {
        fs.mkdirSync(process.env.UI_SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: path.join(process.env.UI_SCREENSHOT_DIR, 'mobile-home.png') });
      }
      await page.getByRole('button', { name: 'Explore', exact: true }).click();
      await page.locator('#explore-cards .song-card').first().waitFor();
      assert.equal(await page.locator('#explore-title').textContent(), 'Jazz');
      await page.getByRole('button', { name: 'Malayalam', exact: true }).click();
      await page.locator('#explore-cards .song-card').first().waitFor();
      assert.equal(await page.locator('#explore-title').textContent(), 'Malayalam');
      await page.goBack();
      assert.ok(await page.locator('#tab-home').isVisible(), 'Browser Back must return to Home');
      await page.locator('#search-input').fill('quiet music');
      await page.locator('#search-input').press('Enter');
      await page.locator('#explore-cards .song-card').first().waitFor();
      assert.equal(await page.locator('#explore-title').textContent(), 'Results for “quiet music”');
      await page.getByRole('button', { name: 'Go back', exact: true }).click();
      await page.waitForFunction(() => document.querySelector('#tab-home').classList.contains('active'));
      await page.locator('#open-player').click();
      assert.ok(await page.locator('#now-playing').isVisible(), 'Now playing opens');
      assert.equal((await page.locator('.now-footer > span').textContent()).trim(), 'Made with ❤️ for Aysha — with love, from Afzal');
      assert.ok(await page.locator('#now-playing').evaluate(el => el.scrollWidth <= el.clientWidth), 'Dedication must wrap without modal overflow');
      if (viewport.width === 390 && process.env.UI_SCREENSHOT_DIR) await page.screenshot({ path: path.join(process.env.UI_SCREENSHOT_DIR, 'mobile-player.png') });
      await page.goBack();
      assert.ok(await page.locator('#now-playing').isHidden(), 'Browser Back closes player before leaving page');
      await page.goForward();
      assert.ok(await page.locator('#now-playing').isVisible(), 'Forward restores the player');
      await page.keyboard.press('Escape');
      await page.locator('#now-playing').waitFor({ state: 'hidden' });
      assert.deepEqual(errors, [], 'No uncaught page errors');
      console.log(`PASS Chromium ${viewport.width}x${viewport.height}: no viewport overflow, visible navigation, search, Back/Forward and dialog history`);

    }
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
