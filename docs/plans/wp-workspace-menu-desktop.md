# Wide workspace "…" (TRACKER 795)

Visual lane, 2026-11-18. Built on TRACKER 789's branch (head e21f3d597) with
package base 14.35 (98bd3bd4) merged. Reference: pinned Paseo 0.15.4
(`1c00b365c`): `screens/workspace/workspace-header-menu.tsx` 58-176
(`WorkspaceHeaderWorkspaceActionItems`, `WorkspaceHeaderMenuDesktop`),
`screens/workspace/workspace-screen.tsx` 472-530 (`WorkspaceHeaderTitleBar`)
and 3814-3877 (its styles), `components/ui/icon-button-chrome.ts`,
`constants/layout.ts`, `components/ui/control-geometry.ts`.

Status: built; gates and captures under "Status".

## Paseo on a wide window

- The screen header's left side is the title bar: the workspace title and
  its project row side by side, then the workspace "…" 8 after them
  (`compactHeaderMenuCluster`, workspace-screen.tsx 488-530). The right side
  holds plugins, scripts, open-in-editor, git actions and the explorer
  toggle. The tab strip with its "+" sits under the header.
- The "…" is `WorkspaceHeaderMenuDesktop`: the large chrome
  (`iconButtonChromeStyle({ size: "large" })`): a 26 square on a wide window
  (`HEADER_CONTROL_HEIGHT`), 32 on a phone, radius md, with a 16
  extra-muted ellipsis. Its menu opens aligned to the trigger's start, 220
  wide.
- Its items are `WorkspaceHeaderWorkspaceActionItems`, the same function the
  phone menu renders, "from the same callbacks, so the two surfaces can't
  drift": Copy workspace path, Copy branch name (with a branch), Import
  session, then a rule and Show setup (with a setup). The desktop menu has no
  launches, terminal profiles or scripts: "Wide layouts make tabs from the
  tab strip's `+` menu, so this one carries workspace actions only."

The TRACKER row places it "beside the tab strip"; Paseo places it after the
header's title (workspace-screen.tsx 488-530), and so does this build.

## Vessel before

The wide pane draws its tab strip ("+", the split "…") and, under it, the
session bar: the title, the state, "machine · agent", the code panel toggle
and the session's "…" (marks, Copy session id, Copy link, Close, Agent log,
Kill). The session bar is the row that plays Paseo's title bar. The
workspace "…" existed only in the compact header (789).

## Rulings (coordinator, 2026-11-18)

- A, as Paseo: the workspace "…" goes right after the session bar's title
  group, 8 apart.
- Its frame is the large square chrome (26 pointer / 32 touch, radius md)
  with a 16 extra-muted ellipsis; the menu is aligned to the start, 220
  wide, with "Workspace actions" as tooltip and accessible name; the items
  come from the shared builder with 789's exclusions.
- The compact island stays large and round.
- Two "…" in the wide bar are accepted until TRACKER 797 moves the session
  actions into the tab menu; the session one is named "Session actions" so
  the two are never ambiguous.

## What was built

[Decision Log]
- Purpose and intent: Paseo's workspace-level "…" on a wide window, with the
  same items and callbacks as the compact one.
- Existing implementation and constraints: 789 drew the compact "…" and its
  menu inside `layout::compact` (state, items and handler of its own); the
  kit toolbar knew the small chrome and 789's header island only.
- Alternatives considered: a second menu for the bar (two item lists that
  could drift, which Paseo's comment warns of); the trigger in the pane's
  tab strip beside "+" (the TRACKER wording; not Paseo's place, and the strip
  already has the split "…").
- Chosen approach: `layout::workspace_menu`: one menu state, one item
  builder and one trigger, drawn in the compact island (`Header::Island`) and
  in the wide bar (`Header::Bar`); its menu is rendered once by the layout
  frame. The kit toolbar gains `ToolbarConfig::large` (26 pointer, 32 touch,
  16 glyph, square); `header_island` is now the large chrome made round.
- Why: one builder keeps the two surfaces from drifting, as Paseo's; the kit
  profile keeps the chrome in the kit (no area-drawn button).
- Pros, cons and impact: the wide bar has two "…" until 797, told apart by
  their names, place and ink (the workspace one extra-muted, after the
  title). The compact "…" now uses Paseo's extra-muted trigger ink
  (`WorkspaceHeaderMenuTriggerIcon`) through the shared trigger.

- Kit: `ToolbarConfig::large`; the frame's size and the glyph's come from one
  `Chrome` (small, large, island).
- App: `layout/workspace_menu.rs` (trigger, overlay, `items`,
  `workspace_path`); `layout::compact` uses it; `conversation::bar` draws the
  trigger after its detail line and names the session "…" "Session actions"
  (role button).
- Words (en, ko): Session actions (세션 작업).

## Differences kept

- No Copy branch name, Show setup, launches, terminal profiles or scripts,
  as in 789 (reasons there).
- Vessel's bar sits under the tab strip; Paseo's header sits above it.

## Tests

- kit `toolbar_button::tests`: the large chrome is 26 on a pointer and 32 on
  touch, square; the island is large and round; the default stays 20 / 14.
- app `layout::workspace_menu::tests`: at 1280 the bar's "…" is a 26 square
  right after the title group (left of the bar's middle); a press opens
  Workspace actions; the shared items are Copy workspace path then Import
  session, each with its glyph.
- app `layout::compact::tests` use the shared trigger and menu.

## Status
