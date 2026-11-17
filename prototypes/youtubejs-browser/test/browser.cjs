// Desktop Chromium proof of the packaged and rebuilt browser bundles; NOT Android or
// source access. Every browser request is intercepted; no provider is contacted.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const modules = process.env.BROWSER_TEST_MODULES;
if (!modules) throw new Error('Set BROWSER_TEST_MODULES (same test tools as tests/mobile.browser.cjs)');
const { chromium } = require(path.join(modules, 'playwright-core'));
const { pathToFileURL } = require('node:url');
const root = path.resolve(__dirname, '..');
const files = {
  '/fixture.js': fs.readFileSync(path.join(root, 'build/fixture.js')),
  '/bundle.mjs': fs.readFileSync(path.join(root, 'node_modules/youtubei.js/bundle/browser.js')),
  '/fixture-adapter.mjs': fs.readFileSync(path.join(root, 'fixture-adapter.mjs')),
  '/fixtures.mjs': fs.readFileSync(path.join(__dirname, 'fixtures.mjs'))
};
(async () => {
  const { default: chromiumBinary } = await import(pathToFileURL(path.join(modules, '@sparticuz/chromium/build/index.js')).href);
  const browser = await chromium.launch({ executablePath: await chromiumBinary.executablePath(),
    args: chromiumBinary.args.filter(arg => arg !== '--disable-web-security' &&
      arg !== '--allow-running-insecure-content'), headless: true });
  try {
    const context = await browser.newContext();
    const unexpected = [], errors = [], messages = [];
    await context.route('**/*', async route => {
      const url = new URL(route.request().url());
      if (url.origin !== 'https://aysha-extractor.test') {
        unexpected.push('external_request'); return route.abort();
      }
      if (url.pathname === '/rebuilt') return route.fulfill({ contentType: 'text/html', body:
        '<!doctype html><title>Rebuilt offline fixture</title><script src="/fixture.js"></script>' });
      if (url.pathname === '/') return route.fulfill({ contentType: 'text/html', body:
        '<!doctype html><title>Offline extractor evaluation</title><script type="module">' +
        'import * as library from "/bundle.mjs";' +
        'import {evaluateFixture} from "/fixture-adapter.mjs";' +
        'import {ID,format,response} from "/fixtures.mjs";' +
        'window.fixtureApi={library,evaluateFixture,ID,format,response};</script>' });
      if (files[url.pathname]) return route.fulfill({ contentType: 'text/javascript', body: files[url.pathname] });
      unexpected.push('unexpected_local_asset'); return route.abort();
    });
    const page = await context.newPage();
    page.on('pageerror', error => errors.push(error.name));
    page.on('console', message => messages.push(message.type()));
    await page.goto('https://aysha-extractor.test/');
    await page.waitForFunction(() => !!window.fixtureApi, { timeout: 15000 });
    const result = await page.evaluate(async () => {
      const {library,evaluateFixture,ID,format,response} = window.fixtureApi;
      const ok = await evaluateFixture(library, JSON.stringify(response()), ID);
      const negative = [];
      for (const raw of [
        response({playabilityStatus:{status:'LOGIN_REQUIRED',reason:'PRIVATE_DO_NOT_LOG'}}),
        response({videoDetails:{...response().videoDetails,videoId:'other000001'}}),
        response({streamingData:{adaptiveFormats:[format({signatureCipher:'s=PRIVATE'})]}})
      ]) {
        try { await evaluateFixture(library, JSON.stringify(raw), ID); negative.push('unexpected_success'); }
        catch (error) { negative.push(error.message); }
      }
      let interpreterMissing = false;
      try { library.Platform.shim.eval({}, {}); }
      catch { interpreterMissing = true; }
      return { runtime: library.Platform.shim.runtime, itag: ok.descriptor.itag,
        fixture_requests: ok.fixture_requests, media_validated: ok.media_validated,
        playback_tested: ok.playback_tested, negative, interpreterMissing };
    });
    assert.deepEqual(result, { runtime: 'browser', itag: 140, fixture_requests: 1,
      media_validated: false, playback_tested: false,
      negative: ['provider_login_or_challenge', 'video_id_mismatch', 'interpreter_required'],
      interpreterMissing: true });
    assert.deepEqual(unexpected, []);
    assert.deepEqual(errors, []);
    assert.deepEqual(messages, [], 'Do not emit provider metadata/URLs through upstream logger');
    console.log(JSON.stringify({ status: 'pass', test: 'packaged_browser_bundle_offline',
      ...result, external_requests: 0, android_tested: false }));
    const rebuiltPage = await context.newPage();
    const rebuiltOutput = rebuiltPage.waitForEvent('console', {
      predicate: message => message.text().startsWith('AYSHA_FIXTURE_RESULT '), timeout: 15000
    });
    rebuiltPage.on('pageerror', error => errors.push(error.name));
    await rebuiltPage.goto('https://aysha-extractor.test/rebuilt');
    const rebuilt = JSON.parse((await rebuiltOutput).text().slice('AYSHA_FIXTURE_RESULT '.length));
    assert.deepEqual(rebuilt, { status: 'pass', runtime: 'browser', itag: 140,
      fixture_requests: 1, media_validated: false, playback_tested: false,
      negative: ['provider_login_or_challenge', 'video_id_mismatch', 'interpreter_required'] });
    assert.deepEqual(unexpected, []);
    assert.deepEqual(errors, []);
    console.log(JSON.stringify({ test: 'rebuilt_browser_bundle_offline', ...rebuilt,
      external_requests: 0, android_tested: false }));
    await context.close();
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
