# W17 authoritative terminal frame contract proposal

Status: design/staging only. No DTO, method registration, client API or live frame
consumer is implemented by this document. C2 row246 source2807ad37 was reviewed;
its full whole and row217 consumer qualification remain open. Only frozen Silent
capture is currently integrated. NodeAuthority is OFF. This proposal uses the
reviewed producer shape; it does not consume C2's separate renderer/cell-constructor
branch or claim decoder restoration.

[Decision Log]
- Purpose: recover a current terminal view after raw history pruning, including
  session TUI and standalone terminals, using one authoritative interpreter.
- Existing assets: the shared actor helper owns Screen for typed Session(SessionId)
  or Standalone(TerminalId); C2 owns borrowed FrameView, modes, bounded physical-row
  markers and Snapshot. ConnectionLife supplies actual context identity/closure;
  terminal_for and ordinary session admission retain their ownership policies.
- Alternatives: decoding a pruned suffix, synthetic ANSI initialization or a copied
  parser loses decoder carry and modes. Tail-distance scrolling drifts on growth.
- Choice: connection-owned bounded viewport slots and demand-driven full typed
  replacement frames, admitted at the output actor. Consumers render cells/modes;
  they never parse a raw suffix after receiving a frame.
- Impact: view and actor epochs are separate from authenticated route identity,
  raw EventSeq, pure buffer epoch and screen revision. No new pairing/book authority.

## Proposed view lifetime and admission

Open/read/close operations name the authenticated typed target and opaque view id.
A view belongs to the canonical ConnectionLife owner plus its effective principal,
that exact target, and a fresh actor epoch. Acting-as on the same connection cannot
reuse another principal's view. Each operation first repeats the ordinary target
Read admission; a view id or pure marker never substitutes for that check. Calls
are nonforwardable because view ownership is local to a real RPC connection.

Proposed limits:16 views per connection,64 per actor across both buffers,256 per
node,30 seconds maximum read wait and5 minutes idle lifetime. Engine marker capacity
is also enforced; no temporary extra slot is needed for atomic replace. Open reply
loss cannot leak indefinitely: disconnect/expiry closes the slot and releases its
marker. Quota acquisition and rollback must not create an unbounded waiter/task map.
These limits are proposed named constants, not existing product guarantees.

A viewport request is Keep, FollowLive or Pin(observed actor epoch, observed screen
revision, physical row). Pin checks the observed cut before replacing any marker;
stale input refuses without retargeting the old view. Keep borrows the engine's
retained anchor. FollowLive releases it. Pure Retained/Evicted/Reset/Released and
Active/Inactive remain distinct, as do frame adjustment reasons. No marker id can
be reconstructed from wire data or EventSeq. Buffer clear/RIS invalidation and
reflow movement belong only to the engine. Eviction may project the oldest row
with an explicit reason; it does not silently rewrite the retained anchor.


Pending reads use the existing canonical node Waits owner, not a frame-specific
permit map. Before a read may wait, acquire the same connection share and
Waiter::of(effective principal, actual local NodeId) origin shares through the
existing terminal_wait/Waits.take mechanism (connection32, origin8 and existing
node/relay shares). A forwarded frame call is refused, never assigned a synthetic
local owner. In addition, each view admits at most one pending read by a checked
slot held through reply/cancellation; all views share a proposed node-wide128
pending-read ceiling. No acquisition waits or spawns a waiter: if any share/slot
is unavailable, answer the current full frame/Unchanged immediately with explicit
WaitNotAdmitted, or the applicable typed projection failure. Release every share
on deadline, disconnect, view close, cancellation or reply completion. The30s
wait timer starts only after successful admission. One view cannot bypass the
origin/connection shares by replacing itself or changing viewport; view mutation
answers immediately and cannot take an additional pending read.

ScreenRevision changes on admitted screen output, resize, modes and finalization;
ViewRevision changes on successful viewport/row selection. A read compares both:
a viewport mutation must not incorrectly return Unchanged just because screen
bytes did not change. Checked counters refuse exhaustion instead of wrapping.

## Full frame and ordered cut

The full reply binds verified NodeId, typed target, actor epoch, screen/view
revisions, selected buffer/epoch, physical first row, dimensions, row-major cells,
explicit origin/projection, cursor position/visibility, input modes and final state.
Cursor shape/blink/renderer appearance is outside C2's reviewed source contract.
Colors, width/continuation and independent styles map explicitly from pure types;
there is no hand decoding, normalization or generated SGR state replay.

The raw next EventSeq is observed in the same actor turn after all output bytes
below it were interpreted and their records published. The frame cannot cross an
await that publishes half a chunk. Notices may advance EventSeq without changing
cells; resize may change the screen revision without a raw record. Return a fresh
cut even for Unchanged. Rendering follows screen/view revisions, not raw replay.
A failed resize/PTY admission cannot leave a successful frame claiming unapplied
geometry; the final implementation must prove this ordering for both actor kinds.
Existing text-capture resize admission alone is not that proof.

Proposed allocation caps:65536 selected cells and2MiB canonical JSON frame payload,
well below the16MiB transport cap. First reject cell cardinality before walking or
allocating DTO cells. Then serialize a borrowed typed projection into a bounded
counting writer before constructing the owned full reply; JSON escaping and all
metadata count exactly. Recheck the completed immutable typed root through the same
serializer. Reuse the validated-root/byte-budget pattern of diagnostics/report.rs,
without sharing diagnostic policy or exposing an untyped Value. Client decoding
must bound incremental cell/text input too; transport length alone is insufficient.
A refusal is explicit FrameTooLarge, never a truncated complete frame.

## EOF sealed-owner agreement

C2 confirmed the original Screen may move into a node sealed FinalOwner at EOF:
no feed, resize or set_scrollback is exposed there; only bounded marker metadata
operations and readonly frame projection may be added after producer qualification.
A new EOF marker must be resolved against that retained final Screen, never an
older Snapshot which has no record of the new marker. Snapshot remains readonly
capture/final-data material, not a decoder seed. No pure-core change is required.

The current node foundation transfers the original frozen Silent Screen into a
sealed final owner, without cloning the full grid at EOF. Existing CLI capture
projects it readonly; no marker or frame API is implemented yet. Repeated finish
cannot replace the retained cut, and the old actor no longer owns a mutable engine.
Pin/FollowLive always require a full frame even if ScreenRevision did not change;
Unchanged applies only to Keep with identical view state. Final event cursor must
include final status publication. Latest actual execution is still owed.

## Required evidence before registration/readiness

- Fake PTY tests for standalone and session targets, partial UTF8/CSI, pruned ring,
  output/resize/frame ordering, final status and stable same-cut modes/cursor.
- Independent connections/principals/targets cannot reuse view handles; disconnect,
  cancellation, reply loss, expiry and capacity release resources without stale reuse.
- Scroll growth/front trim/reflow, alternate transitions and reset preserve explicit
  anchor outcomes; stale Pin and counter overflow refuse before mutation.
- Escaped/control/wide cell text and exact byte/cell boundary tests exercise both
  producer serialization and bounded client decode; no eager per-output grid clone.
- Named generation, strict changed-source execution and the one enclosing full whole
  on the gate49-fixed base, reported to V3 for row217 as well as the W17 package.

Normal-query responder admission/failure is specified separately in ADR0095.
Frame-only clients never answer UnansweredReplies overlap candidates. No source
review or scoped test is treated as live frame, quiescence or delivery proof.
