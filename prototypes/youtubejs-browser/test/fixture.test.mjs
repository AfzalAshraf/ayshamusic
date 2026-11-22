import test from 'node:test';
import assert from 'node:assert/strict';
// Explicit WEB entry: Node is a test runner only, not the library platform.
import * as library from 'youtubei.js/web';
import { evaluateFixture } from '../fixture-adapter.mjs';
import { ID, format, response } from './fixtures.mjs';

const run = raw => evaluateFixture(library, JSON.stringify(raw), ID);
const rejects = (raw, code) => assert.rejects(run(raw), { message: code });

test('real web library parses a fabricated direct AAC response, not playback', async () => {
  const originalFetch = globalThis.fetch;
  let networkCalls = 0;
  globalThis.fetch = async () => { ++networkCalls; throw new Error('network disabled'); };
  try {
    const result = await run(response());
    assert.equal(result.descriptor.video_id, ID);
    assert.equal(result.descriptor.itag, 140);
    assert.equal(result.descriptor.content_length, 123456);
    assert.equal(result.fixture_requests, 1);
    assert.equal(result.media_validated, false);
    assert.equal(result.playback_tested, false);
    assert.equal(networkCalls, 0);
    assert.equal(library.Platform.shim.runtime, 'browser');
  } finally { globalThis.fetch = originalFetch; }
});
test('wrong video never accepted', () => rejects(response({
  videoDetails: { ...response().videoDetails, videoId: 'other000001' }
}), 'video_id_mismatch'));
test('invalid id rejected before library work', () =>
  assert.rejects(evaluateFixture(library, '{}', 'https://bad.test'), { message: 'invalid_video_id' }));
for (const status of ['LOGIN_REQUIRED', 'UNPLAYABLE', 'ERROR', 'CONTENT_CHECK_REQUIRED', undefined]) {
  test(`denial ${status} wins over supplied playable-looking media`, () =>
    rejects(response({ playabilityStatus: { status, reason: 'PRIVATE_BODY_DO_NOT_LOG' } }),
      status === 'LOGIN_REQUIRED' ? 'provider_login_or_challenge' : 'provider_unavailable'));
}
for (const flag of ['isLive', 'isPostLiveDvr', 'isUpcoming', 'isPrivate']) {
  test(`reject ${flag}`, () => rejects(response({ videoDetails: {
    ...response().videoDetails, [flag]: true
  } }), 'unsupported_content'));
}
for (const url of [
  'http://rr1-fixture.googlevideo.com/videoplayback',
  'https://googlevideo.com.evil.test/videoplayback',
  'https://evilgooglevideo.com/videoplayback',
  'https://127.0.0.1/videoplayback',
  'https://rr1-fixture.googlevideo.com:444/videoplayback',
  'https://user:pass@rr1-fixture.googlevideo.com/videoplayback',
  'https://rr1-fixture.googlevideo.com/not-media',
  'https://rr1-fixture.googlevideo.com/videoplayback#fragment',
  'file:///tmp/file'
]) {
  test(`unsafe source ${url} rejected`, () => rejects(response({
    streamingData: { adaptiveFormats: [format({ url })] }
  }), 'no_direct_aac'));
}
for (const overrides of [
  { contentLength: '0' }, { contentLength: '-5' }, { contentLength: '536870913' },
  { contentLength: 'not-a-number' }, { contentLength: undefined },
  { mimeType: 'audio/webm; codecs="opus"' }, { qualityLabel: '1080p' },
  { drmFamilies: ['WIDEVINE'] }, { drmTrackType: 'DRM_TRACK_TYPE_1' },
  { type: 'FORMAT_STREAM_TYPE_OTF' }, { bitrate: 0 }
]) {
  test(`unsupported format ${JSON.stringify(overrides)} rejected`, () => rejects(response({
    streamingData: { adaptiveFormats: [format(overrides)] }
  }), 'no_direct_aac'));
}
for (const overrides of [
  { signatureCipher: 's=FAKE_SECRET&url=FAKE' },
  { cipher: 's=FAKE_SECRET&url=FAKE' },
  { url: format().url + '&n=FAKE_CHALLENGE' }
]) {
  test(`no silent unsupported decipher ${Object.keys(overrides)[0]}`, () => rejects(response({
    streamingData: { adaptiveFormats: [format(overrides)] }
  }), 'interpreter_required'));
}
test('best eligible bitrate wins, never a higher bitrate video stream', async () => {
  const result = await run(response({ streamingData: { adaptiveFormats: [
    format({ bitrate: 900000, itag: 137, qualityLabel: '1080p' }),
    format({ bitrate: 64000, itag: 139 }), format()
  ] } }));
  assert.equal(result.descriptor.itag, 140);
});
test('HLS/DASH-only response is not a direct audio source', () => rejects(response({
  streamingData: { hlsManifestUrl: 'https://manifest.test/hls', dashManifestUrl: 'https://manifest.test/dash' }
}), 'no_direct_aac'));
test('metadata errors are bounded and contain no upstream body', async () => {
  await assert.rejects(evaluateFixture(library, 'PRIVATE_BODY_DO_NOT_LOG', ID), { message: 'malformed_metadata' });
  await assert.rejects(evaluateFixture(library, 'x'.repeat(1024 * 1024 + 1), ID), { message: 'fixture_size_limit' });
});
test('upstream evaluator is deliberately unavailable, not unsafe eval', async () => {
  assert.throws(() => library.Platform.shim.eval({ output: 'FAKE' }, {}), /provide your own JavaScript evaluator/);
});
