# Bounded native direct-player experiment

Status: **BLOCKED: v2 failed on host+CI; opt-in v3 also returned player_bot_check
on the actual Linux host. Stop both variants; no native media bytes obtained.**
Compilation/offline tests pass, but source acquisition is not resolved. The build
commands below are retained for reproduction, NOT the recommended next action.
No working production replacement or 15 MB claim.
The old rusty_ytdl/Boa candidates remain failed for the required host/track. This
experiment does not erase that evidence or add a hidden fallback. Production5050,
UI/Vercel, root dependencies, installer and Rodio remain unchanged.

## Why a separate experiment

User approved further fixing/building and asks for terminal compile instructions.
Instead of another large extractor revision, prototypes/rust-direct uses reqwest0.12
(defaults off, rustls/json), single-thread Tokio, Serde/serde_json. No rusty_ytdl,
Boa, QuickJS, Node, Python, yt-dlp, mpv, browser engine or subprocess in this path.
It is Rust application code with ordinary native TLS/OS dependencies, not a claim
that every cryptographic implementation or OS library is written in Rust. It does
not include search, related tracks, full downloading, decoding or playback.

Current v2 candidate uses one fixed HTTPS public-player POST with VISIONOS context,
client version1.02, header client ID101 and matching agent/device/OS metadata. The
previous v1 used ANDROID_VR1.65.10; it is retained in Git history, not selected now.
The mechanism remains no HTML/config/visitor extraction, no downloader parser or JS.
See the compatibility correction below for exact source references and limitations.
No token generation, credential collection, sign-in, age/content-check flags or
restriction bypass is implemented. If the response requires unavailable authorization,
signatures or tokens, fail honestly. A client profile is not a compatibility guarantee
or an exemption from provider controls. No extractor implementation code is copied;
these are protocol configuration facts used by independently written request/parser.

## Bounds and safety

- Strict11-character ID; no arbitrary metadata URL/endpoint input.
- HTTPS-only, no proxy, no redirects; fixed player endpoint, no credentials/cookies.
- Read at most1MiB metadata into an owned byte buffer; oversize/chunked overrun fails.
  Parser skips unknown fields with typed structures. HTTP/TLS internals/allocations
  are not proven heap-bounded by that1MiB payload limit.
- Require playability OK, matching video identity and non-live content. Former live
  content is conservatively rejected too. No sign-in/age/private access is attempted.
- At most256 adaptive formats accepted after parsing; parsing occurs within metadata
  byte cap. Pick highest bitrate AAC/mp4a.40.2/audio-MP4 direct URL, no video dimensions.
- Cipher-only entries fail signature_required_unsupported. Missing URLs fail honestly.
- Media URLs limited to HTTPS googlevideo.com subdomains, no credentials/alternate port;
  no redirects. Hostname validation is not a fully hardened public proxy/SSRF service;
  the CLI is local only, no server endpoint exposed. DNS/network/library behavior still
  needs review before any public integration.
- Read/validate64KiB MP4 prefix and discard it. Require ftyp signature at bytes4..8.
  This is not a full MP4 integrity/decode/seek/audibility test. Range ignored by upstream
  can cause additional buffered/network bytes before cancellation.
- Connect10s, per-request20s, overall60s cooperative deadline plus recommended75s
  external kill deadline. No retry/cache/file output. No URLs/response bodies/provider
  reason text logged; only fixed codes/timings/itag/process HWM.
- Code0=prefix succeeded,1=probe failed,2=arguments invalid. No fallback.

## Build on the actual Linux host

Only after CI compile/tests pass, use the new package path (NOT rust-extract):

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
cargo test --manifest-path prototypes/rust-direct/Cargo.toml &&
cargo build --release --locked --manifest-path prototypes/rust-direct/Cargo.toml &&
timeout --kill-after=2s 75s \
  ./prototypes/rust-direct/target/release/aysha-direct-prototype \
  --video-id Jm7Satp7Zqk
```

No sudo/install/service restart, old lock deletion or OS changes. First cargo test
creates this new package's own lock; release uses --locked. Preserve it locally.
Build artifacts ignored. Cargo lock snapshot retained as CI artifact; transitive
lock is not committed. Compile/download RAM/size is not runtime memory. Pause audio
before performance measurement to reduce contention, without restarting the service.

If the result fails, retain that as a failed experiment. Do not replace production.
If it passes, record cold/warm successes and same-format/host comparison including
full service cgroup before performance claims. The existing benchmark harness still
selects the old rust-extract binary; it does NOT benchmark this candidate. Do not
mislabel its old results. Full song/audio/production integration is not implemented.

## CI evidence model

Job rust-direct-prototype runs six offline tests, release build, CLI validation and
asserts the resolved graph has no extractor/JS-engine crates. A bounded public probe
of the already user-tested ID is recorded as a GitHub notice, not an acceptance gate.
Its process exit and JSON status remain explicit even when upstream fails. Green
build job means code/tests passed, NOT a successful network observation. Cloud runner
IP/region may differ from the actual host. No network result can certify the user's
hardware/aggregate15MB budget. No server secrets or private track corpus is uploaded.

## Recovery and limits

No production rollback required: don't run/integrate this isolated binary. The old
experiment remains for reproduction only. A native direct request cannot decode
signatures, generate integrity tokens, bypass provider restrictions or implement the
whole maintained YouTube extraction ecosystem. If that is required for the accepted
workload, the Rust-only and15MB goals must be evaluated together, not solved by
moving workers outside accounting or quietly enabling yt-dlp/Node fallback.

## Actual first verification outcome — do not call this fixed

Sourceb845189, GitHub Actions43264608690: all5code/regression jobsPASS, including
six direct-prototype unit tests, release build, CLI, noJS-engine dependency audit,
existing root/audio/browser suites. The public compatibility observation separately
FAILED: candidateaysha-direct-v1-android-vr, codeplayer_not_playable, process_exit1,
VmHWM4088KiB (~4.19decimalMB). The low failed-request memory is NOT working extraction
or a15MB full-service acceptance. Provider did not return playabilityOK on that cloud
runner; no raw reason was logged, so do not infer bot/IP/auth/region/video removal as
confirmed cause. Actual Linux-host behavior is unknown, not silently assumed better.
Do not promote this experiment or ask for another home rebuild as a guaranteed fix.

For the user's request to compile the best currently verified working server, use
root package (which still uses the existing yt-dlp/Node/mpv production helpers):

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
cargo test &&
cargo build --release
```

Output target/release/music-server is compiled only; it does NOT replace/restart the
installed service. Do not launch a second5050 instance while production owns the port.
This is not a new Rust-only extraction fix and does not claim100MB or15MB compliance.
The direct-package build command above remains an optional experiment/reproduction,
not the best working production build. Avoid confusing these paths or invoking the
installer merely to compile. Native extraction-to-playback migration remains blocked.

## v2: researched client correction and rejection classification

Found a concrete missed compatibility detail: inspected yt-dlp _base.py explicitly
comments that AndroidVR1.65.10 formats have been403 since2026-08-17. _video.py selects
VISIONOS as its current _DEFAULT_JSLESS_CLIENTS, not android_vr. The prior research
read configuration values without acting on surrounding deprecation comments. This
was an avoidable candidate-selection error, not a reason for another unchanged build.
References inspected through GitHub API (configuration facts only):

- https://github.com/yt-dlp/yt-dlp/blob/12ac9aca79671420724227aca37bc888c604a6f8/yt_dlp/extractor/youtube/_base.py
  — android_vr deprecation and visionos context/client version/header ID.
- https://github.com/yt-dlp/yt-dlp/blob/master/yt_dlp/extractor/youtube/_video.py
  — current JS-free default; child-directed-content limitations remain.
- https://github.com/yt-dlp/yt-dlp/blob/master/test/test_youtube_misc.py
  — public reference video IDBaW_jenozKc is used in upstream tests.

v2 identityaysha-direct-v2-visionos uses that client profile, no extra dependencies,
no retries/fallback/auth/provider-restriction flags. Public/identity/live/URL/byte/time
bounds remain unchanged. This is a mechanism correction supported by source, not proof
of source acquisition on the target host. No claim that the prior CI player rejection
was specifically caused by the old profile before the new observation is obtained.

Playability errors now distinguish missing/unknown status, login required, age/content
check, unplayable, live offline, generic provider error and explicit English bot-check
text signal. Raw reason/status/messages are parsed but never echoed. Localized/unknown
messages do not become guessed categories; machine status remains available as a
fixed code. An OK response still must pass identity/non-live/valid URL/media validation;
no restriction is satisfied automatically. Eight offline tests include classification
and privacy cases. CI observes the same requested ID plus one public reference,
sequentially (maximum2requests), each in its own bounded process, no playlist/corpus
scraping. Results are explicitly separated by case. Both failing in CI does not prove
all tracks/regions/hosts fail. No new target build is requested solely to rename errors.

### v2 live result: cloud bot check, not a solved extraction

Source4984f54 CI43267932699 all5build/regression jobsPASS, including8direct offline
tests/release/noJS dependency audit. Separate observations:

- requested track: fail/player_bot_check, VmHWM3976KiB, process_exit1;
- reference track: fail/player_error, VmHWM4008KiB, process_exit1.

These are fixed classifications, not raw provider text. Requested case indicates an
explicit English bot-check signal on that runner; it is not proof a native JS engine
or another client would solve it. Reference generic error remains undiagnosed.
Client correction did not establish successful audio acquisition. Bot classification
is a rejection, not a challenge solver, and no bypass/credentials/tokens are added.
The old generic player_not_playable did not give this evidence. Low RSS is still a
failed-request measurement, not15MB working media acceptance. No faster claim.

Actual Linux-host v2 has not run (the earlier c1de8e0 root build did NOT build this
separate package). One optional current direct-package probe using the build command
above is materially different from repeating either rusty_ytdl candidate or the old
AndroidVR request. It distinguishes target-network behavior from CI. No installer,
service restart, package deletion or provider workaround is needed. If it reports
player_bot_check too, stop automatic retries/client roulette: acquisition under the
current unauthenticated/native scope is blocked. Do not hide an external downloader
or move extraction to another host to claim Rust-only15MB compliance. Keep production
working and report need for a supported/authorized source-access design. If it succeeds,
verify actual decode/audio and representative resource workload before integration.


## Target-host v2 outcome: bot-check blocker confirmed

User's home-host output at412d4c7: eight testsPASS; release buildPASS (52.86s); native
v2 resultplayer_bot_check, VmHWM4404KiB (4,509,696bytes, about4.51decimalMB), fail,
production_readyfalse. This confirms the classifier found an explicit bot-check text
signal in that response; it does not prove the exact challenge mechanism, why this
connection is challenged or that every provider client/account/track is blocked.
There was no media URL/sample success, no native decode/audio, and no working15MB
acceptance. Build dependency/download volume is distinct from runtime RSS.

This satisfies the agreed stop condition: do not request another identical compile,
rotate profiles/IPs, disable security, hide an external downloader or move extraction
outside memory accounting to claim success. No credentials/raw cookies/tokens should
be pasted into chat. Existing production stays on its current helpers, unchanged.

The next design gate is source access, not Rust compilation. Evaluate a provider-
supported/authorized access path and its playback/redistribution constraints before
further YouTube integration; do not assume an API supplies raw audio. If YouTube is
mandatory, keep the currently working production path while native access feasibility
is unresolved; it does not meet Rust-only/15MB. For proving native decode/playback
without this upstream dependency, explicitly approved owned/local audio or permitted
direct media URLs can be a separate test workload, not a replacement of the user's
YouTube requirement or evidence that extraction was solved. No source/auth/backend
architecture change is approved by this checkpoint. No installation is needed.

## Next bounded diagnostic: existing public AAC baseline, no native retry

After hostv2bot-check, establish whether the existing tool can currently read the
same sample on that host, rather than assuming earlier2/3success still holds:

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
python3 scripts/benchmark_extract.py --baseline-only --video-id Jm7Satp7Zqk --runs 1
```

This is explicitly Python diagnostic tooling invoking existing yt-dlp/Node for the
baseline, NOT a Rust server change or a native success claim. No compilation/native
request/install/restart/production API calls. --ignore-config/no cookies/no proxy,
no cache and same AAC/MP4 constraint as prior comparison; consequently not an exact
reproduction of any custom production config. Reads64KiB, validates MP4 header, kills
and reaps only its own process group;75sdeadline. Raw URLs/provider stderr discarded.
Unique private mode0600 report under ignoreddiagnostics/, printed for manual review.
Upstream ignoring Range may transfer extra network buffers; no full-song claim.
One run is a point-in-time diagnostic, not reliability/performance acceptance.

- PASS: public AAC sample is obtainable by the baseline on this host for this attempt.
  Next inspect supported source-access/protocol capabilities missing from the native
  implementation before selecting a concrete patch. Do not conclude the cloud/host
  network is never challenged or that the exact native root cause is proven.
- FAIL: do not repeat/build native candidates. Investigate the baseline failure layer
  (tool/runtime, network, format availability or provider rejection) with a separate
  bounded diagnostic; this alone does not prove a service-wide outage/bot cause.
- Rust attempts0/native_executedfalse is intentional, not missing test coverage.
  No successful native speed/memory comparison took place in baseline_only mode.

No native extraction fix is claimed by this diagnostic addition. Keep production
unchanged while source access remains unresolved. No new account/token request or
provider bypass is approved by this plan.

REA was actually used for a public-source static audit after the successful host
baseline. See [REA_NATIVE_EXTRACTION_PLAN.md](REA_NATIVE_EXTRACTION_PLAN.md) for evidence,
limitations and the new --baseline-only --trace-stages diagnostic. It does not rerun
this failed native request, install REA on the host or claim signature transforms
solve a bot check. No native source-acquisition fix has yet been demonstrated.

## Opt-in v3 standard-context experiment — CONCLUDED BLOCKED on host

Host offline profile at731166c: CLI and imported package both2026.08.19; VISIONOS1.02,
Apple/device/OS/UA/clientheader101 match nativev2. Generated defaultcontext adds
UTCtimezone/offset and headerbuilder addsOrigin. Emptyofflineconfig has no visitor
header and REQUIRE_JS_PLAYERfalse. Matchingversionstrings do not prove binaryidentity
or capture the actual successful request. No network/native execution in that check.

Concrete bounded variant: `--standard-context` selects identity
`aysha-direct-v3-standard-context`. It adds ONLY `Origin: https://www.youtube.com`
to the player POST and `context.client.timeZone="UTC"`, `utcOffsetMinutes=0` to its
JSON. Without that flag, legacyv2 remains unchanged (and should not be retried).
Unit tests compare built requests without sending: URL/method/all otherheaders and
bodyfields match v2 after removing those additions. No dependency/client/format change,
watchbootstrap/session/visitor/consent/auth/cookie/POtoken/restrictionflags/JS engine,
media-header change, redirects, fallback, installation or production integration.

This is one low-scope experiment justified by confirmed host defaultdifferences, not
proof that thosefields caused earlierfailure. Not a one-variable causality test:
threefields change together and upstream time/network/challenge state may change.
Earlierv2 metrics are historical, not a simultaneous controlled comparison.

### Historical reproduction only — DO NOT RERUN after the recorded host failure

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
cargo test --locked --manifest-path prototypes/rust-direct/Cargo.toml &&
cargo build --release --locked --manifest-path prototypes/rust-direct/Cargo.toml &&
timeout --kill-after=2s 75s prototypes/rust-direct/target/release/aysha-direct-prototype \
  --video-id Jm7Satp7Zqk --standard-context
```

Keep the existinghostCargo.lock; no lock deletion, dependencyupdate or fullrootbuild.
If --locked reports a missing/outdatedlock, stop and share the error; do not update
productiondependencies as a workaround. BuildCPU/memory is not runtimeRAM accounting.
Expected:11offlinetests and releasebuild, then one fixedJSON with candidatev3identity.
PASSrequires65536MP4prefixbytes; production_readyremainsfalse. PeakRSSis this isolated
process only, not the server+decoder/search/listenerbudget. There is no batch/reference
request/retry or JS/yt-dlp fallback. Timeoutprocess exit124 may lackJSON; report that.
Do not run this benchmark concurrently with another resource test.

If player_bot_check/login/otherrestriction persists: stop this variant, retain failure,
no repeated trial/flagroulette. Next missingcontext investigation is watchbootstrap
versus standalonePOST, requiring explicit scope review before introducing anonymous
visitor/sessionhandling. If signature_required_unsupported appears, we have reached
a different stage; evaluate boundednative transformneeds rather than declare success.
IfPASS: preserve evidence, then separately design media/decode/control/wholebudget
acceptance; do not install or attribute the success solely toOrigin/UTC.

CI now performs offlineverification only for this package. Removed automaticlivev2
requests on everypush because their agreedhost+CIstopcondition was already reached.
No v3live request made inCI; the target-hosttrial above is opt-in and singleattempt.

Verification: sourcec751075 CI43272722794 all5jobsPASS; isolateddirecttests/release/
noJSdependencygraph/invalidopt-inCLIpassed. Local68PythonregressiontestsPASS. Agent
has no localCargo; CIresults are not targethostacquisition/playback proof. V3live
acceptance pending the explicit oneattempt command above.

## Latest host outcome and stop gate

User host atb2efd81:11/11testsPASS, releasebuild17.66sPASS;
`aysha-direct-v3-standard-context` -> `player_bot_check`, peak4404KiB
(4,509,696bytes), production_readyfalse. No media bytes obtained.
Origin+UTC additions were insufficient for this attempt; neither field-level
causality nor a provider-side rootcause is established. Same v2/v3 rejectioncode
is not proof that the provider received/evaluated both requests identically.

No further v2/v3 trial, rebuildloop or JS-engine port is justified by this result.
Machine-readable supplied evidence: research/NATIVE_ACQUISITION_HOST.json.
Offline review command `python3 scripts/check_native_acceptance.py` returns
`status: blocked`, exit1 **intentionally**. It never launches a native executable,
request, service, build or package install; memory and whole-stack gates stayfalse.
New regression tests ensure successful baseline/build/lowRSS cannot override a native
rejection. The checker is a historical evidence gate, not independent media validation.

### Next proposed scope — explicit approval required, not implemented

Publicsource12ac9aca... `_extract_player_responses` obtains anonymous visitor context
from watchconfig/initialplayer; `_extract_visitor_data` selects VISITOR_DATA,
INNERTUBE_CONTEXT.client.visitorData or responseContext.visitorData;
`generate_api_headers` can send X-Goog-Visitor-Id. This is an upstream capability
absent from our standalonePOST, not proof it was used or sufficient on user's host.
Do not collect HARs, headers, cookie exports or visitorvalues in chat.

Proposed single isolated Rust experiment: one publicwatch GET to fixedYouTubeorigin,
read at most2MiB into boundedbuffer, extract boundedanonymousvisitorcontext (max4KiB)
without executingJS; retain inRAM for this one invocation only; use only with same
fixedoriginplayerPOST, then at mostonevalidated64KiBmediaprobe if player permits.
No cookiejar/accountlogin/consentacceptanceautomation/JS/POtoken generation or capture,
proxy/clientrotation/persistenttracking/sessionreuse/fallback/productionchange.
No raw page/visitorvalue/signedURL logged or saved. HTTPS/no redirects;60stotaldeadline.
Retainallidentity/live/restriction/origin/format/metadata bounds. Consent/challenge/login/
missingcontext/oversizepage -> fixedfailure, no workaround or secondtry.

Before live use: offline synthetic fixtures for JSONescaping/stringboundaries,
ambiguous/missing/oversizedvalues, hostileHTML, originvalidation, no-valueleakage and
cancellation. One host attempt only after tests/buildpass; repeatedrestriction stops.
This changes source-access/sessioncontext scope and needs approval; no implementation
or hostcommand for it is supplied yet. Even if approved and successful it is NOT
whole-app Rust/15MB or proof that all provider access requirements can be supported.

## Approved v4: bounded anonymous watch bootstrap

User explicitly approved the bounded experiment via the scope choice. This supersedes
only the approval-pending proposal above; v2/v3 remain stopped, not automatically retried.
Opt-in flag `--watch-bootstrap` selects `aysha-direct-v4-watch-bootstrap`.
No production integration and no automatic live CI requests.

Flow: fixed-origin public watchGET -> conservative inert literalJSON extraction ->
validate initialplayer playability/identity/nonlive -> same VISIONOS/Origin/UTC POST
with the opaque anonymous value in `X-Goog-Visitor-Id` -> existing validatedAAC64KiBprobe.
Header goes only to the fixed playerendpoint, not media. No value in JSON/logs/files.
V4doesnot put visitorData in the requestbody; this matches the inspected upstream
header-builder path, not a claim of fullrequestequivalence. UnknownJS/protocolhandling
is deliberately unsupported. No externalhelpers/credentials/cookies/consentacceptance/
POtoken/signaturegeneration/JS/proxy/redirect/clientrotation/persistentsession/fallback.

New bootstrap.rs accepts literal yt cfg calls (literalname `ytcfg.set(...)`) and
literal `var ytInitialPlayerResponse = {...};` or bareassignment at inline-scriptstart,
or immediately after another acceptedliteral. Does not search arbitraryJS strings,
comments/functions/templates or evaluatecode. Handles quote-awaretagend, rawtext and
HTMLcomments/templates; excludes external/module/unknownattributedscriptmodes.
This is a conservative HTML/JS subset, NOT a complete browserparser; unsupported
wrappers/locale/markup can return missing/invalidcontext and stop. Staticpagepatterns
cannot establish absence of every providerchallenge. Recognizedconsent/challengemarkers,
initialplayerrestrictions/auth, ambiguous/differentvisitorvalues, duplicateplayerJSON,
missingidentity/context, malformedJSON/headercharacters failclosed. No bypass/retry.

Caps:2MiBwatchpayload/1MiBeachliteralJSON/64JSONnesting/4KiBacceptedvisitorvalue;
1MiBplayerpayload/256formats/64KiBmedia. TypedSerde skipsunknownfields; JSONstrings
can allocate up to the literalpayloadcap before visitorlengthvalidation. Reqwest/TLS
internalbuffers are not a provenheapcap. Fullwatchresponse is dropped before player
request; visitorobject dropped after playerread. Dropping is NOT securezeroization;
requestheaders can have ownedcopies and transientTLS/HTTPbuffers. No persistence or
cross-invocationreuse. Wholeasyncflow remains60s with20s/request+10sconnect limits;
external75stimeout recommended. Timeout drops the future and ownedstate (offlinetest),
not proof of all realnetwork teardownpaths. FailureJSON includes fixedstage init/watch/
player/media; rawmetadata/reasons/headers/values neverechoed. No success inferred.

Offlinefixtures cover opaquevalues, escaping/JSONstringboundaries, missing/conflicting/
oversizeddata, identity/login/live/consent, inertHTML/JSstrings/comments/rawtext/template/
attributecontent, malformed/duplicateJSON, explicitmodeCLI, exactheader/bodydelta,
no leakage to mediaheaders, deadline cancellation and ownedstatedrop. CI verification
must pass before the explicit hosttrial. Historical evidencegate remainsblocked until
newactualhostevidence is supplied; approval/codebuild alone doesnotchange acceptance.

### V4 verification and one approved host trial

Source7d0890c: CI43274748320 all5jobssuccess, directjob131380060168 tests/release/
noJSdependencygraph/strictCLI+invalidbootstrapCLI allpassed. Sourcehas21Rusttests
(11retained+7bootstrapfixtures+3mode/header/cancellationtests). No agentlocalCargo;
verification isCI, not localRustbuild/targethostsuccess. No livev4attempt made here.

**Terminal: BUILD + ONE APPROVED NATIVE DIAGNOSTIC — no install/restart**

```bash
cd /home/afine/ayshamusic &&
git pull --ff-only origin arena/a850fb0d-ayshamusic &&
cargo test --locked --manifest-path prototypes/rust-direct/Cargo.toml &&
cargo build --release --locked --manifest-path prototypes/rust-direct/Cargo.toml &&
timeout --kill-after=2s 75s prototypes/rust-direct/target/release/aysha-direct-prototype \
  --video-id Jm7Satp7Zqk --watch-bootstrap
```

No standalonev2/v3 retry. Do not add --standard-context; modes are mutuallyexclusive.
Existinghostlockpreserved/no dependencyupdate. Expected21offlinetests thenbuild and
oneJSONcandidateaysha-direct-v4-watch-bootstrap. PASSrequires65536MP4prefixbytes;
production_readyfalse. Failures includestage and fixedcode. Do not collect/share raw
pagecontent, visitorvalues, cookies, signedURLs, HTTPtraces or debuglogs.

- watch_size_limit/player_missing/visitor_missing/markup/literal errors: stop and
  sharefixedJSON; parser/protocolsubsetneedsreview, not retry or capincrease by default.
- watch_restriction_detected/player_bot_check/login/age/content/auth: stop; no workaround,
  account export, generatedtoken or secondattempt. Recordexactstage; bootstrapmayitself
  beblocked, or permittedbootstrapmaystillnotyieldplayeraccess.
- signature_required_unsupported/format/mediafailure: newstageevidence, not success;
  investigateboundednextcapabilityseparately, neverfallback toPython/Node/yt-dlp.
- PASS: prefixonly, not decode/control/audiooutput/search/productionorwholeRAMproof.
  Separatelyplanactualplayback/completeaccountingandrepresentativepermittedinputs.
- Build/lockfailure: stop/shareerror withoutdeletinglock/upgradingproductiondeps.
  Exit124/noJSON: recordtimeout; no loop. No servicesareinstalled/restarted.

Offlinehistoricalevidenceguardstillreturnsblocked until actualnativehostsuccessis
recorded; its nextactionnowreflectstheapprovedv4scope, not a freshapprovalrequest.
