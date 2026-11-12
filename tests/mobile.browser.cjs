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

// Valid silent media for fixture-only source loading. play() remains stubbed below.
// A JSON /api/stream response caused asynchronous decoder errors during UI tests.
const silentWave = Buffer.alloc(44 + 16000);
silentWave.write('RIFF', 0); silentWave.writeUInt32LE(silentWave.length - 8, 4);
silentWave.write('WAVEfmt ', 8); silentWave.writeUInt32LE(16, 16);
silentWave.writeUInt16LE(1, 20); silentWave.writeUInt16LE(1, 22);
silentWave.writeUInt32LE(8000, 24); silentWave.writeUInt32LE(16000, 28);
silentWave.writeUInt16LE(2, 32); silentWave.writeUInt16LE(16, 34);
silentWave.write('data', 36); silentWave.writeUInt32LE(16000, 40);

// Exercise the real UI-preview server, not fixture API responses, inside an iframe.
async function checkInteractiveSandbox(context) {
  const { createPreviewServer } = require('../scripts/preview_ui.cjs');
  const server = createPreviewServer();
  await new Promise(resolve => server.listen(0, '0.0.0.0', resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  try {
    for (const mobile of [true, false]) {
      const page = await context.newPage();
      page.setDefaultTimeout(12000);
      try {
        await page.setViewportSize(mobile ? {width:390,height:844} : {width:1440,height:900});
        await page.emulateMedia({ reducedMotion: 'reduce' });
        const errors = [];
        page.on('pageerror', error => errors.push(error.message));
        await page.route('http://arena-preview.test/', route => route.fulfill({ contentType: 'text/html', body: `<style>html,body{margin:0;height:100%}iframe{border:0;width:100%;height:100%}</style><iframe title="Interactive preview" sandbox="allow-scripts allow-same-origin" src="${origin}/"></iframe>` }));
        await page.goto('http://arena-preview.test/');
        const frame = page.frameLocator('iframe');
        const title = frame.locator('#love-intro-title');
        await frame.locator('#love-intro').waitFor({state:'visible'});
        await frame.locator('[data-preview-motion]').last().filter({hasText:'Hearts always on'}).waitFor();
        assert.equal(await title.evaluate(async () => (await document.fonts.load('18px "Delius"', 'AYSHA')).length), 1);
        const missingAudio = await context.request.get(origin + '/api/search?q=test');
        assert.equal(missingAudio.status(), 503, 'Preview must not fake a live audio backend');
        if (mobile) await title.tap(); else await title.click();
        assert.equal(await frame.locator('.heart-particle').count(), 17, 'Actual preview animates without the Rust backend');
        await title.evaluate(() => clearHeartEffects());
        await title.evaluate(el => { const r = el.getBoundingClientRect(); el.dispatchEvent(new MouseEvent('click', {bubbles:true, clientX:r.x+r.width/2, clientY:r.y+r.height/2})); });
        assert.equal(await frame.locator('.heart-particle').count(), 17, 'Coordinate click works even when detail defaults to zero');
        await title.evaluate(() => clearHeartEffects());
        if (mobile) await frame.locator('.love-skip').tap(); else await frame.locator('.love-skip').click();
        await frame.locator('#love-intro').waitFor({state:'hidden'});
        await frame.locator('#heart-effects-page .heart-particle').first().waitFor({state:'visible'});
        await frame.locator('#tab-home .page-heading').evaluate(() => clearHeartEffects());
        await page.emulateMedia({ reducedMotion: 'reduce' });
        await frame.locator('[data-preview-motion]').first().filter({hasText:'Hearts always on'}).waitFor();
        if (mobile) await frame.locator('#tab-home .page-heading').tap(); else await frame.locator('#tab-home .page-heading').click();
        const sample = await frame.locator('#tab-home').evaluate(async () => {
          const first = document.querySelector('.heart-particle');
          const reveal = document.querySelector('#heart-effects-page .heart-reveal');
          const letter = document.querySelector('.love-letter-particle');
          const sample = { count:document.querySelectorAll('.heart-particle').length,
            flight:getComputedStyle(first).animationName,
            flutter:getComputedStyle(first.querySelector('.butterfly-morph svg')).animationName,
            color:getComputedStyle(reveal).color, mask:getComputedStyle(reveal).maskImage,
            transition:getComputedStyle(reveal).transitionDuration, size:getComputedStyle(reveal).maskSize,
            butterflies:document.querySelectorAll('.heart-burst.is-butterfly').length,
            letters:[...document.querySelectorAll('.love-letter-glyph')].map(el => el.textContent),
            letterColor:getComputedStyle(letter).color, fontSize:getComputedStyle(letter).fontSize,
            fontFamily:getComputedStyle(letter).fontFamily, y:first.getBoundingClientRect().y };
          // Measure the same short-lived node inside one browser call. Multiple
          // protocol round trips can outlive its animation on a busy runner.
          await new Promise(resolve => setTimeout(resolve, 400));
          sample.afterY = first.getBoundingClientRect().y;
          sample.connected = first.isConnected;
          return sample;
        });
        assert.equal(sample.count, 17);
        assert.equal(sample.flight, 'heart-flight');
        assert.equal(sample.flutter, 'heart-flutter');
        assert.equal(sample.color, 'rgb(237, 48, 75)');
        assert.ok(sample.mask.includes('data:image/svg+xml'));
        assert.equal(sample.transition, '0.12s');
        assert.equal(sample.size, '112px 104px');
        assert.equal(sample.butterflies, 12);
        assert.deepEqual(sample.letters, ['A','Y','S','H','A']);
        assert.equal(sample.letterColor, 'rgb(255, 255, 255)');
        assert.equal(sample.fontSize, '18px');
        assert.ok(sample.fontFamily.includes('Delius'));
        assert.ok(sample.connected, 'Measured particle remains alive during flight');
        assert.ok(sample.afterY < sample.y - 15, 'Actual preview flies under Reduce Motion');
        assert.deepEqual(errors, []);
        console.log(`PASS actual UI-only preview in iframe (${mobile ? 'touch' : 'mouse'}): hearts work while backend returns503, detail0 works, hearts fly even with Reduce Motion`);
      } finally { await page.close(); }
    }
  } finally { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
}

let activeViewport = 'startup';
(async () => {
  const { default: binary } = await import(pathToFileURL(path.join(modules, '@sparticuz/chromium/build/index.js')).href);
  const browser = await chromium.launch({ executablePath: await binary.executablePath(), args: binary.args, headless: true });
  try {
    const context = await browser.newContext({ viewport: { width: 320, height: 740 }, isMobile: true, hasTouch: true });
    // Stub only media output, including after a real document reload.
    await context.addInitScript(() => {
      window.mediaPlayAttempts = 0;
      HTMLMediaElement.prototype.play = function() { window.mediaPlayAttempts++; return Promise.resolve(); };
      HTMLMediaElement.prototype.pause = function() {};
    });
    await checkInteractiveSandbox(context);
    const page = await context.newPage();
    for (const viewport of [{ width: 320, height: 740 }, { width: 360, height: 800 }, { width: 390, height: 844 }, { width: 412, height: 915 }, { width: 768, height: 1024 }, { width: 844, height: 390 }, { width: 1440, height: 900 }]) {
      activeViewport = `${viewport.width}x${viewport.height}`;
      await page.setViewportSize(viewport);
      page.removeAllListeners('pageerror');
      await page.unroute('**/*');
      const errors = [];
      const playRegistrations = [];
      let serverTrack = null;
      let likedTracks = tracks.slice(1);
      page.on('pageerror', error => errors.push(error.message));
      await page.route('**/*', async route => {
        const url = new URL(route.request().url());
        if (url.hostname === 'img.youtube.com') {
          const color = ['#725269', '#5b666b', '#926354', '#496059'][Math.max(0, Number(url.pathname.match(/sample(\d)/)?.[1] || 1) - 1)];
          return route.fulfill({ contentType: 'image/svg+xml', body: `<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200"><rect width="200" height="200" fill="${color}"/><circle cx="100" cy="100" r="65" fill="#0003"/><circle cx="100" cy="100" r="25" fill="#ffffff20"/></svg>` });
        }
        if (url.pathname === '/') return route.fulfill({ contentType: 'text/html', body: html });
        if (url.pathname === '/icons.svg') return route.fulfill({ contentType: 'image/svg+xml', body: icons });
        if (url.pathname === '/api/stream') return route.fulfill({ contentType: 'audio/wav', body: silentWave });
        let data;
        if (url.pathname === '/api/play') { playRegistrations.push(url.search); serverTrack = tracks[0]; data = { status: 'playing', mode: 'browser' }; }
        else if (url.pathname === '/api/stop') { serverTrack = null; data = { status: 'stopped' }; }
        else if (url.pathname === '/api/search') data = tracks;
        else if (url.pathname === '/api/liked') data = likedTracks;
        else if (url.pathname === '/api/like' || url.pathname === '/api/unlike') {
          const track = route.request().postDataJSON();
          likedTracks = likedTracks.filter(t => t.id !== track.id);
          if (url.pathname === '/api/like') likedTracks.push(track);
          data = { status: 'ok' };
        }
        else if (url.pathname === '/api/next') { serverTrack = tracks[1]; data = { status: 'playing', mode: 'browser', track: tracks[1] }; }
        else if (url.pathname === '/api/queue') data = [];
        else if (url.pathname === '/api/status') data = { current_track: serverTrack, is_playing: !!serverTrack, volume: 0.8, queue_len: 0 };
        else if (url.pathname === '/api/bt/status') data = { connected: false, device: 'JBL Flip Essential', mac: '04:CB:88:92:43:60' };
        else data = { status: 'failed', error: 'Read-only UI test fixture' };
        return route.fulfill({ contentType: 'application/json', body: JSON.stringify(data) });
      });
      if (page.url().startsWith('https://aysha.test/')) await page.evaluate(() => { sessionStorage.clear(); localStorage.clear(); });
      await page.emulateMedia({ reducedMotion: viewport.width === 360 ? 'reduce' : 'no-preference' });
      await page.goto('https://aysha.test/');
      await page.locator('#love-intro').waitFor({ state: 'visible' });
      assert.equal(await page.locator('#love-intro-title').textContent(), 'You are loved…');
      assert.equal(await page.locator('#love-intro-lines p').count(), 2);
      assert.equal(await page.locator('#love-intro-lines').evaluate(el => getComputedStyle(el).opacity), '1', 'Letter is fully readable immediately, not fading through its five-second reading time');
      assert.equal(await page.locator('#love-intro').evaluate(el => getComputedStyle(el).backgroundColor), 'rgb(8, 8, 8)');
      assert.equal(await page.evaluate(() => window.mediaPlayAttempts), 0, 'Love letter must not start audio');
      assert.ok(await page.locator('#love-intro').evaluate(el => el.scrollWidth <= el.clientWidth), 'Love letter has no horizontal overflow');
      const skipBox = await page.locator('.love-skip').boundingBox();
      assert.ok(skipBox.width >= 44 && skipBox.height >= 44 && skipBox.y >= 0 && skipBox.y + skipBox.height <= viewport.height);
      assert.ok(await page.evaluate(async () => (await document.fonts.load('18px "Delius"', 'AYSHA')).length === 1), 'Cute font loads locally inside the HTML');
      // Touch produces one burst (not pointerup + synthetic-click duplicates), even in a native modal.
      await page.locator('#love-intro-title').tap();
      {
        // Sample the short-lived burst in one round trip, without font-loading
        // or many protocol calls consuming its animation lifetime.
        const burst = await page.evaluate(async () => {
          const host = document.getElementById('heart-effects-intro');
          const first = host.querySelector('.is-butterfly');
          const sample = { count:host.querySelectorAll('.heart-burst').length,
            letters:[...host.querySelectorAll('.love-letter-glyph')].map(el => el.textContent),
            hearts:host.querySelectorAll('.love-letter-heart use').length,
            letterColor:getComputedStyle(host.querySelector('.love-letter-particle')).color,
            normal:host.querySelectorAll('.heart-burst:not(.is-butterfly)').length,
            color:getComputedStyle(first).color, pointer:getComputedStyle(host).pointerEvents,
            flight:getComputedStyle(first).animationName,
            flutter:getComputedStyle(first.querySelector('.butterfly-morph svg')).animationName,
            y:first.getBoundingClientRect().y };
          // Touch release must not cancel flights. Keep measurement in the page
          // so protocol latency does not consume the short animation lifetime.
          document.dispatchEvent(new PointerEvent('pointerout', { pointerType: 'touch', relatedTarget: null }));
          await new Promise(resolve => setTimeout(resolve, 400));
          sample.afterCount = host.querySelectorAll('.heart-burst').length;
          sample.afterY = first.getBoundingClientRect().y;
          return sample;
        });
        assert.equal(burst.count, 12);
        assert.deepEqual(burst.letters, ['A','Y','S','H','A']);
        assert.equal(burst.hearts, 5);
        assert.equal(burst.letterColor, 'rgb(255, 255, 255)');
        assert.equal(burst.normal, 0);
        assert.equal(burst.color, 'rgb(237, 48, 75)');
        assert.equal(burst.pointer, 'none');
        assert.equal(burst.flight, 'heart-flight');
        assert.equal(burst.flutter, 'heart-flutter');
        assert.equal(burst.afterCount, 12, 'Touch release preserves all twelve butterflies');
        assert.ok(burst.afterY < burst.y - 15, 'Tap heart visibly lifts and flies upward, not a stationary reveal');

        if (viewport.width === 390 && process.env.UI_SCREENSHOT_DIR) {
          fs.mkdirSync(process.env.UI_SCREENSHOT_DIR, { recursive: true });
          await page.waitForTimeout(180);
          await page.screenshot({ path: path.join(process.env.UI_SCREENSHOT_DIR, 'mobile-love-hearts.png') });
        }
        await page.waitForFunction(() => document.querySelectorAll('.heart-particle').length === 0);
      }
      if (viewport.width === 390) {
        if (process.env.UI_SCREENSHOT_DIR) await page.screenshot({ path: path.join(process.env.UI_SCREENSHOT_DIR, 'mobile-love-letter.png') });
        await page.waitForFunction(() => document.getElementById('love-intro-lines').textContent.includes('A little world'));
        assert.equal(await page.locator('#love-intro-lines p').count(), 2);
        await page.waitForFunction(() => document.getElementById('love-intro-lines').textContent.includes('Every note'));
        await page.locator('#love-intro').waitFor({ state: 'hidden', timeout: 17000 });
        assert.equal(await page.evaluate(() => window.mediaPlayAttempts), 0, 'Automatic reveal must not autoplay');
      } else if (viewport.width === 360) {
        assert.equal(await page.locator('.love-bubble').evaluate(el => getComputedStyle(el).animationName), 'none');
        assert.equal(await page.locator('#love-intro-lines').evaluate(el => getComputedStyle(el).animationName), 'none');
        await page.keyboard.press('Escape');
        await page.locator('#love-intro').waitFor({ state: 'hidden' });
      } else {
        await page.locator('.love-skip').click();
        await page.locator('#love-intro').waitFor({ state: 'hidden' });
        await page.waitForFunction(() => document.querySelectorAll('#heart-effects-page .heart-burst').length === 12);
      }

      await page.locator('#quick-picks .song-card').first().waitFor();
      assert.equal((await page.locator('.footer').textContent()).trim(), 'Made with ❤️ for Aysha — with love, from Afzal');
      assert.equal((await page.locator('#tab-home .page-heading').innerText()).replace(/\s+/g, ' ').trim(), 'Your music. Your kind of moment. For you from my heart');
      assert.equal(await page.locator('.profile-mark').textContent(), 'AYSHA');
      assert.ok(await page.locator('.profile-mark').isVisible(), 'Full profile name visible even at 320px');
      assert.ok(await page.locator('.profile-mark').evaluate(el => el.scrollWidth <= el.clientWidth && parseFloat(getComputedStyle(el).fontSize) <= 9), 'Small profile name fits inside the badge');

      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `Horizontal viewport overflow at ${viewport.width}`);
      const nav = await page.locator('#nav-home').boundingBox();
      assert.ok(nav.height >= 44 && nav.width >= 44, 'Navigation target must be at least 44px');
      if (viewport.width === 390 && process.env.UI_SCREENSHOT_DIR) {
        fs.mkdirSync(process.env.UI_SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: path.join(process.env.UI_SCREENSHOT_DIR, 'mobile-home.png') });
      }
      // The fifth navigation item remains touchable on narrow phones.
      for (const name of ['Home', 'Explore', 'Library', 'Quotes', 'Bluetooth']) {
        const box = await page.getByRole('button', { name, exact: true }).boundingBox();
        assert.ok(box && box.width >= 44 && box.height >= 44, `${name} target at ${viewport.width}`);
        assert.ok(box.x >= 0 && box.x + box.width <= viewport.width, `${name} stays within viewport`);
      }
      await page.getByRole('button', { name: 'Quotes', exact: true }).click();
      assert.ok(await page.locator('#tab-quotes').isVisible());
      assert.equal(await page.locator('#nav-quotes').getAttribute('aria-current'), 'page');
      assert.equal(await page.evaluate(() => localStorage.getItem('aysha.comfort.v1')), null, 'Browsing must not write preference data');
      const firstQuote = await page.locator('#quote-text').textContent();
      await page.locator('#quote-next').click();
      assert.notEqual(await page.locator('#quote-text').textContent(), firstQuote);
      await page.getByRole('button', { name: 'Rest', exact: true }).click();
      assert.equal(await page.locator('#quote-category-label').textContent(), 'Rest');
      await page.locator('#quote-save').click();
      assert.equal(await page.locator('#quote-save').getAttribute('aria-pressed'), 'true');
      await page.locator('[data-quote-category="saved"]').click();
      assert.equal(await page.locator('#quote-saved-count').textContent(), '1');
      if (viewport.width === 390 && process.env.UI_SCREENSHOT_DIR) await page.screenshot({ path: path.join(process.env.UI_SCREENSHOT_DIR, 'mobile-quotes.png') });
      await page.locator('#tab-quotes .quote-settings summary').click();
      assert.ok(!(await page.locator('#quote-personalize').isChecked()));
      await page.locator('#quote-personalize').check();
      await page.locator('#quote-more').click();
      if (viewport.width === 390) {
        await page.reload();
        await page.locator('.love-skip').click();
        await page.locator('#love-intro').waitFor({ state: 'hidden' });
        await page.locator('#tab-quotes.active').waitFor();
        assert.equal(await page.locator('#quote-saved-count').textContent(), '1');
        assert.ok(await page.locator('#quote-personalize').isChecked());
        await page.locator('#tab-quotes .quote-settings summary').click();
      }
      await page.locator('#tab-quotes .quote-reset').click();
      assert.equal(await page.locator('#quote-saved-count').textContent(), '0');
      assert.ok(!(await page.locator('#quote-personalize').isChecked()));
      assert.equal(await page.evaluate(() => localStorage.getItem('aysha.comfort.v1')), null);
      await page.locator('[data-quote-category="saved"]').click();
      assert.ok(await page.locator('#quote-next').isDisabled());
      assert.match(await page.locator('#quote-text').textContent(), /Save a message/);
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'Quotes must not overflow');
      await page.goBack();
      assert.ok(await page.locator('#tab-home').isVisible(), 'Back from Quotes returns Home');
      await page.goForward();
      assert.ok(await page.locator('#tab-quotes').isVisible(), 'Forward restores Quotes');
      await page.goBack();
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
      // Exercise actual hero/expanded-player buttons with media calls stubbed;
      // this verifies UI wiring and requests, not audible playback or YouTube.
      if (viewport.width === 390) {
        await page.locator('#quick-picks .song-card').first().click();
        await page.waitForFunction(() => window.mediaPlayAttempts === 1);
        await page.locator('.btn-stop').click();
        await page.locator('#hero-toggle').click();
        await page.waitForFunction(() => document.getElementById('hero-play-label').textContent === 'Pause');
        assert.equal(playRegistrations.length, 2, 'Hero Play must restart the stopped selection');
        await page.locator('#open-player').click();
        await page.locator('.now-footer button').click();
        await page.locator('#full-play-toggle').click();
        await page.waitForFunction(() => document.getElementById('full-play-toggle').getAttribute('aria-label') === 'Pause');
        assert.equal(playRegistrations.length, 3, 'Expanded Play must also restart after Stop');
        await page.waitForFunction(() => (document.getElementById('audio-player').getAttribute('src') || '').includes('sample1'));
        assert.match(await page.locator('#audio-player').getAttribute('src'), /sample1/);
        // Stop clears the server track, then reload must recover from tab storage.
        await page.locator('.now-footer button').click();
        await page.waitForFunction(() => document.getElementById('full-play-toggle').getAttribute('aria-label') === 'Play');
        await page.reload();
        await page.locator('.love-skip').click();
        await page.locator('#love-intro').waitFor({ state: 'hidden' });
        await page.locator('#quick-picks .song-card').first().waitFor();
        assert.equal(await page.locator('#hero-title').textContent(), tracks[0].title);
        assert.equal(await page.locator('#hero-toggle').getAttribute('aria-label'), 'Play');
        assert.equal(await page.evaluate(() => window.mediaPlayAttempts), 0, 'Reload must not autoplay');
        await page.locator('#open-player').click();
        assert.equal(await page.locator('#full-artist').textContent(), tracks[0].artist);
        await page.locator('#full-play-toggle').click();
        await page.waitForFunction(() => window.mediaPlayAttempts === 1);
        assert.equal(playRegistrations.length, 4, 'Reloaded Play starts the selected track again');
        // Also reload while the server still advertises browser playback.
        await page.reload();
        await page.locator('.love-skip').click();
        await page.locator('#love-intro').waitFor({ state: 'hidden' });
        await page.locator('#quick-picks .song-card').first().waitFor();
        await page.evaluate(() => pollStatus());
        assert.equal(await page.locator('#hero-toggle').getAttribute('aria-label'), 'Play');
        assert.equal(await page.evaluate(() => window.mediaPlayAttempts), 0);
        await page.locator('#hero-toggle').click();
        await page.waitForFunction(() => window.mediaPlayAttempts === 1);
        assert.equal(playRegistrations.length, 5);
        const sourceBeforeQuotes = await page.locator('#audio-player').getAttribute('src');
        await page.getByRole('button', { name: 'Quotes', exact: true }).click();
        await page.locator('#quote-next').click();
        await page.locator('#quote-save').click();
        assert.equal(playRegistrations.length, 5, 'Quotes never registers/restarts music');
        assert.equal(await page.evaluate(() => window.mediaPlayAttempts), 1);
        assert.equal(await page.locator('#audio-player').getAttribute('src'), sourceBeforeQuotes);
        await page.getByRole('button', { name: 'Home', exact: true }).click();


      }
      // Real controls with fixture endpoints; media output is still stubbed.
      await page.getByRole('button', { name: 'Home', exact: true }).click();
      await page.locator('#quick-picks .song-card').first().click();
      await page.waitForFunction(() => (document.getElementById('audio-player').getAttribute('src') || '').includes('sample1'));
      const sourceBeforeFavorite = await page.locator('#audio-player').getAttribute('src');
      const playsBeforeFavorite = await page.evaluate(() => window.mediaPlayAttempts);
      await page.locator('#open-player').click();
      await page.evaluate(() => clearHeartEffects());
      await page.locator('#full-favorite').click();
      await page.waitForFunction(() => document.getElementById('full-favorite').getAttribute('aria-pressed') === 'true');
      assert.ok(await page.locator('#heart-effects-player .heart-particle').count() > 0, 'Favorite control still works through the decorative modal layer');
      assert.equal(await page.locator('#heart-effects-player').getAttribute('aria-hidden'), 'true');
      assert.equal(await page.locator('#full-favorite').getAttribute('aria-label'), 'Remove from favorites');
      const heartBox = await page.locator('#full-favorite').boundingBox();
      assert.ok(heartBox.width >= 44 && heartBox.height >= 44);
      const playBox = await page.locator('#full-play-toggle').boundingBox();
      assert.ok(Math.abs(playBox.x + playBox.width / 2 - viewport.width / 2) < 2, 'Play stays centered with the heart added');
      assert.equal(await page.locator('#audio-player').getAttribute('src'), sourceBeforeFavorite);
      assert.equal(await page.evaluate(() => window.mediaPlayAttempts), playsBeforeFavorite);
      assert.ok(await page.locator('#now-playing').evaluate(el => el.scrollWidth <= el.clientWidth));
      if (viewport.width === 390 && process.env.UI_SCREENSHOT_DIR) await page.screenshot({ path: path.join(process.env.UI_SCREENSHOT_DIR, 'mobile-favorite.png') });
      // End first track: prompt stays about sample1 while autoplay selects sample2.
      await page.evaluate(() => document.getElementById('audio-player').dispatchEvent(new Event('ended')));
      await page.waitForFunction(() => (document.getElementById('audio-player').getAttribute('src') || '').includes('sample2'));
      assert.ok(await page.locator('#full-song-rating').isVisible());
      assert.equal(await page.locator('#full-song-rating-title').textContent(), tracks[0].title);
      assert.equal(await page.locator('#full-title').textContent(), tracks[1].title);
      const rateButton = page.locator('#full-song-rating').getByRole('button', { name: 'Rate 4 out of 5', exact: true });
      const rateBox = await rateButton.boundingBox();
      assert.ok(rateBox.width >= 44 && rateBox.height >= 44);
      assert.ok(rateBox.y >= 0 && rateBox.y + rateBox.height <= viewport.height, 'Rating choices are immediately within the visible player viewport');
      await rateButton.click();
      assert.ok(await page.locator('#full-song-rating').isHidden());
      await page.keyboard.press('Escape');
      await page.locator('#now-playing').waitFor({ state: 'hidden' });
      await page.getByRole('button', { name: 'Library', exact: true }).click();
      await page.locator('#full-history .card-heart[data-track-id="sample1"]').waitFor();
      assert.equal(await page.locator('#song-ratings-count').textContent(), '1');
      assert.match(await page.locator('#song-ratings-list').textContent(), /4\/5/);
      await page.getByRole('button', { name: 'Home', exact: true }).click();
      if (viewport.width === 390) {
        await page.reload();
        await page.locator('.love-skip').click();
        await page.locator('#love-intro').waitFor({ state: 'hidden' });
        await page.locator('#quick-picks .song-card').first().waitFor();
      }
      await page.locator('#quick-picks .song-card').first().click();
      await page.waitForFunction(() => (document.getElementById('audio-player').getAttribute('src') || '').includes('sample1'));
      await page.locator('#open-player').click();
      assert.equal(await page.locator('#full-favorite').getAttribute('aria-pressed'), 'true');
      assert.match(await page.locator('#full-song-score').textContent(), /4\/5/);
      await page.locator('#full-favorite').click();
      await page.waitForFunction(() => document.getElementById('full-favorite').getAttribute('aria-pressed') === 'false');
      await page.keyboard.press('Escape');
      await page.locator('#now-playing').waitFor({ state: 'hidden' });
      await page.evaluate(() => document.getElementById('audio-player').dispatchEvent(new Event('ended')));
      await page.waitForFunction(() => (document.getElementById('audio-player').getAttribute('src') || '').includes('sample2'));
      assert.ok(await page.locator('#song-rating').isHidden(), 'Already-rated song skips prompt, including after reload');
      assert.ok(await page.locator('#full-song-rating').isHidden());
      await page.evaluate(() => document.getElementById('audio-player').dispatchEvent(new Event('ended')));
      await page.locator('#song-rating').waitFor({ state: 'visible' });
      assert.equal(await page.locator('#song-rating-title').textContent(), tracks[1].title);
      assert.ok(await page.locator('#song-rating').evaluate(el => el.scrollWidth <= el.clientWidth));
      await page.locator('#song-rating').getByRole('button', { name: 'Not now', exact: true }).click();
      await page.getByRole('button', { name: 'Library', exact: true }).click();
      await page.waitForFunction(() => document.getElementById('song-ratings-count').textContent === '1');
      await page.locator('#full-history .card-heart[data-track-id="sample1"]').waitFor({ state: 'detached' });
      if (viewport.width === 1440) {
        await page.evaluate(() => clearHeartEffects());
        await page.mouse.move(650, 400);
        const reveal = page.locator('#heart-effects-page .heart-reveal');
        await page.waitForFunction(() => document.querySelector('#heart-effects-page .heart-reveal.is-revealed'));
        const originalRect = await reveal.boundingBox();
        const originalTiles = await reveal.innerHTML();
        const originalX = await reveal.evaluate(el => el.style.getPropertyValue('--reveal-x'));
        assert.equal(await page.locator('.hover-butterfly').count(), 2, 'Hover lifts two tiny butterflies from the revealed tiles');
        assert.equal(await page.locator('.heart-burst').count(), 0, 'Hover does not fire a click burst');
        assert.equal(await reveal.evaluate(el => getComputedStyle(el).maskSize), '112px 104px');
        await page.waitForTimeout(40);
        await page.mouse.move(740, 450);
        await page.waitForTimeout(160);
        assert.notEqual(await reveal.evaluate(el => el.style.getPropertyValue('--reveal-x')), originalX);
        assert.deepEqual(await reveal.boundingBox(), originalRect, 'Background field stays fixed');
        assert.equal(await reveal.innerHTML(), originalTiles);
        assert.ok(await reveal.evaluate(el => parseFloat(getComputedStyle(el).opacity) > .4));
        if (process.env.UI_SCREENSHOT_DIR) await page.screenshot({ path: path.join(process.env.UI_SCREENSHOT_DIR, 'desktop-compact-heart-butterflies.png') });
        await page.waitForFunction(() => getComputedStyle(document.querySelector('#heart-effects-page .heart-reveal')).opacity === '0');
        await page.mouse.click(720, 650);
        assert.equal(await page.locator('#heart-effects-page .heart-burst').count(), 12);
        const butterfly = page.locator('#heart-effects-page .heart-particle').first();
        const startBox = await butterfly.boundingBox();
        await page.waitForTimeout(700);
        const flyingBox = await butterfly.boundingBox();
        assert.ok(flyingBox.y < startBox.y - 20, 'Desktop click launches upward butterfly flight');
        if (process.env.UI_SCREENSHOT_DIR) await page.screenshot({ path: path.join(process.env.UI_SCREENSHOT_DIR, 'desktop-butterflies-white-letters.png') });
        await page.waitForFunction(() => document.querySelectorAll('.heart-particle').length === 0);
        await page.evaluate(() => { for (let i = 0; i < 100; i++) spawnHeartParticle(300, 200, true); });
        assert.equal(await page.locator('.heart-particle').count(), 32, 'Rapid activity stays bounded');
        await page.emulateMedia({ reducedMotion: 'reduce' });
        await page.waitForFunction(() => document.querySelectorAll('.heart-particle').length === 0);
        await page.mouse.click(500, 150);
        assert.equal(await page.locator('.heart-particle').count(), 17, 'User-requested always-on hearts survive Reduce Motion');
        assert.equal(await page.locator('.heart-particle').first().evaluate(el => getComputedStyle(el).animationName), 'heart-flight');
      }
      assert.equal(await page.locator('#audio-player').evaluate(el => el.error), null, 'Fixture stream must be decodable; audio errors have dedicated unit coverage');
      assert.deepEqual(errors, [], 'No uncaught page errors');
      console.log(`PASS Chromium ${viewport.width}x${viewport.height}: no viewport overflow, visible navigation, search, Back/Forward and dialog history`);

    }
  } finally { await browser.close(); }
})().catch(error => {
  console.error(error);
  // API annotations remain available when hosted raw-log downloads cannot be reached.
  const diagnostic = `Viewport ${activeViewport}: ${error.stack || error}`.replace(/%/g, '%25').replace(/\r/g, '%0D').replace(/\n/g, '%0A');
  if (process.env.CI) console.error('::error title=Browser regression::' + diagnostic);
  process.exitCode = 1;
});
