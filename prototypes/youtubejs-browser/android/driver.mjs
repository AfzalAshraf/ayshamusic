// Android test APK asset ONLY. No native bridge, external fetch, real metadata
// or signed source URL is present. Only this sanitized result reaches the test.
import * as library from 'youtubei.js/web';
import { evaluateFixture } from '../fixture-adapter.mjs';
import { ID, format, response } from '../test/fixtures.mjs';
(async () => { try {
  const result = await evaluateFixture(library, JSON.stringify(response()), ID);
  const negative = [];
  for (const raw of [
    response({ playabilityStatus: { status: 'LOGIN_REQUIRED', reason: 'FIXTURE_DENIAL' } }),
    response({ videoDetails: { ...response().videoDetails, videoId: 'other000001' } }),
    response({ streamingData: { adaptiveFormats: [format({ signatureCipher: 's=FIXTURE_ONLY' })] } })
  ]) {
    try { await evaluateFixture(library, JSON.stringify(raw), ID); negative.push('unexpected_success'); }
    catch (error) { negative.push(error.message); }
  }
  console.log('AYSHA_FIXTURE_RESULT ' + JSON.stringify({ status: 'pass',
    runtime: library.Platform.shim.runtime, itag: result.descriptor.itag,
    fixture_requests: result.fixture_requests, media_validated: result.media_validated,
    playback_tested: result.playback_tested, negative }));
} catch {
  console.log('AYSHA_FIXTURE_RESULT ' + JSON.stringify({ status: 'fail', code: 'android_fixture_failed' }));
} })();
