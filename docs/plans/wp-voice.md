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
