# Isolated Rust extractor gate — ONE approved live experiment

Not an APK dependency, playback fix or installed player. The user approved Rust-
only extractor research (including a Rust-implemented JS runtime if needed) and
one public-video CI experiment:12requests,8MiB,60seconds,two small media ranges,
no account credentials or bypasses. Earlier unrelated watch approval was consumed;
this is a newly explicit approval. Do not rerun after the live step starts.

Candidate: `Mithronn/rusty_ytdl`, unmerged PR62, exact commit
`b390352a5db4b57aa630e8dc7b427fb560feccd5`. Reviewed changes introduce ANDROID_VR
in the fuller Rust extractor. It uses Boa (Rust JS engine); no Python/Node runtime.
Published crate/main have known403 reports, so this revision is research, not a
claimed production solution. MIT/Apache2 upstream, no source vendored here.

The dedicated executable is offline unless `--approved-once` is explicitly given.
The integration denied manual workflow dispatch (403); no live test was started
by that API call. The Android workflow therefore additionally permits one explicit
approval-marker push with the exact recorded parent commit. A future normal push
cannot match that parent. This single gated push calls the reusable source workflow;
ordinary push/PR/APK builds do not run the source test. Manual invocation remains
possible only with the explicit approval input and must be disabled after use. Build/guard tests occur
before any live step. A build-only failure does not consume a live experiment.

A loopback CONNECT guard restricts outbound TLS connections to YouTube/googlevideo
on443, at most12. The injected HTTP client forces HTTP/1, Connection:close,
zero idle pooling, no redirects/retries/cookies or supplied proof tokens. Therefore
each admitted connection carries at most one request. The guard reserves bytes
before every read: at most8MiB admitted encrypted bidirectional transport bytes
including metadata/player code overhead. Decoded gzip content is not claimed to
be separately capped at8MiB. Inner deadline55s and external kill60s bound the run.
No other client is constructed by the exercised `new_with_options/get_basic_info`
path. Signed source URLs, HTML, JS and response bodies are never printed/uploaded.

Public test video is fixed to `dQw4w9WgXcQ`, not a user-account resource. The only
media requests permitted by program structure are first64KiB and final64KiB of one
AAC/MP4 format. No retries, fallback songs, login, download/stream/ffmpeg invocation
or full-track transfers. Source rejection stops the experiment immediately.

Passing these ranges is necessary, not sufficient: Android compilation, Rust
AAC decoding, size and actual phone tests would still be needed before an APK.
A cloud/CI network's result cannot certify the user's phone/region. If blocked,
retain the sanitized code/counters and stop; do not start another live experiment
without approval. Cargo.lock and sanitized JSON are the only uploaded evidence.
