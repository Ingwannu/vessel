# Shared Usage and catalog glyphs

Decision: 수정 사용 — extend the existing Glyph/Assets/drawing renderer and its
24-unit/1.5-stroke line macro; no new dependency, alias or area SVG.

[Decision Log]
- Purpose: supply the one reserved Gauge for E2/S2 Usage and PackagePlus for G3.
- Constraints: check package base and V1 ledger before appending; preserve every
  existing vector/path and use the kit's own drawings in its existing line style.
- Alternatives: area drawings, a chart/package substitute or copied vendor assets.
- Choice: original dial/needle and package/plus vectors in the existing registry.
- Why: callers use one named asset; no second renderer or additional dependency.
- Costs: consumer first-use appearance and interaction remain their final package
  evidence, not a claim from a vector/raster unit test.

Pinned Paseo1c00b365: settings-screen.tsx HOST_SECTION_ITEMS usage257 and actual
SidebarHostSectionButton842/1020, densityIconSize(md)16 pointer/20 touch and
explicit SIDEBAR_ICON_STROKE_WIDTH1.5. Provider catalog fallback is
components/provider-catalog-list.tsx29/68–77, PackagePlus20 in its36 surface2
frame. Its LucideProvider chain supplies1.5 (the inherited stroke was independently
confirmed by V1); a library default must not be assumed. The kit line_glyph macro
has the matching grid/stroke, but the vectors are original, not extracted assets.

Only Gauge and PackagePlus are appended; no CircleGauge/MoreHorizontal aliases,
no change to existing assets or mark controls. Registration uniqueness, parsed
stroke width and full ink bounds are checked, and both render at16/20/24 through
the existing SVG raster path. These source/raster tests do not prove native
light/dark first-use captures, visual comparison or E2/S2/G3 product wiring.
Changed-crate narrow and V1 review are still owed for this checkpoint. No whole
package gate or UI pass is claimed.

Exact5ecd remote Clippy after the VM-resize143 resubmission compiled Kit and
returned one float_cmp lint in the new parsed-stroke assertion; no tests ran.
The regression now compares IEEE bits to literal1.5_f32, preserving exactness
without an epsilon or lint exception. SVG/vector bytes and all render cases
are unchanged; old-source red outcome is retained. Corrected checks are owed.
