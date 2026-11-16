# Additive source segmented layout

Owner: G2, coordinator3993faee; Track F is closed. Isolated on announced
app base10 3e7820e3. No moving F1, menu, field, tooltip, ordinary Button or
Toolbar source is part of this leaf.

[Decision Log]
- Purpose: the Schedules default MD filter must have its compact geometry.
- Existing assets: kit segmented renderer, SegmentSize, ControlSizing, touch44.
- Source: pinned Paseo1c00b365 schedules-screen.tsx279–284 uses default MD;
  segmented-control.tsx55–65 resolves the size;190–203 maps hover/press/disabled;
  control-geometry.ts32–35/55–61/308–326 resolves outer32/44 and inset2.
- Choice: segmented_at(size,sizing) and segment_at(...,size,sizing,tokens) opt
  into one private renderer. MD outer32/44, inner28/40, px12/gap4/label14;
  SM outer28/inner24/px8/label12 remains the same in both layouts.
- Why: caller wrappers would conceal a wrong inner target and duplicate geometry.
  Legacy segmented/segment and public height/inner methods keep their pointer
  dimensions and paint. Only the new opt-in gets unselected pressed surface3,
  after hover surface2; selected remains surface3. No old caller is migrated.
- Input ownership: as in the existing API, caller owns value/change, attaches no
  disabled callback and refuses a choice of its current value. No new input or
  tooltip controller is introduced.

Both-theme literal style tests cover MD/SM/layout, padding/gap/type/ink,
selected+disabled, and legacy defaults. An authored real GPUI test drives
hover/down/up and inspects actual painted scene quads, comparing source and
legacy press behavior and real inner bounds. It is not native pixels, OS,
accessibility or a feature-screen approval. Current source has local direct
rustfmt/size/token/diff checks only; remote Kit strict/tests and V1 are owed.

The queued F1 consumer run3a5 is unchanged and excludes this leaf. F1 opts in
only after the checked producer handoff and preserves source widths/no-op
selection behavior. No whole package or priority lane is requested here.

## Exact completed narrow receipt

Exact577bb0a015222b70da3d384b45d7301abc47abf2 completed ordinary SHORT,
priority0: schedules-20261115T185339Z-577bb0a0 Kit all-target Clippy0;
schedules-20261115T190451Z-577bb0a0 full Kitlib235/0/0. Both runs have fonts/
cargo0, no skips, immutable exact checkout and actual Kit Checking/Compiling.
The both-theme literal case and actual scene hover/down/up/inner-bounds case
explicitly executed successfully. Author verified every collected run-file hash
and all four step digest prefixes. Prior source precheck is not relabelled as
execution review; same-row V1 final disposition is requested separately. No
App, native pixel/OS/a11y/touch, consumer or full F1 approval is inferred.
