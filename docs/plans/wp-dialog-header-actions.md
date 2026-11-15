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
redraw. The producer stays isolated atop the reviewed base8 dialog/popover range;
the selected narrow result below does not imply V1 review or lead landing.
The exact4c9 first suite passed formatting and structural checks, then stopped
at one unused test import and six primitive float comparisons. Gap helpers now
return typed Pixels and compare literal pixel units; no lint allowance, default
or product behavior changes.

Exactdd7ef7eca521e0ceb59c05e089f2f08ba38bc4b5 passed ordinary selected run
wp-desktop-popover-narrow-20261115T153102Z-dd7ef7ec: fonts, fmt, quality,
all-target Clippy and test all0. Aggregate1447passed/0failed/2ignored includes
Kit196/0/0; all three added title-action tests explicitly passed. The source range
from6c70a785 has only the canonical dialog, its additive input/tests and docs.
The documentation-only follow-up leaves the executed native source unchanged.
No App/Link/manifests/lock/root delta, native-build/export, OS capture, IME/a11y,
actual W5 consumer or full E3/package approval is claimed. Historical native
Linux focused-field F5 remains open. Caller-owned stale-action authority and the
existing G4 actual-close ACK are still separate obligations.

V1 row249 passed bounded SOURCE/requested selected NARROW on exactdd7ef7ec plus
docs-only465705edd. The review independently checked five source pins, executed
commit/spec/checkout/log hashes and all three added cases. Its fixtures are dark,
headless desktop and compact windows; this does not prove both-theme visuals,
native Linux focused-field F5, consumer authority or OS/IME/accessibility behavior.
The lead may merge only this selected producer and preserve newer kit unions.
