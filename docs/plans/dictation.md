# Dictation

Node-side speech to text, following Paseo's composer dictation. ADR 0067
is the contract. The protocol number is `dictation::SINCE`, given at
merge. This plan does not change `PROTOCOL_VERSION`.

## Current resume point (2026-11-15)

The designated head d432ef0f and source-reviewed file-diagnostic prerequisite are
on the feature branch. Product wiring now connects the real bounded registry,
provider, canonical ConnectionLife accessor and shared codec, with five pending
method-table rows. The protocol constant remains None. The generated guard checks
Operate and the domain context refuses agents. Those are implementation drafts,
not a product test or merge pass. The CLI now connects its validated file reader
to a serial acknowledged sender and concurrent next consumer, with bounded
finish/cleanup waits and final-only stdout. Those new cases are unexecuted.

Settings now validate the whole merged candidate, resolve explicitly selected
keys even on a no-op, and use the existing single-lock atomic writer/apply order.
Hidden old/unassigned responses skip presence I/O. Actual-write failure, deletion
race through start, typed historical downgrade, scope/agent checks on all five
assembled handlers, real connection close, stop-tracker joins and private wire
causes have new unexecuted regressions. A real-provider loopback service case
checks a short capture tail, partial and final.

Next: narrow strict checks and focused tests on this committed wiring, fix
actual failures in the backend and file command/driver. Regenerate declarations
for the final registered surface and run one whole product/package gate. The
previous generated snapshot passed four tests but was stale; it excludes later
ErrorDetail/registration additions and is only an interim generated artifact.
Earlier resume paragraphs below are historical checkpoints. Local engines,
app capture and TTS remain deferred. No real ASR, HTTPS credential, platform
service-account or microphone behavior is claimed.

## What Paseo does

Read for ADR 0067. Paths are Paseo's packages, not a machine.

### Wire

`packages/protocol/src/messages.ts`

- Client to daemon, 1713–1736: `dictation_stream_start { dictationId,
  format }`, `dictation_stream_chunk { dictationId, seq, audio, format }`
  (`seq` a non-negative integer, `audio` base64), `dictation_stream_finish
  { dictationId, finalSeq }`, `dictation_stream_cancel { dictationId }`.
  `format` is a free string. The comment's example is
  `audio/pcm;rate=16000;bits=16`.
- Daemon to client, 3569–3611: `dictation_stream_ack { ackSeq }` (`ackSeq`
  any integer; the stream starts at -1), `dictation_stream_finish_accepted
  { timeoutMs }`, `dictation_stream_partial { text }`,
  `dictation_stream_final { text, debugRecordingPath? }`,
  `dictation_stream_error { error, retryable, reasonCode?, missingModelIds?,
  debugRecordingPath? }`.

Scopes, `packages/server/src/server/authorization/operation-permissions.ts`:
every dictation message is `workspace.write` (84–87, 319–323). Local model
list is `daemon.read`; download and delete are `daemon.manage` (186–188,
425–427).

Routing, `packages/server/src/server/session/voice/index.ts` 1–218: the
messages belong to the requesting socket. The socket closing cancels the
voice source. A second socket has its own map, so the same `dictationId`
on two sockets does not collide.

Client, `packages/app/src/dictation/dictation-stream-sender.ts`: chunks
are sent without waiting, at most 128 per flush turn (line 5). A
reconnect starts a new id and resends from seq 0 (test at
`dictation-stream-sender.test.ts` 132). Ids are `msg_<epoch>_<random>`
(`packages/app/src/types/stream.ts` 36–38). The web microphone emits
16 kHz PCM16, about one second a chunk, format
`audio/pcm;rate=16000;bits=16`
(`packages/app/src/hooks/use-dictation.shared.ts` 34,
`use-dictation-audio-source.web.ts` 190–194).

### Stream

`packages/server/src/server/dictation/dictation-stream-manager.ts`

- 17–27: mono, 16-bit. Final timeout default 10_000 ms, cap 300_000. Plus
  15_000 per pending segment, 1_500 per pending audio second, 250 per
  missing seq. Auto-commit default 15 seconds. Silence peak default 300,
  overridable by `PASEO_DICTATION_SILENCE_PEAK_THRESHOLD`.
- 179–340 `handleStart`: replaces any stream with the same id. No
  provider: error "Dictation STT not configured", not retryable. Rate
  from the format, default 16_000 (`packages/server/src/server/speech/audio.ts`
  80–91); a non-positive rate fails. Resamples to the provider's rate.
  Prompt default at 188–190, overridable by
  `PASEO_DICTATION_TRANSCRIPTION_PROMPT`. On connect, ack `-1`.
- 343–406 `handleChunk`: unknown id fails retryable. Format must match.
  `seq` below the next to forward is only re-acked. Gaps are buffered
  with no cap. Odd PCM lengths fail later inside the peak helper, not
  at the door. Debug audio is appended when the debug writer exists.
- 408–482 `handleFinish`: no chunk yet fails at once, retryable. Then
  the budgeted timeout.
- 486–488 `handleCancel`: drops the stream, no error message.
- 630–648: a 15 second block quieter than the peak is cleared, not
  committed. The same on the finish tail (664–690).
- 748–770 and 787–800: final text is committed segments joined with a
  space and trimmed. Silence-only accepted audio produces a final empty string; no chunks fail finish.
- Language default `"en"` (155). Tests: unset config is English; blank
  `PASEO_DICTATION_LANGUAGE` is unset; the variable beats the setting
  (`dictation-stream-manager.test.ts` 169–214).

There is no cap on streams, chunk bytes, buffered gaps, or total audio.

### Providers

`packages/server/src/server/speech/speech-provider.ts` 43–70: a session
takes PCM16 at `requiredSampleRate`, then `connect`, `appendPcm16`,
`commit`, `clear`, `close`. Events: `committed`, `transcript`, `error`.

Endpoint, `providers/openai/stt.ts`: required rate 24_000 (72). Audio is
written to a temp file and deleted in `finally` (196–197, 261–268). Model default
`whisper-1` (201). Log-probabilities are requested only for
`gpt-4o-transcribe` and `gpt-4o-mini-transcribe` (202–211). Confidence
default `-3.0` (216); `isLowConfidence` is the average log-probability
below that (225–228). Nothing under `server/dictation/` reads
`isLowConfidence`. Voice mode does (`voice-turn-controller.ts` 69–78
and 180–182).

Config, `providers/openai/config.ts` 34–46: key from the saved config,
else `OPENAI_STT_API_KEY`, else `OPENAI_API_KEY`. No key means no
endpoint config. Base URL from the config, else `OPENAI_STT_BASE_URL`,
else `OPENAI_BASE_URL`. Model: `STT_MODEL` then the saved model.

### Settings

`packages/server/src/server/speech/speech-config-resolver.ts` 48–49:
dictation on by default, voice mode off by default. Provider default
`local` (39–43). Enabled flag and provider can be overridden by
`PASEO_DICTATION_ENABLED` and `PASEO_DICTATION_STT_PROVIDER` (82–90).

`providers/local/config.ts` 39, 47 and 60: language default `en`; local STT
model default is the catalog entry with `defaultFor: "stt"`.

`providers/local/sherpa/model-catalog.ts` 259–267: that entry is
`parakeet-tdt-0.6b-v2-int8`, archive 482_468_385 bytes. The other
speech-to-text archive sizes are at 273, 282, 291, 300, 309, 318, 327,
336, 354. Download and delete are out of this track's first slices.

Read path redacts the key to `hasApiKey`
(`speech-settings.ts` 96–102). A patch stores `apiKey` in the daemon
config (`persisted-config.ts` 99–118).

### Debug files

`packages/server/src/server/agent/recordings-debug.ts` 8–24: debug is
off unless `PASEO_DICTATION_DEBUG` is `1`, `true`, `yes`, or `on`.
`DICTATION_DEBUG_AUDIO_DIR`, if set, is used even when that flag is
off, by `dictation-debug.ts` 20–27. The combined WAV is written only
when the flag is on (manager 503–508).

## What Vessel already has

| Need | Where | Decision |
| --- | --- | --- |
| A method table with scope and `since` | `crates/vessel-proto/src/rpc.rs` | Modify: five rows. Not a new frame. |
| Base64 bytes | `TerminalBytes` | New type `DictationPcm`. Same encoding, so the two cannot be swapped. |
| Settings object, omitted at the default | `settings.rs`, `vessel-node` `settings` | Modify: one optional `dictation`. |
| Secret storage, redacted | `secret.set` since 6 | Reuse. The key is never a settings field. |
| Agent refusal, exhaustive | `vessel-node` `workspace/policy.rs` | Modify: five arms, refused. |
| Forwarding, exhaustive | `Method::forwardable` | Modify: five arms, not forwardable. |
| HTTPS | `reqwest` in the workspace, used by `vessel-llm` | Reuse from `vessel-node`. Not a new crate. `vessel-llm` is the chat proxy and does not gain an audio API. |
| A speech crate | none | Not adopted. No new `vessel-*` edge. |

Checked against `docs/ARCHITECTURE.md`'s crate table. This slice changes
no edge, so the table and ADR 0001 stay as they are. The implementation
slice still adds no `vessel-*` edge.

## Slices

| Slice | What | State |
| --- | --- | --- |
| 1 | ADR 0067 and this plan. No code, no protocol bump, no dependency. | corrected: confidence, chunk length, stated differences, `dictation::SINCE` |
| 2 | The five methods, the stream, a fake provider, the endpoint provider against a loopback HTTP fake, settings, the secret name. Gate on that commit. | next |
| 3 | `vessel dictate <file>` as a client of those methods. Then stop. | after 2 |

Local models and a microphone in the app wait for review. They are
specified in the ADR so a later slice does not invent a second contract.

## Stated differences

Each one is in ADR 0067 with its reason. Short list:

- Acks are the chunk call's reply. Readiness is the start reply. No
  `-1`.
- Partials and the final are replies to `dictation.next`, not a new
  server frame and not a session event.
- Audio is never written. Paseo's debug directory and the endpoint's
  temp WAV are not implemented.
- The key is a node secret. Readers with `read` learn present or
  absent, not the name and not the value.
- No environment overrides. Defaults match Paseo with those variables
  unset.
- Caps Paseo does not have: chunk 256 KiB, `seq` 0..=1_799, reorder
  32 or 1 MiB, 30 minutes of audio, 2 commits in flight (further
  chunks wait), 2 streams per connection, 8 on the node, transcript
  64 KiB. `seq` past 1_799 is `TooLong`. 1_800 one-second chunks are
  30 minutes, so the two caps meet.
- Chunks are one second of PCM16 at the stream's rate, except one
  shorter tail. Paseo's web recorder emits that shape
  (`use-dictation-audio-source.web.ts` 189–198) and does not refuse a
  shorter middle chunk. Vessel does (`BadAudio`), so a client cannot
  fill `seq` before the 30 minutes. `vessel dictate <file>` sends
  one-second chunks and one shorter tail.
- `format` is only on `start`. Paseo sends it on every chunk
  (`messages.ts` 1720–1724). The format is fixed for the stream.
- `DictationId` is 1–64 of `[A-Za-z0-9_-]`. Paseo's `dictationId` is
  any string (`messages.ts` 1715). The id has to be safe in a log
  field.
- `local` is the stored default and `start` fails `LocalNotBuilt`
  until an engine is linked. It does not silently switch to the
  endpoint.
- Agents are refused. Paseo's `workspace.write` would allow an
  agent-scoped client. Not forwardable.
- An id is node-wide among live streams. Paseo isolates ids per
  socket. A second connection using a live id is `conflict`.
- Replacing a stream tells a waiting `next` (`Replaced`). Paseo drops
  it with no message.
- Empty finish fails the finish call itself. Paseo pushes a stream
  error.
- No confidence setting, and the endpoint does not request
  log-probabilities. Paseo stores `confidenceThreshold` (default
  `-3.0`, `stt.ts` 216) and voice mode reads `isLowConfidence`
  (`voice-turn-controller.ts` 180–182). The dictation stream never
  reads it. Voice mode is out of scope, so a field nothing here can
  observe is not in the contract.
- Voice mode, TTS, and model download are not in these slices.

## Implementation slices

The original implementation draft was too large to review as one change.
It is being rebuilt in responsibility-sized parts:

- 2a: audio wire values and validated decoders only; no callable methods,
  provider, settings, or node behaviour yet.
- 2b1: validated provider metadata and the local-model catalogue; no settings
  object or I/O yet.
- 2b2a: unregistered settings values and strict keep/set/reset patch.
- 2b2b: settings persistence and wiring, using the existing settings/secret path.
- 2c onward: bounded audio processing, endpoint, stream actor and the five
  RPC rows, split at those responsibility boundaries with their refusal tests.
- 3: the CLI file client, after the node service passes review.

Decision: modify existing serde/base64 infrastructure. Keep separate PCM
and transcript types because terminal bytes and user speech have different
limits and privacy requirements. The existing base64 decoder has no size
cap; PCM therefore checks its encoded length before allocating decoded
bytes. No dependency is added. Decoder failures are typed and never contain
user audio or transcript text. Debug output is redacted. PCM MIME parameters
are decoded once; duplicates are refused rather than taking the last value.
The API retains any u32 sequence so the stream can return `TooLong`, while
finish-budget decoding refuses values over five minutes.

## Review correction: authoritative versions and bounds

Decision: modify existing types. `SINCE` uses `ProtocolVersion` and aliases
`PROTOCOL_VERSION` until the integrator assigns its permanent version. No
second numeric protocol value can drift from the base. The input bounds
have one definition shared by validation and error formatting; byte-to-KiB
conversion also has one definition. The refusal text and accepted inputs
stay the same. No new dependency or wire method is added.

## TLS configuration extraction

Decision: modify and reuse the existing catalogue TLS construction. The
endpoint needs the same OS roots, explicit ring provider and ALPN, while
reqwest's `rustls-no-provider` feature cannot build without an explicit
provider. Moving only that construction to node `http_tls` avoids copying
policy or installing a process-global provider. No dependency changes.
The catalogue retains its URL validation, redirect/proxy policy, timeouts
and `FetchError::Tls` source. Existing tests and expectations are unchanged.

The existing coverage is the two catalogue reader tests and the loopback
HTTP tests `the_catalogue_is_fetched_when_asked_and_an_entry_is_added_from_the_copy`
and `an_unreachable_registry_is_unavailable_and_the_copy_stays`. They cover
fetch/cache/refusal behaviour, not a real HTTPS handshake or every OS trust
store. The future endpoint must invoke synchronous root loading on a
blocking worker; this extraction preserves catalogue initialization timing.

## Resolved secret lookup extraction

Decision: modify and reuse the existing secret reader/getter. The header
consumer needs the existing non-serializable `ResolvedValue`, whereas
`secret_value` deliberately constructs a serializable wire value for
`secret.copy`. `FleetFiles::resolved_secret` keeps parsing and lookup in the
secret owner; the old wire wrapper delegates and keeps its behaviour.
An inherent method avoids a new module export or wider visibility. No
store format, credential, scope, error or dependency changes.

Existing tests remain unchanged: `set_and_remove_keep_comments_and_other_entries`
and `errors_never_show_a_value` in the secret module, and the actual fleet
copy test `a_secret_is_copied_only_where_the_home_was_granted_admin`.
The endpoint that consumes the new helper is a later slice; this does not
verify sending its header or authenticating to a real provider.

## Boundary test maintenance

Decision: modify existing tests and API comments. Boundary fixtures derive
from each value type's existing constants, including multibyte transcript
lengths, encoded audio size and finish-budget JSON. This keeps the tested
edge aligned when a bound changes, without changing accepted inputs or
refusal messages. Count calculations use explicit integer division, keeping
the rounding policy visible to the lint gate. No dependency, setting or wire
method is added.

## Settings contract values

Decision: extend the existing dictation values, reuse SecretName and its
central validator, and add a change type because Option alone loses the
keep/reset distinction. The strict incoming patch has a separate lenient file
decoding view; unknown nested fields cannot make the whole file fall back to
defaults. Unknown file options may be forgotten on write, but known settings
and the secret reference survive. No dependency is added. Paseo speech-settings.ts
149–175 clears null fields and resets site options on provider changes;
persisted-config.ts 99–115 makes provider fields strict. The contract now
records those behaviours before implementation. Read views expose only key
presence; a safely decoded/redacted reference is only for a write and lookup.

The value-only slice adds no NodeSettings field, persistence or method.
Tests cover defaults, omission versus null for every field, provider switching,
unknown choices, inactive options and secret-reference diagnostics. The later
persistence slice must test failed writes/refusals leave the whole file and
runtime unchanged and newly selected references resolve before saving.

## Audio and connection ownership implementation

Decision: new node helpers, reusing DictationPcm, SampleRate and Bytes. The
node's attachment RIFF signature check detects images, not audio; it cannot
resample, check silence or encode PCM WAV. No existing node converter or
connection identity provides this responsibility. Standard checked integer
arithmetic avoids an audio dependency and keeps the mono PCM contract narrow.
Opaque Arc identity plus a context-drop token reuses existing cancellation;
existing Outbound::closed also covers transport closure before context teardown.

Paseo pcm16-resampler.ts owns floating interpolation with a carry sample;
dictation-stream-manager.ts343–406 and619–645 append a whole chunk before
checking the threshold. The proposed integer phase retains continuity, with
explicit fixture-limited rounding differences in ADR0067. Tests will cover
chunking invariance, source fixture counts/samples, silence, WAV metadata,
pre-mutation segment overflow, opaque owner identity and lifetime cancellation.
The actual transport-close test belongs to wiring, not to the identity helper.

## Endpoint implementation sequence

Decision: extend the node's extracted http_tls and resolved_secret paths,
with new fixed-form and JSON edge codecs where the existing HTTP clients have
other policies. Reuse reqwest, getrandom, rustls and serde_json already declared
by the node; no package, feature or dependency is added. A global HTTP pool,
a second TLS block, a secret loader and a multipart library are unnecessary.

First stage the independent TLS-only cache and fixed multipart encoder. Each
owns its typed error; neither depends on the HTTP client's error or destination.
The cache test cancels its first waiter while one initializer continues. Form
tests check independent framing, UTF-8 fields, one binary WAV, bounded delimiter
collisions and an entropy failure that keeps its cause. Then stage the foreign JSON decoder and HTTP client in separate review slices;
the client wraps those lower causes once.

The client tests use only loopback, covering the fixed header/body, redirect
refusal, a chunked response over its cap, invalid/empty credentials before work,
request timeout through the response body and safe transport diagnostics.
Decoder tests cover missing/wrong shapes, escaped credential echoes, structured
TooSmall classification, empty text, syntax causes and ordinary backslash speech.
The client makes one attempt, explicitly disabling reqwest protocol retries.
Loopback503-then200 and429 with retry hints must fail after one request; a closed
connection and body deadline must not resend. This differs from OpenAI SDK6.44.0
(client.js149/159/360-383/516-566: ten minutes, two retries). A failed segment
becomes Upstream with no Final; the file client never automatically retries.
These tests are owed until those staged implementations run.

## Stream implementation sequence

Decision: new bounded state owners, extending the existing audio, identity,
endpoint, settings and secret paths. The reference manager has no resource-bound
actor to reuse. Tokio bounded channels, watch state, child cancellation tokens
and owned permits already serve the node; no actor framework or dependency is
added. Reuse workspace::trim_project_text for the reference's JavaScript trim
rule, not a new character table. Keep file reads in FleetFiles::resolved_secret;
the fleet mutation permit pattern is useful, its membership OS lock is not.

Paseo manager285–306 emits out-of-order completions immediately with submission-
order placeholders; 408–483/586–617 seal before the finish estimate and count the
whole unacknowledged suffix. Source-only inferences from stt.ts120–168 and
manager271–325/710–783 are recorded in the ADR: mutable overlapping commits and
a finishing TooSmall that can wait for a final id it never records. Detached
segments and empty completed TooSmall results deliberately avoid those paths;
no executed reference reproducer is claimed.

Stage input ordering first, then aggregate text/finish arithmetic, then the
stream actor and service admission in reviewable slices. Each rule's adversarial
tests accompany its code. The ADR names the tests of bounded update queues,
completion-phase capture, pending wait cancellation, replacement/preparation,
quota ownership through blocking work, fresh secrets and two-stream isolation.
Wiring adds the real transport-close, scope/ownership and version-filter cases.
These tests are owed; this documents the choices before the implementation.

## Resume point

Done: wire audio and settings values, provider metadata, TLS and secret lookup
extractions; staged audio/owner, root cache, multipart, JSON response and HTTP
helpers. Their historical whole-gate and review evidence is scoped below. Input
ordering, submission-order text, adaptive budgets, DTOs, update delivery and
provider jobs have narrow checks. They remain staged until product wiring.

Done as staged source: the exclusive actor and fresh secret reader/reservations
passed strict node all-target Clippy and all 76 dictation unit cases, including
twelve actor and four reservation/reader cases. The reader security delta passed
source review. Those results do not prove registry admission or product wiring.
No service registry, callable settings field, RPC registration or CLI command
exists yet. The separate settings/file-view draft and the file preflight decoder
are in progress; their own checks are not yet complete.

Next: merge the announced call-table head at a clean boundary, then bounded
service admission, fresh secret preparation, settings and common file-policy
integration, pending method rows and policy/ownership/close tests. Complete old
settings literals only with the optional-field commit. Keep the one method table
and existing settings/secret owners. Four settings compatibility/deletion/atomic
failure cases and failed-root-cache/recreated-service tests remain owed.

The file CLI follows the service using the new WAV amendment. It drains next
concurrently and prints only a final after finish acceptance. The feature package
gets one whole product gate and one combined review; credential/path changes keep
their security review. Removing staging must compile the actual product and test
real transport close. No current result claims a live service, real HTTPS/ASR,
platform microphone, credential or service-account verification. Local engines,
app capture and TTS remain deferred.

## Input ordering implementation

Decision: a new pure input owner, reusing DictationPcm, SampleRate and ChunkSeq.
The existing resampler and transport frame queue cannot own application-level
gaps, tail placement or distinct accepted/forwarded cursors. No new wire DTO or
dependency is needed: acknowledgement remains Option<ChunkSeq> internally.
One parent chunk-duration constant supplies accepted byte counts and the existing
segment headroom; total duration derives from the sequence cap.

Half-done: staged input source and eleven tests cover held-tail acknowledgement,
matching finish, gap unlocking one chunk at a time, duplicate/overflow priority,
unchanged state on bad shape, count/byte reorder bounds, current-chunk admission
with full holds, duration, finish without audio and filling gaps after finish.
Narrow clippy and all eleven input tests pass; the package's whole gate remains
owed. The pure owner does not test canceled RPC replies or provider backpressure; those remain actor tests.

## Transcript and finish arithmetic implementation

Decision: new pure text and budget owners, reusing Transcript/WithinMs bounds,
Duration, ChunkSeq and workspace::trim_project_text. The audio owner already
returns rounded provider-buffer duration; no second byte-duration codec, trim
policy or dependency is added. Completion storage is separate from joining,
so an ignored TooSmall can settle a slot without constructing a Partial.
Budget computation freezes no timer; first/repeated-finish admission remains
an actor responsibility.

Half-done: staged source, six text and five budget tests. Text covers out-of-order
results, empty/interior spacing, JS edges, pending counts and aggregate UTF-8/
separator limits before join allocation. Budget covers independent terms, the
whole unacknowledged suffix, fractional rounding, zero floor and saturation at
the shared wire cap. Narrow node/proto clippy and all 49 staged dictation tests
pass; the package's whole gate is still owed.

## Unregistered request and update values

Decision: extend the existing dictation value module with request/reply data,
not a new transport or Request implementation. Chunk audio and transcript text
keep the existing redacted, validated types through nesting. Ack absence remains
Option<ChunkSeq>, and finish budgets reuse WithinMs. The tagged reason owns the
local model only in LocalNotBuilt; no optional model sibling can accompany a
different reason. Unknown update/reason kinds remain readable as Unknown.

Half-done: four value tests for ack omission/negative refusal, forward decoding,
redacted parent Debug plus nested PCM validation, and model-bearing refusal.
No method row, client method, ErrorDetail variant, NodeSettings field or protocol
assignment is added. Narrow proto clippy and the four value tests pass; TS
regeneration tests pass with no generated diff. The package gate remains owed.

## Bounded update ownership

Decision: a pure outgoing buffer, reusing Transcript and Tokio oneshot replies.
It owns two partials, one terminal and one waiter. Generic failure payloads keep
typed internal causes out of this buffer's policy, ready for the RPC boundary.
Failed delivery restores ownership synchronously; no update is coalesced, and
a later terminal is returned instead of replacing the first.

Narrow clippy and four buffer tests pass: backpressure/FIFO, canceled response
restoration, extra-wait refusal/replacement and immutable selected terminal.
This proves the buffer only; actor control races, provider job
cancellation and quota/connection integration remain the package's tests.

## Independent provider jobs

Decision: extend the existing endpoint behind a closed endpoint/fake source and
a bounded JoinSet. A completed later job can release a slot before the earlier
one finishes; every job captures Running/Finishing immediately on provider
return, independently of result folding. Typed causes remain available, while
Debug stays fixed. Existing loopback test setup is reused, with no policy or
assertion changes in its original tests. No runtime trait or dependency is added.

Half-done: seven job tests cover out-of-order slot reuse and full ownership return,
phase observation before delayed collection, abort-and-drain, real HTTP/codecs,
closed-provider typed causes and a worker panic with fixed diagnostics. Narrow
checks remain owed. The layer does not
yet prove the actor's finish-only TooSmall mapping or owner/transport cancellation.

## Staged actor implementation

Decision: modify the tested Input/Text/Budget/Updates/Jobs components into one
exclusive stream owner. Existing transport queues do not own gap ordering,
finish metadata or per-segment result order; a separate scheduler is necessary.
No dependency or alternate codec, TLS builder or secret reader is introduced.
The actor advances one accepted chunk at a time, while a separate select observes
next, transport/node/owner close, first-stop selection and the fixed deadline.
Independent provider jobs capture finish phase at return, before result folding.
A deferred success retains backpressure without blocking control. Retirement
preserves a previously selected terminal cause and aborts/joins provider tasks.
Unread terminals retain only output, never the HTTP pool, credential or audio.

Resume: the actor and its tests are staged under the existing outer test gate;
its narrow compile and twelve actor runtime cases passed together with the
other dictation helpers. Registry admission, secret preparation integration,
settings persistence, RPC policy/transport wiring and the file CLI remain
unimplemented. This is source under development, not product
or end-to-end evidence. The feature package gets its whole gate at completion.

Actor narrow compile follow-up: the first check refused a private buffer
constructor and test fixtures calling constructors these types do not offer.
Use the existing validating `SampleRate::new` and `DictationPcm::new`; widen
only the buffer constructor the real actor now calls. First-stop tests also
force a provider failure ready after cancel selection, and replacement/cancel,
closure and an expired deadline after an already selected failure. Repeated
finish is checked against the original actual expiry, not only equal budgets.
These source corrections still await the next narrow check and runtime tests.

## Admission and file-client contract follow-up

The ADR now bounds concurrent replacement metadata with one per-id mailbox and
an explicit Conflict resource floor, before implementing that registry. Its
worker keeps existing stream leases until real teardown or blocked reads finish.
The CLI amendment records the absence of a Paseo equivalent, the existing asset
check, the narrow validated WAV subset, no new dependency, concurrent next drain,
final/finish reply ordering and cancellation/no-resend behavior. Both are contracts,
not implementation or runtime evidence. No new wire number is assigned here.

Actor compile correction: the validating constructors now compile. The next
narrow check found nine documentation/style/checked-test-arithmetic lints; these
are fixed without allowances, weaker assertions or longer deadlines. The
corrected source passed strict node all-target Clippy and its twelve runtime
cases; transport/service/product behavior remains unverified.

## Secret-reader and reservation implementation

Decision: modify/reuse existing semaphore permit ownership and the existing
FleetFiles::resolved_secret lookup; no new file format, storage reader, lock or
dependency. A stream lease owns node and connection permits only. The independent
read gate derives its capacity from the node reservation bound, and presence
uses that gate without taking a stream reservation. Both permits and an optional
stream lease move into the blocking closure, so dropping its async waiter cannot
release capacity while real I/O continues. Values are resolved freshly; fixed
read diagnostics keep typed store/worker causes without formatting their names,
paths or payloads. No connection lifetime guard is retained by a lease.

Four staged cases cover both admission bounds/partial failure, actual store
edits/deletion/empty values, canceled blocking reads retaining the lease, and
corrupt-store fixed diagnostics. Strict Clippy and all four cases passed. This remains test-only
source, not a service admission or RPC/permission proof; the registry and settings
consumers follow. Credential/path code receives its security review separately
from the eventual combined feature-package review.


## File preflight decoder

Decision: new narrow file decoder, reusing the existing asynchronous file APIs,
validated SampleRate/DictationPcm and one-second chunk contract. The attachment
reader has no WAV/container parser and the node's WAV helper encodes rather than
decodes; neither can validate a selected file before connecting. A general audio
framework would add dependencies and conversion policy to this test client.
The standard-library integers/byte conversions and existing Tokio file operations
are sufficient. Pcm16Mono gains a constructor from an already parsed rate so this
edge does not format text and parse it again.

The staged decoder seeks through the whole declared RIFF extent before returning
an open input. Metadata has one named byte cap, including headers, unknown chunks,
format extensions and padding. Format and data chunks are unique; sample width,
channels, byte rate, alignment, rate and duration are checked before any stream
exists. Reading then allocates at most one input second. Later growth is ignored;
later truncation is an I/O failure. No audio is logged or written by the command.
Seven synthetic-file cases cover these boundaries. Narrow checks are pending;
there is no command registration, RPC, cancellation or end-to-end claim yet.

## Consuming shared speech primitives

The staged dictation adapter now calls speech_audio for conversion and WAV
framing; only the feature's validated input adaptation, 15-second commit policy,
silence peak and detached segment bound remain here. The duplicated integer
converter/WAV block is removed. Their three arithmetic fixtures and WAV-field
assertions are covered by the shared codec's already passing eight cases; the
three dictation policy cases remain beside the adapter and preserve their commit,
silence and pre-mutation refusal assertions. Allocation is reserved before a
bounded segment changes. The duplicated private owner module is removed; the
shared connection_life keeps its two passing identity/lifetime cases. Real context
setup and transport observation still await the service caller. This consumer
adaptation has not yet passed its own narrow check or product gate.

The first file-decoder Clippy check compiled the new paths and found only two
test-style lints (get/unwrap and a local import after statements). They are fixed
without changing assertions, format rules or limits. Runtime checks remain owed.

## Inactive settings field and file projection

The first field changes SINCE to an unassigned optional version. A present object,
even empty or containing only null resets, makes the existing settings call
unassigned; a null block is refused. Reply projection composes with the existing
recovery projection and hides dictation on both settings replies and nested fleet
envelopes. No number, new method or service activation is introduced here.

The store's sole settings reader gains a lenient dictation view using the same
validated values; flattening the remaining settings avoids another closed list
of top-level keys. Unknown nested fields are ignored, all known overrides remain,
and the existing later-file copy policy protects the original before a rewrite.
Existing complete patch literals gain only their default completion. Existing
reply fixtures gain an absent dictation field because that reply has no Default;
no previous value or assertion changes. Borrowing saved settings replaces their
old Copy assumption without changing existing parking/driver behavior.

These files are a contract/file-codec draft with narrow checks and generated
TypeScript still pending. Settings application/secret validation, the service
and registered calls remain subsequent work; the real wire rejects this pending
field before dispatch. No live or assigned-feature behavior is claimed.

The first TS generation found a flat-name collision with the existing LSP
Language. Only the dictation TS alias is renamed DictationLanguage; Rust names,
validation and JSON stay unchanged. New metadata aliases are classified Named
in the existing single alias registry, and the outer block is optional but not
nullable in generated input, matching its strict object decoder.

The first local TS check found only the new Language alias collision. The
corrected source's second check was still waiting without compilation when it
was moved to the remote TS artifact workflow. The generated declarations and
remaining narrow checks are owed; this inactive field is not product evidence.
The future-file test compares every known override for both endpoint and local
settings before and after rewriting, and preserves the original backup bytes.

## Current-snapshot provider preparation

The staged preparation factory reuses Reader and the existing endpoint
Specification; no third secret reader or settings cache is introduced. Its
caller must obtain current overrides after retiring the prior generation. The
factory validates the merged selection, refuses disabled/local/unknown choices
before secret I/O, freshly resolves a selected reference and validates the header
before the shared TLS cache/client preparation. A lease is retained through
preparation and cloned into actual blocking reads. Each result owns its client
and key independently. Fixed outer diagnostics retain internal typed causes.
Three new cases exercise actual loopback headers after two secret edits, deletion,
local/disabled/unknown choice against a corrupt store, and absent/blank/invalid
header keys. They are uncompiled staged source, not registry/product evidence.

## Bounded registry implementation

The staged registry has one serial worker per admitted id. Its map lock covers
admission, lookup and identity-checked cleanup only. One pending replacement
mailbox stores metadata; full admission preserves both generations. A notification
wakes the current worker without consuming that mailbox before teardown, avoiding
a second worker-held pending request and a transition race in caller-side stop
selection. The worker joins the old actor before asking the existing settings
owner for a current snapshot or resolving a new key. Pending start cancellation
cannot publish an orphan, and preparation keeps its lease until real work ends.

Both the supervisor and actor own the reservation lease, so a failed supervisor
cannot free capacity while its actor still runs. A supervisor's active-task guard
selects retirement rather than detaching. Service handles own a drop-stop guard;
workers retain only weak registry membership and independent configuration/reader
owners. Shutdown closes admission before joining the tracked workers. Final permit
release, matching-entry removal and completion become visible under the same map
lock. No connection lifetime guard is retained by resources. The actual existing
RPC Outbound close future remains independent of context-drop cancellation.

Eleven new forcing cases cover owner checks on every operation, both reservation
bounds, unread terminal retention, old teardown before blocked current-settings
read, failed replacement, a full replacement mailbox, physical memory-RPC outlet
close with the context guard retained, stale cleanup and fresh secret deletion.
Canceled preparing starts and last-service-handle drop also retain/release their
reservations without canceling a sibling or its connection. An existing-settings
snapshot case uses an empty session runtime with explicit
temporary homes and no agent. These are uncompiled source and do not establish
product behavior. The direct registry tests serve no dictation methods over RPC;
registered scopes, the actual Connection accessor, settings mutation/availability,
failed-root sharing/recreation and full product/CLI flow remain owed.

Paseo dictation-stream-manager.ts486–488 calls cleanup directly for cancel, so
cancel of an already absent id answers Ack here too. A foreign live id still
refuses ownership. This preserves the reference's idempotent cleanup without
granting one connection another connection's recording.


## Root-cache lifetime regression

A test-only loader probe now forces one typed root refusal, then delegates to
the real TLS owner. Two actual registry starts share the same cached refusal
and one loader invocation; recreating the service admits a later start and
performs the second load. No product loader policy changes. This closes the
missing test's source draft, but execution and product-wiring evidence are still
owed. The test does not establish a real TLS handshake or OS trust-store repair.


## Typed call refusal detail

The existing ErrorDetail envelope now has a dictation variant containing the
previously validated DictationReason. A local-engine failure therefore cannot
lose its required model or combine that model with an unrelated reason. The
envelope becomes Clone rather than Copy because the model id is owned. A new
test pins typed roundtrip, old-reader Unknown fallback and future-reason decoding.
No method is registered by this value-only commit; unassigned calls remain
refused, and no new detail can reach a peer through this inactive service.
Regeneration and narrow compile/tests remain owed; the queued earlier TS snapshot
does not include this addition.

Root initialization now lives in its own private function, separately from the
wait loop. OnceLock still installs exactly one receiver synchronously and the
same independent producer owns the result. This keeps the cache operation and
loader below the function budget without adding another state or retry path.

### Historical prerequisite-base checkpoint (superseded)

Next: stage real audio/owner helpers under the test-only dictation module, with
tests beside each rule, followed by endpoint and actor slices. These establish
staged source behaviour, not a product path. Wiring waits for the shared protocol
accessor and call-table refactor, removes staging, and gets a whole product gate.
Existing settings literals gain default completion only in the optional-field
commit, without changing any old value or assertion. Do not split the shared
guard; the common plumbing refactor owns its structure. Local engines and
microphone capture remain deferred. The CLI file client follows the node service
and still needs its file-format contract amendment.

## Settings-file diagnostic prerequisite

The central file reader is being hardened separately from the pending dictation
field. Invalid existing blocks can quote a mistaken credential in serde errors;
the store must keep that cause private and publish category/path metadata only.
ADR 0038 records the reuse decision and cost. No Paseo behavior is claimed for
this security floor. Narrow store/node tests and strict Clippy remain owed; no
product dictation wiring or whole-package evidence is established by this fix.

Resume: the safe parse diagnostic and four store tests plus one startup-log
test are implemented on the designated node head. Local rustfmt, size and diff
checks pass; compilation and test execution remain owed on the build machine.
The lock change only adds the already locked path-tracker to the store dependency
list. Submit the exact commit and narrow evidence for security review before
merging it into the feature package.

The first executed narrow Clippy stopped on unwrap_in_result in the test log
writer; no product code failed. The writer now recovers its own capture mutex
through PoisonError::into_inner, as existing internal tables do. Test inputs,
marker assertions and recovery branches are unchanged. The first failed log is
retained; strict and runtime checks on this correction remain owed.

## Product wiring decisions

Decision: extend the existing settings writer, generated method table and actual
connection constructor. No second settings/secret parser or presence identity is
introduced. Context construction enforces the person-only rule for every handler,
as the side-question domain enforces its own rule. Canonical identity and quota
are separate fields, both cloned for acting-as contexts; only observers reach
stream workers. Test-only control methods stay test-only; product teardown runs
through the serial registry and actual joins.

The node already distinguishes work tasks from subscription forwarders. Group
those existing trackers in ServiceTasks rather than adding an independent
shutdown path, letting registry retirement use work completion rather than the
forwarder grace. The single Parts construction line changes; node.rs stays
within its recorded size and no size exception grows.

The typed historical downgrade test exercises the pre-dictation decode/write
shape and keeps the common later-copy backup plus named secret. It does not run
an older binary. The real failed-write case replaces only its own temporary data
directory with a non-directory after validation, then restores it before checking
the old bytes and settings; it makes no OS-account or permission assumption.
The test-only write gate coordinates an actual writer, not a substituted result.

All new tests and product configuration remain unexecuted at this checkpoint.
The prior TS artifact has ts-rs trailing spaces and is kept byte-for-byte; owned
Rust/document whitespace checks pass separately. It needs final regeneration.

The common settings writer's caller-cancellation race is a separate prerequisite:
a detached blocking write can outlive its caller's borrowed lock. The current
implementation retains the existing order; it does not claim to repair that
race. Product handover must consume the shared tracked OwnedMutexGuard writer
and its deterministic real-I/O regression, rather than introducing a second
writer here. Our write gate is before spawn and proves deletion/write-refusal
ordering, not cancellation safety inside a blocking write.

## File-command implementation decision

Reuse the existing async FileInput preflight and NodeClient/Unary APIs. A closed
client adapter has an actual node plus a cfg(test)-only bounded scripted edge
using the existing Call/Reply table, so tests introduce no duplicate method
protocol. Sender and next reader run concurrently after readiness; only the
accepted Final reaches stdout. Existing UUID generation mints an independent
stream handle. ADR0067 records wait/cleanup budgets, no resend and the private
command-error boundary required by main's source-chain renderer. No dependency
is introduced. This contract/code checkpoint has no CLI execution evidence yet.


### File command implementation checkpoint

Existing assets reused: NodeClient's typed unary calls, the asynchronous file
preflight/reader and the CLI output helper. A test-only closed client variant
uses the same Call/Reply types; there is no new transport or protocol table.
Tokio's existing dependency enables test-util only for deterministic wait-clock
regressions. No new dependency, recognizer, decoder or product timer is added.

The command validates before node resolution, serially verifies contiguous acks
while draining next, keeps an early Final private until finish acceptance, and
never reconnects or resends. A retained primary failure survives failed or timed
out cancellation. Its opaque command error prevents main's recursive source
renderer from quoting a remote speech message; concrete causes remain private.

New tests cover real file chunks/tail, partial drain, early/future updates,
missing ack, active cancellation, truncation after an acknowledged chunk,
finish-acceptance and returned-final budgets, cleanup timeout and marker privacy.
The file command's seven earlier parser cases remain unchanged except exposing
three fixture builders within this module. Execution is still owed; direct
rustfmt and the size check are source checks only. Final package validation also
waits for the common settings writer's cancellation-safe tracked persist owner.


### Canonical close accessor at the product boundary

Connection setup creates exactly one ConnectionLife and starts one guard-free
observer of the existing Outbound::closed signal. It exits on transport close or
last-context drop and owns no I/O, quota, service or context. Thus consumers that
retain only life().closed() also observe physical transport close while an RPC
context remains alive during grace; acting-as contexts share the same signal.
The pure identity module remains transport-independent. A real memory-RPC test
retains both contexts, closes one transport, awaits the canonical child observer
and proves another connection remains open. This new source still owes execution.
