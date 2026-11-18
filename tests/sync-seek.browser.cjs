// UI fixture checks only. No real Telegram account, uploads or provider requests.
const assert=require('node:assert/strict');
const fs=require('node:fs');const path=require('node:path');
const modules=process.env.BROWSER_TEST_MODULES;
const {chromium}=require(path.join(modules,'playwright-core'));
const html=fs.readFileSync(path.join(__dirname,'../src/index.html'),'utf8');
const icons=fs.readFileSync(path.join(__dirname,'../src/icons.svg'),'utf8');
(async()=>{
  const {default:binary}=await import(require('node:url').pathToFileURL(path.join(modules,'@sparticuz/chromium/build/index.js')).href);
  const browser=await chromium.launch({executablePath:await binary.executablePath(),args:binary.args,headless:true});
  try {
    const context=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true});
    const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
    let cancelled=false;const seeks=[];
    const track={id:'abcdefghijk',title:'Fixture romantic melody',artist:'Fixture artist',thumbnail:'https://img.youtube.com/vi/abcdefghijk/default.jpg'};
    await page.route('**/*',async route=>{
      const url=new URL(route.request().url());
      if(url.hostname==='img.youtube.com')return route.fulfill({contentType:'image/svg+xml',body:'<svg xmlns="http://www.w3.org/2000/svg" width="80" height="80"><rect width="80" height="80" fill="#753346"/></svg>'});
      if(url.pathname==='/')return route.fulfill({contentType:'text/html',body:html});
      if(url.pathname==='/icons.svg')return route.fulfill({contentType:'image/svg+xml',body:icons});
      let data={status:'ok'};
      if(url.pathname==='/api/status')data={native_playback:true,audio_engine:'media3'};
      else if(url.pathname==='/api/player')data={state:'paused',position_secs:20,duration_secs:200,video_id:track.id,track};
      else if(url.pathname==='/api/tg/configuration')data={configured:true,saved:true,api_id:12345};
      else if(url.pathname==='/api/tg/status')data={configured:true,authorized:true,saved_tracks:3,failed_tracks:0,wait_seconds:0,quota:{queued_tracks:cancelled?1:2,max_per_window:18},items:[...(!cancelled?[{id:1,video_id:'abcdefghijk',title:'Fixture uploading song',artist:'Artist one',status:'uploading'}]:[]),{id:2,video_id:'nexttrack01',title:'Fixture waiting song',artist:'Artist two',status:'pending'}],active:cancelled?null:{id:1,stage:'uploading',bytes:40,total:100}};
      else if(url.pathname==='/api/tg/cancel'){assert.equal(route.request().postDataJSON().id,1);cancelled=true;data={status:'ok',cancelled:true};}
      else if(url.pathname==='/api/seek'){seeks.push(route.request().postDataJSON());}
      else if(['/api/liked','/api/queue','/api/search'].includes(url.pathname))data=[];
      else return route.abort();
      return route.fulfill({contentType:'application/json',body:JSON.stringify(data)});
    });
    await page.goto('https://sync-fixture.test/#settings');await page.locator('.love-skip').click();
    await page.waitForFunction(()=>document.getElementById('tg-saved').textContent==='3');
    await page.waitForFunction(()=>document.getElementById('full-seek').disabled===false);
    assert.match(await page.locator('#telegram-settings-status').textContent(),/Connected · Saved Messages/);
    assert.equal(await page.locator('#tg-syncing').textContent(),'2');
    assert.match(await page.locator('#tg-sync-list').textContent(),/Uploading · 40% read/);
    assert.match(await page.locator('#tg-sync-list').textContent(),/Waiting/);
    assert.equal(await page.locator('#hero-selftest').count(),0);
    const output=process.env.UI_SCREENSHOT_DIR;
    if(output)fs.mkdirSync(output,{recursive:true});
    for(const theme of ['crimson','charcoal']){
      await page.evaluate(t=>setColorTheme(t),theme);
      for(const width of [320,390,844]){
        await page.setViewportSize({width,height:width===844?390:844});
        assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
        if(width===390 && output)await page.screenshot({path:path.join(output,'telegram-'+theme+'.png')});
      }
    }
    await page.setViewportSize({width:390,height:844});
    await page.locator('[aria-label="Cancel saving Fixture uploading song"]').click();
    await page.waitForFunction(()=>document.getElementById('tg-syncing').textContent==='1');
    assert.match(await page.locator('#tg-sync-feedback').textContent(),/Save cancelled/);
    assert.equal(await page.locator('#tg-sync-list .sync-row').count(),1);
    // Drive the native range's actual input/change events; fixture checks request
    // identity/position, not ExoPlayer decoding (separate instrumentation test).
    await page.locator('#open-player').click();
    for (const viewport of [{width:320,height:568},{width:390,height:640},{width:390,height:844},{width:844,height:390}]) {
      await page.setViewportSize(viewport);
      const stop=await page.locator('#now-playing [aria-label="Stop playback"]').boundingBox();
      const play=await page.locator('#full-play-toggle').boundingBox();
      assert.ok(stop && stop.y>=0 && stop.y+stop.height<=viewport.height,'Stop visible without scrolling '+JSON.stringify(viewport));
      assert.ok(play && play.y>=0 && play.y+play.height<=viewport.height,'Play visible without scrolling');
      assert.equal(await page.locator('#now-playing').evaluate(el=>el.scrollTop),0);
      if(output && viewport.width===390 && viewport.height===844) await page.screenshot({path:path.join(output,'fitted-player.png')});
    }
    await page.setViewportSize({width:390,height:844});
    await page.locator('#full-seek').evaluate(input=>{input.value='750';input.dispatchEvent(new Event('input',{bubbles:true}));input.dispatchEvent(new Event('change',{bubbles:true}));});
    await page.waitForFunction(()=>!seekBusy);
    assert.deepEqual(seeks.at(-1),{position:150,video_id:track.id});
    await page.locator('#now-playing [aria-label="Close now playing"]').click();
    await page.evaluate(()=>receiveSharedVideo('abcdefghijk'));
    assert.equal(await page.locator('#shared-video-dialog').isVisible(),true);
    await page.getByRole('button',{name:'Cancel',exact:true}).click();
    assert.equal(await page.locator('#shared-video-dialog').isVisible(),false);
    await page.reload();await page.locator('.love-skip').click();
    assert.equal(await page.locator('html').getAttribute('data-theme'),'charcoal');
    assert.deepEqual(errors,[]);
    console.log('PASS Telegram dashboard fixtures: live counters/stages, cancellation, crimson/charcoal at320/390/844, persisted theme, native seek request, no Test button/JS errors');
    await context.close();
  }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
