# A rule above Create label in the Labels list (TRACKER 807)

Visual lane, 2026-11-18. Built on package base 14.56 (`cb2b7e066`). Filed by
the coordinator from TRACKER 792's frames. Reference: pinned Paseo 0.15.4
(`1c00b365c`): `workspace-labels/picker.tsx` 154
(`{snapshot.rows.length > 0 ? <MenuSeparator /> : null}`),
`components/ui/menu/menu-item.tsx` (`MenuSeparator`, `separator` style:
height 1, `marginVertical: spacing[1]`, `borderAccent`).

Status: built; gates and frames follow.

## Paseo

The Labels page lists the labels, then a separator, then Create label; with
no label rows shown (none yet, or a search that matched none) there is no
separator.

## Vessel before

The Labels list is the kit combobox (TRACKER 791); a combobox row could
carry a heading but not a rule, so Create label ran straight on from the
last label.

## Plan and as built

[Decision Log]
- Purpose and intent: Paseo's separator between the labels and Create
  label.
- Existing implementation and constraints: the combobox draws its rows from
  `ComboRow`s and already draws a row's `heading` above it; `picker.rs` is
  at its recorded size and may not grow.
- Alternatives considered: a separator entry in the rows (every caller's
  row index and choice table would shift); the app drawing its own rule
  outside the list (it would not scroll with the rows).
- Chosen approach: `ComboRow::separated`: the list draws Paseo's rule (1
  high, 4 above and below, `borderAccent`, the list's full width) above the
  row when a row is shown before it, none above the first. The labels list
  sets it on Create label, so a list filtered to no labels has no rule, as
  Paseo's `rows.length > 0`. `anchor_probe` moved to `picker/probe.rs`
  (re-exported unchanged) so `picker.rs` shrinks (683 → 672 recorded).
- Pros, cons and impact: one field, every other list unchanged; rows gain
  a `combobox-row-{index}` selector and the rule `combobox-rule-{index}`
  for tests.

## Tests

- kit `picker::rows::separator_tests`: the rule between the row before and
  the separated row, 1 high with 4 above and below, as wide as the rows;
  none for a separated row shown first.
- app `sidebar::row_menu::tests::create_label_has_paseos_rule_above_it_while_labels_are_shown`:
  one rule above Create label in the opened list; none after a search that
  leaves no label.
