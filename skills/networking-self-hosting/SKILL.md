---
name: networking-self-hosting
description: Senior-level, evidence-driven workflow for secure remote access, origin protection, Linux service operation, media delivery, diagnostics and rollback for the local Rust music player.
---

# Networking & Self-Hosting — senior engineering playbook

## 1. Mandate and authority

Make the existing local music service safely reachable and operable, not merely reachable. Own network trust boundaries, connector operation, deployment evidence and recovery. This is a repository skill, not an installed service, an authentication implementation, a professional certification or permission to deploy.

Use one active department per response. Follow [the main upgrade contract](../main-upgrade/SKILL.md), [the approved baseline](../../docs/UPGRADE_BASELINE.md) and the latest [QA ledger](../../docs/LOCAL_QA_LEDGER.md). Architecture approval precedes changed authentication, session ownership, public exposure or maintenance behavior. A reported failure goes through reproduction and diagnosis before more features.

**Never deploy merely because this skill exists.** Separate observed facts, source findings, hypotheses, recommendations and unperformed tests. Unknown is not passed. Do not describe a configured limit, green dashboard or passing fixture as production acceptance.

## 2. Non-negotiable project contract

- Keep production on the user's Linux/systemd host, with Rust `music-server` on port **5050**. A tunnel is an outbound connector, not replacement hosting; the host must remain powered on.
- The Arena sandbox is not that host. Its one interactive UI-only preview also uses **5050**; do not recreate the duplicate 5051 preview. Preview servers bind `0.0.0.0` and accept the preview host. A same-host production tunnel can use loopback, but production LAN-binding changes require approval. Never apply the production loopback recommendation to break the sandbox preview.
- Preserve the approved Android-first romantic UI, exact dedication, embedded assets, Back/history, one audio element and explicit output labels. Do not rewrite frontend effects during a networking task. Heart effects are browser-only; unavailable audio APIs do not explain disabled pointer animations.
- Maximum target: **100,000,000 bytes (100 MB)** for the agreed concurrent workload. Scope, device count, inclusion of `cloudflared`/helpers/shared audio services and evidence must be settled before a compliance claim. Rust-only RSS is not the service budget.
- Existing global playback state, permissive CORS, automatic speaker selection and process ownership are public-launch blockers until inspected and resolved for the approved semantics. Do not infer authentication or isolation from a private-looking URL.
- Independent browser playback versus a synchronized room is a product decision. One home Bluetooth speaker remains one explicitly authorized shared output. Do not silently create accounts, shared favorites, cross-device sync or a new persistence model.
- Nighttime updates require approved scope, time, timezone, idle policy and recovery. No arbitrary unattended branch-head updates or silent restart during listening.
- No passwords, tokens, cookies, signed media URLs or private keys in chat, Git, screenshots, process arguments or diagnostic bundles. No new router forwarding or SSH/firewall changes without an explicit plan and recovery path.

## 3. Intake: obtain decisions once, then record them

Before implementation, resolve these consequential unknowns; do not repeatedly ask settled questions:

| Decision | Required answer / consequence |
| --- | --- |
| Ownership and audience | Who owns the domain/zone, who may log in, and who administers the speaker? Do not invent hostnames or identities. |
| Playback semantics | Independent devices or synchronized room; maximum simultaneous devices; speaker ownership and interruption policy. |
| Access path | LAN only, protected Internet hostname, or private-network clients; whether approved direct LAN access must remain. |
| Resources | Whole-stack 100 MB accounting boundary, acceptable cold-start delay, disk-cache permission/cap, supported workload. |
| Privacy/persistence | Session expiry, whether favorites belong to a device/person/shared library, deletion and backup expectations. |
| Service operation | Host OS/architecture/systemd version, existing units/drop-ins, maintenance window/timezone and rollback access. |
| Provider suitability | Current applicable Tunnel/CDN/account terms and media-delivery plan suitability; unresolved means no sustained-audio launch claim. |

Produce a short decision record with accepted choices, unresolved blockers, smallest next change, acceptance tests and rollback. Use [the session/tunnel proposal](../../docs/REMOTE_MULTI_DEVICE_PLAN.md), [the memory plan](../../docs/MEMORY_BUDGET_PLAN.md) and [local service guide](../../docs/LOCAL_SERVICE.md); do not duplicate conflicting architecture.

## 4. Inspect before prescribing

Record repository revision, working-tree changes, actual installed executable path, service account, versions, listeners, current routing and baseline playback. Preserve unrelated work. Confirm whether evidence is from sandbox, CI or target host.

Safe preflight examples on the **intended machine**, not a claim these were run:

```bash
git status --short
git rev-parse HEAD
systemctl --version
ss -lntp
systemctl show aysha-music.service --property=ActiveState,SubState,MainPID,FragmentPath,ControlGroup,MemoryCurrent,MemoryPeak
curl --silent --show-error --max-time 5 --output /dev/null --write-out '%{http_code} %{content_type}\n' http://127.0.0.1:5050/
```

Check `cloudflared --version` only where installed; absence is not a tunnel failure. Confirm flags against that binary's help and the matching official documentation. Inspect units/drop-ins locally before editing, but redact secrets before sharing outputs: full unit dumps, process listings, headers, environment files and debug logs may disclose credentials. A loopback 200 confirms only that request, not authenticated remote audio.

Do not blindly apply instructions for another Ubuntu/systemd release, architecture, container or service user. Capture a baseline before changing firewall, transport, unit sandboxing or package version. Change one layer at a time.

## 5. Design and enforce every trust boundary

Approved target topology, not implemented state:

```text
allowed phone/browser over HTTPS
  -> deny-by-default Access identity policy
  -> named Cloudflare Tunnel / cloudflared
  -> protected local origin :5050
  -> validated identity + authorized device session
  -> bounded stream jobs / explicitly authorized speaker controller
```

1. **Edge access:** create a narrow identity allow policy before publishing the hostname. Cover the UI, icons, every API, audio/Range requests and any future transport. Audit more-specific paths, wildcards, bypass/service policies and alternate hostnames; do not exempt audio to fix login problems. Access setup and token validation: [1](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/self-hosted-public-app/).
2. **Connector validation:** enable supported Protect with Access or implement origin-side signed-token validation. If validating in Rust, verify signature with trusted keys, allowed algorithm, issuer, intended application audience, expiry/not-before and bounded clock skew. Plan key rotation and bounded key caching; malformed/unknown/unverifiable tokens fail closed. Never trust an email/forwarded-identity header by itself. Token fields: [4](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/authorization-cookie/application-token/).
3. **Origin bypass:** connector validation only protects requests traversing that connector. An exposed direct LAN/public origin can bypass it. Use an approved loopback/firewall boundary or app-side authentication on every allowed path. Preserve explicitly approved LAN access through a designed auth policy, not an accidental exception. Check both IPv4 and IPv6 and existing forwarding rules. Do not declare a private origin from DNS alone.
4. **Authorization/session ownership:** validated identity is not permission to control every device or the home speaker. Session IDs are not authentication. Handoff per-device state and process ownership changes to Backend after design approval; require cross-device negative tests before independent-listening claims.
5. **Browser request safety:** CORS is not authentication or CSRF protection. Restrict allowed origins; cookie-authenticated mutations need the agreed CSRF defense and Origin/Referer validation strategy, not wildcard CORS or SameSite alone. Missing/null Origin is not proof of trust. Normal authenticated read/media requests may legitimately lack Origin; do not blindly break them. Reject state-changing GET designs. Guidance: [1](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html) and [1](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS).
6. **Upstream media boundary:** validate supported URLs/redirect destinations and bound expensive requests; do not create an arbitrary URL proxy or permit internal-address fetches. Network access must not bypass job/session/resource ownership.

If a boundary remains open, report the blocker and stop public rollout. Do not weaken checks to make a demonstration pass.

## 6. Browser/audio delivery and authentication

- Keep UI/API/audio URLs same-origin and relative. Browser-facing localhost is the listener's machine, not the remote server. Native audio elements cannot be assumed to attach a custom service-token header; use the approved browser cookie/auth flow, never secrets in query strings or frontend code. Cookie/credential behavior: [2](https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API/Using_Fetch).
- Test authenticated first play, reload, expired login before a new stream, expired/revoked login during an existing stream, denied users and recovery. Observe ongoing-stream revocation behavior; do not promise instantaneous disconnection unless implemented and measured.
- Verify actual content type and bounded response bytes. A 200 login HTML page is not playable audio; a redirect is not necessarily a decoder failure. Do not fix this by relabeling HTML/JSON as audio.
- Verify HEAD, finite/suffix/unsatisfiable Range behavior only where the delivery design supports it. A server may ignore Range and return 200; do not advertise seeking or forge 206. For supported partial responses verify Content-Range, body length and 416 handling. Reference: [1](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Range_requests).
- Distinguish connect timeouts, header wait, first playable bytes and long-running stream idle behavior. Do not apply one short whole-request timeout to kill healthy songs. Slow clients need backpressure and cancellation, not whole-song RAM buffering.
- Authenticated APIs/personal data must not become publicly cacheable. Inspect actual edge/cache rules and cross-identity behavior. Audio cache policy needs an approved privacy/storage/media-rights design. No offline/PWA claim without real served/tested worker behavior.
- Test actual Android Chromium and iPhone Safari where claimed: first gesture, lock/background, resume, Wi-Fi/cellular change, weak network and seeking. Tunnel success does not resolve the existing iPhone reset or certify a native decoder.

## 7. Credentials, connector and Linux operation

### Secrets and installation

Use the provider's official package/source appropriate to the host, record version/provenance and inspect supported flags before installation. Prefer a named managed tunnel for persistent access. Do not launch a public quick tunnel around missing auth gates.

For a supported remotely-managed connector, prefer a protected token file (`--token-file`) over a literal command-line token. This flag is version-dependent; verify installed support first. File ownership/mode must let only the intended account/root read it. Run-parameter reference: [2](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/configure-tunnels/run-parameters/).

Consider systemd `LoadCredential=` only when supported and tested on that host, with the connector reading the credential file rather than exposing its content. A protected on-disk file is a fallback, not a reason to paste a token into a unit. Environment variables are not the preferred secret store. Official implementation reference (reviewed credential/privilege/home-protection entries): https://github.com/systemd/systemd/blob/main/man/systemd.exec.xml . Treat upstream main as reference, not proof that an installed stable version supports a directive.

Rotate exposed credentials, inspect redacted logs/backups/history for the leak, and verify the old credential no longer works. Rotation may affect active streams: plan the maintenance and recovery. Never ask the user to send the credential.

### Connector transport

Check DNS and outbound connectivity before modifying transport. Cloudflare documents port 7844 over UDP for QUIC and TCP for HTTP/2; recheck current destinations rather than hardcoding an old IP list. `auto` can fall back to HTTP/2 when UDP cannot connect. A forced protocol change needs a measured reason and retest, not an inbound firewall opening. Sources: [1](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/configure-tunnels/tunnel-with-firewall/) and [2](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/configure-tunnels/run-parameters/).

Keep metrics/diagnostic listeners loopback-only or otherwise explicitly protected. Do not confuse an internal metrics port with another public app preview: the sandbox still has only 5050.

### Service lifecycle and hardening

Run `cloudflared` separately from the Rust application with least privilege and explicit ownership. Do not start another foreground music-server next to systemd. Validate candidate unit syntax/paths and actual service account before daemon-reload/restart, and do not run privileged changes without approval.

- Match Type to real daemon behavior; do not use notify/watchdogs unless the process actually implements readiness/watchdog notification.
- Evaluate Restart=on-failure, backoff and StartLimitIntervalSec/StartLimitBurst together. A restart loop that interrupts everyone is a failure, not resilience. Official behavior: [1](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html).
- Network ordering is not application readiness. Test connector retry/recovery, DNS loss and delayed origin startup; do not restart the whole audio service to recover an unrelated connector issue.
- Apply NoNewPrivileges and filesystem/process restrictions incrementally with regression evidence. This app uses binaries under `/home/afine`, and speaker operation may depend on `/run/user`, PipeWire, D-Bus, Bluetooth and audio permissions. Blind ProtectHome/PrivateNetwork/identity changes can break it. Do not copy a generic hardened unit and call it secure.
- Keep writable state/cache/log paths explicit and bounded. Review capabilities and permissions without disabling protections broadly to suppress an error.
- Confirm SSH logout/reboot recovery on the target host during an approved window. A green CI build or `active` state is not that test.

## 8. Resource and availability engineering

Use [the memory acceptance plan](../../docs/MEMORY_BUDGET_PLAN.md). Choose workload and accounting before setting ceilings. Capture cgroup current/peak/events, child processes, loaded and idle samples, cancellations and long-duration growth. Record extraction, Node, mpv, Rust and connector contribution; assess shared audio services explicitly.

If the 100 MB budget includes separate services, use an agreed accounting parent/slice or an honest aggregate method without double counting. Separate cgroups do not make their memory free. Sampled RSS is not a guaranteed aggregate peak. No hidden swap/helper exclusion or unmeasured unlimited-listener promise.

Bound extraction/search/prefetch concurrency, queued work, timeouts and retries together. Share approved in-flight work where safe. A networking configuration cannot fix global downloader/process ownership; route that implementation to Backend after approval.

MemoryHigh provides pressure/throttling; MemoryMax is a hard backstop that can invoke OOM handling. Neither alone proves continuous playback within budget. Test effective parent limits and OOM events; do not select an arbitrary cap or claim an OOM/restart is optimization. Official resource control: [1](https://www.freedesktop.org/software/systemd/man/latest/systemd.resource-control.html).

Availability also includes host power/sleep, disk headroom, DNS, identity provider, home uplink and connector recovery. A connector replica does not make one origin host highly available or synchronize playback. Avoid extra infrastructure without a measured need and approved resource cost.

## 9. Diagnose by layer, with bounded evidence

A Healthy tunnel only confirms connector-to-Cloudflare connectivity, not internal-service reachability: [3](https://developers.cloudflare.com/tunnel/monitoring/). Do not make random TLS/transport changes on a dashboard-status assumption.

| Symptom | First discriminator | Safe next action / prohibited shortcut |
| --- | --- | --- |
| Hostname fails to resolve | DNS answers from affected network, approved zone/hostname | Correct intended record/route; do not expose origin as a workaround. |
| Connector disconnected | Local process/version, sanitized transport/DNS errors, egress | Check required outbound path; do not open inbound 5050/router forwarding. |
| Tunnel healthy but app unreachable/502 | Same-host origin response and configured Service URL | Separate wrong port/stopped app/TLS mismatch; do not declare provider failure without local evidence. |
| Login loop or denied request | Access policy, audience/session expiry, cookie context, response chain | Fix approved auth flow; never add a bypass or put service tokens in browser URLs. |
| UI works, audio fails | Response status/type, auth redirects, Range behavior and media error code | Reproduce one authorized bounded stream; no fabricated success or MIME-only fix. |
| Cross-device interruption | Session IDs, generations and owned child process events | Preserve evidence and hand off to Backend; not a tunnel-protocol problem. |
| Stream reset/stall | Origin vs remote trace, idle timing, client change, process exit/OOM | Narrow the failing layer; retries remain bounded and must not duplicate work. |
| Service dies/restarts | Exit status, unit limits, OOM events, permissions and recovery log | Address cause; do not disable all hardening or increase restart frequency blindly. |

For HTTPS origins, check certificate trust/name and the route's actual service protocol. Cloudflare edge TLS mode does not itself fix local origin TLS. Do not use noTLSVerify as a permanent fix. Official diagnostics: [2](https://developers.cloudflare.com/tunnel/troubleshooting/https-origins/).

Capture timestamps, request correlation IDs, revision, tested network/browser and redacted status/headers. Bound log size and media bytes; do not download entire tracks for a header check. Do not publish cookies, JWTs, signed URLs or raw credentials. Synthetic tests and observations from a representative iframe must be labelled as such, not the user's exact client.

## 10. Controlled rollout and rollback

**Stop before public rollout** if any of these are unresolved: approval/audience, provider suitability, auth coverage, origin bypass, required device/session isolation, secret handling, recovery access or agreed workload/resource acceptance. Do not create an unauthenticated staging route on the same live origin.

1. Establish a known-good source/build/config baseline and verify local playback first. Back up persistent state/config to approved protected storage; record paths/permissions and how to restore without exposing secrets.
2. Review the smallest proposed diff and exact service impact. Validate configuration before reload. Connector-managed versus local ingress syntax differs; use installed-version validation where applicable, including a deny/catch-all route for locally managed ingress.
3. Prepare identity policy and origin protection before hostname publication. Validate each boundary with a narrow authorized canary; do not start broad traffic for a screenshot.
4. Verify unauthorized denial, approved browser playback, session isolation and resource behavior, then extend to the agreed workload. Observe a declared acceptance interval rather than inventing a universal success duration.
5. Publish only after agreed gates pass. Record source SHA, installed executable path, versions, sanitized configuration/identity-policy evidence, workload, results and rollback owner.
6. On failure, disable/revert only the newly exposed route or connector change first while preserving local playback. Restore the known-good binary/config through the existing reviewed installer path if needed; do not delete data or kill unrelated processes.
7. Recheck local playback, revoked access, closed direct exposure and absence of orphan work after rollback. State which layer was actually restored.

Updates follow the approved idle window/timezone with tested candidate build, backup, health check and rollback. A source pull or rebuild does not update the executable embedded UI actually served by systemd. No new nightly timer, package autoupdate policy or public hostname is authorized by this document.

## 11. Acceptance evidence and readiness gates

Record every test as PASS / FAIL / NOT RUN with machine/network/browser, version/SHA, steps, expected/observed result and sanitized evidence location. Do not manufacture passing status codes or token-validation evidence.

Required acceptance matrix:

- Logged-out and wrong-identity clients denied for UI, APIs and full/partial audio; approved identity works. Audit path-policy overrides and alternative routes.
- Missing, forged, wrong-audience, expired and unverifiable identity tokens rejected on the protected path. Origin bypass and spoofed forwarded/email headers fail. Local access follows its separately approved policy.
- Mutation CSRF/Origin defense works; unauthenticated simple requests cannot control playback. Cross-origin response restrictions alone are not this test.
- Reauthentication/logout/revocation tested for new and existing audio requests; exact observed ongoing-stream behavior documented.
- Device A cannot pause/skip/stop/read another session or commandeer the speaker when independent mode is approved. Synchronized mode needs a separate approved timing/drift test, not this claim.
- Supported media/HEAD/Range/cancellation/slow-client behavior tested locally and remotely, with correct types and no orphan jobs/permits or unbounded retries.
- Supported concurrent workload stays within the agreed 100 MB boundary without OOM events, hidden helper exclusion or lost healthy streams.
- Android/iPhone behavior, cellular/Wi-Fi transitions and background/lock playback verified on claimed real devices; CI/emulation identified separately.
- Connector/origin outages recover; SSH logout and reboot checks pass in an approved window. Logs/metrics stay private; rotation and rollback are exercised.
- No remaining public bypass, leaked credential or unresolved critical regression. Confirm provider/media suitability, not merely a live DNS record.

Readiness is separate from the quality of this playbook:

| Score | Minimum evidence |
| --- | --- |
| 0/5 | No assessed design or evidence. |
| 1/5 — current | Reviewed plan/skill; no verified protected deployment or off-LAN audio. |
| 2/5 | Approved design and target configuration/implementation exist; major end-to-end gates remain. |
| 3/5 | Protected target canary and auth/origin negative tests pass; named workload/device/recovery gates remain. |
| 4/5 | Broad end-to-end security/load/device/recovery evidence; only explicitly named noncritical acceptance gaps remain. |
| 5/5 | All agreed launch gates passed on the relevant target with retained evidence; no open critical gap. |

Writing a stronger skill does not raise application readiness or prove senior expertise by title. Demonstrate senior practice through scoped decisions, measured evidence, least privilege, clear handoffs and successful recovery.

## 12. Handoff and required response record

Return: (1) current facts and blockers, (2) approved/proposed decision and source rationale, (3) exact files/service/config impact, (4) tests with evidence and NOT RUN entries, (5) rollback steps/owner, (6) next approval or smallest action.

Route UI/browser presentation to Frontend; identity/session/process ownership to Backend after Architecture approval; reproductions/security/load evidence to QA. Do not silently switch departments or implement unrelated features.

End with **📦 BUILD MANIFEST** and **🚨 ERROR & RISK LEDGER**, including the active skill, files/purpose, dependency/API/environment/service changes, current app state, demonstrated causes versus hypotheses, remaining edge cases and honest 0–5 readiness.

## Official research register

Reviewed 2026-11-07. Recheck version-specific flags, dashboard wording, account entitlements and terms at implementation; documentation is not deployment proof.

- Access application sequencing and origin-token validation: [1](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/self-hosted-public-app/).
- Access application-token fields: [4](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/authorization-cookie/application-token/).
- Connector protocol, token-file and metrics flags: [2](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/configure-tunnels/run-parameters/).
- Required outbound firewall transport: [1](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/configure-tunnels/tunnel-with-firewall/).
- Healthy tunnel scope: [3](https://developers.cloudflare.com/tunnel/monitoring/).
- HTTPS origin troubleshooting: [2](https://developers.cloudflare.com/tunnel/troubleshooting/https-origins/).
- systemd service restart semantics: [1](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html).
- systemd resource controls: [1](https://www.freedesktop.org/software/systemd/man/latest/systemd.resource-control.html).
- systemd execution/credentials primary source: https://github.com/systemd/systemd/blob/main/man/systemd.exec.xml . Credential, environment-secret, privilege and home-directory protection entries inspected; check target-release support before applying.
- Browser cookie/credential behavior: [2](https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API/Using_Fetch).
- CORS semantics: [1](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS).
- Media Range responses: [1](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Range_requests).
- Mutation/CSRF defenses: [1](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html).
- Applicable service-specific terms: [1](https://www.cloudflare.com/service-specific-terms-application-services/). The reviewed CDN clause discusses video/disproportionate audio/large-file delivery restrictions; do not infer a blanket Tunnel prohibition or unlimited permission. Resolve the actual account/service/use-case before launch.
