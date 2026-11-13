# Prepared watch-only coverage inventory — not a new acquisition fix

## Latest confirmed host outcome

The host at efde115 successfully ran the offline evidence guard. It returned blocked,
network_executedfalse/native_executedfalse, next offline_source_coverage_review_no_live_retry.
This is the expected gate behavior, not a new native trial or successful extraction.
The earlier v5.1 host packet at04e3f8b remains the latest native observation. Do not
record its pasted repeat as another request or ask for another unchanged guard run.

## Offline upstream audit result

Inspected the already pinned yt-dlp source at
[12ac9aca79671420724227aca37bc888c604a6f8](https://github.com/yt-dlp/yt-dlp/tree/12ac9aca79671420724227aca37bc888c604a6f8),
not a live provider response:

- `_base.py` `_extract_visitor_data`: looks for VISITOR_DATA,
  INNERTUBE_CONTEXT.client.visitorData and responseContext.visitorData. It can also
  consume a user-configured visitor override; that is not allowed in this experiment.
- `_video.py` `_extract_player_responses` passes webpage_ytcfg, initial_pr and
  player_ytcfg to that helper. It does not pass ytInitialData at this point. Do not
  add a fourth speculative page object just because its name occurs elsewhere.
- `_base.py` `_download_ytcfg` has a fixed client-to-page map. VISIONOS is NOT in that
  map and returns an empty config rather than downloading another config page.
  Therefore those alternate config-page code paths do not establish a missing
  VISIONOS bootstrap source or explain why the baseline worked.
- The helper applies its paths generically to its arguments; our typed reader uses
  a narrower config schema and a separate initial-player schema. These are not
  identical parsers. For example responseContext.visitorData in a config object is
  currently ignored. There is no evidence the host page actually uses that shape.
- Upstream's broad ytcfg regex search is not copied into arbitrary JS strings,
  functions or conditional blocks. No generated visitor, account cookie, client
  rotation, external helper, JS execution, new endpoint or network request added.

Conclusion: no new same-page source or host wrapper cause is established by this
review. Current counts leave three possibilities: raw visitor names in ignored
content; names in recognized objects but outside typed paths; or no raw names in
the examined regions. Escaped names can be missed by raw matching. A source graph
alone cannot resolve these alternatives.

## Prepared code, without relaxing the parser

`prototypes/rust-direct/src/coverage.rs` adds optional fixed counters to the existing
separate `--watch-inspect` branch, now identity `aysha-direct-v5.2-coverage-only`.
The full native bootstrap identity/visitor source/schema/request behavior stays v5.1.

Inventory is enabled only for watch-inspect. Other modes use Observation::default
and do not initialize or serialize the optional coverage inventory. Region categories:

- Ignored nonempty script tails, after existing trivia handling.
- Bodies of script tags skipped by the attribute policy (including empty bodies).
- Successfully typed/auth-checked config objects.
- Successfully identity/playability/header-checked initial-player objects.

Each category reports inspected/nonempty region counts and how many contain exact
raw bytes `ytcfg`, or `VISITOR_DATA`/`visitorData`. One hit per region, not occurrence
counts. Ignored tails also receive coarse declaration/parenthesis/window/other prefix
buckets. These are raw-text navigation hints, **not a JavaScript lexer or executable
code detection**. Comments, strings, dead code, substrings and localization text may
match. Escaped names and unexamined inert regions may not. No token, value, body,
attribute, length, hash, offset, URL or error snippet is printed or persisted.

The optional inventory owns only fixed counters, borrows the already bounded page
slices, and returns nothing to the trusted data reader. A marker hit never creates a
visitor, overrides missing/auth/restriction errors or authorizes a player request.
All existing page/literal/depth/header/request/deadline bounds remain in force. This
is not a new parser fix, whole-heap cap, authentication change or production integration.

Added offline tests cover region-not-occurrence counts, raw-string/comment/dead-code
false positives, escaped-key misses, prefix boundaries, large-region/no-value-storage,
unchanged errors with inventory enabled/disabled, skipped attributes not supplying
visitor data, and unknown config responseContext not becoming an accepted source.
Verification: source02e1173, CI43415804715 all five jobs PASS; direct job131853270437
passed62Rust tests/release/no-JS dependency graph/offline CLI. Local84Python tests PASS.
Source files _base.py and _video.py were also byte-compared to the pinned GitHub API
contents, not just trusted by directory name. No local Cargo is available in the agent
workspace; Rust verification is CI evidence, not local hardware/provider acceptance.

The existing watch-only branch still has no player/media call. No new live attempt
has been made by the agent or scheduled by the evidence guard.

## Commands and approval boundary

**Optional development verification only — not required on the music host:**

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
cargo test --locked --manifest-path prototypes/rust-direct/Cargo.toml coverage_
```

Expected: coverage regression tests pass, with no provider/media requests. Cargo may
need its normal registry/build dependencies; this is offline with respect to media,
not a promise that dependency downloads are unnecessary. No install, service restart
or audio output. If a test/build fails, stop and share normal test output; do not run
bootstrap, reset Cargo.lock or repeat live requests to repair it. CI verification is
recorded in the QA ledger; repeating this on the host is optional.

**Approval update:** the user now explicitly requests real server download/play-stream
and budget checks. Proceed only with the already prepared ONE watch-only diagnostic
and a read-only running-service budget audit, not a bootstrap/player/media retry.
Current commands and interpretation: [STREAM_BUDGET_CHECK.md](../STREAM_BUDGET_CHECK.md).
The watch-only check is one public watch GET, at most2MiB,
same fixed origin/user-agent/deadlines, no player/media request even on parser pass,
no credentials/cookies/persistence/JS or new sources. Only the ordinary fixed JSON
with optional coverage counters should be shared. It may still fail visitor_missing;
that failure plus region hints is diagnostic evidence, not acquisition success.

If the user declines, stop the anonymous watch-bootstrap experiment and review the
native acquisition approach before further source-access changes. If approved but
no useful evidence results, stop rather than turning this into a sequence of guessed
wrapper fixes or repeated scans. Unsupported grammar still requires a specific inert
fixture and negative security tests before accepting a new data source.


## Host fixture verification received

User pulled f207206 and ran the coverage_ test filter: compilation2.54s,7passed,
0failed,55filteredout, testtime0.12s. This verifies the seven fixture tests on the
host; it is not a full62-test host run, ordinary CLI binary build, upstream request,
visitor acquisition or playback/memory evidence. Cargo test builds the test harness,
so any future approved live command must explicitly build the normal executable.
No need to repeat these successful tests. Fixture output itself was not approval.
The later explicit real-check request separately authorizes the bounded diagnostic;
see the approval update above. The repeated host test had cached compilation0.10s,
7passed/55filtered/testtime0.12s and still no provider request.


## Completed v5.2 diagnostic: approval consumed, no unchanged retry

User checkout3839d5c normalbuild1.62sPASS, watch_visitor_missing before player/media,
HWM11104KiB. Twoignoredtails rawvisitorname hits/fourrawytcfg; parsedconfig/player
and11skippedattribute bodies zero rawvisitorname hits. This narrows offline coverage
review, not a usable value or exact wrapper. Watch-onlyattempt completed and no
additionalrequest is scheduled. The same user's service samples fail15MB/15MiB at
101,171,200bytes. ALL_RUST_UPGRADE_GATES.md is the current replacement decision;
previous approval/procedure sections above are historical, not permission to repeat.
