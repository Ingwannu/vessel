# Shared action-menu leading content and progress

[Decision Log]
- Purpose: schedules, session and git actions need Paseo's leading glyphs,
  pending label/spinner, fixed width and a vertical kebab, once in the kit.
- Existing assets/constraint: reuse the current menu surface, keyboard focus,
  placement, palette, spinner and trigger-bounds probe. Legacy constructors
  and the seven-argument `menu` entry stay unchanged.
- Alternatives: draw a second per-screen menu, attach glyphs beside the kit
  menu, or change every existing caller. Each would drift or break callers.
- Choice: `MenuItem::with_leading`, typed `ItemStatus::Pending` with its label,
  `Content::with_width` at `menu_with`, a shared kebab trigger and `open_end`
  using actual bounds. Data, drawing and trigger live in small submodules;
  the public `menu::` paths remain the same.
- Why: pending replaces the leading glyph and idle words together and cannot
  be selected, including Enter after a previously highlighted item changes
  state. One row recipe supplies existing and extended menus alike.
- Cost: four original line glyphs and one shared touch-target constant; no
  dependencies, submenu or success-state API. App consumers are not in this
  control-only commit.

References read whole: Paseo `components/ui/dropdown-menu.tsx`,
`components/ui/menu/menu-item.tsx`, `menu-surface.tsx`, `menu-geometry.ts`, `components/ui/{control-geometry,control-sizing}.ts`, and
`components/schedules/schedule-row.tsx`. The reference's menu-navigation
fixtures cover submenus, which this control does not add. Selection tests
instead cover this extension's pending/disabled boundary and stale highlight.
No reference code or SVG was copied.

Values: normal leading glyph 14 in a 16-point slot; pending spinner 16,
muted even on a destructive action. Label line height 18, row border 1,
horizontal margin 4, padding 8 across/4 vertically, radius 6; row minimum 28
with a pointer and 44 with touch. Disabled/pending opacity is 50%, with no
hover/pressed fill or handler; enabled hover/pressed/highlight uses surface2.
Text is foreground (destructive on destructive actions); ordinary leading
content is muted. Full-width group separators and the existing menu surface
and shadow remain shared. Legacy width is min 180/max 320; explicit width
supports the schedule menu's 220. All sizes are logical points.

The kebab uses vertical dots at 14, radius 4 and padding 4. Its minimum is 20
for a pointer (visual frame 22), 44 for touch, from control-geometry.
The reference's hitSlop8 is now represented outside that visual/layout frame:
actual hit38/60 before parent clipping, with the same inner visual anchor.
Its hover fill is surface2 and ink foreground; idle is transparent/muted.
Only hover changes it, as the reference's trigger callback does. The existing
horizontal More glyph is retained. The original Pause, Play and RotateCw
vectors use the same 24-unit/1.5-stroke kit macro, and Pencil already exists.
The shared inventory grows from 55 to 59; rendering tests load the new paths and
check painted content and clear edges at 24.

`TriggerAnchor` reuses the existing `select::Anchor` type and `anchor_probe`.
`open_end` anchors the menu's upper-right corner below the trigger's lower
right, offset 4, with the existing 8-point window snap margin. `open(point)`
retains its former pointer placement. Fixed width 220 does not depend on
where within the hit area the pointer clicked.

Tests cover exact colours/opacity in both themes, geometry, legacy defaults,
label/glyph replacement, forward/back wrapping, no selectable items, and
Enter on an item that became pending or disabled after highlight. No focus
ring, new palette, screen-local notice, fixture route or data is introduced.

## Review correction: one-line labels and changing rows

[Decision Log]
- Purpose: preserve Paseo's row pitch for long idle and pending labels.
- Constraint: `menu-item.tsx` uses `numberOfLines={1}` and a content slot
  with flexShrink 1/minWidth 0; the initial native extension wrapped words.
- Alternative: a taller wrapping menu. That changes the reference layout.
- Choice: the same shrinking, single-line slot, with the existing picker's
  `one_line` normalization for line breaks and repeated whitespace.
- Why: a menu is one of the reference's explicit single-line surfaces;
  long screen titles can still wrap outside it.
- Cost: long menu labels may ellipsize, as in the reference. Runtime long
  English/Korean idle/pending captures remain part of first-use review.

A menu can also shrink while open. An out-of-range keyboard highlight now
restarts forward/back navigation from the corresponding end; Enter still
refuses every stale, pending or disabled index. Regression cases include
an index just beyond the list, 99 and usize::MAX in both directions.
The shared trigger bounds and placement renderer are unchanged here.


## Runtime correction: explicit vector ink

The first native probe revealed invisible trigger vectors even though the
bounds, click handling and menu worked. GPUI 0.3.10's `Svg::paint` requires
its own text colour; it does not inherit the parent's colour. Both profiles
now set ink on the SVG itself. Schedule alone uses the nearest ancestor's
hover group to change its vector to foreground; GitHeader stays muted.
A regression checks the actual SVG style, not merely a palette helper, in
both themes, profiles, densities and enabled/disabled states. The original
probe captures remain excluded evidence; corrected captures must be rebuilt.


## Schedule hit allowance correction

The initial source-only trigger omitted schedule-row.tsx's hitSlop 8.
"Schedule unchanged" meant preserving its visual/hover geometry while
adding GitHeader, not retaining this omission. The Schedule frame remains
22/44, glyph14, radius4 and padding4; its expanded input region is38/60.
Negative margins retain the layout footprint, the inner bounds remain the
end anchor and the hover wash stays on the inner frame. Enabled triggers
stop mouse-down propagation so a containing row does not also activate;
disabled triggers install no handler. Parent clipping/adjacent hit priority
and native inside/outside/anchor/paint behaviour require explicit probes.
Earlier22/44 descriptions were visual sizes, not verified hit sizes; no
Schedule trigger visual approval had been given before this correction.

The public chose callback now documents that the per-item dismissal policy
has been applied. KeepOpen is allowed; closing is no longer promised.
