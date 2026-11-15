# Spoken conversation package

## Decision before code

Decision: modify existing settings/secret/session boundaries; add voice-only
actors and a desktop audio owner because none exists. ADR 0080 is proposed,
with architecture review required before product code. ADR 0067's owner was
consulted: only PCM/WAV/TLS/secret primitives are potential shared assets;
its stream commit/prompt/confidence policy is not a voice API.

## Reference behaviour read (Paseo 0.15.4)

Paths below are relative to the reference repository, not copied sources.

| Source | Lines | Behaviour |
| --- | --- | --- |
| packages/app/src/voice/voice-runtime.ts | 13-24, 525-540 | cue timing and display smoothing |
| same | 706-810 | old target disable, initialize, node enable, capture; rollback/stop |
| same | 557-580, 662-704, 900 onward | reconnect resync, grouped playback, barge-in |
| packages/app/src/components/realtime-voice-overlay.tsx | 17-110 | 44-point mute/stop, meter, switching disables actions |
| packages/app/src/screens/settings/voice/voice-page.tsx | 43-165 | offline/unsupported gates; dictation and voice sections, per-site editor |
| packages/server/src/server/speech/speech-config-resolver.ts | 39-49, 68-120 | local providers; dictation on, voice off |
| packages/server/src/server/speech/providers/openai/config.ts | 8, 25-65 | site resolution, tts-1/alloy/PCM defaults |
| packages/server/src/server/speech/providers/openai/stt.ts | 195-232 | temp WAV; logprobs for two known models; mean threshold -3.0 |
| packages/server/src/server/session/voice/voice-session.ts | 354-450 | readiness, target agent, voice enable/disable and response |
| packages/server/src/server/session/voice/voice-turn-controller.ts | 14, 16-45, 69-104, 136-185 | final deadline, filler partials, typed input phases, confidence |
| packages/server/src/server/speech/providers/local/sherpa/silero-vad-session.ts | 10-24, 75-119, 206-263 | 16k/512/0.5; 800 ms confirm, 1000 ms ending |
| packages/server/src/server/agent/tts-manager.ts | 30-32, 107-120, 150-245, 350-420 | 260-unit segments, two prefetched; per-segment collected PCM and receipts |
| packages/server/src/server/speech/providers/openai/tts.ts | 25-30, 59-83 | tts-1/alloy/PCM; complete response collected before playback |
| packages/server/src/server/voice-config.ts | 1-62 | voice prompt replacement/disable and spoken-input wrapper |
| packages/server/src/server/speech/providers/local/sherpa/model-catalog.ts | 256-267, 358-373 | default STT/TTS model ids and archive sizes |

The area-2 composer survey section 7 and Settings survey part D's Host Voice
section are the inventory. Source overrides any shorthand in those notes.

## Differences and reasons

- Typed RPC long-poll replaces the reference's unsolicited voice messages:
  the node has one versioned table, not a second frame protocol.
- Named resource caps, explicit overload/refusal, finite provider and receipt
  waits: a multi-user node cannot retain unbounded audio or pending speech.
- Generation-bound explicit reconnect replaces automatic enable resync:
  no implicit second microphone/provider/agent lease after uncertain cleanup.
- Credentials use the existing secret store, read presence only. No raw
  config key, process environment override or displayed secret name. One
  credential owner prevents accidental export and cross-site reuse.
- No audio files, debug dumps, text previews or upstream error logging:
  privacy floor over the reference's temporary WAV/debug diagnostics.
- ALSA/WASAPI plus explicit software AEC replaces WebAudio/Expo:
  GPUI does not supply browser/mobile audio engines. Hardware support and
  AEC quality require independent physical-platform checks.
- Local inference is unavailable until vetted native runtime, notices and
  model downloads exist. Default remains local/off, never endpoint fallback.
  This is an explicit incomplete parity dependency, not a completed feature.
- Plugin non-PCM arbitrary audio and mobile/browser device engines are
  outside the Linux/Windows package; no claim that those platforms work.
- Thinking cue audio must be generated independently, not copied from the
  reference's asset; preserve its timing and purpose.

## Implementation order and module map

1. ADR/plan review. Resolve inference distribution and common-code ownership
   before manifests/shared edits; protocol number assigned only at merge.
2. Node skeleton on the node line: proto voice values/requests/settings,
   node voice actor/provider seam, client adapter, fail-closed version gate.
3. App skeleton: pure voice state/ordering/lease model, adapter fake, hidden
   area registration. No invented RPC is sent before node implementation.
4. Implement node VAD/STT, spoken session input/prompt and scoped speak TTS,
   receipt/abort ownership; settings and readiness. Add real adapters only
   after method-table prerequisites and assignment.
5. Implement capture/playback/DSP, overlay and settings in EN/KO. Wire into
   composer and existing settings with registration-only shared changes.
6. Narrow checks each commit. One root gate and one native gate at package
   end; own fake fleet/display captures at 1280x900/390x844 and 800x600 in
   dark/light EN/KO, every capture inspected. One node review and app review.

Node: proto voice/{mod,values,settings,requests}; node voice/{actor,registry,
turn,stt,tts,playback}; service/voice; client voice wrapper. Shared PCM/TLS
extraction is owned by integration, never copied from a private staged API.
App: view voice/{mod,phase,playback,capture}; link voice/{mod,adapter,audio};
app areas/voice/{mod,overlay,settings}; locale voice.ftl. No new kit control.

## Required regression evidence

Start prerequisites before any side effect; old peer refusal in both
directions; bad PCM/rate/sequence rejection before work; cross-owner and
agent capture refusal; speak restricted to its current voice session;
busy capture; rollback on node/capture start failure; late-generation audio
and receipts; partial/full queue cancellation; held/rejected/lost remote-stop
reply with full capture/playback queues: no post-Stop PCM sends or late
callback upload, local capture released on every failure, uncertain remote
record still visible and no implicit restart; interrupted and failed
playback acknowledgements; barge-in during TTS and agent work; empty,
filler-only and low-confidence speech; 260-unit segmentation including
Unicode; prefetch two; timeout cleanup; key edit/delete visibility; URL,
secret, audio and text redaction through Debug/Display/error sources;
settings read/patch compatibility and provider-switch clearing; mute
upload gating; disconnect/device-loss release; non-running button gate;
EN/KO compact overlay and settings. No real agent or provider in tests.

## Resume point

Done: isolated app and node branches; worker/common/native rules and
reference survey/source read; ADR 0067 owner consulted; coordinator gave
both-side ownership; integrator asked once for pending feature/version.
Direct candidate archives measured; no manifest or product code changed.

Current: ADR 0080 initially returned for ordinary Stop ordering. Corrected:
local generation fence/upload gate/queued PCM discard/playback and capture
release precede every remote await; uncertain remote cleanup is independent.
Resubmitted to V3 before any node product code or dependency change. Local native inference distribution
and transitive audio dependency inventory remain explicit prerequisites.

Next: obtain V3 architecture verdict; implement node and app skeletons
against a fake adapter without sending missing calls; amend dependency
measurements before adding crates. App-base and Quality B announcement
must precede wire registration. No gates/captures/runtime proof yet.


### Shared capture-slot draft

`view::audio` now separates reservation, running, stopping and confirmed
release. A voice reservation excludes dictation before a remote enable;
requesting Stop immediately rejects frames but retains ownership until
actual stream-stop/drop confirmation. Late start/release completions cannot
retarget a later consumer. This is a pure model, not a hardware API or
consumer-ready engine. The process-wide `link::audio` owner remains to be
implemented. The initial remote view-clippy snapshot failed only an
unnecessary qualification after the import fix; corrected in the next
snapshot, with narrow recheck still owed.


### Narrow evidence and desktop DSP draft

The initial protocol snapshot after the shared-table base passed 290
library tests including TS freshness. The app pure-state/capture-slot
snapshot passed view clippy and all 866 view library tests. These are
snapshot-specific narrow checks, not the package gate or device proof.

The desktop link now stages an actual Sonora processor with CPAL sample
conversion under tests, with retained audio dependency notices. It checks
frame sizes and non-finite samples before processing, keeps capture mono
16 kHz PCM16, uses explicit stereo downmix/upmix edges, and feeds the actual
converted device-rate render stream to the voice processor. A separate
unfiltered converter prevents source-rate changes from resetting AEC.
This has no physical device actor or visible feature yet; DSP and uploader
narrow clippy/tests are the next checks. Initial format exclusions and the
lockfile's inactive-target dependencies are stated in the inventory.

The app consumed the second shared package base (machine welcome identity
and kit editability seams) after its preceding narrow run ended. No whole
package gate has been run or claimed.

## Capture owner/thread source slice (2026-11-15 07:52 UTC)

[Decision Log]
- Purpose: dictation and conversation across multiple windows must not open parallel microphones.
- Existing constraints: view::audio holds pure lease transitions; CPAL streams cannot be moved to GPUI/network workers; immediate Stop must bypass full queues.
- Alternatives: one engine per Link/window would violate exclusivity; releasing on a queued Stop would permit overlap during driver teardown.
- Choice: a process singleton reserves synchronously, the device thread retains a private owner guard, and Handle::stop closes an independent atomic callback fence. The stream is dropped before the guard publishes release. A poisoned owner remains unavailable.
- Consequences: a stuck driver can keep the slot busy, while upload/callback permission remains fenced. No silent replacement/restart. Callbacks accumulate one 10 ms partial block and use a bounded nonblocking queue; overflow stops. Native errors discard foreign diagnostic text. Tests use fake devices, including a held destructor, and an already-stopped native startup that must open no device.

The physical capture source is staged behind the existing outer cfg(test); no UI invokes it. It currently contains the input stream only. Conversation playback/AEC reverse-stream timing, physical device behavior, permissions, Linux non-root and Windows remain unverified and must be completed before removing staging. CPAL's default device/config is used without logging its name. No audio file or secret is read/written. [CPAL 0.16 DeviceTrait](https://docs.rs/cpal/0.16.0/cpal/traits/trait.DeviceTrait.html) defines the typed build callbacks and fallible device/config access used here.

Remote link clippy c34b4173 did not reach our crate: alsa-sys could not find ALSA development metadata. Build VM owner B2 installed libasound2-dev1.2.11 and checked pkg-config as build account; the next admitted build is still owed. Local link narrow is queued; no duplicate remote retry while the submit-lock repair is pending.

Local link clippy-1 reached the new device/DSP/controller test build and reported 24 own-code lint failures (unused format accessor, explicit discarded-error bindings, exhaustive CPAL I24 handling, array chunk APIs, nested condition/borrow style and a ready fake future). Those sources are corrected without lint allowances or budget increases. Rerun and actual tests remain owed; this failure was not a device permission/audio/runtime test. Log `/tmp/work/wp-voice/link-clippy-local-1.log`.

Remote link clippy-2 (run wp-voice-narrow-20261115T080933Z-cc215ff4) confirms ALSA metadata now works and reaches our crate. It caught one test helper array-reference conversion after the chunk API change; fixed by copying the typed two-byte array directly. Remaining compilation/tests are still owed. Local waiting cargo-slot PIDs3148625(link)/2929893(node) were each reverified by exact argv+cwd and terminated before admission, per A13; no running build/other worker was stopped. T clarified submissions were already released; no new remote request duplicated an active same-label run.

Remote link clippy-3 d408b386 reached the test helper and caught its remaining explicit .iter() for-loop style; corrected with the direct typed slice iterator, no suppressions. Both app and node trees now merge the shared base3 6c782cb6 (QualityB + N2 c5da5305), with owned work preserved. Existing narrow runs keep their immutable snapshots; no whole gate was running.

## Shared voice meter source slice (2026-11-15 08:23 UTC)

Coordinator approved W19 as sole producer of kit::voice_meter; dictation will consume it. Existing kit/view have no dependency edge, so view::voice owns validated normalized Level, SpeechActivity and Mute telemetry, while kit owns a separate checked drawing Level and closed State. App will map them explicitly rather than add a kit->view edge. The renderer uses three 8-point bars/gaps, heights 12+30*level*[1.2,1.05,0.9], existing foreground tokens and shared QUICK one-shot transitions. Mute and unchanged input draw statically; no idle pulse is scheduled. Native opacity clamps at one (the reference computes above one at loud speech); shared 150ms motion replaces separate70/140ms reference timing under the project motion rule. GPUI geometry tests cover light/dark muted/listening/speaking. DisplayMeter stages reference120ms/0.02 publication and0.35/0.18 smoothing from last published value, rounded to three decimal places. f32 display arithmetic may differ at floating rounding ties by one display step; no PCM or recognition policy depends on it. Checks/tests remain owed.

## Shared cancellation preparation (2026-11-15 08:34 UTC)

A source integration check found the staged uploader/device had independent cancellation flags. They now use one private link::cancel::Fence implementation (atomic close + Notify), with upload::with_fence accepting the reserved device owner's same Arc. Every device callback/error/overrun and controller stop uses close(), so wake-up cannot depend on a full data queue or polling. Added partial-callback/held-ownership and supplied-fence/full-queue tests. This prepares the adapter boundary; the actual product adapter is still absent, so no runtime wiring/readiness is claimed. Dictation and voice continue to share one device owner.

Link clippy snapshot97523c3d passes after ALSA provisioning and own lint repairs: run wp-voice-narrow-20261115T082505Z-97523c3d, native-fonts0/native-cargo0, clippy16s (606+s queue), no tests or devices. This covers capture/DSP/Stop sources before the new shared cancel::Fence preparation; the next link test/clippy must cover that later delta. Base4 3cd0d3fc now merged as97523c3d; only README append conflict, both original voice and incoming Overview/host-rename paragraphs retained. Meter tests/kit build and product wiring remain owed.

Kit clippy fe03e75e passed (run wp-voice-narrow-20261115T084955Z-fe03e75e, native86s, no device). First kit geometry run1/1: measured middle origin y8 while the test expected8.25, with height43.5 preserved. GPUI0.3.10 taffy.rs383 applies round_half_toward_zero to absolute origins; updated expected native y=[6,8,10], retaining reference heights=[48,43.5,39]. This is a native-origin pixel snapping difference, not a control geometry change. Rerun owed; both themes remain in the test matrix.

### Staged speaker drain ownership

The desktop audio owner now stages a separate bounded two-segment playback
book. A segment retains its slot through the predicted last-sample device
drain deadline, rather than acknowledging when PCM is merely queued/copied.
Invalid/missing device time produces Failed; Stop and barge-in fence readers
outside the data queue, and stale completion cannot rewrite terminal receipts.
PCM is decoded in 10 ms blocks with a zero-padded final tail; packet/reader/block
Debug never exposes samples. This is an internal fake-driven contract: CPAL
output callback, actual timing events, AEC render reference and wire receipts
are not connected or claimed by this commit.

The second meter geometry run still failed on a guessed third-bar origin
(10 versus native 10.5). The check now retains exact bar widths/heights/gaps
and permits only half a native pixel of centering error against ideal source
positions, accommodating GPUI's nested layout snapping. This changes the
fixture only; no production geometry changed. Both failed runs remain evidence;
a passing run is still owed.

Base updates: app consumed the selected line through e5e654b4's shared dialog
focus seam; node consumed913f05d8, including the reviewed shared codec. The
voice consumer remains staged, so its outer test gates stay in place. Generated
TypeScript after the table extraction passed3/0/0 before commit e91a01a44.

### Device callback and reference timing draft

Capture now waits on the callback channel, callback fence and consumer closure
on its dedicated current-thread executor; the former 5 ms polling sleep is
removed. A silent or stalled callback cannot delay local Stop cleanup. Foreign
float frames are finite and normalized before entering DSP; post-DSP overshoot
is still clipped when converting PCM.

Speaker preparation is separate from AEC reverse processing. A bounded callback
writes prepared samples (checking both global and segment fences), records the
actual quantized samples as 10 ms reference blocks, and emits a final scheduled
event with CPAL's predicted DAC time plus the callback offset. Full reference or
completion queues fence the engine; empty output is silence. Cancelled-block
scanning is bounded, so a racing producer cannot keep a callback busy forever.
Tests use synthetic CPAL callback timestamps and no output device. Native output
stream creation, duplex actor wiring, clock/platform/AEC quality checks remain
owed; this source is still staged and not an advertised audio capability.

### Capture queue byte accounting

The uploader's100-frame queue alone did not enforce the ADR's2-second bound:
100 maximum100-ms chunks could retain10seconds. Add a64000-byte permit budget
including the in-flight request; maximum chunks stop at20 and normal20-ms
chunks at100. Byte overflow fences upload out of band, with explicit Overrun.
No PCM is truncated and no held send can release its permit early. The existing
full queue/held stop fakes are retained, plus a maximum-chunk regression.

Speaker packets now reject duration above180s; queued preparation expires after
10s, rendering after PCM duration+30s (at most210s), and valid final callback
transitions to its separate at-most3s DAC drain. The owner must await the earliest
book deadline alongside control/device channels. Missing callback/never-started
regressions cannot produce Played. These are staged source obligations until the
duplex actor drives them.

G2's selected shared Button prerequisite82584106 merged atc9d5cc334. Its exact
voice profile is still G2-owned and pending. V1 reserved W19 Mic/MicOff and
SquareFilled; the new assets use pinned Lucide1.50.0/9d9418a2 with source2.5
stroke and intrinsic filled square, preserving existing outline Stop. Full
ISC/Feather MIT notice and provenance are adjacent to embedded SVGs. Distribution
notice inclusion and final inspected UI remain owed.

Narrow99539cabe returned8 link lints plus the kit fixture arithmetic lint.
The intentionally hidden Block fields and ignored watch/map returns had already
been corrected in the subsequent source slice. Remaining fixture fixes compare
float bits, use checked time subtraction, assert returned transitions/readers,
and express fractional geometry via mul_add. No lint allowance was added.
Node strict at57ee128a8 passed (remote23s); node voice runtime suite on merged
d5487770f is queued separately, not yet a runtime pass.

### Shared capture consumer API draft

Existing owner/callback/DSP code is reused through link::audio::Capture instead
of a second hardware engine. acquire(Dictation) reserves the global slot and
opens CPAL only on its worker; Frame has a typed lease and fixedFormat(rate16k,
Mono,160frames), with borrowed PCM. control() returns a cloneable stop/status
handle while another task awaits next(). Dropping capture stops its callback
fence; controls do not prolong the real device guard. CaptureState::Released
still requires stream destruction. This API is compiled as product code, but
is not exposed by a product button and has no physical-device verification.

Conversation acquisition explicitly returns ConversationUnavailable until the
duplex output owner exists, even though deterministic output-callback mechanisms
are staged under tests. Other platforms return typed Unsupported before device
work. Dictation capture uses noise suppression/gain only; AEC requires an actual
render reference and is not claimed for the capture-only API. No node call or
readiness is invented by acquisition. Narrow compile/tests are owed before
publishing a consumer-ready hash to W12b.

The pure voice overlay now exposes Absent/Switching(Mute)/Active(Mute), derived
from the existing ownership state. Stopping retains the last mute value, so an
uncertain cleanup does not redraw a muted microphone as open or re-enable a
handler. Switching disables both actions and replaces only Stop with the shared
spinner, matching realtime-voice-overlay.tsx. Actual renderer consumes G2's
reviewed exact Circle profile when available; the pending source checkpoint is
not imported as ready.

STT/TTS source suite on merged d5487770f passed26/0/0 (remote runtime test
step0.18s after prebuilds), including the shared24k WAV/multipart policy,
low-confidence rejection, bounded/invalid provider replies and redaction.
It used loopback fakes only, with no provider credentials or model inference.
Native strict c31ffbf4 returned three small defects: one glyph doc identifier,
one immediate fake future unnecessarily async, and one fake startup unwrap in
a Result. Fixed without allowances; current public capture API and overlay
changes still need their first combined strict/runtime verification.

The shared capture frame also carries validated normalized block RMS for the
existing view::voice::DisplayMeter; it is calculated after capture processing
while PCM16 is encoded. It is never interpreted as speech detection, and audio
Debug still shows only length/identity. Muting can leave this measurement live
while suppressing upload. This avoids screen-local PCM scans or a second meter
producer. Actual device levels/processing quality remain unverified.

The in-flight capture budget regression holds a real fake upload, verifies its
permit remains charged while the channel has a free slot, then checks overrun
cancellation releases all queued/in-flight bytes before a held remote Stop.
The prior77ce467a remote check was canceled by signal143 while queued (no compile
step). W19 did not cancel it or treat it as a test failure; build-tool owner T
was asked whether a new latest-head submission is released before resubmitting.

Public capture API, RMS, overlay values, glyphs and staged output compile under
strict narrow25e5c8d9 except one unnecessarily qualified Future in the fake
reader signature. Fixed to the2024-prelude name. This is compiler evidence only;
no runtime/device/consumer-ready claim follows until the new head's checks pass.

### Acquisition-wide capture accounting

The per-stage caps alone could retain up to4s(raw1s+normalized1s+uploader2s).
Each hardware10ms block now reserves320 normalized PCM bytes from the owner's
single64000-byte budget before buffering begins. Its opaque permit moves with
the raw block, normalized Frame, and capture-backed uploader Frame, including
an in-flight request. The2s bound is therefore shared rather than added across
queues. Existing per-channel caps remain stricter local pressure limits.

Consumers retain Frame or its owning batch through upload acknowledgement;
public Frame is not cloneable and its fields/permit are private. Captured-source
upload validates the exact capture lease/purpose. Bare-byte frames are test-only.
Stop drains/retires the owned frames before remote teardown; a synthetic-device
regression checks permits survive channel transfer and are all released while
the remote Stop reply is held. No microphone opens in that fixture. Compile and
runtime checks of this accounting change are still owed.

### Capture-origin mute intervals

[Decision Log]
- Purpose: queued PCM must never become uploadable merely because mute was quickly toggled off.
- Existing constraint: view mute state and checking at upload time cannot identify audio buffered during a previous mute interval; level processing still needs microphone samples.
- Alternatives: clearing one queue misses partial callbacks and in-flight requests; discarding every device block stops level processing.
- Choice: snapshot an opaque upload interval before the first sample of each normalized block. Mute permanently closes that interval; unmute creates a new one. The callback suppresses upload on control-lock contention, never waits on it. Stop independently closes the acquisition fence.
- Effect: device normalization and RMS continue while muted; the staged uploader drops invalid queued frames and cancels a held upload on interval closure. Copies of PCM outside its owning frame remain forbidden by the acquisition budget contract. A new upload cannot make already transmitted bytes disappear remotely.

The mute mutator and voice adapter remain test-only until the real duplex
conversation owner exists. Public frames expose an uploadable predicate; no
product route claims mute or conversation readiness. Fakes exercise an immediate
mute/unmute with a held request and queued PCM, then acquire the entire byte
budget before issuing Stop to prove mute itself released all old frames.
Callback tests cover a partial block spanning each control transition. These
checks are newly authored and await remote execution; no physical microphone,
provider cancellation or platform permission claim follows from them.

### Device-worker playback preparation

The staged playback pump now connects the existing bounded receipt book,
packet reader, prepared-output queue and the single shared processor. It
reserves queue capacity before decoding the next10ms block and yields after
at most100blocks even if a callback concurrently frees space. A full speaker
queue therefore cannot hold capture/control execution. Pending identities are
pruned against the same book, so dropped handles cannot accumulate stale work.
Actual callback references are fed separately, and only validated callback
drain events can become Played. Terminal failure now also fences already
prepared samples and readers; timeout or owner destruction cannot leave a
failed segment audible.

Fakes cover preparation versus actual output/reference/drain, a saturated
queue during interruption and new speech, abandoned prefetch and output loss.
This remains test-only while native duplex construction and its owner loop
are unfinished. No speaker stream, acoustic echo quality or real callback
clock has been tested by this source work. The existing queued10second
preparation deadline stays explicit: the server sends the next TTS segment
only after playback confirmation, while synthesis prefetch remains node-side
(Paseo tts-manager.ts189–251). It is not an arbitrary long app playlist.

Per-segment cancellation is also checked between worker steps: a handle can
close after deadline advancement but before reader preparation. That cancelled
reader is skipped within the two-reservation bound; it does not turn a normal
barge-in into a global device failure. A queued-cancel/new-segment fake covers
that interleaving. This follow-up is newer than the accae1d3 test snapshot and
will need its own subsequent batched native narrow evidence.

### Waiting cue timing policy

The pure view cue scheduler follows voice-runtime.ts435–489: wait1500ms
in the exact active generation's Waiting phase, cancel on real speech/output
or Stop, and schedule another cue only after its own play promise settles,
then cue duration+350ms. The reference cue duration is2980ms; this reserves
that duration for an independently generated Vessel cue rather than copying
Paseo's embedded PCM. Mute clears the reference's speaking telemetry and does
not itself suppress a wait cue. The policy consequently ignores stale speaking
telemetry while the active state is muted. Old generations/old cue completions
cannot schedule new audio. It produces one-shot actions only, not idle motion.

Source tests cover sub1500ms speech-segment gaps, held cue playback, duplicate
completion, Stop/speech/output cancellation, mute, stale generations and invalid
clock input. Tests are new and remote execution remains owed. This is policy,
not an audio asset or running timer; generated cue PCM, shared duplex rendering
and its actual AEC reference/receipt integration remain unfinished.

### Receipt watch-lock correction before runtime

Source audit found that the terminal-output fencing change in accae1d3 made
Receipt::stopped read its watch state while settle invoked it under that same
watch's write lock. This would deadlock receipt transitions. The correction
splits the atomic cancellation predicate from the terminal-state read and uses
only atomics inside send_if_modified. A five-second child-process regression
exercises rendering/draining/Played, Failed and Interrupted, checks that exactly
one child test ran, and kills/reaps only its own spawned child on failure; a
reintroduced lock regression cannot indefinitely stall the suite itself.

The accae1d3 remote runtime run was still queued when this was found. Its
cancellation/replacement was requested under the tool's explicit approval rule;
no evidence from that immutable source is represented as a pass. Existing e50
strict evidence precedes this defect and remains bounded to that earlier source.
New corrected source and the regression still owe remote Clippy/runtime checks.

### Approved shared Circle producer and staged overlay composition

Consumed only G2's selected82584106..2b080afd Circle producer after V1's exact
bounded SOURCE/Kit Clippy0+137/0/0 handoff. The README append conflict retained
both current menu/base documentation and the Circle section. Current LG20,
GREEN_500, broader glyph inventory, exports and palette were preserved; only
the producer's RED_600/RED_800 additions came in. This producer proof does not
cover W19's glyph first use or the latest combined workspace.

The test-only areas::voice overlay uses that one Button renderer and the shared
voice_meter. Source container75, padding16/15.5, radius2xl, border/surface1 and
8-point action gap follow realtime-voice-overlay.tsx; both density layouts keep
the same44-point circles/icon20. State::overlay owns switching/mute; switching
disables both actions and only Stop loads. Current mute overrides stale meter
telemetry, so muted bars remain static. Labels are typed Fluent EN/KO getters
matching the reference accessible names. Caller-provided meter change identity
must change only on a published meter update; a stable repaint is not a pulse.

New actual-composition GPUI fakes cover390/800/1280 widths, light/dark, EN/KO,
open/muted/starting/stopping, fixed touch geometry and input transitions. These
checks have not executed yet. No Shell voice route, actual backend callback,
physical microphone, native capture or OS accessibility claim is added. Final
W19 Mic/MicOff2.5 and filled Square artwork/notice review, inspected captures,
combined gates and real controller integration remain owed.

### Native runtime compilation feedback

Run wp-voice-narrow-20261115T121847Z-3f04d305 reached compilation but
returned101: the pump test used an unavailable Sender::len method; cue tests
also discarded twelve state/action results. No runtime pass is claimed. The
queue assertion now uses bounded sender capacity, and cue tests assert the
expected transition/action. The receipt child probe explicitly consumes its
read count and attempts cleanup even if its initial wait-status check errors.
These are corrections to existing fakes, without a second queue or device
implementation. A new combined App/Link/Kit/View strict check includes the
previously unchecked shared Circle composition and Fluent overlay labels.

### Shared input stream before duplex wiring

[Decision Log]
- Purpose: duplex must use the same microphone constructor and ownership path.
- Existing constraint: Native combined raw input construction with capture-only DSP.
- Alternatives: duplicate CPAL setup in a conversation engine; factor raw stream ownership.
- Choice: device::input owns the native stream/callback queue, while Native retains the capture-only processor. Device metadata validation stays in device and can serve output too.
- Why: the forthcoming duplex bundle can feed actual output references to its one processor without another microphone constructor or engine.
- Impact: Dictation's public API, raw format conversion, bounded queues and stream-before-owner destruction stay the same. The raw constructor now also refuses an already-closed local fence before device discovery. Conversation stays unavailable. No device is opened by tests or this source change; remote compilation and lifecycle fakes remain required.

### Native duplex stream bundle

[Decision Log]
- Purpose: actual output references and capture processing must share one DSP owner.
- Existing assets: shared raw Input, output Callback/Ports, Pump and process capture guard.
- Alternatives: second audio engine or independent playback DSP; a device-thread bundle using these existing pieces.
- Choice: staged generic duplex Streams owns raw input/output, one Processor and one Pump. Its CaptureDevice receive waits on output references/completions, capture, cancellation and exact receipt deadlines; references are handled before capture when both are ready. Native output uses CPAL default ALSA/WASAPI metadata and the same validation/redacted error policy as input.
- Why: callback queueing/preparation is distinct from actual output and DAC completion, and Stop must wake even when input is silent. Both streams remain in the worker's device bundle until destruction, ahead of the capture guard.
- Impact: fake sources exercise ordering/drain/start failures without opening devices. The native constructor remains staged, public Conversation acquisition still refuses, and controller command/mute/barge-in wiring remains owed. This is not physical AEC, OS permission, microphone or playback proof. No dependency or runtime/model is added.

The duplex integration audit also removed watch reads from callback/reader
stop predicates. Receipt keeps an atomic terminal bit published alongside its
watch transition; only the worker/UI uses watch state. This preserves failure
fencing without a potentially blocked callback read. The finite child probe
now deliberately holds the watch write lock while the failed reader must stop
and refuse its next block. New source and fakes have not executed yet.

### Independently generated waiting cue

[Decision Log]
- Purpose: finish the waiting cue without copying Paseo's embedded audio asset.
- Existing assets: view::voice owns its2980ms duration/scheduling and shared Packet/Pump owns all output/AEC references/receipts.
- Alternatives: reuse the reference waveform (excluded); embed another licensed recording; generate a small original waveform.
- Choice: generate three quiet sine notes with smooth envelopes at startup,24kHz PCM16 mono for exactly the existing duration, retaining one immutable Bytes buffer for repeated packets.
- Why: independent synthesis needs no asset licence/download/decoder and travels through exactly the same playback cancellation/reference path as speech.
- Difference: the melody/waveform is newly authored, while purpose, duration and wait/repeat policy match the reference. Perceptual loudness/taste and actual device quality remain unverified. No separate speaker/engine or cue timer is activated.

### Reuse the GPUI power boundary at controller integration

The repository had no voice keep-awake owner, but the already-selected
gpui-pre0.3.10 has App::prevent_idle_sleep (app.rs1656) returning a drop-bound
ActivityGuard and App::on_system_sleep (1680) with multiple subscriptions.
The Linux platform uses the existing desktop portal Suspend inhibitor
(platform.rs600/1663); Windows uses a retained PowerCreateRequest/SystemRequired
request (platform.rs145/577). No new power crate, D-Bus client, unsafe code,
command or global callback replacement is needed in Vessel.

Controller integration must own both pending acquisition tasks and active
guards: local Stop drops/cancels them before remote awaits; a delayed or stale
acquisition releases its guard without restarting capture. Sleep notifications
perform only immediate local fencing/cleanup and leave remote uncertainty
visible. GPUI exposes fake idle-sleep delay/failure/count controls for these
race tests. None of this is wired or verified on an OS yet, so keep-awake
parity remains explicitly incomplete rather than an implied audio-engine feature.

### Overlay Fluent validation feedback

Run wp-voice-narrow-20261115T130733Z-013f9e87 passed fonts and returned101
from the view build script: voice-mute lacked its message-attached translator
context comment. The file-level group comment did not meet that rule. All
three EN/KO action messages now have their own context comments; label text is
unchanged. No current overlay/App strict or runtime pass is claimed. The next
immutable strict snapshot also contains the newer duplex/cue source.

A source audit applying the root run's same-name/result-fake diagnostics to
the newer native bundle found the analogous risks before its queued run. The
raw-input trait now names input_format/start_input/input_failure/next_input,
distinct from native Input methods; start-failure fakes return a typed failure
instead of unwrapping a lock in Result functions. No behavior or lint policy
is relaxed. The queued36a1 source predates these corrections.

Overlay composition was divided into placement and two private shared-button
adapters to keep each function within the package's function budget. Painting,
labels, generation/input guards and debug selectors remain the same existing
Kit controls. Existing actual-composition tests remain the verification; no
parallel button renderer or test mirroring was introduced. Queued30bc strict
predates this structural change; latest combined compilation remains owed.

### Native multi-crate strict feedback and test-condition restoration

Run wp-voice-narrow-20261115T141213Z-30bc681a returned101. During the base6
merge, W19 combined the base's separate cfg(test)/platform attributes on shared
testing into cfg(all(...)). Clippy no longer recognized that helper subtree as
test code, producing panic/unwrap diagnostics in unchanged base fixtures. The
base's exact two-attribute shape is restored; own audio testing uses it too.
No fixture body or lint exemption is added and platform/test conditions are
logically unchanged. This corrects the merge rather than editing foreign tests.

Own corrections borrow classified playback failures, make the immutable
lease/time-only Scheduled event Copy, check fixture multiplication, import the
IntoElement return trait explicitly and use the existing imported renderer
without redundant qualification. The new settings types are outside this native
check. Root189e2cc0 strict(node/proto/web) separately passed; it predates shared
patch adoption/new voice settings and does not prove their runtime.

### Duplex command admission before conversation activation

[Decision Log]
- Existing assets: one global Capture Owner/Fence, bounded Packet/Pump/Book, receipt atomics and the shared upload mute interval. Streams already drives one processor and actual callback references.
- Choice: add a two-admission command plane whose semaphore permit follows every retained raw-PCM Packet/Reader alias, not merely the data queue. Per-request and speech-interval fences travel into the existing receipt; callbacks only read atomics. No second renderer, capture engine or AEC processor.
- Why: a queue of two plus a separate Book of two would retain four full segments. Barge-in must fence queued and prepared old speech immediately, even if the device worker is waiting, while a late wake must not interrupt a newer interval.
- Impact: Pending cancellation and local Stop notify independently of the full command/data queue. The worker prunes closed receipts rather than performing a delayed interrupt-all. Mute retains its independent capture-origin upload policy. This stays staged against fakes; public Conversation acquisition and product routes remain unavailable.


The first corrected-package native strict run on f3a74dc4 reached actual Link
and App compilation and found two own draft borrow errors: cloning the speech
interval needed the mutex guard dereferenced before the Arc edge, and GPUI's
Bounds::contains takes borrowed Points. Both are corrected without changing
control or geometry expectations. The next check includes current base8c;
prior failed run/compile logs remain evidence, not a runtime or lint pass.


### Staged GPUI power ownership

[Decision Log]
- Purpose: a pending sleep-inhibitor acquisition must not outlive the voice generation or reacquire ownership after local Stop.
- Existing assets: GPUI owns Linux portal and Windows power-request integration, drop-bound ActivityGuard, cancellable tasks, multiple sleep subscriptions and deterministic fake delay/failure/count controls.
- Alternatives: a second power crate or platform command would duplicate that boundary; retaining only an acquired guard loses pending work ownership.
- Choice: the staged area::voice::power owner retains the existing typed Generation, pending task, guard and subscription together. Stop first closes its state and drops pending work/guard; completion has only a weak ownership reference and no audio-start path. Sleep closes this same state before invoking a synchronous local-cleanup callback outside any RefCell borrow.
- Why: separate generation objects cannot release one another's guard, repeated cleanup is harmless, errors become the closed Unavailable status without foreign diagnostics, and no new OS dependency or global callback replaces GPUI.
- Impact: three new tests use GPUI's fake platform for held release, delayed cancellation across generations and acquisition failure. They are source-only until their VM execution receipt. Actual sleep notification dispatch, Linux portal/Windows behavior, and controller/device integration remain unverified. Linux guard drop schedules portal close asynchronously in GPUI, so local ownership release is not a claim that the portal already acknowledged release. Public voice activation stays disabled.
