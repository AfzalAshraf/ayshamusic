# REA-assisted native extraction plan — evidence before another rewrite

## Confirmed baseline

User host ef5dd83 public/no-config/no-cookie AAC prefix probe PASS,64KiB, first-byte
3437.45ms, total3657.58ms, sampled tool-group peak93296KiB (~95.5decimalMB).
Native v2 previously gotplayer_bot_check on same host. These were different attempts,
not simultaneous controlled requests: a successful baseline establishes this sample
was obtainable then, not a universal absence of challenges or a known native cause.
Python/Node/external helpers still exist in the measured baseline and are not free.

## What REA actually provides

Inspected https://github.com/morluto/rea (MIT, source revision reviewed
 a8527d43b8439476f5c9d1497315dfe7dd5e1f38). REA is a Node-based reverse-engineering
analysis/MCP toolkit, not a Rust extractor/player/compiler, bot-check solution or
pure-Rust port generator. Native binary analysis can require separate providers;
not needed here because public source is available. No global/MCP/agent setup or
host service changes were made.

Actually ran npm-pinned rea-agents6.9.0 in isolated /home/user/rea-lab. Host agent's
Node22.22.3 is below REA's supported range; installed a separate npm-pinned Linux
Node26.12.1 executable outside this repo and explicitly ran REA with it. Package
installation had lifecycle scripts disabled; no target/build/solver scripts executed.
The install emitted an engine warning from npm's old runtime; analysis itself used
the verified supported runtime. This is development research, NOT a15MB service.
No suggestion to upgrade the user's system Node or install commercial disassemblers.

Target: yt-dlp/ejs commitd48c6303a5a6455a42ef9e0b445149cadea51279, package.json and nine
non-test src TS files fetched from public GitHub. REA static analysis:9parsed sources,
0parse failures,1895AST nodes,66findings;452semantic unknowns,0reconstructed modules.
Thus do NOT claim full semantic recovery/dependency equivalence. Compact reproducible
source hashes/statistics/evidence identity: research/REA_EJS_AUDIT.json. Full1.85MB
analysis bundle/public input and tool installation remain outside Git; not production
artifacts. Public EJS is Unlicense; no EJS implementation is copied into our service.

Reproduction AFTER obtaining matching tools/inputs in an isolated development folder:

```bash
/path/to/supported/node /path/to/rea-agents/scripts/rea.mjs \
  analyze-javascript-application /path/to/selected-ejs-source \
  --artifact-format directory --max-heap-mb 256 --analysis-timeout-ms 20000 \
  --max-semantic-nodes 4000 --max-semantic-nodes-per-file 800 \
  --detail summary --evidence_output /new/path/evidence.json --format json
```

This is a developer reproduction template, not a user-server installation command.
Evidence output refuses overwrites. REA read source as inert text; analysis did not
run its JS, access YouTube, attach to production or inspect user credentials.

## Source findings (not inferred execution on the user's host)

Public EJS main.ts -> preprocessPlayer/getFromPrepared -> per-request n/sig solvers.
solvers.ts parses player AST via meriyah, rewrites/generates via astring, then uses
Function("_result", code) to instantiate functions. This is real JS evaluation,
not a general string-reversal algorithm. Multiple candidates must agree or fail.
Source reference:
https://github.com/yt-dlp/ejs/tree/d48c6303a5a6455a42ef9e0b445149cadea51279/src/yt/solver

Signature/n transforms and provider bot/authorization/integrity checks are separate.
Porting this solver does NOT by itself solve a rejected player response. The direct
prototype has neither webpage/client orchestration nor signature execution; the
successful baseline passing --js-runtimes node does NOT prove it executed Node or
needed this particular latest EJS version. Do not announce a native engine dependency
or port everything based on static analysis alone. GPL/proprietary dependencies and
provider terms must be reviewed separately before any implementation reuse.

## Concrete next code: safe stage trace of the working baseline

Added --trace-stages to the existing --baseline-only diagnostic, using only stdlib
Python development tooling, not server logic. The child receives --verbose, but raw
stderr is concurrently drained and discarded. Only fixed known-client request and
jsc-provider markers, booleans and a truncation flag enter the report. No raw stderr
file, full request/response, challenge strings, cookies, headers or account data.
Classification examines at most256KiB, max4KiB per line; overlong lines discarded.
Output is deliberately lossy; empty markers DO NOT prove no JS/client activity.
A client request marker does NOT prove that client supplied the selected media URL.
Trace observations can change timing; do not treat this as a performance benchmark.

### Terminal: DIAGNOSTIC ONLY — no REA installation, rebuild or restart

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
python3 scripts/benchmark_extract.py --baseline-only --trace-stages \
  --video-id Jm7Satp7Zqk --runs 1
```

Same AAC64KiB validation/external75sdeadline, own-child cleanup, unique0600ignored
report; one public request attempt, no native retry/config/cookies/proxy/client override.
Production API/service not touched. Review fixed report before sharing. If the baseline
now fails, retain failure: do not run a loop or automatically fall back to another path.

## Implementation gates for Rust responsibilities

1. **Now: confirm host stages.** Compare safe client/EJS markers to pinned public-source
   inspection. Unknown markers/version differences need source review, not guessing.
2. **Native acquisition design:** choose supported public-source/client orchestration;
   any authorization/integrity requirement is explicit. Respect bot/restriction failure,
   never silently use external downloaders/proxies/tokens. Do not collect accounts in chat.
3. **Only if needed: native signature worker experiment.** A Rust-owned, bounded parser/
   interpreter or Rust JS engine requires explicit dependency/behavior review, CPU/heap/
   cancellation budgets and offline fixtures before live use. Reusing Boa previously
   exceeded15MB on a failed path; do not promise a leaner interpreter fits. Native TLS
   and OS libraries need honest accounting, not a claim every dependency is Rust.
4. **Acquisition acceptance:** actual successful media read on representative permitted
   public tracks with no external helpers, restriction failures surfaced, cold/warm
   latency and complete memory boundary measured. No further production integration
   until this gate passes.
5. **Decoder/output:** existing Rodio/Symphonia audio prototype, real AAC decode and
   audible output, EOF/seek/pause/reconnect/cancellation and owned-worker tests; no mpv.
6. **Server/API/search/queue/session:** independent accepted semantics, bounded jobs and
   buffers, authenticated remote access. Search/related-song discovery are separate
   implementations, not provided by REA/EJS. Full15MB capacity is not assumed.
7. **Production switch:** target-host integration/device/memory/failure tests, explicit
   install/restart approval and rollback. Rust HTTP server already exists; current
   subprocess responsibilities remain until real native equivalents pass.

Only diagnostic/analysis changes are implemented here. This plan is not blanket approval
for new authentication, licensing, provider-access, hosting, persistence or session
architecture. No whole-app Rust-only or15MB compliance is claimed.
