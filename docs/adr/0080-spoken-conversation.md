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
