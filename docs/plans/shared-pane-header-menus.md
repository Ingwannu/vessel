# Pane header menus (TRACKER 776)

Visual lane, 2026-11-16, on package base 14.22. Reference: pinned Paseo 0.15.4
(`1c00b365c`): `screens/workspace/workspace-desktop-tabs-row.tsx` 225-415,
`screens/workspace/workspace-new-tab-menu.tsx`, `workspace-tabs/launcher/`,
`components/ui/pane-content-toolbar.tsx` 100-225, `components/ui/menu/menu-item.tsx`,
`stores/workspace-layout-actions.ts` 335-349, 800-841, 1530-1552, 2040-2062.

## What changed

1. The pane header's two split buttons became Paseo's one "…" (`More`, label
   "More actions" / "작업 더 보기"). It opens a menu below it, its end at the
   button's end, 4 down, 220 wide, with "Split pane right" (`Columns2`) and
   "Split pane down" (`Rows2`), each with its bound chord at the row's end.
2. Order: + | maximise/restore | … (maximise only with two or more panes).
3. Toolbar tooltips show the bound chord after their words (the New tab
   button: `NewTab`), as Paseo's `ToolbarButton shortcut`.
4. "+" opens the new-tab menu: the view's launch `catalog` in the pane order
   (Agent, Terminal, Changes, Diff, Files, Browser, Pull request), below "+",
   its start at the button's, 4 down, at least 200 wide, each row with
   Paseo's icon (`SquarePen`, `SquareTerminal`, `FileDiff`, `Files`, `Globe`,
   `GitPullRequest`).
5. An emptied pane is removed as Paseo removes it, so the empty pane's X is
   gone (see "Empty panes").
6. A disabled toolbar action takes no hover or pressed fill (item 7, the
   app reviewer's finding on the viewport review), as the viewport profile.

## Kit (shared)

[Decision Log]
- Purpose: the header needs Paseo's menu rows with a trailing `<Shortcut>`,
  tooltips with a chord, and menus anchored under a trigger at either edge.
- Existing assets: `MenuItem` had no trailing slot; `Tooltips` drew words only;
  `MenuState` opened at the pointer or at a trigger's end; the command
  centre's `key_badges` already draws Paseo's `Shortcut` look.
- Choice: `MenuItem::with_shortcut(chords)` (drawn by `key_badges`,
  `marginLeft: auto`); `ToolbarButton::shortcut(chords)` through a keyed
  tooltip content (words, then badges, gap 8); `MenuState::open_start`;
  `Content::with_min_width`; five Lucide 1.50.0 glyph roles appended after
  the command roles. The Toolbar profile draws hover and pressed fills only
  for an enabled action.
- Alternatives: a second badge drawing for menus (two looks for one
  component); per-surface anchors (the menu would not follow its trigger).
- Impact: additive fields and builders; `MenuItem` struct literals use
  `..MenuItem::new`, so existing callers are unchanged.

## App

[Decision Log]
- Choice: both menus are the kit menu (`header::menu_overlay`), opened from the
  button's measured bounds (a probe beside the tooltip's, as touch layouts
  install no tooltip); each choice runs only while the layout it was opened for
  is current (`PaneAction`).
- Launch availability: only Agent starts from the header today. It opens the
  new-agent page, as the passive New tab's own button does, because a
  workspace draft tab cannot be created outside tests yet (W2's seed bridge).
  The others are listed disabled, as Paseo's `item.disabled`: Terminal (the
  terminal panel), Changes, Diff and Files (the explorer panels), Browser
  (row 509) and Pull request.
- Recorded against row 336 (W15, the new-tab launcher): Paseo's other two
  launcher groups, plugin panels and the "Terminal profiles" group with its
  accessory row, are not in this menu yet.
- Chords: only for actions Vessel performs (New tab, both splits). The launch
  shortcuts (`TabAgent`, `NewTerminal`, `TabChanges`, `TabFiles`,
  `TabBrowser`) are bound but not yet performed, so their chords are not shown
  until they are.
- Shared caption helper: `areas::shortcuts::caps` (moved from the palette's
  `keys_of`, which now calls it).

## Empty panes (view)

[Decision Log]
- Purpose: Paseo never shows an empty ordinary pane with a close X: a pane goes
  with its last tab (`detachTabFromTree`).
- Rules copied: closing keeps the main pane (`DEFAULT_PANE_ID`), Explorer and
  the last ordinary pane; moving keeps the source only when the tab stays in it
  (a split of its own pane) or crosses into or out of Explorer.
  `Layout::collapse_emptied` applies them in `close_confirmed` and `drop_tab`.
- Difference: Paseo puts a fresh New tab into a kept empty pane
  (`ensureRetainedPaneHasTab`); Vessel leaves it empty, as before.
- Focus mode: Paseo's X in this row is Exit focus mode. Vessel's focus mode is
  the maximised pane, whose way back is the selected Restore pane toggle (the
  W12 adoption), so no X is drawn.

## Not in this change

- The full tab rail (overflow, the inline "+" after the tabs when they fit,
  drag and hover close, the compact header island) stays with its own rows;
  "+" stays in the toolbar position.

## Tests

- Kit: `menu::shortcut_tests`, `menu::tests` (start placement, minimum width),
  `tooltip::tests::keyed_words_keep_the_body_face_and_show_chords_only_when_bound`,
  `icon_chrome::tests::a_disabled_toolbar_action_paints_the_same_under_hover_and_press`,
  `assets::launch_role_tests`.
- App: `areas::layout::header::tests` (order and chrome, "+" opens the new-tab
  menu, Agent opens the new-agent page, "…" splits the pane it was opened on,
  a stale choice is refused).
- View: `layout::tests::a_pane_goes_with_its_last_tab_but_main_and_the_last_pane_stay`,
  `layout::tests::moving_a_pane_s_last_tab_away_removes_the_pane`.

Gates: pending (the build VM was stopped by the Max host's memory guard on
2026-11-16 13:25 UTC; nothing compiled locally).
