// Reference-inspired optional theme + deterministic performance contracts.
// All responses are fixtures. No provider/account/media acquisition.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const modules = process.env.BROWSER_TEST_MODULES;
const {chromium} = require(path.join(modules, 'playwright-core'));
const root = path.join(__dirname, '..');
(async () => {
  const {default:binary} = await import(require('node:url').pathToFileURL(path.join(modules, '@sparticuz/chromium/build/index.js')).href);
  const browser = await chromium.launch({executablePath:await binary.executablePath(),args:binary.args,headless:true});
  try {
    const context = await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true});
    const page = await context.newPage(); const errors = []; page.on('pageerror',e=>errors.push(e.message));
    const tracks = ['Midnight memories','A little closer','நிலவே · A moonlit melody','Rain on the window','Somewhere with you','The quiet hours'].map((title,i)=>({id:'abcdefghij'+i,title,artist:'Aysha’s collection'}));
    const counters={status:0,player:0,search:0}; let releaseSearch; const searchGate = new Promise(resolve=>releaseSearch=resolve);
    await page.route('**/*', async route=>{
      const url = new URL(route.request().url());
      if(url.hostname==='img.youtube.com') return route.fulfill({contentType:'image/svg+xml',body:'<svg xmlns="http://www.w3.org/2000/svg" width="600" height="600"><defs><linearGradient id="g" x2="1" y2="1"><stop stop-color="#8c459c"/><stop offset=".5" stop-color="#244c64"/><stop offset="1" stop-color="#2eae93"/></linearGradient></defs><path fill="url(#g)" d="M0 0h600v600H0z"/><g fill="none" stroke="#c7c7e9" stroke-opacity=".4"><circle cx="300" cy="300" r="230"/><circle cx="300" cy="300" r="200"/><circle cx="300" cy="300" r="170"/></g><circle cx="300" cy="300" r="100" fill="#172331"/><circle cx="300" cy="300" r="10" fill="#d3eadd"/></svg>'});
      if(url.pathname==='/') return route.fulfill({contentType:'text/html',body:fs.readFileSync(path.join(root,'src/index.html'),'utf8')});
      if(url.pathname==='/icons.svg') return route.fulfill({contentType:'image/svg+xml',body:fs.readFileSync(path.join(root,'src/icons.svg'),'utf8')});
      let data={status:'ok'};
      if(url.pathname==='/api/status'){counters.status++;data={native_playback:true,audio_engine:'media3'};}
      else if(url.pathname==='/api/player'){counters.player++;data={state:'paused',position_secs:46,duration_secs:220,video_id:tracks[0].id,track:tracks[0]};}
      else if(url.pathname==='/api/search'){counters.search++;await searchGate;data=tracks;}
      else if(url.pathname==='/api/liked')data=tracks.slice(1,4);
      else if(url.pathname==='/api/queue')data=tracks.slice(4);
      else if(url.pathname==='/api/tg/configuration')data={configured:false,saved:false};
      else if(url.pathname==='/api/tg/status')data={configured:false,authorized:false,items:[],quota:{queued_tracks:0,max_per_window:18}};
      else return route.abort();
      return route.fulfill({contentType:'application/json',body:JSON.stringify(data)});
    });
    await page.goto('https://aurora-fixture.test/'); await page.locator('.love-skip').click();
    assert.equal(await page.locator('html').getAttribute('data-theme'),'crimson','existing default unchanged');
    // Hold provider search indefinitely; local favorites and queue still arrive.
    await page.locator('#recent-history .song-card').first().waitFor();
    assert.equal(await page.locator('#quick-picks').getAttribute('aria-busy'),'true');
    await page.waitForFunction(()=>state.queue.length===2);
    releaseSearch(); await page.locator('#quick-picks .song-card').first().waitFor();
    await page.evaluate(()=>switchTab('settings'));
    await page.locator('#theme-aurora').click();
    assert.equal(await page.locator('#theme-aurora').getAttribute('aria-pressed'),'true');
    await page.evaluate(()=>switchTab('home'));
    const output=process.env.UI_SCREENSHOT_DIR;if(output)fs.mkdirSync(output,{recursive:true});
    for(const viewport of [{width:320,height:568},{width:390,height:844},{width:844,height:390},{width:1440,height:900}]){
      await page.setViewportSize(viewport);
      assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'no viewport overflow');
      const hero=await page.locator('#hero-box').boundingBox();
      const picks=await page.locator('#quick-picks').boundingBox();
      const liked=await page.locator('#recent-history').boundingBox();
      assert.ok(hero.y<picks.y && picks.y<liked.y,'featured → horizontal picks → liked list');
      assert.equal(await page.locator('#quick-picks').evaluate(el=>getComputedStyle(el).gridAutoFlow),'column');
      if(output && viewport.width===390)await page.screenshot({path:path.join(output,'aurora-home.png')});
      await page.evaluate(()=>openNowPlaying());
      for(const selector of ['#full-play-toggle','#now-playing [aria-label="Stop playback"]']){
        const box=await page.locator(selector).boundingBox();
        assert.ok(box && box.y>=0 && box.y+box.height<=viewport.height,'controls fit '+selector+' '+JSON.stringify(viewport));
      }
      const play=await page.locator('#full-play-toggle').boundingBox();
      const dialog=await page.locator('#now-playing').boundingBox();
      assert.ok(Math.abs(play.x+play.width/2-dialog.x-dialog.width/2)<8,'Play centered');
      assert.equal(await page.locator('#now-playing').evaluate(el=>el.scrollTop),0);
      if(output && viewport.width===390)await page.screenshot({path:path.join(output,'aurora-player.png')});
      await page.evaluate(()=>closeNowPlaying()); await page.locator('#now-playing').waitFor({state:'hidden'});
    }
    // Native polling is one authoritative endpoint, not status+player each tick.
    const before={...counters};await page.evaluate(async()=>{await pollStatus();await pollStatus();await pollStatus();});
    assert.equal(counters.status,before.status,'no redundant native status discovery');
    assert.ok(counters.player>=before.player+3,'authoritative player still refreshed');
    assert.equal(counters.search,1,'theme/layout changes cause no extra provider searches');
    await page.reload();await page.locator('.love-skip').click();
    assert.equal(await page.locator('html').getAttribute('data-theme'),'aurora','choice survives reload');
    await page.evaluate(()=>setColorTheme('charcoal'));assert.equal(await page.locator('#quick-picks').evaluate(el=>getComputedStyle(el).gridAutoFlow),'row','classic layout restored');
    await page.evaluate(()=>setColorTheme('crimson'));assert.equal(await page.locator('#theme-crimson').getAttribute('aria-pressed'),'true');
    assert.deepEqual(errors,[]);
    console.log('PASS Aurora optional theme: default/restore, featured-card-list order, fitted centered player at320/390/844/1440, parallel favorites/queue, no extra search and no redundant native status polls');
    await context.close();
  } finally {await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
