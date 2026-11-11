const assert = require('node:assert/strict');
const { test } = require('node:test');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const script = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf8').split('<script>')[1].split('</script>')[0];

function harness(connected = false) {
  const calls = [];
  const elements = new Map();
  const element = id => {
    if (!elements.has(id)) elements.set(id, {
      style: {}, value: 80, textContent: '', innerHTML: '', src: '', volume: 0.8,
      listeners: {}, attributes: {},
      pause() {}, load() {}, removeAttribute() {},
      setAttribute(name, value) { this.attributes[name] = value; },
      addEventListener(name, callback) { this.listeners[name] = callback; },
      play() { this.played = true; return Promise.resolve(); },
    });
    return elements.get(id);
  };
  const context = vm.createContext({
    console, URLSearchParams, encodeURIComponent, navigator: {},
    document: { getElementById: element, querySelectorAll: () => [] },
    setInterval() {}, setTimeout() {},
    fetch: async (url, opts) => {
      calls.push({ url, opts });
      return { ok: true, json: async () => url === '/api/bt/status' ? { connected } : { status: 'playing' } };
    },
  });
  // Disable only the page's initial network load; keep all production functions intact.
  vm.runInContext(script.replace("switchTab('home');", ''), context);
  return { context, calls, element };
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
