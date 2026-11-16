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

## Every command-centre row at 2.2 (coordinator addition, 2026-11-16)

[Decision Log]
- Purpose: in Paseo every glyph in the command-centre list is drawn at 2.2
  (its Plus at 2.4). Vessel's list has rows Paseo's does not (Add a machine
  and each machine, the board, the theme rows, session rows, every settings
  section); after the first part they were still at the provider's 1.5. The
  coordinator asked for every row to match.
- Alternatives: a `Command*` role per remaining glyph (about two dozen more
  copies of shared paths, which could drift from the shared roles and grow the
  inventory each time a row is added); restroking the shared roles (every
  other surface would turn heavy).
- Choice: a derived asset. Each glyph drawn by `line_glyph!` (root stroke 1.5)
  is also served under `kit/command-weight/<file>`, the same SVG with its root
  stroke replaced by 2.2 (`Assets::load`), and `Glyph::command_weight_path`
  names it. The kit's command row (`command/row.rs`, `command_glyph`) draws
  its leading glyph and the chosen row's check (the theme rows) from that
  path, so every icon in a command list is at 2.2. The `Command*` roles and the
  drawings that are not 1.5 line glyphs (window controls, activity, the mark,
  `Import`) keep their own path: they are already at their stroke.
- Impact: no new inventory entries; the shared list (`Assets::list`) is
  unchanged; one string replacement per row glyph when the asset is first
  loaded (GPUI caches rasterised SVGs). The command row now uses `svg()`
  directly instead of `picker::glyph` (same size and colour). The W22 rows'
  `CommandKeyboard`/`CommandPanelLeft` remain correct; without them the
  derived 1.5 → 2.2 path gives the same weight.

Tests: `assets::command_weight_tests` (each line glyph's derived asset is the
shared SVG with only the root stroke at 2.2: the same vectors, each one drawn
at 1.5 now at 2.2, and a child's own stroke kept, as the spinner's arc at 2;
the Vessel-only row glyphs draw every vector at 2.2; other drawings and the `Command*` roles keep their path and have no
derived asset; the Vessel-only rows are derived; the shared list is unchanged)
and `areas::palette::rows::stroke_tests::every_other_row_draws_at_the_command_weight`
(every settings section, theme, machine, board and session row).

Results of the addition (2026-11-16, build VM; nothing compiled locally): whole kit
suite unfiltered cmd-weight-kitall3-08bd87972 316/0; app palette tests
cmd-weight-palette-79a49c08a 2/0 (the app is unchanged since); strict
cmd-weight-strict5-d238b52e7 all 0 (d238b52e7 = 08bd87972 + test formatting). Captures
from the exact export of 08bd87972 against the shared base14.19 fleet (protocol 28):
no capture-only cherry-pick was needed.

Coordinator visual PASS (2026-11-16): every command-centre glyph matches Paseo (2.2,
Plus 2.4). The derived command-weight assets are approved instead of explicit `Command*`
copies: one source per path and no drift between a shared role and its command copy.
