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
