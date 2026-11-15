# Complete readonly frames over compacted history

[Decision Log]
- Purpose: a frame promising rows times columns must expose every selected cell.
- Existing assets: the single engine compacts default trailing history cells;
  physical capture intentionally reads only stored columns and trims text.
- Alternatives: allocate or grow history on reads, retain full-width blank rows,
  or let each node/UI renderer invent missing cells. These add mutation, memory
  or competing interpretation at consumers.
- Choice: only the frame projection checks that the physical row exists, then
  borrows its stored cells or returns an immutable default blank for its implicit
  trailing columns. Physical buffer/capture APIs keep their existing meaning.
- Why: padding is part of the authoritative grid, independent of wire allocation
  or a renderer. No parsing, mutable decoder restoration or allocation is needed.
- Effects: empty and short history rows produce complete frames. Invalid frame
  coordinates and genuinely missing rows still refuse; stored colored cells and
  wide continuation projection remain unchanged.

The actual d7dd display regression failed on retained history row1,column1
before this repair. The dedicated frame regression walks the complete promised
cardinality for both the live authority and its immutable snapshot, including
an entirely compacted empty history row. It also asserts out-of-frame refusal
and that the physical buffer API is not silently widened. Narrow checks remain
owed. The frozen2807 source/whole is unchanged and cannot prove this repair.

Exact06f3 narrow verification executed38 tests with no failures and strict
all-target Clippy passed. Additional review coverage checks every selected
cell through wider/narrower resize, alternate/reset transitions, preserved
wide/continuation ink and unchanged snapshot geometry, and directly checks
that a nonexistent physical row remains unavailable. That strengthened test
head still needs its own narrow result and changed-source enclosing gate.

The strengthened a637 head executed39 tests with no failures. Its strict check
rejected unchecked arithmetic in the test's full-grid counter. The counter now
checks overflow without weakening the cardinality assertion or changing product
code. Its strict check and the changed-source enclosing whole remain owed.
