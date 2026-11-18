# The empty pane as Paseo's New tab panel (TRACKER 336)

Visual lane, 2026-11-18. Built on TRACKER 798's branch (head fdce50849,
package base 14.44). Reference: pinned Paseo 0.15.4 (`1c00b365c`):
`panels/new-tab-panel.tsx`, `workspace-tabs/launcher/index.tsx` and
`internal/catalog.ts`, `components/terminal-profile-icon.tsx`,
`protocol/src/terminal-profiles.ts`.

Status: built and gated on package base 14.50 (head eb0ad6a0e); frames
with the coordinator.

## Rulings (coordinator, 2026-11-18)

1. Disabled rows at half opacity with no hover, skipped by Up/Down, the
   same look as the kit's other disabled controls.
2. Profile icons: muted marks keyed by the shipped profile id;
   SquareTerminal for Pi.
3. No pencil that looks live and does nothing: "Edit profiles" is drawn
   disabled, as (1), until Settings › Terminals exists.

## Differences from Paseo

- Disabled rows and the disabled pencil have a look (half opacity, no
  hover); Paseo has none, because its rows are rarely disabled.
- "Edit profiles" is disabled: Settings › Terminals is not drawn yet.
- Most built-in rows and every profile row are disabled until their panels
  render in a pane; Agent opens the new-agent page and does not replace the
  New tab tab.
- The profile rows are the shipped four whatever the node is configured
  with, until the node's profiles reach the app.
- No chords on the rows and no launch shortcuts from the panel yet.
- Paseo's "+" menu lists the same groups (a separator, the profiles'
  label, the rows, Edit profiles); Vessel's "+" menu keeps the built-ins
  until the kit menu has a section label (follow-up).

## Paseo

- A New tab tab's body (and nothing else) is `NewTabPanel`: a scroll view
  whose content is centred both ways with 24 padding, holding a rail as wide
  as the pane up to 380, its groups 48 apart.
- Groups, in order (`useWorkspaceTabLaunchCatalog`, surface "panel"):
  1. built-ins, no label: `PRIMARY_LAUNCH_ORDER` agent, terminal, changes,
     diff (not on a phone), files, browser, pull request; hidden ones left
     out, disabled ones listed;
  2. plugin panels, no label, only when a plugin has workspace panels;
  3. terminal profiles, labelled "Terminal profiles" with the accessory
     "Edit profiles" (Settings › Terminals of that host), only when there
     are profiles. With no configuration the four shipped ones show (Claude
     Code, Codex, OpenCode, Pi), each with its provider's icon in the muted
     ink.
- A group: rows 4 apart; a label row first when labelled: the label 12,
  line 20, extra muted, 2 in from the rows' edges, 4 above the first row,
  and at its end the pencil (12, half opacity) in a 20 box whose negative
  margin keeps the row 20 tall; the box takes surface 2 on hover.
- A row: at least 44 tall, 12 across, 8 between parts, radius 8, a 1
  border (transparent), surface 1. Hover or focus: surface 2; focus: the
  accent border; pressed: 85% opacity. Parts: icon 16 muted, label (flex,
  foreground, one line), the action's chord at the end.
- Keys: the first row takes focus when the panel mounts in the interactive
  pane; up and down move through the rows and wrap; Enter presses the
  focused row; a press on the panel outside a row focuses the first row.
  The launch shortcuts (new agent, new terminal, browser, changes, files)
  launch from the panel while its pane is focused.
- A launch replaces the New tab tab (`{ kind: "replace", tabId }`).

## Vessel before

`layout::draw::launcher`: "Choose what to open in this pane." and a New
agent button (opens the new-agent page), for a New tab tab and for a pane
with no tab.

## Plan

[Decision Log]
- Purpose and intent: the empty pane as Paseo's panel, so New tab offers
  what the pane header's "+" offers, in Paseo's form.
- Existing implementation and constraints: the "+" menu (776) lists the
  view's `catalog(LaunchPurpose::Primary, ..)` with only Agent ready; the
  others are listed disabled until their panels render in a pane. Agent
  opens the new-agent page, because a workspace draft tab cannot be created
  outside tests yet (W2's seed bridge). Vessel's launch shortcuts are bound
  but not performed. The app has no plugin panel list and no node terminal
  profiles (`terminal_profiles::CREATION_PROTOCOL` is `None`); Settings ›
  Terminals is not drawn. Brand marks come only from a node's typed agent
  identity (ADR 0048).
- Alternatives considered: keep the passive line with more buttons (not
  Paseo); render the "+" menu's rows in the pane (a menu's rows, not the
  panel's); leave the profile group out until node profiles arrive (loses
  the group the coordinator asked for).
- Chosen approach: a new `layout::new_tab` panel drawn by the app, fed by
  the same catalog, availability, words and glyphs as the "+" menu (moved
  to a shared `layout::launch`); the profile group from
  `Profiles::shipped()` (Paseo's no-configuration list), its rows disabled
  as Terminal is; one focus handle for the panel with a highlighted row.
- Why: one catalog and one availability table keep the "+" menu and the
  panel the same; Paseo shows the shipped profiles when it has no
  configuration, which is Vessel's case today.
- Pros, cons and impact: most rows are disabled until their panels land
  (honest, as the "+" menu); the profile rows show the shipped four even
  if a node is configured otherwise, until the node's profiles reach the
  app; no chords on the rows (the menu's rule).

Details:
- Built-ins: `catalog(Primary, mode, availability)`; Agent ready, the rest
  disabled. Disabled rows draw at half opacity with no hover or press, as
  the kit menu's disabled rows (Paseo has no disabled style; in Vessel most
  rows are disabled and must not look live).
- Plugin panels: none in the native app yet, so the group is not drawn
  (as Paseo with none).
- Terminal profiles: label "Terminal profiles" / "터미널 프로필", rows Claude
  Code, Codex, OpenCode, Pi, disabled. Icons: the muted marks of Claude
  Code, Codex and OpenCode, keyed by the shipped profile's id (the source is
  the app's own constant, not a label, so ADR 0048 holds); Pi has no mark
  in Vessel, so it takes Paseo's no-icon form (SquareTerminal, muted).
  "Edit profiles" / "프로필 편집": the pencil is drawn disabled (half its
  usual 50% opacity, no hover box, no press), as the disabled rows, until
  Settings › Terminals is drawn; then it opens that section.
- Keys: Up/Down move the highlight over the ready rows and wrap; Enter
  launches the highlighted row; the panel takes focus when the New tab
  shortcut (`LayoutAction::NewTab`) creates it and on a press inside it,
  highlighting the first ready row. The accent border
  shows only while the panel holds focus.
- Launch: Agent opens the new-agent page (as the "+" menu); the New tab
  tab stays until a workspace draft tab can replace it.
- Not done here: chords on rows and launch shortcuts performed from the
  panel (they wait for the launches to work); replacing the tab.
- Cleanup folded in (coordinator): the session menu's Copy agent id row
  (`MenuAction::CopyId`) lost its path when 800 removed the bar's session
  "…"; the tab menu's own Copy agent id covers it. The row, its action
  and its words go.

## Also in this branch

- Package base 14.48 merged. TRACKER 804 had reworded the dead Copy
  session id row (Copy agent id, a copied toast) and added its test
  (`conversation::copy_tests`); the row goes as planned, with that test.
  The tab menu's own Copy agent id and its toast stay (`layout::tab_menu`).
- The rename field's limit from the node's protocol (TRACKER 801; the 800
  plan's follow-up).
- A strip tab's label is one line with a tail ellipsis, as Paseo's: the
  kit tab wraps its words in a box that may shrink (`min_w(0)`,
  `truncate`), so a tab capped at 128 ends in "…" instead of a cut letter
  (the coordinator, from the 700 frames). Test: kit `tabs::tests`
  `a_long_label_ends_inside_its_capped_tab`.

## Tests

- app `layout::new_tab::tests`: groups and order (built-ins in Paseo's
  order, Diff left out on a phone, the profile group with its label and
  pencil); the rail at most 380 and centred; Up/Down skip disabled rows and
  wrap; Enter on Agent opens the new-agent page; a disabled row does
  nothing on a click.
- the session menu and tab menu tests without Copy agent id in the session
  menu.

## Frames

At 1280 and 390, dark English and light Korean: a New tab tab with the
panel; the first row highlighted after Ctrl+T.
