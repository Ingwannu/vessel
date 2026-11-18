# Compact menus as Paseo's sheets (TRACKER 798)

Visual lane, 2026-11-18. Built on TRACKER 800's branch (head aba008370,
package base 14.42), with the kit menu's sheet form from TRACKER 791.
Reference: pinned Paseo 0.15.4 (`1c00b365c`):
`screens/workspace/workspace-header-menu.tsx` 256-279,
`screens/workspace/workspace-tab-trailing-accessory.tsx` 88-130,
`screens/workspace/compact-workspace-header/tab-switcher.tsx`,
`components/ui/combobox.tsx` (`bottomSheetHeader`, `comboboxTitle`).

Status: visual PASS (coordinator); app reviewer round 1 returned R1 (the
600-768 band), fixed at the root below; round 2 with the app reviewer.

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

## One compact width (app reviewer R1; the coordinator's ruling)

The app reviewer found that between 600 and 768 the wide bar and strip
were drawn while the two titled menus came up as sheets: the kit's phone
width (`kit::breakpoint::PHONE`) was 768 and the layout's Compact 600.
Paseo has one compact width, below `md` (600; `constants/layout.ts`
`isCompactBreakpoint`, `styles/unistyles.ts`), and every sheet form
switches there.

[Decision Log]
- Purpose and intent: one width for the phone form, as Paseo.
- Existing implementation and constraints: `PHONE` 768 (the page's
  sidebar fold) drove about sixty places: dialogs as sheets, titled menus
  as sheets, touch sizes in the conversation bar and elsewhere, the title
  bar's icons, the Settings sidebar and list form, the code panel's full
  screen form, some densities. The layout's Compact, the sidebar's sheet
  (`SIDEBAR_COMPACT`) and the pickers (`picker::COMPACT`) were 600.
- Alternatives considered: a Compact condition on the two sheet titles
  (the app reviewer's fix; the band stays wrong for every other sheet).
- Chosen approach (the coordinator): `PHONE` is 600; `picker::COMPACT` is
  `PHONE`; the view's `SIDEBAR_COMPACT` is its `COMPACT_LAYOUT_WIDTH`; the
  app asserts at compile time that `PHONE` equals `COMPACT_LAYOUT_WIDTH`
  (the view cannot depend on the kit). Settings' own sheet branch in
  `sidebar::arrangement` became the sidebar's rule and went.
- Pros, cons and impact: from 600 to 767 dialogs are centred cards, menus
  popovers, controls pointer-sized, the title bar has its wide statuses,
  Settings docks its list (below its 720 minimum until E2's frame) and
  loads its sections at once, the code panel is beside or over the
  conversation and its file viewer editable. A touch window in that band
  gets pointer-sized targets, as Paseo's. The browser build's page still
  folds its sidebar at 768.
- Tests that assumed 768: Settings' sidebar form (now a sheet at 590, docked
  at 650 and 800) and the shortcuts page at 700 (not the phone form).

## Follow-up at 600 (branch backend/m12-width-followups)

The coordinator asked for frames at 600, the narrowest desktop width now,
of Settings and of a session with a permission card, and for anything that
overflows there to be fixed; the app reviewer and the coordinator added the
message search card.

- The sidebar's foot at its least width (200): Labels was squeezed to one
  letter a line. Labels and the gear now keep their size, and the build
  with its "·" shows whole or not at all (the coordinator: a lone "…" read
  as a glitch), so at 600 the foot reads "reload  Labels  ⚙" and at 700 it
  is whole. Done by layout alone (a reversed, wrapping row one line tall
  that clips; decision log in `sidebar::foot`). Tests: `sidebar::policy_tests`
  `at_the_least_width_the_foot_keeps_labels_on_one_line`,
  `at_the_least_width_the_build_and_its_dot_go_and_reload_leads`,
  `with_room_the_build_and_its_dot_come_before_reload`.
- The message search card (`conversation::search`, shown with
  `VESSEL_APP_MESSAGE_SEARCH`) was a fixed 640 and ran past any narrower
  column (before 798 too, above 768). It is now as wide as the column less
  the conversation content's 16 inset, up to 640 (360 on a phone).
- Settings at 600 and the permission card at 600 needed nothing (the card's
  520 is a maximum height).

## Tests

- kit `dialog::header_title_only_tests`.
- app `layout::compact::tests`: the switcher has its title and no close
  button; a row's "…" and the Workspace actions open as sheets as wide as
  the window; at 700 Workspace actions and a strip tab's menu are
  popovers.

## Status

Gates on 836f8d079 (build VM, ordinary lane): strict (fonts, fmt, quality,
clippy) 0; app 326 passed, 0 failed, 2 ignored; kit 378. Frames at 390 on
the base14.41 fleet (export of 836f8d079), dark English and light Korean:
the switcher's title-only sheet, a row's menu sheet titled with the tab's
label, and the Workspace actions sheet.
