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
