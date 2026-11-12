# Native acquisition: current v5.1 plan

**Status: native acquisition BLOCKED pending target-host evidence.** Parser compatibility
has been consolidated and offline-tested; this is not a working playback/server replacement.
Extraction readiness remains **2/5**. No 10–15 MB whole-server claim.

## What the host proved

- v2 and v3 compiled/tested, then received `player_bot_check`.
- v4/v4.1 stopped at local watch parsing, before player/media requests.
- v4.2/v4.3 diagnostics narrowed the rejection to a value outside strict JSON.
- v4.4 at `5142200` stopped with `watch_config_js_single_quote` before player/media.
- Latest host at `39dd9fe`: debug build PASS in 1.64 s; v5 stopped with
  `watch_visitor_missing`, stage `watch`, player/media requested `false`.
  Initial-player playability/identity checks passed; no accepted visitor was found in
  recognized sources. The report does not distinguish absent visitor data from skipped
  config scripts. Failed debug HWM: 10,884 KiB (11,145,216 bytes), not release,
  successful-acquisition or whole-service memory acceptance.
- Earlier same-track yt-dlp baseline fetched an AAC/MP4 64 KiB prefix successfully.
  Its client/JS markers do not prove exact request equivalence or absence of JS.

Machine-readable supplied observations: [NATIVE_ACQUISITION_HOST.json](research/NATIVE_ACQUISITION_HOST.json).
Prior decisions/commands are preserved in [RUST_DIRECT_HISTORY.md](research/RUST_DIRECT_HISTORY.md)
**for history only—do not execute archived trial commands**.

## Static data grammar introduced in v5

Replaced `config_numbers.rs` and the double-quote-only object framer with one bounded
`literal_data.rs` reader, shared by config and initial-player data. This is a data
parser, **not a JavaScript interpreter or regex quote replacement**.

Supported static grammar:

- Objects/arrays; single- or double-quoted keys/strings; ASCII identifier property keys.
- Escaped quotes/backslashes, standard control escapes, `\xHH`, `\uHHHH`, valid UTF-16
  surrogate pairs and escaped line continuations. Decoded strings are validated UTF-8.
- JSON numbers, booleans and null; comments and optional object/array trailing commas.
- Statement whitespace/comments, fixed `ytcfg.set(...)`, and `var`/`let`/`const` or
  bare `ytInitialPlayerResponse` assignments at inline-script start/after accepted data.
- The prior narrowly guarded non-finite metadata path: exact NaN/Infinity/-Infinity
  placeholders only in metadata the typed Config discards. Never visitor/login/context
  fields or descendants; escaped keys are decoded before checking protection.

Rejected rather than executed or guessed:

- Calls, references, string concatenation/operators, getters, computed keys, spread,
  regex/templates/interpolation, BigInt/non-JSON number formats and object-literal
  prototype mutation. This is deliberately not all of ECMAScript.
- Missing/truncated data, invalid escapes/UTF-8/surrogates, array holes, dangling
  structures, unrelated dynamic config calls and unsupported HTML/script wrappers.
- Duplicate recognized fields remain duplicates in canonical JSON so the typed
  security/schema checks can reject them. No first/last-value overwrite.
- String `"false"` never becomes boolean false; login true is still rejected. Escaped
  CR/LF/NUL remain real decoded controls and are rejected by the visitor/header guard,
  not sanitized into an accepted opaque value.

Comments/trailing commas/quoted strings are intentional parts of the static grammar,
not permission to repair arbitrary invalid programs. Unknown scripts are not evaluated,
and static parsing does not establish full browser-runtime equivalence. HTML handling
is a conservative subset, not a complete browser parser or universal challenge detector.

## V5.1: verified wrapper compatibility gap, not a confirmed current-host cause

Re-inspected the pinned public archive used in the earlier syntax audit. Its main
config literal starts inside `(function() {window.ytplayer={}; ytcfg.set({...}); ...})();`,
not at script top level. V5 skips this function-prefixed script. The archive also has
an exact fixed message-localization helper before closure and subsequent viewport
calls. This is an offline reproduction of a real reader gap, **not proof that the
current host received this wrapper**. See [the structural audit](research/WATCH_LITERAL_SYNTAX_AUDIT.json).

V5.1 matches only a parameterless anonymous IIFE starting with static config or the
exact optional `window.ytplayer={};` reset. All config statements still go through
the data reader and typed checks. An optional localization helper must match the
entire audited token scaffold and take one static message object. Both standard
immediate-invocation parenthesis placements are checked. No function is executed.

The wrapper must fully close and be immediately invoked in the recognized syntax;
uninvoked/truncated/named/parameterized/async/arrow/conditional functions are not visitor
sources. Unknown operations, returns, nested candidate wrappers, auth mutations or
altered localization helpers inside a recognized wrapper stop the probe. No token
search into arbitrary functions, strings or comments. After a complete wrapper, only
accepted static data/fixed viewport prefixes are read; unrelated suffix scripts are
ignored, as before. This is not whole-JS runtime equivalence or permission to accept
arbitrary wrapper forms. The dependency/provider/privacy/request boundaries are unchanged.

The synthetic fixture `prototypes/rust-direct/fixtures/static-wrapper.html` captures
that scaffold with invented values, including a deliberately non-executable bootstrap.
It covers real archived structure rather than only convenient top-level object fixtures.

All watch-mode reports now include `watch_observation`, fixed counts/booleans only:

- Eligible `inline_scripts`, `skipped_script_attributes`, `ignored_script_tails`.
- Typed `config_objects`, fully closed `wrapped_config_blocks`, validated `initial_players`.
- `config_visitor_present`, `context_visitor_present`, `player_visitor_present`.

These are partial-progress observations (also on error), not execution traces, browser
proof, data-validity claims or media success. A config counter increments after typed
parse/auth checks but may precede a visitor/header or wrapper-closure failure. No raw
page, key, value, offset, hash, visitor length or request detail is exposed. Observations
are per invocation only, not saved by the program and cannot authorize player requests
when the parser returns an error. They do not add a network request or a fallback.

## Network and privacy boundary

V5.1 `--watch-bootstrap` identity: **`aysha-direct-v5.1-static-wrapper`**.

One explicit application-level chain:

1. Fixed HTTPS public watch GET, no cookie jar/authentication/redirects/proxy.
2. Parse inert static data; require initial-player permitted playability, matching
   video identity and non-live content; require unambiguous bounded anonymous visitor data.
3. One fixed VISIONOS player POST with existing Origin/UTC fields and visitor header.
   No visitor in the body, no signature/PO-token generation or restriction flags.
4. Only if permitted: choose validated direct AAC URL and read one MP4 64 KiB prefix.

The visitor header goes only to the fixed YouTube player origin, never the media origin.
No credentials, account cookies, consent acceptance, generated tokens, client rotation,
JS runtime, yt-dlp/mpv/Python/Node subprocess, persistence, tracking reuse or fallback.
Provider restrictions stop the chain. No raw page, visitor value, signed media URL,
request header, reason text or raw exception is printed/saved. State is invocation-only;
dropping memory is not secure zeroization of allocator/HTTP/TLS buffers.

The optional `--watch-inspect` mode remains separate, identity `aysha-direct-v5.1-watch-observation`.
It can never request player/media even when parsing passes. It is **not an extra request
recommended before or after the one full trial below**.

## Resource bounds—not whole-heap certification

| Item | Bound |
|---|---:|
| Watch payload | 2 MiB |
| Individual literal / canonical output | 1 MiB each |
| Object/array nesting | 64 |
| Accepted visitor data | 4 KiB |
| Non-finite metadata replacements | 256 |
| Player metadata / formats | 1 MiB / 256 |
| Media prefix | 65,536 bytes |
| Full async chain / request / connect | 60 s / 20 s / 10 s |

Strict JSON is borrowed unchanged. Non-JSON static data uses one canonical output
buffer reserved to 1 MiB plus temporary decoded strings (bounded by the input) and
small parser state; no whole Value object tree. HTTP/TLS/Serde/allocator overheads
are not fully heap-capped by these payload limits. Full watch buffers are released
before player/media work; no whole-song buffer. Debug RSS is not a release benchmark.

## Verification

The source has **55 Rust tests** covering:

- String/key escaping, Unicode/surrogates, comments, trailing commas, nested data,
  framing, deterministic string corpus comparisons and size/depth/output limits.
- Auth/type/duplicate/identity/restriction/live/header guards and private fixed errors.
- Protected metadata constants; unsupported executable forms; owned-state cancellation.
- Whole synthetic watch pages through parser → offline request construction → fixed
  player-response selection → exact prefix validation, including negative cases.
- Header isolation and success/error observations without opaque visitor data.
- Archive-shaped static wrapper fixtures, exact closure/invocation, fixed localization
  helper, hidden/conditional/uninvoked/nested/dynamic wrappers and missing-data counters.

Source `a169b48`: CI **43288159691**, all five jobs PASS; direct job **131426972404**
passed tests, release build, dependency graph and invalid-CLI offline checks. Local
Python evidence tests: **80 PASS** after recording v5 host failure and v5.1 next action.
The evidence guard also rejects purported prefix passes explicitly marked watch-only,
watch stage, or no player/media request.

These pipeline tests are **synthetic and offline**, not live provider, codec/audio-device
or whole-stack acceptance. No live v5.1 trial has been made by the agent. Cargo is absent
locally; native tests/builds are verified in CI. No automatic live requests in CI.
Python's evidence guard continues to return blocked until native media success is recorded.

## Next: one changed native trial, not more syntax-only probes

The user approved anonymous watch bootstrap, and requested fixing the remaining issues.
This trial stays within that same source-access scope; no new authentication, deployment
or session architecture. Keep the working production service intact.

**Terminal: DEBUG BUILD + ONE NATIVE DIAGNOSTIC — no install/restart**

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
cargo build --locked --manifest-path prototypes/rust-direct/Cargo.toml &&
timeout --kill-after=2s 75s prototypes/rust-direct/target/debug/aysha-direct-prototype \
  --video-id Jm7Satp7Zqk --watch-bootstrap
```

No repeated test/release LTO cycle on the host is needed after verified CI. Preserve
its existing Cargo.lock; no dependency update/deletion. Do not run an inspect probe first.

Expected: one JSON with candidate `aysha-direct-v5.1-static-wrapper` and either:

- `status: pass`, `bytes: 65536`: native prefix acquisition worked for this attempt.
  `literal_syntax_normalized` and `metadata_nonfinite_ignored` report only fixed booleans.
  This is **not** full codec validation/playback, reliability, release memory or production readiness.
- Failure with fixed stage/code: stop, share the single normal JSON (including fixed
  `watch_observation` counts/flags), do not loop or collect raw pages/visitor values/
  cookies/headers/debug traces. If `watch_visitor_missing` with parsed configs but all
  visitor-path flags false, no visitor exists in those recognized sources: stop, do
  not synthesize one or repeat syntax-only tweaks. Zero configs with ignored scripts
  instead identifies a coverage gap; wrapper errors identify unsupported scaffold.
  Either requires specific offline evidence/scope review before another patch/trial,
  not another arbitrary exception or regex search. Player restriction
  means stop this access path; no token/account/proxy workaround. A signature/media failure
  is new-stage evidence, not success or permission to fall back to external helpers.
- Build/lock failure or timeout (exit124 may lack JSON): stop and share normal output;
  no lock reset, production update or automatic retry.

## Remaining work in dependency order

1. **Native acquisition gate:** successful real bytes, restrictions handled honestly,
   representative permitted inputs, no hidden helpers. This remains unpassed.
2. **Audio replacement:** existing isolated Rodio/Symphonia prototype must decode the
   acquired AAC and prove audible output, EOF/seek/pause/volume/cancellation/reconnect;
   no mpv fallback. Native prefix acquisition alone does not satisfy this gate.
3. **All server responsibilities:** replace search/related discovery and existing
   extraction/player subprocess roles; test API/control/queue/recovery. Production
   session/account/storage/tunnel changes need their separate approvals.
4. **Complete 10–15 MB acceptance:** measure the agreed whole-stack boundary and device
   load in release, including audio/shared services and all owned workers; no hidden
   helper, off-host accounting, swap dependence or OOM cap as a fake success.
5. **Production switch:** only after the above, explicit install/restart approval and rollback.

V5.1 fixes the known archived static wrapper gap and adds missing-data observations. It does not pretend
that every remaining resource, playback, provider-access or multi-device issue is solved.
