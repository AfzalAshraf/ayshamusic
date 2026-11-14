# Aysha readiness — evidence to 5/5

Goal: every department reaches 5/5 through implemented, measured acceptance. This
is not a guarantee of defect-free software and does not raise today's scores.
Only one department implements at a time. Active work: **QA & Debugging**, diagnosing
the observed service-memory failure before Performance changes.

## Actual external tracking

Linear project: **Aysha Music — readiness to 5/5**, team Afzalashraf (AFZ).
Created through the connected Linear app; earlier work was recorded in the repository
ledger, not retrospectively claimed as Linear issues.

- [AFZ-5 — memory failure diagnosis](https://linear.app/afzalashraf/issue/AFZ-5/aysha-attribute-and-resolve-service-memory-peak-above-100-mb): In Progress; new bounded `--memory-samples` probe awaits target evidence.
- [AFZ-8 — architecture decisions](https://linear.app/afzalashraf/issue/AFZ-8/aysha-approve-sessionresource-architecture-before-multi-device-work): backlog; independent/synchronized sessions, device count, exact byte threshold, budget scope and speaker ownership.
- [AFZ-6 — secure remote access and Vercel role](https://linear.app/afzalashraf/issue/AFZ-6/aysha-secure-remote-apiaudio-and-decide-vercel-frontend-role): backlog; approval and security gates before exposure.
- [AFZ-7 — device/deployment acceptance](https://linear.app/afzalashraf/issue/AFZ-7/aysha-complete-real-device-playback-and-deployment-acceptance): backlog; audibility, iPhone issue, autoplay, Bluetooth when requested, accessibility, rollback and reboot.

Vercel project **ayshamusic** created in team **afine** (slug afine1). Project
retrieved after creation: no deployments, not live, no Git repository attached.
This satisfies project preparation only. No secrets, domains, routes, service
migration or billed add-ons configured. The local Linux Rust/systemd service on
5050 remains production. Decide whether Vercel is a UI-only preview or a frontend
to a secured home backend before connecting Git or deploying. An unmodified static
UI would request /api on its own origin; deploying it alone does not produce a
working remote player. Browser localhost is not the home service. A first deployment
may require projectSettings controls unavailable through this connector; do not
claim deployment success from project creation.

## Department order and evidence gates

| Department | Current | Required for 5/5 |
| --- | --- | --- |
| QA & Verification | 3/5 | Repeatable regression/failure/load coverage and real-host/device acceptance; first attribute the observed memory peak. |
| Architecture | 2/5 | Approved session/resource/security decisions reflected in implementation and failure tests. |
| Backend | 2/5 | Validated contracts, owned state/processes, approved persistence and tested device isolation. |
| Playback & Audio | 2/5 | Audible Stop/Play/refresh/navigation/autoplay and supported mobile/background/output behavior without reported interruptions. |
| Performance & Memory | 1/5 | Measured aggregate maximum within agreed 100 MB boundary across the approved concurrent workload, without OOM/restart concealment. |
| Security & Privacy | 1/5 | Verified authentication, authorization, CSRF defenses, privacy and origin/session isolation; negative tests pass. |
| Networking & Self-Hosting | 1/5 | Protected remote audio, origin-bypass protection and tested transport/reconnect failure handling. |
| Deployment & Operations | 2/5 | Tested install/rollback/reboot/observability and explicitly approved maintenance. |
| UI & Mobile Frontend | 4/5 | Preserve approved design and pass real supported-device/navigation/accessibility acceptance. |

Architecture approval precedes implementation that changes ownership/authentication
or hosting boundaries. Security precedes public exposure, even if the performance
work is unfinished. UI polish is not a substitute for resolving backend risks.

## Current target evidence, not hypotheses

User reports at checkout b83961a: service active/running, zero automatic restarts,
built/installed on-disk files match, exact served UI matches checkout, UI/status/queue
schemas pass and Python/frontend/Rust suites exit0. Status says playing=true; this
does not prove audible output. Current memory readings 97.90–98.16 MB; historical
service peak119,746,560 bytes (119.75 MB/114.20 MiB) exceeds both interpretations of
100 MB. Cause unknown. No Bluetooth sink, which is optional for browser mode.
Do not blame cache/downloaders, declare a leak, restart to erase history, or impose
a hard OOM limit without profiling evidence. Do not call all departments verified.
