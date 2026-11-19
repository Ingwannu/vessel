# The sidebar grouped by project, as Paseo's (TRACKER 682)

Visual lane, 2026-11-19. After TRACKER 662/663 (the rows; coordinator
visual PASS, with the app reviewer), on package base 14.66. Reference:
pinned Paseo 0.15.4 (`1c00b365c`): `components/sidebar-workspace-list.tsx`
(`ProjectBlock` 1680-1850: the header row, its children, the toggle row,
the ghost row; `handleToggleCollapsed`), `components/sidebar/
project-leading-visual.tsx` (the chevron on hover),
`components/sidebar/sidebar-group-visibility.ts` (7 rows, more on
demand), `stores/sidebar-view-store.ts` (`groupMode`: `project` by
default, or `status`), `stores/sidebar-collapsed-sections-store`
(collapsed projects, kept per device), `projects/workspace-structure.ts`
(projects sorted by name; one project across hosts by an equivalence key).

Status: plan for the coordinator; nothing built.

## Paseo

- Grouping "Project" (the default) lists every project as a header row
  (662's row) with its workspaces under it, indented 16; "Status" lists
  status groups.
- A press on the header folds or unfolds the project; while hovered its
  mark becomes the 14 chevron (down when open, right when folded).
  Folded projects are kept per device.
- Under an open project: its first 7 workspaces and every one not done,
  the open one when it would be hidden, then "Show more" / "Show less"
  (in place). A project with no workspace shows the ghost row "New
  workspace" when a host can make one, else nothing.
- Projects are in a stored order (dragged), else by name; one project on
  several hosts is one header (an equivalence key from the repository).
- There is no separate projects list: in project grouping the tree is the
  list.

## Vessel today

- The sidebar has a Projects section (662's rows; a press starts a new
  chat in the project) and below it the sessions grouped by machine or
  by status (`prefs::Grouping`: `machine`, the default, or `status`).
- The node already links a session to its project: `SessionInfo.workspace`
  (protocol 10) names the project for sessions at its root, in a folder
  of it, or in one of its worktrees; 662 keeps it on the card
  (`CardDetail::runs_in`). No node change is needed. Sessions of a node
  older than 10, or started outside any project, have no project.
- The page and the bridge know three groupings: `machine`, `status` and
  `workspace` (the registry's workspaces per machine with their label
  chips, ruled 2026-11-16, ADR 0106); the native app draws the first two.
  The native document may state its own default (TRACKER 793 did, for
  delivery) and may carry native-only layout fields (`sidebar_width`).

## Plan

[Decision Log]
- Purpose and intent: the native sidebar's default view reads as Paseo's:
  projects heading their sessions, foldable.
- Existing implementation and constraints: the rows (662/663); projects
  per machine (`ProjectsView`, read from online machines); the session's
  project (`runs_in`); the shared preferences (`Grouping`, the page and
  the bridge's contract test).
- Alternatives considered:
  - rows under a project as registry workspaces (Paseo's unit): the
    registry has workspaces for project roots only (ADR 0106: worktrees
    wait for their own backing decision), so worktree sessions would have
    no row;
  - reuse the page's `workspace` value for this tree: the page draws a
    different shape under that name (workspace rows per machine);
  - keep the Projects section and add a fold to it: two lists of the same
    projects.
- Chosen approach:
  - a third grouping `project` in the native preferences ("Project" /
    "프로젝트" in the Display menu's Grouping page, before Status);
  - in it, the Projects section becomes the tree: each project (one row
    per machine and project, the machine named at the row's end when
    several machines have projects, as today) heads its listed sessions
    (session rows as 663, indented 16); the session-to-project link is
    `runs_in`;
  - the header press folds or unfolds; the chevron on hover replaces the
    mark (P6); the plus and the kebab stay as 662's; folded projects are
    kept in the native document's layout section (native-only, as
    `sidebar_width`), by machine and project id;
  - under an open project: Paseo's 7, every one not done, the open one,
    then "Show more" / "Show less" ("더 보기" / "간략히 보기"); an empty git
    project shows the ghost "New workspace" row;
  - sessions with no project: one trailing group per machine, headed by
    the machine's name and state (as the page's workspace grouping keeps
    "sessions in no workspace"), so an unreachable machine still says so;
  - projects by name, numeric and case-insensitive (Paseo's fallback
    order); `Fleet::sidebar_order` walks the tree as drawn (open projects'
    shown rows, then the trailing groups), so the jump keys and their
    numbers follow it;
  - Machine and Status keep today's sections.
- Why: Paseo's tree with the link the node already sends, nothing waiting
  on the registry, the shared document unchanged for the page.
- Pros, cons and impact: a third value the page does not draw yet (its
  own item: either draw it or map it); with several machines a project
  that lives on two of them is two headers (no cross-machine identity in
  `ProjectInfo`); no drag order. Frames at 1280 and 390, dark English and
  light Korean: open and folded projects, the chevron on hover, Show
  more, the ghost row, a machine's trailing group, an unreachable
  machine.

## Questions for the coordinator

- Q1 The value: a native `project` grouping as above (the page and the
  bridge to follow in their own item), or another way (the shared
  `workspace` value, or adding `project` to the bridge and the page in
  this item)?
- Q2 The default: Paseo's default is Project. Make it the native default
  (a stated difference from the page's `machine`, as 793's delivery), or
  keep `machine` until the page has it?
- Q3 Sessions with no project: one trailing group per machine (proposed),
  or Paseo-like hoisted rows at the end with no header?

## Tests

- view: the tree from a fleet (membership by `runs_in`, a worktree
  session under its project, no-project sessions in their machine's
  trailing group, the 7 + not-done rule, the open one kept, name order);
  `sidebar_order` follows it, folded projects skipped.
- app: a press folds and unfolds; the chevron is the leading visual on
  hover (a11y: the header says expanded); folded projects survive a
  restart (the document); Show more / Show less.

## Rulings (coordinator, 2026-11-19)

- Q1: `project` is added to the shared grouping value in the bridge in
  this item, so the document never holds a native-only value. The page
  reads `project` as its Machine view (`pageGrouping`) until its own item
  (TRACKER 812, page Project grouping).
- Q2: Project is the native default, as Paseo's. Existing users keep the
  value they chose; only an unset value takes the new default. The page's
  Machine default is a stated difference until 812.
- Q3: one trailing group per machine for sessions with no project, headed
  by the machine's name and state; an unreachable machine stays visible.
- Accepted differences: two headers for a project on two machines; Paseo's
  dragged project order is TRACKER 811 (after 810); projects sort by name
  until then.

## As built

- Bridge and page: `Grouping::Project` in `crates/vessel-web` (generated
  `web/src/wire/bridge.ts`); the page maps it to Machine (`pageGrouping`
  in `web/src/data/sidebar.ts`, used by the list and the Display menu).
- Native document: `Grouping` defaults to `Project`; folded projects in
  `folded_projects` (native only, left out while empty, read leniently).
- View: `sidebar::tree` (`Fleet::sidebar_tree`, `SidebarTree::order`,
  `by_name`, `PROJECT_SESSIONS`): only reachable machines' projects are
  listed; an unreachable machine's sessions wait in its trailing group.
- App: `areas/sidebar/projects.rs` draws the list (blocks with role group,
  sessions flush at 8, the toggle row, the trailing machine groups through
  `machines::group`); `areas/workspace/sidebar/tree.rs` the heading
  ("Projects", the Display trigger, the plus), the header (the project row
  folding, the chevron in `SidebarSurfaces::chevron` on hover,
  `aria_expanded`) and the ghost row. The shortcut numbers count the tree
  as drawn. Show more is kept in memory only, as Paseo's.
- A header that is not ready (cloning, failed, missing) keeps opening its
  page in Settings and draws no chevron.
