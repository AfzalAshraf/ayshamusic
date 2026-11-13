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

## Host stage result and next offline comparison

User supplied traced run at host-reported2026-11-12T17:39:58.809904Z:
AAC65536bytesPASS, first4028.85ms,total4242.01ms, sampledgrouppeak98672KiB
(101,040,128bytes). VISIONOS request marker, webpage request marker, no recognized
JSprovider/challenge marker, classification not truncated. Nativeexecutedfalse.
This narrows investigation toward request construction/bootstrap, NOT an immediate
JS-engine port. It does not prove no JS executed or which client supplied media.
The host timestamps are preserved as supplied, not asserted as the agent's wallclock.
No performance comparison to the earlier non-traced attempt is valid.

Public source reference _base.py at12ac9aca79671420724227aca37bc888c604a6f8:
_extract_context sets timeZoneUTC and utcOffsetMinutes0; generate_api_headers
sets Origin and may include visitor data. _video.py constructs a richer player
query and obtains visitor data from watch/config/player metadata. Nativev2 lacks
Origin/timezone and has no watch bootstrap. These are observed implementation gaps,
NOT proven bot-check causes. Some upstream context flags/session/auth behavior need
separate review; do not mechanically copy everything or add session/auth handling.
No signature/POtoken/provider-restriction solution is inferred from this comparison.

New scripts/inspect_source_profile.py uses the INSTALLED Python package's default
profile helpers only: no initialize/extract/webpage/JS/provider calls, no cookies or
config supplied, cache disabled, plugin search disabled, socketconnect/connect_ex/
create_connection and downloader.urlopen guarded. This is trusted installed-package
inspection, NOT a malicious-Python-code sandbox. Auto-init disabled avoids downloader
initialization; it does not prove all possible third-party import code is inert.
No package installation, update, logging, cache, report file, private values or API
request capture. Only known profile fields, equality/presence booleans and bounded
package version printed. Private API differences fail with fixed errors, not traces.
Synthetic empty config deliberately cannot describe actual host watch-session state.

Verified offline against public source commit12ac9aca... in isolatedlab (not installed
into production/project Python): profile1.02/device/OS/UA/header101 match nativev2;
UTC/timezone fields and Origin differ. REQUIRE_JS_PLAYERfalse. This source tree's
version module reads2026.08.19; commit identity, not that static release label, pins
this research. It is not evidence of user's installed package or CLI identity.

### Terminal: OFFLINE DIAGNOSTIC ONLY — no network probe, Rust build or restart

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
timeout 10s yt-dlp --ignore-config --version &&
timeout 15s python3 scripts/inspect_source_profile.py
```

Expected: public CLI version then fixed-field JSON comparing installed Python defaults.
Compare package_version with CLI version; even a match isn't binary/source identity.
If package missing/incompatible: do not pip install/upgrade production; share fixed
error and CLI version, then resolve interpreter/package packaging. Exit124 = timeout;
do not loop. Matching profiles with only known Origin/timezone gaps does not establish
those gaps cause rejection: next request must investigate justified public bootstrap
context or a narrowly controlled native change, not repeat identical builds. Any
visitor/session handling needs explicit scope review before implementation. Different
profile defaults call for a minimal version-supported profile review first. Keep
production and stopped5051 untouched. All-Rust/15MB remains unproved.


## Latest v3 host result: stop context-only variant

Hostb2efd81:11tests/releasePASS, v3stillplayer_bot_check,4404KiBfailedrequestHWM.
Origin+UTC did not establish acquisition. No reason to repeat or add JS on this
evidence. See RUST_DIRECT_PROTOTYPE.md finalsection for stoppedtrial, offline
acceptancegate and explicitlyunapprovedanonymouswatchbootstrap scope. No new
networkdiagnostic or rebuild is requested. PureRustgoal/15MBstillunproved.

## V4 scope approved; implemented and offline verified, live result pending

Userapproved boundedanonymouswatchbootstrap. Implemented opt-inRust--watch-bootstrap
with inertliteralJSONreader, initialplayerrestriction/identitygate, fixedorigin
visitorheaderonly, no credentials/cookies/persistence/JS/POtoken/fallback and no
productionwiring. SeeRUST_DIRECT_PROTOTYPE.md for exactbounds/subset/command/stops.
CI43274748320 over7d0890c: all5jobsPASS inclnativefixtures/build/CLI. No livev4probe.
REA remainsresearchonly, no dependency/runtime integration or new REA execution.


## V4 host stopped at local parser; v4.1 changes literal compatibility only

Host120c5fe21tests/releasePASS; stagewatch/watch_literal_invalid/5764KiBfail. No
playerPOST/mediaGET reached, so no bootstrap-enabledplayeraccessclaim eitherway.
PublicarchivedHTMLconfirms theoldreader rejects anunrelatedtwoargumentviewportsetter;
not proof ofexacthostcause. Syntheticfixtures/narrowskip/whitespace+EOFtermination
correctionandspecificfixederrorsimplementedasv4.1withoutsource-accessscopeexpansion.
SeeRUST_DIRECT_PROTOTYPE.md latestsection for onechangedtrial/stopbranches. No newREA
execution, JSengine, fallback, auth, consentautomation, persistenttracking or production
change. PureRustfullservice/10–15MBstillunproved.


## V4.1hostconfigdecodefailed; watch-onlystructuraldiagnosticnext

Host5dd69ff25tests/releasePASS; stagewatch/watch_config_invalid/6016KiBfailedbootstrap,
no playerPOST/mediaGET. Do not infer exactfieldcauseorjustaddmoreparserexceptions.
Newseparate--watch-inspect/v4.2diagnosticdistinguishesJSONsyntax/incomplete, knownfield
typesandduplicateswithfixedcodes; evenonparsesuccessneverrequestsplayer/media.
SameapprovedfixedwatchGET/privacy/restrictionbounds, no new access/auth/persistence
scope. SeeRUST_DIRECT_PROTOTYPE.md latestdebugbuild+singlewatchonlycommand. No newREA
execution/runtimeintegrationorclaimofnativeextraction, providerfixor15MBcompliance.


## Host v4.2 identifies strict JSON syntax category, not schema mismatch

Host1bae4b4:28tests/debugPASS; watch_config_json_syntax, no player/media,10848KiBdebug
HWMnotproductionbudget. Existingdiagnostictoo coarse; nextv4.3samewatch-onlymode
reportscomprehensivefixedsyntaxsubtypes, no repair/evaluation/rawdata. Upstream
strictFalsedecoderfixturecontrast (rawcontrols/NaNaccepted,hexescapes/trailingcommas
rejected) isresearchonly, nothostrootcauseorpermissionforblanketleniency. See latest
RUST_DIRECT_PROTOTYPE.mdcommand/failurebranches. Oldv4.2diagnosticnotrequestedagain.
No newREAexecution/dependency/auth/persistence/provideraccessscope/productionchange.


## Host v4.3 expected-value failure; conditional metadata compatibility, not JS execution

Host127817ddebugPASS; expected_value atwatch/no player/media/10964KiBdebugHWM. Exacttoken
unknown. UpstreamfixturechecksacceptNaN/Infinity/-Infinity,rejectundefined. V4.4inert
scannerhandlesonlyexactconstantsindiscardedconfigmetadata, protectsvisitor/auth/context
subtreesincludingescapedkeys, retainsstrictSerdevalidation/capsandnoJS. Unsupported
JSvaluesnowfixedsubtypes. Extraownedbufferupto1MiBwhencompatibilityisused, no wholeRAM
claim. Nextcommandwatch-inspectONLY, notbootstrap/playertrial. Seelatestdirectguide
forprotected/nonfinite/unsupported/passbranches; no assumedhistoricalrootcause. No new
REAexecution/deps/credentials/persistence/endpoint/provider-accessscope/productionchange.

## V5: consolidated data-only reader, not another quote exception or REA runtime

Host5142200 stopped at single-quote-leading config value before player/media. User
requests the remaining issues fixed. Replaced incremental config_numbers/framing with
literal_data.rs supporting bounded static strings/escapes/keys/containers/comments/
trailing commas while rejecting execution and retaining typed privacy/restriction/
identity/duplicate/auth guards.46Rust tests include a complete synthetic pipeline;
source4cf5566 CI43284047821 all5jobs PASS. No live v5/provider/codec/budget acceptance.
Current direct guide now contains one next full native trial and a dependency-ordered
plan for audio/search/controls/complete memory/production, with historical commands
archived. No new REA run/dependency or whole-app Rust/10–15MB claim.

### V5.1 exact static wrapper, not generic function traversal

User host39dd9fe compiled1.64s but watch_visitor_missing before player/media. Archive
c9b17c81855981ed898636ced2ba57c3a85a6eba main config lives inside an IIFE; prior top-level
reader skips it. This is reproduced coverage evidence, not proof of host shape.
V5.1 matches only the fixed straight-line anonymous parameterless IIFE/config/optional
localization scaffold and complete immediate invocation, never executes/searches general
functions. Adds only value-free source counts/visitor-presence flags; no generated value,
account/session/token provider/REA production dependency.55Rust tests/sourcea169b48
CI43288159691 all5PASS, actual acquisition/audio/full10–15MB still blocked. Current
direct guide supplies ONE changed bounded trial and explicit absent-data/unsupported-
wrapper/restriction stop branches; no unchanged v5 retry or separate inspect request.

### V5.1 trial consumed; Graphify development source map does not unblock acquisition

Host04e3f8b debug3.16sPASS/watch_visitor_missing before player/media; config1/player1,
no visitor in recognized sources, ignoredtails35/wrapper0. Archived wrapper assumption
not confirmed, no unchanged request advised. User suggested Graphify; actually ran
pinned52cb28218eea0c68d80a9f4b2ae102d2fa63e267/graphifyy0.9.108 isolated code-only,
backendnone/querylogdisabled,197nodes447edges. See research/GRAPHIFY_SOURCE_REVIEW.md
for where to inspect prefix/schema/attribute policies and query limitations. No REA or
Graphify runtime integration/fallback, no current-page inspection or visitor generation.
Evidence guard now consumes trial/stops stale recommendations; offline review only,
83PythonPASS, nativeRust unchanged55tests/all5CI43289003239PASS. Acquisition2/5,
codec/controls/search and complete10–15MB gates remain unpassed.

### Offline guard confirmed; bounded coverage diagnostic prepared, no new live approval

User hostefde115 guard behaved asintended: blocked,nonetwork/nonative,nextofflinereview.
Pinnedupstreamplayerflow uses same3visitorpaths acrossgenericargs; Rusttypedassociation
is narrower. ytInitialData notpassedthere, andVISIONOS absentfromconfigpagemap. No new
samepagesource/providerfixestablished. Verified_upstreambytes atpin12ac9aca; noREAor
Graphifyrerun/runtimeintegration. Preparedwatch-onlyrawmarker/prefixregioncounters,
notJSexecution/dataparsing/sourceacceptance. Source02e1173 CI43415804715 all5PASS,
62Rust/84PythonPASS. Explicitapprovalstillrequired forONEextrawatch-onlyGET; guardstop
remains. See research/WATCH_COVERAGE_DIAGNOSTIC.md, no newnative/livecommand yet.

### Explicit real-check request: one bounded coverage GET and current-service budget audit

Userasksactualdownload/play-streamwithin10–15MB, notmorefixture-onlystatus. Record
approvalforpreparedv5.2watch-onlyGET plusreadonlylocalserviceaudit,noplayermedia/
newsource/auth/deployment. Guardnextapprovedwatchonly,nativegateblocked. Newdiagnostic
servicecgroup15MBand15MiBcurrentthresholds (notlegacy100MB), separatehistoricalpeak/
swap/helperRSS andplayingflags; neverclaimswholeRust/audioacceptance. All5CI43417518725
PASS/source891b1c9,62Rust/100PythonPASS. Noagentliveattempt/targethostbudgetresultyet.
CurrentorderedcommandsinSTREAM_BUDGET_CHECK.md; nativebootstrapstillnotretried,
normalproductionunchanged,extraction2/5,10–15MBstillunproved.
