const assert = require('node:assert/strict');
const { test } = require('node:test');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const script = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf8').split('<script>')[1].split('</script>')[0];

function harness(connected = false, storage = new Map()) {
  const calls = [];
  const elements = new Map();
  const element = id => {
    if (!elements.has(id)) elements.set(id, {
      style: {}, value: 80, textContent: '', innerHTML: '', src: '', volume: 0.8,
      listeners: {}, attributes: {},
      currentTime: 0, paused: true, playCount: 0,
      pause() { this.paused = true; },
      load() { this.currentTime = 0; },
      removeAttribute(name) { if (name === 'src') this.src = ''; delete this.attributes[name]; },
      setAttribute(name, value) { this.attributes[name] = value; },
      addEventListener(name, callback) { this.listeners[name] = callback; },
      play() { this.played = true; this.paused = false; this.playCount++; return Promise.resolve(); },
    });
    return elements.get(id);
  };
  const context = vm.createContext({
    console, URLSearchParams, encodeURIComponent, navigator: {},
    document: { getElementById: element, querySelectorAll: () => [] },
    sessionStorage: {
      getItem(key) { return storage.get(key) || null; },
      setItem(key, value) { storage.set(key, value); },
    },
    setInterval() {}, setTimeout() {},
    fetch: async (url, opts) => {
      calls.push({ url, opts });
      return { ok: true, json: async () => url === '/api/bt/status' ? { connected } : { status: 'playing' } };
    },
  });
  // Disable only the page's initial network load; keep all production functions intact.
  vm.runInContext(script.replace("initializeNavigation();", ''), context);
  return { context, calls, element, storage };
}

const track = { id: 'abc', title: 'Song & title', artist: 'Artist', url: 'https://www.youtube.com/watch?v=abc' };

test('audio source is a proxy URL without JSON fetching', async () => {
  const h = harness();
  assert.equal(await h.context.getStreamUrl(track), '/api/stream?url=' + encodeURIComponent(track.url));
  assert.equal(h.calls.length, 0);
});

test('browser playback uses query parameters and browser mode', async () => {
  const h = harness();
  await h.context.playSong(track);
  const play = h.calls.find(call => call.url.startsWith('/api/play?'));
  assert.ok(play);
  const params = new URLSearchParams(play.url.split('?')[1]);
  assert.equal(params.get('mode'), 'browser');
  assert.equal(params.get('title'), track.title);
  assert.equal(play.opts.body, undefined);
  assert.equal(h.calls.some(call => call.url.startsWith('/api/stream')), false);
  assert.equal(h.element('audio-player').src, '/api/stream?url=' + encodeURIComponent(track.url));
  assert.equal(h.element('audio-player').played, true);
});

test('connected Bluetooth selects server playback, not browser audio', async () => {
  const h = harness(true);
  await h.context.playSong(track);
  assert.equal(new URLSearchParams(h.calls[1].url.split('?')[1]).get('mode'), 'bt');
  assert.notEqual(h.element('audio-player').played, true);
});

test('disconnect uses the disconnect endpoint', async () => {
  const h = harness();
  await h.context.disconnectBT();
  assert.equal(h.calls[0].url, '/api/bt/disconnect');
});

test('HTTP failures are not silently converted to empty JSON', async () => {
  const h = harness();
  h.context.fetch = async () => ({ ok: false, status: 400, text: async () => 'invalid request' });
  await assert.rejects(h.context.api('POST', '/api/play', track), /HTTP 400: invalid request/);
});

test('Bluetooth script errors appear in the interface', async () => {
  const h = harness();
  h.context.fetch = async () => ({ ok: true, json: async () => ({ error: 'script missing' }) });
  await h.context.scanBluetooth();
  assert.equal(h.element('bt-device-text').textContent, 'script missing');
});


test('Next consumes returned track without posting play a second time', async () => {
  const h = harness();
  h.context.fetch = async (url, opts) => {
    h.calls.push({ url, opts });
    return { ok: true, json: async () => ({ status: 'playing', mode: 'browser', track }) };
  };
  await h.context.nextTrack();
  assert.deepEqual(h.calls.map(call => call.url), ['/api/next']);
  assert.equal(h.element('audio-player').played, true);
});

test('Previous uses the same single navigation transaction', async () => {
  const h = harness();
  h.context.fetch = async (url, opts) => {
    h.calls.push({ url, opts });
    return { ok: true, json: async () => ({ status: 'playing', mode: 'browser', track }) };
  };
  await h.context.prevTrack();
  assert.deepEqual(h.calls.map(call => call.url), ['/api/prev']);
});

test('missing previous track shows an error without replacing audio', async () => {
  const h = harness();
  h.element('audio-player').src = 'current';
  h.context.fetch = async () => ({ ok: true, json: async () => ({ status: 'failed', error: 'No previous track' }) });
  await h.context.prevTrack();
  assert.equal(h.element('audio-player').src, 'current');
  assert.equal(h.element('time-display').textContent, 'No previous track');
});

test('navigation is single flight while a request is pending', async () => {
  const h = harness();
  let finish;
  h.context.fetch = (url, opts) => {
    h.calls.push({ url, opts });
    return new Promise(resolve => { finish = () => resolve({ ok: true, json: async () => ({ mode: 'browser', track }) }); });
  };
  const first = h.context.nextTrack();
  await h.context.nextTrack();
  assert.equal(h.calls.length, 1);
  finish();
  await first;
});

test('play/pause changes SVG symbol and accessible label together', () => {
  const h = harness();
  h.context.setPlayingState(true);
  assert.match(h.element('play-svg').innerHTML, /#pause/);
  assert.equal(h.element('play-toggle').attributes['aria-label'], 'Pause');
  h.context.setPlayingState(false);
  assert.match(h.element('play-svg').innerHTML, /#play/);
  assert.equal(h.element('play-toggle').attributes['aria-label'], 'Play');
});

test('volume states reference distinct local SVG symbols', () => {
  const h = harness();
  for (const [value, name] of [[0, 'volume-off'], [30, 'volume-low'], [80, 'volume-high']]) {
    h.context.updateVolumeIcon(value);
    assert.match(h.element('volume-svg').innerHTML, new RegExp('#' + name));
  }
});


test('Stop then Play restarts the selected browser track from the soundtrack card', async () => {
  const h = harness();
  await h.context.playSong(track);
  await h.context.stopPlay();
  assert.equal(h.element('audio-player').paused, true);
  assert.equal(h.element('audio-player').src, '');
  await h.context.togglePlay();
  assert.equal(h.element('audio-player').playCount, 2);
  assert.equal(h.element('audio-player').src, '/api/stream?url=' + encodeURIComponent(track.url));
  assert.equal(h.calls.filter(call => call.url.startsWith('/api/play?')).length, 2);
  assert.equal(h.calls.some(call => call.url === '/api/resume'), false);
});

test('repeated Stop remains restartable without losing the selected song', async () => {
  const h = harness();
  await h.context.playSong(track);
  await h.context.stopPlay();
  await h.context.stopPlay();
  await h.context.togglePlay();
  assert.equal(h.element('audio-player').playCount, 2);
  assert.equal(h.element('hero-title').textContent, track.title);
});

test('Pause then Play resumes without creating another track registration', async () => {
  const h = harness();
  await h.context.playSong(track);
  await h.context.togglePlay();
  assert.equal(h.element('audio-player').paused, true);
  await h.context.togglePlay();
  assert.equal(h.element('audio-player').paused, false);
  assert.equal(h.calls.filter(call => call.url.startsWith('/api/play?')).length, 1);
  assert.equal(h.calls.filter(call => call.url === '/api/resume').length, 1);
});

test('Bluetooth Stop then Play starts mpv again rather than resuming a dead process', async () => {
  const h = harness(true);
  await h.context.playSong(track);
  await h.context.stopPlay();
  await h.context.togglePlay();
  assert.equal(h.calls.filter(call => call.url.startsWith('/api/play?')).length, 2);
  assert.equal(h.calls.some(call => call.url === '/api/resume'), false);
  assert.equal(h.element('audio-player').playCount, 0);
});

test('Play with no selected song still asks the listener to choose one', async () => {
  const h = harness();
  await h.context.togglePlay();
  assert.equal(h.element('time-display').textContent, 'Select a song first');
  assert.equal(h.calls.length, 0);
});

test('failed restart keeps Play available instead of pretending audio started', async () => {
  const h = harness();
  await h.context.playSong(track);
  await h.context.stopPlay();
  h.context.fetch = async () => ({ ok: true, json: async () => ({ status: 'failed', error: 'Downloader unavailable' }) });
  await h.context.togglePlay();
  assert.equal(h.element('play-toggle').attributes['aria-label'], 'Play');
  assert.equal(h.element('audio-player').playCount, 1);
  assert.equal(h.element('time-display').textContent, 'Downloader unavailable');
});

test('browser Stop remains locally stopped if its server notification fails', async () => {
  const h = harness();
  await h.context.playSong(track);
  h.context.fetch = async () => { throw new Error('Network unavailable'); };
  await h.context.stopPlay();
  assert.equal(h.element('audio-player').paused, true);
  assert.equal(h.element('audio-player').src, '');
  assert.equal(h.element('play-toggle').attributes['aria-label'], 'Play');
  assert.match(h.element('time-display').textContent, /Stopped on this device; server update failed/);
});


test('Stop then refresh restores the selected song without autoplay and Play restarts it', async () => {
  const original = harness();
  await original.context.playSong(track);
  await original.context.stopPlay();
  const refreshed = harness(false, original.storage);
  assert.equal(refreshed.element('hero-title').textContent, track.title);
  assert.equal(refreshed.element('full-artist').textContent, track.artist);
  assert.equal(refreshed.element('audio-player').playCount, 0);
  assert.equal(refreshed.element('play-toggle').attributes['aria-label'], 'Play');
  await refreshed.context.togglePlay();
  assert.equal(refreshed.element('audio-player').playCount, 1);
  assert.equal(refreshed.calls.filter(call => call.url.startsWith('/api/play?')).length, 1);
  assert.equal(refreshed.calls.some(call => call.url === '/api/resume'), false);
});

test('refresh during browser playback is restartable, not falsely playing from global status', async () => {
  const original = harness();
  await original.context.playSong(track);
  const refreshed = harness(false, original.storage);
  refreshed.context.fetch = async () => ({ ok: true, json: async () => ({ current_track: track, is_playing: true, volume: 0.8 }) });
  await refreshed.context.pollStatus();
  assert.equal(refreshed.element('play-toggle').attributes['aria-label'], 'Play');
  assert.equal(refreshed.element('audio-player').src, '');
  await refreshed.context.togglePlay();
  assert.equal(refreshed.element('audio-player').playCount, 1);
});

test('polling does not replace this tabs selected song with another devices song', async () => {
  const h = harness();
  await h.context.playSong(track);
  await h.context.stopPlay();
  h.context.fetch = async () => ({ ok: true, json: async () => ({ current_track: { ...track, id: 'other', title: 'Other device' }, is_playing: true }) });
  await h.context.pollStatus();
  assert.equal(h.element('hero-title').textContent, track.title);
  assert.equal(h.element('full-title').textContent, track.title);
  assert.equal(h.element('play-toggle').attributes['aria-label'], 'Play');
});

test('corrupt and unavailable tab storage do not prevent playback', async () => {
  for (const storage of [new Map([['aysha.selection.v1', '{broken']]), {
    get() { throw new Error('Storage denied'); }, set() { throw new Error('Storage full'); },
  }]) {
    const h = harness(false, storage);
    await h.context.playSong(track);
    await h.context.stopPlay();
    await h.context.togglePlay();
    assert.equal(h.element('audio-player').playCount, 2);
  }
});


test('restored selection supplies valid artwork to Chromium MediaSession', async () => {
  const original = harness();
  await original.context.playSong(track);
  const refreshed = harness(false, original.storage);
  refreshed.context.navigator.mediaSession = { setActionHandler() {}, setPositionState() {} };
  refreshed.context.MediaMetadata = function(data) {
    assert.equal(typeof data.artwork[0].src, 'string');
    Object.assign(this, data);
  };
  refreshed.context.restoreSelection();
  await refreshed.context.togglePlay();
  assert.equal(refreshed.element('audio-player').playCount, 1);
  assert.equal(refreshed.context.navigator.mediaSession.metadata.title, track.title);
  assert.equal(new URLSearchParams(refreshed.calls.find(call => call.url.startsWith('/api/play?')).url.split('?')[1]).get('thumbnail'), 'https://img.youtube.com/vi/abc/hqdefault.jpg');
});

test('Bluetooth selection survives Stop and refresh and restarts via play, not resume', async () => {
  const original = harness(true);
  await original.context.playSong(track);
  await original.context.stopPlay();
  const refreshed = harness(true, original.storage);
  assert.equal(refreshed.element('full-output').textContent, 'Home Bluetooth speaker');
  await refreshed.context.togglePlay();
  assert.equal(new URLSearchParams(refreshed.calls[0].url.split('?')[1]).get('mode'), 'bt');
  assert.equal(refreshed.calls.some(call => call.url === '/api/resume'), false);
  assert.equal(refreshed.element('audio-player').playCount, 0);
});

test('invalid saved selection schema is ignored without breaking the empty player', () => {
  for (const saved of [null, {}, { mode: 'bt', track: {} }, { mode: 'browser', track: { ...track, url: 'javascript:alert(1)' } }, { mode: 'browser', track: { ...track, title: 'x'.repeat(501) } }]) {
    const h = harness(false, new Map([['aysha.selection.v1', JSON.stringify(saved)]]));
    h.context.syncNowPlaying();
    assert.equal(h.element('full-artist').textContent, 'Choose a song from Home or Explore.');
    assert.equal(h.element('audio-player').playCount, 0);
  }
});
