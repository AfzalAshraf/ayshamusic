// Real Chromium checks of the generated static HTML, not live audio acceptance.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const {pathToFileURL} = require('node:url');
const {build} = require('../scripts/build_vercel_preview.cjs');
const modules = process.env.BROWSER_TEST_MODULES;
if (!modules) throw new Error('Set BROWSER_TEST_MODULES to the test browser dependencies');
const {chromium} = require(path.join(modules,'playwright-core'));
(async () => {
  const {default:binary} = await import(pathToFileURL(path.join(modules,'@sparticuz/chromium/build/index.js')).href);
  const {html} = build();
  const icons = fs.readFileSync(path.join(__dirname,'../src/icons.svg'));
  const browser = await chromium.launch({executablePath:await binary.executablePath(),args:binary.args,headless:true});
  try {
    const context = await browser.newContext({isMobile:true,hasTouch:true,reducedMotion:'reduce'});
    for (const width of [390,1440]) {
      const page = await context.newPage();
      await page.setViewportSize({width,height:900});
      const errors = [];
      let apiRequests = 0;
      page.on('pageerror', error=>errors.push(error.message));
      await page.route('**/*', route=> {
        const url = new URL(route.request().url());
        if (url.pathname.startsWith('/api/')) { apiRequests++; return route.fulfill({status:503,contentType:'application/json',body:'{"status":"failed","error":"Audio unavailable"}'}); }
        if (url.hostname==='static-preview.test' && url.pathname==='/') return route.fulfill({contentType:'text/html',body:html});
        if (url.pathname==='/icons.svg') return route.fulfill({contentType:'image/svg+xml',body:icons});
        return route.fulfill({status:404,body:''});
      });
      await page.goto('http://static-preview.test/');
      await page.locator('#love-intro').waitFor({state:'visible'});
      assert.match(await page.locator('#love-intro').innerText(), /Frontend preview · Audio unavailable/);
      await page.evaluate(()=>document.fonts.load('18px "Delius"','AYSHA'));
      if (width===390) await page.locator('#love-intro-title').tap(); else await page.locator('#love-intro-title').click();
      assert.equal(await page.locator('.heart-particle').count(),17);
      await page.locator('.love-skip').click();
      assert.equal(await page.locator('.vercel-preview-note').isVisible(),true);
      await page.locator('#nav-quotes').click();
      assert.equal(await page.locator('#tab-quotes').isVisible(),true);
      await page.evaluate(async()=> {
        try { await api('GET','/api/search?q=test'); throw new Error('Unexpected API success'); }
        catch (error) { if (!error.message.includes('Frontend preview only')) throw error; }
      });
      await page.waitForTimeout(600);
      assert.equal(apiRequests,0,'Static preview must not poll or contact a music backend');
      assert.deepEqual(errors,[]);
      assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'No viewport overflow');
      console.log(`PASS generated Vercel preview ${width}px: honest banner, intro/effects/quotes, no API requests or JS errors`);
      await page.close();
    }
  } finally { await browser.close(); }
})().catch(error=> { console.error(error); process.exitCode=1; });
