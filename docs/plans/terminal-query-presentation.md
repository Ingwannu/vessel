# Terminal query presentation and attach follow-up

The prerequisite engine remains a separate frozen package. This branch owns
the following D1 work; none is consumer-ready or activated yet.

[Decision Log]
- Purpose: one node answer for normal suppressed queries across views and
  replay, with explicit best-effort handling of rare ambiguous legacy spans.
- Existing assets: one vt100/VTE decoder, typed shared cells and immutable
  snapshots. Legacy CLI attach forwards terminal output and raw stdin.
- Alternatives: guessed Kitty defaults, a separate query scanner, suppressing
  stdin responses, or refusing raw attach. Defaults break mode negotiation;
  scanning duplicates interpretation; response shapes collide with real keys;
  refusal removes a working feature.
- Choice: retain parameter presence and token boundaries in the same decoder.
  Publish presentation bytes with node-answered non-rendering queries omitted.
  Preserve other complete bytes and sequence progress, including empty output.
- Why: the node acts as a multiplexer; old emulators never see a query that the
  node already answered, including replayed output. Real stdin is untouched.
- Impact: terminal presentation deliberately omits answered query requests.
  Incomplete tokens are withheld with one named bound and explicit refusal.
  No synthetic ANSI is used to reconstitute or reflow engine state.

VTE 0.15.0 is the existing transitive dependency, MIT OR Apache-2.0, published
archive 57,487 bytes. Exact archive/member hashes and both licenses are kept.
One root patch points only at this copy. T must re-audit pure classification.
Two numbered minimal metadata changes are approved; no upstream submission.

Node glue belongs to the CLI/node owner: one actor orders output, replies and
presentation publication. NodeAuthority remains off until this path lands and
passes. Tests must cover chunk-split queries, history replay, old raw clients,
empty sequence advancement and unchanged real key forwarding.

Current clients later use authoritative typed frames. Full frame replacement
has its own revision; raw EventSeq is not a screen revision. Stable viewport
anchors must hold inspected rows across growth/eviction and be invalidated on
buffer reset. Modes/anchors/projection come from this same engine. No DTO/call
or mutable decoder restoration is invented by the consumer.

The CLI attach renderer remains D1 work: shared cells/modes render to terminal
ANSI, with raw replay interpreted silently. ANSI is output to the real terminal,
never fed into an engine. The native renderer remains the sole kit control.

## Resume point

VTE metadata and the vt100 bridge are staged with pure presentation regressions.
Normal replies are separated from complete presentation; canceled-prefix/query
sharing preserves the span unchanged, omits the node reply and emits typed
AmbiguousOverlap offsets. The tested title-overlap exception follows the
coordinator ruling. Partial tokens and UTF-8 are bounded and retained, executed
C0 callbacks survive query omission. Parameter-presence callback regression is
included; negotiated modes/frame/anchors are not implemented yet.

Next: T audit, exact changed-package tests/strict checks, then modes and frames. No node activation,
transport or renderer acceptance has occurred.

## Negotiated mode contract

Reference read: protocol terminal-input-mode.ts and its tests, server
terminal/terminal.ts writeOutputToHeadless (modes commit in parsed-output
callback) and disposeResources (tracker reset at disposal). The reference
tracker itself was invoked for16 scenarios/239 whole-or-byte steps. Its source
SHA and exact outputs are retained in the fixture; no reference code is copied
into product source. This checks state, replies and modified-Enter admission;
the fixture preamble is not a claim of a shipped CLI renderer.

InputModes keeps independent Kitty/Win32/application-cursor/bracketed-paste
state. Bare Kitty push defaults1, explicit0 stays0; setter second0 clears and
other second values replace as the reference does. Pops restore saved flags
with count at least1; over-pop reaches0. Mode stack is bounded at1024 and raw
flags at u32; overflows refuse explicitly instead of silently clamping VTE's
u16 normalized values. These are resource/type floors versus the unbounded
reference tracker. Input negotiation is decoded from the same parser; CSI
subparameter forms are not regex-matched negotiation. In-band RIS leaves the
negotiation tracker intact, matching reference disposal-only tracker reset.
Read-only snapshots carry modes from the same cut as cells/cursor.

Verification: exact791ec973 presentation stage strict0 and18/0/0; its earlier
metadata had presence-only scalars. New u32 metadata/mode changes and239-step
fixture still owe exact checks. Node actor/output-ordering/input integration,
frames/anchors, CLI/native consumers and activation remain unverified.

### Canonical oracle correction after actual red

585d4ad8 executed23/1/0: the actual per-byte reference trace loses the standalone
ESC before Kitty push. The existing raw fixture is preserved byte-for-byte in
paseo-chunk-loss-behaviour.json and scratch evidence; the original failed log
remains. Approved stated difference: VTE retains chunk-invariant mode state.
A fresh actual reference tracker receives the accumulated original prefix once
per checkpoint for the canonical239-step comparison. All its raw output fields
remain unmodified; the historical raw trace is a separate named regression,
not a claim that Vessel reproduces the bug. Strict585 stopped at ConPTY doc
backticks; correction also retains both previously-unused test reply results.

The31 existing cell checkpoints now also compare the byte-wise multiplexer
and its query-stripped old raw client against the unchanged golden authority,
including resize. New exact test/strict checks are owed.

## Physical viewport markers

[Decision Log]
- Purpose: retain inspected rows across output, trim and width reflow without
  inventing tail offsets or replaying old ANSI.
- Existing assets: the sole vendored grid owns row mutations. The exact server
  headless334 Buffer::addMarker642–677 adjusts physical rows on trim/insert/
  delete; reflow emits edits at530–539. It does not track a logical cell offset.
- Alternatives: per-row Arc identities, an unbounded row-remap log, or a second
  app/node viewport registry. They add allocation/history or duplicate ownership.
- Choice: a bounded marker table inside each engine grid, with typed facade
  handles bound to one Screen allocation, buffer and reset epoch. Caller-owned
  markers have explicit create/resolve/release/replace lifetime. Node view slots
  bind those handles to authenticated connection/terminal/actor authority.
- Why: marker adjustment belongs to the same row surgery as reflow/retention.
  It cannot drift from parsed grid geometry, and nodes never infer it from logs.
- Cost: finite marker slots refuse explicitly; epochs distinguish Reset from
  actual eviction and release. Anchor identity never grants transport access.

The actual pinned headless public registerMarker oracle has5 programs and15
checkpoints: output/trim, capacity shrink, wider/narrower wrapped rows, explicit
IL/DL and scroll region, primary/alternate/history clear/RIS. Its observed
physical semantics are deliberate: wider resize disposes a removed continuation
row's marker; region scrolling can replace the contents at a retained physical
position. A marker is not a frozen text snapshot or logical-grapheme bookmark.
The original raw outputs are retained in scratch marker-oracle.json.
Implementation/tests remain owed. Node agreed this lifetime has no glue conflict;
wire IDs alone never authorize a read/release or select another route.

### Marker implementation resume

The grid now stages64 marker slots per buffer, non-reused local identities,
explicit release/replacement and physical row edits. The facade binds each
handle to a Screen allocation/buffer/epoch; actual grid-reset callbacks advance
epochs before reset. Readonly snapshots keep the same marker/cell/mode cut.
New pure tests cover full-cap atomic replacement, stale/foreign/reset handles,
alternate preservation, capacity/growth/eviction and11 unchanged actual xterm
physical marker checkpoints. All15 oracle observations remain in the fixture;
4 reset observations are separate from those11 position comparisons. Typed
history-clear epochs intentionally reset all old handles, including visible
markers; the reference disposes only removed-history markers at that operation.
This stronger reset is the explicit frame/anchor invalidation contract.
Exact marker tests/strict checks remain owed; modes6e remains25/0 plus strict0.
No transport/view-slot ownership or renderer is implemented by this pure API.

Marker headd47 executes30/0/0, including11 exact reference marker checkpoints
and all31 original cell checkpoints through bulk/Mux/old-raw-client paths.
Strict stopped at two discarded integer-conversion sources and one missing
field doc. Corrections preserve the original TryFromIntError in the library
error and document the explicit clamp; new strict compilation remains owed.
The prerequisite final docs/consumer-gate condition are merged without any
product delta from that prerequisite. No full-package approval is inferred.

The first corrected strict run reached tests and reported only a wildcard on
the owned AnchorState and a map get/unwrap. Explicit remaining variants and
direct fixture indexing preserve every assertion. No whole gate or review is
claimed from this lint correction.

## Borrowed frame projection

[Decision Log]
- Purpose: atomically project cells, cursor position/visibility, negotiated modes
  and marker disposition for an on-demand authoritative frame.
- Existing asset: the one engine's retained cells and snapshot cut. Cloning a
  maximum-size grid for every viewport read defeats the bounded frame request.
- Alternatives: restore a mutable decoder from visible rows or clone all cells
  before selecting the requested viewport. Neither preserves the required cost
  or decoder state boundary.
- Choice: a borrowed FrameView over live Screen or immutable Snapshot. It holds
  no runtime/actor and allocates no cell grid. Typed viewport intent chooses
  FollowLive or a caller-owned pinned marker; adjustments remain explicit.
- Why: the node actor can preflight cell count and enforce its actual encoded
  response-byte budget before copying selected borrowed cells into wire DTOs.
- Cost: a live borrowed view cannot outlive its authority borrow. The node owns
  frame revision/event cut, bounded waiters and transport/admission; no pure
  frame identity is treated as authentication or derived from EventSeq here.

Frame origin records retained/evicted/reset/inactive/beyond-live adjustment,
requested marker projection and actual first row. Released handles refuse.
The viewport height is explicitly limited to the current screen's row count;
latent alternate right columns remain available to capture but never leak into
visible frame columns. Program cursor shape, CLI rendering and UI validation
remain separate unfinished D1 work; this projection exposes the prerequisite's
position/visibility contract only.

FrameRows selects a prefix height from the reference viewport origin, not a
PTY resize or a different terminal size. A complete renderer asks for the
current terminal row count. Smaller bounded reads retain the same explicit
first-row coordinate. The live path borrows Screen directly, avoiding a full
grid Snapshot clone just to select a small viewport. Wire minimum-size/cell
preflight and exact encoded-byte refusal stay with the node DTO owner.

## Final candidate preservation and responder limit

UnansweredReplies preserves the ordered candidates from the same parser pass,
separate from normal Replies. One completed overlap emits one candidate set;
chunking does not duplicate it, later feeds do not replay it, and the node's
ordinary writer receives none of it. The raw span remains unchanged. A normal
Kitty query is also explicitly tested with all split points: negotiation
remains in presentation and only the query is omitted/replied by the node.

The responder guarantee is normal-only. Multiple old raw clients/history
replay may duplicate ambiguous responses; a frame-only observer receives no
answer for that exception. Candidates must not be settled/written or replayed
as an instruction until a separate future route exists. No speculative
delegation RPC is added here. NodeAuthority stays off until the reviewed node
responder contract and the full whole gate pass.

The exact9dfdf0a1 alias-table leaf alone is staged as34c08f6e5 for the follow-up
whole; no other batch changes were adopted. Prerequisite0d7 remains unchanged
on its separate branch. A fully green enclosing whole on this corrected base
can settle its open consumer condition as well as this extension's gate.
The independent native workspace must point its VTE patch table at the same
vendored copy when adopting this dependency; no native dependency/compile or
renderer retirement is part of this pure producer yet.
