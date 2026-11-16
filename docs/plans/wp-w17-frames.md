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

Proposed limits:16 views per connection,32 per actor across both buffers,256 per
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

## Subsequent pure-producer finding

C2 reports an actual full-grid regression at d7ddbb0a: history rows trimmed of
trailing default cells make FrameView::cell return None inside advertised frame
geometry (41 passed/1 failed). Frozen2807 is unchanged and its whole continues;
Silent text capture is unaffected. Even a green frozen whole does not qualify a
frame consumer until the pure owner's minimal projection correction and full-grid
regressions are reviewed/executed. The node will not synthesize default cells to
hide this gap or copy physical-row interpretation. No frame API is adopted here.

## Reserved appearance and row-break properties (not adopted)

C2's separate display/CLI branch proposes cursor_appearance with shape optionally
Block/Underline/Bar and blink independently optionally On/Off. An unset program
value means the renderer's default; it does not authorize inventing a block shape
or blink choice. Actual pinned xterm can retain shape while invalid DECSCUSR changes
blink. RowBreak::Soft means this physical row continues into the NEXT row, matching
the server's getLine(i+1).isWrapped; Hard is distinct. All metadata must come from
the same authoritative frame cut and engine, never a second tracker or text guess.

These properties are reserved only in this design. The current row246 contract
has position/visibility, not appearance/row breaks; C2's additional26 golden
checkpoints and semantic tests/vendor corrections remain pending. An unimplemented
property must be explicit unsupported coverage, not conflated with a supported
program-unset default or Hard row. No DTO/serializer/API is frozen by this note,
no separate renderer branch is consumed, and complete D1 copy/cursor appearance
parity is not claimed. The padding correction merged onto announced1c is reported
source-satisfactory but its ONE corrected whole still gates producer qualification.

## Additional measured engine boundary before consumer qualification

C2 reports baseline3666 strict0 with44 tests passed/2 failed: pinned xterm1049
copies the primary cursor into the alternate buffer, while frozen2d5/vt100 starts
it at0,0. This changes wrap/cell placement when entry begins at a nonzero cursor;
it is not merely a renderer-appearance issue and can affect interpreted capture
and future CPR/frame results. The independent blink-only DECSCUSR case also fails.
The preserved oracle expanded26→34 checkpoints; same-engine repair d105d3f46 is
staged with narrow proof pending, not an adoption request. Frozen2d5 whole is
unchanged. W17 will not infer general capture/frame/query parity from its earlier
bounded corpus or an eventual green2d5 whole while this known divergence remains.
Require the pure owner's measured/reviewed correction before W17 final consumer
qualification. Current source uses frozen Silent observation with capture feature
None and NodeAuthority OFF. No node-side coordinate compensation is permitted.

C2 subsequently retained an actual43/2 display regression at202764a2: repeated1049
entry must clear the existing alternate cells, and DECSTR must reset actual VT
application-cursor/bracketed-paste state. Candidate346653a9 is submitted, not
checked/reviewed/adoptable. The coordinator's policy preserves the pinned server
tracker's separate Kitty/Win32 stack behavior through the same decoded owner;
primary/alternate saved attributes remain buffer-local. W17 makes no compensating
tracker or coordinate repair and keeps the source dependency hold. A successful
frozen2d5 whole alone cannot settle these additional measured semantic failures.

## Continuous-output gesture progress (design correction, no API frozen)

C2 identified that requiring every ordinary scroll Pin to match ScreenRevision
can starve scrolling forever while output grows. The strict absolute-Pin proposal
is therefore limited to explicit absolute row selection. It is not the normal
wheel/page navigation path, and no DTO or call is registered by this correction.

The proposed relative path is owned by the same authenticated connection,
effective principal, typed target, actor incarnation and viewport slot. It checks
the expected ViewRevision and buffer/epoch, not an unchanged ScreenRevision.
The actor derives the relative selection from the CURRENT pure marker/frame
projection in its ordered turn, then performs the bounded marker replacement.
Live output/reflow never discards a queued gesture solely because screen bytes
advanced. Existing pinned text stays anchored; actual eviction clamps with the
engine's explicit reason. Reset or inactive-buffer intent has an explicit no-apply
refusal or declared clamp; no implicit retargeting or coordinate reconstruction.

C2's opening controller must retain a bounded wheel/page intent while a read is
outstanding, coalescing only unsent compatible intent. Sent intent is immutable.
A view admits one mutation at a time through the existing bounded actor control
path, without an unbounded enqueue waiter; a full path refuses before mutation.
A mutation is immediate, takes no second pending-read share, and wakes the existing
read through ViewRevision. All original identity, view-slot and Waits limits remain.

No mutating transport loss authorizes automatic retry. A bounded per-view last
operation identity, payload fingerprint and acknowledged outcome prevents duplicate
application. A repeated known identity reports its recorded outcome; a mismatched
payload or identity older than retained history refuses/returns OutcomeUnknown,
never applies again. A later full frame can have a newer screen cut while its
operation receipt still identifies the original applied view revision. Closed or
reopened views do not replay an old opening's unknown intent into a new actor.
These are contract requirements, not an implemented receipt service.

Frame serialization/allocation refusal must not disguise a committed view change
as no mutation: distinguish refused-before-apply from applied-without-frame, and
retain its operation outcome before returning any full-frame budget failure.
No clipped full frame, rollback by guessed coordinates or replayed delta is allowed.

Required actual tests add continuous output between gesture and admission, a read
outstanding during scroll, trimming/reflow on the retained marker, epoch/buffer
transitions, lost reply and duplicate operation identity, and applied view mutation
whose frame exceeds byte/cell admission. Ordinary output alone must not prevent
one bounded owned gesture from making progress. Producer qualification and V3
interface review remain required before any wire/API implementation.

## Per-view history eligibility and preference-owner handoff

A local/native view preference never calls the sole actor Screen::set_scrollback:
that would destroy other views' and CLI capture's shared retained history. Keep
the actor's authoritative retention policy separate from each view's eligible
history floor and requested/effective limit. The per-view floor is engine-marker
metadata, not a copied buffer or another terminal interpreter.

For one opening, apply the coordinator's minSaved trim before its finalSaved
limit at one ordered owner cut. Reducing0 then increasing8 cannot reveal text
that this same view already excluded, even if the actor still retains it for
other readers. Increasing the limit permits future history without moving that
floor backwards. Preserve the eligible floor and view identity across preference
owner replacement: old-owner drainedB is the successor's initial value; laterC
is normal new-owner delivery. Fence stale owner generations/effects. Reopening a
distinct view is not a claim that this preference deleted the actor's global
history; that lifetime policy must stay explicit.

A floor plus pin can require two markers per buffer per view. The proposal lowers
actor view admission from64 to32 and accounts/reserves this worst-case capacity
against the pure64 slots per buffer. A first scroll must not unexpectedly fail
because all floor-only views exhausted its future pin capacity. Marker replacement
uses the pure atomic operation; no temporary extra slot or unbounded remap journal.
Reset/inactive-buffer floor handling remains explicit and uses engine epochs.

Requested, effective-cap and actually available history are distinct. With an
authoritative actor retaining at most10k lines, a UI request above10k cannot be
acknowledged as completed retention. Proposed negotiation clamps to the actual
actor cap with explicit requested/effective/available metadata, or refuses when
that capability is unavailable; it never infers a larger cap from a saved setting.
The final typed cap-response policy requires C2/V3 interface agreement before
implementation. CLI capture and other view floors remain unaffected.

Additional evidence must cover0-to8 monotonic trim, B/C owner handoff ordering,
stale-owner delivery, simultaneous views with different limits, primary/alternate
floors, both marker-slot bounds, and above-node-cap requests without false retained
history claims. No retention wire type, node call or pure engine mutation is added
by this contract note. Current capture remains Silent/NodeAuthority OFF.
