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
