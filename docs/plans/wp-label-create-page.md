# Create label as a page of the Labels menu (TRACKER 792)

Visual lane, 2026-11-18. Built on package base 14.51 (590471c12). impl F
confirmed 792 is free. Reference: pinned Paseo 0.15.4 (`1c00b365c`):
`workspace-labels/picker.tsx` (`useWorkspaceLabelMenuPages`,
`WorkspaceLabelPickerPage`, `WorkspaceLabelCreatePage`),
`components/ui/menu/menu-item.tsx` (`MenuTextField`, `menuRowContentInset`,
the `item` and `field` styles), `components/ui/menu/menu-sub.tsx`,
`menu-geometry.ts`.

Status: plan approved by the coordinator (2026-11-18), including the bare
`PickerPlace::Popover` taking the phone form; being built.

## Paseo

- A workspace row's menu has a Labels row that opens the Labels page; its
  last row (after a separator when there are labels) is "Create label", a
  `MenuSubTrigger` with a muted 14 Plus in the leading slot, disabled while
  the host is offline. It opens a second page, "Create label", with
  `hoverIntent: false`: only a press opens it, and while it is open hover
  closes nothing.
- On a wide window the page is a flyout beside the Labels flyout, at the
  Create label row; in a compact sheet it replaces the Labels page under a
  header titled "Create label" with the way back.
- The page, top to bottom:
  - `MenuTextField`: a row's box (min height 28, 4 across, 8 inside, 4 up
    and down, border 1 `border`, radius 6, fill surface2; border accent
    while focused); the text 12 on an 18 line, placeholder "Label name",
    focused when the page opens, not editable while offline or saving;
    Enter creates.
  - The ten colour swatches on the row text's rail (inset 4 + 1 + 8 = 13),
    8 above and below; the first colour chosen; picking one only picks.
  - A separator.
  - "Create": a menu row, disabled while offline or while the name is
    empty; while saving it reads "Creating…" with the spinner; it does not
    close the menu.
  - A failure shows as the menu's hint under it ("Unable to update label"
    or the host's words); the draft stays.
- Success: the label is created and assigned, and the page goes back to
  the Labels list (the menu stays open).

## Vessel before

The Labels list is the kit combobox in the row menu's flyout (TRACKER
791). Its "Create label" row closes the list and opens a separate kit
dialog (`labels::picker::create`) with the name field, the swatches, Back,
Create, and a close button; the row menu waits undrawn behind it
(`RowPicker::Dialog`, a difference recorded in 791). Success returns to
the list.

## Plan

[Decision Log]
- Purpose and intent: Create label as Paseo's press-only page inside the
  menu, replacing the separate dialog.
- Existing implementation and constraints: the kit submenu model already
  has press-only pages (`Submenus::press_only`, Paseo's `hoverIntent:
  false`), nested flyouts by path depth, and combobox rows that open a page
  beside the list (`ComboRow::page`, `ComboboxState::set_submenus`). The
  view's picker state (`PickerPage::Listing / Creating / Submitting`), the
  name field, the swatch renderer (`labels::colors`) and the create and
  answer flow (`labels_picker_submit`, `return_to_list`) stay as they are.
  The kit has no menu text field.
- Alternatives considered: keep the dialog (not Paseo); draw the form
  inside the Labels flyout under the list (Paseo replaces the list by a
  page, it does not extend it).
- Chosen approach:
  - Wide window: "Create label" becomes a `ComboRow` with
    `page: "labels-create"` (press-only) in the Labels flyout, sharing the
    row menu's `Submenus`; the row menu draws that page as a depth-1
    flyout at the row (`submenu::flyout`), after the Labels flyout.
  - Phone: the page replaces the Labels sheet: a kit sheet titled "Create
    label" whose header leads back to the list (the picker's own `Back`),
    with the same rows.
  - The page's rows: a kit `MenuTextField` (new, in the kit's menu module:
    the `TextField` in Paseo's field box), the existing swatch row on the
    13 rail, a separator, a kit menu row "Create" (pending words and
    spinner from `MenuItem`'s status), and the menu hint for a failure.
  - The create dialog and `RowPicker::Dialog` (the menu hiding behind it)
    go; the menu stays drawn with the page beside it.
- Why: one menu model draws both presentations, as 791 did for the list,
  and only the page's content is new.
- Pros, cons and impact: the row menu keeps its keys and its outside-press
  rule with one more page (`Submenus::contains` covers the flyout). The
  canonical `PickerPlace::Popover` (no menu around it) takes the phone
  form, the sheet with its way back, as Vessel has no flyout chain there.

Details to keep:
- The Plus mark in the Create label row's leading slot, muted, 14.
- Escape on the page: back to the list (the path's last page), as the
  sheet's back and Paseo's `goBack`.
- Words: "Label name", "Create", "Creating…" (Paseo's `workspaceLabels`),
  in en and ko where Vessel lacks them.

## Tests

- kit: the menu text field's box and rail (min height 28, text at the row
  text's inset), its focus border.
- app `labels::picker::tests`: Create label opens on a press and not on a
  resting pointer; the page's field is focused; Create is disabled with an
  empty name and enabled after typing; Enter creates; success returns to
  the list with the menu still open; a failure keeps the draft and shows
  the hint; on a phone the page is a sheet titled "Create label" whose
  back returns to the list.

## Frames

At 1280 (the flyout beside the Labels flyout: empty, typed, saving, a
failure) and at 390 (the sheet page), dark English and light Korean.
