# Menu rows named for assistive technology (TRACKER 808)

Visual lane, 2026-11-18. Built on TRACKER 807's head (`e4c2b65ac`, package
base 14.60 merged; 807 has its FINAL PASS and touches the same picker).
Filed by the coordinator from the app reviewer's note on 792 (the form
page's Create row), with the Files sort trigger added. Reference: pinned
Paseo 0.15.4 (`1c00b365c`): `components/ui/menu/menu-item.tsx` (the row's
`Pressable`: `accessibilityRole="menuitem"`, `accessibilityState` with
`checked` only for a row that draws a check, `aria-checked`, `disabled`),
`components/file-explorer-pane.tsx` 1236 (the sort trigger: a `Pressable`
whose name is its visible word).

Status: plan approved by the coordinator (2026-11-18) with a ruling on
list rows (below); built and gated on 14.61; frames with the coordinator.

## Paseo

- Every menu row is a `menuitem`; a row that draws a check also says
  `checked`; a disabled row is disabled; its name is its text (the label,
  then a hint or value it shows).
- The Files sort trigger's name is the order it shows ("Name", "Modified",
  "Size").

## Vessel before

The kit menu's rows (`menu/draw.rs` `row`, `menu/sub_row.rs`, the form
page's action row) and the combobox's rows (`picker/rows.rs`) draw no role
and no name; GPUI's accessibility tree has no node for them. The sort
trigger has none either.

## What GPUI 0.3.10 can say

- Roles (`.role`, any accesskit role: `MenuItem`, `MenuItemCheckBox`,
  `MenuItemRadio`), a name (`aria_label`), a checked state (`aria_toggled`),
  expanded (`aria_expanded`), shortcuts (`aria_keyshortcuts`), and the
  actions an element takes (a click handler gives `Click`).
- No setter for disabled or has-popup on an element (the accesskit node
  has them, but `Div::write_a11y_info` writes only the aria fields above),
  and its debug tree has no field for them either.

## Plan

[Decision Log]
- Purpose and intent: every menu row named and typed for assistive
  technology, as Paseo's; the sort trigger named.
- Existing implementation and constraints: one kit renderer draws every
  menu's rows; GPUI's element API (above); tests read GPUI's real tree
  (`a11y_snapshot`, as TRACKER 795's).
- Alternatives considered: a custom element wrapping each row to write
  disabled and has-popup into its accesskit node (a second layout and paint
  path for every row, unseen by the debug tree, so untestable here); a GPUI
  fork (not ours to change).
- Chosen approach:
  - kit menu rows (ordinary, sub, form action): `Role::MenuItem`; name = the
    words shown (the pending words while pending) and the row's hint or
    value after them ("Copy agent id 16aa40f", "Grouping Project").
  - disabled and pending rows: no `Click` (they already take no press), the
    tree's way of saying a node cannot be pressed.
  - sub rows (a page): `aria_expanded(open)`, the nearest GPUI has to
    has-popup (ARIA's submenu trigger carries both).
  - the Files sort trigger: `Role::Button`, named by its word.
- Why: everything said goes through GPUI's own tree, and every part is
  testable there.
- Pros, cons and impact: disabled and has-popup are not said as states;
  named as differences.

Question for the coordinator: kit menu rows have no checked state; only
the combobox's rows draw a check (the Labels list, model and mode lists).
(a) Leave the combobox for a later item, or (b) include it here: a
combobox row `MenuItemRadio` with `aria_toggled` in a list of one choice,
`MenuItemCheckBox` in a list of many (a header flag the Labels list sets),
named as the menu rows.

## Tests

- kit: a menu's rows in GPUI's tree are menu items with their words and
  hint as names; a disabled and a pending row offer no Click, an enabled
  one does; a sub row says expanded false, then true when its page opens.
- app: the sort trigger is a button named by its order.

## Ruling (coordinator, 2026-11-18)

The disabled and has-popup gap is accepted as a stated difference; the
custom element rejected. List rows as Paseo builds them:

- Rows Paseo draws as a menu with a check (the Labels list,
  `workspace-labels/picker.tsx`): `Role::MenuItem` with toggled; rows there
  without a check carry no checked state.
- Rows of lists Paseo builds as a Combobox (model, mode lists:
  `ComboboxItem`, `combobox.tsx` 298): `Role::Button` named by its words,
  no toggled.
- No `MenuItemRadio` or `MenuItemCheckBox` (Paseo uses neither).

## As built

- kit `menu/a11y.rs`: `row_body` names every row (ordinary, page, the form
  page's action) `Role::MenuItem` with its words and hint; `sub_row` adds
  `aria_expanded(open)`. Disabled and pending rows already take no click,
  so they offer no Click.
- kit `picker/rows.rs`: `RowRole` (`Button` default, `MenuItem`,
  `CheckedItem`) on `ComboRow::role`; a row is named by its label and its
  detail; `CheckedItem` says `Toggled::True`/`False` from `checked`.
  `initial_tile`/`initial_of` moved to `picker/initial.rs` (re-exported) so
  `picker.rs` shrinks (676 → 652 recorded).
- kit `a11y_snapshot`: `nodes(window, role)` with name, toggled, expanded
  and actions, for tests.
- app: the Labels list's label rows `CheckedItem`, Create label `MenuItem`;
  the Files sort trigger `Role::Button` named by its word.

## Differences

- Disabled and has-popup are not said as states (GPUI 0.3.10 has no setter
  and its tree has no field); a row that cannot be chosen offers no Click,
  and a page's row says expanded.
- A row's shortcut chords are not said (`aria_keyshortcuts` wants ARIA's
  key names, the kit draws its own chord words).
