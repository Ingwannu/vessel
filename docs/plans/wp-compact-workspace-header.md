# Compact workspace header (TRACKER 789)

Visual lane, 2026-11-17, on package base 14.28; base 14.32 (with TRACKER
777's import sheet) merged 2026-11-18. Reference: pinned Paseo 0.15.4
(`1c00b365c`): `screens/workspace/workspace-screen.tsx` 3363-3440 (`isMobile`),
`screens/workspace/compact-workspace-header/{index,tab-switcher}.tsx`,
`screens/workspace/workspace-header-menu.tsx`, `components/headers/compact-header.tsx`,
`screens/workspace/workspace-tab-menu.ts`, `docs/menus.md` ("When the items
differ per form factor").

Status: built on the coordinator's rulings below; gates and captures under
"Status" at the end.

## Paseo at compact width

- The tab strip is not drawn. The workspace's header row is Paseo's compact
  header: the sidebar menu island, the title and a line under it, then an
  actions island.
- The actions island holds three controls:
  - The tab switcher: a 16 count badge in the large icon chrome
    ("Switch tabs (n open)"). It opens a non-searchable list titled
    "Workspace tabs". Each row is a tab with its trailing tab menu, and the
    current tab is checked. The footer has two secondary buttons, side by
    side: "New agent" (`SquarePen`) and "New terminal" (`SquareTerminal`).
  - "…" (`WorkspaceHeaderMenuMobile`): a bottom sheet titled
    "Workspace actions". It lists the rarer launches (New browser tab),
    then the workspace actions shared with the wide menu (Copy workspace
    path, Copy branch name, Import session, Show setup). Below those come
    the "Terminal profiles" group and "Edit profiles". Scripts and plugin
    rows also live here, since the header has no room for their own
    buttons.
  - The explorer toggle.
- The per-tab menu (`workspace-tab-menu.ts`): Copy resume command, Copy
  agent id, Copy terminal id, Copy file path, Reload agent, Rename, Close,
  Close tabs above/below, Close other tabs.

## Vessel at 390 today

From the top: the titlebar (sidebar menu, search, brand, window controls).
Under it is the pane's tab head: the tabs and "+", with "…" for the splits.
Under that is the session bar: the title, its state, "machine · agent", the
panel toggle and the session's "…" (marks, Copy session id, Copy link,
Close, Agent log, Kill). Then the content.

## Proposal

1. In `LayoutMode::Compact`, no pane tab head and no session bar. One
   compact workspace header row replaces both, under the titlebar. The
   titlebar keeps the sidebar menu, so the header has no menu island of its
   own.
   - Left: the session's title (one line), with the line Vessel's bar
     shows today under it (state, "machine · agent").
   - Right: the tab switcher, "…" and the panel (explorer) toggle.
2. The tab switcher uses the kit combobox as a compact sheet, "Workspace
   tabs". Its rows are the layout's tabs, labelled as the tab head labels
   them, with the current tab checked. Its footer has "New agent" (the
   new-agent page, as 776's "+ › Agent") and "New terminal" (disabled
   until a terminal tab can be made, as in 776's menu).
3. "…" is a compact sheet, "Workspace actions", built as its own menu, as
   docs/menus.md asks: one component per surface, with shared items in one
   function.
   - The launches the footer lacks, disabled where 776's menu disables
     them.
   - "Copy workspace path" (the session's folder).
   - "Import session": 777's sheet opened for this host and folder, so it
     reads "This workspace" with "Show all".
4. The session actions from today's bar "…" move to the tab switcher's row
   menu for the agent tab, as Paseo's tab menu. Paseo's equivalents are
   Copy agent id ≈ Copy session id, Rename and Close. Copy link, the marks,
   Agent log and Kill are Vessel's own, and would join that row menu.

## Rulings (coordinator, 2026-11-17)

- One compact workspace header in `LayoutMode::Compact`, laid out as Paseo's,
  replacing both the pane tab head and the session bar.
- Q1: the session actions move to each tab row's trailing menu in the
  switcher, as Paseo's tab menu (Copy agent id, Rename, Close…); Vessel's
  extras that Paseo's tab menu lacks (marks, Copy link, Agent log, Kill)
  follow Paseo's items in that same row menu. No second "…" in the header.
- Q2: Import session stays out of "…" for now. When 777 lands in a package
  base, rebase and add the row ("This workspace" + Show all) as the last step
  of 789, or as a small follow-up if 789 is already final.
- Q3: compact only. The wide "…" (`WorkspaceHeaderMenuDesktop`) is TRACKER
  795.
- Q4: the per-tab row menu comes with 789 (Q1 needs it).
- Titlebar (2026-11-18): keep it at every width. Vessel is a desktop app,
  and its titlebar carries the window controls and the drag region. Paseo's
  phone screen has no titlebar only because a phone has no window chrome;
  Paseo's desktop keeps its frame. So the header row sits under the titlebar,
  without its own menu island.

## What was built

### Kit

- `ToolbarButton::count` (`toolbar_button/count.rs`): Paseo's tab switcher
  trigger, the open-tab count in an outlined badge (at least 16 wide, 16
  tall, 2 across, radius 2, a 1 border of `foregroundMuted`; the number 10
  on a 12 line, medium, muted) inside the toolbar's own frame, so the
  hitbox, hover, press, disabled guard and tooltip are the toolbar's.
  Decision: one constructor beside `loading` rather than an app-drawn square
  beside the toolbar buttons (a second button style).
- `screen_header::Content` gains `subtitle` and `editing`. The kit's compact
  frame was Paseo's `CompactHeader` without a subtitle ("until a consumer
  needs it"). With a subtitle, the title is 14 (Paseo's `fontSize.base`)
  instead of 16, as Paseo's `singleLineTitle` is only for a title alone.
  `editing` puts the caller's field in the title's place while the title is
  renamed. The frame keeps the 56 row, 8 above, 12 across, no fill and no
  rule.

### App

- `layout::compact` draws the header in `LayoutMode::Compact` in place of
  the pane's tab head, through the kit frame: the session's heading as title
  and subtitle, then one island with the switcher trigger, "…" (Workspace
  actions) and the code panel toggle (Paseo's explorer toggle, "Open/Close
  Explorer sidebar"), all in the toolbar's touch chrome.
- `conversation::heading` gives the header the bar's words: the title as
  the bar shows it, and a line with the state badge or the released chip,
  "machine · agent", the lost line and a refused rename. These are the bar's
  own pieces, so they read the same. The session bar is not drawn in compact
  mode (`session::body`).
- The switcher (`compact/switcher.rs`) is the kit dialog as a fitting sheet
  (at most 90 % of the window), titled "Workspace tabs". Every tab is
  listed, pane by pane in strip order. A row: a 16 icon slot (the agent's
  mark, or the tab kind's glyph), the label 14 in the foreground, the
  current tab's muted check, and a 28 "…". A row's press shows that tab and
  closes the sheet. The footer has New agent (the new-agent page, as 776's
  "+ › Agent") and New terminal (disabled until a terminal tab can be made,
  as in 776's menu), side by side, 12 apart.
- A row's "…" opens the tab menu (`layout::menu`, surface `Switcher`) under
  it, over the sheet (`Layer::AboveDialog`), 220 wide. Its rows follow
  Paseo's order: Copy agent id (Copy ID on a terminal, Copy path on a
  file); Rename (the open session's, when it can be renamed: the header's
  title becomes the field, without the line under it while it is
  edited); a rule; Close tabs above, Close tabs below,
  Close other tabs; then Close. On the open session's agent tab, Close is
  the session's Close (a root archives first, as the bar's Close did).
  After a rule come Vessel's own actions that Paseo's tab menu lacks: the
  marks, Copy link, Agent log, Kill. Copies keep the sheet open; every other
  session action closes it first, so its confirm or log is on top.
- "…" (Workspace actions): Copy workspace path (the session's folder,
  disabled until the node describes it), then Import session. Import session
  opens 777's sheet for this host and folder, so it reads "This workspace"
  with "Show all".
- Words (en, ko from Paseo's own ko): Switch tabs ({count} open), Workspace
  tabs, New terminal, Workspace actions, Copy workspace path, Copy agent
  id, Close tabs above/below, Open menu for {label}, Open/Close Explorer
  sidebar.

### Differences kept

- Vessel's titlebar keeps the sidebar menu, so the header has no menu
  island of its own (the coordinator's titlebar ruling).
- Paseo's menus are bottom sheets on a phone (`compactMode="sheet"`, the
  row menu titled with the tab's label). The kit menu is a popover with
  touch rows, as the bar's "…" was. A sheet form of the kit menu would serve
  every compact menu; it is not in this item.
- The switcher is the kit dialog's sheet, which keeps its close button.
  Paseo's combobox sheet has a title only. The kit combobox has neither a
  row's trailing control nor a footer.
- Paseo's tab menu rows lead with icons. Vessel's tab and session menus have
  none, and three of those icons (arrow-left-to-line, arrow-right-to-line,
  copy-x) are not in the kit.
- The header's glyphs are the toolbar's 14, not Paseo's 16.
- Not listed in Workspace actions: New browser tab and terminal profiles
  (Paseo leaves them out where it cannot create them), Copy branch name
  (Vessel knows only the branch a session started on), Show setup (no page).
- Close tabs above/below act within the tab's pane (the layout's
  `CloseScope::Before`/`After`); the switcher lists every pane's tabs.

## Tests

- kit: `toolbar_button::count::tests` (the 16 badge centred in the 32 touch
  frame, one press; more digits widen it); `screen_header` (a subtitle steps
  the compact title from 16 to 14).
- app `layout::compact::tests`: one 64-tall header row as wide as the
  window, no pane controls, "…" 32 square at the row's middle; the switcher
  lists both tabs and a press shows the pressed one and closes it; New agent
  opens the new-agent page; the agent row's menu is Paseo's items then the
  session's, with the rules where Paseo has its separator, and the New tab's
  is the closes with Close last; a row's "…" opens the tab menu over the
  sheet; Workspace actions copies the folder once known; Import session
  opens the import sheet.
- app `layout::header::tests::compact_width_draws_no_pane_controls` replaces
  776's compact "+" test (the "+" is not drawn at compact width any more).

## Status
