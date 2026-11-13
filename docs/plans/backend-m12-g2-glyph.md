# Shared calendar-clock glyph

[Decision Log]
- Purpose: the schedule empty states and the sidebar entry need the same
  calendar-with-clock mark that the reference uses.
- Existing assets/constraint: the kit has no calendar or clock. Line glyphs
  already share a 24-unit view box, 1.5-unit stroke, round caps and joins.
- Alternatives: copy an external SVG, draw a screen-local mark, or substitute
  an unrelated kit glyph. Those would break asset ownership or the reference.
- Choice: `Glyph::CalendarClock`, an original calendar/clock composition,
  through the existing `line_glyph!` macro, registered in the shared inventory.
- Why: one monochrome mask serves both themes and both consumers; no drawing
  or third-party asset is copied into the app.
- Cost: one vector and inventory entry. No dependency or consumer is changed
  in this commit. The line drawings intentionally share one registry file so
  registration and geometry can be reviewed together.

The asset test loads the registered path through `AssetSource`, checks its
24-unit view box, renders it at 24 and proves it is painted with clear outer
edges. Comparison captures use the actual native app, at 24 in both themes,
with the existing Folder at the left and Server at the right. A local,
uncommitted drawing patch provides that comparison and is removed before the
full native gate. The consumer's reference size is separate: Paseo's
`ICON_SIZE.lg` is 20 for empty states, not the requested 24-point comparison.

The inventory grows from 54 to 55. `spikes/native/README.md` lists the asset.
The sidebar consumer belongs to the schedules slot the lead supplies; the
empty-state consumer belongs to the list slice. Neither is included here.
