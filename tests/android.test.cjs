const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const read = p => fs.readFileSync(p, 'utf8');

test('Android router serves every website catalogue/library/navigation/icon dependency', () => {
  const routes = read('src/android_catalog.rs');
  for (const path of ['/icons.svg', '/api/search', '/api/queue', '/api/queue_add', '/api/queue_clear', '/api/liked', '/api/like', '/api/unlike', '/api/next', '/api/prev']) {
    assert.ok(routes.includes(`.route("${path}"`), path);
  }
  const lib = read('src/lib.rs');
  assert.match(lib, /merge\(android_catalog::routes\(\)\)/);
  assert.match(lib, /layer\(Extension\(phone\)\)/);
  assert.match(routes, /image\/svg\+xml/);
  assert.match(routes, /include_str!\("icons.svg"\)/);
});

test('Android native capability is embedded before first interaction, not only polled', () => {
  assert.ok(read('src/lib.rs').includes('INDEX_HTML.replace("nativePlayback: false", "nativePlayback: true")'));
  assert.match(read('src/index.html'), /nativePlayback: false/);
});

test('Android branch APK build never overwrites the main public release', () => {
  const workflow = read('.github/workflows/android.yml');
  assert.doesNotMatch(workflow, /action-gh-release|contents: write/);
  assert.match(workflow, /upload-artifact/);
  assert.match(workflow, /cargo test --lib/);
  assert.match(workflow, /AYSHA_PREVIEW: "1"/);
  assert.match(read('android-app/app/build.gradle.kts'), /applicationIdSuffix = "\.rustaudio10"/);
});

test('startup waits for the on-phone API and retains external-server independence', () => {
  const activity = read('android-app/app/src/main/java/com/afzal/ayshamusic/MainActivity.kt');
  assert.match(activity, /MediaApi.baseUrl/);
  assert.match(activity, /MediaApi.verifyBackend\(\)/);
  assert.match(activity, /for \(attempt in 1\.\.30\)/);
  assert.doesNotMatch(activity, /192\.168\.|vercel\.app|e2b\.app/);
  assert.match(read('android-app/app/build.gradle.kts'), /versionCode = 10/);
});


test('Android passes application context across JNI before starting Rust workers', () => {
  const bridge = read('android-app/app/src/main/java/com/afzal/ayshamusic/RustBridge.kt');
  assert.match(bridge, /private external fun startServer\(filesDir: String, cacheDir: String, context: Context, port: Int, engine: String, instance: String\)/);
  assert.match(bridge, /startServer\(filesDir, cacheDir, context.applicationContext, BuildConfig.API_PORT, BuildConfig.AUDIO_ENGINE, MediaApi.instanceId\)/);
  const lib = read('src/lib.rs');
  assert.match(lib, /context: JObject<'local>/);
  const start = lib.slice(lib.indexOf('pub extern "system" fn Java_com_afzal_ayshamusic_RustBridge_startServer'));
  assert.ok(start.indexOf('initialize_audio_context(&mut env, &context)') < start.indexOf('boot(files_dir, cache_dir)'));
});

test('Android pins a global context once and guards CPAL audio initialization', () => {
  const lib = read('src/lib.rs');
  assert.match(lib, /static AUDIO_CONTEXT: Mutex<Option<GlobalRef>>/);
  const init = lib.slice(lib.indexOf('fn initialize_audio_context('), lib.indexOf('/// JNI version negotiation'));
  assert.ok(init.indexOf('AUDIO_CONTEXT.lock()') < init.indexOf('if stored.is_some()'));
  assert.ok(init.indexOf('if stored.is_some()') < init.indexOf('ndk_context::initialize_android_context('));
  assert.match(init, /env.new_global_ref\(context\)/);
  assert.match(init, /vm.get_java_vm_pointer\(\).cast\(\)/);
  assert.match(init, /global.as_obj\(\).as_raw\(\).cast\(\)/);
  assert.match(init, /\*stored = Some\(global\)/);
  const player = read('src/player.rs');
  const loop = player.slice(player.indexOf('fn audio_loop('));
  assert.ok(loop.indexOf('if !crate::android::audio_context_ready()') < loop.indexOf('DeviceSinkBuilder::open_default_sink()'));
  assert.match(read('Cargo.toml'), /ndk-context = \{ version = "=0.1.1", optional = true \}/);
});


test('release shrinking preserves the Kotlin object and its JNI signatures', () => {
  assert.match(read('android-app/app/proguard-rules.pro'), /-keep class com\.afzal\.ayshamusic\.RustBridge \{[^}]*\*;\s*\}/);
});


test('native decoder input is seekable and PCM callback does not fetch network', () => {
  const player = read('src/player.rs');
  assert.match(player, /with_byte_len\(len\)/);
  assert.match(player, /with_seekable\(true\)/);
  assert.match(player, /PcmSource::prepare/);
  assert.match(read('src/pcm_stream.rs'), /try_recv\(\)/);
  assert.match(read('.github/workflows/android.yml'), /AYSHA_MEDIA_FIXTURES/);
  assert.match(read('src/range_media.rs'), /old unseekable path must reproduce/);
});

test('Telegram account setup is in Settings, not Now Playing, and source order is Telegram first', () => {
  const html = read('src/index.html');
  const settings = html.slice(html.indexOf('id="tab-settings"'), html.indexOf('<div class="footer">'));
  assert.match(settings, /openTelegramAccount/);
  assert.match(settings, />Add Telegram account</);
  assert.doesNotMatch(settings, /id="telegram-account-button"[^>]*hidden/);
  const player = html.slice(html.indexOf('id="now-playing"'), html.indexOf('<script>'));
  assert.doesNotMatch(player, /Reset learning|tg-api-hash|tg-password/);
  const lib = read('src/lib.rs');
  const start = lib.slice(lib.indexOf('async fn start_track('));
  assert.ok(start.indexOf('tg_storage::lookup') < start.indexOf('youtube::resolve_forced'));
  const storage = read('src/tg_storage.rs');
  assert.match(storage, /PeerId::self_user/);
  assert.match(storage, /iter_download/); assert.match(storage, /upload_stream/);
  assert.match(storage, /archive_incomplete/);
});


test('dual audio APKs isolate identities, ports and playback-only dependencies', () => {
  const gradle = read('android-app/app/build.gradle.kts');
  for (const suffix of ['rustaudio10', 'androidaudio10']) assert.ok(gradle.includes(`applicationIdSuffix = ".${suffix}"`));
  for (const port of ['17651', '17652']) assert.ok(gradle.includes(`"API_PORT", "${port}"`));
  assert.match(gradle, /"androidAudioImplementation"\("androidx.media3:media3-exoplayer:1.8.0"\)/);
  const api = read('android-app/app/src/main/java/com/afzal/ayshamusic/MediaApi.kt');
  assert.match(api, /UUID.randomUUID/);
  for (const key of ['instance_id', 'audio_engine', 'api_port']) assert.ok(api.includes(`"${key}"`));
  assert.match(api, /if \(!verified\) return/);
  const native = read('android-app/app/src/androidAudio/java/com/afzal/ayshamusic/PlatformAudio.kt');
  assert.match(native, /setHandleAudioBecomingNoisy\(true\)/);
  assert.match(native, /setAudioAttributes/);
  assert.match(native, /STATE_READY/);
  assert.doesNotMatch(native, /youtube\.com|googlevideo\.com|my\.telegram\.org/);
  const workflow = read('.github/workflows/android.yml');
  for (const name of ['ayshamusic-rust-audio-arm64', 'ayshamusic-android-audio-arm64']) assert.ok(workflow.includes(name));
});

test('red launcher supports both adaptive masks and legacy round icons without recoloring Play', () => {
  assert.match(read('android-app/app/src/main/res/values/colors.xml'), /#E82D46/);
  for (const icon of ['ic_launcher', 'ic_launcher_round']) {
    assert.match(read(`android-app/app/src/main/res/mipmap-anydpi-v26/${icon}.xml`), /adaptive-icon/);
  }
  assert.doesNotMatch(read('src/index.html'), /--accent: ?#e82d46/i);
});
