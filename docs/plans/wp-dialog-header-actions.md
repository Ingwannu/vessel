# Additive dialog title actions

Modify the canonical dialog renderer. Its existing row already matches Paseo's
24px horizontal inset,16px vertical inset and8px gap; no separate header or modal
renderer is needed. No new dependency is introduced.

[Decision Log]
- Purpose: UsageModalFrame places HostFilter, RefreshAll and Options in its title row.
- Constraint: existing DialogContent struct literals and absent-slot geometry must remain unchanged.
- Alternatives: a new struct field would break every Rust literal; an area header would duplicate the renderer.
- Choice: DialogContent::with_header_actions returns typed DialogInput, and the same dialog entry accepts Into<DialogInput>.
- Reason: plain content converts with None; the builder carries one optional caller-owned element before the existing close button.
- Consequences: title yields with single-line truncation only when actions exist. Callers own action context/ticket admission; no UI lease or host authority is inferred here.

Paseo references: packages/app/src/usage/usage-modal.tsx:59-90 passes controls
through header.actions and desktopMaxWidth640. Its controls.tsx:19-71 groups the
host filter/refresh/options with spacing[2]=8. The canonical row is in
components/adaptive-modal-sheet.tsx:111-145,380-440: paddingHorizontal spacing[6]=24,
paddingVertical spacing[4]=16, gap spacing[2]=8; title flex1/minWidth0 and
numberOfLines1; an optional horizontal actions row precedes the close control.
The existing native close dimensions28desktop/44compact stay unchanged.

API: keep every old DialogContent literal. Pass content.with_header_actions(body)
to the same kit::dialog::dialog call. The body is an arbitrary existing UI element;
the shared renderer adds no provider callbacks, host data or action authorization.
The added DialogInput has private fields and a plain-content From conversion.

Tests install literal24/16/8 rail assertions in desktop/compact layouts, verify
absent-slot legacy61/77 header heights, keep the close button visible with a long
title, dispatch actual caller controls without closing the dialog, reuse G4's
actual-opener focus on close and exercise retained caller-context refusal before
redraw. These new tests are not yet compiled or executed. Source formatting and
budgets are not a runtime or V1 approval. The producer stays isolated atop the
reviewed base8 dialog/popover range; then remote narrow, V1, and lead landing.
The exact4c9 first suite passed formatting and structural checks, then stopped
at one unused test import and six primitive float comparisons. Gap helpers now
return typed Pixels and compare literal pixel units; no lint allowance, default
or product behavior changes. Tests have not executed yet. The next suite uses
the ordinary lane under the shared-seam build rule.
