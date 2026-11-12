const assert = require('node:assert/strict');
const { test } = require('node:test');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const script = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf8').split('<script>')[1].split('</script>')[0];

function harness(connected = false, storage = new Map(), quoteStorage = new Map()) {
  const calls = [];
  const elements = new Map();
  const element = id => {
    if (!elements.has(id)) elements.set(id, {
      style: {}, value: 80, textContent: '', innerHTML: '', src: '', volume: 0.8,
      listeners: {}, attributes: {},
      classList: {
        values: new Set(),
        contains(name) { return this.values.has(name); },
        add(name) { this.values.add(name); },
        remove(name) { this.values.delete(name); },
        toggle(name, force) { if (force === undefined) force = !this.values.has(name); if (force) this.values.add(name); else this.values.delete(name); return force; },
      },
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
    localStorage: {
      getItem(key) { return quoteStorage.get(key) || null; },
      setItem(key, value) { quoteStorage.set(key, value); },
      removeItem(key) { quoteStorage.delete(key); },
    },
    setInterval() {}, setTimeout() {},
    fetch: async (url, opts) => {
      calls.push({ url, opts });
      return { ok: true, json: async () => url === '/api/bt/status' ? { connected } : { status: 'playing' } };
    },
  });
  // Disable only the page's initial network load; keep all production functions intact.
  vm.runInContext(script.replace("initializeNavigation();", ''), context);
  return { context, calls, element, storage, quoteStorage };
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


test('Quotes browsing is local, untailored by default, and writes no viewing data', () => {
  const h = harness();
  h.context.initializeQuotes();
  assert.equal(h.element('quote-personalize').checked, false);
  assert.equal(h.element('quote-more').disabled, true);
  assert.match(h.element('quote-text').textContent, /earn tenderness/);
  const first = h.element('quote-text').textContent;
  h.context.nextQuote(() => 0);
  assert.notEqual(h.element('quote-text').textContent, first);
  h.context.filterQuotes('rest');
  assert.equal(h.element('quote-category-label').textContent, 'Rest');
  assert.equal(h.quoteStorage.size, 0);
  assert.equal(h.calls.length, 0);
});

test('saved quotes survive reload, can be unsaved, and have an honest empty state', () => {
  const h = harness();
  h.context.initializeQuotes();
  h.context.toggleQuoteSave();
  assert.equal(h.element('quote-save').attributes['aria-pressed'], 'true');
  const refreshed = harness(false, new Map(), h.quoteStorage);
  refreshed.context.initializeQuotes();
  assert.equal(refreshed.element('quote-saved-count').textContent, 1);
  refreshed.context.filterQuotes('saved');
  assert.equal(refreshed.element('quote-save-label').textContent, 'Saved');
  refreshed.context.toggleQuoteSave();
  assert.equal(refreshed.element('quote-saved-count').textContent, 0);
  assert.equal(refreshed.element('quote-next').disabled, true);
  assert.match(refreshed.element('quote-text').textContent, /Save a message/);
});

test('explicit opt-in alone enables preference weighting and turning it off stops it', () => {
  const h = harness();
  h.context.initializeQuotes();
  h.context.toggleQuoteSave();
  const weight = () => vm.runInContext('quoteWeight(comfortQuotes[0])', h.context);
  assert.equal(weight(), 1);
  h.context.moreQuotesLikeThis();
  assert.equal(weight(), 1);
  h.context.setQuotePersonalization(true);
  assert.equal(weight(), 3);
  h.context.moreQuotesLikeThis();
  assert.equal(weight(), 4);
  h.context.setQuotePersonalization(false);
  assert.equal(weight(), 1);
  assert.equal(h.element('quote-more').disabled, true);
  const saved = JSON.parse(h.quoteStorage.get('aysha.comfort.v1'));
  assert.deepEqual(Object.keys(saved).sort(), ['more', 'personalize', 'saved', 'version']);
});

test('weighted selection responds to explicit choices while retaining variety', () => {
  const h = harness();
  h.context.initializeQuotes();
  h.context.toggleQuoteSave();
  const countLove = enabled => {
    h.context.setQuotePersonalization(enabled);
    let count = 0;
    for (let n = 0; n < 200; n++) {
      vm.runInContext("quoteState.currentId = 'rest-1'; quoteState.category = 'all'", h.context);
      h.context.nextQuote(() => (n + 0.5) / 200);
      if (h.element('quote-category-label').textContent === 'Love') count++;
    }
    return count;
  };
  const baseline = countLove(false);
  const tailored = countLove(true);
  assert.ok(tailored > baseline, `${tailored} must exceed ${baseline}`);
  assert.ok(tailored < 200, 'Other categories remain in the mix');
});

test('quote reset deletes the dedicated record without touching music selection', async () => {
  const h = harness();
  await h.context.playSong(track);
  const music = h.storage.get('aysha.selection.v1');
  const playCalls = h.calls.length;
  h.context.initializeQuotes();
  h.context.toggleQuoteSave();
  h.context.setQuotePersonalization(true);
  h.context.resetQuotePreferences();
  assert.equal(h.quoteStorage.has('aysha.comfort.v1'), false);
  assert.equal(h.element('quote-personalize').checked, false);
  assert.equal(h.element('quote-saved-count').textContent, 0);
  assert.equal(h.storage.get('aysha.selection.v1'), music);
  assert.equal(h.calls.length, playCalls);
  assert.equal(h.element('audio-player').paused, false);
});

test('malformed or unavailable quote storage never breaks navigation or in-memory choices', () => {
  for (const quoteStorage of [new Map([['aysha.comfort.v1', '{bad']]), {
    get() { throw Error('Denied'); }, set() { throw Error('Full'); }, delete() { throw Error('Denied'); },
  }]) {
    const h = harness(false, new Map(), quoteStorage);
    h.context.initializeQuotes();
    h.context.filterQuotes('courage');
    h.context.toggleQuoteSave();
    h.context.setQuotePersonalization(true);
    h.context.moreQuotesLikeThis();
    assert.equal(h.element('quote-category-label').textContent, 'Courage');
    h.context.resetQuotePreferences();
    assert.equal(h.element('quote-saved-count').textContent, 0);
    assert.equal(h.element('quote-personalize').checked, false);
  }
});

test('quote storage filters unknown IDs and unbounded preference values', () => {
  const data = { version: 1, saved: ['love-1', 'love-1', '<script>'], personalize: 'true', more: { love: 999, rest: 3, unknown: 4 } };
  const h = harness(false, new Map(), new Map([['aysha.comfort.v1', JSON.stringify(data)]]));
  h.context.initializeQuotes();
  assert.equal(h.element('quote-saved-count').textContent, 1);
  assert.equal(h.element('quote-personalize').checked, false);
  assert.equal(vm.runInContext('quoteState.more.love || 0', h.context), 0);
  assert.equal(vm.runInContext('quoteState.more.rest', h.context), 3);
});

test('full-player favorite is disabled without a selection and makes no request', async () => {
  const h = harness();
  h.context.syncNowPlaying();
  assert.equal(h.element('full-favorite').disabled, true);
  await h.context.toggleCurrentFavorite();
  assert.equal(h.calls.length, 0);
});

test('full-player heart uses existing favorites endpoints, updates state and preserves audio', async () => {
  const h = harness();
  h.context.renderGrid = () => {};
  await h.context.playSong(track);
  h.context.fetch = async (url, opts) => {
    h.calls.push({ url, opts });
    return { ok: true, json: async () => url === '/api/liked' ? [] : { status: 'ok' } };
  };
  const source = h.element('audio-player').src;
  const playCount = h.element('audio-player').playCount;
  await h.context.toggleCurrentFavorite();
  const like = h.calls.find(c => c.url === '/api/like');
  assert.equal(JSON.parse(like.opts.body).id, track.id);
  assert.equal(typeof JSON.parse(like.opts.body).thumbnail, 'string');
  assert.equal(h.element('full-favorite').attributes['aria-pressed'], 'true');
  assert.equal(h.element('full-favorite').attributes['aria-label'], 'Remove from favorites');
  assert.ok(h.element('full-favorite').classList.contains('liked'));
  await h.context.toggleCurrentFavorite();
  assert.ok(h.calls.some(c => c.url === '/api/unlike'));
  assert.equal(h.element('full-favorite').attributes['aria-pressed'], 'false');
  assert.equal(h.element('audio-player').src, source);
  assert.equal(h.element('audio-player').playCount, playCount);
});

test('favorite failures stay unselected and report an error instead of false success', async () => {
  const h = harness();
  h.context.renderGrid = () => {};
  await h.context.playSong(track);
  h.context.fetch = async url => ({ ok: true, json: async () => url === '/api/liked' ? [] : { status: 'failed', error: 'Offline' } });
  await h.context.toggleCurrentFavorite();
  assert.equal(h.element('full-favorite').attributes['aria-pressed'], 'false');
  assert.equal(h.element('full-favorite').disabled, false);
  assert.match(h.element('full-action-status').textContent, /Offline/);
});

test('favorite requests are single flight and pending changes do not like the next track', async () => {
  const h = harness();
  h.context.renderGrid = () => {};
  await h.context.playSong(track);
  h.context.fetch = async () => ({ ok: true, json: async () => [] });
  await h.context.refreshFavorites();
  let finish;
  let writes = 0;
  h.context.fetch = () => { writes++; return new Promise(resolve => { finish = () => resolve({ ok: true, json: async () => ({ status: 'liked' }) }); }); };
  const first = h.context.toggleCurrentFavorite();
  assert.equal(h.element('full-favorite').disabled, true);
  await h.context.toggleCurrentFavorite();
  assert.equal(writes, 1);
  vm.runInContext("state.current_track = { id: 'next', title: 'Next track' };", h.context);
  h.context.syncNowPlaying();
  finish();
  await first;
  assert.equal(h.element('full-favorite').attributes['aria-pressed'], 'false');
  assert.equal(vm.runInContext("state.liked[0].id", h.context), track.id);
});

test('favorites loaded from server update the full-player heart', async () => {
  const h = harness();
  await h.context.playSong(track);
  h.context.fetch = async () => ({ ok: true, json: async () => [track] });
  await h.context.refreshFavorites();
  assert.equal(h.element('full-favorite').attributes['aria-pressed'], 'true');
});

test('natural end asks about the completed song and continues autoplay without waiting', async () => {
  const h = harness();
  await h.context.playSong(track);
  let advances = 0;
  h.context.nextTrack = () => { advances++; vm.runInContext("state.current_track = {id:'next', title:'Next song'}", h.context); };
  h.element('audio-player').listeners.ended();
  assert.equal(advances, 1);
  assert.equal(h.element('song-rating').hidden, false);
  assert.equal(h.element('song-rating-title').textContent, track.title);
  h.context.rateCompletedSong(4);
  const stored = JSON.parse(h.quoteStorage.get('aysha.song-ratings.v1'));
  assert.deepEqual(stored.entries, [{ id: track.id, title: track.title, rating: 4 }]);
  assert.equal(h.element('song-rating').hidden, true);
  assert.match(h.element('song-ratings-list').innerHTML, /4\/5/);
});

test('rated songs skip prompts after reload; dismissing alone does not rate a song', () => {
  const h = harness();
  assert.equal(h.context.askForSongRating(track), true);
  h.context.dismissSongRating();
  assert.equal(h.quoteStorage.size, 0);
  assert.equal(h.context.askForSongRating(track), true);
  h.context.rateCompletedSong(5);
  const refreshed = harness(false, new Map(), h.quoteStorage);
  assert.equal(refreshed.context.askForSongRating(track), false);
  assert.equal(vm.runInContext('songRatings.pending', refreshed.context), null);
});

test('stop, skip and invalid ratings do not save ratings or prompt', async () => {
  const h = harness();
  await h.context.playSong(track);
  await h.context.stopPlay();
  h.element('audio-player').listeners.ended();
  assert.equal(vm.runInContext('songRatings.pending', h.context), null);
  await h.context.nextTrack();
  assert.equal(h.quoteStorage.has('aysha.song-ratings.v1'), false);
  h.context.askForSongRating(track);
  for (const rating of [0, 6, 2.5, '5', NaN]) h.context.rateCompletedSong(rating);
  assert.equal(h.quoteStorage.has('aysha.song-ratings.v1'), false);
});

test('rating prompt remains accessible inside the modal and returns outside when closed', () => {
  const h = harness();
  h.context.askForSongRating(track);
  h.element('now-playing').open = true;
  h.context.renderRatingPrompt();
  assert.equal(h.element('song-rating').hidden, true);
  assert.equal(h.element('full-song-rating').hidden, false);
  h.element('now-playing').open = false;
  h.element('now-playing').listeners.close();
  assert.equal(h.element('song-rating').hidden, false);
});

test('unavailable rating storage works for this page and reports failed persistence honestly', () => {
  const store = { get() { throw Error('Denied'); }, set() { throw Error('Full'); }, delete() { throw Error('Denied'); } };
  const h = harness(false, new Map(), store);
  h.context.askForSongRating(track);
  h.context.rateCompletedSong(3);
  assert.equal(h.context.askForSongRating(track), false);
  assert.match(h.element('full-action-status').textContent, /page only/);
  h.context.clearSongRatings();
  assert.match(h.element('full-action-status').textContent, /may return/);
});

test('malformed rating data is bounded, sanitized and rendered as text', () => {
  const data = { version: 1, entries: [
    { id: 'abc', title: '<script>bad</script>', rating: 4 },
    { id: 'abc', title: 'duplicate', rating: 5 },
    { id: 'bad', title: 'Bad score', rating: 99 },
  ] };
  const h = harness(false, new Map(), new Map([['aysha.song-ratings.v1', JSON.stringify(data)]]));
  h.context.renderSongRatings();
  assert.equal(h.element('song-ratings-count').textContent, 1);
  assert.ok(h.element('song-ratings-list').innerHTML.includes('&lt;script&gt;'));
  assert.ok(!h.element('song-ratings-list').innerHTML.includes('<script>'));
});

test('rating capacity keeps existing ratings instead of evicting them and asking again', () => {
  const h = harness();
  h.context.initializeSongRatings();
  vm.runInContext("songRatings.entries = Array.from({length:1000}, (_,i) => ({id:'song-'+i,title:'Song '+i,rating:4}));", h.context);
  h.context.askForSongRating(track);
  h.context.rateCompletedSong(5);
  assert.equal(vm.runInContext('songRatings.entries.length', h.context), 1000);
  assert.equal(h.context.askForSongRating({ id: 'song-0', title: 'Song 0' }), false);
  assert.match(h.element('full-action-status').textContent, /1,000 ratings/);
});

test('deleting ratings leaves quote preferences and music selection alone', async () => {
  const h = harness();
  await h.context.playSong(track);
  h.context.initializeQuotes();
  h.context.toggleQuoteSave();
  const quoteChoices = h.quoteStorage.get('aysha.comfort.v1');
  const music = h.storage.get('aysha.selection.v1');
  h.context.askForSongRating(track);
  h.context.rateCompletedSong(2);
  h.context.clearSongRatings();
  assert.equal(h.quoteStorage.has('aysha.song-ratings.v1'), false);
  assert.equal(h.quoteStorage.get('aysha.comfort.v1'), quoteChoices);
  assert.equal(h.storage.get('aysha.selection.v1'), music);
  assert.equal(h.context.askForSongRating(track), true);
});

function introHarness() {
  const h = harness();
  let now = 1000;
  let tick;
  let cleared = 0;
  let focused = 0;
  h.context.Date = { now: () => now };
  h.context.setInterval = fn => { tick = fn; return 123; };
  h.context.clearInterval = id => { assert.equal(id, 123); cleared++; };
  h.context.document.activeElement = { focus() { focused++; } };
  const dialog = h.element('love-intro');
  dialog.showModal = () => { dialog.open = true; };
  dialog.close = () => { dialog.open = false; };
  return { ...h, advance(ms) { now += ms; tick(); }, cleared: () => cleared, focused: () => focused };
}

test('love-letter opening presents three two-line verses for five seconds each and reveals at 15s', () => {
  const h = introHarness();
  h.context.startLoveIntro();
  assert.equal(h.element('love-intro').open, true);
  assert.match(h.element('love-intro-lines').innerHTML, /In the quiet/);
  assert.equal((h.element('love-intro-lines').innerHTML.match(/<p>/g) || []).length, 2);
  assert.equal(h.element('love-intro-progress').style.width, '0%');
  h.advance(4999);
  assert.match(h.element('love-intro-lines').innerHTML, /In the quiet/);
  h.advance(1);
  assert.match(h.element('love-intro-lines').innerHTML, /A little world/);
  h.advance(5000);
  assert.match(h.element('love-intro-lines').innerHTML, /Every note/);
  h.advance(4999);
  assert.equal(h.element('love-intro').open, true);
  h.advance(1);
  assert.equal(h.element('love-intro').open, false);
  assert.equal(h.cleared(), 1);
  assert.equal(h.focused(), 1);
  assert.equal(h.calls.length, 0);
  assert.equal(h.element('audio-player').playCount, 0);
  assert.equal(h.quoteStorage.size, 0);
  assert.equal(h.storage.size, 0);
});

test('Skip/Escape clean up letter timers and never resume the intro from a stale tick', () => {
  const h = introHarness();
  h.context.startLoveIntro();
  h.element('love-intro').listeners.cancel();
  assert.equal(h.element('love-intro').open, false);
  assert.equal(h.cleared(), 1);
  h.context.finishLoveIntro();
  assert.equal(h.cleared(), 1);
  assert.equal(h.focused(), 1);
});

test('throttled background opening closes on return instead of delaying all verses', () => {
  const h = introHarness();
  h.context.startLoveIntro();
  h.advance(60000);
  assert.equal(h.element('love-intro').open, false);
  assert.equal(h.cleared(), 1);
});

test('unsupported native dialog leaves the normal player usable without a timer', () => {
  const h = harness();
  let timers = 0;
  h.context.setInterval = () => timers++;
  h.context.startLoveIntro();
  assert.equal(timers, 0);
  assert.notEqual(h.element('love-intro').open, true);
});

function heartEffectsHarness() {
  const h = harness();
  const timers = new Map();
  let timerId = 0;
  const nodes = [];
  const listeners = {};
  const preference = { matches: false };
  h.context.clearTimeout = id => timers.delete(id);
  h.context.setTimeout = fn => { const id = ++timerId; timers.set(id, fn); return id; };
  h.context.document.addEventListener = (name, fn, options) => { listeners[name] = { fn, options }; };
  h.context.matchMedia = () => preference;
  h.context.document.createElement = () => {
    const p = { style: { setProperty(name, value) { this[name] = value; } }, listeners: {},
      classList: { values: new Set(), add(name) { this.values.add(name); }, remove(name) { this.values.delete(name); }, contains(name) { return this.values.has(name); } },
      getBoundingClientRect() { return { left: 20, top: 30, width: 300, height: 300 }; },
      addEventListener(name, fn) { this.listeners[name] = fn; },
      remove() { this.removed = true; },
    };
    nodes.push(p);
    return p;
  };
  for (const id of ['heart-effects-page', 'heart-effects-intro', 'heart-effects-player']) {
    h.element(id).appendChild = particle => { particle.host = id; };
  }
  h.context.initializeHeartEffects();
  return { ...h, get particles() { return nodes.filter(p => p.className.startsWith('heart-particle')); }, get fields() { return nodes.filter(p => p.className === 'heart-reveal'); }, timers, listeners, preference, event: (extra = {}) => ({ clientX: 90, clientY: 100, detail: 1, button: 0, pointerType: 'mouse', buttons: 0, target: { closest: () => null }, ...extra }) };
}

test('click/tap launches eight fluttering local hearts at the click without network/storage/audio', () => {
  const h = heartEffectsHarness();
  h.context.heartPointerClick(h.event());
  assert.equal(h.particles.length, 8);
  assert.equal(h.timers.size, 9);
  assert.ok(h.particles.every(p => p.host === 'heart-effects-page' && p.innerHTML.includes('/icons.svg#heart')));
  for (const p of h.particles) {
    assert.equal(p.style.left, '90px');
    assert.equal(p.style.top, '100px');
    assert.ok(parseFloat(p.style['--heart-lift']) <= -180);
    assert.ok(parseFloat(p.style['--heart-life']) >= 2200 && parseFloat(p.style['--heart-life']) <= 2800);
    assert.ok(parseFloat(p.style['--heart-flap']) >= 150);
    assert.ok(parseFloat(p.style['--heart-size']) >= 18);
  }
  assert.equal(h.fields.length, 1);
  assert.equal(h.calls.length, 0);
  assert.equal(h.quoteStorage.size + h.storage.size, 0);
  assert.equal(h.element('audio-player').playCount, 0);
  assert.equal(h.listeners.click.options.passive, true);
});

test('butterfly hearts use native-dialog layers and survive closing the intro or player', () => {
  const h = heartEffectsHarness();
  h.element('love-intro').open = true;
  h.context.heartPointerClick(h.event());
  assert.ok(h.particles.every(p => p.host === 'heart-effects-intro'));
  h.element('love-intro').open = false;
  h.element('love-intro').listeners.close();
  assert.ok(h.particles.every(p => p.host === 'heart-effects-page'));
  h.element('now-playing').open = true;
  h.context.relocateHeartEffects();
  assert.ok(h.particles.every(p => p.host === 'heart-effects-player'));
  h.element('now-playing').open = false;
  h.element('now-playing').listeners.close();
  assert.ok(h.particles.every(p => p.host === 'heart-effects-page'));
  assert.equal(h.timers.size, 8);
});

test('keyboard, secondary clicks and form editing do not launch butterflies', () => {
  const h = heartEffectsHarness();
  h.context.heartPointerClick(h.event({ detail: 0, clientX: 0, clientY: 0 }));
  h.context.heartPointerClick(h.event({ button: 2 }));
  h.context.heartPointerClick(h.event({ target: { closest: () => ({}) } }));
  assert.equal(h.particles.length, 0);
  assert.equal(typeof h.listeners.pointermove.fn, 'function', 'Hover adds reveal, but clicks do not depend on it');
});

test('butterfly flights are capped and removed on animationend or fallback', () => {
  const h = heartEffectsHarness();
  for (let i = 0; i < 100; i++) h.context.spawnHeartParticle(12, 25);
  assert.equal(h.particles.filter(p => !p.removed).length, 32);
  assert.equal(h.timers.size, 32);
  h.particles.at(-1).listeners.animationend();
  assert.equal(h.timers.size, 31);
  for (const callback of [...h.timers.values()]) callback();
  assert.equal(h.timers.size, 0);
  assert.ok(h.particles.every(p => p.removed));
});

test('explicit always-on hearts ignore reduced motion but hidden pages still stop work', () => {
  const h = heartEffectsHarness();
  h.preference.matches = true;
  h.context.heartPointerClick(h.event());
  assert.equal(h.particles.length, 8);
  assert.equal(h.timers.size, 9);
  h.preference.matches = false;
  h.context.heartPointerClick(h.event());
  assert.equal(h.particles.length, 16);
  h.context.document.hidden = true;
  h.listeners.visibilitychange.fn();
  assert.equal(h.timers.size, 0);
  h.context.heartPointerClick(h.event());
  assert.equal(h.particles.length, 16);
});

test('butterfly initialization is idempotent and invalid coordinates never create nodes', () => {
  const h = heartEffectsHarness();
  h.context.document.addEventListener = () => { throw new Error('Duplicate listener'); };
  h.context.initializeHeartEffects();
  h.context.spawnHeartParticle(NaN, 10);
  h.context.spawnHeartParticle(10, Infinity);
  assert.equal(h.particles.length, 0);
});

test('touch pointerout never cancels newly launched flights', () => {
  const h = heartEffectsHarness();
  h.context.heartPointerClick(h.event({ pointerType: 'touch' }));
  h.listeners.pointerout.fn({ pointerType: 'touch', relatedTarget: null });
  assert.equal(h.particles.filter(p => !p.removed).length, 8);
  assert.equal(h.timers.size, 9);
});

test('butterfly flight and red spotlight CSS coexist', () => {
  const source = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf8');
  assert.ok(source.includes('@keyframes heart-flight'));
  assert.ok(source.includes('@keyframes heart-flutter'));
  assert.ok(source.includes('rotateY('));
  assert.ok(!source.includes('heart-constellation'));
  assert.ok(source.includes('revealHeartField'));
  assert.ok(source.includes('color:#ed304b'));
});


test('embedded-preview coordinate clicks with detail zero still launch butterfly hearts', () => {
  const h = heartEffectsHarness();
  h.context.heartPointerClick(h.event({ detail: 0, clientX: 130, clientY: 220 }));
  assert.equal(h.particles.length, 8);
  assert.ok(h.particles.every(p => p.style.left === '130px' && p.style.top === '220px'));
});

test('hover reveals a stationary red tile pattern without spawning butterflies or growing nodes', () => {
  const h = heartEffectsHarness();
  let now = 1000;
  h.context.Date = { now: () => now };
  h.context.heartPointerMove(h.event());
  assert.equal(h.fields.length, 1);
  assert.equal(h.particles.length, 0);
  const field = h.fields[0];
  assert.ok(field.innerHTML.includes('<pattern'));
  assert.ok(field.innerHTML.includes('width="100%" height="100%"'));
  assert.equal((field.innerHTML.match(/<use /g) || []).length, 1, 'Pattern reuses one local icon, no viewport-sized DOM grid');
  assert.equal(field.style['--reveal-x'], '70px');
  const original = field.innerHTML;
  h.context.heartPointerMove(h.event({clientX: 140}));
  assert.equal(field.style['--reveal-x'], '70px');
  now += 32;
  h.context.heartPointerMove(h.event({clientX: 140}));
  assert.equal(field.style['--reveal-x'], '120px');
  for (let i = 0; i < 100; i++) h.context.revealHeartField(i, i);
  assert.equal(field.innerHTML, original);
  assert.equal(h.fields.length, 1);
  assert.equal(h.timers.size, 1);
  for (const callback of [...h.timers.values()]) callback();
  assert.equal(field.classList.contains('is-revealed'), false);
});

test('mouse exit fades reveal only; touch exit does not cancel flying hearts', () => {
  const h = heartEffectsHarness();
  h.context.heartPointerClick(h.event());
  h.listeners.pointerout.fn({ pointerType: 'touch', relatedTarget: null });
  assert.equal(h.timers.size, 9);
  h.listeners.pointerout.fn({ pointerType: 'mouse', relatedTarget: null });
  assert.equal(h.timers.size, 8);
  assert.equal(h.fields[0].classList.contains('is-revealed'), false);
  assert.ok(h.particles.every(p => !p.removed));
});

test('reveal fields are bounded to page/intro/player and ignore touch scrolling, dragging and edits', () => {
  const h = heartEffectsHarness();
  for (const extra of [{pointerType:'touch'}, {buttons:1}, {isPrimary:false}, {target:{closest:()=>({})}}]) h.context.heartPointerMove(h.event(extra));
  assert.equal(h.fields.length, 0);
  h.context.revealHeartField(80, 90);
  h.element('now-playing').open = true;
  h.context.revealHeartField(80, 90);
  h.element('love-intro').open = true;
  h.context.revealHeartField(80, 90);
  assert.equal(h.fields.length, 3);
  assert.equal(h.timers.size, 3);
  h.context.document.hidden = true;
  h.listeners.visibilitychange.fn();
  assert.equal(h.timers.size, 0);
  assert.ok(h.fields.every(f => !f.classList.contains('is-revealed')));
});
