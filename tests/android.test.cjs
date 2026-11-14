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
  assert.match(read('android-app/app/build.gradle.kts'), /applicationIdSuffix = "\.context7"/);
});

test('startup waits for the on-phone API and retains external-server independence', () => {
  const activity = read('android-app/app/src/main/java/com/afzal/ayshamusic/MainActivity.kt');
  assert.match(activity, /http:\/\/127.0.0.1:5050/);
  assert.match(activity, /MediaApi.get\("\/api\/status"\)/);
  assert.match(activity, /for \(attempt in 1\.\.30\)/);
  assert.doesNotMatch(activity, /192\.168\.|vercel\.app|e2b\.app/);
  assert.match(read('android-app/app/build.gradle.kts'), /versionCode = 7/);
});


test('Android passes application context across JNI before starting Rust workers', () => {
  const bridge = read('android-app/app/src/main/java/com/afzal/ayshamusic/RustBridge.kt');
  assert.match(bridge, /private external fun startServer\(filesDir: String, cacheDir: String, context: Context\)/);
  assert.match(bridge, /startServer\(filesDir, cacheDir, context.applicationContext\)/);
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
