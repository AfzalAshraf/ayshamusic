---
name: networking-self-hosting
description: Plan and verify private remote access to the local Rust player using an authenticated tunnel without exposing shared playback controls.
---

# Networking & Self-Hosting department skill

This is a repository workflow, not an installed connector or deployed tunnel. Architecture approval comes before deployment or new application code. Use one active department per response and retain the build/error ledger.

## Scope

Keep the Rust server on the existing Ubuntu host, port 5050 and systemd. A tunnel is a network connector, not a replacement cloud music backend. Android Chromium is the primary remote browser. The Linux host must remain powered on. Decide independent playback versus synchronized rooms explicitly before implementation.

## Readiness rating

Begin at 1/5 for this project: documented design only, no tunnel, Access policy or off-LAN playback evidence. Reach 5/5 only after the launch checks below pass. Never award points for merely writing configuration examples.

## Procedure

1. Confirm domain ownership/Cloudflare zone, who may log in, device count, simultaneous-playback semantics, Bluetooth permissions, persistence needs and RAM-budget scope.
2. Inspect the current API for global mutable state and unauthenticated endpoints. A tunnel alone does not make them safe or isolate users.
3. Research current official Tunnel, Access and applicable media-delivery terms. Separate CDN terms from Tunnel-specific permission; neither assume all audio is prohibited nor promise unlimited free audio relay. Resolve suitability before using it for sustained music.
4. Create the deny-by-default Access application and narrow identity allow policy before enabling a public hostname route. Protect the entire hostname, including audio and all API paths; never bypass /api/stream to work around authentication.
5. Prefer a named, persistent tunnel managed as a separate systemd service. Install using current official Ubuntu x86_64 instructions, not curl-pipe-shell from unverified blogs.
6. Keep tunnel credentials out of chat, Git, screenshots, logs and shell history where feasible. Use a protected local token/credential file supported by the installed connector. Rotate any exposed credential.
7. Route the selected hostname to http://127.0.0.1:5050 only when cloudflared is on the same host. Do not expose an origin port or enable new router port forwarding. If keeping the existing LAN bind, separately restrict direct origin access to prevent bypassing Access. Do not change SSH firewall rules blindly.
8. Enable Access-token validation in the connector or verify signed Access JWTs in the app. Never trust a browser-supplied email header. App session IDs identify playback sessions; they do not authenticate users.
9. Use same-origin relative URLs and browser cookies for both the UI and audio. Test an expired login session instead of adding credentials to stream query strings. Do not put service tokens in frontend code.
10. Isolate browser-device playback state and child processes. Bluetooth is one physical shared output and needs an explicit administrator-controlled mode, not automatic selection for every remote phone.
11. Bound concurrent downloaders, request work, queues and session lifetime. Reject excess work without breaking another listener. Measure music-server, cloudflared, yt-dlp, mpv and JS-runtime memory separately; do not claim all fit within 20 MB.
12. Test Android Chromium on cellular data, screen lock/background, reconnect, Access reauthentication and two real devices. Authentication, process cleanup, buffering, seeking limitations and mobile data use are acceptance gates.
13. Disable only the public route if needed for rollback, preserving local playback. Verify two independent services recover after SSH logout and reboot.

## Launch gates for 5/5

- Confirmed domain/plan suitability and protected hostname; logged-out and unauthorized users cannot access UI/API/audio.
- No direct internet origin route or Access bypass; loopback/LAN policy explicitly approved.
- Two phones can play different tracks, skip, pause and change volume without altering each other, if independent mode is chosen.
- Bluetooth controls are limited to the approved administrator role; remote browser playback does not stop mpv.
- Session and concurrency limits, logout/revocation behavior, resource use and rate limits are tested.
- Android Chromium background/lock-screen behavior is measured rather than guaranteed.
- cloudflared and the Rust service survive SSH logout, recover after reboot and have a documented rollback.
- Credentials never enter Git or public logs.

## Sources to recheck

- https://developers.cloudflare.com/tunnel/
- https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/self-hosted-public-app/
- https://www.cloudflare.com/service-specific-terms-application-services/
- https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/get-started/create-remote-tunnel-api/
