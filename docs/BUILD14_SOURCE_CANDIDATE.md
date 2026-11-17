# Build14 — installable source-repair candidate

## Download — built and available

**[Download Aysha Android Audio14 APK ZIP](https://github.com/AfzalAshraf/ayshamusic/actions/runs/44141628349/artifacts/14354662898)**

Unzip and open the APK on the phone. Source/receipt **2928821**.
Android44141628349 **SUCCESS**: release134245832199 and emulator134245832747.
Linux44141628223 **SUCCESS all6jobs**. ZIP4,972,583bytes,
SHA256`e1213a3743f83d0185011c9fdfe8719892f25cae367d32e301f04ccda96c4170`.
Artifact metadata verified, not local binary inspection or stable signing proof.
Five actual Media3 fixture tests pass, including600s@4x and10s source gap;
separate from Rust/provider/JNI. Safe annotations:
`docs/research/build14-media3-emulator-result.json`. Complete affected-song and
physical-phone playback are **not yet verified**; this is an installable candidate.

## Why this APK exists

User explicitly requested an installable app to check, not another research-only
report. A new candidate is justified by the exact build13 screenshot: ANDROID
and IOS head reads succeeded but preparation tail reads returned403 on Suno
Aisha/SonyMusicIndiaVEVO, video `fAzO3s7Dgnw`. Timeout/decoder fixture tests cannot
fix that denial. This candidate changes actual source acquisition; it is **not
yet confirmed fixed on the phone or the affected public song**.

## Actual production change

- Media3 APK's default acquisition is one VISIONOS1.02 public-client profile,
  reviewed from MIT youtubei.js18.3.0 `Constants`/`HTTPClient` protocol fields
  (ID101, matching context/UA). Not a full YouTube.js library integration; no
  JavaScript runtime, NewPipe/GPL library or signing interpreter added to APK.
- No ANDROID→IOS→TV rotation in normal Media3 playback. A provider sign-in/bot
  challenge stops acquisition. No credentials, token generation, proxy, account
  bypass or identity retry after denial. Existing visitor ID is reused.
- Matching video ID required, direct AAC-LC/audio-only known-length streams
  only. Unsupported cipher/n signing, manifests, DRM, live/private/upcoming,
  unsafe URLs and excessive length are rejected, not disguised as playable.
- Metadata responses are bounded2MiB/12s; metadata redirects are refused.
  Candidate error bodies and request URLs are not exposed in diagnostics.
- Actual Rust media pipeline and Media3 remain unchanged: head **and tail**
  validation, request-specific redirects,64KiB range blocks, seek/long-track
  handling and20s Media3 inactivity budget retained. No validation removed.
- Telegram archive acquisition and selftest use the same default profile policy;
  Telegram remains Rust and user-owned. No new claim of login/download/upload.
- Legacy non-Media3 library defaults stay unchanged. Main Linux server mode,
  player UI/icon/dedication/Settings/Telegram choices untouched.

## Candidate identity

- Aysha Android Audio14, versionCode14 /1.3.6-preview, Android8+/ARM64.
- Package remains `com.afzal.ayshamusic.androidaudio10`, port17652.
- No server/helper/Node/Python runtime required. No GPL integration/relicensing.
- Preview signing is not a stable production signing solution. Do not instruct
  blind uninstall if update is refused: existing local Telegram/settings could
  be lost. The sandbox integration could not list repository secrets (permission
  denial), so do not claim an existing release signing key was verified.

## Evidence and boundaries

Local108Node/142Python PASS. Added real local-HTTP Rust fixture tests for exact
request context/headers/visitor, no credentials, direct response parsing,
metadata size/redirect/HTTP denial/error redaction. Added policy tests for one
Media3 profile and terminal challenges. CI compile/host/emulator/release now
**PASS**, with run identity recorded above. No new live provider research run: earlier challenge
allowance remains sealed. No fallback from that challenge through this profile
was executed in CI. Offline fixture proof is not public source-access proof.

The user can check this source-repair hypothesis with an installable candidate;
it must not be advertised as a verified playback fix. Download/artifact and
actual CI result are recorded above; they do not certify public source access.

## Phone check after download

Open the new Audio14 launcher/receipt, select the exact Suno Aisha song from
SonyMusicIndiaVEVO, and play. Successful phone acceptance means audible sound,
progress beyond the old failure and completion of the song (then background/
pause/resume checks). If it fails, stop rather than repeatedly pressing Play;
the error/receipt distinguishes a provider challenge, unsupported direct source,
head/tail403 or actual Media3 failure. Never send Telegram credentials in chat.
