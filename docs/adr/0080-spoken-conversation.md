# ADR 0080: Spoken conversation and desktop audio

Status: initial contract accepted, 2026-11-15. Engine/dependency distribution
amendments and product integration still require their own evidence.
Protocol number given at merge; `voice::SINCE: Option<ProtocolVersion>` is
`None` until assigned by the node integrator. No schema change: audio,
voice leases and playback receipts are ephemeral.

## Purpose and intent

A person starts an opt-in spoken conversation with an existing agent,
speaks while listening, interrupts an answer by speaking, mutes capture,
and stops. The native app owns the physical microphone and speaker; the
node owns turn detection, transcription, agent input and speech synthesis.
Host Voice settings select dictation STT, conversation STT and conversation
TTS independently. Ordinary typing and voice use the same session authority.

The reference is Paseo 0.15.4, read from its source, not copied. Detailed
locations and differences are in `docs/plans/wp-voice.md`.

## Existing implementation and constraints

- ADR 0067 owns dictation: PCM16 mono values, STT settings, endpoint
  validation, bounded in-memory WAV/multipart and transcription HTTP.
  Its current implementation is staged, private and test-only. It is not
  a product API. Its owner confirmed that microphone capture, VAD, TTS,
  local inference and model downloads are outside that package.
- Dictation's one-second chunks and 15-second commit policy cannot drive
  low-latency conversation. Its provider does not request logprobs; voice
  must retain Paseo's low-confidence rejection separately.
- The node has one typed method table and `required_protocol()`.
  `node.settings`/`node.set_settings` own settings persistence;
  `FleetFiles::resolved_secret` owns secret resolution. New secret readers
  and raw key settings would create another authority and are rejected.
- The app has pure `view`, network/device `link`, GPUI `app` and controls
  `kit`. Settings already has a reserved Voice section, currently hidden.
  There is no existing microphone/playback engine or voice service.
- The node integrator must assign the version and announce the method-table
  prerequisite before any new call is wired. App code never invents a call
  absent on its node base. Until then, a typed internal adapter and an
  in-memory test fake exercise the state machines; no entry point is shown.

## Alternatives considered

1. Browser/WebView audio inside the GPUI app. Rejected: adds an engine,
   permission origin and IPC just to access devices, and duplicates the
   native audio lifecycle. Paseo desktop can use WebAudio because its UI
   already runs in a browser.
2. Spawn ffmpeg/arecord or PowerShell for capture. Rejected: separate
   installation, fragile process/permission boundaries, and no shared
   playback reference for acoustic echo cancellation.
3. Send raw microphone audio straight to a cloud realtime model. Rejected:
   substitutes a different assistant for the selected coding agent and
   bypasses the node's session, permissions and speak-tool path.
4. Reuse the complete dictation actor for voice. Rejected: commit cadence,
   finalization, confidence and cancellation semantics differ. Share codec
   and transport primitives only after their owning branch is integrated.
5. Handwrite OS audio/ONNX/WebRTC FFI. Rejected: our code forbids unsafe;
   the standard library provides none of these APIs.

## Choice and reasons

### Ownership and flow

One app audio owner holds an exclusive `CaptureLease` with a closed
`CapturePurpose::{Dictation, Conversation}`. It survives route changes,
not app shutdown. Dictation and voice cannot record concurrently. A second
request is a typed `CaptureBusy` refusal, never a second open device.

`view::voice` owns validated presentation values, phases, mute, generation
fencing, playback ordering and pure VAD boundary state. It does no I/O.
`link::voice` owns the bounded conversation actor, node adapter, playback
acknowledgements and cancellation deadlines. Shared `link::audio` owns
the exclusive microphone/playback device thread, purpose leases and PCM
conversion; dictation consumes that same engine rather than opening another.
`app::areas::voice` draws the composer overlay and host settings through
existing kit controls. Words come from EN/KO catalogs. Shared shell/link
files receive registration lines only.

`vessel-proto::voice` owns wire values and requests. The node's `voice`
actor owns connection-bound leases, STT/VAD/TTS providers and transient
outbound updates. Service methods authorize before work. Session input,
reload and interrupt use the existing session service, not an alternate
harness or prompt sender. The provider owner supplies the typed transient
AgentSetup field and per-driver capability; the lifecycle owner supplies
the one reload actor. Voice owns only bounded policy text and generation-
fenced enable/restore state. Unsupported drivers are explicitly refused;
there is no ordinary user-prompt or arbitrary environment fallback. The MCP path exposes `speak` only to the agent
whose session currently has a live voice lease. It cannot open a microphone
or select another session. Agent tools and direct person calls have
separate request types and authority checks.

Starting validates readiness and a non-running existing session, then
reloads its voice instruction and installs the speak capability, before
capture starts. Failure rolls back that generation; uncertain remote
cleanup is surfaced. Local Stop first fences all capture callbacks, gates uploading, discards
queued unsent PCM, stops local playback and capture and releases the local
capture lease, independently of any remote await or full data queue. Only
then does remote teardown interrupt the running turn, cancel speech,
disable voice and restore the prompt. Local cleanup runs on every error,
timeout and lost-reply path. Remote teardown uncertainty remains visible
and retains its separate remote-generation record, never microphone or
upload permission. A new conversation cannot silently replace that remote
record; dictation can acquire the already released local capture lease.
Disconnect or OS device interruption has the same immediate local privacy
boundary; it never keeps a microphone streaming to a replacement connection. Reconnect may start a new explicit generation only
when the previous lease is confirmed gone.

### Providers and settings

One optional top-level `voice` field is added to node settings. Absent means
voice off. STT and TTS each default to `Local`; local defaults are Parakeet
v2 (`parakeet-tdt-0.6b-v2-int8`) and Kokoro English (`kokoro-en-v0_19`).
Dictation remains ADR 0067's independent field and default-on switch.
Provider selection drops the previous provider's fields rather than moving
credentials between sites. Omitted patches keep, null clears, and invalid
patches change neither file nor effective settings. Unknown values remain
readable but cannot be applied. Old writers preserve newer settings through
the existing backup/refusal contract. Reads expose key presence only.

`Endpoint` is OpenAI-compatible HTTPS (or parsed loopback HTTP), default
base `https://api.openai.com/v1`. Voice STT uses `/audio/transcriptions`,
`whisper-1`, language `en`. Models `gpt-4o-transcribe` and
`gpt-4o-mini-transcribe` request logprobs; finite mean below -3.0 means
non-speech. Missing logprobs do not manufacture low confidence. Dictation's
fixed prompt is not silently inherited. TTS uses `/audio/speech`, `tts-1`,
`alloy`, PCM16 mono 24 kHz. Closed provider enums dispatch at a validated
boundary; responses never leak foreign JSON beyond their decoder.

Node VAD uses Silero at 16 kHz, windows of 512 samples, threshold 0.5,
internal minimum silence/speech 0.2/0.1 seconds, boundary confirmation
800 ms and ending silence 1000 ms. The dormant client VAD constants are
not the reference algorithm. Barge-in cancels current synthesis/playback
and interrupts the agent on confirmed speech start. Empty/low-confidence
final text returns to listening without submitting an agent turn.

Local model loading/download is a separate prerequisite inside this
package, not permission to substitute a peak detector or a fake provider.
No local engine is advertised ready before its runtime and selected model
are available. Local backend returns `LocalNotBuilt` while absent; missing
models return typed readiness. Full local parity remains incomplete until
the dependency/model license and distribution amendment below is reviewed.

### Linux and Windows devices, crates and licences

Chosen capture/playback candidate: `cpal = 0.16.0`, Apache-2.0, in the
native link crate only under Linux/Windows target conditions. It supplies
safe Rust callback APIs over ALSA (Linux, including system plugin routing)
and WASAPI shared mode (Windows). ASIO/JACK optional features stay off.
No process is spawned. GPUI stays on its UI thread; a dedicated device
owner creates/drops both streams and owns the audio processing instance.
The node never opens the app's hardware. No system package is installed by
this feature; Linux build prerequisites must be supplied by the build image.

Chosen echo processing candidate: `sonora = 0.3.0`, BSD-3-Clause, default
library only, no examples or FFI. Rendered output feeds the reverse stream
before the corresponding capture block; capture passes through AEC3,
noise suppression and gain control in 10 ms mono blocks. Use the same
engine for thinking cues and TTS so the echo reference includes both.
CPAL itself does not provide echo cancellation. Unsupported device rates,
formats or processor initialization fail with typed errors; never silently
claim echo cancellation. Conversion/downmix/resampling is bounded and
preserves state across callbacks rather than resetting each chunk.

Direct crate archives inspected from crates.io on 2026-11-15:

| Candidate | Archive bytes | Unpacked source bytes | License |
| --- | ---: | ---: | --- |
| cpal 0.16.0 | 104465 | 506821 | Apache-2.0 |
| sonora 0.3.0 | 111649 | 565551 | BSD-3-Clause |
| sherpa-onnx 2.0.1 (not selected yet) | 55820 | 267432 | Apache-2.0 |

Sources: [CPAL](https://github.com/RustAudio/cpal),
[Sonora](https://github.com/dignifiedquire/sonora),
[sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx).
These are source/archive sizes, not linked binary size. CPAL's OS crates
and Sonora's companion crates must be inventoried before manifest changes;
Linux/Windows linked-size delta must be measured before release.
`std` has no audio API and existing dependencies have no device/AEC owner.

Local inference candidate is sherpa-onnx's safe wrapper, with ONNX Runtime
(MIT). It is deliberately not selected/added in this initial decision:
its sys build may fetch prebuilt native archives, whose platform size,
notices, supply-chain pins and runtime-loading paths need a further
reviewed amendment before any manifest change. Model weights have their
own licenses. Default reference archive sizes are 482468385 bytes STT and
319625534 bytes TTS; those numbers do not establish redistribution rights.
Silero, Kokoro's espeak data and every selected model require exact notices
and trusted-source digests before distribution. Neither a model nor
native binary is copied from Paseo's bundle. No automatic build-script
fetch or unverified download is accepted as a production dependency.

### Wire contract, authorization and compatibility

Proposed methods, all pending on one `SINCE = None` until assignment:

| Call | Request / reply | Authority |
| --- | --- | --- |
| `voice.start` | client lease id + session / ready generation | person operate |
| `voice.chunk` | lease + generation + sequence + PCM / contiguous ack | owning connection operate |
| `voice.next` | lease + generation / typed transient update | owning connection operate |
| `voice.played` | lease + generation + playback id + outcome / ack | owning connection operate |
| `voice.abort` | lease + generation / interrupted or typed failure | owning connection operate |
| `voice.stop` | lease + generation / stopped or uncertain cleanup | owning connection operate |
| `voice.speak` | bounded speech text / playback finished outcome | agent's own active voice session only |

No call is forwardable through a fleet home. A paired read-only principal,
agent capture request or another connection's lease is refused before
provider/device work. A new start never replaces another owner's live
lease. `next` is cancellable long-poll, not a durable event log; audio
updates never enter session storage. Transcript prompts and intentional
assistant speech follow the ordinary session history, unlike raw audio.
A cancelled `next` consumes no queued update before response readiness.

Typed ids, sequence, sample rate, PCM, character counts, confidence,
playback group and duration values validate at decode. The initial node
emits one complete packet per synthesis segment, as the reference TTS
manager does at lines 397-410; no arbitrary partial-group stream is exposed.
Low-confidence and empty transcription variants cannot carry text. Audio `Debug`
shows byte length only; transcript/speech values redact content. Old peers
cannot send/receive the calls; omitted settings are not interpreted as
support. Unknown settings/reasons remain readable. Method registration,
client wrappers and generated TS land together only after the node exists.

### Limits and failure behaviour

All added bounds become named constants at their owner. Additional limits
are deliberate resource floors, never silent truncation:

| Resource | Bound / behaviour |
| --- | --- |
| Active voice | 1 per app; 1 per connection; 8 per node; one owner per session |
| Start nonce history | at most128 retained attempts per connection and1024 per node, including cancelled-before-start markers; explicit Capacity refusal, no eviction/rebinding of a live connection's ids |
| Capture wire | 16 kHz mono PCM16, 20 ms normal chunks; at most 100 ms/chunk |
| Capture queue | 2 seconds; overflow stops capture with CaptureOverrun |
| Input ordering | contiguous sequence; exact duplicates idempotent; gaps fail lease |
| Utterance | 60 seconds / 1920000 PCM bytes at 16 kHz; TooLong ends turn |
| Final STT | 10000 ms, from Paseo voice controller |
| HTTP | connect 10 s, request 60 s; no redirects, env proxy or automatic retry |
| STT response | 512 KiB before decoding; text at most 64 KiB UTF-8 |
| Speech input | 64 KiB UTF-8; split at sentences/clauses/words, 260 UTF-16 units |
| Composite transient prompt | existing 16 KiB AutomationText transport cap, including base and policy; refuse before reload |
| TTS prefetch | 2 ahead plus the current segment, matching Paseo replacement scheduling; 8 MiB each, at most24MiB retained decoded PCM per generation before transport buffers |
| Playback receipt | PCM duration + 30 s, bounded by 210 s; timeout fails speak |
| Closed receipts | 10 s TTL, 256 ids maximum; unknown/foreign id refused |
| Outbound queue | 2 audio segments plus 16 control events; pressure fails lease |
| App/node operations | finite 10 s start/stop/abort; local Stop gates immediately before remote awaits; remote uncertainty is visible |
| Meter | publish at 120 ms or 0.02 delta; attack 0.35/release 0.18 |
| Thinking cue | wait 1500 ms, repeat after cue duration + 350 ms |

A receipt acknowledges played, failed or interrupted, never queued audio.
Generation fences reject callbacks, audio and receipts from an earlier
lease. Cancellation is out-of-band from bounded data queues. Reconnect,
shutdown and abort must be able to stop even when those queues are full.

### Secrets and reach

Keys are stored only by existing secret.set, selected by validated secret
references, resolved on a blocking worker for a fresh provider session.
Reads return present/absent, never the name or value. No library reads a
process environment or home to obtain credentials. The shared node TLS
configuration and secret owner are reused after their integration; there
is no copied TLS root/secret-loading implementation.

The administrator-selected endpoint receives PCM/WAV or speech text with
that site's bearer key. HTTPS can reach that configured host; plain HTTP
is restricted to parsed loopback. Userinfo, query and fragment are refused.
Redirects and environment proxy lookup are disabled. Keys are never placed
in command lines, added environment variables, settings files or URLs.
Errors redact URL, headers, bodies, text, audio and foreign diagnostics;
private causes must also be safe under Debug/source traversal. No debug
recording switch, temporary audio file or transcript preview log exists.
Cancellation cannot retract bytes already accepted by an external provider.

## Advantages, costs and effects

The app stays native; the node remains the only provider/credential owner.
Shared codec boundaries avoid duplicate audio parsers without coupling
voice turn policy to dictation. Typed ownership and finite queues bound
capture, synthesis and stale replay. Costs are extra desktop audio/DSP
crates and platform testing, and a separately audited local inference
runtime/model distribution. A fake/provider unit test proves contracts,
not microphone permissions, audio quality or actual provider service.

Deliberate differences, detailed in the plan, include RPC long-poll rather
than ad-hoc push, bounded resources, explicit reconnect rather than implicit
resync, secret references rather than raw config keys, no env overrides or
audio debug dumps, and readiness refusal until real local inference ships.
These decisions do not establish whole-package parity by themselves.

## Turn-buffer resource amendment (2026-11-15 08:28 UTC, staged)

Paseo voice-turn-controller.ts500–538 appends detector/STT data while listening,
before confirmation. Its endpoint STT buffer can therefore accumulate idle audio.
Vessel's staged input buffer keeps only the latest two seconds (64000 PCM16 bytes
at16kHz) while unconfirmed, preserving the800ms confirmation plus detector/chunk
latency without retaining endless silence. After the confirmed edge it retains
at most1920000 bytes, sixty seconds including that pre-roll and ending silence.
Overflow refuses before mutation and retires the turn/lease in its eventual
actor; it never truncates confirmed speech. A Stop/abort discards both prefix and
current utterance without STT commit. This is a deliberate resource difference,
not proof that a two-second prefix covers physical/network conditions; detector
and codec integration plus waveform validation remain owed.

### Pending-start cancellation target clarification

A Stop request cannot require a server epoch before the start reply was received.
The typed target therefore distinguishes Starting(client VoiceId nonce) from
Active(full VoiceLease). Both require the canonical opaque connection owner;
Starting is not a bearer capability and cannot address another connection's
start. The registry must cancel/retain the original start record through its
actual factory/reload cleanup, fence late completion, and never bind that nonce
to a later acquisition. Active always requires its exact epoch. This closes the
implementation gap between the client-selected cancellation id described above
and an epoch-only Stop request; it adds no method, feature number or schema.

Local Stop still fences capture/upload/playback first. Sending pending Stop does
not wait for the delayed start result; a late ready result is retired, never used
to open capture. Unknown/rejected/lost remote cleanup stays explicitly uncertain.
Owning-registry held-start/late-ready/cross-connection tests are owed with wiring;
typed serialization fixtures alone do not prove remote teardown.

### Pending-start cancellation and bounded nonce history

The registry records a caller-owned Starting Stop even if its Start handler
has not yet reserved the attempt. A later Start on that same connection and
nonce is refused before provider or reload work. For a reserved generation,
Stop cancels its child token synchronously; late startup success cannot mark
it active. A foreign connection cannot cancel an existing reservation, and
an active target additionally checks the exact node epoch.

Original ids are retained through confirmed remote teardown and the owning
connection's lifetime. Unknown pending-stop markers are scoped by the opaque
canonical connection owner. Unlike Paseo, this requires explicit finite nonce
history limits:128 per connection,1024 per node. Saturation refuses additional
admission or unknown nonce cancellation; it never evicts a marker to manufacture
space or returns Stopped without recording the cancellation. Such refusal is
remote uncertainty to the app, whose local Stop boundary already remains closed.
Closing a connection cancels its jobs but cannot free a remote reservation
before restore is confirmed. Uncertain cleanup retains its active quota and
session exclusion. This pure registry currently remains staged under tests;
actual Connection::life wiring and reload/provider teardown are not supplied
by this module or proved by its state-transition fakes.

### Endpoint finalization slice

The initial endpoint path emits one final transcript for the bounded utterance;
no partial provider transcription is exposed. Paseo's streaming session can
commit multiple segments, suppress filler-only partials and reconnect once.
Vessel does not substitute filler classification for VAD or implicitly replay
microphone audio after provider failure. This is incomplete incremental
transcript parity. Finalization still uses the reference10second deadline,
measured from the ending boundary and including worker queue delay. Timeout
or cancellation yields no text for a prompt and closes only that job's child
provider token. Real turn/agent dispatch stays gated on reviewed VAD and
lifecycle support, rather than claiming that endpoint HTTP alone enables voice.


## Proposed amendment: minimal VAD acquisition (2026-11-15)

Status: **not approved**. This amendment requests review of one bounded,
quarantined acquisition/inventory phase only. It does not authorize a dependency
manifest, extraction into a product tree, native/model loading, distribution,
auto-download or voice readiness. The coordinator requires this amendment and
Vessel notices before any new model/native-runtime download. A PASS must name
that acquisition scope explicitly before any such download begins.

### Purpose, constraints and alternatives

Endpoint STT/TTS still needs actual turn detection. Windows/Gate/Boundary tests
prove framing/policy only. The reference uses a629KiB16k-only Silero v4 export
and Sherpa1.12.28 filtering; replacing it with RMS or a current v5/v6 model
changes behaviour. Full Sherpa distribution adds unrelated ASR/TTS and
phonemization concerns. The proposed smaller path is safe Rust ort with an
explicitly pinned CPU ONNX Runtime. Direct FFI/application unsafe, system-library
probing, wrapper binary-download defaults and implicit provider fallback are
rejected. Model/API/operator compatibility remains unproved.

The later local ASR/TTS slice stays separate and `LocalNotBuilt`. Kokoro's
espeak-ng data is GPL-related and is **excluded**, along with every previously
quarantined model/native candidate, from this proposal. No asset is obtained
from or copied out of Paseo. Reference bytes were only hashed to compare the
official publisher digest; acquisition would use the official release.

### Exact proposed acquisition allowlist

The following are publisher metadata, not downloaded-artifact attestations:

| Artifact | Official release asset id | Bytes | SHA256 |
| --- | ---: | ---: | --- |
| [silero_vad.onnx](https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx) |271935959|643854|9e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6|
| [onnxruntime-linux-x64-1.28.2.tgz](https://github.com/microsoft/onnxruntime/releases/download/v1.28.2/onnxruntime-linux-x64-1.28.2.tgz) |542003864|9128991|d7209b8751b27b862b0c76332c2e20e203396edb5dab700ecf4bb485cf147415|
| [onnxruntime-win-x64-1.28.2.zip](https://github.com/microsoft/onnxruntime/releases/download/v1.28.2/onnxruntime-win-x64-1.28.2.zip) |542003886|78620837|c4eedd29489d5feca21866d054638416f3655bf6b18851b3b6b85c8313e95c35|

Only Linux x86_64 and Windows x86_64 CPU candidates are in scope; other
architectures have no selected artifact and remain explicitly unavailable.
The [release metadata](https://api.github.com/repos/microsoft/onnxruntime/releases/tags/v1.28.2)
pins runtime source33ca9628233dc8f002435e868d4c2e9f82766ca1. The
[model publisher metadata](https://api.github.com/repos/k2-fsa/sherpa-onnx/releases/tags/asr-models)
agrees with the reference model's existing643854-byte digest.

The possible later Rust closure is also frozen for inspection; nothing is added
to a manifest or lock here. `ort=2.0.0-rc.15`, defaults off, only
`std,load-dynamic,api-28`; its exact ort-sys dependency also has defaults off.
The source feature closure selects only ort-sys/std+disable-linking+API17..28,
smallvec/const_generics and libloading. No ndarray, tracing, model fetching,
TLS, binary download/copy, pkg-config, GPU/Web/Azure or training feature.
Dev dependencies are not product closure. Cargo's actual resolved feature graph
still requires build-VM proof before any manifest adoption.

| Crate archive | Bytes | SHA256 | Declared licence |
| --- | ---: | --- | --- |
| ort2.0.0-rc.15 |168000|37c87aa7a90b7ff3e0fe36cccd475df8d67f15f07f4d6c370c4b38464232813c|MIT OR Apache2|
| ort-sys2.0.0-rc.15 |59054|8d22b0417f1838db5a3ac0b3f98dfe39f6fb73624aa0a27c6bbb5040b9c7db2b|MIT OR Apache2|
| libloading0.9.0 |33816|754ca22de805bb5744484a5b151a9e1a8e837d5dc232c2d7d8c2e3492edc8b60|ISC|
| existing smallvec1.16.4 |34822|d0d5d4b1981b8bbbb7aed204cae5726d683d5d6ac7e74ec54122ea28f315e7c8|MIT OR Apache2|
| existing cfg-if1.0.5, Unix only |9578|4e7648175b45a9a48536d676f68d918270699102aa8dab5496df06904c914600|MIT OR Apache2|
| existing windows-link0.2.1, Windows only |6133|f0805222e57f7521d6a62e36fa9163bc891acd422f971defe97d64e70d0a4fe5|MIT OR Apache2|

Published [crate records](https://crates.io/api/v1/crates/ort/2.0.0-rc.15)
and pinned [wrapper manifests](https://github.com/pykeio/ort/blob/5c564c58507f35fa4adc387a79fce1016e73956b/Cargo.toml)
are the metadata source. Existing cached archives for the final three rows
were independently size/hash-checked for their notice files. New crate archives
are not yet inspected. Rust1.92 is the wrapper/sys floor and1.88 libloading's;
this is metadata, not successful platform compilation.

### Notices and acquisition gate

Vessel-side [NOTICE](../notices/voice-vad/NOTICE.md) retains Silero v4 MIT,
k2-fsa Apache2 attribution, runtime MIT plus full pinned source third-party
notices, wrapper/sys MIT+Apache2, libloading ISC and locked dependency MIT
notices. [source-manifest.json](../notices/voice-vad/source-manifest.json) pins
each notice's source/bytes/SHA256. Runtime third-party text is325054 bytes,
SHA2560e07b95f3a8d6230037707c5c4a2b554d12c4cb67369669ac255635528ffcee2.
It is a source-wide set, **not** an attestation of either CPU release archive.

If V3 explicitly accepts acquisition: the scope is the three first-table
model/native assets plus the first three new Rust crate archives in the second
table; existing cached dependencies are not downloaded again. Crate URLs use
https://crates.io/api/v1/crates/{name}/{version}/download with those exact names,
versions, sizes and digests. Download into isolated scratch, enforce exact
maximum/published size before retention and verify SHA256 before inspection.
Use anonymous HTTPS, no proxy/environment credentials, connect10s/read60s and
at most2 validated HTTPS redirects to official GitHub release-asset or crate
CDN hosts. Record the initial allowlisted URL, asset id, final host and byte
count only; never retain a signed redirect URL/query or authentication header.

No archive extraction or loader is run. Inventory at most4096 members,512
UTF8 path bytes/depth16,256MiB expanded bytes per native archive and8MiB per
crate. Reject traversal/absolute/duplicate paths, links and device entries;
an upstream link is recorded as a blocker for the final layout, never extracted.
Hash member bytes through a streaming reader. Inspect only small licence/notice
texts (at most1MiB each/8MiB aggregate). The model remains an opaque hashed
file. Do not execute an installer, Cargo build script, inference or native
constructor during that phase. Never use wrapper auto-download or environment
credentials. No change to service accounts, system libraries or permissions.

Return the exact archive member inventory, unpacked sizes, per-library/model
digests and complete archive/source-notice reconciliation to V3. Any unmatched
notice/terms, missing conversion attribution or unexpected library blocks
adoption. A further reviewed amendment must specify the final file allowlist
and target layout before manifest/load/distribution work. This acquisition
request is intentionally narrower than runtime or distribution approval.

### Requirements retained for a later implementation review

- One node-owned runtime factory/model byte source; separate recurrent state
  for each of at most8 active voice leases. Known fixed model/hash/schema only;
 512 finite16k samples, exact h/c state schema and probability validation. Never
  interpret arbitrary uploaded model files or execution-provider options.
- Absolute installer-owned library path and exact digest/version/architecture
  checks precede all ort APIs. Explicit init_from must succeed before any
  Session/Tensor/default environment call; otherwise refuse. Pinned source
  contains an environment/system fallback and panic path if this ordering is
  violated. No application unsafe or custom FFI. This ordering and failure
  behaviour need adversarial subprocess tests before activation.
- Explicit CPU execution, one inference thread per admitted lease and no
  background model fetch. Disable telemetry and install a foreign-log-dropping
  logger before creating sessions. ORT Windows telemetry and wrapper defaults
  are enabled upstream; CPU execution alone is not an egress guarantee. Prove
  offline behaviour and captured stdout/stderr privacy on both platforms.
- Bound model/input/state/actor counts and measure actual native allocations,
  linked/loaded sizes, CPU/latency and reset/drop behaviour. No hard native heap
  bound is claimed by the present source/metadata. Inference failure/OOM/invalid
  outputs must retire the generation without a fabricated transcript.
- Golden model probabilities/detected boundaries must match the pinned reference
  filtering/window lookahead; synthetic score tests alone are insufficient.
  Existing local-Stop, cancellation, own-session and prompt/MCP prerequisites
  remain mandatory. Neither this proposal nor successful endpoint HTTP turns
  on voice or local ASR/TTS readiness.
