# Shared header navigation glyphs

[Decision Log]
- Purpose: the screen header's leading marks match Paseo once in the kit.
- Constraint: legacy Menu is three full bars; PanelRight names another panel.
- Alternatives: silently change either legacy asset or draw local header bars.
- Choice: original PanelLeft on the 24-unit/1.5-stroke line grid, and an
  original MenuCompact registered at its exact 16×12 painted footprint.
- Why: `headers/sidebar-menu-toggle.tsx` uses a PanelLeft on desktop and
  two 16-wide bars plus an 8-wide bar on a phone. No reference asset is taken.
- Cost: these two entries plus the paired transfer glyphs grow the inventory
  from 65 to 69; no existing path/caller changes. The transfer provenance
  is in `backend-m12-g2-transfer-glyphs.md`.

MenuCompact deliberately is not a 24-grid glyph: the reference's composed
Views are 16×12 with bars at y 0, 5.25 and 10.5, height 1.5 and radius 0.75.
Filled rounded rectangles preserve those outer dimensions; round stroke
caps at x 0 and 16 would paint outside them. Its SVG frame and the header's
16×12 element agree. Literal vector extents and native-size raster rows,
gaps and the short bar's clear right half are tested; PanelLeft is loaded
and rendered at 24 with clear outside edges. Both legacy paths remain.

The screen-header control is a separate commit. Its SVGs set ink explicitly,
which this GPUI pin requires; native captures must verify final appearance.

The SVG normalizer computes a rounded-rect extremum at x=0 as about
-2.75e-32. The bounds assertion normalizes only zero-origin noise smaller
than f32::EPSILON (under 0.00000012 of a pixel); its y/width/height values
and native raster row/gap/right-half checks remain exact. This handles
floating representation, not a change to the reference geometry.
