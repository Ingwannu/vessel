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

This package starts on the base-13 tree. The reference oracle now retains
34 finite-amount and 13 exact unsigned cases; the stock-Fluent carrier
characterization tests are source ready, VM check owed. ADR 0101 records
the exact candidate dependency requests and the scratch measurement contract.
The coordinator requires its yes/no before lock changes; the tool owner is
preparing the missing offline experiment interface. No dependency, runtime
patch or product behavior has been adopted. The next action is that narrow
characterization check and scratch-only runtime adapter comparison, not
another run of the finished generator checks. All Rust compilation stays
on the build VM.
Stage 1 is now a controlled offline scratch graph, pinned at `d633cdd4d` with
code snapshot `bc948736b`. Run `i18n-number-experiment-20261116T014841Z-d633cdd4`
is queued; it resolves/audits the graph before strict and eight narrow probe
tests. The ICU comparison may follow only after that label is terminal.
It is not an installed-app measurement or product lock update.

The stock-carrier characterization's exact `08076af53` strict run
`i18n-narrow-20261116T004824Z-08076af5` passed, with actual View recompilation.
Its three runtime tests remain queued in
`i18n-narrow-20261116T014848Z-08076af5`; no execution result is borrowed.

Later unlinked sources prepare folder-based placement, whole amount patterns,
a single-locale narrow kernel, signed-zero keys and whole numeric-pattern
English fallback. They are outside the frozen stage-1 inputs and remain
uncompiled. Actual app callback overlays, the agreed native-workspace View
example `number_format_probe`, generated-message unsigned binding/fallback,
dependency size/time artifacts and coordinator adoption approval remain owed.

No own fleet/display is active. About dependency rows must preserve the voice
package's coupled audio entries when that accepted package reaches the line.

### 2026-11-16 carrier preparation and first controlled graph

Stock carrier controls exact08076af53: strict zero, runtime three passed,
zero failed/ignored; actual View recompiled. Narrow D633 feasibility: offline
metadata zero, graph-admission red, compile_attempted=false. Existing package
versions/checksums unchanged but four packages gained feature-unified edges;
T is examining the exact allowlist rather than weakening the guard. Both
receipts are preserved. No ICU retry yet and all release/wasm delta cells
remain unmeasured.

New unlinked option/decimal preparation and custom carrier controls retain
source/English numeric results, exact unsigned input and compact magnitude
through nested NUMBER. These remain source-only; the actual native callback,
generated binding, numeric-pattern failure boundary and fixed wasm example
overlays are still owed. Product source/manifests/locks are unchanged.
