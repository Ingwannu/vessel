# One drag for the kit's bottom sheets (TRACKER 799)

Paseo's bottom sheets close on a downward pan (`enablePanDownToClose`:
`components/ui/menu/menu-surface.tsx` 416, `components/ui/combobox.tsx` 986,
`components/adaptive-modal-sheet.tsx` 750). Since 20d5380a6 the kit's menu
sheet and list sheet draw the dialog's handle but do not move; the dialog
sheet does (`sheet/snap`: stops, pointer slop, velocity, spring, the
content-scroll handoff).

## Decision (ADR-light)

[Decision Log]
- Purpose: one gesture for the three sheets: a pan down on the handle, or on
  the body while its list is at the top, moves the sheet and closes it on
  release past its stop (or fast enough), else it springs back; the dimmer
  fades with it.
- Existing assets and constraints: `sheet/snap` is the dialog's engine:
  `State` (positions as fractions of a height, two open stops, spring,
  projected-velocity release), `listener` (a press stays a click until the
  slop; content pans hand off to its scroll), and the dialog's wrapper
  `dialog/sheet.rs` (`SheetState`, `SheetFrame`, close on `Pan(Closed)`).
  The dialog sheet fills fixed fractions of the window; Paseo's menu sheet
  is content-sized (`enableDynamicSizing`) and so is the kit's list sheet.
- Alternatives:
  1. A second, small drag for content-sized sheets: two gestures to keep in
     step (slop, velocity, scroll handoff), what the coordinator asked to
     avoid.
  2. Content-sized stops as authored percentages: whole percents misplace a
     content-sized sheet by up to 1% of the window.
  3. The engine's positions measured against the sheet's own height with
     one open stop at 100% of it (Paseo's single dynamic snap point).
- Choice: 3. `SnapPoints::CONTENT` (both open stops at the full height: one
  open stop at position 0, closed at 1); the host passes the sheet's
  measured height instead of the window's, so `Geometry::top` is the
  downward offset and `backdrop` the dimmer's share. The dialog's wrapper
  moves to `sheet/snap/host.rs` and gains a content-sized entry that starts
  at rest (no slide-in, as today) and measures its panel each frame.
  `MenuState` and `ComboboxState` keep one each; their sheets put the
  listener on the handle (`Origin::Handle`) and on the body
  (`Origin::Content`, the body's own scroll handle), offset the panel by
  `top`, scale the scrim by `backdrop`, and close with their own
  close-and-restore on `Pan(Closed)`.
- Why: one engine, one pointer adapter and one release rule for all three;
  the only new input is which height the positions are measured against.
- Costs and limits: the first frame after opening is unmeasured and cannot
  be dragged until drawn; the menu sheet's rows are not a scroll list, so
  its body hands off nothing. A row press that moves past the slop becomes
  a pan, as in the dialog.

## Packet

- K: `SnapPoints::CONTENT`, `sheet/snap/host.rs` (from `dialog/sheet.rs`),
  the dialog switched to it unchanged, the menu sheet and the list sheet
  adopting it. Tests: engine (content stops: release near and far, fast
  flick), menu sheet and list sheet (handle pan past the stop closes and
  restores focus; a short pan springs back and keeps it; a row tap still
  chooses), the existing dialog sheet suite unchanged. Captures to the
  coordinator (390, dark/light): a menu sheet and a list sheet mid-drag.
