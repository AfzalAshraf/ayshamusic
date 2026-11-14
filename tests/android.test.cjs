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
  assert.match(lib, /layer\(Extension\(android_catalog::new_state\(\)\)\)/);
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
  assert.match(read('android-app/app/build.gradle.kts'), /applicationIdSuffix = "\.preview"/);
});

test('startup waits for the on-phone API and retains external-server independence', () => {
  const activity = read('android-app/app/src/main/java/com/afzal/ayshamusic/MainActivity.kt');
  assert.match(activity, /http:\/\/127.0.0.1:5050/);
  assert.match(activity, /MediaApi.get\("\/api\/status"\)/);
  assert.match(activity, /for \(attempt in 1\.\.30\)/);
  assert.doesNotMatch(activity, /192\.168\.|vercel\.app|e2b\.app/);
  assert.match(read('android-app/app/build.gradle.kts'), /versionCode = 2/);
});
