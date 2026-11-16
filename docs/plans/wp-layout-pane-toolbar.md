# Pane header shared-control consumption

## Decision Log

- Purpose: replace the layout header's empty-label ghost buttons and window/fork
  glyph reuse with the checked shared Toolbar and canonical panel UI roles.
- Existing assets: the announced line already supplies `ToolbarButton`, its
  typed small geometry/selection, the same `Tooltips` controller, and the five
  canonical panel glyphs. Layout actions and their captured selection check exist.
- Alternatives: changing ordinary Button sizes would affect unrelated callers;
  local SVGs, timer or style overrides would duplicate shared owners. No external
  toolkit is needed for this existing header.
- Choice: modify this header to consume those shared APIs. Keep its original
  current-selection check before dispatch and original local operations. Pass
  the actual drawn Layout to decide whether multiple panes exist.
- Why: source20/32 frames, fixed14 glyphs, extra-muted ink, current selected
  maximize/restore state and bottom8/300ms desktop tooltip now have one owner.
- Cost: the current direct split controls and empty-only close remain. This is
  shared-control consumption, not full source tab-rail or split-menu parity.

## Pinned source

Paseo 0.15.4 `1c00b365c9e0b74c1580eba33d905d693125ae21`, paths relative
to `packages/app/src/`:

- `screens/workspace/workspace-desktop-tabs-row.tsx:297-380`: fixed14 extra-muted
  Columns2/Rows2 and current Maximize/Minimize; maximize/restore label and selected
  state follow the actual pane. The source uses a split dropdown, still a gap here.
- `components/split-container.tsx:740-742`: maximize is offered for multiple panes.
- `components/ui/pane-content-toolbar.tsx:81-160` and
  `components/ui/icon-button-chrome.ts:10-18,99-161`: use the shared small chrome
  and desktop-only bottom8/300ms tooltip. Compact sizing is the app's actual
  compact layout policy, never inferred from a mouse/device capability.
- `screens/workspace/workspace-desktop-tabs-row.tsx:406-413`: UI X, independent
  from a window close role. This adapter uses it for its existing empty-pane close.

## Scope and checks

No Kit, Link, transport, resident staging or close authority is changed. The
header's UI action remains bound to its original complete `LayoutActionContext`;
this predicate is current selection equality, not a visit/lease/operation guard.
The new/close controls retain their existing behavior. A single-pane layout no
longer offers a meaningless pane maximize control; two-pane maximize displays
the shared selected highlight and switches to the panel restore glyph/label.

The authored GPUI cases exercise actual header paint bounds/click dispatch and
retained original-selection refusal against the same real layout book. They
have not executed. Receiving App strict/runtime are owed, followed by source
review and the lead's receiving gate. No native screenshots, source tab-rail
parity, shortcut badges, accessibility, OS/IME, resource or effect proof follows
from existing shared-control checks.
