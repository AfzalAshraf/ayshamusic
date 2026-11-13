# Graphify: where to check the native watch-reader coverage

## Answer

Graphify is useful here as a **development-only source map**, not a native extraction
library, runtime debugger, provider session source or all-Rust server component.
It was actually run in an isolated agent-workspace virtualenv, not installed on the
user's music server or registered as an assistant skill/hook. No cloud account or API
key was used. No production dependencies/configuration/services changed.

Upstream reviewed: [Graphify README at the inspected commit](https://github.com/Graphify-Labs/graphify/blob/52cb28218eea0c68d80a9f4b2ae102d2fa63e267/README.md).
Source pin **52cb28218eea0c68d80a9f4b2ae102d2fa63e267**, package **graphifyy 0.9.108**,
Apache-2.0. Its default branch at review was v8. Upstream distinguishes deterministic
local code AST extraction from model-backed docs/PDF/media semantic extraction. It
also documents default local query logging; this run explicitly disabled it.

## What was run and delivered

- Source inspected: `prototypes/rust-direct/src/{main,bootstrap,literal_data}.rs`
  at Aysha source revision **04e3f8b**. Includes test code, not the whole server.
- `GRAPHIFY_BACKEND=none` and `GRAPHIFY_QUERY_LOG_DISABLE=1` on every command;
  `extract --code-only`, then `cluster-only --no-label`.
- **197 nodes, 447 edges, 15 communities**; edge tags **400 EXTRACTED / 47 INFERRED**.
- Actual queries: `explain watch_visitor`, `explain parse_script`, `path probe remember`.
- Generated `graph.json`, `GRAPH_REPORT.md`, `graph.html` remain outside Git under
  `/home/user/graphify-lab/aysha-direct/graphify-out/` in the agent workspace.
- User-facing single-file graph: `/home/user/graphify-lab/Aysha-native-reader-graph.html`.
  Open/download that attachment and search `parse_script` or `watch_visitor`. Its paths
  are relative to the three-file input root, so `bootstrap.rs` means the file below.

The generated HTML ordinarily loads vis-network from unpkg. For this deliverable,
vis-network **9.1.6** was obtained from the npm registry and embedded inline, verified
against the original template's SHA384 SRI value. No external script/resource tags
remain; the library's license notices are retained. All four inline scripts passed
`node --check`. Attempted browser smoke was **NOT RUN successfully**: cached Chromium
could not start because the sandbox lacks `libnspr4.so`. No browser/OS package change
was made. This is an explicit visualization-testing limitation, not a Graphify extraction
or native media success. Use the Markdown source locations below if the viewer blocks HTML.

## Where to inspect first

These are source locations at 04e3f8b (Rust source unchanged by this evidence review).

| Check | Source location | Why |
|---|---|---|
| Network boundary | [main.rs:166](../../prototypes/rust-direct/src/main.rs#L166) `watch_visitor` | One bounded public HTML read, then `parse_observed` |
| Script eligibility | [bootstrap.rs:264](../../prototypes/rust-direct/src/bootstrap.rs#L264) `parse_observed` | HTML/inert-container/attribute handling; counters; final missing-visitor gate |
| Exact static wrapper | [bootstrap.rs:341](../../prototypes/rust-direct/src/bootstrap.rs#L341) `wrapper_open` | Narrow parameterless/static-data scaffold, no generic function search |
| Prefix coverage policy | [bootstrap.rs:373](../../prototypes/rust-direct/src/bootstrap.rs#L373) `parse_script` | Recognizes wrapper/config/player prefixes; lines400–405 return after unknown nonempty tails |
| Visitor locations | [bootstrap.rs:33](../../prototypes/rust-direct/src/bootstrap.rs#L33) `Config`, `Context`, `ResponseContext` | Only top-level VISITOR_DATA, context.client.visitorData, player.responseContext.visitorData |
| Validation, not generation | [bootstrap.rs:157](../../prototypes/rust-direct/src/bootstrap.rs#L157) `remember` | Opaque visitor alphabet/length/conflict rejection; no generation or substitution |
| Syntax support | [literal_data.rs](../../prototypes/rust-direct/src/literal_data.rs) | Bounded static data only, not arbitrary JavaScript execution |

Manually verified EXTRACTED call edges include:

```text
probe -> watch_visitor -> parse_observed -> parse_script
                                           |-> config_start / decode_config
                                           |-> wrapper_open / wrapper_close
                                           |-> player_start
                                           |-> remember
```

The branch drawing is a source-navigation summary, not execution order or proof all
branches execute. Graphify's generic shortest `path probe remember` query instead
returned an **INFERRED 0.55 reference shortcut via Observation and the containing file**.
Do NOT present that shortest graph path as a runtime call trace. Read confidence tags,
edge types, source locations and real code; symbol resolution/AST extraction are not
compiler reachability or completeness guarantees. No graph finding establishes current
provider page contents or that visitor reuse would make the player request playable.

## Latest host result, and what not to infer

User supplied at source04e3f8b: debug build3.16s PASS; v5.1 failure watch_visitor_missing,
stagewatch, player/media requestedfalse. HWM10632KiB (10,887,168bytes) is a failed debug
process, not complete server/release/playback memory compliance.

Observed eligible inline scripts38, parsed config objects1, validated initial players1,
closed recognized wrappers0, ignored nonempty tails35, skipped-attribute scripts10.
All three recognized visitor-presence flagsfalse.

This proves **absence in the parsed recognized sources only**. It does not establish
that the whole page lacks visitor data, that the35tails are all config-bearing, or
that10skipped attributes are all defects (external scripts are intentionally skipped).
The archived wrapper hypothesis is not validated by this host packet. Graphify has no
copy of that live page and cannot recover its hidden/skipped values from source code.

## Next action: offline coverage investigation, no unchanged live retry

The single approved v5.1 trial is consumed and recorded. The acceptance checker now
returns **blocked**, next **offline_source_coverage_review_no_live_retry**. A stale
awaiting flag cannot revive a trial whose host result is already recorded; explicit
offline review also overrides stale watch/native recommendations.

1. Use this map to audit the early-return/attribute policies against pinned public,
   inert, value-free/synthetic fixture shapes, including unsupported prefixes.
2. Any new source or wrapper support needs a concrete reproducible fixture, retained
   auth/type/restriction/ambiguity checks, and a negative execution/conditional test.
   Do not blindly expand regex search through arbitrary JS or add another guessed exception.
3. If no supported anonymous visitor source can be established, stop this bootstrap
   path and review alternatives/scope; no generated visitor, cookie/account/token,
   client rotation, helper fallback or bypass of provider restrictions.
4. Only after specific evidence and a meaningful patch/scope review propose a new host
   request. Do not run watch-inspect plus bootstrap or silently expand source access.

### Optional diagnostic on the host — offline evidence review only

No native rebuild, upstream request, audio output, install or restart:

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
python3 scripts/check_native_acceptance.py
```

Expected `status: blocked`, `next: offline_source_coverage_review_no_live_retry`,
`network_executed: false`; exit **1** is the expected failed acceptance gate, not a
request to repeat the live probe. If evidence is missing/invalid, stop and report its
fixed code; do not run a network trial to repair the offline checker.

### Reproduce Graphify on a development machine, not as a service dependency

After separately installing the reviewed Graphify version in an isolated development
virtualenv (not global `graphify install`, no assistant hooks or cloud account needed):

```bash
GRAPHIFY_BACKEND=none GRAPHIFY_QUERY_LOG_DISABLE=1 graphify extract \
  prototypes/rust-direct/src --code-only --out "$HOME/aysha-reader-map"
GRAPHIFY_BACKEND=none GRAPHIFY_QUERY_LOG_DISABLE=1 graphify cluster-only \
  "$HOME/aysha-reader-map" --no-label
GRAPHIFY_BACKEND=none GRAPHIFY_QUERY_LOG_DISABLE=1 graphify explain parse_script \
  --graph "$HOME/aysha-reader-map/graphify-out/graph.json"
```

Open `$HOME/aysha-reader-map/graphify-out/graph.html` or read its `GRAPH_REPORT.md`.
The default upstream HTML may require its CDN script; the delivered agent-generated
attachment embeds that dependency. Do not run an unrestricted docs/media semantic
pass or enable a backend as a substitute for this explicitly code-only review.
Graph output and dev-tool resource usage are not included as hidden production workers.
