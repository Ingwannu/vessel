# Shared button content, loading and measured sizing

Coordinator assigned G2 the sole renderer writer, before the remaining F1 work.
Three control-only changes are ordered: typed content, loading content, measured
size parity. Consumers are S2 settings reorder and G3 provider catalog Add.

Pinned Paseo1c00b365: components/ui/button.tsx (whole), control-geometry.ts (whole),
components/ui/control-size.tsx, components/provider-catalog-list.tsx (whole), and
screens/settings/sidebar/sidebar-nav-section.tsx (whole). MD is pointer32/px16
or touch44/px24, glyph14. SM is28/px12 in both layouts, glyph12. Icon-only width
is the chosen height and horizontal padding0. Catalog Add explicitly chooses SM;
its override is only flexShrink0. Sidebar reorder uses MD with an explicit muted
14-point React element, so its ink stays muted even when Ghost text hovers.
This corrects the earlier ledger's assumed touch32 SM value.

[Decision Log]
- Purpose: one shared label/icon/loading button, without empty text or area divs.
- Existing constraints: legacy label entries and all filled/Ghost paint states
  retain their signatures/defaults; icons need their own SVG ink on this GPUI.
- Alternatives: toolbar pills, area arrows/spinners, or another button renderer.
- Choice: Label/Icon/IconAndLabel with automatic or explicitly muted icon ink;
  extract the existing typed renderer to widgets/button.rs before it grows.
- Why: one paint and geometry path preserves the accepted pointer/touch model.
- Costs: compatibility label-only calls without a Window still default Pointer;
  ownership of their density migration remains outside this control delta.

Step1 is a source checkpoint. Existing caller invocations are unchanged; the
legacy label entry delegates to Label in the same renderer. No actual control,
consumer, narrow or UI PASS is claimed until its specified checks finish. The
F1 list and its pending physical-node binding correction are outside this scope.

## Isolated producer boundary

This control-only producer is directly on6036eeb8, with no F1 app/View/Link/menu
changes. The ControlSizing module is included because the assigned base lacks
it. The full F1 branch retains its menu alias regression; only that nonexistent
producer alias assertion is excluded here. Step2 reuses exact434933b3 loader,
asset and legacy spinner blobs; the base size record only lowers719 to711.
No old Link size entry is revived. Only the final range head is to be narrowed;
no independent stage1/stage2 gate or UI approval is claimed from that result.
