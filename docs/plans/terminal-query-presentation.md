# Terminal query presentation and attach follow-up

The prerequisite engine remains a separate frozen package. This branch owns
the following D1 work; none is consumer-ready or activated yet.

[Decision Log]
- Purpose: one query answer across node-owned views, current CLI rendering,
  older raw CLI clients and history replay.
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
