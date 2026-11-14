# Android-first frontend update

## Implemented

- Original dark Aysha visual design with warm rose artwork, clearer typography and compact song rows. This is music-app inspired, not an official YouTube Music clone or branded product.
- Portrait-responsive layout, dynamic viewport height, safe-area-aware bottom navigation, visible labels, 44px primary controls and a mini-player that does not require horizontal page scrolling.
- Full-screen Now Playing dialog using the existing audio element and existing controls. Browser Back closes it before leaving the page; Forward restores it; Escape/cancel is history-aware.
- Tabs use browser history, restore scroll positions and support a header back button. Home is the fallback for a direct link with no in-app history.
- Search/genre buttons work with a stale-response guard. Search results are not overwritten by the Explore tab's initial genre fetch.
- Loading skeletons, empty states, a live error message region, keyboard controls, reduced motion and browser zoom support.
- Added local back/collapse SVG symbols. Removed the frontend's links/registration to unserved PWA manifest/service-worker resources; no new offline/installable-PWA behavior is claimed.
- Browser seek is not offered by the new full player because the sequential server stream does not implement byte ranges. Volume and playback are real controls, not decorative stubs.

Only src/index.html is served by the Rust root handler. Rebuilding alone does not install its new embedded HTML.

## Install on the confirmed Linux host

```bash
cd /home/afine/ayshamusic
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
bash scripts/install_local.sh
```

Stop a foreground music-server with Ctrl-C first if one is running. The installer manages the systemd binary and service, preserving its existing checks/backups. Reload the page after installation; if an old page still appears, use a hard refresh and inspect any old service-worker registration from a previous deployment before clearing site data. Clearing site data may remove local settings/login state.

## Automated checks

Existing test suites:

```bash
node --test tests/frontend.test.cjs
python3 -m unittest discover -s tests -p 'test_*.py' -v
```

Real-browser layout/history checks (test-only packages, installed outside the repo):

```bash
npm install --prefix /tmp/aysha-browser-check --no-audit --no-fund \
  playwright-core@1.65.0 @sparticuz/chromium@153.0.0
BROWSER_TEST_MODULES=/tmp/aysha-browser-check/node_modules \
  node tests/mobile.browser.cjs
```

The packaged Linux headless Chromium needs its OS shared libraries (including libnss3/libnspr4). This does not install Node, npm or a browser as a production player dependency. On minimal containers, use the package's bundled shared libraries or a suitable browser-test image; do not confuse a browser-launch dependency error with an application error.

For screenshots, set UI_SCREENSHOT_DIR to a temporary/output directory. Generated previews are excluded from Git. The suite uses read-only sample API fixtures and sample artwork, not the live server or actual song downloads.

Checked viewports: 320x740, 360x800, 390x844, 412x915, 768x1024, 844x390 and 1440x900. Assertions cover viewport overflow, navigation size, tab/search/genre behavior, in-app Back, browser Back/Forward and modal close/restore, plus uncaught page errors.

## Manual Android acceptance still needed

- Open in the actual Android Chromium browser; verify portrait, rotation and changing address-bar/keyboard height.
- Play actual audio; open/close the player without restarting it; check pause/resume, Next, Previous and both volume sliders.
- Test the hardware/system Back gesture, screen lock, background playback and resuming after network loss. Desktop emulation does not certify these.
- Check TalkBack/focus behavior and large text/zoom, especially long titles and errors.
- Two-device independence, Bluetooth radio connection, authentication and tunnel streaming remain separate backend/networking gates.

UI provisional readiness: 4/5 after automated layout checks; not 5/5 without the target-device checks above. Other current department scores remain Product/Architecture 2/5, Backend 2/5, QA/Security 3/5, Networking/Self-Hosting 1/5.

## Cloudflare and nightly updates: pending networking decisions

No cloudflared installation, tunnel, DNS change or nightly timer was made in this UI pass. The agent workspace is not the user's Linux host, and no remote shell access to that host is available here.

Before preparing the host-side networking/update installer, confirm:

1. The actual domain/subdomain and whether Cloudflare manages its DNS; who may log in.
2. Exact maintenance time and IANA timezone. Do not infer timezone from an SSH machine timestamp or the user's approximate location.
3. Update scope: cloudflared only, music-player releases too, or Ubuntu security updates too. These are different mechanisms and risk levels.
4. Whether updates must defer while audio is playing, and how rollback should work.

Recommended policy for approval: check nightly, install only within an approved idle maintenance window, run tests/health checks before accepting player changes, retain rollback, and never auto-merge unreviewed repository changes or restart active listeners just because the clock passed a guessed time.

Public rollout still requires device-session isolation and Access protection described in docs/REMOTE_MULTI_DEVICE_PLAN.md. Do not publish the current unauthenticated shared-state API while those gates remain open.

## Stop versus Pause regression

After choosing a track, Stop now releases its stream/process while retaining the selection in this page. Any shared Play control (soundtrack card, mini-player or expanded player) starts that track again from the beginning with /api/play. Pause retains the active source/process, so Play resumes via /api/resume without registering the track twice. Repeated Stop remains restartable. The last selected song/output now survives a reload in the same tab via sessionStorage; see the refresh regression below.

Manual target check: select a song, hear it, press the soundtrack card's Stop, press Play, and confirm it starts again. Repeat using expanded-player Stop/Play, and test Pause/Play separately. The browser fixture suite exercises these buttons with media calls stubbed; audible restart and Bluetooth hardware still require local checks.


## Refresh / empty Now Playing regression

The last selected track and output mode are saved in this tab's `sessionStorage` under `aysha.selection.v1`. Reload restores validated metadata, generated artwork and a Play state synchronously, without autoplay or a misleading Pause icon. Press Play to restart from the beginning. Browser audio cannot continue uninterrupted across a document reload; seeking to an old position is not promised by the sequential proxy. Bluetooth may continue on the server across a browser refresh, but this page conservatively offers a restart rather than assuming a server process exists.

Storage is tab/origin-scoped, not a cross-device profile, permanent library or guarantee after closing the tab. Corrupt data and unavailable storage fall back to normal in-memory playback; refresh recovery then needs a new selection or available server metadata. No listening-time or quote-preference tracking was added. Stored fields are only track ID, title, artist, URL and output mode. Clearing this site's session data removes them.

Status polling must not replace an existing local selection with another device's track or claim browser playback solely because global `/api/status` says playing. If a fresh tab has no selection, available server metadata can provide a restartable candidate, not an automatically playing audio source. This does not isolate backend controls/queues across devices.

The real Chromium fixture test clicks a Home card, Stop/Play, expanded Stop/Play, then actually reloads both after Stop (server track null) and during playback (server still playing). It checks restored text, no autoplay, working Play and no uncaught errors. Media output alone is stubbed; live audio remains a host/device test. MediaSession artwork is derived from the track ID because restored/server tracks may not supply a thumbnail; undefined artwork otherwise throws in Chromium before playback starts.


## Quotes bottom tab

The user-requested fifth tab is Quotes (Home, Explore, Library, Quotes, Bluetooth). At <=900px it uses a five-column safe-area bottom bar; desktop retains the sidebar. The existing heart symbol is reused without a new icon/asset dependency. The separate #quotes page offers 24 local original messages, category chips, Next, save/unsave, an empty Saved view and collapsed preference controls. Warm rose/serif quote cards extend the approved design rather than replacing the music screens or dedication.

Personalization is opt-in/off by default and affects only the For you mix through explicit saved categories and More like this actions. There is no gaze/dwell/mood tracking, timer, remote quote request or inference. A dedicated localStorage record (aysha.comfort.v1, bounded/validated IDs and category counters) persists only explicit choices. Reset deletes it, leaving music sessionStorage intact. Unavailable storage retains in-memory interaction with an honest message. Reset failure reports that old choices may return on reload. No account sync, app-wide hide-nav setting or anxiety/relationship/safety guarantee is claimed.

Browser checks cover Quotes at all seven viewports, five >=44px navigation targets, no overflow, category/Next/save/empty/reset/opt-in, real #quotes reload, Back/Forward and no audio-source/play-count change when entering Quotes during fixture playback. During this test, reloading #quotes then returning Home revealed the previous history path suppressed loading a never-loaded Home view; renderTab now fetches missing content rather than marking an uninitialized tab loaded. Screenshot previews/mobile-quotes.png was inspected; generated screenshots remain ignored. The actual sandbox UI-only preview on port 5050 was also checked at #quotes with local Next and no uncaught page errors. Hardware/live audio is not established by these UI tests.

## Full-player favorites and completed-song ratings

The expanded Now Playing controls now include a 44px heart: outlined for unselected, filled pink for a favorite, with dynamic Add/Remove from favorites labels and aria-pressed. The Play button remains centered using a non-interactive balancing spacer. No selected song disables the heart. It uses the existing /api/liked, /api/like and /api/unlike contracts (full Track JSON including thumbnail), not a parallel favorites store. Initial reads are shared, mutation requests are single flight per track and failed requests never falsely select the heart. Cards/counts and the Library list update from successful changes. Errors/success are visible inside the modal as well as the general status. These favorites retain the backend's existing global, in-memory lifetime; no persistence/session architecture is added.

A browser audio natural end opens an optional 1–5 prompt while Next/autoplay continues. Stop, skip and audio errors do not rate or prompt; stopped-source stale end events are ignored. The completed ID/title are snapshotted before navigation, so rating the finished song never rates the next song by mistake. A pending prompt ignored until the next unrated completion is replaced, not queued indefinitely. Not now dismisses without marking rated. The prompt is an accessible fixed card, with an instance inside the native player dialog when it is open so modal inertness never blocks the controls. It does not steal focus or pause playback. Mobile star targets are >=44px and remain in view, including phone landscape.

Explicit scores are saved only in this browser/origin under aysha.song-ratings.v1. Already-rated IDs skip future prompts here, including reload. No view/dwell/mood tracking, server analytics, automatic recommendation changes, account sharing or cross-device aggregation is added. Library shows the latest 20 scores and a delete-all control, plus the current song's score in the full player. The validated record holds up to 1,000 IDs (128 characters max), title snapshots (300 characters) and integer scores (1–5). Existing ratings are not evicted at capacity; new saves are refused with a message to manage storage. Denied/full/corrupt storage does not break music, but denied writes mean suppression survives only this page; clearing ratings permits prompting again. Quote preferences/music selection use separate keys and are untouched.

Limits: home mpv/Bluetooth completion has no reliable browser end notification, so automatic rating prompts currently cover browser playback only. Do not infer natural completion from a pause, stale status or elapsed timeout. Favorites remain global/server-memory, while ratings are per browser. A UI-only sandbox cannot perform live favorites mutations/audio; its existing explicit unavailable-backend responses are preserved.

Checks: 46 frontend cases plus all seven browser viewports cover favorites add/remove/Library/labels/failure/single-flight/current-track changes, rating snapshot/autoplay/modal/non-modal targets, rating skip after actual reload, dismiss/reset/capacity/storage sanitation/isolation and preservation of music source. Fixture media output and synthetic end events are not proof of real speaker/phone playback. Screenshots are ignored; the active 390px heart layout was visually inspected.

## Romantic opening letter

Each full document load opens a native modal love letter before the player: monochrome black/white, a white heart bubble, “My dearest Aysha,” and “You are loved…”. Three original two-line verses are shown for five seconds each (15 seconds total), with a short upward lyric entrance and subtle bubble breathing. The bottom says “Opening your little world of music”; its line indicates letter time only, never backend/network readiness. Existing API loads proceed independently and keep their real loading/error states. No media starts on opening/reveal. Skip and Escape reveal the player immediately; native dialog focus/inertness protects the underlying UI. No history entries or storage flags are added: full refresh shows the letter again; in-app tabs do not. No-JS/unsupported-dialog browsers fail open to the existing player. A wall-clock deadline avoids replaying 15 seconds after background timer throttling. Reduced-motion disables the movement, but keeps the same reading time and Skip control. Text and heart come from local markup/icons only; no image, video, remote font, new API or dependency.

Home now contains a heart and Library includes a small heart motif, while conventional Play/Pause/Next/volume/search/Bluetooth silhouettes and labels remain intact. Existing favorites/Quotes branding hearts and rose UI are preserved behind the monochrome opening.

Verification: 50 frontend tests (four new opening cases: 5s verse boundaries/15s deadline, cancellation cleanup, background time jump, unsupported-dialog fail-open). Seven browser sizes verify opening text, two-line inventory, monochrome background, no overflow, reachable 44px Skip, Escape, reduced-motion styles, actual natural timed reveal/verse progression at 390px, no audio start, and existing navigation/favorites/ratings/refresh flows after dismissal. Inspected the generated 390px opening screenshot. Actual Android/TalkBack reading timing remains a manual acceptance item; Skip is always available, and the letter is intentionally only three short verses plus greeting/signature.

Reading-time refinement: the two-line letter is fully opaque from the first frame; its subtle 8px entrance never fades text out. LOVE_VERSE_MS is explicitly 5000 and total/progress derive from verse count. Each pair has five seconds of full-opacity reading time, with three pairs totaling 15 seconds. Browser regression checks assert opacity 1 even during entrance.

## Pointer heart trail and tap/click bursts

Integrated the supplied interaction idea into the existing UI, not a separate demo/template: mouse-only pointermove leaves a 10px local SVG heart at most once per 90ms when not dragging/editing. A primary pointer click/touch-generated click produces eight 12–18px hearts with outward/upward scatter, a downward second leg and fade over one second. Keyboard activation, inputs, selects, textareas and contenteditable are excluded. No touchmove trail or pointerup listener, so scrolling is not decorated and one tap is not deliberately handled twice. Uses the existing project-owned heart, pink/rose in the player and white during the monochrome letter.

Three aria-hidden fixed, clipped, pointer-events:none layers cover the main UI and both native dialogs (ordinary z-index cannot paint above the browser's modal top layer). Particles are non-focusable/non-interactive; listeners are passive and never prevent controls' defaults. The entire page is capped at 32 active particle nodes/timers, evicting oldest first. animationend removes each node; a 1.1s fallback also removes it after closed modals/interrupted animations. Hidden documents clear/suppress particles. Reduced-motion disables the CSS layer and suppresses JS creation, including live preference changes. No data capture/storage/network API/audio action, framework, font, image or dependency was added. This bound is not evidence of meeting the separate 100 MB aggregate-service requirement.

Verification: 56 frontend tests cover eight-heart burst, no network/storage/audio side effects, modal host selection, mouse throttle/touch/drag/editing/keyboard exclusions, cap and animation/fallback cleanup, runtime reduced-motion/visibility cleanup, invalid coordinates and idempotent setup. Seven browser sizes pass the existing suite plus actual touch-generated eight-burst count, white intro hearts, pointer-transparent modal layers, working favorites through the full-player layer and reduced-motion suppression. Desktop checks cover mouse trail, 32-node stress cap, cleanup and live reduced-motion changes. Test counts distinguish .heart-burst from any independent mouse trail present in browser emulation. Inspected generated mobile-love-hearts.png (ignored) for the monochrome opening; this does not replace real Android motion/performance/accessibility acceptance.

## Stationary hidden-heart reveal (supersedes the hover trail)

The user selected “Pointer reveals hidden hearts”, not a big heart that follows the pointer. Hover no longer spawns particles: a deterministic 111-heart SVG grid forms one stationary large heart centered in each active viewport. An implicit heart equation selects grid positions; the existing local heart symbol renders each point. A soft radial CSS mask uncovers nearby hearts as the mouse moves. Only mask coordinates change (32ms maximum update frequency); layout and SVG contents stay fixed. No perpetual requestAnimationFrame/animation loop. A 650ms inactivity timeout starts a 900ms opacity fade. Taps reveal the same fixed field and retain the eight-heart scatter burst. Small discovery captions explain the interaction on Home and the opening. White field opacity is reduced to .22 over the letter so the fully opaque text remains readable; the main/player field uses rose.

Fields are lazy-created once per page/intro/player layer (at most 333 fixed heart uses in three SVGs), with at most one reveal timeout per layer. This is separate from the existing 32 transient burst-particle cap; do not describe all visual nodes as capped at 32. Pointer exit, document hiding and reduced-motion changes cancel pending effects; a clear resets the hover throttle, ensuring the first new move is not lost. Reduced-motion prevents field creation and hides layers/captions. Keyboard/form-editing/touch-scroll exclusions, noninteractive aria-hidden clipped layers and existing dialog coverage remain. No backend/network/storage/audio behavior or dependency changed. The original five-second verse timing is untouched.

Verification: 58 frontend tests include deterministic 111-heart count and mirrored shape, fixed markup under 100 reveals, mask-local coordinates, exact 32ms boundary/reset, single-field/timer reuse, three-context bounds and exit/visibility/preference cleanup. Seven Chromium sizes verify tap reveal/burst, white intro and reduced-motion behavior; desktop asserts fixed bounding box and unchanged SVG after moving the spotlight, idle fade to zero, burst cap and live motion-preference cleanup. Inspected both desktop-heart-reveal.png and the softened mobile-love-hearts.png (ignored). First desktop retest exposed the prior throttle swallowing the first move immediately after clear; reset-on-clear fixes this rather than weakening the test. Desktop/browser emulation is not real-phone performance or TalkBack acceptance.

## Click/tap butterfly-heart flight (supersedes stationary reveal)

User rejected the fixed reveal and explicitly requested hearts flying like butterflies from clicks/taps on PC and mobile. Removed the constellation SVG/mask, reveal timers and mouse-hover dependency. A single primary click handler (including touch-generated clicks) creates eight 18–26px local SVG hearts at the interaction point. Each heart follows an upward swaying flight (180–290px lift) over a randomized 2.2–2.8s lifetime, with a separate rapid perspective/rotate/scale flutter animation on the heart icon. They remain initially visible, then fade as they fly away. This is a heart-shaped flutter effect, not a stationary heart silhouette or actual butterfly image. White in the letter, pink/rose in the main/full player. Discovery captions now say tap/click to let love fly. No hover is required and no pointermove spawning remains.

Removed document pointerout cleanup: touch pointer exits must not kill newly launched flights. Explicit close listeners relocate in-flight hearts to the currently visible page/dialog layer, so Skip and closing the full player do not hide the effect in a closed modal. Keyboard/form-editing exclusions, pointer-transparent aria-hidden layers, reduced-motion suppression (including live changes) and hidden-page cleanup remain. Cap is again just 32 transient hearts/timers; all 333 static constellation uses and reveal timers are gone. animationend or lifetime+150ms fallback cleans each node. No APIs, storage, dependencies, backend or audio behavior changed; five-second letter verses remain unchanged.

Verification: 58 frontend cases including launch coordinates/lift/lifetime/local SVG, no side effects, native-modal relocation, cap/fallback/animation cleanup, no hover/pointerout dependency, reduced-motion/visibility/idempotence/exclusions, and flight/flutter CSS inventory. Seven Chromium sizes pass existing playback/navigation/favorites/ratings/opening regressions and touch flight checks. Those checks compare actual bounding boxes at launch and 600ms after tap (upward displacement), confirm both flight and flutter animations, dispatch touch pointerout and assert all8 survive, and check cleanup. Skip-generated hearts are verified in the visible page layer. Desktop additionally compares boxes across700ms after an actual mouse click and stress-tests cap/live reduced motion. Inspected desktop-butterfly-hearts.png; real Android/iPhone browser performance and physical-device acceptance remain pending.

## Interactive Arena/sandbox preview and coordinate-click regression

Heart animation is frontend-only and never requires the Rust/audio backend. To run the real source with truthful backend-unavailable responses: `node scripts/preview_ui.cjs` (default port5050). The server binds0.0.0.0, accepts preview hosts, serves local icons and uncached live HTML, and returns503 for unsupported audio/library APIs instead of simulating playback. A preview-only status in the banner and love-letter footer states “Hearts ready” or “Hearts off: Reduce Motion is enabled”. It updates with motion preference changes and reports a missing animation script if initialization fails. These labels are preview diagnostics only; production Rust HTML/API behavior is unchanged.

The click handler previously rejected all detail=0 events, including coordinate-bearing clicks from embedded/automation integrations. It now accepts those positioned clicks while still excluding coordinate-less keyboard activation (detail0, coordinates0/0), secondary clicks and form edits. Confirmed before/after on the actual preview: coordinate-bearing MouseEvent default detail0 created0 hearts before,8 after; ordinary real touch already created8 without a backend. This demonstrates an event-compatibility bug, not proof that the user's particular Arena client emitted that event. OS Reduce Motion and selecting a static file/screenshot rather than the live preview remain separate possible explanations until user confirmation.

The browser suite now also launches the actual preview server on an ephemeral port and loads it in a representative sandboxed iframe, with no API fixtures for that server. At phone and desktop viewport sizes it verifies touch/mouse activation while the actual search API returns503, detail0 coordinate activation, modal-close relocation and truthful live reduced-motion status/suppression. The iframe test is not an end-to-end certification of Arena's own proxy/client. It shares one browser context with separate pages to avoid test-browser context-lifecycle hangs; page cleanup and forced HTTP connection cleanup are explicit. Existing seven fixture viewport checks remain. Total frontend59/Python17 pass; no backend installation needed for hearts.

## Explicit always-on heart preference (supersedes reduced-motion suppression)

The user confirmed the preview displayed “Hearts off: Reduce Motion is enabled on this browser” and explicitly requested hearts always on. Heart effects now ignore prefers-reduced-motion in JS and CSS. Removed the matchMedia gate/change listener, reduced-motion layer hiding and discovery-caption hiding. Narrow, specific !important heart-flight and heart-flutter rules restore only those two animations within the existing reduced-motion media query; letter entrance, bubble breathing, scrolling and other UI animations still obey reduced motion. No continuous auto-spawning: hearts still launch on click/tap, are bounded at32, clean themselves up and do no work in hidden tabs. This is an explicitly requested product preference, not a general removal of accessibility handling across the UI.

Preview status now says “Hearts always on — tap/click the page to make them fly”, independent of OS motion setting. Both active sandbox preview launchers were restarted so this cached diagnostic text updates as well as the live source. Browser regression checks start the actual iframe preview with Reduce Motion enabled, verify ready/always-on status, 8 hearts and both animation names, and measure upward motion after touch/mouse activation while backend search returns503. The360px reduced-motion fixture likewise verifies visible flight, while bubble/letter animations remain disabled.59 Node,17 Python and all nine browser checks (two real-preview iframe inputs plus seven fixture sizes) pass locally.

## Combined red-heart spotlight + butterfly flights

User asks to restore the pointer-revealed red hearts under the dark background while keeping the butterfly interaction. These are now additive, not replacements: mouse hover reveals a fixed red-heart tile pattern under a soft radial mask across the viewport; click/tap reveals nearby tiles and launches the existing eight fluttering hearts. The carpet covers the screen instead of being confined to one large heart silhouette. Only spotlight coordinates move; the tiles stay fixed. Intro tiles are red at lower opacity to preserve letter readability; flying intro hearts remain white.

The fixed field is a single SVG pattern referencing one local heart symbol plus a full-size rect, lazy-created in each of the three existing aria-hidden/noninteractive page/dialog layers. It does not generate a viewport-sized DOM grid. Maximum3 field wrappers/patterns and3 reveal timers, separately from max32 transient butterfly nodes/timers. Mouse updates are throttled32ms; idle650ms starts a .9s fade. Form editing, drag and touch scrolling do not reveal; taps work without hover. Mouse exit fades only the field and never cancels flights; touch pointerout clears neither. Closing a modal clears its field and preserves/reparents flights. Hidden-page cleanup remains; always-on heart exceptions also restore field transitions under Reduce Motion, while other UI still respects that setting. No backend, dependency, audio/storage/analytics behavior changed.

62 frontend cases and17 Python checks pass. Nine browser checks pass (two actual no-backend iframe inputs plus seven fixture sizes): preserve click/tap upward motion, modal/control behavior and always-on animations; verify red color/radial mask/transitions in the actual reduced-motion preview; desktop verifies hover creates no butterflies, stationary field geometry/markup with moving mask, visible opacity and eventual idle fade. Unit checks cover tile reuse/one local use, throttle, bounded field/timer reuse, context bounds, exits and hidden-page cleanup. Inspected desktop-red-heart-spotlight.png (ignored). Existing live previews read the edited source at refresh; no host installation required for this sandbox effect.

## Compact heart-shaped reveal with little red butterfly morphs

User requested revealed hearts become small cute red butterflies, a smaller/denser reveal and a love-symbol footprint instead of a circle. The mask is now the original heart silhouette encoded as a local SVG CSS mask (112x104px, no radial/circular mask). Heart tiles shrink from14px in40px cells to8px in20px cells. The field remains fixed across the viewport while this small heart-shaped window follows the pointer. No downloaded image/dependency.

Mouse reveal launches two12–16px red butterflies at actual20px tile centres, at most once per240ms during mouse movement. Their first8px-scale appearance aligns with the revealed hearts; a short CSS crossfade changes the heart into the new local butterfly symbol, then it flutters/lifts/fades using existing bounded flight logic. No idle emitter or extra JS morph timers. Click/tap still launches8 particles, now including2 red butterfly morphs and6 existing hearts; pre-click hover particles are removed to avoid doubling the mouse-click burst. Intro butterflies are red while the other flying intro hearts stay white. All32-particle/context/timer/visibility/control protections remain; morphology explicitly participates in the user-requested always-on motion exception.

The local sprite now contains19 symbols with the new project-owned butterfly. Child morph animationend bubbles must not end the whole flight: removal is now filtered to the particle's own animationend target.65 frontend cases and17 Python checks pass; nine browser checks preserve actual no-backend mouse/touch/Reduce Motion behavior, verify compact data-SVG mask dimensions and2 red butterfly morphs, and retain flight displacement/navigation/favorites/ratings/reading timing checks. New unit cases verify tile-centre origins,12–16px/red styling,240ms emission bound, click-burst deduplication, morph-child completion safety and heart-not-circle geometry. Inspected desktop-compact-heart-butterflies.png (ignored). Physical device performance remains pending.

## Single sandbox preview port

Use only port5050 for the interactive sandbox UI. The duplicate5051 debug preview was stopped at the user's request; it was the same source, not a separate feature build. The helper now defaults to5050, serving all current compact heart-shaped reveals, red butterfly morphs and click/tap flights. Historical entries mentioning two running ports describe earlier verification, not the current preview setup.

## Fuller clicks: same tiny red butterflies, no larger hearts

User asks clicks to release more of the same cute butterflies from revealed hearts. Click/tap now emits12 red12–16px heart-to-butterfly morphs (previously2 butterflies+6 larger hearts). Lift-off comes from6 nearby20px tile centres inside/around the compact reveal, instead of all particles sharing the pointer coordinate. Hover stays2 per240ms. Pre-click hover particles are removed to avoid duplicate bursts. The existing112x104 heart mask, always-on behavior, cleanup and max32 concurrent particles remain unchanged. Only5050 preview remains active;5051 is not restarted.

Verification:66 frontend cases,17 Python checks and nine browser checks pass locally. Tests require12 all-red butterfly morphs (zero larger heart particles),6 tile-centre origin positions, preserved touch/mouse/iframe/no-backend/Reduce Motion flight behavior and rapid-click32-particle cap with one reveal timeout. Inspected mobile-love-hearts.png for the red butterflies. No dependency/API/audio/storage change; physical-device acceptance still pending.

## White A / Y / S / H lettering in click bursts

Every click/tap adds the exact requested four letters A,Y,S,H in white, with one tiny white local SVG heart per letter, alongside the12 small red butterflies (16 particles total per click). Letters float/sway/fade through the same existing flight lifecycle, do not affect hover emission, and are decorative/aria-hidden via their effect layer. Native modal relocation, pointer transparency, always-on user preference, hidden cleanup, one5050 preview and compact reveal remain unchanged. Letters and butterflies share the existing32-particle limit rather than adding another unbounded system.

Font: unmodified Short Stack Latin400 normal WOFF2 from @fontsource/short-stack5.3.0, bundled inline as a CSS data URL (about24KiB binary) so production's existing embedded HTML serves it without a new route, font CDN or runtime npm dependency. License/copyright: docs/licenses/short-stack-OFL.txt (Sorkin Type Co, SIL OFL1.1). Only these letter particles use the font; the rest of the approved UI typography is unchanged. The glyph inventory is fixed, not user-supplied HTML.

Verification:68 frontend tests,17 Python checks and nine browser checks pass locally. Test exact letter inventory, white color/heart count,12 butterflies+4 letters, local WOFF2/license, shared rapid-click cap, no letters on hover and existing cleanup. Browser loads the embedded font explicitly and checks white letters/heart icons in the actual no-backend iframe and opening; prior input/motion/navigation/ratings checks remain. Burst font loading was moved before launch and fleeting metrics consolidated into one round trip after the first test attempt consumed too much of the flight lifetime while awaiting the font. This preserves the12-butterfly alive/movement checks, without extending the production animation to satisfy a test. Inspected desktop-butterflies-white-letters.png (ignored).

## Full A / Y / S / H / A and softer, smaller lettering

Corrected the click/tap inventory to the full requested A,Y,S,H,A (including the final A), with5 white heart accents. Reduced glyph size21→18px and heart accents8→7px; re-centred all5 letter origins at16px spacing. Replaced Short Stack with the softer rounded handwritten Delius Latin400, unmodified WOFF2 from @fontsource/delius5.3.0 (about17KiB binary), embedded directly in CSS. New license/copyright: docs/licenses/delius-OFL.txt (Natalia Raices, SIL OFL1.1). Earlier Short Stack description/license remains historical; its font is no longer loaded. No CDN, runtime dependency or backend route added.

A click is now12 butterflies+5 letters=17 particles, sharing the unchanged32-particle cap. Hover/compact reveal/always-on behavior/only5050 remain unchanged.68 frontend tests and17 Python checks pass; nine browser checks verify exact AYSHA,5 white hearts, actual18px Delius/font load, reduced-motion/no-backend input and prior UI regressions. Actual iframe short-lived animation metrics now use one round trip with font loading before the burst, matching the earlier main-suite sampling improvement; no production lifetime changes. Inspected refreshed desktop-butterflies-white-letters.png.

Visible effect instructions removed at user request: no “Reveal a tiny heart · let red butterflies fly” caption on Home or opening. The effects themselves remain unchanged; earlier discovery-caption notes describe historical UI only.
