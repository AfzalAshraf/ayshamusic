// Fabricated data only. This is not a downloaded response or the affected song.
export const ID = 'fixture0001';
export function format(overrides = {}) {
  return { itag: 140, mimeType: 'audio/mp4; codecs="mp4a.40.2"',
    bitrate: 128000, audioQuality: 'AUDIO_QUALITY_MEDIUM', audioSampleRate: '44100',
    audioChannels: 2, contentLength: '123456', approxDurationMs: '30000',
    url: 'https://rr1-fixture.googlevideo.com/videoplayback?expire=1999999999&sig=FAKE_ONLY',
    ...overrides };
}
export function response(overrides = {}) {
  return { playabilityStatus: { status: 'OK' }, videoDetails: {
    videoId: ID, title: 'Fabricated offline fixture', lengthSeconds: '30',
    channelId: 'OFFLINE_FIXTURE', author: 'Not a real channel',
    thumbnail: { thumbnails: [] }, isLiveContent: false
  }, streamingData: { expiresInSeconds: '3600', formats: [], adaptiveFormats: [format()] }, ...overrides };
}
