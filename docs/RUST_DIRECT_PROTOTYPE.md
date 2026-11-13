# Native acquisition: v5.2 diagnostic complete — replacement blocked

**Status: native acquisition BLOCKED pending target-host evidence.** Parser compatibility
has been consolidated and offline-tested; this is not a working playback/server replacement.
Extraction readiness remains **2/5**. No 10–15 MB whole-server claim.

## Latest decision: not a replacement release

User completed the v5.2 coverage diagnostic and two service-budget windows at3839d5c.
Watch-onlynormalbuild1.62sPASS, visitor_missing, no player/media. Rawvisitorname hits
in2ignoredtails only; no usable source/exactsyntax established. Currentservice sampled
max101,171,200bytes, historicalpeak218,480,640bytes (notattributedtothiswindow),
swap905,216bytes; yt-dlpRSSmax71,299,072/Rust13,381,632bytes, notsummed. Both15MB and
15MiB fail. Bothplayingflags true in secondwindow is not audible-output confirmation.

The request for the full all-Rust upgrade is retained, but conditional replacement is
not allowed now. See [ALL_RUST_UPGRADE_GATES.md](ALL_RUST_UPGRADE_GATES.md) for all
production helper roles, implementation gates and no-fake-release boundary. Existing
workingservice preserved. Watch-onlyapproval is consumed; **no unchanged host rerun**.
The gate reports current_service_budgetfail/native_replacement_allowedfalse. This
readiness report does not change or enforce the installer; no runtimefix is claimed.

## What the host proved

- v2 and v3 compiled/tested, then received `player_bot_check`.
- v4/v4.1 stopped at local watch parsing, before player/media requests.
- v4.2/v4.3 diagnostics narrowed the rejection to a value outside strict JSON.
- v4.4 at `5142200` stopped with `watch_config_js_single_quote` before player/media.
- v5 at `39dd9fe`: debug build1.64s PASS; `watch_visitor_missing` before player/media.
- **Latest host at `04e3f8b`: debug build3.16s PASS; v5.1 `watch_visitor_missing`**,
  stage `watch`, player/media requested `false`. One config and one validated initial
  player; no visitor in any of the three recognized sources. Observed eligible scripts38,
  ignored nonempty tails35, skipped-attribute scripts10, recognized closed wrappers0.
  This does not identify the contents of ignored tails or prove whole-page absence.
  Failed debug HWM10,632KiB (10,887,168bytes), not release/working-stack memory acceptance.
  **The archived wrapper hypothesis is not confirmed; stop unchanged live trials.**
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

The optional `--watch-inspect` mode remains separate, identity `aysha-direct-v5.2-coverage-only`.
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

The source now has **62 Rust tests** covering:

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
Python evidence tests: **83 PASS** after recording the v5.1 host failure and consumed-trial
stop gate. Final unchanged Rust source at04e3f8b was also verified by CI43289003239,
all five jobs PASS. Trusted parsing/bootstrap behavior is unchanged after the latest
failure. Optional watch-only coverage inventory is now prepared, verified at02e1173 /
CI43415804715 all5jobs PASS,62Rust tests and84localPython tests. This inventory required separate approval, now recorded from the user's explicit
real stream/budget-check request. No parser fix or additional native bootstrap retry
is claimed. Follow [STREAM_BUDGET_CHECK.md](STREAM_BUDGET_CHECK.md) for current commands.
The evidence guard also rejects purported prefix passes explicitly marked watch-only,
watch stage, or no player/media request.

These pipeline tests are **synthetic and offline**, not live provider, codec/audio-device
or whole-stack acceptance. No live v5.1 trial has been made by the agent. Cargo is absent
locally; native tests/builds are verified in CI. No automatic live requests in CI.
Python's evidence guard continues to return blocked until native media success is recorded.

## Historical v5.2 preparation — diagnostic and budget procedure now completed

The approved v5.1 trial is complete and failed. Its counters show absence in recognized
sources, not the shape or contents of35ignored script tails.10skipped attributes are
not10proven defects; external scripts are intentionally not read.0closed wrappers does
not prove no wrappers exist anywhere; only none were accepted by this reader.

Graphify was actually run, development-only, on the three direct-reader Rust files at
04e3f8b.197nodes/447edges/15communities, code-only with all model backends and query
logging disabled; no assistant hooks/cloud account/server installation. The code map
points to parse_script's unknown-prefix early return and the three visitor schema
paths. It cannot recover a live page from source or certify runtime reachability.

See **[Graphify source review: where to check](research/GRAPHIFY_SOURCE_REVIEW.md)** for
source locations, verified edges, reproducible queries, the graph attachment location,
privacy boundary, actual host evidence and the offline investigation plan. Graphs,
visualization assets and the Python development virtualenv stay outside Git/runtime.
Code AST extraction and queries passed; attempted sandbox browser smoke was blocked by
missing libnspr4.so (inline HTML script syntax checks passed). No UI or OS change made.

The evidence checker keeps the v5.1 native trial consumed and acquisition blocked.
The user's later request to check real download/play-stream and budget is recorded as
approval for the prepared ONE watch-only diagnostic plus read-only service measurement.
Its next action is now one_approved_watch_only_coverage_inventory_no_player_or_media.
Explicit approval, matching candidate/status/review and an unconsumed request are
required; fixture passes alone never authorize a request. No new player/media trial.

The user has now run this offline checker at efde115 on the host: blocked,
network_executedfalse/native_executedfalse, nextoffline_source_coverage_review_no_live_retry.
This is expected. **No need to repeat that command.** It is not another native result.

The follow-up upstream audit established no new fourth field path in the player flow;
VISIONOS does not have an alternate download_ytcfg config-page mapping. The typed Rust
schemas remain narrower than upstream generic helper paths; this does not prove host
page shape. See [prepared coverage diagnostic](research/WATCH_COVERAGE_DIAGNOSTIC.md)
for the pinned source comparison and optional development-only regression command.

Optional `--watch-inspect` now has identity `aysha-direct-v5.2-coverage-only` and can
collect only fixed counts of raw names/prefix buckets in ignored tails, skipped bodies
and recognized config/player literals. Disabled/absent in other modes; no value recovery
or parser relaxation. Strings/comments can match; escaped keys may not. It cannot
supply visitor data or call player/media. **Prepared and offline-tested, not approved
as a native acquisition trial.** It is now authorized solely as ONE watch-only
coverage diagnostic after the user's explicit real-check request.

No new bootstrap retry is recommended. The one watch-inspect command and a separate
read-only service-cgroup15MB/15MiB audit are in [STREAM_BUDGET_CHECK.md](STREAM_BUDGET_CHECK.md).
100Python tests and all5CI jobs43417518725 passed at891b1c9. No service or playback
control mutation, source expansion, helper fallback or resource-budget pass is implied.
Any next reader change must
have a specific offline unsupported-shape reproduction with negative security fixtures,
not another guessed exception, broad JS regex search, generated visitor, account cookie,
token/client switch or hidden fallback. If supported anonymous data cannot be established,
stop this bootstrap path and review scope before further live/source-access changes.

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
