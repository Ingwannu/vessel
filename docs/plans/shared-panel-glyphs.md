# Canonical panel glyph roles

[Decision Log]
- Purpose: workspace rails need split columns/rows, panel maximize/restore and
  UI-close roles. Window chrome and the session-fork mark are different roles.
- Existing assets: the append-only Glyph inventory, one exhaustive drawing
  match, one24-grid/1.5-stroke macro and the existing SVG asset/raster engine.
- Alternatives: reusing window/fork shapes, local area SVGs, raster assets,
  aliases or a new icon dependency would obscure the role or duplicate drawing.
- Choice: append Columns2, Rows2, PanelMaximize, PanelMinimize and X after the
  published95 prefix. Use canonical Lucide1.50 vector data through the existing
  line renderer and retain the full upstream ISC/Feather notices.
- Why: callers get distinct typed roles without a second asset loader, fallback,
  bitmap or window-control substitution. Every original name/path/vector/order
  stays unchanged; the existing frozen-prefix snapshot remains the contract.
- Impact: no caller is migrated here. These are static SVG paths and inherit
  the same theme ink and fractional display scale as existing line glyphs.

Reference: pinned Paseo1c00b365, screens/workspace/
workspace-desktop-tabs-row.tsx:22-27,301-305,332-346,412,438. Toolbar roles use14
extra-muted, while tab menu X uses16 muted. app/_layout.tsx:926 provider and
styles/theme.ts:602 set the effective stroke1.5, overriding library default2.
Canonical source: lucide-icons/lucide tag1.50.0 icons/{columns-2,rows-2,maximize,
minimize,x}.svg and LICENSE. Full copyright/permission notices live beside the
brand documentation; vectors are embedded through the existing macro.

Tests check unique registration,24-grid/1.5 parsed strokes, no image/text node,
role-specific divider/corner ink (so swapping maximize/minimize fails), and
unclipped vector rasterization at logical14/16/20 with scales1/1.25/1.5/2.
Existing95 glyphs and their snapshot are preserved. Raster tests are not native
GPUI first-use/OS/consumer UI approval; those remain with the workspace rail.

Resume: isolated source on package base9 db42d9c5. No Rust compile/test or V1
pass yet; serialize its narrow suite after the currently queued owner tests,
then provide the exact checked leaf to review and the shared line first.

Source preflight: independently compare the old95 declaration/path sequence,
the full old drawing prefix and frozen inventory test against base9; all are
byte/value identical. The five appended SVG node/attribute sequences match
the canonical1.50 source exactly, while the existing line macro supplies the
actual provider stroke1.5. No bitmap/image node is introduced. The system
librsvg source overview in light/dark at all listed sizes/scales was opened:
axes, outward/inward corners and X remain distinct and unclipped. That is
source-only rendering, not executed Rust raster tests or native UI evidence.

Asset/drawing module documentation distinguishes the newly licensed canonical
panel vectors from older original drawings, points at the full notice and
names both10-grid window chrome and24-grid UI roles. This changes no old SVG,
loader or prefix entry; it prevents the new licensed vectors being described
as original artwork.

## Exact isolated narrow result

Source b41d3d84324d3b38ac0c3673ca007d2dda152176 passed Kit all-target strict
Clippy and all214 Kit lib tests,0 failed/0 ignored, on the build machine.
Runs: shell2-narrow-20261115T175542Z-b41d3d84 and
shell2-narrow-20261115T175924Z-b41d3d84. Both font/Cargo steps returned0;
actual Kit Checking/Compiling and all three new role/raster tests plus the
frozen original-prefix case are explicit in the logs. The named scheduling
permission covered these two sequential checks; no concurrent Cargo ran.

The review packet pins all six changed files, exact source diff, canonical
vector/source comparisons, unchanged95 prefix and raw run/log hashes. The two
source SVG overview originals were inspected in both themes. This result is
not native consumer first-use, OS, full application, F4 or workspace-toolbar
approval. No caller/toolbar/button or future Import is included. This result
commit changes documentation only; its native diff from b41 is empty.

Local boundary-script provenance is corrected: that script internally invoked
Cargo metadata, so its earlier local result is not fresh VM-only boundary
evidence. No local compilation or pass is claimed; future quality/boundary
execution remains VM-only. The isolated manifests, lock and dependency edges
are byte-identical to package base9. Direct pinned rustfmt, size, tokens and
text/import/doc checks in the preflight invoked no Cargo.
