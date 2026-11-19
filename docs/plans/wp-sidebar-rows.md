# Project row and workspace row, as Paseo's sidebar (TRACKER 662/663)

Visual lane, 2026-11-19. Planned on package base 14.61 (`c2b8cb67`),
built on 14.63 (`786a4ed3`), merged with 14.66 (`f8928581`: TRACKER
809's project mark, which the badge now hangs from). Reference:
pinned Paseo 0.15.4 (`1c00b365c`): `components/sidebar/project-leading-visual.tsx`,
`components/sidebar/sidebar-workspace-row-content.tsx`,
`utils/sidebar-project-row-model.ts`, `components/sidebar-workspace-list.tsx`
(project row 925-1010, styles 2600-2720, `ProjectRowTrailingActions` 405-560),
`utils/project-status-badge-content.ts`, `utils/status-indicator-geometry.ts`.
impl F's TRACKER 809 owns the project's mark itself (its icon, else its
lettered square); this item draws around it and does not redo it.

Status: rulings in (below); built, gates and frames to follow.

## Visible differences: the project row (662)

Native: `areas/workspace/sidebar.rs`, the Projects section: rows 36 high,
8 inside, radius 8, 4 apart, the mark 16, the name 14 muted; a press starts a
new chat in the project (or opens its Settings page when it is not ready),
a not-ready row's reason at its end in Tiny.

- P1 Status badge on the mark (Paseo's `ProjectStatusIndicator`): the
  project's aggregate status in a 12 round shell straddling the mark's
  bottom-right corner at -4, filled with the row's own background (sidebar,
  hover, selected, surface2) so it knocks out of the mark: needs input →
  the amber filled `CircleAlert` 12; failed and attention → a 6 dot in their
  colours; running → the status ring anchored to the same corner; done →
  nothing. Native: no status on a project row.
- P2 Leading slot: as tall as the title's 20 line, the mark centred in it
  (native: the mark sits in the row's flex centre; same visually at one
  line, named for the badge's anchoring).
- P3 Hover actions at the row's end, always shown on touch: "New workspace"
  plus (when a host of it can make a worktree) and the project kebab
  (`MoreVertical`, muted, foreground on hover) with its menu (220; a sheet
  on a phone). Native: none.
- P4 Selected fill while the project is the one shown; pressed fill
  surface2. Native: hover fill only.
- P5 While the project is being removed, an 8 spinner replaces the mark.
- P6 On hover the mark becomes the section's 14 chevron (expand or
  collapse), because Paseo's project row heads its workspaces. See Q1.

## Visible differences: the workspace row (663)

Native: `areas/sidebar/session_row.rs`: a session's row (a Vessel row is a
session): the agent's mark, a 20 slot with the web page's marks (an 8 dot
with a 1 ring, the needs-you fill, a 12 spinner, an ended dash, an unread
fill), the title (unread in the primary colour, medium), labels on a second
line when there are any, the kebab on hover.

- W1 One leading status slot (Paseo's `WorkspaceStatusIndicator`, the icon
  box wide, 20 high): busy → the status ring; needs input → the amber
  filled `CircleAlert` 12; attention → a 6 dot (attention colour); failed →
  a 6 dot (failed colour); done → a 6 dot in `foregroundExtraMuted` at 30%,
  its room kept. Native draws the web page's 8 dot, ring, spinner and dash.
- W2 No agent mark ahead of it (a Paseo workspace holds agents). Native
  leads with the agent's mark. See Q2.
- W3 The title 14 on 20 in the foreground at 76%, 100% on hover, 92% while
  creating; no weight change. Native: an unread title in the primary colour
  at medium weight, others muted.
- W4 The meta line under the title (`WorkspaceMetaRow`): the branch, the
  host badge, the change request hint, services, labels. Native: labels
  only, when there are any.
- W5 Rows under a group header indented 16 (8 + 8) by the row's padding,
  so fills keep the full width. Native: 10 (`INSET`) for every row.
- W6 The shortcut badge (18 tall, surface0, a surface2 hairline, the
  number in 12 medium muted) at the row's end while the modifier is held.
- W7 The hover card on a pointer rest (`WorkspaceHoverCard`: the
  workspace's details, its change request). See Q3.

## Questions for the coordinator

- Q1 Structure: Paseo nests workspace rows under their project's row
  (grouping Project by default; Status and Labels too). Native lists
  projects on their own and sessions under their machine (grouping Machine
  or Status). P6 (the chevron, collapse) and W5's indent under a project
  only mean something with project grouping. Proposal: 662/663 draw the rows
  (P1-P5, W1, W3-W6) in today's groupings; project grouping is its own item.
- Q2 The agent mark (W2): Paseo has none, but a Vessel row is one agent's
  session. Proposal: keep it as a named difference.
- Q3 The hover card (W7): proposal: its own item.

## Plan (after the rulings)

[Decision Log]
- Purpose and intent: the sidebar's two rows read as Paseo's.
- Existing implementation and constraints: the rows above; the session's
  state comes from `SessionMark`; a project's sessions are its cards with
  that project (`CardDetail::project`); the mark is 809's.
- Alternatives considered: redraw the sidebar around projects in one step
  (Q1, a structural change beyond the rows).
- Chosen approach: a view function for Paseo's bucket of a session and a
  project's aggregate (needs input over failed over running over attention
  over done, as `getProjectStatusBadgeContent` surfaces them); a kit status
  slot (ring, alert, dot) shared by both rows; the badge drawn over 809's
  mark; the row's title, meta line, indent and badge as listed.
- Pros, cons and impact: both rows change look; frames at 1280 and 390,
  dark English and light Korean, with a project in each status.

## Rulings (coordinator, 2026-11-19)

- Q1: this item draws the rows in today's groupings (machine, status):
  P1-P5, W1, W3-W6. Grouping by project, with P6's chevron and the rows
  under their project, is TRACKER 682 (next, plan first: it may need the
  node's link from a project to its sessions).
- Q2: no agent mark, as Paseo; not a named difference. The status slot
  leads the row as Paseo's W1.
- Q3: the hover card is TRACKER 810, after 682.
- Frames: 1280 and 390, dark English and light Korean, each status shown
  (running ring, needs input, failed, attention, done) and a project
  being removed.

## As built

- view: `Card::bucket` (Paseo's bucket in `attention::Bucket`, which held
  Paseo's five already: needs input from the board's needs-you, failed
  from a failed process, running from work or a start, attention from
  unread, else done); `Fleet::project_bucket` (the fold over a project's
  listed sessions on its machine, through the card's new `in_project`,
  the project a session runs in, worktrees included); `workspace_kind`;
  the words (Paseo's bucket names, the project row's menu and question).
- kit: `status_mark` (the workspace slot, the badge on a mark with its
  knockout following the row's hover and press, the running ring, the
  needs-input alert), `theme::SidebarSurfaces` (Paseo's sidebar fills
  per theme), the status dot colours, three glyphs, `Points::EIGHT`.
- app: the session row (`sidebar/session_row.rs`, `row_meta.rs`): W1,
  W3-W6 and the vertical kebab over the title's end with Paseo's scrim;
  the project row (`workspace/sidebar.rs`, `sidebar/actions.rs`): P1-P5,
  the dark hover fixed (it was the light theme's `surface3`).
- P4: Paseo never fills its project header as selected
  (`selected={false}`); the row presses to `surface2` as before.
- W2 dropped as ruled.

## Differences, named

- The rings turn from when each is drawn (Paseo's share one clock).
- The change request on the meta line says, it does not link (the
  board's card and the Changes tab open it); Paseo's services item has no
  source here; its branch and project items are off by default.
- The project menu has no Open in new window or Open in file manager
  (Paseo's desktop shell's); the plus's tooltip has no shortcut (no
  binding for a new worktree).
- Needs input also covers the board's own needs-you reasons (failing
  checks, requested changes), which Paseo has not.
