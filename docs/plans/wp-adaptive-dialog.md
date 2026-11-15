# Shared adaptive dialog configuration

The coordinator authorized one typed extension of the existing dialog and snap
engine. Legacy Normal520/Wide640 fitting geometry and 60/90 snap defaults stay
unchanged. DialogLayout adds positive desktop width, AtMost/Exact viewport height,
and CompactLayout::Fixed/Draggable using validated Percentage and SnapPoints.
No second renderer is introduced. G4's reviewed captured dismissal and ReturnFocus
remain authoritative; only actual dismissed acknowledgment clears snap state.

[Decision Log]
- Purpose: authored changelog geometry and arbitrary adaptive content surfaces.
- Constraint: preserve all existing control defaults and one gesture engine.
- Alternatives: per-area sheets or replacing legacy fitting layout globally.
- Choice: typed opt-in layout and snap points, defaulting to existing behavior.
- Reason: explicit callers get exact dimensions without changing existing modals.
- Tradeoff: callers select geometry; platform rendering still needs actual proof.

Reference: Paseo packages/app/src/changelog/internal/changelog-sheet.ts:80-90
uses desktop620/exact85%, compact85/95 draggable. Configuration tests verify actual
short-content bounds and snap/scroll handoff. The desktop worker's earlier be31
Kit suite passed166 tests including these changes; this isolated dd13 union is
not yet checked and does not inherit a new combined PASS from that older result.

Producer source is isolated from E3 app/link changes. Run Kit all-target Clippy
and Kit library tests remotely, then request V1 review before lead consumption.
