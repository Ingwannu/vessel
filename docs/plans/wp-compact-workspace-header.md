# Compact workspace header (TRACKER 789)

Visual lane, 2026-11-17, on package base 14.28. Reference: pinned Paseo 0.15.4
(`1c00b365c`): `screens/workspace/workspace-screen.tsx` 3363-3440 (`isMobile`),
`screens/workspace/compact-workspace-header/{index,tab-switcher}.tsx`,
`screens/workspace/workspace-header-menu.tsx`, `components/headers/compact-header.tsx`,
`screens/workspace/workspace-tab-menu.ts`, `docs/menus.md` ("When the items
differ per form factor").

Status: in progress, on the coordinator's rulings below.

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
