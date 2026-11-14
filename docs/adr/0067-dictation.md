# ADR 0067: Dictation, spoken into the node

Status: proposed, 2026-11-14. The protocol number is the one given at
merge. This commit does not change `PROTOCOL_VERSION` (20 in
`crates/vessel-proto/src/node.rs`). New calls are gated on one constant,
`dictation::SINCE`, so the integrator changes that one line. Nothing in
this decision is on the wire until the slice that adds the methods.

The behaviour this follows is Paseo's composer dictation (read for this
decision; file and line in `docs/plans/dictation.md`). Voice mode, local
model download and a microphone in an app are not this decision.

## Purpose and intent

A person speaks, and the node returns text. The audio is a stream with
ordered chunks, a contiguous acknowledgement, partial text while the
person is still talking, and one final text, or a typed failure. The
person can cancel. Nothing in that path is dropped without a refusal
that names what was wrong.

Paseo does this on by default, with a local model unless an
OpenAI-compatible endpoint is chosen. Vessel takes those defaults. The
endpoint is the first provider that runs. The local models are the
second provider: named here, not linked, until this decision is
reviewed.

## Existing implementation and constraints

Vessel has no dictation. Searches of the node and the app find no
speech stream, no dictation setting and no audio provider.

What already exists, and is reused rather than replaced:

- One RPC table (`rpc_methods!` in `crates/vessel-proto/src/rpc.rs`,
  ADR 0002). A method names its request, its response, its scope and
  the first protocol that has it. A new frame kind is not required:
  a call may wait for its answer, as `fleet.watch` does.
- Binary payloads are `Bytes` in memory and base64 on the wire
  (`TerminalBytes` in `crates/vessel-proto/src/terminal.rs`). Audio
  uses the same encoding and its own type, so a PTY buffer cannot be
  passed where speech goes.
- Settings are one object, read with `node.settings` (`read`, since 12)
  and changed field by field with `node.set_settings` (`admin`), kept
  in `settings.json` (ADR 0038, `crates/vessel-proto/src/settings.rs`).
  A field at its default is omitted, so an older client sees the bytes
  it already understands.
- A secret is `secret.set` / `secret.list` / `secret.delete` (`admin`,
  since 6). The value is redacted and is never a settings field.
- Who may call is the method's scope, then the agent's refusal list
  (`crates/vessel-node/src/workspace/policy.rs`) and the forwardable
  list (`Method::forwardable`). A new method does not compile until
  both lists decide.
- `vessel-llm` speaks chat, responses and messages for the model
  proxy (ADR 0015). It does not speak audio transcription. Stretching
  it would put dictation inside the proxy. The transcription POST
  lives in the node's dictation provider and uses the workspace's
  existing `reqwest` (already what `vessel-llm` uses): rustls, no
  redirects, timeouts, a bounded body, the key stripped from errors.
  The standard library does not speak HTTPS. No new crate.

Constraints that bind the shape:

- Ids, rates, sizes, durations and kinds are newtypes or enums. No
  bare string decides behaviour past the decoder. No sentinel (`-1`
  is not an acknowledgement). No `bool` whose call site does not read
  as a sentence. No float on the dictation wire. Paseo's
  log-probability threshold belongs to voice mode, which is out of
  scope; dictation does not store one.
- Audio and the transcript never go into a log, an error message, a
  test failure, the session log or a file. A refusal names a reason
  enum.
- Libraries read no environment and no home. Paseo's environment
  overrides are not copied. The defaults below are Paseo's defaults
  when those variables are unset.
- An older peer in both directions must keep working. Adding a method
  is refused by `since` on a connection that negotiated less. Adding
  a settings field that is omitted at its default does not change
  the bytes an older client already parses.

## Alternatives considered

1. Paseo's messages as they are: unsolicited `dictation_stream_*`
   frames beside the RPC table, audio as a free base64 string, `ackSeq:
   -1` before the first chunk, errors as text plus a `retryable` bool.
   Rejected. Vessel has one method table; a second framing scheme
   splits the decoder. `-1` is a sentinel. A string error cannot be
   matched. A bool does not say which retry.

2. A new `ServerFrame` variant that pushes partials. Rejected. Every
   frame parser and `for_protocol` would grow, and today's
   subscription events are a resumable log. Dictation is not resumed
   and is not stored. A waiting call is the mechanism the table
   already has.

3. One shot: the whole recording in one call, no partials. Rejected.
   Paseo emits partial text as segments commit, and the contract is
   that stream, not a file upload. The CLI in the next slice may send
   a file, but it still speaks the stream.

4. Default the provider to the endpoint, because the local engine is
   not linked yet. Rejected. Paseo's default is local and on. A
   stored default that is not Paseo's would be a migration later.
   Until the engine exists, a start against `local` is refused with
   a typed reason. It is not turned off in secret and it is not
   switched to the endpoint.

5. A new crate for speech. Rejected. The wire types belong in
   `vessel-proto`. The task that holds a stream, the settings and the
   HTTP call belong in `vessel-node`, which already holds settings
   and secrets. No `vessel-*` edge is added.

## The choice

The number given at merge adds five methods, each `operate`, each not
forwardable, each refused to an agent, each gated on `dictation::SINCE`.
`PROTOCOL_VERSION` stays 20 until the integrator sets that constant.
Settings ride on the existing settings calls. The secret rides on the
existing secret calls.

### The stream

`DictationId` is minted by the client, the way Paseo mints
`msg_<time>_<random>`. One to 64 characters, ASCII letters, digits,
`_` and `-` only. Paseo's `dictationId` is any string
(`packages/protocol/src/messages.ts` 1715). Vessel narrows it so the
id can sit in a log field without quoting or escaping. Anything else
is `invalid_request` before a stream opens. The node does not mint
it, because the client must name the stream before the first chunk
and again after a reconnect.

`dictation.start { id, format }` opens the stream on this connection
and answers when the provider session is ready. Readiness is this
reply. There is no acknowledgement of "nothing yet".

`format` is decoded once into `Pcm16Mono { rate }`. The media type is
`audio/pcm`, bits are 16, one channel, and `rate` is an integer from
8_000 to 48_000. Paseo's clients send `audio/pcm;rate=16000;bits=16`.
Anything else is `invalid_request` before a provider is opened.
Paseo sends `format` again on every chunk (`messages.ts` 1720–1724).
Vessel does not. The format belongs to the stream; a different rate
is a new `start`.

`dictation.chunk { id, seq, pcm }` carries one chunk. `seq` starts at
0 and is a `ChunkSeq` from 0 through 1_799. `pcm` is `DictationPcm`:
`Bytes` in memory, base64 on the wire, even length, at most 256 KiB
decoded. The length the contract expects is one second of 16-bit mono
at the stream's rate, `rate * 2` bytes (32_000 at 16 kHz). At the
highest accepted rate that is 96_000 bytes, so one second always fits
in the 256 KiB chunk. Paseo's web recorder cuts on that second
(`packages/app/src/hooks/use-dictation-audio-source.web.ts` 189–195,
`chunkSamples = outputRate` with `outputRate` 16_000) and then emits
one shorter remainder when samples are left (197–198). `vessel dictate
<file>` sends the same shape: one-second chunks of PCM16 at the rate
`start` declares, then one shorter final chunk when the bytes do not
divide evenly. A file shorter than one second is that one short chunk.

A chunk shorter than one second, down to one sample, is a tail. The
stream holds at most one, and only with a `seq` higher than every
`seq` already accepted. It is not committed until `finish` names that
`seq` as `final_seq`. A second short chunk, or any chunk with a higher
`seq` than the tail, is `invalid_request` `BadAudio` and is not stored.
The stream stays, so the client can finish the audio already accepted.
`finish` that names some other `final_seq` while a tail is held is the
same `BadAudio` on the finish call, and the stream stays so a
corrected finish can succeed.

A `seq` above 1_799, or a chunk whose samples would take accepted
audio past 30 minutes, refuses with `TooLong`. The chunk call returns
`invalid_request` `TooLong`, the stream ends, and a waiting
`dictation.next` receives `Failed { TooLong }`. 1_800 chunks of one
second are 30 minutes at any accepted rate, so a client that sends
the expected chunks meets one cap. `seq` is checked before the shape
rules: a `seq` past 1_799 is `TooLong` even when the bytes are also
short.

A chunk that was accepted answers `DictationAck { through }`, the highest contiguous `seq`
the node has accepted, counting from 0. A repeat of an earlier `seq`
answers the same acknowledgement and is not transcribed again. A gap
is held, in order, and does not advance `through`.

`dictation.finish { id, final_seq }` is Paseo's finish. The reply is
`FinishAccepted { within_ms }`, not the transcript. `within_ms` is the
budget below. The transcript arrives on `dictation.next`.

`dictation.cancel { id }` drops the stream and its audio and answers
`Ack`. No transcript is produced.

`dictation.next { id }` waits for the next update:

- `Partial { text }`, the committed segments joined by a single space,
  trimmed, as Paseo joins them. Sent again whenever a segment's text
  changes. Not stored.
- `Final { text }`, once. The stream is then gone. A later call for
  that id is `not_found`.
- `Failed { reason }`, once. The stream is then gone.

Cancelling the `dictation.next` call (the connection's cancel frame)
only stops the wait. The stream continues until `dictation.cancel`,
`dictation.finish`, a failure, or the connection closing. Closing the
connection drops every stream that connection opened, and the audio
with them.

`start` for an id this connection already holds drops that stream
first. A `dictation.next` still waiting on it ends with
`Failed { Replaced }`. Audio of the old stream is freed, not kept.
`start` for an id another live connection holds is `conflict`.

Text is at most 64 KiB of UTF-8. More than that from a provider is
`Failed { TranscriptTooLong }`, not a shorter string.

### Commit, silence, and the finish budget

These numbers are Paseo's, with the environment overrides removed:

- Audio is committed to the provider every 15 seconds of samples at
  the provider's rate (16-bit mono, so two bytes a sample). Paseo's
  default `DEFAULT_DICTATION_AUTO_COMMIT_SECONDS`.
- A block whose peak absolute sample is below 300 is treated as
  silence: it is discarded, not committed. Paseo's default
  `PASEO_DICTATION_SILENCE_PEAK_THRESHOLD`.
- On finish, a trailing silence block is discarded the same way. A
  trailing block that is not silence is committed.
- The provider's "segment too small" failure, decoded at the provider
  edge into that variant, is an empty segment during finish, not a
  stream failure. Paseo matches the words "buffer too small"; Vessel
  matches the variant, never the sentence.
- Finish budget, in milliseconds: base 10_000, plus 15_000 for each
  segment still in flight, plus 1_500 for each second of audio not
  yet committed, plus 250 for each `seq` still missing up to
  `final_seq`, and never above 300_000. The reply's `within_ms` is
  that number. When it expires, `dictation.next` gets
  `Failed { TimedOut }`.
- Finish with no chunk accepted yet is `invalid_request`, detail
  `NoAudio`, on the finish call. The stream ends there. There is no
  second message. Paseo pushes that failure as a stream error after
  accepting the finish; Vessel answers it once, on the call, because
  a stream that never held audio has nothing further to wait for.

The prompt given to a provider, when that provider has a prompt
field, is Paseo's default sentence: transcribe only what the speaker
says, add no words, preserve punctuation and casing, and return an
empty transcript for silence or non-speech. It is not a setting.

Confidence is not a dictation setting and not a field of `Partial` or
`Final`. Paseo's shared speech-to-text client computes
`isLowConfidence` from `confidenceThreshold`, default `-3.0`
(`packages/server/src/server/speech/providers/openai/stt.ts` 216–228).
Voice mode reads that flag
(`packages/server/src/server/session/voice/voice-turn-controller.ts`
69–78 and 180–182). Nothing under `packages/server/src/server/dictation/`
reads it. Voice mode is out of scope, so this contract does not store the
threshold and does not request log-probabilities.

### Provider seam

```text
DictationProvider
  Local
  Endpoint
  Unknown   (a newer node's value; refused on write; start fails)
```

`Local` is the default. This build has no local engine. `start` with
`Local` fails `unsupported`, detail `LocalNotBuilt { model }`, and
names the model id. It does not download, does not fall through to
the endpoint, and does not pretend to be off.

`Endpoint` is an OpenAI-compatible transcription endpoint.
`POST {base}/audio/transcriptions`, multipart, one segment at a time,
`response_format=json`. The body is built in memory. The key goes in
the header the provider's API expects, taken from the node's secret
store under the name the setting holds. Default model `whisper-1`.
Default base `https://api.openai.com/v1`. Default language `en`.
The request does not set `include` to `logprobs`. Paseo does, and only
for `gpt-4o-transcribe` and `gpt-4o-mini-transcribe` (`stt.ts`
202–211), so it can set `isLowConfidence`. Nothing in this contract
reads that flag.

The provider is a closed enum in the node, not a trait with one
implementation. Tests use a fake provider behind the same enum's test
constructor, not the network. The endpoint's tests speak HTTP to a
listener on loopback started by the test.

### Settings, defaults, secret

On `NodeSettings`, one optional `dictation`. Omitted when every field
is the default, so a peer at today's `PROTOCOL_VERSION` (20) sees the
same settings bytes as today. The object is meaningful at
`dictation::SINCE` and above.

| Field | Default | Paseo, when its environment is unset |
| --- | --- | --- |
| switch | on | `DICTATION_ENABLED_BY_DEFAULT = true` |
| provider | `local` | `dictationStt` default `"local"` |
| language | `en` | resolver default `"en"`; blank is unset |
| local model | `parakeet-tdt-0.6b-v2-int8` | catalog `defaultFor: "stt"` |
| endpoint model | `whisper-1` | `model ?? "whisper-1"` |
| endpoint base | `https://api.openai.com/v1` | the SDK's default host |
| key | absent | no key until one is saved |

The switch is `on` or `off`, not a bool. `off` makes `start` fail
`unsupported` with `Disabled`. The value is still kept.

A change (`node.set_settings`, `admin`) names only the fields it
sets. `unknown` provider or an unknown local model id is refused and
writes nothing. Language is a short code: 1 to 16 characters, ASCII
letters and `-`. The base is `https`, a host, no userinfo, no
fragment, at most 256 characters. The endpoint model is 1 to 64
characters, no spaces.

The key is not a field of the settings file. The setting holds a
`SecretName` the admin already stored with `secret.set`. `node.settings`
(`read`) reports `key: present` or `key: absent`, never the name and
never the value: a read-only device learns whether dictation can
authenticate, not which secret it uses. The name is visible to
`admin` through `secret.list`, as every other secret name is.
`start` with `endpoint` and `key: absent` fails `unavailable` with
`KeyAbsent`.

A client that negotiated a protocol below `dictation::SINCE` does not
send these methods (`Call::since`). It ignores a `dictation` object if
a newer node included one (serde's default: unknown fields do not
fail). A node below `dictation::SINCE` has no object and no methods. A
newer client, seeing a negotiated protocol below `dictation::SINCE`,
treats dictation as absent. It does not apply the defaults to an older
node and then call methods that node does not have.

### Privacy: where the audio goes

Audio exists in three places, all memory, all dropped when the stream
ends:

1. The client's buffer, until it has sent the chunk. Not this node's
   problem, and not written by this node.
2. The connection, as a request frame. The frame is not copied into
   the session log, a tracing field, or a file.
3. The stream's task: the reorder hold, the open segment (at most 15
   seconds, or less if a commit happened), and at most two segment
   bodies handed to the provider. The endpoint provider puts one body
   into the HTTPS request and drops it when the response is read or
   the call fails. No temporary file.

The transcript exists only in the `dictation.next` reply. It is not
written to the session log, to settings, or to a tracing field.

Paseo, when `PASEO_DICTATION_DEBUG` is set or `DICTATION_DEBUG_AUDIO_DIR`
is set, writes PCM chunks and a WAV under a debug directory and can
put that path on the final message and on an activity line. Paseo's
endpoint provider also writes the WAV to a temporary file and deletes
it after the request. Vessel does none of that. There is no debug
switch that stores audio.

The key, the base URL's userinfo (which the parser rejects), and the
audio bytes never appear in a log line or an error `Display`.

### Bounds

Paseo does not cap concurrent streams, chunk size, reorder depth, or
total audio. The caps below are a floor, each a typed refusal, never
a silent cut.

| Bound | Value | Refusal |
| --- | --- | --- |
| Chunk | 256 KiB decoded, even length; one second, or one shorter tail | `invalid_request` `BadAudio` |
| `seq` | 0..=1_799 | `invalid_request` `TooLong`, then the stream ends |
| Reorder hold | 32 chunks or 1 MiB, whichever first | `Failed` `ReorderOverflow` |
| Audio accepted per stream | 30 minutes at the stream's rate, 16-bit mono | `invalid_request` `TooLong` on the chunk that crosses it, then the stream ends |
| Commits in flight | 2 per stream; a further chunk waits, it is not dropped | the call stays open until a slot frees or the stream fails |
| Streams | 2 per connection, 8 on the node | `unavailable` `TooMany` |
| Finish wait | at most 300_000 ms, formula above | `Failed` `TimedOut` |
| Transcript | 64 KiB UTF-8 | `Failed` `TranscriptTooLong` |
| Text frame | a chunk's base64 fits the connection's 16 MiB frame (`MAX_FRAME`) | the connection's existing frame refusal |

30 minutes at 16 kHz is 57_600_000 bytes of PCM; at 48 kHz,
172_800_000. The cap is computed with checked multiplication from the
rate. Only the open segment and the reorder hold are retained, not
the whole 30 minutes: committed audio is dropped after it is handed
to the provider.

`dictation.next` ends with `Failed` only after a stream exists:
`Replaced`, `ReorderOverflow`, `TooLong`, `TimedOut`,
`TranscriptTooLong`, `Upstream`, or `Cancelled` (the connection went
away during the wait). `dictation.cancel` itself answers `Ack`.
`NoAudio`, `Disabled`, `KeyAbsent`, `LocalNotBuilt` and `TooMany`
refuse the call that hit them (`finish` or `start`) and do not open
a stream. `Upstream` is the only failure a client should retry, by
opening a new id. The others fail again until the input or the
setting changes. Retry is the reason, not a separate bool.

Wire errors use the existing `ErrorCode` (`invalid_request`,
`unsupported`, `unavailable`, `not_found`, `conflict`, `forbidden`)
plus an `ErrorDetail::Dictation { reason }` with `#[serde(other)]`
`Unknown` on the reason, so a newer reason stays readable. No new
`ErrorCode`.

### Scopes

Paseo requires `workspace.write` for every dictation message, and
`daemon.read` / `daemon.manage` for listing and downloading local
models.

| Call | Scope | Why |
| --- | --- | --- |
| `dictation.start`, `chunk`, `finish`, `cancel`, `next` | `operate` | Sending audio is an action, as `terminal.send` is. A read-only device cannot stream a microphone. |
| `node.settings` | `read` | Unchanged. The dictation object is part of the read. |
| `node.set_settings` | `admin` | Unchanged. |
| `secret.set` for the endpoint key | `admin` | Unchanged. |

An agent is refused all five methods. A microphone is a person's.
Paseo allows them to any client with `workspace.write`, which an
agent-scoped client can hold. Vessel does not.

None of the five is forwardable. Audio does not hop through a fleet
home. A person dictates to the node the connection is on.
`node.settings` is already not forwardable.

Local model list, download and delete are not methods of this
decision. They belong to the slice that links an engine. When they
exist, list is `read` and download and delete are `admin`, matching Paseo's
`daemon.read` and `daemon.manage`, and they are not forwardable
(they write the node's disk).

### Local models, proposed and not added

Not a dependency of this decision. The slice that adds one records
the measured library size on each platform in an amendment here
before the manifest changes. Sizes below that are not measured are
marked so.

Engine: `sherpa-onnx` (the C++ library Paseo's `sherpa-onnx-node`
1.12.28 binds). License Apache-2.0, from that repository's `LICENSE`
on its default branch. It brings ONNX Runtime, which is MIT. No
second ONNX crate. Platforms it publishes, which are the platforms
this repository already builds: Linux x86_64 and aarch64, macOS
x86_64 and arm64, Windows x86_64. CPU only; a GPU is not required.
The linked library's byte size was not measured for this decision.
Do not add the crate until that size is written down.

Models, the speech-to-text rows of Paseo's catalog. Archive sizes are
the exact sizes Paseo refuses any other download for. Licenses are
the weights' licenses, not sherpa-onnx's license. A conversion
repacked into a sherpa archive still needs that archive's notice
checked before a download is implemented.

| Id | Archive bytes | Weights' license | Role |
| --- | --- | --- | --- |
| `parakeet-tdt-0.6b-v2-int8` | 482_468_385 | CC-BY-4.0 (NVIDIA model card for `parakeet-tdt-0.6b-v2`) | default |
| `parakeet-tdt-110m-en-int8` | 108_035_095 | CC-BY-4.0, same family; confirm the card before adding | optional |
| `parakeet-tdt-0.6b-v3-int8` | 487_170_055 | CC-BY-4.0 (NVIDIA v3 card, same terms as v2; confirm) | optional, 25 languages |
| `parakeet-tdt-ctc-0.6b-ja-int8` | 489_389_564 | confirm before adding | optional |
| `whisper-tiny-en` | 118_071_777 | OpenAI Whisper is MIT; confirm the archive notice | optional |
| `whisper-base` | 207_557_382 | MIT, same confirmation | optional |
| `whisper-turbo` | 563_790_207 | MIT, same confirmation | optional |
| `sense-voice-int8` | 165_783_878 | confirm the FunAudioLLM card and the archive notice before adding | optional |
| `dolphin-base-int8` | 80_671_385 | confirm before adding | optional |
| `giga-am-v3-ru` | 170_197_019 | confirm before adding | optional |

Voice-mode TTS models (Kokoro and the rest of that catalog) are not
part of dictation. Silero VAD, which Paseo bundles for voice mode, is
not part of dictation: silence here is the peak threshold, not a VAD
model.

No model is downloaded by this build. A `local` start names the model
and stops.

## Why this and not the others

The method table is how Vessel adds a call without a second protocol.
A waiting `dictation.next` is a push to the one client that opened the
stream, without a log and without a new frame. Defaults stay Paseo's,
so the later local engine does not migrate settings. Caps exist where
Paseo is unbounded because an open stream holds memory on a node that
serves more than one person. The key uses the secret store because
that store already redacts, and a settings file is not a place a key
survives a backup unnoticed.

## Advantages, costs and effects

Advantages: the compiler owns the method list, the scopes, the agent
refusal and forwarding. Audio has a bound and a place it is allowed to
be. An older client is unchanged when the setting is at its default.

Costs: five methods for what Paseo does with fire-and-forget messages.
A client holds one `dictation.next` open while it sends chunks, which
Vessel's connection already allows (concurrent calls). Base64 costs
about a third on the audio, the same trade `TerminalBytes` records.
The default configuration refuses `start` until a person chooses the
endpoint and stores a key, or until the local engine is linked. That
refusal is the cost of not lying about a model that is not there.

Effects, when the implementation slice lands:

- `vessel-proto` gains the types and the five rows. `PROTOCOL_VERSION`
  stays 20. `dictation::SINCE` becomes the assigned number, and it is
  the only version this track names. `docs/PROTOCOL.md` records both
  directions in the commit that publishes that number.
- `vessel-node` gains the stream task and the endpoint provider. No
  `vessel-*` edge is added. `reqwest` is used from the node as an
  existing workspace dependency; the external-owner table does not
  give it a single owner.
- `Method::forwardable` and the agent refusal match each gain five
  arms. Both are exhaustive, so missing either does not compile.
- The CLI's `vessel dictate` is the slice after that, a client of
  these methods, not a second implementation.
- Generated TypeScript is regenerated in the slice that changes the
  wire, not in this decision.

What this decision does not do: link `sherpa-onnx`, download a model,
open a microphone, implement voice mode (including its confidence
flag), or choose the protocol number. The integrator sets
`dictation::SINCE`.
