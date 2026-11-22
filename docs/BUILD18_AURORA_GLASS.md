# Build 18 — optional Aurora Glass

## User decision and scope

User clarified that Build17's speed was a compliment, requested a larger visual/performance upgrade, and explicitly selected a **new optional theme**, not a replacement default. Reference: uploaded `image-16.png`, three dark-glass music screens. This is a reference-inspired adaptation of the existing app, not an iOS mockup pasted into Android and not a Kotlin/TDLib rewrite.

VersionCode18 / 1.7.0-preview; same Android Audio package, port17652, ARM64/Android8+. Rust backend, Media3, Telegram session/settings paths, Share flow, metadata pipeline, cancellation semantics and actual Mix remain unchanged. No provider request made by this agent, new extractor, runtime, album/artist fabrication, autoplay downloads or licensing-dependent imagery.

## Optional look

Settings → Make it yours → **Aurora Glass**. Crimson remains the default; existing Crimson/Charcoal choices are respected. Uses existing localStorage key with a third validated value; switching back restores original layout. No new external assets, fonts, theme service or generated artist portraits.

- Dark teal/purple ambience, restrained glass gradients, pale mint controls and coral favorites.
- Home order: greeting and heading → featured current selection → horizontally scrolling circular-artwork picks → liked-song list. Existing sources/data remain accurately labeled: no invented trending playlists, album catalog, lyrics or waveforms.
- Featured artwork/title with working Play and Stop, compact horizontally scrolling cards with explicit Play/Queue/Favorite targets.
- Existing Explore/Library lists get round thumbnails and quieter separators; controls and semantics remain intact.
- Larger artwork-led player, centered title/artist, real draggable progress and fixed Stop/Previous/Play/Next/Favorite. Controls and exact dedication remain visible at tested sizes; overflow details can scroll internally.
- Colors/layout scoped to `data-theme="aurora"`; account modal/permissions and release signing behavior preserved. No continuously animated backgrounds or extra blur loops; reduced-motion scrolling respected.

## Performance changes (bounded claims)

1. **Independent Home loading.** Favorites and queue start alongside picks rather than waiting behind provider search. Each section can render independently; all settled errors still propagate to existing retry handling. Deterministic browser test holds search unresolved and verifies favorites + queue already available.
2. **Half the recurring native poll requests after discovery.** Once this page knows its backend is native, it polls the authoritative `/api/player` directly instead of `/api/status` followed by `/api/player` each tick. Single-flight and hidden-tab guards retained. Browser/Linux path still discovers/status-polls normally. This is a request-count reduction, not a claimed doubling of playback/download speed.
3. **Batched list rendering.** Build cards off-DOM in one DocumentFragment, attach once with replaceChildren, use Set membership for liked/queue lookups rather than scanning both lists for each card. Thumbnails retain lazy loading, add asynchronous decode and explicit dimensions. Existing play/queue/heart/keyboard handlers preserved; source IDs URL-encoded; nullable text safely escaped.

No speculative downloads, increased upload rates, provider retries/rotation, persistent search cache, lossy conversion or faster playback clock. Network/source latency can still dominate. No physical RAM/battery/startup benchmarks claimed.

## Verification

Local **122 Node / 142 Python PASS**. Aurora browser fixture PASS: default unchanged, choice restored after reload, settings control, featured/picks/list order, 320x568/390x844/844x390/1440x900 viewport bounds, Play centered, Stop and Play visible at dialog scroll0, parallel loading and no redundant native status calls, theme changes do not issue extra provider searches. Real Chromium screenshots reviewed with abstract fixture artwork (not live music/production metadata).

Full mobile/dashboard/seek/Aurora/static preview browser regression PASS locally. One initial full-browser attempt caught a native-output visibility regression when capability was already known at startup; fixed before delivery. A later local run reported 11 rather than 12 butterflies during a timing-sensitive touch-effect assertion; an unchanged complete rerun passed, and source-matched Linux CI44368990061 passed all six jobs including the browser suite. This is recorded as timing sensitivity, not a claimed universal animation fix.

Initial Android run44368990168: release build/R8/locked phone Rust/size SUCCESS; emulator failed because an uncaught client SocketException in the fixture worker killed instrumentation (`docs/research/build18-initial-native-result.json`). Follow-up **b9ac4dcc02a8ac098da160c8f4c903a20c7913e8** catches only SocketException for Media3's canceled HTTP range clients. Production Media3 unchanged; event deadlines, decoder checks and full-file transfer assertions are retained. Final Android run **44370633275 SUCCESS**: release build job **135036128888**, emulator job **135038776971**, **all eight instrumentation tests PASS**. Actual pause/resume/error/gap/seek and full ten-minute fixture playback (test-only4x clock), share-link allowlist and bounded native Bitmap artwork processing. Results in `docs/research/build18-media3-emulator-result.json`.

Linux run44370633056 then caught the same real-time butterfly displacement timing sensitivity at412x915 (`docs/research/build18-initial-browser-result.json`). Follow-up **e3f389ceee878134d120685f1e753c623e70ca76** changes only the browser test: captures the real touch event synchronously, pauses actual CSS animations, samples their 0ms/400ms timeline positions, then resumes them. Identical counts, touch-release persistence, displacement and cleanup assertions retained; it no longer mistakes runner/protocol scheduling for an animation regression. Production effects unchanged. Complete local browser/dashboard/seek/Aurora/static suite PASS again. Linux **44372066575 SUCCESS, all six jobs PASS**, including the full mobile/Telegram/seek/Aurora/static preview browser suite and provider-offline prototypes. APK source remains b9ac4dc; only test/docs changes follow it. Prior Build17 results are not Build18 release evidence.

## Remaining physical/distribution gates

Preview candidate, not production-certified: stable signing-key continuity/inspection, real-phone cold/warm Share, Telegram new audio/art presentation, background/process death/notification denial, network transitions, accessibility/device-WebView coverage, battery/RAM. If Android rejects an in-place update, **do not uninstall or clear app data** (Telegram session loss risk). Exact dedication and existing launcher retained. Historical flaky emulator seek outcome remains documented under Build17; do not claim an established root-cause fix from an unchanged rerun.


## Build 18 candidate

[Download ARM64 APK artifact (GitHub ZIP)](https://github.com/AfzalAshraf/ayshamusic/actions/runs/44370633275/artifacts/14457844193). Artifact14457844193, ZIP **5,085,899 bytes**, archive SHA256 **94330ccf9c1ed7f4436d1007e52d0c30130aa3e5deed076d15dd0f413da6208a**. These are GitHub ZIP metadata, not an extracted APK hash/signing-certificate inspection. CI enforces release APK size budget15MB. Preview build uses AYSHA_PREVIEW=1; do not represent as production-signed. Retention14days, not a permanent published release.

Emulator results artifact14460405122, ZIP27,764bytes, archiveSHA256 **9b73dfaa06aaf1defee5f0e7efb418796f0ba0f83e877148b520e59164f13432**. Instrumentation excludes provider acquisition/Rust JNI integration; release compilation and locked Rust host tests are separate evidence.

Extract the `.apk`, attempt Update over the current install, then **Settings → Make it yours → Aurora Glass**. Existing theme remains unchanged until selected. If Android rejects Update, stop; never blind-uninstall/clear data. Phone appearance, real music/Telegram art and background behavior remain physical acceptance gates. Local screenshots in ignored `previews/build18/` use abstract fixture artwork, not invented production songs or provider evidence.
