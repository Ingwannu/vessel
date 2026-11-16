# Settings row with a selectable code value

Kit leaf on package base 14.13 for the pairing offer's confirmation card (the
coordinator's visual RETURN on G4 Offer 88ab, 2026-11-16). Additive: `code_row`
and its callers are unchanged.

[Decision Log]
- Purpose: Paseo's host confirmation (`hosts/host-confirmation-sheet.tsx` 69-90)
  marks every `DetailRow` value `selectable`: a machine's id, a key's
  fingerprint and each endpoint can be selected and copied. Vessel's
  `settings::code_row` draws its value as plain text that cannot be selected.
- Existing assets: `settings::code_row` (title 14 foreground, value 12 code face
  muted, 4 under it, the row 16 by 16) and `plain_text` (selectable literal text
  on the shared text engine, used by tool bodies).
- Alternatives: a copy button (not the reference); building the row in the
  pairing area from `row_with_content` (a second copy of the row's words
  layout); changing `code_row` (its callers would gain a window/context
  argument and a keyed engine view they do not need).
- Choice: `settings::selectable_code_row(id, title, value, first, tokens,
  window, cx)`: the same title and value line as `code_row`, the value drawn by
  `plain_text` under the caller's stable `id`. The row body is shared: `row_of`
  now builds its hint line and calls `row_of_parts`, which this row calls with
  its own value element.
- Impact: one engine view per value, kept by `id`; the value wraps inside the
  row (a long host may break at any character) and never widens the card.

Tests (`settings::selectable_tests`): the value starts 16 in, 41 below the
row's top (16 padding, the title's 21-point line, 4) and ends 16 from the
right; a long value in a 200-wide row wraps to more than one 20-point line
inside the row. The existing row geometry tests stay unchanged.

## Selection inside dialogs

The first captures showed no selection: a drag over the value in the pairing
dialog selected nothing. The window's text-selection layer is gpui-base's Root
child, painted under every deferred layer; its bubble-phase mouse-down listener
runs after the dialog card's, and the card called `stop_propagation` on every
left press (so a press inside it would not reach the scrim's close). No text in
any kit dialog could be selected.

[Decision Log]
- Choice: the card occludes the scrim's hitbox (`occlude()`) in place of
  stopping the press. The scrim's close is hitbox-gated, so a press inside the
  card still never closes; the window's raw selection listener now sees it.
- Alternatives: a flag the card sets and the scrim reads (two places to keep in
  step); a selection layer inside each dialog (a second layer per window).
- Impact: element listeners under a dialog stay behind the scrim's own
  occlusion, as before. Window-level raw bubble listeners (the selection layer)
  now see presses on a dialog card; capture-phase `on_mouse_down_out` was never
  affected by the old stop. Controls that own their press (fields) suppress a
  selection start themselves.

Test (`dialog::selection_tests`): with the selection layer under a dialog, a
drag over text in the card selects it, and the dialog does not close. The
existing dismissal tests (scrim, Escape, close glyph) stay unchanged.

Results (2026-11-16, build VM; nothing compiled locally): strict
selectable-row-20261116T084823Z-295b1b58 (fonts, fmt, quality, whole native clippy 0);
tests selectable-row-narrow-20261116T083525Z-f7ea9e45 (`-p vessel-app-kit --lib --
settings:: plain_text dialog::`) 26/0. In the pairing offer's export the drag selects a
machine id at 1280 and an endpoint on the 390 sheet.
