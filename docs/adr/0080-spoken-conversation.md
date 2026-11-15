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

Status: **V3 row221 PASS for quarantined acquisition/inventory only**, on
exact docs commit0a6b0e641. The six fixed acquisitions and inventories were
performed within that scope; findings are recorded below. It does not authorize a dependency
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


### Proposed Phase A Windows continuation after the enforced refusal

Status: **V3 row221 continuation CONTRACT PASS**, on exact docs commitda77d7d18.
The bounded continuation was performed; it does not authorize final layout,
manifest, load or distribution. The first
[Phase A inventory](../plans/wp-voice-vad-inventory.md) stopped before
Windows member26, the408244224-byte `lib/onnxruntime.pdb`; the existing256MiB
expanded-member cap was enforced without inflating that member. Linux links
were rejected and remain unextracted. This finding does not broaden row221.

Propose a second bounded inspection of the **same already-verified** Windows
archive (asset542003886, SHA256c4eedd29489d5feca21866d054638416f3655bf6b18851b3b6b85c8313e95c35),
with no new download or fallback. Its purpose is to finish metadata/library/
notice correspondence while excluding debug symbols from every product plan.

The proposed exception is metadata-only for exactly
`onnxruntime-win-x64-1.28.2/lib/onnxruntime.pdb`, with declared expanded
size exactly408244224. Do not inflate, extract, hash or load its payload.
Reject any additional oversized member or changed PDB identity/size/type.
An explicitly skipped payload has no per-member digest and must never appear
in a loadable-file allowlist. Full archive SHA256 still authenticates its bytes.

Before allocating directory entries, enforce4096 members, a central-directory
byte limit1MiB, UTF8 paths512bytes/depth16 and a declared metadata-only total
size limit512MiB. This **new metadata declared-size limit** needs explicit
V3 approval; it is not silently substituted for the original expanded cap.
Every path, type, duplicate, local-header bounds and method is still checked.
Links/devices/encryption/ZIP64 remain rejected.

All other payloads retain the original256MiB aggregate actual-inflation budget,
1MiB individual/8MiB aggregate notice budget, bounded decompression chunks and
CRC/SHA validation before accepting a member. Record complete member metadata
with the PDB labelled skipped/not-hashed; hash every other regular member.
No container extraction, Cargo/build-script/installer execution, dynamic
library/model load, manifest, distribution or activation is authorized by
this request. A final reviewed file/layout amendment remains mandatory after
notice correspondence; Linux aliases and all debug/test DLL assets remain
excluded unless separately reviewed.


### Continuation receipt: complete metadata and notice correspondence

The [complete Windows inventory](../evidence/voice-vad-phase-a/windows-continuation.json)
contains34 entries/4646 central-directory bytes/425969843 declared metadata
bytes. The sole approved PDB payload is explicitly skipped, with no SHA or
verified payload CRC, zero inflated bytes and mandatory product exclusion.
All other entries total17725619 actual inflated bytes; CRC/SHA checks passed.
The separate405504-byte providers-shared PDB was handled normally within the
original budget and is also excluded from future product plans. No exception
was generalized to other PDB names or extra payload bytes.

Windows LICENSE1094B/SHA c250d6278f0b47a6439fb7592b08b58a55eb9f535aa49a1db63211c3f982b674
and ThirdPartyNotices331175B/SHA fb0af774b4d7cffc5b9d046f2aaeade2f37df2f80abf8033c95dfffcc77a8866
are **not raw-byte matches** to the LF source notices. Full bounded
[notice reconciliation](../evidence/voice-vad-phase-a/windows-notice-reconciliation.json)
proves the only difference is CRLF:21/6121 pairs respectively, no bare CR, and
normalizing those pairs produces byte-exact previously pinned source texts.
No wording/attribution was dropped or compared by a prefix alone. Source LF
notices remain retained; this correspondence is not legal clearance, ABI/model
compatibility, a final installation layout or distribution approval.

Five synthetic continuation guards passed: exact skip never invokes inflater/
hasher/CRC, changed PDB size, different PDB name, declared-total cap before
entry materialization and central-directory byte cap. These are parser tests,
not inference or physical-device evidence. The initial enforced Windows refusal
and all original limits/results remain preserved alongside this continuation.

## Proposed Phase B: fixed private layout and dependency build proof

Status: **V3 row221 Phase B CONTRACT PASS**, exact437739b2c..05984b9a0,
preparation/compile-only. Private selective extraction and bounded static
inspection are now recorded below. No dependency/lock change, Cargo build
script, model/native load or distribution has been performed on this branch. This is a bounded next gate; it does not change the closed
Phase A policy, its preserved refusal or skipped PDB evidence.

[Decision Log]
- Purpose: select the exact regular files needed to develop the required VAD,
  while leaving runtime/model execution and product activation behind later
  reviewed source and compatibility checks.
- Existing assets: Phase A verified every selected file's full archive/member
  size and SHA; Windows notices have a completed full CRLF-only comparison.
  The approved Rust candidate closure and retained licence files already exist.
- Alternatives: a system ORT search, wrapper auto-download, whole-archive
  deployment, recreating upstream links or importing a Sherpa ASR/TTS bundle
  would widen trust, files and licence scope without helping this detector.
- Choice: one fixed, private component root and explicit platform subdirectory,
  a positive regular-file manifest, fixed Rust features and build-VM-only proof.
- Why: absolute versioned-file loading needs no unversioned Linux alias;
  development must not accidentally admit the large PDB or crate test DLLs.
- Impact: CPU x86_64 candidates only; unavailable system ABI prerequisites,
  feature drift, unexpected imports or notice mismatch block the next stage.
  Product runtime support, downloads and local ASR/TTS remain absent.

### Exact layout candidate and notice correspondence

The machine-readable [positive file manifest](../evidence/voice-vad-phase-b-layout.json)
is derived only from the committed completed inventories. Each source member,
relative destination, exact byte count, SHA and role is explicit. The model
stays opaque at `model/silero-v4.onnx`. Each target directory contains two
regular native candidates under `lib/`, original LICENSE/ThirdPartyNotices/
Privacy text, VERSION_NUMBER and GIT_COMMIT_ID. The Linux entry is the actual
`libonnxruntime.so.1.28.2`, not a recreated `.so`/`.so.1` symlink. The Windows
entry is `onnxruntime.dll`; its providers-shared DLL is co-located, as is the
Linux providers-shared library. Co-location is a candidate dependency layout,
not proof of which libraries are loaded or whether their imports resolve.

The selected native payload is24291672B on Linux or15839856B on Windows;
model bytes643854 are common to either choice. These are file sizes, not
resident/linked size or installer overhead. No debug symbols, headers, import
libraries, static archives, test DLL, GPU/provider library, alias or unlisted
file is loadable under this manifest. All PDBs are excluded regardless of
whether their payload was normally hashed or explicitly skipped in Phase A.

Retain the platform archive's original notices byte-exact, including Windows
CRLF, alongside the Vessel-side source notices and NOTICE attribution. No
notice is removed because its component seems optional. Full source-wide
ORT third-party text remains distinct from binary component correspondence;
the matching archive text is not a per-object licence/ABI attestation. Preserve
Silero MIT and the k2-fsa conversion attribution plus Apache2, both wrapper
licences, libloading ISC and existing transitive notices. The positive manifest
pins each unique source notice once and points to the Vessel attribution file;
that file must accompany the complete component in any later reviewed package.
This proposal contains no Kokoro/Kitten/Piper/espeak data or local ASR/TTS.

### Requested private preparation only

V3 accepted ONLY the following next phase. The build owner currently requires
its fixed handoff schema before actual fixture creation or transfer; that
prerequisite is still pending:

1. Re-verify the same approved archives and opaque model. Stream only the
   positive native/notice/metadata members into a fresh task-owned private
   directory outside product trees. Reuse the bounded parsers and original
   member/count/path/CRC/digest rules; do not create aliases or traverse links.
   Files remain0600, directories0700, owned by the task UID/GID. Temporary
   files are accepted by exact size+SHA before atomic rename; on mismatch no
   candidate final file is retained. Preserve original archives and first
   refusal. No new artifact URL or fallback download is introduced.
2. Perform **non-executing** static ELF/PE import/architecture inspection of
   those exact extracted native candidates, and read only bounded version/git/
   privacy metadata. Record the imported system ABI libraries, relative/dynamic
   dependencies and any additional terms. Never use ldd or a loader as an
   import inspector. Unknown non-system dependencies or changed source/version
   identity block adoption; do not fetch/install missing libraries.
3. Add the exact previously inventoried ort2.0.0-rc.15 candidate to the root
   workspace as a **node dev dependency only**, defaults off, solely std,
   load-dynamic and api-28. This keeps the existing outer test-only detector
   implementation staged and adds no product runtime availability. Resolve
   on the build VM and retain the full lock/feature tree; the allowed new
   product closure is only ort, ort-sys, libloading and the already inventoried
   smallvec/cfg-if/windows-link versions and features above. Any additional
   dependency, runtime download or feature activation is a stop condition.
   Normal exact-checksummed Cargo registry source unpacking is distinct from
   the positive native fixture layout: libloading's test DLLs may exist only
   as inactive source-cache fixtures, never in that layout or loader search.
4. Review ort-sys's pinned disable-linking build branch before permitting its
   build script to run, then compile the staged wrapper only on the build VM.
   No build-time runtime/model fetch, installer, pkg-config, native link search
   or environment-derived provider settings are allowed. Reconcile the actual
   feature closure with the locked proposal and retain actual crate Checking/
   Compiling evidence. Dependency compilation alone must not call an ORT API.

No model or native library may be loaded during this preparation. No setup
script or upstream executable/test binary is run. No product-tree extraction,
service/system library write, permission/account change, installer, automatic
model download, native distribution, runtime advertisement or voice activation
is authorized by the requested phase. Final release packaging is a separate
review after runtime/source/platform proof.

### Subsequent detector/loader checkpoint remains mandatory

After preparation, return the exact extracted-file hashes/modes/ownership,
static imports and metadata, actual Cargo closure and compiler receipt to V3.
Submit loader/detector source before any native constructor or model session
runs. It must enforce explicit verified absolute path initialization before
all ort APIs, CPU-only environment, disabled telemetry and a foreign logger
that retains no message/text/audio. Its recurrent input/output schema, reset,
finite probability, per-lease resource bounds, full-queue Stop and inference
failure/cancellation handling require separate tests. Generic ort fallback or
panic paths cannot become the error policy.

There is currently no supported gate API to inject private external binary
fixtures into a VM label. A named, digest/size/ownership-preserving fixture
handoff is now assigned to the build-tool owner by the coordinator: a fixed
voice-label allowlist, exact SHA/size manifest, UID/GID/mode preservation,
per-file receipt and refusal of mismatch/extra files/links. No handoff has run.
Prepare only task-private0600 files/0700 directories after this phase is
approved; never commit binaries into Git or substitute arbitrary SSH builds.
Both-platform offline/log privacy, ABI and reference probability/boundary
compatibility, native allocation/latency/loaded size and hardware quality are
still owed. This phase cannot establish a hard native heap bound or unblocked
native cancellation. W22/lifecycle/MCP prerequisites remain independent.


Phase B review independently matched all14 platform files, the model and9
unique source notices. Mandatory NOTICE.md must be copied from the exact
reviewed05984b9a0 source with its full size/SHA included in handoff receipts.
The wrapper has build=false; its load-dynamic feature includes preload-dylibs
and ort-sys/disable-linking. V3 read the complete pinned ort-sys build main
(SHA2569d4af3dc20d1462964e80e7829446012acd4effa262d82cd3a11640341f07dc3):
with verified disable-linking it returns after check-cfg emission, before any
lookup/copy/download/link branch. This is source proof, not actual resolved
Cargo features or a compiler result. DOCS_RS is not a substitute for the
required branch. The actual lock/feature tree must be verified before its
build script executes; no native/model API is authorized.


### Phase B private preparation receipt, no build/runtime result

The [25-file receipt](../evidence/voice-vad-phase-b/preparation.json) matches
T's exact fixed-v1 manifest SHA256eac00e475b80f805b52b44f57fe9177344cfb87095e87965461eedfb41488fa7:
41785720 total bytes, UID/GID1000:1000, files0600 and directories0700, no
extra file/link. Full reviewed NOTICE.md is3211B with SHA256
56aca735a4499d5372d9c4205a775727f33e66b953ce3ae07a0b6be3a230dfae.
All approved input archives/model were reverified before streaming only
selected files; full container validation/CRC completed before final rename.
Linux aliases and every PDB/test DLL remain outside the private candidate
root. The skipped408244224B PDB was never inflated or hashed. No binary or
model bytes were committed to Git.

[Static imports](../evidence/voice-vad-phase-b/static-imports.json) identify
both ELF64/x86_64 and PE/AMD64. Each inspector had a whole5s deadline and
256KiB combined-output cap enforced during reads. The initial objdump private
headers command exceeded that cap on Windows and was refused; preserve its
source/log/partial Linux evidence. A narrower llvm-readobj file-header/import
inspection completed with the SAME limits; four outputs total35109bytes.
No ldd, native constructor, upstream executable or model API ran.

Linux runtime's SONAME is libonnxruntime.so.1 and RUNPATH is $ORIGIN; direct
absolute versioned-file loading remains a candidate, with no recreated link.
Its NEEDED entries are libdl/librt/libpthread/libstdc++/libm/libgcc_s/libc and
ld-linux-x86-64; providers-shared imports the C/C++/math runtime set. These
static names are not actual loader resolution/availability proof.

Windows imports include KERNEL32, ADVAPI32, SETUPAPI, dbghelp, dxgi, API-set
contracts, MSVCP140/MSVCP140_1 and VCRUNTIME140/VCRUNTIME140_1. The delay-import
directory is zero in both candidates. The CPU archive does import dxgi;
CPU-only inference must not be inferred from that archive name or import list.
MSVC/system ABI availability and actual loaded dependencies remain unverified;
no additional DLL is fetched, installed or added to the positive manifest.

[Metadata](../evidence/voice-vad-phase-b/metadata.json) reports1.28.2 and source
33ca9628233dc8f002435e868d4c2e9f82766ca1 on both platforms, preserving original
line endings. Full retained Privacy.md explicitly states official Windows
telemetry defaults ON. This reinforces the required disabled-telemetry/custom
logger/offline source/runtime checkpoint; this static inspection proves none
of those postconditions.

Author-owned extraction/inspector sources and all output hashes are frozen
privately with five synthetic guards (path traversal, source links, minimal
child environment, output bound and whole-timeout group cleanup). The guards
are not native/ABI/model tests. T's importer remains pending and no VM transfer
has happened. A separate private dependency-preparation branch records only
the exact ort DEV manifest candidate; offline lock/feature receipt and V3
acceptance must precede any compilation/build-script execution.

## Real upload transport prerequisite discovered before wiring

The current native uploader is a typed fake-adapter composition, not evidence
that generic client RPC retracts queued audio. The shared RPC PendingCall drop
only tries to enqueue Cancel; a full command queue can lose that request. Its
writer currently handles an already queued Command::Call without an independent
per-call validity check. Cancelling the waiting future therefore cannot by
itself prove that uncommitted PCM has been discarded at the real writer.

Do not wire voice upload through ordinary Client.call on that assumption. A
single shared transport owner must provide and review typed cancellation at
the writer admission boundary, independent of the full data/control queue.
The proof must hold a transport/sink, fill queues, drop/Stop/mute the call and
show queued uncommitted PCM is never fed later. Define the exact commitment
point and distinguish bytes already admitted to a transport from retractable
local audio; do not claim remote receipt timing can undo in-flight bytes.
Any cancellation during partial framing must retire the affected transport
safely rather than corrupt a subsequent frame or silently restart capture.

The coordinator assigned the isolated shared RPC leaf to W19 on announced
node head9c150c6e. Its source reuses the per-call oneshot closure at dequeue
and during registration/sink readiness waits, with no public client API change.
The helper registers a live waiter immediately before synchronous transport
admission, so skipped calls leave no phantom registration. Four memory-only
regressions and the full VM RPC suite are queued, not a result. This package
has not adopted the leaf and real voice transport remains unconnected. This prerequisite
adds no call, protocol number, fallback or permission to activate voice. The
existing local device/upload fences and uncertain remote cleanup remain
required in addition to the transport proof.


V3 accepted the PhaseB private preparation/static receipts as evidence only
on e7f1f8c09. Exact25-file hashes/modes/ownership, notice and bounded inspector
records were independently rechecked. This closes that preparation debt, not
system ABI, model/native execution, telemetry/privacy, compile or activation.
The build owner has installed the fixed RO/noexec importer; actual transfer
is still0 and awaits the offline lock/feature receipt and its V3 acceptance.

## Dependency decision after the offline-index refusal

Status: **A1 selected by the coordinator on 2026-11-15; staged provisioning,
resolve and compile gates only. Native/model loading remains unapproved.**
T's exact old-base run wp-voice-resolve-20261115T155318Z-1a3570d4 stopped at
`cargo update --workspace --offline`: ort was absent from the offline index.
The feature-tree command did not run. Before/result Cargo.lock are both110992B,
SHA2564184d1bddc304bede01b965d166d70154ae6f9365c24631572fd7c3ad9e6a411;
empty patch/tree are failure artifacts, not successful generation. No network,
compiler/build script, native/model API or fixture transfer occurred.

[Decision Log]
- Purpose: run the same pinned16k-only Silero v4 model with an auditable offline
  Linux/Windows build, instead of accidentally enabling ORT's downloader.
- Existing assets: exact1.28.2 CPU regular-file/notices inventory, opaque model,
  reviewed wrapper/sys source and fixed private fixture manifest already exist.
  They establish file/feature provenance, not VAD accuracy or latency.
- Alternatives: default ort; explicitly dynamic ort; explicitly supplied native
  link assets; self-contained tract-onnx; Sherpa or a different detector.
- Selected choice: A1, defaults-OFF dynamic ort with ONLY std, load-dynamic
  and api-28. It has the smallest already-inventoried new closure and keeps the
  reference ONNX backend. This selects a dependency path, not an adopted runtime.
  A0, the default downloader, is rejected. B (tract) is a documented fallback
  and is not provisioned now.
- Why: the default downloader is rejected. No measured comparison currently
  justifies either a tract speed claim or rejecting it for accuracy. A different
  VAD would change the required probability/boundary behavior.
- Impact: T is authorized to provision only the three exact new package records
  and archives below with a checksum-bound receipt. Next comes fixed offline
  scratch resolve on the current node base and lock/feature-tree review, then a
  separately reviewed offline compile for Linux and Windows MSVC on the build
  VM. No model/native load, automatic download or distribution follows.
- Fallback trigger: reconsider B only if A1 fails a Windows build or load gate,
  exceeds the 32 ms p99 window budget, or produces an unacceptable installer
  size. Installer acceptability needs an explicit decision using measured size;
  no invented numeric size waiver is implied. Loading and any fallback
  acquisition still require their own reviews.

### Options and offline Windows consequences

| Option | Build/runtime assets and features | Licence/size evidence | Windows consequence / status |
| --- | --- | --- | --- |
| A0 default ort | Defaults include download-binaries, copy-dylibs, TLS/tracing/ndarray and api-31. Not selected. | This expands beyond the reviewed CPU1.28.2 closure. | Network-backed native provisioning is incompatible with our offline build; rejected. |
| A1 explicit dynamic ort | Exact ort/ort-sys2.0.0-rc.15; defaults false; only std/load-dynamic/api-28. load-dynamic selects preload-dylibs and sys/disable-linking, API17..28 transitively. No downloader/copy/pkg-config/GPU/fetch-models/tracing/ndarray/TLS. | ORT MIT plus full source/archive notices; wrapper MIT OR Apache2; libloading ISC. Selected native payload24291672B Linux /15839856B Windows plus common643854B model. Linked/RSS delta unmeasured. | Rust/sys compilation needs no ORT import library or DLL: disable-linking returns before native lookup. Build x86_64-pc-windows-msvc with the approved VM SDK/toolchain offline; later reviewed loader verifies an absolute DLL path/hash, disables telemetry and drops foreign logs. Actual Windows build/load remains unproved. Selected dependency path A1. |
| A2 supplied native link assets | Defaults false with download/copy off, explicit pinned platform library/import-library inputs; no implicit system search or fetch. | Same ORT terms, but Windows import .lib and linker layout require a NEW positive-file amendment; they are excluded from current fixtures. Source-build ORT would add a much larger CMake/third-party source cache and notices. | Native link/loader search and MSVC CRT proof grow; current dynamic-only approval does not cover it. Not selected for this small detector. |
| B tract-onnx0.23.10 | Evaluation proposal defaults false, transformer/getrandom-js off; no GPU/CLI/backend extras. Same opaque Silero bytes, no ORT DLL. Full normal/build closure still must be resolved, pinned and inventoried. | Original crate work MIT OR Apache2; upstream ONNX/TensorFlow protocol files have separate notices. The full closure and compiled size/RSS are unmeasured/unapproved. Model licence/size unchanged. | Rust MSRV1.91 fits our1.93 floor, but tract-linalg has cc/build templates and generated SIMD assembly. Its Windows-specific assembler path must pass the approved Linux-host MSVC cross-build; self-contained runtime does not eliminate build scripts/native kernels. No support or speed claim before proof. |
| C existing alternatives | Sherpa wraps more native machinery; energy/RMS or another learned VAD changes the model. NNEF preconversion introduces another model artifact to pin. | Sherpa source candidate Apache2 was inspected earlier, native closure unselected. No new model/archive is acquired here. | Larger dependency/conversion review or different turn semantics; not a substitute for the required Silero parity. |

The already-retained exact ort source is the authority for A0/A1 features;
V3 independently checked its sys early-return branch. For tract, source-only
facts above are from the exact version tag's [ONNX manifest](https://github.com/sonos/tract/blob/v0.23.10/onnx/Cargo.toml),
[linalg manifest](https://github.com/sonos/tract/blob/v0.23.10/linalg/Cargo.toml),
[build source](https://github.com/sonos/tract/blob/v0.23.10/linalg/build.rs) and
[MSRV](https://github.com/sonos/tract/blob/v0.23.10/Cargo.toml). The
[upstream overview](https://github.com/sonos/tract/blob/v0.23.10/README.md)
describes its self-contained runtime/format approach and notice exceptions.
These are documentation/source inspections, not dependency archive acquisition,
Windows compilation or proof that this particular Silero graph is supported.
No tract archive/closure/hash is selected for fetching. Its comparison remains
a fallback study under the triggers recorded above, not a parallel provision.

### Authorized offline-index provisioning and ordered gates

For A1, the ONLY candidate new package/version records and matching cached
crate archives are ort2.0.0-rc.15, ort-sys2.0.0-rc.15 and libloading0.9.0;
existing smallvec1.16.4, cfg-if1.0.5 and windows-link0.2.1 records/artifacts must
match the already pinned checksum table above. Do not refetch existing caches.
T must bind sparse-index records, exact registry checksums and archive bytes in
a receipt, retain provenance, and refuse any extra version/dependency/feature.
No ambient credentials/config are copied, no build script populates the cache,
and no runtime archive is fetched by Cargo. Only T owns this authorized cache
provisioning, preferably reusing the already verified exact archives. After its
receipt, run fixed offline scratch resolve on the current node base, review
the lock diff/feature tree, then allow a separately reviewed offline compile
for Linux and Windows MSVC. Do not fetch or modify a lock in this document step.
The old82 result cannot qualify a new node-base graph.

For B, offline provisioning would additionally need its complete exact
normal+build graph: tract-onnx/hir/core/data/linalg/nnef/onnx-opl/extra and their
resolved third parties, including code-generation/compiler helpers. Versions,
archive hashes, licences/notices and actual enabled defaults must be inventoried
BEFORE any fetch approval; no made-up complete closure is asserted here. CPU
only, no transformers/GPU/Web/CLI features and no runtime network fetching are
constraints, not proof of the unresolved graph. Its default features and
transitive build scripts cannot be accepted merely from the phrase pure Rust.

Both candidates' Windows builds must run on the build VM with the approved
x86_64-pc-windows-msvc SDK/cross-toolchain and frozen offline source cache.
Record the exact target/toolchain, full features and linker/compiler evidence;
never install a runtime or download an SDK from a package build script. A1
compiles bindings without linking native ORT; B must also compile its selected
kernel/assembly path. Neither has an actual Windows result yet.

### Measured comparison is still missing; frozen measurement contract

| Measurement on the pinned Silero fixture set | A1 Linux | B Linux | A1 Windows | B Windows |
| --- | --- | --- | --- | --- |
| Model load/schema/state-reset compatibility | not executed | not executed | not executed | not executed |
| Window probabilities / recurrent-state parity | unmeasured | unmeasured | unmeasured | unmeasured |
| Speech precision/recall/F1 | unavailable: no labelled speech set | same | same | same |
| Warm p50/p95/p99 latency, cold setup, RSS, binary size | unmeasured | unmeasured | unmeasured | unmeasured |

No benchmark from another machine/model, crate-source size or fake probability
unit test is substituted for those numbers. Current approval explicitly bars
native/model entrypoints; tract model loading is also a model API. The missing
offline index prevents even candidate compilation. Measurements therefore
cannot be honestly supplied before provisioning and subsequent source/runtime
permission. This is a remaining decision condition, not a pass or waiver.

The [frozen original PCM fixtures](../evidence/voice-vad-benchmark-fixtures.json)
contain silence, independently synthesized tones, impulses and a saturation
stress pattern at16k mono PCM16. No microphone/provider/reference audio is used.
They can measure numerical equivalence and synthetic-negative events, not
speech recall or real-device accuracy. Hashes are fixed before either backend
runs; files remain private and OUTSIDE the existing25-file importer allowlist.
No fixture was transferred to the VM by this work.

After explicit acquisition/source/execution approval: run the SAME model SHA,
window/lookahead/order, conversion and zeroed/reset h/c states on both backends;
record every finite probability and state-shape check. Use a predeclared maximum
probability delta1e-4, exact threshold/boundary event agreement and identical
chunk-invariance outcomes; failures block substitution. This tolerance is a
proposed numerical criterion, not an observed error. Keep existing0.5/0.35,
0.1s/0.2s internal and800ms/1000ms outer policy fixed. Benchmark each fixture
with100 warm windows and10 repeated passes, separate cold setup, report p50/
p95/p99 per512-sample window, peak RSS and one/eight independent lease cases.
A warm p99 below the32ms window budget is the proposed latency gate; no value
is measured yet. CPU model/thread count, toolchain, OS and exact build/source
hash must accompany results. Capture stdout/stderr with finite time/byte bounds;
network remains disabled and all privacy/reset/cancellation checks still apply.
A licensed labelled speech fixture set needs its own source/licence/hashes
before any precision/recall claim or acceptance of acoustic parity.

### Windows pre-environment telemetry ordering remains a load gate

Source-only investigation after selecting A1 found a qualification gap; it does
not change the allowed offline dependency preparation/compile sequence.
The exact retained ort-rc.15 `src/environment.rs` is22542 bytes, SHA256
f3981bcddc584c6437c43a3303df2145c963c2be8a3ac918487d434ad60c755c.
Its builder creates the environment (lines529–582), then applies
`with_telemetry(false)` via DisableTelemetryEvents (586–590). A custom logger
is installed at environment creation, but does not itself disable platform ETW.
These source bytes were read into bounded memory from the already verified
archive; no native code or model was loaded or extracted for this inspection.

At the candidate native source commit33ca9628233dc8f002435e868d4c2e9f82766ca1,
[Environment::Initialize](https://github.com/microsoft/onnxruntime/blob/33ca9628233dc8f002435e868d4c2e9f82766ca1/onnxruntime/core/session/environment.cc#L352)
calls startup LogProcessInfo before environment creation returns. The
[Windows implementation](https://github.com/microsoft/onnxruntime/blob/33ca9628233dc8f002435e868d4c2e9f82766ca1/onnxruntime/core/platform/windows/telemetry.cc#L130)
starts enabled, registers an ETW provider, and gates process-info emission on
provider registration and its enabled flag. Thus the builder's later disable
is not proof that no startup event could precede it. This is a source ordering
inference, not an observed Windows event, network transmission or binary ABI
attestation; the actual host's ETW consumer state has not been inspected.

The pinned [C API implementation](https://github.com/microsoft/onnxruntime/blob/33ca9628233dc8f002435e868d4c2e9f82766ca1/onnxruntime/core/session/onnxruntime_c_api.cc#L280)
currently ignores the environment argument when disabling telemetry, but that
does not make a null argument a documented safe Rust initialization contract.
Do not introduce an unsafe/null-pointer call, patch an official binary, depend
on the operator's ETW settings, or use a network-isolated test as a substitute
for resolving this ordering. No Vessel-side safe pre-environment disable path
has been established.

Before native Windows loading, the separate V3 loader checkpoint must settle
this startup boundary with a supported, reviewed initialization mechanism or
another explicitly reviewed asset/backend decision. A telemetry-free native
rebuild would require a new exact asset/build closure, hashes and notices; it
is not admitted by the current positive manifest. The documented B fallback
may be reconsidered if this blocks the Windows load gate, under the existing
coordinator ruling; it is not automatically selected or provisioned. A1's
compile-only approval, full post-initialization telemetry disable, discard-only
logger, and later CPU/model/schema/resource/privacy gates remain distinct.
