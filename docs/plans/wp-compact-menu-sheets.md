# Compact menus as Paseo's sheets (TRACKER 798)

Visual lane, 2026-11-18. Built on TRACKER 800's branch (head aba008370,
package base 14.42), with the kit menu's sheet form from TRACKER 791.
Reference: pinned Paseo 0.15.4 (`1c00b365c`):
`screens/workspace/workspace-header-menu.tsx` 256-279,
`screens/workspace/workspace-tab-trailing-accessory.tsx` 88-130,
`screens/workspace/compact-workspace-header/tab-switcher.tsx`,
`components/ui/combobox.tsx` (`bottomSheetHeader`, `comboboxTitle`).

Status: built; gates under "Status".

## Paseo on a phone

- Workspace actions (`WorkspaceHeaderMenuMobile`): `DropdownMenu
  compactMode="sheet"` with `sheetTitle` "Workspace actions": a sheet from
  the bottom with the handle and the title.
- A switcher row's "…" (`MobileTabTrailingAccessory`): the same sheet,
  titled with the tab's label (`sheetTitle={presentationLabel}`).
- The tab switcher (`MobileWorkspaceTabSwitcher`): a `Combobox` with a
  `title` and no `header`, so its sheet draws the title alone under the
  handle: 24 across, 8 under, base size, medium, no close button and no
  rule.

## Vessel before

The 789 menus were popovers at every width; the switcher was the kit
dialog's sheet with its title row, rule and close button.

## What was built

[Decision Log]
- Purpose and intent: the compact header's menus as Paseo's sheets.
- Existing implementation and constraints: the kit menu takes a sheet title
  (`Content::with_sheet`, 791) and comes up as a sheet below the phone
  width; the kit dialog always drew its close button.
- Alternatives considered: draw the switcher's title row in the app (a
  second header for one sheet); the kit combobox (it has neither the row's
  trailing "…" nor a footer, as 789 found).
- Chosen approach: the two menus pass their sheet titles; the kit dialog
  gains Paseo's combobox sheet header (`DialogInput::with_title_only`),
  which the switcher uses.
- Why: one menu renderer and one dialog renderer keep focus, Escape and the
  scrim; only the header's form is new.
- Pros, cons and impact: the title-only form draws the title alone, so a
  back, actions, search or subtitle given with it are not drawn. On a wide
  window the menus stay popovers (the sheet title applies on a phone only).

- Kit: `dialog/header_title_only.rs` (the row) and
  `DialogInput::with_title_only`; a test that it has no close button, the
  title 24 across, and that Escape still closes.
- App: `layout::workspace_menu` titles its menu "Workspace actions";
  `layout::menu` titles the tab menu with the tab's label
  (`draw::tab_label`); `layout::compact::switcher` uses the title-only
  header.

## Tests

- kit `dialog::header_title_only_tests`.
- app `layout::compact::tests`: the switcher has its title and no close
  button; a row's "…" and the Workspace actions open as sheets as wide as
  the window.

## Status

Built; gates pending.
