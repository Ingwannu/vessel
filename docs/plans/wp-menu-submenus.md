# Menu submenus in the native kit (TRACKER 791)

Paseo's menus open a decision's page as a submenu: a flyout beside the row on
a wide window, the page replaced in place with a back header in a compact
sheet. The native kit has neither (`kit/src/menu.rs`: "no submenus"; the
Display menu replaces its root in place on every width; the row menu's
Labels item closes the menu and opens the picker on its own). This plan adds
the capability to the kit and adopts it in the two menus Paseo builds with it.

Reference (pinned Paseo): `docs/menus.md`; `components/ui/menu/`
`menu-navigation.ts`, `menu-surface.tsx` (`HOVER_OPEN_DELAY_MS` 90,
`HOVER_CLOSE_GRACE_MS` 260, `SUBMENU_OVERLAP` 5, `MenuFlyout`),
`menu-sub.tsx`; `components/ui/anchor.ts` (`computePosition`, `EDGE_PADDING`
8, no horizontal flip for submenus); adopters
`components/sidebar/sidebar-workspace-menu.tsx` (Labels, `compactMode="sheet"`)
and `components/sidebar/display-preferences/menu.tsx` (`compactMode="sheet"`).

## Decision (ADR-light)

[Decision Log]
- Purpose: a menu row that leads to a decision opens its page beside the
  menu, the menu staying open, as Paseo's do; the row reads as the answer
  ("Grouping  Project ›"). The workspace row menu's Labels entry and the
  sidebar's Display menu use it.
- Existing assets and constraints: one action-menu renderer (`menu.rs`,
  `menu/draw.rs`) with its render lease, layers, focus return and keyboard;
  one list (`picker.rs` `combobox`) that already draws a chevron row
  (`ComboRow::opens`), a back title row and a compact bottom sheet. GPUI
  paints deferred surfaces in a global order, and a press on a surface drawn
  outside a panel is "outside" that panel.
- Alternatives:
  1. Nest page elements inside the menu panel: the panel clips
     (`overflow_hidden`), the same reason Paseo declares pages as data and
     draws flyouts as siblings.
  2. `gpui-component`'s submenu: its own theme, actions and key contexts
     (the reason `menu.rs` does not use it).
  3. A flyout per screen: two copies of placement, hover intent and outside
     handling, the drift `docs/menus.md` warns about.
- Choice: one kit module, `submenu`, that both renderers use:
  - `MenuPath`, Paseo's navigation model, pure and unit-tested: open a page
    (truncating to the trigger's depth, so sliding across triggers swaps
    branches), close a depth, back, prune pages that vanished, current page.
  - `Submenus`, the state a screen keeps beside its menu: the path, each
    trigger row's measured bounds, and hover intent (open after 90 ms of
    rest, close 260 ms after leaving, a pointer inside the flyout cancels
    its pending close; a page marked press-only, Paseo's `hoverIntent:
    false`, opens only on press and stops hover closing while it is open).
    Timers carry a generation, so a superseded timer does nothing.
  - `flyout`: the page's surface at the trigger row's right edge, top on the
    row (side right, align start), overlapping it by 5, kept inside the
    window with an 8 margin on both axes, never flipped (Paseo keeps a
    submenu's side so it does not move out from under the pointer; the
    coordinator confirmed 2026-11-17). No backdrop of its own; a press
    inside it is not an outside press for the menu under it.
  - Rows: a sub-trigger row draws its current value (muted) and a 14-point
    chevron; while its page is open it has the hover fill (`active`) and no
    check (`selected` is the check alone). Pressing it opens the page and
    never closes the menu.
  - Compact (below 600, opted in per menu as Paseo's `compactMode="sheet"`):
    the menu is a bottom sheet and a page replaces the list in place with a
    back header (the list's existing sheet and back row); Back pops one page,
    Close ends the menu.
- Why: Paseo's one model drives both presentations (the flyout chain and the
  sheet's stack are the same path), so the kit keeps one path and two
  drawings, and each screen keeps only what its rows do.
- Costs and limits: a screen with submenus keeps a `Submenus` beside its
  menu or list state. Keyboard: Enter or Space on a sub-trigger opens its
  page as a press does (Paseo has no arrow-key submenu navigation either).
  Labels' Create stays the existing dialog in the first adoption (Paseo
  shows it as a press-only page inside the flyout); a named difference
  until the form moves into a page.

## Packets

- K1 kit: `submenu` (path, hover intent, flyout placement, outside-press
  containment), sub-trigger rows for the action menu and the list, an
  inline list panel a flyout can host, and the action menu's compact sheet
  (opt-in). Tests: path rules (unit); hover open at 90 ms, close at 260 ms,
  cancel on entering the flyout, parent stays open, a press in the flyout
  keeps the menu, geometry clamped at the window's right and bottom, compact
  page in place and Back (GPUI). The unfiltered kit suite runs on it.
- K2 Display menu: its pages as flyouts on a wide window; compact keeps the
  sheet with pages in place.
- K3 row menu Labels: the picker list as the Labels flyout on a wide window
  (the row menu staying open, its Labels row active); compact: the row menu
  as a sheet, Labels replacing it in place with a back header.
- Captures to the coordinator (1280/390, dark/light, EN/KO): both menus with
  a flyout open, a flyout clamped at the window's edge, the compact pages.
