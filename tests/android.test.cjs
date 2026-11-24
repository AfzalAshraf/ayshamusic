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
  assert.doesNotMatch(workflow, /contents: write|gh release create|AYSHA_RELEASE_ENABLED/);
  assert.match(workflow, /upload-artifact/);
  assert.match(workflow, /cargo test --lib/);
  assert.match(workflow, /AYSHA_PREVIEW: "1"/);
  assert.match(read('android-app/app/build.gradle.kts'), /applicationIdSuffix = "\.androidaudio10"/);
});

test('startup waits for the on-phone API and retains external-server independence', () => {
  const activity = read('android-app/app/src/main/java/com/afzal/ayshamusic/MainActivity.kt');
  assert.match(activity, /MediaApi.baseUrl/);
  assert.match(activity, /MediaApi.verifyBackend\(\)/);
  assert.match(activity, /for \(attempt in 1\.\.30\)/);
  assert.doesNotMatch(activity, /192\.168\.|vercel\.app|e2b\.app/);
  assert.match(read('android-app/app/build.gradle.kts'), /versionCode = 23/);
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
  assert.match(settings, /aria-label="Add Telegram account"/);
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


test('Media3 is the only delivered Android flavor; Rust still owns source and identity', () => {
  const gradle = read('android-app/app/build.gradle.kts');
  assert.match(gradle, /applicationIdSuffix = "\.androidaudio10"/);
  assert.doesNotMatch(gradle, /create\("rustAudio"\)/);
  assert.ok(gradle.includes('"API_PORT", "17652"'));
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
  assert.ok(workflow.includes('ayshamusic-android-audio-arm64'));
  assert.doesNotMatch(workflow, /Upload Rust Audio APK|ayshamusic-rust-audio-arm64/);
  const build = read('android-app/scripts/build_apk.sh');
  assert.match(build, /assembleAndroidAudioRelease/);
  assert.doesNotMatch(build, /for VARIANT in rustAudio/);
});

test('romantic rose-gold Aysha launcher supports adaptive and legacy icons without recoloring Play', () => {
  assert.match(read('android-app/app/src/main/res/values/colors.xml'), /#210817/);
  for (const icon of ['ic_launcher', 'ic_launcher_round']) {
    const xml = read(`android-app/app/src/main/res/mipmap-anydpi-v26/${icon}.xml`);
    assert.match(xml, /adaptive-icon/);
    assert.match(xml, /@drawable\/ic_launcher_artwork/);
    assert.doesNotMatch(xml, /launcher_red|ic_launcher_foreground/);
  }
  assert.doesNotMatch(read('src/index.html'), /--accent: ?#e82d46/i);
});


test('Media3 preflight retains first/tail before platform Play and source proof stays sealed', () => {
  const platform = read('src/platform_audio.rs');
  assert.ok(platform.indexOf('PreparedFetch::new') < platform.indexOf('"op":"play"'));
  const source = read('src/range_media.rs');
  assert.match(source, /media_redirect_target_rejected/);
  assert.match(source, /media_redirect_limit/);
  assert.match(source, /header\(reqwest::header::RANGE/);
  assert.match(source, /Duration::from_secs\(12\)/);
  assert.match(read('prototypes/media3-source-check.rs'), /LIVE_APPROVAL_AVAILABLE:bool=false/);
  const workflow = read('.github/workflows/android.yml');
  assert.doesNotMatch(workflow, /AYSHA_MEDIA3_SOURCE_APPROVAL|production-source-once:/);
  assert.match(workflow, /media3-emulator-fixture:/);
});


test('long playback keeps the loopback read budget above the Rust worker budget', () => {
  const native = read('android-app/app/src/androidAudio/java/com/afzal/ayshamusic/PlatformAudio.kt');
  assert.match(native, /setReadTimeoutMs\(20000\)/);
  assert.match(native, /setConnectTimeoutMs\(8000\)/);
  assert.match(native, /setMediaSourceFactory/);
  const ranges = read('src/range_media.rs');
  assert.doesNotMatch(ranges, /media_url_lock_failed/);
  assert.match(ranges, /redirected_response\(&self.url,send\)/);
  assert.match(read('src/platform_audio.rs'), /stage=stream; offset=/);
});


test('build14 Media3 source policy is single-profile and shared with Telegram', () => {
  const youtube = read('src/youtube.rs');
  assert.match(youtube, /MEDIA3_CLIENTS: \[InnertubeClient; 1\] = \[VISIONOS\]/);
  assert.match(read('src/lib.rs'), /youtube::playback_clients\(audio_engine_name\(\) == "media3"\)/);
  assert.match(read('src/tg_storage.rs'), /crate::resolve_default\(http,id,/);
  assert.match(youtube, /"VISIONOS" => "101"/);
  assert.match(read('src/source_selection.rs'), /terminal_provider_error/);
  assert.match(read('src/range_media.rs'), /media_prepare_tail_failed/);
});

test('native ready events have an independent executor from slow Next controls', () => {
  const api=read('android-app/app/src/main/java/com/afzal/ayshamusic/MediaApi.kt');
  assert.match(api,/val events = java.util.concurrent.Executors.newSingleThreadExecutor\(\)/);
  assert.match(api,/if \(event\) events else if \(trackChange\) navigation else posts/);
  assert.match(api,/readTimeout = if \(trackChange\) 60000 else TIMEOUT_MS/);
  assert.match(api,/navigating.compareAndSet\(false, true\)/);
  assert.match(api,/if \(trackChange\) navigating.set\(false\)/);
});

test('shared links are ID-only and handed to the verified local page, not loaded as navigation', () => {
  const manifest=read('android-app/app/src/main/AndroidManifest.xml');
  assert.match(manifest,/android.intent.action.SEND/);assert.match(manifest,/android:mimeType="text\/plain"/);
  const activity=read('android-app/app/src/main/java/com/afzal/ayshamusic/MainActivity.kt');
  assert.match(activity,/RustBridge.sharedVideoId/);assert.match(activity,/JSONObject.quote\(id\)/);
  assert.match(activity,/if \(!localPageReady \|\| !MediaApi.isBackendVerified\(\)\) return/);
  assert.doesNotMatch(activity,/request.grant\(request.resources\)/);
  assert.doesNotMatch(manifest,/READ_EXTERNAL_STORAGE|WRITE_EXTERNAL_STORAGE|MEDIA_CONTENT_CONTROL/);
  const storage=read('src/tg_storage.rs');assert.match(storage,/\.document\(uploaded\)/);assert.match(storage,/Attribute::Audio/);assert.match(storage,/song_metadata::filename/);
  assert.match(read('android-app/app/build.gradle.kts'),/Production release requires your release keystore/);
});

test('native icon picker remains while automatic updater is removed', () => {
  const manifest = read('android-app/app/src/main/AndroidManifest.xml');
  for (const name of ['Rose','Classic','Velvet','Cute','Red','Ivory','Butterflies','Embossed']) {
    assert.match(manifest, new RegExp('android:name="\\.Launcher'+name+'"'));
  }
  assert.equal((manifest.match(/android:enabled="true" android:exported="true" android:icon="@mipmap\/ic_/g)||[]).length, 1);
  assert.doesNotMatch(manifest, /REQUEST_INSTALL_PACKAGES|FileProvider/);
  const activity=read('android-app/app/src/main/java/com/afzal/ayshamusic/MainActivity.kt');
  assert.match(activity, /request\.isForMainFrame && trustedSettingsPage/);
  assert.doesNotMatch(activity, /AppUpdater|install-update|check-update/);
  assert.doesNotMatch(read('src/index.html'), /app-update-heading|check-update|auto-app-update/);
});
