# Remote Android listening and simultaneous-device architecture

Status: proposal awaiting user decisions. No application changes or tunnel deployment in this turn. User reports the current player plays successfully; this does not establish the output device, two-device isolation or systemd boot recovery.

## Current source findings

- src/main.rs has exactly one AppStateInner.player: PlayerState. Track, history, queue, volume, likes and playback mode are shared by every request/device.
- The page polls /api/status and changes its track information from that shared state. Two browsers can therefore overwrite each other's displayed track/control state.
- start_track kills mpv using a global process pattern even for browser playback. Browser sessions can interfere with the physical speaker.
- The frontend automatically selects Bluetooth when the host reports a connected speaker. A remote phone may therefore control the home speaker instead of hearing its own audio.
- Each /api/stream request can start its own downloader. There is no global/per-session stream admission limit. No device-session identity is checked.
- The API binds to 0.0.0.0:5050, has permissive CORS and no application authentication. Successful single-device playback is not sufficient evidence for public launch.
- Seek/range support, actual Android lock-screen behavior, multiple listeners and total memory remain unverified. Persistence remains in-memory.

## Proposed department ratings

Ratings are provisional evidence-based readiness for the requested local-plus-remote application, not guarantees or a certification. This table supersedes historical initial scores only as a current assessment; old ledger entries remain an audit trail.

| Department | Rating | Remaining acceptance gate |
| --- | --- | --- |
| Product & Architecture | 2/5 | Decide independent vs synchronized playback, login audience, domain, concurrency, memory and persistence. |
| UI/UX Design & Frontend | 3/5 | Tested control/icon regressions and reported playback; Android touch, background, reauthentication and explicit output/device isolation still unverified. |
| Backend & Database | 2/5 | Global mutable state and process ownership conflict with multiple users; auth/session isolation and resource admission needed. |
| QA, Security & Debugging | 3/5 | Prior Linux build/tests and reported playback; two-device/security/load, hardware and reboot acceptance missing. |
| Networking & Self-Hosting | 1/5 | Skill and plan only; no domain, connector, Access policy or cellular test configured. |

A 5/5 score requires observed acceptance evidence, not simply more source files.

## Proposed user flows (approval needed)

### Recommended default: independent browser listeners

1. An allowed person opens the chosen HTTPS hostname on Android Chromium and authenticates through Access.
2. The app creates an opaque, unpredictable, server-issued device session using a same-origin Secure, HttpOnly, SameSite cookie. The validated Access identity authorizes access; the app session separates device playback.
3. Each device chooses tracks and has its own current track, queue, history, volume and playback errors. Likes may be per identity or per device, to be decided explicitly.
4. Play starts audio on that phone by default. It does not automatically select the home JBL just because the speaker is connected.
5. A pause/skip/stop/reconnect on phone A does not mutate phone B or stop home-speaker playback.
6. A stream request belongs to one authorized device session; it is bounded and released on cancellation. An excess request is rejected without terminating another user's valid stream.
7. A separate authorized administrator may select the single home-speaker output. Bluetooth has one owner/controller at a time, not unlimited independent physical outputs.

### Alternative: synchronized listening room

If the user wants all devices to hear the same session, use a separately designed shared room with a controller, authoritative playback clock, updates and drift correction. Multiple HTTP audio requests by themselves do not synchronize playback. Do not silently implement a shared queue as if it were independent playback, or promise synchronized audio without timing tests.

## Proposed modules and data boundaries

Keep existing handlers stable where feasible, moving state selection and ownership out of global mutable playback:

```text
src/main.rs           router/startup and shared dependencies
src/sessions.rs       device-session creation, authorization association, expiry and bounded storage
src/playback.rs       pure per-session queue/history transitions
src/media.rs          owned stream/download lifecycle and concurrency admission
src/speaker.rs        single administrator-controlled mpv/Bluetooth output
src/index.html        Android-first controls and explicit playback destination
src/icons.svg        embedded icon inventory
skills/networking-self-hosting/SKILL.md
```

Proposed model, not yet implemented:

- AppState: HTTP client, bounded SessionStore, SpeakerController and shared resource limits.
- Session: server-issued ID, optional validated identity association, current track, queue, history, playback status and last activity.
- StreamLease: owning session, request generation, downloader handle, cancellation and permit. Never infer authorization from a query-string session ID.
- SpeakerController: permitted admin identity/role, current owner/generation, mpv child handle and IPC state. Stop only the owned process; remove global pkill patterns.
- Concurrency policy: one current stream per device plus a global configured cap; initial cap chosen after device-count and memory answers. Handle Android retries without orphan children or accidentally killing a newer stream.
- State updates/radio fetches must be scoped to the requesting session and checked against its current track generation before applying late results.

No database or package is selected until persistence requirements and the resource budget are agreed. Ephemeral sessions and persistent user likes are separate decisions.

## Tunnel architecture

```text
Android Chromium (HTTPS)
  -> Cloudflare Access policy and login
  -> named Cloudflare Tunnel
  -> cloudflared on the existing Ubuntu server
  -> http://127.0.0.1:5050
  -> local Rust API -> owned yt-dlp stream -> phone audio
```

The local server remains the application/audio origin. The tunnel uses outbound connectivity; it is not a hosted replacement for Rust and does not keep a powered-off machine available. Official overview: [2](https://developers.cloudflare.com/tunnel/).

The Access policy must protect the complete hostname before a route is enabled. A public tunnel hostname without Access is public, not automatically private. Cloudflare recommends creating the Access application before the route and validating Access tokens in the connector or origin: [1](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/self-hosted-public-app/).

## Safe setup order

1. Confirm the domain/subdomain, existing Cloudflare account/zone, permitted login audience and media-delivery suitability. Do not send tokens/passwords to the coding agent.
2. Implement/test per-device isolation and explicit speaker output before promoting this application for simultaneous public use.
3. Keep the working Rust systemd service and verify the local API. Review origin binding: loopback-only is preferable for a same-host connector, but do not remove approved direct LAN access without asking. If LAN access stays, enforce its intended access policy separately.
4. Create a deny-by-default Access application for the selected hostname with a narrow allow list (for example selected family email addresses using the configured login method).
5. Create a named tunnel in the dashboard, select Linux amd64 for the confirmed Ubuntu x86_64 host, install the official cloudflared package, and register it as its own systemd service. Use the current official dashboard instructions and protect the generated token locally. Remotely-managed tunnel installation is documented here: [2](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/get-started/create-remote-tunnel-api/).
6. Only after the security/isolation gates pass, map the chosen public hostname to HTTP 127.0.0.1:5050. Enable Protect with Access/token validation as supported by the configured connector. Do not add router port forwarding.
7. Protect /api/stream with the same Access session as the page; do not put secrets in media URLs or create an unauthenticated audio exception. Keep page/API/audio same-origin with relative browser URLs.
8. Do not cache personalized API responses or audio at the edge. Cache bypass is a correctness/privacy measure, not evidence that media delivery is allowed by the provider's terms.
9. Verify from Android cellular data, not only the home Wi-Fi. Test signed-out denial, allowed login, unauthorized denial, stream startup, next/previous, background/screen lock and expiry/relogin.
10. Test two phones playing different tracks at once; pause/skip/stop/volume on each, close one tab and confirm the other continues. Verify downloaders and memory recover after disconnect.
11. Verify both systemd services after SSH logout and a planned reboot. If rollout fails, disable the public route without disrupting local music.

## Important Cloudflare media caveat

Current Cloudflare CDN service-specific terms reserve restrictions for video and a disproportionate percentage of audio/large-file delivery without the appropriate paid services. That is a CDN-specific clause; it is not by itself a blanket determination about every Tunnel configuration. Do not promise unlimited free music streaming or assume cache bypass removes the concern. Confirm the intended plan/configuration with Cloudflare before relying on sustained audio relay. Official terms: [1](https://www.cloudflare.com/service-specific-terms-application-services/).

If that use is not suitable, choose an approved private-network access path or media-capable delivery provider. A custom domain is not a reason to bypass provider terms. Only stream material the user is permitted to access/share; technical access does not grant redistribution rights.

## Android Chromium acceptance criteria

- First playback works from a user tap, with visible loading/error/retry state; autoplay restrictions are handled honestly.
- All primary actions have accessible labels and usable touch targets; icons are locally embedded.
- Media Session play/pause/next/previous reflect the actual selected device session.
- Screen lock/background playback is tested on the actual Android/Chromium build and battery settings; browser/OS process suspension is not promised away by a service worker.
- Do not advertise seeking until byte-range behavior is implemented/tested. The current sequential proxy advertises no ranges.
- If adding PWA manifest/service worker support, serve it from the Rust router and never cache /api/* or audio in the service worker. A registered but unserved worker does not make the app an offline PWA.
- Access expiry, network changes between Wi-Fi/cellular, closing/reopening the page and browser retries do not corrupt another session.
- Polling/backoff and related searches are bounded per active device so multi-device use does not multiply uncontrolled work.

## Capacity and RAM

The 20 MB constraint needs clarification before a concurrency promise. Independently playing devices generally need multiple downloader streams and may launch JS-runtime processes; cloudflared is an additional daemon. Report peak RSS separately for Rust, downloader/JS processes, mpv and tunnel, plus total. Home upload use scales with the sum of active streams and overhead (for illustration, three 160 kbps streams require at least 480 kbps of audio payload). Buffering may burst above media bitrate. No fixed device capacity is promised without load measurements.

## Decisions requested

1. Independent tracks per device, or one shared synchronized room?
2. Which domain/subdomain is available, and is the zone already in Cloudflare? Must access be private to selected people or public?
3. Maximum simultaneous listeners, and does the 20 MB cap include all child/tunnel processes or only Rust?
4. May remote users control the home JBL, and should likes/queue/history survive restart?

After these are answered and the design is approved, the Backend & Database department implements session/output isolation; QA verifies it; Networking & Self-Hosting deploys the protected route; UI/UX validates the actual Android experience. One department is active per response, with failures returned to QA before further feature work.
