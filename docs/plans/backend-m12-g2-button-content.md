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

Step2 adds ButtonConfig (typed size/layout/activity) and button_control, which
returns a styled Button with an input guard. Disabled or loading refuses a
supplied on_click callback; completion renders an enabled control again. The
same renderer replaces any leading glyph with LoadingSpinner Small20 and keeps
the label. New Outline uses the source borderAccent and transparent fill;
catalog Add chooses Small28/px12/font12-medium. Explicit static muted artwork
is replaced by the button's own variant ink while loading, as the source does.
A headless input regression deliberately supplies callbacks in every state.
Existing legacy label APIs remain unchanged; their disabled callback convention
is not retroactively changed by this new guarded entry. Source/narrow/UI proof
is still owed for this checkpoint.
