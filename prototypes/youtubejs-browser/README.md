# Isolated permissive browser extractor evaluation

**Not integrated into the APK. No live network transport. No playback claim.**

User approved on-device extraction, then rejected GPL integration/relicensing.
NewPipe is excluded. This evaluates MIT **youtubei.js18.3.0**'s explicit **web**
entry/bundle against fabricated offline responses. No Node executable would be
shipped: Node here runs development tests, analogous to existing frontend tests.
Android's existing WebView is a possible runtime, **not yet tested/integrated**.

## Reproduce (diagnostic/offline fixtures)

```sh
npm ci --prefix prototypes/youtubejs-browser --ignore-scripts --no-audit --no-fund
npm test --prefix prototypes/youtubejs-browser
# Optional desktop Chromium fixture check, existing test-only tools outside repo:
BROWSER_TEST_MODULES=/path/to/test-tools/node_modules \
  node prototypes/youtubejs-browser/test/browser.cjs
```

Test tools: playwright-core1.65.0 and @sparticuz/chromium153.0.0, as used by the
existing mobile browser CI. On the Arena sandbox Chromium needs
`LD_LIBRARY_PATH=/tmp/al2023/lib`; CI Ubuntu has the system libraries. First local
browser setup attempts failed because Chromium is ESM-only and libnspr4 was not
on the path; both setup issues corrected before reporting a pass. Browser test
filters out packaged Chromium's disable-web-security/allow-insecure-content
flags. No CORS/security disabling is proposed for production.

`fixture-adapter.mjs` constructs a real library session **locally for testing**,
disables player/config/session-cache retrieval and injects a response-only fetch
function. There is no call to global/native fetch. It accepts exactly one fixture
player request; initialization unexpectedly contacting any other endpoint fails.
The standalone browser test intercepts every page request and rejects anything
outside its fabricated local origin. No music-provider query is run.

Selection rejects video mismatch, provider denials, live/upcoming/private media,
video/DRM/OTF/non-AAC formats, unsafe source URLs, unknown/oversized length and
cipher/`n` transformation requirements. Returned URL is **private internal data**,
not a status response. Future integration would require Rust revalidation and
head/tail checks. Fixture result explicitly says `media_validated:false` and
`playback_tested:false`.

## Pinning and licenses

- youtubei.js18.3.0, MIT, published2026-10-23; package-lock has npm integrity.
  https://github.com/LuanRT/YouTube.js
- Resolved source-module dependencies: @bufbuild/protobuf2.17.0
  (Apache-2.0 AND BSD-3-Clause), fflate0.8.3 (MIT), meriyah7.4.1 (ISC).
  meriyah overridden because registry latest7.4.2 is dated2026-11-08, later than
  the session date. All selected versions were available by2026-11-07.
- Browser test additionally checks upstream's packaged self-contained browser
  bundle from the same npm18.3.0 tarball; SHA256:
  `343518935745d56c7ee4082d6e59c568f6a19a5e78c0e3386a38892260d37d38`.
  npm overrides do **not** rewrite embedded prebundled dependency code. Before
  shipping, rebuild from pinned reviewed source modules, enumerate embedded
  versions/notices and audit that exact artifact, not only npm's dependency tree.
- npm audit of installed source-module graph reported zero known advisories at
  evaluation time. This is not a security certification or an audit of embedded
  prebundled code, provider behavior or Android runtime.
- No upstream code/binary is vendored or redistributed in Git here. No app
  license changed. Shipping would require retaining MIT/ISC/BSD/Apache notices;
  permissive is not the same as notice-free. Inspect third-party source as well
  as top-level metadata before final distribution.

## Unresolved gates

1. Default browser evaluator **throws** when asked to decipher. Do not use
   `eval`/`new Function` on downloaded player scripts in a privileged UI WebView.
   A vetted sandboxed interpreter/transform implementation and its license,
   resource limits, Android size and engine isolation are still needed for signed
   formats. Direct fixture URLs avoid this requirement and do not prove it away.
2. Upstream HTTPClient defaults to `credentials:include` and automatic redirects.
   A native bounded transport must override both, allowlist metadata endpoints,
   validate every redirect, reject credentials/cookies, latch restrictions,
   enforce time/request/byte budgets and redact all upstream errors. Do not call
   browser fetch directly or disable CORS/WebView security.
3. A fresh, explicitly justified source test must respect the sealed CI challenge
   and stop rules. No alternate client/network/proxy retry was made here.
4. Actual Android WebView/minSdk/R8/JNI/cancellation, Rust handoff/Telegram archive,
   complete affected-song playback and physical phone/network acceptance remain
   open. Existing APK and UI are unchanged. Do not deliver this as a fixed APK.
