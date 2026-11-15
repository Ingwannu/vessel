# Dictation

Node-side speech to text, following Paseo's composer dictation. ADR 0067
is the contract. The protocol number is `dictation::SINCE`, given at
merge. This plan does not change `PROTOCOL_VERSION`.

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
  space and trimmed. Empty audio still produces a final empty string.
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

## Resume point

Done: audio values, provider metadata, TLS and resolved-secret extraction, and
unregistered settings values. The named node head was merged; keep/set/reset,
provider switching, inactive-field refusal and reference privacy are implemented.
No call, NodeSettings field, file codec, stream or HTTP provider is registered.

Settings wiring still owes the four file/race/atomicity tests in ADR0067. Use the
integrator's common file policy when that head is merged; do not add a parallel
loader. Preserve unknown fields if the common policy preserves them. The
selection check is not a secret-store transaction and caches no credential.

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
