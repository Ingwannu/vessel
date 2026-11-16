# Shared locale number formatting

State: contract/skeleton; ADR 0101 precedes code. No new dependency or
formatter implementation has been adopted.

## Scope and structure

Follow Paseo 0.15.4 `usage/format.ts:185–199` for full-finite amounts with
typed units and provide a separate exact unsigned-count path. ADR 0101 owns
the range, rounding, data and measurement requirements. The shared view
kernel prepares display text; app consumers cache it at events and borrow it
while rendering. Locale data remains in translation folders.

The existing `Number`, `NUMBER`, grouping rules and `FiniteNumber` are the
starting assets. Their current float/`u32` limits do not close this package.
The backend remains undecided until measured; candidate experiments are not
an approved public API or consumer implementation.

## Sequence

1. Commit ADR/contract and reference findings.
2. Prove exact scalar/rounding/selector feasibility with maintained APIs.
3. Measure linked release/wasm cost for ICU and a narrower data-driven kernel;
   distinguish probe results from actual installed-app deltas.
4. Append the measured choice to ADR 0101, then implement one shared producer.
5. Check strict/view/wasm and give the checked interface to the Usage owner.
6. Record consumer integration, language-event preparation and visual evidence
   separately. No existing generator or General pass supplies those results.

## Resume

The generator package is V4 row 248 source/changed-View PASS at `c8290e56e`
with documentation leaf `bfc9f6f19`; the lead merged it at `f2169502` and it
is present in green package base 13 `2ebeedf9`. Its 1086 passing View tests
are historical generator evidence. No numeric-extension test has run yet.

This package starts on the base-13 tree. The next action is scalar/selector
feasibility and an immutable dependency-cost experiment, not another run of
the finished generator checks. All Rust compilation stays on the build VM.
No own fleet/display is active. About dependency rows must preserve the voice
package's coupled audio entries when that accepted package reaches the line.
