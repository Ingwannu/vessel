# Shared small toolbar icon chrome

Status: source preparation; no compile, runtime, source-adoption or header parity
claim. This adds the coordinator-assigned shared control, not a second ordinary
Button renderer or a change to ButtonContent/Small/Medium sizing.

## Pinned reference

Paseo 0.15.4, commit `1c00b365c9e0b74c1580eba33d905d693125ae21`:

- `packages/app/src/components/ui/pane-content-toolbar.tsx:81`: ToolbarButton
  calls small icon chrome with owner selected/open/disabled state and a label.
- `components/ui/icon-button-chrome.ts:10-25,72-96,99-161`: small frame20 pointer,
  compact32; no padding, radius md, no shrink; active/hover/press/open highlight,
  disabled opacity50%; helper glyph14 pointer/18 compact.
- `screens/workspace/workspace-desktop-tabs-row.tsx:252,301,305,344-357`: the
  workspace rail explicitly supplies14-point extra-muted vectors, including the
  selected maximize/restore pair. The fixed glyph does not scale with the frame.

These are distinct from ordinary Button's Small28 and Medium32/44, and from other
checkouts' newer control-geometry values. They must not be interchanged silently.

## Decision Log

- Purpose and intent: share one exact small icon hitbox and interaction chrome
  between pane headers and other toolbar owners.
- Existing assets and constraints: the kit already owns vector assets, neutral
  interaction-highlight/radius tokens, explicit ControlSizing, ButtonState and
  screen-owned Tooltips. Ordinary Button has different size and press semantics.
- Alternatives: area-owned squares; empty-label ordinary Button; change ordinary
  sizes; or a named toolbar control using those existing lower-level assets.
- Choice: one `toolbar_button` module with ToolbarButton, immutable ToolbarConfig,
  typed Selection/Disclosure and ToolbarIcon ink/fixed-or-adaptive sizing.
- Why: no caller geometry override or duplicate per-area interaction handling,
  while selected/menu state remains with its actual owner.
- Benefits, costs and effects: no new dependencies, connection/resource identity,
  listener, menu state or feature admission. Ordinary Button source stays intact.
  The normal handler API refuses disabled callbacks even when one is supplied.

Callers pass the actual window layout once and rebuild the config from their own
selection/menu state. The toolbar supplies role/name but creates no keyboard
listener. Its optional tooltip decorates the same element through existing
Tooltips, whose immediate/top/no-fade behavior differs from the reference toolbar
tooltip's300ms/bottom presentation. That existing tooltip difference remains
explicit; this chrome slice does not claim to solve it or OS accessibility/focus.

The requested checks cover literal20/32/14/18 values, selected/open/hover/pressed
paint, fixed disabled opacity and actual GPUI bounds/click refusal/re-enable.
Header consumption, correct restore glyph/label choice, real menu ownership,
keyboard/focus, inspected captures and surface parity require their own union.
