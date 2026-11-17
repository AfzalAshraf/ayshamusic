// Isolated research ONLY. No network transport or Android integration exists here.
// Nothing in this directory is bundled into the APK.
export class FixtureError extends Error {
  constructor(code) { super(code); this.name = 'FixtureError'; this.code = code; }
}
const fail = code => { throw new FixtureError(code); };
const MAX_MEDIA = 512 * 1024 * 1024;
const MAX_METADATA = 1024 * 1024;

export function selectDirectAac(info, requestedId) {
  if (!/^[A-Za-z0-9_-]{11}$/.test(requestedId)) fail('invalid_video_id');
  // Denials take precedence over any streamingData supplied alongside them.
  const status = info.playability_status?.status;
  if (status === 'LOGIN_REQUIRED') fail('provider_login_or_challenge');
  if (status !== 'OK') fail('provider_unavailable');
  if (info.basic_info?.id !== requestedId) fail('video_id_mismatch');
  if (info.basic_info?.is_live || info.basic_info?.is_post_live_dvr ||
      info.basic_info?.is_upcoming || info.basic_info?.is_private) fail('unsupported_content');
  const formats = info.streaming_data?.adaptive_formats || [];
  let needsInterpreter = false;
  const candidates = [];
  for (const f of formats) {
    if (!f.has_audio || f.has_video || f.is_type_otf || f.drm_families?.length ||
        f.drm_track_type || f.fair_play_key_uri ||
        !/^audio\/mp4;\s*codecs="mp4a\.40\.2"$/.test(f.mime_type || '')) continue;
    if (f.signature_cipher || f.cipher) { needsInterpreter = true; continue; }
    let url;
    try { url = new URL(f.url); } catch { continue; }
    if (url.protocol !== 'https:' || url.port || url.username || url.password || url.hash ||
        !/^[a-z0-9-]+\.googlevideo\.com$/.test(url.hostname) ||
        url.pathname !== '/videoplayback') continue;
    // Do not silently return an untransformed n challenge as a working source.
    if (url.searchParams.has('n')) { needsInterpreter = true; continue; }
    if (!Number.isSafeInteger(f.content_length) || f.content_length <= 0 ||
        f.content_length > MAX_MEDIA || !Number.isSafeInteger(f.itag) || f.itag <= 0 ||
        !Number.isFinite(f.bitrate) || f.bitrate <= 0) continue;
    candidates.push({ video_id: requestedId, url: url.href, itag: f.itag,
      content_length: f.content_length, bitrate: f.bitrate, mime: f.mime_type });
  }
  candidates.sort((a, b) => b.bitrate - a.bitrate || a.itag - b.itag);
  if (!candidates.length) fail(needsInterpreter ? 'interpreter_required' : 'no_direct_aac');
  // Private result: never serialize into UI/status/logs. Rust must independently
  // revalidate it and prepare head+tail before treating it as playable.
  return candidates[0];
}

/**
 * Real library session/parser exercised against one IN-MEMORY response only.
 * generate_session_locally is fixture setup, NOT a live acquisition strategy.
 * The supplied library must be youtubei.js/web (or the identical web bundle).
 */
export async function evaluateFixture(library, raw, requestedId) {
  if (!/^[A-Za-z0-9_-]{11}$/.test(requestedId)) fail('invalid_video_id');
  if (typeof raw !== 'string' || new TextEncoder().encode(raw).length > MAX_METADATA)
    fail('fixture_size_limit');
  let parsed;
  try { parsed = JSON.parse(raw); } catch { fail('malformed_metadata'); }
  if (parsed?.playabilityStatus?.status === 'LOGIN_REQUIRED') fail('provider_login_or_challenge');
  if (parsed?.playabilityStatus?.status !== 'OK') fail('provider_unavailable');
  // Disable upstream raw-body/session logging. Do not attach upstream errors as
  // causes: they can contain source URLs, provider bodies or visitor identifiers.
  library.Log.setLevel(library.Log.Level.NONE);
  let calls = 0;
  const fixtureFetch = async (input, init) => {
    if (++calls !== 1) fail('unexpected_fixture_request');
    const url = new URL(typeof input === 'string' ? input : input.url);
    if (url.origin !== 'https://www.youtube.com' || url.pathname !== '/youtubei/v1/player' ||
        (init?.method || input.method) !== 'POST') fail('unexpected_fixture_request');
    const body = JSON.parse(init.body);
    if (body.videoId !== requestedId || body.serviceIntegrityDimensions ||
        new Headers(init.headers).has('Authorization') ||
        new Headers(init.headers).has('Cookie')) fail('unexpected_fixture_request');
    // No native/global fetch call: impossible to contact a provider here.
    return new Response(raw, { status: 200, headers: { 'Content-Type': 'application/json' } });
  };
  try {
    const tube = await library.Innertube.create({
      generate_session_locally: true, retrieve_player: false,
      retrieve_innertube_config: false, enable_session_cache: false,
      fail_fast: true, fetch: fixtureFetch, lang: 'en', location: 'US'
    });
    const info = await tube.getBasicInfo(requestedId);
    const descriptor = selectDirectAac(info, requestedId);
    if (calls !== 1) fail('unexpected_fixture_request');
    return { descriptor, fixture_requests: calls, media_validated: false, playback_tested: false };
  } catch (error) {
    if (error instanceof FixtureError) throw error;
    fail('library_fixture_failed');
  }
}
