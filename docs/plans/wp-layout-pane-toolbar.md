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

The GPUI cases exercise actual header paint bounds/click dispatch and direct
invocation of a captured `PaneAction` after the original selection changed,
against the same real layout book. The latter is not an actual retained UI
callback/visit proof. The pinned UI X example is exit-focus, not source evidence
of the existing empty-pane close behavior. No native screenshots, source
tab-rail parity, shortcut badges, accessibility, OS/IME, resource or effect proof
follows from shared-control checks.

## Executed affected checks

Exact `bf2a52e6f4eefedb5f4fa85a705fc7ed1b81e91f` completed requested locked
App all-target Clippy `-D warnings` in
`wp-layout-core-union-20261116T031857Z-bf2a52e6`, fonts/cargo0; actual Kit/App
Checking, View/Link cached. Same-head run
`wp-layout-core-union-20261116T032318Z-bf2a52e6` compiled View/Kit/Link/App and
passed all 40 selected App BIN `areas::layout::` cases, 0 failed/ignored, 146
filtered out. All three new header cases passed explicitly. The compact case
measures the actual one-button row and also asserts that compact Tooltips install
no anchor; the pointer case uses its actual shared trigger measurement.

The earlier `f0f59a05` strict run also passed, but its compact fixture had not
been corrected or executed. The new test marker/measurement changes no product
header geometry or action. This docs-only receipt adds no native delta and is
not a new-commit execution claim. Final review and lead receiving gate remain
separate, with the direct split/order/menu/badges and full-rail gaps above.
