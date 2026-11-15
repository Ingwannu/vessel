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

## Verified portable source checkpoint

Exact source5901da6a passed strict View/App all-target Clippy (fonts0/cargo0)
and App+View tests on the build machine:1187 passed,0 failed,2 existing ignored.
App96/0/2, View959/0/0, contracts10+3, page117 and docs2 are the raw groups.
The four actual root/toggle GPUI cases and three pure sidebar-budget cases are
explicitly successful. View/Kit/Link/App were actually compiled in the test run.
The earlier c8 test-style failure remains separate; the original queued test
run was cancelled before acquisition under explicit scheduling authorization
and retained terminal143, then the exact same source/command ran once in the
priority lane. Cancellation is not a compiler failure or a pass.

Runs: shell2-narrow-20261115T165454Z-5901da6a (strict) and
shell2-narrow-20261115T174716Z-5901da6a (tests). Source pins/diff and complete
raw logs are retained in the review packet. This docs-only result commit has
no native source difference from590. No whole native gate, captures, physical
shortcut, OS behavior, canonical Explorer, workspace identity or full F4 pass
is inferred. Settings' current exceptions remain until its atomic frame
consumer; the registry owner must remove the staged dead-code expectation
when installing the actual ToggleBoth caller.

Boundary-check provenance correction: earlier local boundary invocations used
Cargo metadata internally; they are not VM-only validation and are not promoted
as fresh VM boundary evidence. No local compilation was performed. Future
boundary/quality checks run only on the build machine; direct pinned rustfmt,
size, tokens and text/import/doc scans do not invoke Cargo.

## Source correction before root-policy captures

[Decision Log]
- Purpose: requested Shown must actually dock at exactly600 with neither
  Explorer nor Settings open. Comparing the known wrong590 visual state
  would conceal a source discrepancy rather than validate the correction.
- Existing constraint: the first portable policy reused one content number
  for both width targeting and visibility admission; this made the native
  frame12 inset hide ordinary content at600.
- Choice: reuse the same Limits resolver with its pinned400 target for all
  workspace widths, but admit ordinary content at0. An open native code
  panel keeps its own480 overlay /800 beside admission. Settings retains
  its existing720 width exception pending E2's atomic list/back fallback.
- Why: Paseo desktop-sidebar-layout.ts40–64 resolves width against400,
  while66–72 returns0 absent Explorer/Settings and74–87 compares that
  separate minimum. app/_layout.tsx431–452 passes the actual Explorer and
  Settings facts. Lowering the target to0 would incorrectly widen the row.
- Impact: default264, native frame12, existing code budget480/800 (Paseo
  Explorer600), and Settings exceptions remain explicit separate choices.
  Global PHONE768 and code-panel placement are unchanged. No canonical
  Explorer renderer or workspace identity readiness is inferred.

Regressions: actual shell at600, Shown, no Explorer/Settings must mount the
scroll body at width200; a pure Explorer-open case retains the400 width
resolver and separate native admission, including a requested600 at800 (resolved388 and hidden, whereas the old
coupling shrank to308 and admitted it). Old590
source/review/test evidence remains scoped to that source. The queued590
export was cancelled before acquisition at18:19:33 UTC (terminal143);
corrected-source tests and the one replacement export are still owed.
