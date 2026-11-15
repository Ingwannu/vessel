# Diagnostics' shared glyph

## Decision Log

- Purpose: settings diagnostics needs the same recognizable tool in every consumer.
- Constraint: no Stethoscope asset existed; diagnostic actions and selection remain E2's.
- Alternatives: substitute an unrelated existing mark, or draw another area-owned vector.
- Choice: one original Stethoscope drawing on the kit's24 grid with1.5 stroke; inventory70 and one registered asset path.
- Why: Paseo settings-screen.tsx SIDEBAR_SECTION_ITEMS selects Stethoscope; SidebarSectionButton uses density md16/20 and stroke1.5. The source's asset is not copied.
- Impact: existing glyph paths/callers remain unchanged. E2 supplies selected foreground or ordinary muted ink.

Registration/render tests cover16/20/24, actual nonempty ink and clear edges. Native comparisons must show the glyph beside existing kit glyphs in both themes; actual Diagnostics navigation evidence belongs to E2's first use.
