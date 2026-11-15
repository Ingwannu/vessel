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

Current: initial ADR 0080 accepted after ordinary Stop ordering correction:
local generation fence/upload gate/queued PCM discard/playback and capture
release precede every remote await; uncertain remote cleanup is independent.
Device ownership releases only after streams actually stop/drop, not after
queueing a stop request. Engine/distribution and method registration remain
unapproved prerequisites. Dictation's app consumer agreed to one shared
link::audio owner and purpose lease, including fenced events and typed frames.

Draft node skeleton: voice request/update values with SINCE None, separate
capture/playback PCM bounds, redacted speech, low-confidence branch without
text; existing base64 decoder extended with opt-in preallocation bounds.
No methods or settings fields registered. Staged detector boundary consumes
Silero observations on a deterministic sample clock; it is not an inference
engine. Tests/narrow compile evidence must complete before its commit. Local native inference distribution
and transitive audio dependency inventory remain explicit prerequisites.

Next: finish narrow skeleton checks; implement node and app skeletons
against a fake adapter without sending missing calls; amend dependency
measurements before adding crates. App-base and Quality B announcement
must precede wire registration. No gates/captures/runtime proof yet.


### Pending registration draft after the shared-table head

The node branch merged the shared method-table head. Seven pending unary
rows use the single unassigned voice gate; next is one long-poll reply,
not an RPC subscription. All methods are nonforwardable and operate;
agent capture/control is refused, and speak is agent-only. Skeleton
handlers return typed runtime-unavailable without opening any provider,
changing a prompt or advertising readiness. The live own-session lease
check belongs to the real implementation before any speak success.
Generated declarations and compile evidence for this draft remain owed.

Voice policy now reuses the existing 16 KiB PromptText transport bound
(including the base and policy), retains redacted transient restoration
state, and refuses replacement while a restore is uncertain. The provider
owner's new setup field/capability must land before the lifecycle owner's
reload and this package's real enable/restore bridge. No driver or reload
actor is duplicated and no user-message/env fallback exists.

The seven rows initially exceeded the shared rpc.rs 500-line budget.
The coordinator directed consuming the already isolated Request/UnexpectedReply
extraction, then awaiting the integrator's lasting table seam; no size
record is raised. New drafts are preserved as WIP before that merge.

## Pending-wire generation repair (2026-11-15 07:43 UTC)

The first pending-row TS generation caught the exhaustive agent reply classifier; the next caught unclassified wire aliases. The seven voice handles/content aliases and the existing PCM format alias now follow the central handle/content/name rules. `VoiceEpoch` is an authority generation handle; `VoiceSeq` is a count. Speech blank validation reuses the existing JavaScript whitespace contract (BOM rejected, NEL retained). These failures are generation coverage, not a provider/runtime test. The pending node handlers remain fail-closed.

## TTS segmentation source slice (2026-11-15 07:53 UTC)

The staged `voice::segments` follows Paseo `tts-manager.ts:30–131`: JS whitespace normalization; sentence boundaries after .!?; greedy clause packing after ,;:; a late-enough space before a hard 260 UTF-16-unit cut. It reuses the existing JS trim validator rather than Rust's different NEL/BOM rules. Rust moves a hard cut before a surrogate pair (up to one unit earlier) because invalid UTF-8/unpaired surrogate output cannot become SpeechText. Added deterministic sentence/clause/word/astral cases; no provider call or playback proof. Provider/reload/codec integration is still outstanding.

Pending-row TypeScript generation now passes locally: `VESSEL_WRITE_TS=1 cargo test -p vessel-proto --features ts --lib typescript` (3 passed, 287 filtered; log `/tmp/work/wp-voice/proto-wire-generation-3.log`). Committed generator output is unedited; ts-rs emits the same trailing-space style as preexisting declarations, so `git diff --check` reports generated lines. Source whitespace is checked separately; this does not claim a clean generated whitespace check. The bridge's combined declaration test and full proto/node checks remain owed.

`voice::transcript` now stages the voice-specific bounded JSON/confidence decoder (`speech/providers/openai/stt.ts:24–47,216–250`). Only the two named transcribe models request scores. Malformed/absent/empty score arrays do not invent low confidence; valid finite mean strictly below -3.0 produces a text-free Rejected variant. Token and optional bytes structure are validated like the reference, without retaining them. JSON/text limits and overflowing sums are typed provider failures rather than foreign exception text. Tests cover strict threshold, models, empty/BOM, malformed tokens/bytes, limits, overflowing sums and redacted rejection. These are deterministic source tests, not live STT evidence.

## Playback receipt source slice (2026-11-15 08:00 UTC)

`voice::receipts` now stages per-generation completion bookkeeping. Audio is not completed when emitted/queued: exact PlaybackId plus owning VoiceLease and explicit Played/Failed/Interrupted settles it once. Duplicate matching receipts are idempotent; foreign/unknown/conflicting receipts cannot settle another pending packet. Two pending packets and 256 ten-second closed receipts bound memory. Wait is the PCM frame duration rounded up to a nanosecond plus 30 seconds; packets exceeding the 210-second total cap are refused before insertion rather than timing out mid-packet. Deadline arithmetic is checked. Expiry and barge-in settle as Failed/Interrupted, so late Played cannot create success. Deterministic clock-value tests cover capacity, deadline equality, retries, generations, TTL and count eviction; no actual playback or real-device completion evidence.

## Endpoint TTS source slice (2026-11-15 08:12 UTC)

The staged voice-only Endpoint receives a fresh ResolvedValue from the existing secret owner, uses node::http_tls::config on a blocking worker, and keeps credential/client ownership per voice endpoint. No second secret reader or TLS-root implementation exists. The request sends model/voice (defaults tts-1/alloy), bounded segment input and response_format=pcm to the validated base's audio/speech path. Proxy/redirect/protocol retries are disabled; connect10s/request60s covers response streaming. HTTP failures become closed diagnostic-free classifications; neither foreign errors/source chains nor URL/key/header/body/audio/text are retained in error diagnostics. This intentionally forgoes foreign error source chains to preserve ADR0080's traversal privacy boundary.

The 8MiB PCM cap is checked before declared-size allocation and at every chunk append; empty/odd data or an explicitly different Content-Type is refused. Missing Content-Type and application/octet-stream remain compatible; audio/pcm is accepted. PCM is 24kHz mono PCM16 little-endian, as the [official OpenAI speech guide](https://developers.openai.com/api/docs/guides/text-to-speech) specifies. [Reqwest retry policy](https://docs.rs/reqwest/0.13.5/reqwest/retry/fn.never.html) is explicitly never. Tests bind isolated loopback listeners with synthetic credentials and cover default request, redaction, redirect refusal/one attempt, oversized/odd/empty/wrong-media bodies and pre-network credential/segment refusal. They are not yet executed and do not establish actual TLS, provider or audio quality. No actual provider/key/model/device has been used. Root pending protocol and reload/codec/settings/identity/MCP prerequisites remain unchanged.

Remote node clippy d3c3d306 reached the staged tests and found five receipt fixture constructor calls using new rather than the existing UUID macro's random(). Corrected those test-only constructors. This is compilation coverage, not a passed service test. Shared base3 6c782cb6 was merged after the snapshot; no whole gate ran during that merge.

Staged input_buffer now separates bounded unconfirmed pre-roll(2s) from confirmed utterance(60s including prefix). See ADR0080 amendment and reference voice-turn-controller.ts500–538. It stores no codec or resampler, only validated CapturePcm bytes; the common codec still awaits dictation/V3. Synthetic tests cover long idle, ordered prefix transfer, whole-turn refusal before mutation, and Stop discard/no stale seal. They are not yet executed and do not prove real VAD/audio behavior.

## Shared-codec voice STT integration (2026-11-15 09:25 UTC, staged)

Coordinator-authorized exact d7f30b6b shared codec/lifetime merge is b19b38537; only appended architecture prose conflicted, all sections retained. speech_audio stays cfg(test): voice is itself staged, so this is not the first real product caller. No private dictation service/API is copied. Voice stt_form normalizes the sealed fixed16k utterance through the shared Resampler, appends the shared24k Wav directly to bounded multipart, and owns only model/language/response_format/include[] metadata. No dictation fixed prompt is inherited. Model-dependent scores policy and response decoding remain voice-specific.

The prior TTS TLS/client/body-limit mechanisms are now one private voice::http lifecycle used by both sites, with per-site clients/keys and the original central TLS/secret owners. The tests share one isolated loopback listener fixture. Added STT real-client loopback cases for request metadata/auth-only-header, low-confidence text-free rejection, unrequested Whisper scores, bounded/rejected response redaction; no real provider/TLS/microphone test. Provider-rate/full sixty-second form tests cover the shared codec integration.

Node test snapshot398f2945 had20 passed/1 failed: the credential fixture expected leading CRLF to fail, although the existing key trim policy removes leading/trailing whitespace. Changed fixture to embedded CRLF, which HeaderValue refuses before work. Its failed unwrap printed only SpeechEndpoint{..}, no fixture key/URL. New refactor/form/fixture/prompt delta tests and strict checks are owed.

W22 clarified Some replaces only the complete supplemental Vessel layer, preserving vendor base; None means the captured native/original launch context. Voice restore may be Some(savedBase+disabled policy), matching voice-session.ts491–555. Private prompt::Base now requires explicit KnownAbsent or Known(&Prompt); unavailable is not composable. Lifecycle must positively supply that base before activation; no getter currently exists. Every driver remains Unsupported and product voice admission remains unassigned/fail-closed.

STT integration narrow at054446302 compiled the shared HTTP and codec paths
but returned three strict-lint defects: Form's intentionally hidden content
header needed non-exhaustive Debug, and two WAV fixture reads used forbidden
get().unwrap(). Fixed without lint allowances or payload diagnostics. Runtime
loopback tests and the new merged-head strict result remain owed.

### Provider cancellation draft

Voice STT/TTS now requires the owning generation's CancellationToken. Already
cancelled work is refused before encoding/request dispatch; the shared site HTTP
boundary prioritizes cancellation while waiting for headers and on every body
chunk. Bounded loopback regressions hold headers/body and verify cancellation
returns without an upstream reply. These fakes use only a synthetic credential
and explicitly release their held server responses before teardown. They are
not a provider/device test and still require narrow verification.

### Staged transient update delivery

Voice owns a bounded two-audio/sixteen-control FIFO and one pending long poll.
A canceled actor reply receiver restores its exact queued update before another
await; a second waiter is refused. Stop/failure clears unsent payloads and holds
one terminal independently of queue room. Successful oneshot send is the actor's
response-ready point, not network delivery or audible output. Separate played
receipts still time out/fail when a response is lost or a client never plays it.
This module does not mutate generic RPC framing or borrow dictation's private
policy API. Actor/service integration and corresponding transport evidence remain
owed; tests only exercise the staged ownership mechanism.

Provider-cancellation strict check75458da5 compiled the new transport paths
but returned seven unnecessary fully qualified token names in fixtures; fixed
by using the parent module's imported type. No operation or diagnostic policy
was weakened. The new bounded update mechanism still owes its own first narrow
compile/runtime check.

STT endpoint Debug is now opaque like TTS, including free-form model/language
metadata. A valid-looking model field can contain an accidentally pasted key;
there is no diagnostic need to print it. A constructor-only loopback fixture
uses that sensitive synthetic model and checks the full endpoint Debug remains
redacted. This does not change persisted model values or the provider request.

The staged detector input edge now carries exactly512 normalized samples across
fixed16k client chunks using the shared aligned PCM view. It neither resamples
nor infers speech; an actual reviewed Silero engine must consume each window.
Chunk-partition invariance and cancellation fakes cover partial input and a
stop raised by the first inference callback in a multi-window chunk. Stop/reset
discards the incomplete window instead of padding a false speech observation.
No model/native runtime is downloaded or loaded by this mechanism.

Pending-start Stop target: proto now distinguishes caller nonce from an active
node lease, so ordinary Stop can cancel before an epoch reply exists. The existing
seven methods/one None feature remain unchanged. Service is still fail-closed;
no application sends this pending contract before its node registry exists.
Nonce ownership/tombstone/late completion obligations are in ADR0080. Generated
TypeScript is owed via the approved remote named ts artifact step.

### TS generation preflight and browser boundary

The f46861ea named TS run compiled proto but failed before either generator
ran: vessel-web's exhaustive browser allowlist did not yet classify the seven
pending voice methods. The bridge now explicitly rejects all seven, as ADR0080
requires a direct owner connection and does not provide browser audio here.
Its existing complete refusal-table regression includes the seven names.
No generated diff came from that failed run, so its unchanged snapshot is not
a fresh wire artifact. The corrected immutable source must run the named TS
step again before any generated bytes are adopted. The node also merged the
announced ede20014 head outside a final freeze; no product activation follows.

### Canonical connection-bound voice reservations

[Decision Log]
- Purpose: reserve one voice owner per connection/session, fence cancelled starts, and keep uncertain restoration visible.
- Existing assets: the reviewed shared ConnectionLife/Owner provide canonical identity and parent closure; no private dictation identity or second transport guard is introduced.
- Alternatives: retiring on connection close assumes restoration succeeded; forgetting old start ids lets a late nonce-only Stop address replacement work; an unbounded tombstone set permits memory growth.
- Choice: a short synchronous registry owns up to8 live/uncertain reservations and bounded nonce history (128 per connection/1024 per node). Connection child tokens cancel generation work without closing the parent connection. Pending Stop records an owner-scoped tombstone before Start admission; activation is impossible after cancellation. Only explicit confirmed teardown releases reservation capacity.
- Effect: capacity is refused explicitly rather than silently evicting live nonce authority. The caller must reserve before awaiting any provider/reload side effect and must perform ordinary authority/readiness preflight. Allocation history, uncertain outcomes and completed remote restoration are separate states.

The registry stays inside the existing test-only voice module. It consumes
the canonical shared pure lifetime primitive, not the unreviewed dictation
branch. The actual Connection accessor is still an upstream integration seam;
there is no product connection, provider or lifecycle wiring in this commit.
New fakes cover pending Stop before reservation, held startup completion,
foreign/stale generations, close without confirmed restore, session exclusion,
and both nonce-history caps. They await remote execution. SINCE stays None.

### Final STT deadline owner

The staged Finalization owns one sealed utterance and one generation-child
cancellation token. Its10second deadline is captured at construction on the
ending edge, before worker queueing. Biased cancellation/deadline selection
precedes a ready provider result; elapsed jobs never open the provider. An
exit guard cancels HTTP work on success, failure, timeout or dropped execution,
while preserving the parent voice generation for another turn. The existing
endpoint implements the typed Transcriber seam; fakes hold jobs and advance
time without providers or credentials. Only VoiceTranscript's closed
accepted/empty/rejected values leave success; no fallback prompt is fabricated.

The current endpoint slice finalizes one bounded utterance in one request.
Paseo voice-turn-controller.ts stores streaming provider partial/committed
segments and can reconnect STT once; this endpoint slice exposes no partial
transcript stream and does not silently reconnect/replay captured speech.
Confirmed VAD, rather than filler-partial text, remains the interruption edge.
Incremental segmentation/partial parity remains an explicit difference, not
something these deadline fakes prove. Product actor dispatch and real VAD
are still prerequisites. New finalization tests await remote execution.

### Spoken input and transient policy are separate boundaries

Accepted final words now have a staged redacted SpokenInput wrapper following
voice-config.ts wrapSpokenInput. It preserves the exact transcript inside the
spoken-input/instruction blocks and produces the existing SessionInput::Prompt
with one ContentBlock::Text, so intentional accepted words use ordinary history.
Empty/rejected variants cannot produce an input. Its bounded allocation is the
existing64KiB SpeechText limit plus fixed wrapper bytes; the16KiB supplemental
system-policy cap does not apply to ordinary user messages. This is not a user
prompt fallback for unsupported system-policy transport or explicit MCP context.
The actor still must recheck exact lease/generation before dispatch, and this
module sends nothing. No text or system content enters its Debug/errors.

### Speech pipeline pre-code decision

[Decision Log]
- Purpose: preserve ordered per-segment playback while preparing the next two segments and keeping cancellation/deadlines finite.
- Existing assets: voice::segments, the endpoint TTS client, the receipt-duration rule and the existing futures-util dependency cover these roles; no new executor/dependency is needed.
- Alternatives: sequential synthesis wastes the reference's prefetch; detached Tokio tasks require a second task/cleanup owner; waiting only on a playback receipt leaves lazy synthesis futures unpolled.
- Choice: reuse FuturesOrdered with at most two ahead-of-playback futures/results and select it alongside the one current playback receipt. Polling remains in the owned speak future. Cancellation drops the futures and cancels their shared child token; per-provider and receipt deadlines remain finite.
- Effect: at most two prefetched segments plus the current segment (the reference schedules the replacement before waiting for current playback). The caller must first authorize the authenticated own-session lease and reserve its single speech-job capacity. The actual actor/MCP producer and canonical outbound/receipt adapter remain separate unfinished wiring; typed fake adapters do not advertise readiness.

The staged synthesis runner now polls its two ordered prefetch futures/results
alongside the current playback wait, without spawning detached tasks. Each
segment receives its own group/receipt id as in tts-manager.ts350–410. The next
packet is emitted only after current Played; Failed/Interrupted ends the job
without claiming full audible success. Shared receipt-duration calculation
remains owned by receipts.rs and gives the same PCM duration+30s/210s ceiling
to the runner and receipt book. Provider preparation has its own60s deadline.
Cancellation and deadlines are selected before other ready work; the child
cancellation guard covers every exit while preserving the parent generation.

The two prefetched segments plus current segment can retain up to24MiB decoded
PCM per voice generation, excluding transport/serialization buffers; this is
not a claim of at most two total PCM packets. Tests hold/reorder synthesis and
hold a playback receipt to check real prefetch progress, stable output ordering,
failed playback, cancellation cleanup and independent deadlines. All are new
and unexecuted. Actual exact-session authorization/single-speech admission,
canonical outbound receipt adapter and product speak registration remain owed.

Spoken-input wrapper prose is independently authored; only the spoken-input
and instruction framing/behavior is retained from the reference. No Paseo
source, embedded cue PCM or native bundle is imported into this package.

### Silero postprocessing pre-code decision

[Decision Log]
- Purpose: a raw model probability is not Sherpa's detected-speech observation consumed by the outer800ms/1000ms boundary.
- Existing assets: fixed512-sample input and outer Boundary exist; no linked model runtime exists. The pinned Paseo runtime uses Sherpa1.12.28 with100ms minimum speech,200ms silence and its default negative-threshold hysteresis.
- Alternatives: passing probability>0.5 directly changes interruption timing; a local RMS/peak gate substitutes a different detector and is not accepted.
- Choice: independently express the pinned probability policy as a closed Rust state machine with finite validated probability input and checked sample positions. Keep it test-only and separate from inference. Reset must also reset the actual recurrent model once one is approved.
- Effect: fake score sequences can prove policy edges but cannot prove Silero inference, model licensing, speech recognition or readiness. Also preserve the source's strict >512 buffered lookahead instead of emitting an exactly-full final window early.

The fixed-window adapter now retains an exactly-full final512-sample window
until a real subsequent sample arrives, matching the pinned runtime's strict
buffer-size >window condition. Partition tests retain the same five processed
windows for a six-window input and a new exact-boundary test supplies one real
following sample. Stop still discards retained lookahead without padding.

The separate probability policy has validated finite0..1 inputs,100ms/200ms
sample-count filters and the pinned0.5/0.35 threshold behavior. Equality does
not invent a new speech start, and mid-band continuation preserves an existing
silence start until a score above the speech threshold clears it. Independent
closed enum states replace reference flags/sentinels; checked u64 clocks refuse
exhaustion. Synthetic probability sequences test these edges and composition
with the outer turn boundary. There is still no runtime, model load, microphone
probe or recognition/quality proof. Runtime+model distribution requires its
separate reviewed ADR amendment before activation.

### Typed turn composition before the real detector

[Decision Log]
- Purpose: connect validated capture order, inference-window carry, probability/outer boundaries, bounded utterance buffering and the ten-second final job.
- Existing assets: Registry owns opaque connection authority; Windows/Gate/Boundary/InputBuffer/Finalization already own the lower policies.
- Alternatives: duplicate those policies in the eventual RPC actor; compose them behind a detector parameter.
- Choice: a staged Turn owns those exact mechanisms and an exact VoiceLease. Only contiguous sequence input is admitted; retransmits/gaps are refused before buffering/inference. Caller still checks connection authority through Registry before reaching it.
- Why: a deterministic detector can exercise the complete policy while model distribution remains unapproved, without pretending RMS is VAD or activating a node call.
- Impact: a confirmed start cancels the preceding turn's child work; an ending edge creates Finalization immediately so queue delay counts. Stop/error retires input, discards pre-roll/lookahead and resets the detector, never flushes into an agent prompt. Native inference, RPC/session/provider integration and real detector readiness remain absent.

### Node strict compilation feedback

Run wp-voice-node-narrow-20261115T124554Z-61e5fa93 returned Cargo101:
nonce accounting used confusing owner/owned names, and the finalization fake
called a nonexistent CapturePcm::try_from(Vec) instead of its validated new
constructor. The counter is renamed retained and the fixture uses the existing
constructor. No runtime pass follows from that failed strict run. The next
immutable strict snapshot includes the later probability/turn composition too.

### Minimal VAD amendment prepared for review

Official pinned licence/source metadata and complete ORT source notices are
now retained under docs/notices/voice-vad with a byte/hash manifest and Vessel
NOTICE. Existing cached smallvec/cfg-if/windows-link archives were matched to
published sizes/checksums before reading notices. No new model/native binary
or inference dependency was downloaded, installed, loaded or shipped.

ADR0080 proposes only a quarantined acquisition/inventory phase for the exact
model/CPU runtime and wrapper archives. It explicitly requires V3 to accept
that phase before download and a later inventory/layout review before any
manifest/load/distribution. Native archive-content correspondence, conversion
provenance reconciliation, actual feature graph/platform support, AEC/device
quality, inference/heap bounds and activation remain owed. This prerequisite
review follows the coordinator's explicit before-download ruling, separate
from the final package review.

Merged announced node82ae7c91 (A8) cleanly asd152294a7; its gate46g named web
rows.test timeout is the coordinator's stated load exception, not a W19 pass
or locally waived test. Current b240 strict run predates this base merge.

Resume: node Turn/finalization/ordered synthesis and native duplex source are
staged against fakes; no real detector, actor/controller, settings wiring,
MCP speak or prompt reload activation exists. Root/native latest strict runs
are queued; full gates, UI captures and final V3/V4 handoff remain owed.

### Turn/synthesis strict feedback

Run wp-voice-node-narrow-20261115T132041Z-b240b3f2 returned101 on thirteen
strict lint diagnostics, not a runtime result. Corrections consume unit-return
closure statements explicitly, use named integer quotient semantics in the
quota fake, return typed fake detector errors, remove redundant Future
qualification and use if-let for the ready prefetch branch. Generic provider
traits now say recognize/generate, distinct from endpoint transcribe/synthesize
methods; endpoint behavior and wire values are unchanged. No lint exception
is added. Latest source still owes strict/runtime on the announced merged base.

### Phase A acquisition result and bounded continuation proposal

V3 row221 explicitly accepted only six fixed downloads/non-extracting inventory.
All six exact size/SHA checks passed with private task ownership and no network
retry, native/model load or manifest change. Five inventories completed; Linux
symlinks are rejected layout entries and Windows stopped before its408244224B
PDB at the original256MiB cap. All inspected archive notices match the retained
source notice bytes. Public exact inventories are in docs/evidence/voice-vad-phase-a.

ADR0080 now proposes an explicit metadata-only exception for that one existing
PDB, with a bounded512MiB declared metadata total and the original256MiB actual
inflation/notice limits for all other members. It has NOT been executed. Return
the findings and proposed continuation to V3 before any exception. Runtime
loading/distribution/final layout and real VAD compatibility remain unapproved.

V3 accepted the exact metadata-only Windows continuation on da77d7d18. It
completed34entries/17,725,619other inflated bytes and retained the large PDB
as skipped/unhashed/CRC-unverified. Full Windows notice comparison proves
CRLF-only differences, not raw-byte equality. The source LF notices and all
first-refusal evidence remain intact. Final runtime file/layout/manifest/load
and real model/device tests are still unapproved and unperformed.

### Typed voice settings before the existing settings-writer integration

[Decision Log]
- Existing assets: checked1f805 shared present/null visitor, redacted SecretReference and DictationChange; Dictation's validated endpoint/language/STT catalogue and SetDictation.over own STT site rules. The dictation owner and coordinator delegated exactly value/overlay pub(crate) visibility, with bodies unchanged.
- External alternative: new ad-hoc JSON/settings store or a second secret decoder is unnecessary; serde and existing validators suffice.
- Choice: project a switch-free STT patch through existing SetDictation.over/resolve (its independent switch field is absent from the voice shape), reuse shared patch/null operations, and add typed TTS-site values/metadata plus root voice switch defaultOff. STT/TTS sites change independently; changing/resetting a provider clears its old inactive fields, while an unchanged provider preserves selected fields.
- Why: retain omission/reset semantics and credential redaction without duplicating policy. TTS model and voice identifiers are distinct types, both bounded through the existing endpoint metadata validator. Local TTS catalogue IDs are metadata only; none grants a model download/load or readiness.
- Impact: values/patch tests only, no optional NodeSettings field/Copy conversion/persistence yet. Existing unknown-field set-aside/read-preservation stays the settings owner's obligation at wiring. No new call, protocol/schema number, secret reader, model runtime or UI route. A switch-only change may retain untouched future site choices, while editing an unknown site requires selecting/resetting a known provider.

Reference: voice-page.tsx43–165 (switches and independent sites), voice-settings.ts
(provider/model defaults and custom selections), speech-config-resolver.ts39–49
(Local/defaultOff), providers/openai/config.ts8/25–65, and local model-catalog.ts
358–462 (TTS metadata only). Existing secret references replace inline API-key
values; local ASR/TTS remains LocalNotBuilt and automatic model download is absent
until its separate reviewed distribution slice.

The STT read projection copies only bounded settings metadata into the existing
default resolver because that API accepts an owned SetDictation shape. It is
not an audio/hot-path copy; preserving one provider/default rule is preferred
to a second implementation. Consuming patch overlays move those fields.

The endpoint TTS adapter consumes distinct TtsModel/TtsVoice values and uses
their centralized defaults. New tests cover deep independent-site edits,
omission/null/provider resets, future-value retention on an unrelated disable,
inactive-field refusal, key-presence reads/redaction, bounded identifiers and
metadata-only role separation. Feature-ts tests exercise nullable patches and
the new classified named aliases. These DTOs do not enter the existing
NodeSettings wire graph yet; generated declarations are unchanged for now.
Runtime/strict results for this slice remain owed; the previous189e2cc0 strict
pass predates it.

Voice settings first VM Proto+ts run437739b2c executed337 tests successfully
and failed3 alias/TS assertions. All9 new non-TS settings policy tests passed;
the new named aliases were inserted out of global order, invalidating the
existing binary-search classifier. Restore sorted positions of only those3
rows, leaving the classifier and prior aliases unchanged. These initial
failures remain recorded; a corrected-head result is still owed.

### Real RPC upload is blocked on a shared writer cancellation edge

Read the existing client PendingCall Drop and driver Command::Call branch:
Cancel is best-effort through the same bounded command queue, while a queued
Call is fed without a per-call fence. The fake Adapter.upload future's drop
semantics do not establish this real transport behavior. Before implementing
its adapter, obtain one shared-writer owner for a typed cancellation/admission
edge and full-queue/held-sink tests. ADR0080 records the commitment and partial
framing proof obligations. No shared RPC edit, real upload or activation has
occurred; do not weaken local Stop or claim already committed bytes retract.
