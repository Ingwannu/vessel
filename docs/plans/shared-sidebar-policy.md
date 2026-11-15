# Portable sidebar policy and toggle seam

[Decision Log]
- Purpose: Settings and the shortcut owner need the one panel decision and
  toggle intent without importing unrelated unfinished F4 controls or pages.
- Existing assets: view::panels owns width limits and placement; app PanelEdge
  already records the actual drawn width and reuses the shared native resize
  handle. The shell already owns both panel preferences and lifecycle.
- Alternatives: another width constant, copied settings frame, per-area
  toggle state or a second resize controller would disagree with the root.
- Choice: extract the existing S2 policy/adapter from Main698 onto announced
  c3429952. Five complete source modules and the arrangement function match
  that immutable Main candidate exactly. Only registration/frame hunks are
  applied to the older sidebar body; its existing renderer stays intact.
- Impact: no new kit control, glyph, navigation inventory, settings page,
  command registry or mutation protocol is included. The source is still
  uncompiled in this isolated union; Main698's pending check is not a pass.

Pure APIs: panels::sidebar_form(viewport), sidebar_share(room,width,context),
sidebar_minimum(context), sidebar_layout(viewport,room,asked,preference,context).
SidebarContext::{Workspace(code placement),Settings};
SidebarLayout::{Sheet,Hidden,Docked{width,sharing}};
Sharing::{Fits,TooNarrow}. App uses
areas::sidebar::arrangement(&Shell,&Window)->SidebarLayout including the live
dragged request, not a second remembered width.
Shell::sidebar_toggle_both(window,cx)->bool is the single legacy owner action.
W22 registers it and owns the subsequent handled-sidebar focus-mode exit;
remove its staged dead_code expectation with the actual product caller.

Reference: Paseo components/desktop-sidebar-layout.ts (whole),
constants/layout.ts:19-22,37-40, utils/desktop-sidebar-toggle.ts:15-30 and
workspace-screen.tsx:2632-2657; off-workspace _layout.tsx:398-411 toggles left
only. Four actual old panel-state combinations and the off-workspace case are
installed as GPUI tests; the existing pure budget tests cover numeric edges.

Explicit native choices:
- Sidebar form follows actual viewport600; global PHONE768 and other areas
  stay unchanged.
- Limits remain least200/most600; default264 and native frame inset12 are
  existing lead-approved differences from Paseo320/full-window budgeting.
- Settings asks the same width rule to leave720 (320 navigation +400 detail),
  per lead instruction. This may shrink the app sidebar before declaring the
  budget too narrow, whereas the source evaluates its requested sidebar and
  share predicate separately.
- Settings alone keeps the old PHONE->Sheet and auto-hide exceptions until E2
  installs the typed list/section/back alternative atomically. It consumes this
  actual result, not copied600/720 constants; stacked frame and control density
  must remain separate. No second320-wide settings frame is created here.
- Existing code-panel minimum/overlay policy remains with its view owner;
  this extraction does not claim the canonical Explorer is mounted.

PanelEdge's measure state, rather than its generic width helper, supplies the
actual drawn width at drag release. Therefore the Settings minimum flows
through the same frame measurement without another clamp or controller.
The obsolete app width_leaving forwarding method has no caller after the
root consumes the pure decision and is removed; pure Limits API stays intact.

No full sidebar UI, canonical empty-workspace routing, node-bound mutation,
all-row accessibility, OS keyboard or latest combined application pass is
claimed. W11/W12 runtime workspace projection and W21 guarded mutations remain
separate dependencies. Freeze source, run exact isolated narrow checks/tests,
then hand this small range to E2/W22; do not pull the moving whole F4 branch.

Exact isolated c8dbce21 strict reached App test compilation and returned101 on
five style diagnostics in policy_tests only (semicolons and redundant closures).
Use the existing Shell::toggle_sidebar method directly and terminate unit
statements; assertions, panel values and product logic remain unchanged.
This corrects the actual failed run, not a claimed successful check.
