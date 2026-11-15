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

Published VTE baseline vendored. Next: minimal metadata changes, pure mode and
presentation regressions, T audit, changed-package checks. No node activation,
transport or renderer acceptance has occurred.
