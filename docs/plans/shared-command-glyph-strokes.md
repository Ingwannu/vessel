# Command centre glyph strokes (TRACKER 778)

Kit leaf on package base 14.16 (visual lane, 2026-11-16).

[Decision Log]
- Purpose: Paseo's command centre draws its root actions heavier than every
  other icon: `PlusIcon` at `strokeWidth` 2.4 and Add project, Settings,
  History, Schedules, Keyboard, Home, Import, Folder, CircleDashed and
  PanelLeft at 2.2 (`command-center/root-registration.tsx` 65-107), against
  the `LucideProvider` default 1.5. In Vessel only the Import row was heavy
  (its own 2.2 role); the others used the shared 1.5 glyphs.
- Existing assets: the glyph inventory (one SVG per role, append-only), the
  `Import` precedent (a command-only role at 2.2), and the shared `Plus`,
  `Gear`, `Keyboard`, `PanelLeft`, `CalendarClock`, `Folder` roles at 1.5 used
  by the sidebar, settings and toolbars.
- Alternatives: re-stroke the shared roles (every other surface would turn
  heavy); a stroke parameter on `Glyph` or `CommandRow` (a new field breaks
  struct literals across the app; asset paths would need a derived-SVG
  loader); a weight in the command row by policy (Paseo's other command rows,
  such as layout actions, stay at 1.5).
- Choice: ten command roles appended after `Scan`, canonical Lucide 1.50.0
  children (from `lucide-react-native` 1.50.0, the version Paseo pins) drawn
  by `stroked_glyph!(width, body)`: `CommandPlus` (2.4), `CommandAddProject`,
  `CommandSettings`, `CommandHistory` (Lucide `History`), `CommandSchedules`,
  `CommandKeyboard`, `CommandHome` (Lucide `House`), `CommandFolder`,
  `CommandCircleDashed`, `CommandPanelLeft` (2.2). Callers keep passing a
  `Glyph`; the command rows choose the command role.
- Consumers here: the palette's New session row (`CommandPlus`) and its
  Settings row for General (`CommandSettings`). The W22 command centre's own
  rows (Show keyboard shortcuts, Toggle the left sidebar) take
  `CommandKeyboard` and `CommandPanelLeft` when W22 rebases on this leaf; its
  Import row already uses the 2.2 `Import`.
- Impact: ten more assets (each about 300 bytes); the shared roles and every
  other surface are unchanged. The existing Lucide notice covers the paths.

Tests: `assets::command_role_tests` (one asset each under `kit/command-`, the
24 grid, every stroke at the role's width and unfilled, the stroked-vector
count of each shape; CommandPlus inks more than the shared Plus; ink at the
16-point row size over fractional scales) and
`areas::palette::rows::stroke_tests` (New and Settings rows use the command
roles; other rows keep theirs).

Results (2026-11-16, build VM; nothing compiled locally): whole kit suite unfiltered
command-strokes-kitall-20261116T090353Z-032880b0 312/0 (the three command role tests);
app palette tests command-strokes-narrow-20261116T090354Z-032880b0 1/0 (rows stroke
test); strict command-strokes-20261116T085846Z-a51a41fb all 0 (032880b0 changes a test
count only). The 14.16 app cannot be exported against the approved protocol-25 shared
fleet, so the captures came from a capture-only cherry-pick of these three commits onto
14.14 (scratch/m12-command-strokes-capture, never for adoption).
