# ADR 0080: spoken conversation — shared RPC cancellation prerequisite

This isolated node leaf records only the shared client-writer prerequisite.
The remaining voice architecture, devices/providers/model qualification and
activation are separate work; this leaf grants no microphone/provider/model
or runtime availability and assigns no protocol/store number.

## Caller-drop fence before RPC transport admission

[Decision Log]
- Purpose: a dropped speech upload must not remain in the client's bounded
  queue and later become a new transport request merely because Cancel could
  not enter that same full queue.
- Existing assets: each Call owns a typed oneshot waiter whose closed state
  already tracks its caller. The driver owns the sink and a bounded channel
  of reply registrations. Public call/subscribe APIs need no new policy flag.
- Alternatives: adding another cancellation queue/token registry duplicates
  the existing per-call state; draining the whole connection discards unrelated
  calls; relying on best-effort Cancel cannot retract an unsent queued request.
- Choice: reuse waiter closure at dequeue and while waiting for registration
  capacity/sink readiness. Register the waiter immediately before synchronous
  transport admission, not while a cancellable call is held behind the sink.
- Why: closure is independent of command-queue capacity, canceled pre-admission
  calls leave no reply registration, and the live waiter is still registered
  before the transport can produce an answer. No actor map or public API grows.
- Impact: ordinary live-call framing, reply routing and FIFO order remain.
  A canceled call can disappear before transmission; once admitted, remote
  cancellation remains best-effort and cannot reverse an agent/side effect.

The transport commitment point is the final synchronous admission step after
sink readiness: recheck closure, install the reply registration and call the
sink's start_send without an intervening await. Requests canceled before that
step are discarded; payloads already committed to the sink/transport cannot be
retracted or proven absent from a kernel/peer buffer. Cancellation concurrent
with that admission is not a promise to undo committed bytes. After transport
failure the existing connection shutdown applies; no partial frame is replayed
onto another generation and no microphone is silently restarted.

Regression obligations: hold a sink, saturate the command queue, drop an actual
PendingCall future so its Cancel offer fails, release the sink and observe no
canceled Request frame; preserve live requests. Also drop a call already
waiting for sink readiness and show no phantom registration or later Request.
A committed request remains distinguishable from an uncommitted cancellation.
Tests use memory transports/typed calls, never microphone/provider/harness.

This is an ADR0080 transport section on the node line, before the full voice
package lands. Retain it when merging the package's broader ADR0080 record.
Voice upload remains unwired until V3, the VM RPC suite and node integration
adopt this shared edge; this leaf does not establish end-to-end voice parity.


#### Exact inactive ndarray metadata candidate

[Decision Log]
- Purpose: let the build-tool owner reproduce offline resolution without enabling an unused array adapter.
- Existing constraints: ort is already defaults-OFF; its optional ndarray requirement is `0.17` with defaults off. `std` contains `ndarray?/std`, which does not enable that optional dependency. The previous offline resolver stopped before producing a lock delta or feature tree.
- Alternatives: enabling ort defaults or adding ndarray as an active dependency would expand the reviewed compiled closure; neither is selected. Disabling the same defaults again cannot supply missing registry metadata.
- Selected choice: pin only ndarray0.17.2 as the optional-package metadata/source candidate through the existing build-tool owner's cache path. No new manifest feature, direct dependency, lock entry or crate acquisition is made by this document.
- Why: the official sparse record and registry version metadata agree on one non-yanked version and archive checksum. Their metadata is independently retained; actual archive verification and offline resolution belong to the cache/resolve receipt.
- Impact: this may satisfy only the next resolver lookup. The record declares further dependencies; if any additional package/version is absent, stop and request a separate scoped ruling rather than provisioning a closure implicitly. Even a successful resolve does not prove ndarray is compiled; require the actual selected feature tree.

| Metadata candidate | Exact pin |
| --- | --- |
| Registry/version | crates.io ndarray0.17.2; non-yanked at metadata inspection |
| Expected crate archive bytes | 243631 |
| Expected archive SHA256 and sparse `cksum` | `520080814a7a6b4a6e9070823bb24b4531daac8c4627e08ba5de8c5ef2f2752d` |
| Publisher licence/MSRV metadata | MIT OR Apache-2.0; Rust1.64 |
| Sparse metadata | https://index.crates.io/nd/ar/ndarray |
| Version metadata | https://crates.io/api/v1/crates/ndarray/0.17.2 |

The retained `docs/evidence/voice-vad-ndarray-metadata.json` contains the exact
selected registry record, metadata response hashes and explicit unverified
archive status. The archive has not been downloaded or inspected by this
metadata task. Licence metadata is not a notice/archive correspondence audit.
The cache owner must bind actual archive bytes to the expected checksum before
any use. Keep the original failed resolve and misleading audit, with their
append-only correction: no lock drift occurred; resolution failed and feature
closure was unevaluated. The accepted future lock delta must list every added
name/version/checksum, and the subsequent offline build must leave that input
lock byte-identical. Linux/Windows compile approval, fixture transfer, native
loading, telemetry and model API gates remain separate and unfulfilled.
