# Vessel web UI — design

This file is the contract for `vessel ui`'s page: why it is shaped this
way, how it maps onto the protocol, and what must not be improvised.

- The tokens are `web/src/ui/tokens.css`. The controls are the kit,
  `web/src/ui/kit/` (§6.13): one component per file, its stylesheet beside
  it, on Base UI for behaviour (ADR 0021). The areas' styles are
  `web/src/ui/styles/*.css`: one file per area, imported in a fixed order
  from `styles/index.css`. There is no other token file (ADR 0018).
- The pictures are the app's own screens: `npm run screens` in `web/`
  captures every main screen in both themes, and the kit's gallery at
  `/kit` (every control in each state and size) light and dark, at full
  width and at 390px.
- Milestone 7 built the first version from a dark, blue-accented draft
  with static mockups. Milestone 8 (ADR 0018) made it white and black, one
  design system, with sessions as a board. Milestone 9 (ADRs 0019 and
  0020) took the rest from agent-orchestrator, the product the owner wants
  this one to resemble: its shell, its values, its lanes, and the node
  deciding each session's lane. The draft's mockups are in the history,
  not in the tree. Milestone 11's track W added what the three references
  have and the page lacked: Settings' General, Appearance, Shortcuts and
  Notifications (Paseo is the reference for settings), drafts and recall
  in the composer, find and the outline in a session, the sidebar's
  Display menu and its sheet on a phone, all kept by the bridge (ADR
  0028).
- **The reference is agent-orchestrator** (Apache-2.0): its `DESIGN.md`,
  its token file, its board view and its controls. Where this file gives a
  number, it is theirs unless it says otherwise. Where it has no such
  control, the second source is
  [GOAT](https://github.com/goat-si/goat) (MIT); agent-orchestrator wins
  where the two differ. The deliberate differences are listed once, in
  §12: keyboard focus always visible, text held to WCAG AA, Pretendard
  after Geist for Hangul, and no glass or blur.

The product is an **Agentic Development Environment**: one seat driving
AI coding harnesses (Claude Code, Codex, Gemini, OpenCode) as sessions on
several of the operator's machines (nodes). It is a control room, not a
marketing site and not a chat app with a sidebar bolted on.

---

## 1. Principles

1. **The daemon is the product; the UI is a client.** Every label, status
   and action maps to a protocol type (`SessionInfo`, `NodeInfo`,
   `AgentEvent`, fleet generation, device scopes). Do not invent a second
   vocabulary. If the wire has `Interrupted`, the badge says interrupted.
2. **Calm density.** Zed's restraint, Linear's clarity. Information is
   close together because the operator is comparing nodes, not browsing a
   landing page. Padding is a 4px grid, not a marketing 80px section.
3. **White and black is the design.** A near-white page framed inside a
   slightly grey shell, near-black ink. Dark is the same system on dark
   greys. Depth is hairlines and one step of grey, not drop shadows.
4. **The accent is the ink; colour is a lane.** Selected, focused and
   primary are the ink: a grey pill behind a row, a grey ring, a
   near-black button. The only hues are the board's: one per lane
   (building blue, validating plum, in review amber, ready green) and one
   for a session that waits on its owner (needs-you red). A colour on
   screen always answers "where is this session". Syntax colours appear
   inside code, diffs and the terminal.
5. **Monospace is a material.** It marks things a machine produced or a
   person could type: ids, paths, models, versions, generations, prompts,
   code, the terminal. Human sentences — navigation, button labels, empty
   states, explanations — are the sans face (Geist, with Pretendard for
   Hangul).
6. **Attribution is visible.** A session created by a person and a
   session created by an agent on another node are not the same row with
   a different icon. Agent-created sessions nest under their creator.
   That is ADR 0013 made visible, not a nice-to-have tree widget.
7. **Live, never silent.** A lagged subscription, a draining node, a
   reconnecting attach are first-class states with copy, not spinners
   that hope. Nothing is skipped silently (`session.subscribe` already
   forbids it).
8. **Taste over chrome.** No gradients, no glows, no card grids of
   identical feature tiles, no 3D device frames, no looping typing
   animation. If a control is not needed this minute, it is in a menu.
9. **Sentence case, and words that earn their place.** Labels, headings,
   buttons and column headers are sentence case; nothing is uppercased
   for effect. A line of help that only restates the control next to it
   is removed, and a view has one primary action: New is in the topbar on
   every page, so no page repeats it in its own head.
10. **Borders are structure.** A hairline separates durable regions:
    panes, lanes, the rows of one list. A thing does not get an outline
    for being rectangular. The one object with a border of its own is a
    board card, because it is a bounded thing to open.

### Why this, not a chat shell

The obvious template for "talk to an agent" is a single transcript and a
prompt box. Vessel's job is fleet-shaped: which machine, which account,
who started this, can I move it, is the member still on generation 48.
A chat shell hides the fleet. The session view is allowed to feel like a
conversation; the rest of the app must feel like an IDE's debug console.

---

## 2. Information architecture and navigation

### 2.1 Places

| Place | What it is | Primary objects |
| --- | --- | --- |
| **Board** | Every session, one machine or all, as a board | session card, status lane, creator |
| **Session** | One session: its timeline and composer, or its terminal | title, timeline, options, permission, summary |
| **New session** | A page, `/new` (§7.5) | prompt, agent, model, effort, permissions, machine, folder |
| **Settings** | Sections, each a page (§3.1): General, Appearance, Shortcuts, Notifications; Agents, Models; Machines, Devices, Sync; About (`data/settings.ts` is the one list) | the send key and where choices are kept; theme, font sizes, the conversation's width and display; every shortcut; notifications; each machine's agents and what they offer, its routes and profiles; the machines (add, rename, default, forget), their devices and what one shares; versions and licences |
| **Welcome** | The first screen while no machine is connected (§3.1) | the add-machine dialog (§7.6), this machine's start command |

There is no home dashboard: the index is the new-session page once any
machine is connected (GOAT's first screen is a draft's composer; the new
chat, 2026-11-13), and the welcome until then; the board is the
sidebar's Board. What was the Fleet page (the
machines and their inspector) is Settings › Machines, as
agent-orchestrator and GOAT keep machines in their settings; Sync and
Devices are sections there too (milestone 10, track W2).

### 2.2 Chrome

```
┌─ shell (grey) ────────────────────────────────────────────────────────┐
│ ▲ Vessel        ┌─ frame (the page, 12px radius, a hairline) ───────┐ │
│ ⌕ Search    ⌘K  │ Board                                          New │ │
│ ✎ New session N ├────────────────────────────────────────────────────┤ │
│ ▥ Board       5 │                                                    │ │
│                 │                                                    │ │
│ Machines        │                                                    │ │
│ ● laptop      3 │                                                    │ │
│   ◌ Fix the ov… │                                                    │ │
│   ● Release no… │                                                    │ │
│ ● gpu-box     2 │                                                    │ │
│ ─────────────── │                                                    │ │
│ Settings      ◐ └────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────────────┘
```

- **One shell, one frame.** The sidebar (240px) and the margin around the
  page are one grey surface, so the sidebar has no edge. The page is a
  framed surface set into it: 12px radius, a 4% hairline, inset 14px top
  and bottom and 16px on the right, flush with the sidebar on the left.
  `features/shell/` draws both.
- **The title bar is inside the frame**: 36px, the page's one heading
  (16px semibold) after its crumbs, then New. A page that names itself
  draws into the bar instead (`TitleBar`, a portal into its slot): a
  session's title, renamed in place (§7.4). On the new-session page the
  bar is bare chrome, without its hairline, as agent-orchestrator's is.
  The link to `vessel ui` shows only when it is not up ("vessel ui ·
  reconnecting", "disconnected", "refused").
- **The sidebar, top to bottom** (agent-orchestrator's): the brand (the
  owner's mark, `docs/design/brand/`, in the text's colour at 18px, then
  "Vessel"; the welcome draws it at 28px, and the page's icon is it in
  white on a rounded tile of the brand colour, all made by the build from
  the brand's file, `web/vite-mark.ts`); Search
  (a filled row with its shortcut as text); New session (with its key,
  `N`); Board, with the live count; "Machines" with Display (a sliders
  icon) and a "+" that adds one (§7.6), each machine with its live
  sessions under it; Settings and the theme switch at the foot. The
  sidebar hides with its shortcut (`⌘B`) or the title bar's button, and
  stays hidden across launches (ADR 0028).
- **Projects** (agent-orchestrator's project tree), above the machines:
  each connected machine's projects, a 32px row each (a folder whose mark
  turns into a chevron on hover, the name, the machine in the passive ink
  when two or more machines have projects, the live count), the first ten
  then "Show N more projects". A row opens to its live sessions, 14px in;
  its "+" (on hover) starts a session there (the new-session page opens
  with the project chosen); the heading's "+" leads to Settings ›
  Projects. The group takes at most two fifths of the sidebar and scrolls
  on its own. A session's row ends with its worktree's branch (mono 11px,
  at most 36% of the row), when it has one.
- **Display** (Paseo's sidebar menu, in agent-orchestrator's menu):
  Grouping (Machine, Status), Sort (Created, Last activity, newest first
  either way) and a "Show ended sessions" switch. Grouping by status
  heads the list "Sessions" and lists Needs you, Working, Idle and Ended
  (a group with none is left out), up to ten sessions each, every row
  naming its machine at the right. The order the sidebar shows is the
  one "next session", "previous session" and "session 1–9" walk (§7.1).
  While the address is under `/settings`, a Back row and Settings'
  sections (from `data/settings.ts`, under their headings, the current
  one marked) take the machines' place.
- **A row** is 32px, 10px radius, 14px medium, secondary text. Its fill is
  a pill that grows from 4px inside the row on hover (150ms) and stays, at
  7% instead of 4%, on the current row. No bar marks the current row.
- **A machine's row** leads with its state dot and ends with its live
  count (an em dash when offline, so it does not look idle-but-fine); its
  dot's word is the machine's state ("Connected", "Offline", the same
  words as Settings, `machineState`). It opens the machine's detail in
  Settings › Machines, which links to the board scoped to it.
- **A session's row** is set in from its machine's row (agent-
  orchestrator's nested list): 22px on the left, so its mark sits under
  the machine's name, not under its dot. It leads with a mark: a spinner
  while its agent works, a filled needs-you dot while it waits on its
  owner, else a hollow dot; then its title (§7.4): a session with no
  title is named by its folder, and one with neither (its folder the
  home) is "Untitled session", never `~`. Up to five per machine,
  pinned ones first, then newest first, then "N more" to the board
  scoped to that machine. Ended sessions are listed only when Display
  asks for them; archived ones never (all three references leave them
  out of their sidebars).
- **Unread** (protocol 10, the machine's `marks.unread`): the row's
  title in semibold primary text and its hollow dot filled in the muted
  ink, agent-orchestrator's unread dot. A mark the page asked for shows
  when the machine reports it.
- **Compact** (below 600px, Paseo's `isCompactBreakpoint`: its `xs`
  and `sm`, below `md` = 600; `src/ui/breakpoints.ts`) the sidebar is a
  sheet (the kit's `Sheet`, GOAT's phone sidebar) from the title bar's menu button, over a scrim, so
  Settings' sections are reachable on a phone; picking a row closes
  it.
- **Search** (`/` or `⌘K`) opens the command palette (§6.13): New
  session and Add a machine; the board and each Settings section; the
  machines (each opening its detail); the sessions by title; the theme.
  The shortcut works with the sidebar hidden: the palette is the
  shell's, the row only opens it.
- **New** is the one primary action, in the title bar on every page but
  the new-session page itself, and opens `/new` on the machine in view
  (§2.5): the session's, the one a Settings section is about, or the one
  the board is scoped to.

### 2.3 Routes

Names are UI, ids are protocol.

```
/                         the board once a machine is connected; the welcome before
/new                      a new session (§7.5)
/new?machine=:node        a new session, starting on that machine
/sessions                 the board, all machines
/sessions?node=:node      the board, one machine
/sessions/:node/:id       one session, its timeline or its terminal; a segment
                          that is not a machine's key or a session's id says
                          "No session has this address." before any call
/settings                 → /settings/general
/settings/:section        one section; an unknown one → the default
/settings/:section?machine=:node            the machine a section is about
/settings/agents?machine=:node&agent=:kind  an agent's card open
/settings/machines?machine=:node            a machine's detail open
/fleet, /fleet/:node      → /settings/machines(?machine=:node)
/sync?node=, /devices?node=  → /settings/sync, /settings/devices (?machine=)
```

Creating a session is a page, `/new` (agent-orchestrator's new-task
page): the chat before it has a session. Every "New" leads there.

### 2.4 Session hierarchy

`SessionInfo.created_by` (protocol 5) is either the owner/device or an
agent `{ node, session }`. Sessions are a **tree** (`data/tree.ts`):

- Roots: created by a person (this device, or another paired device),
  newest first.
- Children: created by that session's agent, on any node.
- Depth is at most 3 (ADR 0013).
- A child can live on a different node than its parent.

On the board (§6.12) the tree is kept two ways, because a parent and its
child need not share a status:

- A card an agent started carries a line "agent of `3f2a`", linking to
  that session, and "on gpu-box" when the parent runs on another node. It
  is there whatever lanes the two are in.
- Inside a lane the tree's order is kept, so a child whose parent is in
  the same lane sits right under it, indented 14px a level (two levels at
  most) with a connector.

Do not flatten this into a card field with a raw id and no link. The tree
*is* the point of a fleet.

### 2.5 Which node is "the" node

Devices, pairing, drain/stop, and the book entry are per machine. Sync
conflicts are per member. The machine meant is one rule,
`data/nodes.ts`' `machineMeant`, for every page:

1. the machine in the address (`?machine=`, a session's, the board's
   scope; opening a machine's detail in Settings selects it), else
2. the last machine the operator selected this session, else
3. the book's `default_node`, else
4. the first in the book (this machine's when the book names none).

Each page says which machines it accepts: a new session only a
connected one; a Settings section one in the book. The sidebar marks
the machine whose detail is open. Switching it in a section's picker
does not leave the section (Sync stays Sync).

---

## 3. Layout grid

A 12-column grid inside `main`, not including the sidebar.

| Breakpoint | Sidebar | Main | Columns | Gutter |
| --- | --- | --- | --- | --- |
| ≥ 1100px | the whole sidebar, 240px | fluid | 12 | 16px |
| 600–1099 | the whole sidebar, 240px (down to 200 below 640, so the page keeps 400) | fluid | 8 | 12px |
| < 600 (compact) | hidden, a sheet from the title bar | fluid | 4 | 8px |

The sidebar beside the page is Paseo's desktop sidebar
(`desktop-sidebar-layout.ts`, TRACKER 806): drawn whole at its width,
narrowed to 200 at least so the page keeps a 400 centre, and hidden only
by its toggle or at a compact width. There is no rail of icons (Paseo has
none; the native app already follows this). Paseo also hides it where a
route needs more than the centre (Settings' split, 720; a workspace with
its explorer, 600); the page's Settings draws its sections in the sidebar
itself and its code panel is a sheet below 1280, so neither applies.
`--sidebar-min-w` and `--center-min-w` (tokens.css) hold the two numbers.

Compact is Paseo's: a window below 600px (TRACKER 805; the page folded
at 768 before, the native app moved to 600 in 798). The number lives in
`src/ui/breakpoints.ts`; stylesheets write `@media (--compact)` and the
build puts the same query in its place (`vite.config.ts`
`compactMedia`, which refuses a width query spelling 599/600 or the old
767/768).

Page padding inside main: 16px (12px when compact). Max content width is
the pane, not a centred 720px reading column — this is an app.

### 3.1 Common page frames

**Settings.** agent-orchestrator's settings page: the section's name is
the title bar's heading (after "Settings"); the body is a centred column
896px wide (GOAT's readable width) with 24px around it and 48px below,
one lead sentence, then sections 40px apart, each a 14px medium heading
(an action at its right: Add machine, a machine picker, a quiet link)
12px over a card of the kit's rows (§6.14). Detail is one level down: a
chosen machine's or agent's card opens under its list, never as a page
of its own, and literal values (an address, an id, a path) are only in
those detail rows. A section about one machine (Agents, Models, Devices,
Sync) names it in its heading and offers a ghost select of the machines;
the choice is the address's `?machine=`, so it survives a reload and a
move between sections. Compact, the column's padding is 12px.

- **Machines**: a row per machine (a machine icon, its name, one line:
  platform, how it is reached, its state, live sessions; "This machine"
  and "Default" badges; a "…" menu of Rename…, Make default and, apart
  and in the danger colour, Forget…). This machine (the node `vessel ui`
  runs beside) has no Rename or Forget. The chosen machine's detail:
  state, how it is reached, what it runs on, its Vessel and protocol,
  its id; its agents (each opening its card in Agents); its live
  sessions.
- **Projects** (Paseo's projects, agent-orchestrator's add-project
  flow): "Projects on <machine>" with the machine picker and "Add
  project" (Add a folder…, Clone a repository…, Start a new project…);
  a row per project (its folder, its state while it clones or after a
  clone failed, its default branch or "no git"); the machine's git when
  it is missing or too old, as a warning above. The chosen project's
  detail under the list: Folder, Default branch, Added; Commands (what
  its `vessel.toml` names for a new worktree and as scripts and
  services, Approved or Not approved, Review… opening the approval,
  Withdraw); Worktrees (branch, setup's
  state, live sessions, why it was kept; Run setup, Remove…); Snapshots
  (Drop, asked again in place: Keep or "Drop for good"). What the machine
  refuses there (removing a worktree or the project, approving, setup, a
  drop) is said in a banner until dismissed, not in a toast. Adding:
  agent-orchestrator's clone dialog (Repository URL, which takes
  `git@host:org/repo.git` too, Clone into, "Repository will be created at
  …"); a clone answers at once and its progress and failure are the
  row's ("Receiving objects · 42%", the reason in words with git's last
  lines).
- **Agents**: every agent Vessel drives and every agent added on the
  machine by its command (ADR 0032, by its label, after the four),
  found before not found, as GOAT's provider rows: the agent's letter,
  its name and version, a dot and one headline: Available (an added one
  "Runs as a conversation; it has no terminal"), Needs attention (with
  the machine's reason: installed but no conversation adapter), Not
  found (an added one naming its program). The added agents are read
  from the machine (`agent.list`) when the section opens. Adding,
  removing, the registry and installing are not drawn (a later track's).
  Its card: status, version, program, then what it offers, asked of the machine when the
  card opens (`harness.options` starts the agent once there; the card
  says so while it waits) as one list in the page's words: Models,
  Effort, Permissions, then the agent's other options, each with its
  values and the one in use.
- **Models**: the machine's routes (provider, model, small model, API)
  and profiles (agent, where its models come from), read-only.
- **Devices**: the devices paired with the machine (what each may do,
  when seen), Revoke behind a confirmation; then a card for a new link:
  a name, New link, and the link and its QR code hidden until asked for
  and hidden again after 30s, on blur and once copied.
- **Sync**: what the machine shares (its version, the secrets its tool
  servers name), its files and tool servers, the machines that pull it
  (up to date, behind, not yet) with any kept edit as a warning above
  them. A machine that shares nothing says how to make it the one that
  does.
- **General**: Composer (the send shortcut: Enter, or ⌘/Ctrl+Enter with
  Enter starting a line; agent-orchestrator's "Chat send key"; and "While
  the agent works": Queue, Steer or Interrupt, what Send does during a
  turn, §6.15), then
  Kept: where the bridge keeps the preferences (the file's path) and the
  drafts, or why they are kept for this launch only.
- **Appearance** (GOAT's and Paseo's): Theme (the colour scheme: system,
  light, dark); Typography (the interface's text size, 12–20px, 14 by
  default, and code's, 10–18px, 12 by default: every type token scales
  with them); Conversation (its width: Comfortable 768px, Wide 1152px,
  Full; its display: Full, or "Collapse tool calls", Paseo's overview,
  which folds each run of tool calls, a single one included, into one
  row saying what ran).
- **Shortcuts** (agent-orchestrator's "Keyboard shortcuts"): a search
  over commands and keys, All commands / Modified, Reset all; then a
  card per group (General, Navigation, Session, Panels), its heading a
  band on the well's fill, a 44px row per command: its name and a
  "Modified" badge, its keys as this platform writes them ("or" between
  two), Edit and a menu (Add another, Remove a key, Restore default). A
  fixed key says "Fixed". Edit turns the keys into the recorder, "Press
  shortcut · Esc to cancel": the first chord held with ⌘, Ctrl, Alt or
  Meta is taken, refused in place when it needs a modifier, when the
  browser keeps it or when text editing uses it. A key another command
  holds opens "Shortcut already in use" (Reassign or Cancel); a clash
  found in what is kept is said on both rows. Compact, the search
  takes its own row.
- **Notifications**: Sessions (Session notifications: Off,
  Notifications only, Sound only, Notifications and sound; Count in the
  tab's title), then This launch (the browser's permission, with Allow
  while it has not been asked; Test notification). See §7.7.

Each of these four is a preference the bridge keeps (ADR 0028), so it
holds across launches and in every tab, with Settings › General's note
when it is kept for this launch only.
- **About**: this page's version and licence, each machine's version and
  the protocol it speaks with the page, and every package the page
  bundles with its licence (checked against `package.json` by a test).

**Welcome.** Paseo's first screen in agent-orchestrator's welcome column
(400px, centred): the mark, "Welcome to Vessel", one sentence, the
primary "Add a machine"; under a hairline, while this machine's own node
is not running, "Or use this machine" with `vessel node start` to copy
and Check again (the page also tries again when it gets focus back). It
leaves for the board as soon as a machine connects, and stays while one
is being redialled, so it does not blink (`data/welcome.ts`).

**Sessions.** The board fills the frame under the title bar: four lanes,
each a 48px heading over its own scrolling column of cards, then a 36px
bar with the archive's count. When the book has more than one node a slim
row above the lanes scopes the board to one. No inspector; a card opens
the session. Below 1024px of pane the board scrolls sideways (lanes keep
256px); compact, the lanes stack.

**Session with a timeline.**

```
title bar 36px (the frame's): mark · title (rename in place) · ⎇ branch · state · ⋯
┌──────────  minmax(0, 1fr)  ───────────┬── 340px (500px for code) ──┐
│      timeline, a column ≤ 768px       │ ☰  ⇄  ▢   (icon tabs)      │
│                                       │ ▸ Plan                     │
│                                       │ ▸ Files                    │
│  ┌ permission, docked over the box ┐  │ ▸ Model calls              │
│  ┌ composer ──────────────────────┐   │ ▸ Details                  │
│  └────────────────────────────────┘   │                            │
└───────────────────────────────────────┴────────────────────────────┘
```

The log and the composer share one column, at most 768px and centred
(agent-orchestrator's transcript; Appearance's Wide is 1152px and Full
the frame's width); the composer's dock is 16px in and 12px off the
frame's foot. The panel beside it is agent-orchestrator's inspector: a
36px head of icon tabs (the kit's `PanelTabs`), Summary, Changes and
Files. Summary is 340px of sections (plan, files touched, model calls,
Details); Changes and Files are the code panel (§6.18), 500px (the
reference's default) and never past 45% of the window. The panel leaves
below 1280px of window, where the branch's chip opens it as a sheet from
the right instead, and hides with its shortcut (`⌘⇧B`). Over the log:
the find bar at its top right and the outline on its left edge (§6.16).

**Terminal session.** The same title bar, a reconnecting banner if
needed, then the terminal consumes all remaining height. No summary.

**New session.** A bare title bar, then one column at most 672px wide,
centred in the frame: the heading "What do you want to work on?" (30px,
regular, tracked tight; 24px when narrow), 20px over the composer with
its tray (§6.15), and under them the page's notes (why Send waits, the
node's failure).

**Dialogs** (add a machine, move a session, confirm kill, revoke or
forget): the kit's (§6.9).

### 3.2 Spacing scale

| Token | px | Use |
| --- | --- | --- |
| `--s-1` | 4 | icon gaps, badge padding |
| `--s-2` | 8 | compact row padding, inline gaps |
| `--s-3` | 12 | default control padding |
| `--s-4` | 16 | pane padding, dialog padding |
| `--s-5` | 24 | section gaps |
| `--s-6` | 32 | page-level gaps (rare) |
| `--s-7` | 48 | empty-state vertical rhythm |

Row heights: sidebar item 32px, table row 40px, topbar 40px, composer
min 72px. Touch targets on a compact window inflate to 44px; on desktop we
keep 32px because this is a pointer-first operator tool, with visible
focus for keyboard.

---

## 4. Design tokens

All tokens live in `web/src/ui/tokens.css` as CSS custom properties on
`:root`. Every colour is one `light-dark(<light>, <dark>)` value (ADR
0021), so the two tables below are the two halves of the same
declarations, and a token cannot exist in one theme only. Which half is
used is the CSS colour scheme (§8, Theme). `web/src/ui/kit/` and
`web/src/ui/styles/` use tokens only; a literal colour there is a defect,
except the pairing QR code, which stays dark on light to scan.

Do not introduce a third palette, and do not add a hue to the chrome.
Status and syntax draw from the tables below.

### 4.1 Colour — light (the design)

| Token | Value | Role |
| --- | --- | --- |
| `--bg-shell`, `--bg-sidebar` | `#F4F4F4` | the shell: sidebar and the margin around the frame |
| `--bg-base` | `#FAFAFA` | the page inside the frame |
| `--bg-surface` | `#FFFFFF` | cards, inputs, the composer |
| `--bg-elevated` | `#FFFFFF` | dialogs, menus (with `--shadow`) |
| `--bg-hover` | ink at 4% | hover, laid over any surface |
| `--bg-active` | ink at 7% | the current row |
| `--bg-well` | ink at 4% | the search row, an avatar's tile |
| `--border-subtle` | black at 8% | a card's edge and its inner divider |
| `--border-faint` | black at 4% | the frame, the title bar, lane dividers |
| `--border-strong` | black at 16% | a focused well |
| `--text-primary` | `#121212` | titles, primary copy |
| `--text-secondary` | `#5D5D5D` | body, rows at rest, card metadata |
| `--text-tertiary` | `#666666` | counts, hints (theirs is `#868686`, which is 3.6:1) |
| `--text-disabled` | `#A3A3A3` | disabled controls (not running text) |
| `--accent` | `#121212` | selection and the primary button: the ink |
| `--ring` | `#8C8C8C` | the focus ring |
| `--st-building` | `#2563EB` | the Building lane, "Working" |
| `--st-validating` | `#9B4871` | the Validating lane |
| `--st-review` | `#B45309` | the In review lane (theirs is `#D97706`, 3.2:1) |
| `--st-ready` | `#15803D` | the Ready lane (theirs is `#16A34A`, 3.3:1) |
| `--st-needs-you` | `#C2412D` | a session that waits on its owner |
| `--st-merged` | `#9333EA` | a merged pull request |
| `--danger` | `#9D0006` | failed, kill, revoke |
| `--online` / `--warn` / `--interrupt` | ready / review / tertiary | a node's and a process's state, in the lanes' hues |

Hover, selection and hairlines are overlays (the ink or black at a few
percent), not fixed greys, so one value is right on the shell, the page
and a card.

### 4.2 Colour — dark

| Token | Value |
| --- | --- |
| `--bg-shell`, `--bg-sidebar` | `#121212` |
| `--bg-base` | `#181818` |
| `--bg-surface` | `#222222` |
| `--bg-elevated` | `#2E2E2E` |
| `--bg-hover` / `--bg-active` | the ink at 4% / 7% |
| `--border-subtle` / `--border-faint` / `--border-strong` | white at 7% / 4% / 14% |
| `--text-primary` | `#F5F5F5` |
| `--text-secondary` | `#A4A4A4` |
| `--text-tertiary` | `#8F8F8F` |
| `--st-building` | `#60A5FA` |
| `--st-validating` | `#E58BB5` |
| `--st-review` | `#F59E0B` |
| `--st-ready` | `#4ADE80` |
| `--st-needs-you` | `#F06445` |
| `--st-merged` | `#C084FC` |
| `--danger` | `#FB4934` |

### 4.3 Contrast (WCAG AA, measured)

Text smaller than 18px needs 4.5:1. All running text in this UI is 10.5px
to 14px, so every pair below is measured at that bar, on the surfaces it
sits on. A token change must re-measure its pairs in the same change.

| Pair | On a card | On the page | On the shell |
| --- | --- | --- | --- |
| light `--text-primary` | 18.7 | 17.9 | 17.0 |
| light `--text-secondary` | 6.6 | 6.3 | 6.0 |
| light `--text-tertiary` | 5.7 | 5.5 | 5.2 |
| light `--st-building` | 5.2 | 5.0 | |
| light `--st-validating` | 5.9 | 5.7 | |
| light `--st-review` | 5.0 | 4.8 | |
| light `--st-ready` | 5.0 | 4.8 | |
| light `--st-needs-you` | 5.1 | 4.9 | |
| light `--st-merged` | 5.4 | 5.2 | |
| light `--danger` | 8.6 | 8.3 | 7.8 |
| light primary button, white on `#1F1F1F` | 16.5 | | |
| light solid danger button, white on `--danger` | 8.6 | | |
| dark `--text-primary` | 14.6 | 16.3 | 17.2 |
| dark `--text-secondary` | 6.4 | 7.1 | 7.5 |
| dark `--text-tertiary` | 4.9 | 5.5 | 5.8 |
| dark lane colours and needs-you | ≥ 5.0 | ≥ 5.6 | |
| dark `--danger` | 4.6 | 5.2 | 5.4 |
| dark solid danger button, `#181818` on `--danger` | 5.2 | | |
| terminal `--term-fg` on `--term-bg` | 16.0 | | |
| terminal `--term-dim` on `--term-bg` / `--term-frame` | 5.8 | 5.1 | |

Lane colours are text only on a card and on the page (a lane's heading).
Their own values for in review, ready and the passive grey are 3.2 to
3.6:1 on white and were darkened; that is the one place the palette is
not theirs.

Do not use `--text-disabled` (2.5 on white) for timestamps or column
headers. Disabled is for controls that cannot be used.

State is never colour alone: every card's status line is a phrase, every
badge a word, and a dot in the sidebar repeats its word in `aria-label`.

### 4.4 Syntax and terminal

Syntax is used inside tool-call JSON, markdown code and diffs. Not as UI
decoration.

| Token | Light | Dark |
| --- | --- | --- |
| `--syn-keyword` | `#7A3DB3` | `#C792EA` |
| `--syn-string` | `#3B6D11` | `#C3E88D` |
| `--syn-func` | `#1D4ED8` | `#93B4FF` |
| `--syn-number` | `#B5471C` | `#F78C6C` |
| `--syn-comment` | `#6B6B6B` | `#8F8F8F` |
| `--syn-const` | `#8A5A00` | `#FFCB6B` |
| `--syn-error` | `#B91C1C` | `#F87171` |
| `--diff-add-bg` | `#16653414` | `#4ADE8018` |
| `--diff-del-bg` | `#B91C1C14` | `#F8717118` |

The terminal is black in both themes: harness TUIs draw for a dark
terminal, and a black panel on a white page is the look itself. Because
the page's accent is black in the light theme, the terminal has tokens of
its own and never uses the page's.

| Token | Light | Dark | Role |
| --- | --- | --- | --- |
| `--term-bg` | `#121212` | `#0C0C0C` | the PTY's background |
| `--term-frame` | `#1F1F1F` | `#1A1A1A` | the outer frame |
| `--term-fg` | `#EDEDED` | same | text |
| `--term-dim` | `#8F8F8F` | same | the frame's chrome line |
| `--term-border` | white at 12% | same | the frame's hairlines |
| `--term-cursor` | `#EDEDED` | same | cursor |
| `--term-selection` | `#EDEDED` at 24% | same | selection |

### 4.5 Radii

| Token | px | Use |
| --- | --- | --- |
| `--r-1` | 4 | badges, small chips |
| `--r-2`, `--r-3` | 8 | buttons, fields, option rows, tooltips |
| `--r-4` | 10 | sidebar rows, board cards, popups and dialogs |
| `--r-5` | 12 | the frame; the terminal's inner frame |
| `--r-6` | 16 | the terminal's outer frame |

The terminal uses the concentric pair 16 / 12 with a 4px gap
(inner = outer − gap). Nothing else is framed that way.

### 4.6 Elevation

Shadows are for what floats. Every popup (menus, selects, popovers,
tooltips, dialogs, the palette) is the reference's popup surface:
`--popup-bg` (white; `#2E2E2E` in dark), an edge of the ink at 2%
(`--popup-edge`), and `--popup-shadow`, `0 4px 12px -4px` of the ink at
5% (black at 14% in dark). Toasts keep `--shadow` (the same offset at
12%, 40% in dark, over a hairline). Cards, rows and tables use hairlines,
never shadows.

A filled button's only depth is inside it (`--shadow-control`): a top edge
lit at 6% and a bottom edge shaded at 10%. No outer shadow, no glow.

`--overlay` (a dialog's scrim): the ink at 45% light, the page's own
black at 85% dark: the reference's. No blur behind it.

### 4.6a Planes

| Token | Value | Layer |
| --- | --- | --- |
| `--z-sheet-scrim` | 50 | a sheet's scrim |
| `--z-sheet` | 51 | the sheet (the narrow sidebar, the narrow session panel) |
| `--z-dialog` | 52 | a dialog and its scrim, the command palette |
| `--z-popup` | 60 | menus, selects, popovers, comboboxes |
| `--z-toast` | 70 | the toast stack |
| `--z-tooltip` | 80 | tooltips |

Paseo's order (`overlay-root.ts`: floating < modal < toast < tooltip):
a dialog opened from a sheet comes over it, a menu inside a dialog over
the dialog, a toast raised while a dialog is open stays readable, and a
tooltip is always on top. Every overlay names its token; no other
`z-index` above 40 is set.

### 4.7 Motion

| Token | Value |
| --- | --- |
| `--ease` | `cubic-bezier(0.23, 1, 0.32, 1)` (the reference's ease-out) |
| `--d-1` | `120ms` |
| `--d-2` | `150ms` |
| `--d-3` | `280ms` |

Animate `opacity` and `transform` only. A popup comes in over 150ms on
`--ease` from opacity 0 and a scale of .98 around its anchor, and goes
over 100ms. A tooltip grows from .95 over 150ms. A dialog: its scrim over
80ms, the dialog over 120ms from opacity 0 and .95.
Inspector and sidebar: width, not a bounce. Tool-call expand: the
`<details>` disclosure, no extra animation. Live status dots do not
pulse unless the state is `starting` or `reconnecting`, and even then
the pulse is opacity 1 → 0.45 → 1 over 1.6s, stopped under
`prefers-reduced-motion`.

---

## 5. Typography

### 5.1 Families

```css
--font-sans: "Geist Variable", Pretendard, -apple-system, BlinkMacSystemFont,
             "Segoe UI", sans-serif;
--font-mono: "Geist Mono Variable", Consolas, "SF Mono", Menlo,
             "JetBrains Mono", "Cascadia Mono", "DejaVu Sans Mono",
             "Liberation Mono", "Noto Sans Mono", Pretendard, monospace;
```

The reference's faces, Geist and Geist Mono (SIL Open Font License 1.1),
bundled from `@fontsource-variable/geist` and `geist-mono` as it bundles
them (ADR 0021); then Pretendard for the scripts Geist lacks, Hangul
first of all; then the system's faces. xterm.js takes the mono stack.

**Mandatory host rule, applied everywhere** (`scripts/check-fonts.mjs`
fails `npm run lint` on a stack that breaks it):

- Pretendard is in every `font-family` and canvas font, ahead of any
  generic family (`monospace`, `ui-monospace`, `serif`); in the mono
  stack, after the real monospace faces. A stack that names Geist names
  Pretendard after it.
- Never put a bare `monospace` or `serif` first.
- Never use Gulim, Dotum, Batang, IBM Plex Mono, or any family that
  renders as a pixel font on Korean Windows Chrome.
- **Bundled only.** Geist, Geist Mono and Pretendard are served from the
  page's own origin (woff2, `font-display: swap`); no font is loaded from
  a CDN. The Latin Geist file is preloaded from `index.html` (Vite gives
  the preload the same hashed name the stylesheet uses), so the first
  paint does not wait on finding it through the CSS.

Two faces and their fallback: no display serif, no third mono.

Weights 400, 500 and 600 only. Text is antialiased
(`-webkit-font-smoothing: antialiased`), titles track −0.01em
(`--tracking-tight`), and numbers that line up in columns or count
(counts, ids, times) are tabular (`font-variant-numeric: tabular-nums`).

Ligatures off on `--font-mono` (`font-variant-ligatures: none`) so
paths and flags do not turn into unfamiliar glyphs.

### 5.2 Scale

Theirs: 10.5, 11, 11.5, 12, 14 and 16px, body at 14.

| Token | Size / line / weight | Family | Use |
| --- | --- | --- | --- |
| `--t-title` | 16px / 20 / 600 | sans | the page's heading in the title bar (tracking −0.01em) |
| `--t-body` | 14px / 21 / 400 | sans | UI copy, markdown body |
| `--t-label` | 14px / 20 / 500 | sans | sidebar rows, emphasis |
| `--t-control` | 14px / 20 / 400 | sans | buttons, inputs |
| `--t-meta` | 12px / 16 / 400 | sans | secondary chrome, helper text; small buttons |
| `--t-kicker` | 11px / 16 / 500 | sans | column headers and small labels, sentence case (never uppercase) |
| `--t-card` | 11.5px / 14.4 / 600 | sans | a board card's title (tracking −0.01em, two lines) |
| `--t-2xs` | 10.5px / 14 / 400 | sans | a card's lines and footer |
| `--t-mono` | 12px / 20 / 400 | mono | ids, paths, models, versions |
| `--t-mono-lg` | 13px / 22 / 400 | mono | a prompt in the timeline, tool I/O |
| (composer) | 14px / 22 / 400 | sans | the prompt being written (agent-orchestrator's composer, §6.15) |
| `--t-term` | 13px / 20.8 (1.6) / 400 | mono | terminal |

Headlines heavier than 600 are forbidden. Markdown in the timeline may
use 600 for `h3`/`h4` equivalents; there is no `h1` inside a session.

---

## 6. Component inventory

A control is a kit component (`web/src/ui/kit/`, §6.13), its look in the
stylesheet beside it; the areas' pieces (cards, timeline, terminal) are in
`web/src/ui/styles/` under the class names below. States to implement:
default, hover, active, focus-visible, disabled, loading, error, empty.
Every state of every control is on `/kit`.

### 6.1 Buttons

| Class | Look | Use |
| --- | --- | --- |
| `.btn` | a quiet grey fill (`#F0F0F0`), no edge | default |
| `.btn.primary` | the ink as a fill (`#1F1F1F`, white text, weight 500) with the inset edge | one primary per view (New, Create, Send, Pair) |
| `.btn.danger` | `--danger` text on no fill, a 12% tint on hover; solid `--danger` only on the confirm step | Kill, Revoke |
| `.btn.ghost` | no fill, secondary text; the sidebar's growing pill on hover | icon buttons, quiet actions |
| `.btn.edged` (variant `outline`) | no fill, a `--border-strong` edge; hover and press fill it | the choices inside a callout with its own fill (Paseo's outline: the file conflict alert's Overwrite and Reload, ADR 0105) |
| `.btn.sm`, `.btn.xs` | 28px and 24px tall, 12px type | row actions, small controls |

Buttons are filled, not outlined (bar `outline`, inside a filled
callout), 32px tall with an 8px radius. Hover
deepens the fill; nothing scales or moves. `shape`: `rect` (8px), `soft`
(10px, a composer's quiet triggers), `round` (a pill, or a circle for an
icon: a composer's Send, Stop and "+"). An `IconButton` may be `primary`
only as a view's one primary action drawn as an icon (Send).

The primary button is the one dark shape on the page; that is why there
is one per view.

Icon + label. Icon-only buttons (`IconButton`) take a `label`, which is
their accessible name and, where the meaning is not obvious, a tooltip's
text. Gap 6px. Radius `--r-2`. Icons are 16px in 32px and 28px buttons,
14px in 24px ones. Busy: a spinner in place of the icon, the width kept,
`aria-busy`. Disabled at 50% opacity, `pointer-events: none`.

### 6.2 Inputs

Text fields, text areas, the composer's box, a combobox. **A field is
one shell**: the shell has the edge, the fill and the focus ring, and the
input inside it draws nothing (no border, outline or fill). The values are
the reference's settings input: `--field-bg` (the ink at 4%) inside
`--field-edge` (black at 8%; white at 3% in dark), radius 8, 14px text,
32px tall (28 small); placeholder `--text-tertiary`; an invalid field's
edge in `--danger`, its message under it. Focus, by mouse or keyboard,
is the shell's `:focus-within`: a 2px `--ring` over its edge (offset
−1px), so a focused field is one box. A text area grows with its text up
to `maxRows`, then scrolls.

Labels sit above, 13px medium in the primary colour, never inside; a
hint (tertiary) or an error (`--danger`) under the control in `--t-meta`
(kit `Field`). A group of choices is a
`FieldGroup` (`role="group"` named by its label, so each radio keeps its
own name).

Selects, segmented controls, switches and checkboxes: §6.13. None is the
browser's.

### 6.3 List rows and tables

`.table` is a `<table>` with hairline row borders, no vertical rules, no
zebra. Column headers are sentence case in `--t-kicker`. Hover
`--bg-hover`. Selected `--bg-active` plus a 2px ink bar on the left.
A list of things with settings or facts (machines, devices, agents) is
not a table: it is the kit's `Row` (§6.14), whose selection is a wash
only.

Columns align: status (dot + word), name (sans), machine facts (mono),
counts (mono, right). Primary click target is the row. Row actions
appear on hover and always in the accessibility tree.

Sidebar `.nav-item`: §2.2. A table's selected row keeps the 2px ink bar;
a sidebar row does not have one.

### 6.4 Status badges

`.badge` + modifier. Height 18px, padding 0 6px, radius `--r-2`,
11px / 500, a `--border-subtle` outline and no fill, text
`--text-secondary`. A status badge puts a 6px dot of the status colour in
front of its word; only `failed` also colours the word. Word always
present. Tinted chips are not used: on a white page a row of pastel fills
reads as decoration and buries the one that matters.

| Badge | Colour | Protocol |
| --- | --- | --- |
| `online` / `serving` | `--online` | node reachable, `NodeState::Serving` |
| `draining` | `--warn` | `NodeState::Draining` |
| `offline` | `--text-tertiary` | book entry, no connection |
| `starting` | `--warn` | `SessionStatus::Starting` |
| `running` | `--online` | `SessionStatus::Running` |
| `exited` | `--text-tertiary` | `Exited` (include code if any) |
| `failed` | `--danger` | `Failed` |
| `interrupted` | `--interrupt` | `Interrupted` |
| `home` | ink, `--border-strong` outline | fleet role |
| `member` | `--text-secondary` | fleet role |
| `direct` / `ssh` / `relay` | secondary, mono | how this client reaches it |
| `conflict` | `--warn` | member has `.vessel-incoming` |
| `terminal` / `structured` | secondary | `SessionMode` |
| `stale` | `--warn` | applied generation < home |

### 6.5 Timeline items (structured)

A vertical 1px rail in `--border-subtle`, 12px from the left. Each
item is a row with a 8px glyph on the rail:

| Event (`AgentEvent`) | Glyph | Body |
| --- | --- | --- |
| `TurnStarted` | filled accent tick | the prompt, mono 13px, on `--bg-elevated`; the files it carried over it (below) |
| `UserMessageChunk` | same | resume replay; muted, labelled "replayed" |
| `MessageChunk` | empty circle | markdown, sans 13px, max 78ch |
| `ThoughtChunk` | dashed circle | collapsed `<details>`, tertiary |
| `ToolCall` | square by `ToolKind` | `.tool-card` card, §6.6 |
| `ToolCallUpdate` | (same card, in place) | do not append a second card |
| `Plan` | checklist | inspector, and a compact strip in-timeline |
| `PermissionRequested` | amber diamond | one line, "Waiting on permission · <tool>", in the review colour; the prompt itself is docked over the composer (§6.7) |
| `PermissionResolved` | the diamond, hollow | the same line, in place: "Allowed · <tool> · <option>", "Declined · …" or "Permission cancelled · …" |
| `Steered` (protocol 10) | the accent tick, hollow | the words sent into the running turn, in the prompt's box, led by "steered" in meta, its files over it |
| `QuestionAsked` | amber diamond | one line, "Waiting on an answer · <message>"; the question itself is docked over the composer (§6.7) |
| `QuestionResolved` | the diamond, hollow | the question's card: its message, each answer by its field's title ("Database SQLite"), Claude's free text joined to its question's line, and "Answered", "Skipped" or "Question closed" |
| `TurnEnded` / `TurnFailed` | rule | reason in meta; with protocol 10's usage, the turn's tokens after it: "30 tokens · 20 in · 10 out" |
| `Ready` | meta | "Agent ready"; the agent's own session id is in the summary's Details |
| `HarnessLog` | meta | stderr, mono, tertiary; error colour if it looks like a login failure |

User and agent are not two chat bubbles on opposite sides. This is a
log, left-aligned, like an editor's output channel, in a column at most
768px wide and centred (agent-orchestrator's transcript width), so a
wide window does not stretch a line past reading.

Only the rows within 1,200px of the view are drawn (milestone 11,
`use-window.ts`); the space of the others is kept by two spacers that
carry the rail, so it runs unbroken across the drawn rows' edges while
the next ones come in. With "Collapse tool calls" a run of tool calls is
one row (`data/rows.ts`). The log follows its end while the reader is
within 48px of it; a reader who scrolled up, or was taken up by find or
the outline, stays where they are as records arrive. New entries are
announced from a live region of the newest one's text.

**A prompt's files** (protocol 10, `prompt.attachments`; agent-
orchestrator's attachment row over the message): an image is a tile
64px tall, its own width, radius 8, that opens the image at full size in
a dialog; any other file a card 8px inside on the hover wash, a 24px
icon box, the name (12px medium, cut at 128px) and "type · size" under
it. An image is fetched with `attachment.read` only when its row comes
within a screen of the view, two at a time, and kept (48 images or
64 MiB, the least recently drawn dropped, `data/images.ts`); one sent
from this tab is shown from its own file and never fetched back. The
page's content policy takes `data:` images and no `blob:` ones
(ADR 0017), so a tile shows a 128px-tall `data:` thumbnail drawn from
the bytes, and the full image is read into a `data:` address only when
it is opened.

Markdown: the sans face, 13/20. Inline code and fences use `--font-mono`
on `--bg-elevated` with a hairline. Headings inside a turn start at
the `h3` equivalent (14px / 600). Lists and blockquotes get a 1px
accent-left only for blockquotes.

### 6.6 Tool-call cards

`<details class="tool-card">`, one box: the timeline's row around it
(`.t-item.tool`) draws no border of its own, and the card's class is not
the row's, so no rule for one can paint the other (they once shared the
name and the card sat in a second box). Summary: kind icon, title, status badge,
location path (mono, truncated from the left), then a chevron that turns
while open; the browser's triangle is not drawn. (A command's icon is the
terminal's mark, not a run triangle: beside the chevron a triangle read
as a disclosure marker.) Body: input then output,
each a labelled `<pre>`. Default **collapsed** when `completed` or
`failed`; **open** when `pending` or `in_progress`.

Kind icons (stroke, 14px): read, edit, delete, move, search, execute,
think, fetch, other. Colour follows status, not kind — kind is the
shape, status is the colour. Otherwise eight rainbow tools fight the
accent rule.

Diffs (`ToolCallContent::Diff`): line-numbered, add/delete backgrounds
from the syntax table, `select-none` on the gutter. Horizontal scroll,
no wrap.

JSON input/output: 12px mono, comment colour for keys if you highlight;
plain is acceptable in v1. Copy button on the summary, `aria-label="Copy tool output"`.

### 6.7 Permission prompts

A pending `PermissionRequested` is the most important thing on the
screen, and it is asked once. (Milestone 9 drew it twice, a card in the
timeline and a bar over the composer, each with every option as a
button; the owner asked for one.)

- **Docked over the composer**, 8px above the box and wholly clear of
  it (Paseo keeps the pending card fully visible above its composer;
  GOAT's drawer tucked 16px under the box was covered once the box
  stayed usable during a permission, TRACKER 794): 12px narrower than
  the box on each side, radius 16, an edge of the review colour at 28%
  and a tint of it at 8%. In it: a 16px shield in
  the review colour; "Approval needed" (11px medium, that colour); what is
  asked, the tool call's title, in 12px mono, and the file it touches;
  then Decline and Approve, `xs`, Approve the primary.
- **The agent's words.** The buttons carry the agent's own names for its
  options. Approve is its first allow-once option, else its first allow;
  Decline its first reject-once, else its first reject; any other answer
  is under "More" (`data/approval.ts`).
- **One line in the timeline**, in the review colour while it waits, that
  becomes the outcome once answered (§6.5).
- First answer wins (protocol). Another follower's answer clears the
  prompt here too, and the line says how it ended. A later click errors
  with conflict: a toast says so, the history stays.
- The composer under it stays usable while it waits (Paseo's; TRACKER
  794): a message then goes by the delivery rules below, a Queue
  stopping the turn instead (it would be stranded behind the request),
  and the placeholder says so. On a node before protocol 10, where
  nothing can be sent during a turn, it is disabled, its placeholder
  "Answer the permission to continue".

**A question the agent asks** (protocol 10, `question_asked`) takes the
permission's place, one at a time, as agent-orchestrator's question
dock does (`question-dock.tsx`):

- 16px narrower than the box, radius 20, the box's surface on the strong
  hairline, 8px above it, at most 65% of the window tall before it
  scrolls. A 12px muted head, "Question · 1 of 2", with a chevron that
  folds the dock to its head; the agent's message (12px muted); the
  field's title as the question (14px semibold).
- **A step per field** (`data/question-form.ts`): a choice is a list of
  rows, each with an 18px number badge, its title (12px medium) and its
  description (11px muted), the picked ones on the 7% wash with the
  badge filled; digits pick a row (agent-orchestrator's); several picks
  when the field takes several. Text and numbers are the kit's field,
  yes/no a switch. Claude's `AskUserQuestion` sends `question_N` and
  `question_N_custom`: the pair is one step, the custom field its
  "Other…" row (badge 0) with "Type your own answer (optional)." under
  it; Claude's questions are ordered by N (the node keeps a form's
  fields by name, so `question_10` would come before `question_2`).
- **The foot**: Back (from the second step), 6px step dots (current in
  the ink, done in the accent), Skip, and Next or, on the last step,
  "Submit answer" (the primary pill). Next checks the step with the
  node's own rules first ("Choose an answer.", "This field is
  required."); a refusal the node still sends (`invalid_request` naming
  a field) is shown at that field and takes the dock to its step. Skip
  declines the question.
- The answer settles it for every follower; one settled elsewhere
  closes the dock here, and the timeline's card says how it ended.
- The composer under it stays usable: a question is part of the turn,
  so a message typed now is queued (§6.15).

### 6.8 Terminal frame

Outer `.term-frame` radius 16, inner `.term-inner` radius 12, 4px gap,
both `--term-border` hairlines, `--term-frame` outside and `--term-bg`
inside: black in both themes (§4.4). Chrome 32px in `--term-dim`: node ·
session prefix · cols×rows · encoding. The PTY itself is full-bleed
inside the inner frame.

Font: `--font-mono` at 13px. Cursor `--term-cursor`, selection
`--term-selection`: the terminal's own tokens, since the page's accent is
black in the light theme. xterm.js takes them as its theme.

### 6.9 Dialogs

The kit's `Dialog`, the reference's base dialog: centred, 520px (Paseo's
adaptive modal card; `lg`, 640px), the popup surface (§4.6) at radius 10,
16px inside. Title 15px /
600; a ghost close button at the top right. Footer right-aligned, ghost
Cancel then one primary. The scrim is `--overlay`, no blur. Base UI
gives `role="dialog"`, `aria-modal`, the label, the focus trap, the
scroll lock and the inert page. Focus moves in on open (the first field
in the body, else the last footer button) and back to the opener on
close. The command palette (§6.13) is a dialog on the same scrim.

A dialog with a second view or a list to search is paged (Paseo's sheet
header; the label manager): an optional back arrow before the title, and
an optional search under the title, unboxed (a 16px muted search icon, 8px
to the text, no fill, no clear button). Its header sits on a rule, and its
footer on another. A paged dialog with a search keeps focus on itself when
it opens, as Paseo does not focus the search.

Dimmer click and Escape cancel, unless a pairing invite is live and
uncopied — then confirm.

Compact (below 600px, §3) a dialog is Paseo's `AdaptiveModalSheet`
(TRACKER 787): a sheet from the bottom as wide as the window, its top
corners at 16, a 32 by 4 handle 8px under its edge, over black at 45%;
65% of the window tall, its footer at its foot. A drag on the handle or
the header raises it to 90% (past halfway, or flung up), lowers it to 65%,
or closes it (well below 65%, or flung down from there); a text field
taking focus raises it to 90% (Gorhom's keyboard "extend"). It takes focus
itself on open, not its first field, so no keyboard opens until a field is
pressed. `sheet="content"` is Paseo's `ContextWindowSheet`: as tall as
what it holds, at most 80%; the context ring opens its figures in one at a
compact width (a press; the hover card elsewhere).

### 6.10 Toasts

Centred below the header, 8px under it, as Paseo's app-shell host
(`toast-host.tsx`: safe area + header + spacing 2), so feedback stays clear
of the lower composer and question actions. The host's bottom edge is
anchored to the composer's dock and a prompt menu over it (CSS anchor
positioning), so no viewport change or growing dock leaves a frame in
which a toast covers a question control; browsers without anchors keep the
measured height (`use-toast-clearance.ts`), one rendering step late. The
card sits in a boundary 92% as wide as the window less 16px a side, at
most 480px, and is as wide as its words within it; clicks pass through
the host around it.

One toast at a time, as Paseo's `ToastHost` (`toast-host.tsx` 81-130): a
new toast replaces the one up and starts its own life (`ui/toast-life.ts`):
2.2s (Paseo's `DEFAULT_DURATION_MS`), an error 3.2s, a `durationMs: null`
toast until the call `toast` returned is made; dismissing an older toast
leaves the newer one. A mouse resting on the card holds the life it has
left, and leaving runs the rest (Paseo pauses its timer on hover).

The card is Paseo's `ToastCard` (`toast-host.tsx` 230-275, 302-333): no
close button; a leading 18px mark per variant (success `circle-check` in
the primary colour, a copied toast's in the text colour, info `info` in
blue-300, warning and error `warn` in amber and the destructive red; the
default variant none), the message 14px in the text colour, 8 by 12
inside, radius 16, `--bg-elevated` with a hairline that info, warning and
error colour, the medium shadow. The page's `toast()` defaults to success
(it reports a finished action); a notice that is neither passes `default`.
The host occupies at most a quarter of the viewport height; a long
message wraps and scrolls. Padding preserves room for the card's shadow.
`role="status"`, `role="alert"` for warning and error.

### 6.11 Empty, loading, error

| State | Treatment |
| --- | --- |
| Empty list | 48px top margin, 16px stroke icon, title 13/600, one sentence secondary, one primary action |
| Loading | 3 skeleton rows, 12px high, `--bg-hover`, no shimmer under reduced motion; otherwise a 1.2s opacity pulse |
| Error | banner at the top of main, `--danger` hairline, message from the protocol, Retry |
| Offline node | the node still lists; sessions pane empty-state "gpu-box is offline" with last-seen |
| Reconnecting | 28px bar under the session header, `--warn`, "Reconnecting to gpu-box · last byte 4s ago". Terminal stays visible, frozen. On resume, the bar becomes a 1.2s "Caught up from seq 1842" status then leaves |

Empty copy is specific ("No sessions on studio") not generic
("Nothing here yet").

### 6.12 The board (Sessions)

The Sessions page is agent-orchestrator's board (ADRs 0018 and 0019): its
lanes, its card, its archive, with Vessel's facts on the card.

**The node decides.** Each session's lane and the phrase on its card are
derived by its node and sent as `SessionInfo.board` (protocol 8): the same
reducer as agent-orchestrator's daemon, ported to Rust. The page shows
what it is sent. `data/board.ts` `placement` is the only place a session
gets a lane, and every lane and phrase is matched exhaustively.

A node older than protocol 8 sends no placement. Its live sessions are in
Building and its ended ones in the archive, and their cards read the
protocol's status word (`running`, `exited 0`), since nothing says what
the agent is doing.

**Lanes**, in delivery order:

| Lane | Column | Colour | What is in it |
| --- | --- | --- | --- |
| Building | `building` | `--st-building` | a live session with no pull request yet |
| Validating | `validating` | `--st-validating` | a pull request in a loop the fleet drives |
| In review | `needs_review` | `--st-review` | a pull request whose next turn is a person's |
| Ready | `ready` | `--st-ready` | merged, closed, mergeable or approved |

- Until a node gathers pull requests (the next milestone) every live
  session is in Building; the other three lanes are there and empty.
- The lanes are one grid divided by 4% hairlines, with one hairline under
  the headings. They are not four rounded panels.
- A lane's heading is 48px: an 8px dot and the name, both in the lane's
  colour (12px medium), the count at the right in tertiary.
- Inside a lane, sessions that wait on their owner come first, then the
  most recently active. Each lane scrolls by itself: 12px of padding (8px
  on the right), 10px between cards.
- **The archive is not a lane.** Archived sessions are counted on a 36px
  bar under the lanes; the count opens them as the same cards, three to
  a row, newest first, with agent-orchestrator's search ("Search
  archived sessions"; "No archived sessions", "No results."). An
  archived card reads its own status word, and a failed one keeps
  `--danger`. With protocol 10 an ended session stays in its lane, its
  card reading its status word, until it is archived; from an older
  node every ended session is in the archive, as before.

**Card.** One session. In order:

1. *Origin*, only when it was not started with the owner token: "agent of
   `3f2a`" (a link; "on gpu-box" when the parent runs elsewhere), or
   "device `ab12cd`".
2. *Title row*: the agent's mark (a letter in an 18px tile: C for Claude
   Code, X for Codex, G, O; the kit's Avatar), then the title, 11.5px
   semibold, two lines at most: the session's own (protocol 9,
   `SessionInfo.title`: asked for, the first line of its first prompt, or
   renamed), else the last segment of its working directory.
3. *Where*: the mode's icon and the working directory, mono 10.5px, cut
   from the left so its end stays.
4. *Who*: the id's first four characters, then the profile (or the
   agent's name) and the model when there is one.
5. *Status line*, over an 8% hairline: the phrase in the lane's colour
   (10.5px medium), with a spinner while work is turning; at the right the
   node (omitted when the board is scoped to one) and how long ago
   ("6m ago", "3h ago", "5d ago"; never weeks).
6. *Failure*, only for `failed`: the node's message, in `--danger`.

- The phrases are agent-orchestrator's, as its daemon writes them:
  "Working", "Blocked", "Awaiting PR", "Fixing CI failures", "Needs human
  review", "Mergeable" and the rest, plus "Idle" for a session between
  turns on a node that cannot see pull requests.
- A phrase can say more than its lane: "Mergeable" and "Approved" are in
  the ready colour, "Merged" in the merged colour, "Closed without merge"
  in `--danger`.
- **Needs you.** A card whose phrase is "Blocked", "CI failing" or
  "Changes requested" waits on its owner: its edge and its phrase take
  `--st-needs-you`, its fill an 8% tint of it, and one overlay of that
  tint breathes on a 3.5s cycle. Under reduced motion the tint holds
  still. Nothing else on the board moves.
- The card is the one object with an edge of its own (principle 10):
  8% hairline, 10px radius, white. Hover lays the ink at 4% over it. No
  shadow, no coloured strip.
- The whole card opens the session. Its link lies under its content, so
  the origin link and the action stay separate targets and nothing
  interactive is nested in an `<a>`.
- One action slot at the end of the title row, shown on hover or focus
  without moving the title: Kill, for a live session (it asks first);
  Archive, for an ended one (it asks as agent-orchestrator does: "Are
  you sure you want to archive this session?", "You can always restore
  it from the Archive section later.", No and Confirm; on a machine of
  protocol 11 it says first that "The terminals and services running
  beside it end too; their output is kept."); in the archive,
  Restore and Delete permanently. Restore starts the session again where
  its agent can (`session.resume`, which unarchives it), else it only
  unarchives it ("Unarchive").
- **Marks on the card** (protocol 10): a 12px pin before the action slot
  of a pinned session; an unread one's title in the primary ink with
  agent-orchestrator's filled dot.
- An empty Building lane, where a new session lands, offers "New
  session" (a quiet button) to `/new`.

### 6.13 Controls (the kit)

`web/src/ui/kit/`, exported from its `index.ts`. Behaviour (focus,
typeahead, placement, dismissal, ARIA) is Base UI's; the look is the CSS
beside each component, from tokens only. Screens import controls from
the kit and never from `@base-ui/react` (lint holds both). Props are
closed unions: a variant, a size, a choice's value type `V` that comes
back typed, and props that only make sense together as one discriminated
union.

**Nothing the browser draws.** No `<select>`, `<input>` or `<textarea>`
outside the kit, and no `title` attribute (the browser's tooltip):
`scripts/check-controls.mjs` fails `npm run lint` on one. The rest is
CSS: no disclosure triangle on `<details>` (a chevron icon turns
instead), selected text in `--selection`, scrollbars in `--scrollbar`
(thin inside popups), no default checkbox or radio glyph, and focus as
the design's ring (§8).

Sizes, from the reference: controls 32, 28 and 24px tall (`--h-md`,
`--h-sm`, `--h-xs`), radius 8 for controls and 10 for popups.

| Part | Anatomy and values |
| --- | --- |
| Popup surface | shared by Select, Menu, Combobox, Popover, Tooltip: `--popup-bg`, radius 10, 4px padding, `--popup-edge`, `--popup-shadow`; at most 320px wide and the window less 32px; long lists scroll inside under a thin bar; motion §4.7. Placed below its trigger 6px away, flipped and shifted at the window's edge (8px from it) |
| Option row | 13px / 1.5 (about 32px), 8px by 6px, radius 8; secondary text at rest; highlighted on the 4% wash in the primary colour; chosen or open on the 7% wash; a muted detail at the right (12px, tertiary, cut first); a group's heading 10px, tracked, tertiary. A row reached from the keyboard also takes the ring, inside it |
| Select | trigger: the secondary button's raised surface (an edge 5% darker, the inset bevel), 13px, the value then its detail, a chevron that turns over 300ms while open; `ghost`: no fill until hover. `md` 32px, `sm` 28px; `fill` or `fit` width. The list is at least the trigger's width, below it, not over it |
| Menu | the option rows with a 15px icon, a shortcut at the right (10px, tracked), a description under the label (320px, `wide`), checkbox rows (16px box, radius 6), radio groups (a check at the right; values that share a group under its heading), submenus (the current value and a chevron; the flyout opens to the right of its row, 5px over it, top-aligned, clamped into the window and never flipped, as Paseo's), separators, a danger row; `narrow` is 256px (a composer's agent menu). `MenuSwitchItem`: a setting's row with a switch at the right, 12px text, 8px by 12px, keeping the menu open; `MenuNote`: a line of tertiary text that is not a choice |
| ContextMenu | the Menu's rows and popup opened at the pointer by a right click on the thing it acts on, or a touch held 500 ms (Paseo's ContextMenu); `actions` is 220px (Paseo's file actions menu). The Files tab's rows open Paseo's file actions: Copy path, Copy relative path, Download, Add to chat for a file; Collapse folder (its own group) and the copies for an open folder; a hairline between groups, Delete red |
| Combobox | a field shell (§6.2) whose suggestions are option rows, at least the field's width; it takes any text. `ghost`: no fill or edge at rest, the 4% wash on hover, the shell while focused (a composer's tray) |
| Popover | the popup surface with 16px inside, 256, 320 or 384px wide (GOAT's), a 13px semibold title |
| Tooltip | 12px on the popup fill, no edge, 6px by 12px, radius 8, a 10px arrow; after 400ms of hover or on keyboard focus; a shortcut after the words. Never takes the pointer |
| Switch | a 32 by 16 track, inset shading only, a 14px thumb sliding 16px; off: the ink's thumb on the 4% track; on: the primary fill and a thumb in its text colour. A hit area 12px past each side |
| Checkbox | 16px, radius 6, the strong hairline on the field fill; checked: the primary fill with a 12px check |
| Segmented | a radio group: a track on the field fill and edge, 2px inside and between, radius 12; 13px segments 32px tall at radius 10 (`sm`: 24px); the chosen one is GOAT's thumb, the surface lifted by a hairline. Icons alone only with every option's icon (its label is the name) |
| Tabs | 14px medium labels 36px tall and 20px apart over a hairline; the chosen one in the ink over a 2px underline that slides to it |
| PanelTabs | agent-orchestrator's inspector strip: a 36px head, 28px icon tabs (14px icons) 4px apart, radius 8, the passive ink; hover and focus the 4% wash, the chosen one the full ink with no fill; a count on a corner (a 12px circle, 7px mono); each icon's name its tooltip and accessible name; arrows, Home and End move |
| SplitButton | GOAT's git control: an action and a chevron that opens its menu, one shape (square inner corners, a hairline between); the action's reason in a tooltip when it is off |
| Badge | §6.4; `word`, `accent` or `mono` |
| Avatar | an agent's mark: a letter in an 18px tile, radius 5, the well's fill, 11px semibold |
| Collapsible | agent-orchestrator's inspector section: a 14px title in the muted colour with an optional count, a 12px chevron that turns while open, the sidebar row's growing pill (radius 10); the body 12px in, grows over 220ms (none under reduced motion) |
| Kbd | as text (11px, tertiary, the sidebar search's), or a key chip: 20px, radius 4, 12px medium on the 7% wash |
| Spinner | 12, 14 or 16px, a ring with a quarter gap, once a second; still under reduced motion |
| CommandLine | GOAT's setup command: the command in mono on the field's fill and edge, radius 8, and a ghost "Copy command" button that says "Copied" on itself for 1.5s (a toast would be under a dialog's scrim) |
| EmptyState, Banner, Skeleton, StatusDot | §6.11 and §4.3 |
| Sheet | §6.17: a dialog from the left edge (the sidebar's on a phone), or the right (a session's panel) |
| TextArea `fill` | fills its container and scrolls inside, without measuring its text on each key (the file editor) |
| TextField `look="popup"` | the field shell on the popup surface, its edge and shadow (the find bar's, §6.16) |
| Command palette | a dialog (§6.9) up to 720px wide, 14vh from the top, radius 14, solid where the reference's is glass: a 16px search line, 14px rows 28px tall inset 6px, the highlighted one on a 9% wash, at most 420px of rows in groups, a foot of keys. `⌘K` or `/` (not while typing) |

Icons are the kit's inline set at the reference's stroke and sizes: its
1.5 on a 24-unit grid is 1 on this set's 16-unit grid (about a pixel at
16px); 16px in buttons and tabs, 15 in menus, 14 in the composer and the
densest buttons. One name per drawing, named after what it draws where a
purpose would fit two uses (`monitor` is the System theme and a device;
`search` is the palette's field and a search tool call; `plan` is a plan,
a thought and the Models section). `ICON_NAMES` lists them once; the type,
the gallery and a test that each has its drawing, and no drawing two
names, all read that list.

**The gallery.** `/kit` shows every control in each state and size, with
long and Korean text. It needs no token and nothing links to it;
`npm run screens` captures it light and dark, at full width and at 390px,
with each popup open from the keyboard. A new control goes into the kit,
with its states on `/kit`, before a screen uses it.

### 6.14 Rows

The kit's `RowList` and `Row`, for machines, devices, agents and settings:
agent-orchestrator's settings card with GOAT's machine row inside it. The
list: radius 8 on the card fill, no border, 4px above and below, rows
divided by hairlines. A row, 10px by 16px: a mark (a 16px icon, or a
state dot), the title at 14px medium, under it a 13px description or a
12px line of facts joined by " · ", then the control (a switch, a badge)
and a menu at the right. **Chosen is the 7% wash, never an edge or a
bar.** Settings' machines, agents, devices and facts are rows.

### 6.15 The composer

`features/compose/prompt-box.tsx`, the same box on the new-session page
and under a running session's timeline. agent-orchestrator's
`.cursor-chat-composer` and `TurnSettingsBar`, with GOAT's docked
approval (§6.7):

- **The box**: radius 20, 12px inside, 6px between the text and its row
  of controls. Light: the white surface held up by a hairline (the ink at
  8%) and a soft shadow (`0 1px 2px` at 4%, `0 8px 24px -10px` at 10%);
  dark: the card's fill (`#222`) with neither. A click on its padding
  puts the caret in the text.
- **The prompt**: 14px sans on a 22px line, three lines at rest, growing
  to seven, then scrolling; no shell of its own (the box is the shell).
  Enter sends, Shift+Enter starts a line, ⌘ or Ctrl with Enter sends too;
  with Settings › General's ⌘/Ctrl+Enter, Enter starts a line and only
  ⌘/Ctrl+Enter sends. The key that ends an input method's composition
  never sends. What is typed is a draft the bridge keeps, one per
  session and one for the new-session page, saved half a second after
  typing stops and when the tab is hidden: leaving, a reload or a new
  launch brings it back; sending clears it. In an empty composer Up
  recalls the prompts sent in the session, newest first, Down walks
  back and past the newest empties it; within a recalled prompt of
  several lines the arrows move the caret until its first or last line,
  and editing it ends the walk (Paseo's and GOAT's rules). The
  new-session page has no history.
- **The row**, 28px: on the left the "+" (on the new-session page a
  round ghost button opening "Start with": a profile, when the machine
  has one for the agent, "Open as terminal" with its arguments, and
  "Attach files to this message"; on a running session, from protocol
  10, the paperclip that picks files, "Attach files"), the context meter,
  the agent control, the permissions control; at the right the one
  round 28px primary button (below).
- **The agent control** (`AgentModelPicker`): a quiet trigger (28px, radius
  10, 12px) reading the model's name and, muted, the effort's; "Loading
  models…" with a spinner while the machine asks the agent. Its menu,
  256px, opens upward: Agent, Model and Effort rows, each with its
  current value and a submenu of the agent's values (under their groups'
  headings); then, under a rule and a heading that says whose they are
  ("Claude Code settings"), the agent's on/off settings as switch rows
  and any other choice it offers as its own submenu. A row the agent does not offer is not there. A failure is
  said in the menu and under the box, and the session starts on the
  agent's defaults.

  The options under that heading keep the agent's own names, with one
  exception: a name the page already uses for a row of its own (Agent,
  Model, Effort, Permissions) is said with whose it is. Claude Code calls
  one of its options "Agent" (which of its own agent definitions runs the
  session); beside the page's "Agent" row (which program runs it) it
  reads "Claude Code agent". Only the name shown changes, and an agent's
  card in Settings shows the same name (`ownName` in `data/options.ts`).
- **The permissions control**: a quiet trigger led by a shield, the
  mode's name and a caret (the one control in the row that reads as a
  select); its 320px menu opens upward, aligned to its right edge, a row
  per mode with a shield, the agent's name for it, its description, and a
  check on the one in effect.
- **What the controls show** is the agent's own, by category
  (`data/options.ts`): `model` the model, `thought_level` the effort,
  `mode` the permissions; toggles are switches; anything else a submenu;
  an option of a kind the page does not know is skipped. On a new
  session they show the agent's values with the person's picks applied,
  and only picks that change something are sent. On a running session
  they show what its log last reported (`options_changed`): a change
  shows when the node reports it, not when it is asked; a refusal is a
  toast and nothing moves.
- **The send button** (agent-orchestrator's, `send-button.tsx`): between
  turns an arrow, "Send". While a turn runs and the text is empty, a
  square, "Stop turn" (it stops that turn, `cancel_turn`, and the
  node's queue holds). While a turn runs with text: what Settings ›
  General's "Default send" says (Paseo's setting, `composer.delivery`,
  kept by the bridge): "Queue message" (an arrow), "Steer message" (a
  turning arrow, the default since ADR 0108, as Paseo's and the native
  app's) or "Interrupt and send". The other way is
  one modifier away: ⌘/Ctrl with the click, or ⌘/Ctrl+Enter
  (⌘/Ctrl+Shift+Enter where ⌘/Ctrl+Enter is the send key), and the icon
  and label turn while the modifier is held; the other way of queue is
  interrupt-and-send (Paseo's "Command/Ctrl+Enter submits"), of steer
  and interrupt queue. Settings › General says so under Default send,
  in Paseo's words for the send key. Steer is offered only where the
  agent says it takes it (`abilities.steer`), and an agent that refuses
  it once is not offered it again while the session's page is open; a
  Steer chosen for an agent that cannot take it replaces the turn
  (Paseo's `steerOrReplaceActiveTurn`, as the native app), and a Queue
  while a permission waits interrupts, each with a line under the label
  that says why (TRACKER 794). Its tooltip says what
  it does and the other way: "Sends when the agent finishes its turn",
  "Ctrl+↵ or Ctrl+click: steer message". With a queued prompt open in
  the box it is "Update queued message". A held queue (after Stop) does
  not hold the next message: it starts the next turn.
- **The node's queue** (protocol 10, agent-orchestrator's queued-message
  dock, `queue-dock.tsx`): over the box, 16px narrower on each side, 8px
  above it, radius 20; a 32px head "2 Queued Messages" (12px medium,
  muted) with a chevron that folds it; a 40px row per prompt, a hollow
  12px circle and its first line, its files' count, then Steer (Send now
  where the agent cannot steer: between turns it starts the next turn,
  during one it interrupts it), Edit, Delete and a grip, at most five
  rows before it scrolls. The grip moves a row by drag (a 2px line shows where it will
  land, GOAT's) or by Up and Down. Edit opens the prompt in the box with
  "Editing queued message" and "Cancel edit" over the text; saving sends
  `edit` with the revision the row had, so an edit that crossed another
  says so instead of overwriting it. While the node holds the queue the
  head says why ("Paused after you stopped the agent", "Paused after the
  turn failed", "Paused: the session ended") with Resume.
- **Files** (protocol 10, where the session's agent takes them): pasted,
  dropped on the box (it tints while a file is over it) or picked with
  the paperclip. Each is a chip over the text (agent-orchestrator's: the
  4% wash, radius 8, an image's 24px thumbnail or a file's icon, the name
  at most 120px, "type · size" under it), with Remove, and, while it
  uploads, GOAT's band along its foot and "Uploading 40%". A file is
  checked as it comes against what the agent takes and the machine's
  limits (`session.conversation`'s `limits`: per file, per image, per
  prompt, per session), with the node's own words for a refusal ("Claude
  Code does not take images in this session."); it is uploaded when the
  message is sent, two at a time, in 1 MiB pieces, and the message goes
  when every file is in. A failed upload says why in the chip, with
  Retry where trying again can help, going on from what the machine has.
  A refused or failed file stops Send until it is removed or retried.
  On the new-session page the files wait for the session, and its first
  message takes them once the agent has said what it takes.
- **`/` and `@`** in one list at the caret (agent-orchestrator's
  suggestion menu, the kit's `Suggestions`): anchored 6px over the box's
  top-left edge, at most 448px wide, up to 50 rows. `/` at the start of
  the prompt lists the agent's own commands (`commands_changed`), names
  that start with the word first, then names that hold it, then
  descriptions; `@` at the start of a word finds the session's files
  with `workspace.find` in its worktree or project (80 ms after typing
  stops, one search in flight), the folder dimmed and the matched
  characters marked. Up and Down move, Enter or Tab inserts (`/review `,
  `@path`, a path with a space in quotes), Escape hides it until the
  word changes. A session with no project says "This session has no
  project to search."
- **The context meter** (agent-orchestrator's, `context-ring.tsx`): a
  20px ring in a 28px box, the arc from the top, in the muted ink, the
  review colour from 70% and the danger colour from 90% full; its
  tooltip "Context window", "84k / 200k tokens (42%)" and, when the agent
  says, "Session cost $0.50". It shows only once the agent has reported
  its usage (`usage` in the log); the figures are the agent's own.
- **An ended session** (protocol 10): a banner in the docks' shape over
  the box, "Session ended" (or "Session archived"), "History is kept.
  Start the session again to continue." and Start again where the agent
  can (`session.resume`); the box disabled, its placeholder "Start the
  session again to continue".
- **The placeholder** says what Enter will do: "Message the agent, tag
  @files, or use /commands" between turns, "Next message you send will
  be queued" while a turn runs (agent-orchestrator's words).
- **The tray** (new session only), hung under the box, 16px in from each
  side, 6px inside, its bottom corners 16px: white on a 6% hairline in
  light, the ink at 3% in dark. It holds the machine (a ghost select,
  only connected machines enabled) and the folder (a ghost combobox
  suggesting the folders sessions ran in on that machine, newest first,
  taking anything typed; empty is the machine's home). On a machine
  where a session was started before, the page starts with the agent,
  profile and options that one had (kept per machine with the bridge;
  Paseo remembers the same).

### 6.16 Find in a session, and the outline

`features/session/find-bar.tsx` and `outline.tsx`, over the session's
model (`data/find.ts`, `data/outline.ts`), reaching the timeline only
through its one seam (`reveal.ts`).

- **The find bar** (agent-orchestrator's): `⌘F` in a session, over the
  log's top right, 12px in, 320px wide; one field on the popup surface
  holding a search icon, the query, the count ("3/12"), a divider and
  three 24px buttons (previous, next, close). Enter goes to the next
  match and Shift+Enter to the previous, wrapping (said to a screen
  reader); Escape closes and gives focus back. It searches the prompts'
  and messages' text, folded for case, accents and runs of white space,
  never the page's DOM, so a match scrolled out of a long log is still
  found; the search settles 100ms after typing, runs in slices that give
  the page back every 200 entries, and while the session grows runs
  again at most every 300ms, never cancelled by records arriving (a
  refresh that finds the same match leaves the view where the reader
  put it). The match is marked, by the same folding as the count, with
  the CSS Custom Highlight API in the reference's
  colours (yellow, the current one orange) and put in the middle of the
  view; focus stays in the field, as agent-orchestrator's does.
- **The outline** (GOAT's minimap, Paseo's outline): from two prompts
  on, a rail on the log's left edge, a 24 by 2px mark per prompt, darker
  while that prompt is on screen. Hovering or focusing a mark shows a
  320px card with the prompt and the start of the reply to it; choosing
  it puts the prompt 16px under the view's top and gives it focus (its
  box takes the focus ring when the keyboard chose it). With the summary
  open the rail moves to the edge and its marks shorten; below 1100px of
  window it is not drawn.
- **Far away.** A match or a prompt may be thousands of rows from the
  drawn ones. The window goes to where its row is estimated to be, so it
  is drawn in the same frame, then holds it where it was put while the
  rows around it are measured (`window-pin.ts`): no frame shows it
  sliding. A wheel, a touch, a press, a key or any other scroll gives
  the view back to the reader.

### 6.17 The sheet

The kit's `Sheet`: a dialog from the left edge, full height, the window
less 12px and at most 320px wide, on the sidebar's surface over the
dialog scrim, sliding in over 200ms (none under reduced motion); Escape
or the scrim closes it. The sidebar uses it when compact (§2.2). From the
right (`side="right"`) it is as wide as the window less 12px, at most
560px, on the page's surface: a session's panel below 1280px.

### 6.18 The code panel

A session's Changes and Files (`features/workspace/`, a chunk loaded the
first time one of them opens). It reads the session's worktree, its
project's folder, or the project whose folder holds the session's own; a
session in no project's folder says so and offers "Add the folder as a
project".

**Changes** (agent-orchestrator's review pane, GOAT's git control):

- The git bar, 40px: the branch in mono 12px after its mark, how far it
  is from its upstream (↑2 ↓1, "not pushed") and its base ("3 ahead of
  origin/main"); at the right GOAT's split button: the action that fits
  now (Commit when there are changes, else Push when ahead, Pull when
  behind, else Fetch; `data/workspace/git-quick.ts`) and the menu:
  Commit…, Push (and set upstream), Pull, Fetch, Switch branch…, New
  branch…, Rename branch…, Stash changes, Stash with untracked files,
  Stashes…; an action that cannot run is off with GOAT's reason.
- A 36px toolbar: the comparison (agent-orchestrator's picker: Changes,
  everything since the base; Uncommitted; Staged; Unstaged; Commits ›
  the latest twenty) with its +N −M, then whitespace, fold or unfold
  every file, and refresh. A worktree opens on Changes, a project's own
  folder on Uncommitted.
- Every changed file one under another: a 36px header (a chevron, the
  file's mark, its folder in the passive ink and its name in the ink at
  13px, the status letter 12px semibold, A green, D red, M orange, R
  blue, "+N −M" 12px tabular; on hover Stage or Unstage, Discard, Open
  full file), then its diff: a hunk's band on the well's fill, lines mono
  12/20 with one 36px number column over a hairline and a 16px +/−
  column, added lines on the green wash and removed on the red, in
  syntax colour. Binary, oversized, submodule and mode-only files say so
  in a line in place of the diff; a hunk longer than the node's page
  (5,000 lines, cut there) ends with a line saying the rest is in the
  full file.
- One windowed list (the timeline's window, read from its top): only the
  rows near the view exist; a file's diff, and each next page of 5,000
  lines, is asked for when its row is drawn. The comparison is read again
  every five seconds while the page is in front, and a diff kept while
  its file's counts stay the same.
- A refusal is said above the list until the next action (`GitNotice`:
  what happened, what to do, the paths it names, a hook's or the
  remote's output), never a toast; a success is a toast. The dialogs:
  "Commit changes" (GOAT's summary box: Branch, with "Default branch" in
  the warning colour, Files; Stage first: Staged only, Tracked files,
  Everything; the message, ⌘/Ctrl+Enter commits), Switch branch (a
  filter and the branches with current, default, remote and worktree
  badges), New branch and Rename branch (a name checked with git's rules
  as it is typed), Stashes (Apply, Pop, Drop, which asks again in place:
  Keep or "Drop for good"), and "Discard changes?" (Paseo's, saying a
  file the last commit does not have, untracked or added, is deleted).

**Files** (agent-orchestrator's file explorer and file pane):

- A 40px toolbar: "Go to file" (by name, `data/workspace/find.ts`; the
  letters matched in bold, "Some paths are not shown" past the node's
  bound), the switch to "Search in files" (lines by content, under their
  file, the first 500), "Show ignored files", and Refresh (the open
  folders listed again; they stay open across a file opened and closed).
- The tree: 28px rows, 6px in and 16px more per level, a folder's open
  or closed mark or a file's, 13px names; a folder is listed when first
  opened; past 5,000 entries a line says to find by name. Windowed.
- A file: a 36px head (back, the path with its folders muted, Edit), its
  lines numbered in syntax colour, windowed, the next page read as the
  reader nears the end; a line found by content is put in the middle of
  the view on the active wash (the pages up to it read first); past
  4,000 characters a line is drawn without colour. Binary: "Binary file
  preview is not available." Edit reads the rest of the file (at most 1 MiB; past it,
  "File is too large to edit"), then Cancel and Save (⌘/Ctrl+S) and an
  8px dot while there are unsaved edits. Save is guarded by the content
  hash the edit started from, which the machine compares itself
  (`file.write`'s `expect: content`, protocol 11): a change on disk since
  writes nothing, keeps the edits and shows both sides (what was read,
  what is on disk now, how many lines the refused edit differs by) with
  "Reload from disk" and "Overwrite", the one save sent without a guard.
  While a save is on its way the editor is read-only (keys typed then
  would be lost when the save closes it); a conflict compares the two
  sides' text only when the edit read the whole file, and each side
  names the start of its content hash, which two versions of one minute
  and like size would not tell apart otherwise.

**A selection to the composer.** Lines selected in a diff or a file show
agent-orchestrator's floating "Add to chat" beside the selection's end;
it appends its captured-code reference to the session's draft (a line
saying it is context, `"path"#L12-L18 (new side)`, the lines fenced as
`diff` or in the file's language, at most 8,192 characters).

**Syntax colour** is the page's own lexer (`data/workspace/syntax.ts`) on
the `--syn-*` tokens (§4.4): comments, strings, numbers, keywords,
constants, called names and markup's tags, a block comment carried from
line to line in a file.

**A worktree's setup** (agent-orchestrator's script run panel): a card
with a 32px head (a spinner, "Setting up · <branch>", the time), the
output as it comes in mono 11px (at most 240px, the end kept, colours
and cursor moves taken out, a progress bar's redraw applied; only the
state is announced to a screen reader, not each page of output), and
when it failed the command that failed and three ways on: Run setup
again, Start anyway, Stop here. Leaving the page stops the start: no
session begins after it, and the worktree stays in Settings › Projects.

**The approval of a project's commands** (no reference asks; ADR 0027,
ADR 0031): "Run this project's commands?", where setup and teardown run
and that a script or service runs when someone starts it; one line of
what changed since the set approved last, by its time ("Since the set
approved today 09:30: setup commands: 1 changed; script test added;
service web added."), from the node's `last_approved` (protocol 11). A
node that does not say (protocol 10, or an approval from before it kept
sets) is compared with the project folder's approved set and the line
says so; with neither, "No earlier approval … is known, so every command
is shown as it is." Then Setup and Teardown in order and Scripts and
services by name (its kind, its folder, what it runs), every one of the
set's scripts, never summarised: approving a set approves its scripts.
A line new since is marked "+" and "new", an edited one "~" and
"changed" with what it was beneath, struck through, a gone one "−",
"removed" and struck through. Then the warning that a script they run
runs whatever it says, the digest; Approve and run, Start without setup,
Cancel (wrapping from the right on a phone). Settings › Projects' row
counts the scripts with setup and teardown.

**Removing a worktree that would lose work** (Paseo's warning, GOAT's
snapshot): `Remove "<branch>"?`, one line per reason the node gave ("2
files with uncommitted changes", "1 commit on no other branch"), then
"Keep a snapshot and remove" (only when a snapshot keeps all of it) or
"Discard and remove", which says first what it deletes.

---

## 7. Interaction details

### 7.1 Keyboard

The operator is in this UI for hours. Shortcuts are first-class.

One registry (`data/shortcuts/`) and one listener hold every key; Settings
› Shortcuts (§3.1) lists and rebinds them, kept with the bridge. ⌘ is
Ctrl off a Mac, and every key is shown as the platform writes it.

| Keys | Action | Group |
| --- | --- | --- |
| `⌘K` or `/` | command palette | General |
| `N` | new session, on the machine in view (`⌘N` is the browser's) | General |
| `⌘,` | Settings | General |
| `G S` | Board | Navigation |
| `G M` or `G F` | Settings › Machines | Navigation |
| `G Y` | Settings › Sync | Navigation |
| `G D` | Settings › Devices | Navigation |
| `Alt+[` / `Alt+]` | previous / next session, in the sidebar's order | Navigation |
| `Alt+1`–`Alt+9` | the sidebar's first to ninth session (fixed) | Navigation |
| `⌘F` | find in the session (§6.16) | Session |
| `⌘L` | focus the composer | Session |
| `⌘B` | show or hide the sidebar | Panels |
| `⌘⇧B` | show or hide the summary | Panels |
| `Enter` | send the prompt; `Shift+Enter` a new line (§6.15) | composer |
| `Escape` | close a dialog, the find bar or the palette | — |

A key held with ⌘, Ctrl, Alt or Meta works while typing in a field; one
without (`N`, `/`, `G S`) stands down there. Nothing fires while an input
method composes, nor in a terminal off a Mac (the terminal owns Ctrl). A
sequence's second key comes within 1.5s. Keys are matched by their place
on the keyboard (`KeyboardEvent.code`), so a layout that moves letters
keeps the keys where a US keyboard has them.

Show the shortcut in menus and as a `kbd` hint on empty states. Do not
show a cheatsheet overlay on first run.

### 7.2 Live updates

- Node presence and session lists: the client already has typed RPC
  and `session.subscribe`. Rows patch in place; they do not flicker a
  reload.
- A new session inserts at the top of its tree position, 180ms fade.
- Status badge changes instantly; do not animate colour.
- Structured timeline appends events as they arrive. `MessageChunk`
  concatenates into the open agent bubble. `ToolCallUpdate` mutates
  the existing card.
- Sequence numbers (`EventSeq`) are the resume key. If the client is
  told `Lagged { resume_from }`, show reconnecting and call
  `session.read` from that seq — never pretend the gap did not happen.
- Multi-follower: permission answers and kills from another device
  appear as they are broadcast. A small meta line "answered on laptop"
  is enough.

### 7.3 Reconnecting

Three layers, because three things fail independently:

1. **This client's link to a node** (direct dropped, falling back to
   relay, or SSH restarting). Topbar device chip turns warn:
   `gpu-box · reconnecting (relay)`.
2. **The session attach** (PTY or structured subscribe). Session
   banner, §6.11.
3. **The node itself draining or gone.** Node badge `draining` /
   `offline`. Creating a session is disabled with the protocol's
   `Unavailable` message.

Fallback order is the product's: direct (3s), then relays, then (if
the book has one) SSH. The UI names the route in use, not the list of
attempts, unless the operator opens the node inspector.

### 7.4 Session actions

A session's title is its name in the title bar, the page's heading. A
click on it (or Enter) turns it into a field: Enter or leaving it saves
(`session.rename`; `data/titles.ts` checks the node's rules first: one
line, at most 120 characters), Escape keeps the old title, and focus
returns to the title. An ended session's title is not renamed.

The rest is in a menu at the bar's end, "Session actions":

| Action | Notes |
| --- | --- |
| Pin session / Unpin session | protocol 10; `session.mark` `pin`, `unpin` |
| Mark as unread / Mark as read | protocol 10; `unread`, `read` up to the last record shown |
| Start again | an ended structured session whose agent can resume (`abilities.resume`, and its own session id is known); `session.resume`, which also unarchives it |
| Archive session / Unarchive | protocol 10; archiving a live session stops its agent first, so the confirmation says so ("Archiving stops the agent on laptop first."); from protocol 11 it also ends the terminals and services beside it, which the confirmation says too |
| Delete permanently | protocol 10, an ended session; confirmed ("Delete this session permanently?", what goes and what stays); `session.delete`, and a toast says what became of its worktree (removed, or kept and why) |
| Move to another profile | a session with a timeline, while live; a dialog listing profiles of the same agent, on the same model route (`session.move_profile`) |
| Copy agent id | Paseo's row for the agent's id (a Vessel session is its agent); the toast says "Copied Agent ID" (`toast.copied`) |
| Kill | live only, apart under a separator; confirm; `session.kill` |

A board card's one action is Kill for a live session and Archive for an
ended one, both confirmed. Moving has consequences (the conversation
continues under another account), so it is a dialog, not an inline
select. A mark shows when the machine reports it (`SessionInfo.marks`).

**Read and unread** (protocol 10). A session is read when its page is in
front and its last record has been on screen: the page sends `read` with
`through` the last record shown plus one, at most once a second, not per
record. After "Mark as unread" nothing is sent as read until a new
record comes or the page is opened again, so the mark stays while the
page is still open.

### 7.5 New session

`/new`, agent-orchestrator's new-task page (`NewChatPage`): the chat
before it has a session. Every "New" leads here (the title bar, the
sidebar's row, an empty Building lane, the palette, `n`), carrying the
machine in view.

1. **Where it runs**, in the tray (GOAT's workspace selector beside the
   machine): the machine named in the address, else the one last chosen,
   else the book's default, else the first; only a connected one. Then
   the project: until the person chooses, the one asked for (the
   sidebar's "+" on its row, or New from a session in it), else the
   project of the machine's most recently active session still among its
   projects (GOAT's "most recently updated project",
   `data/workspace/last-project.ts`), else "No project", which keeps the
   folder field (it suggests where sessions already ran); and, in a git
   project, Local (its folder as it
   is), New worktree with the base it starts from ("From main"), or
   Existing branch. A project still cloning, a branch name git refuses,
   or no branch chosen holds Send with the reason under the box.
2. **The agent**: the machine's first of the four that runs with a
   timeline, else its first, else its first added agent; chosen in the
   agent control's Agent submenu, where the agents added on the machine
   follow the four by their label ("added" under each). An added agent
   is compared by its key, never kept as the machine's default (the
   page's per-agent defaults hold the four), and starts on its own
   defaults. When the machine will not start it now (`conflict`: an
   added agent's secret not set there, one of the four being installed,
   a profile's route without its key) the menu and the line under the
   box say so plainly, naming the agent's secrets, and Send stays off
   until "Ask again" (beside the line) asks the machine once more: a
   secret set since, or an install ended, is not noticed otherwise.
3. **What it runs with**: the machine asks the agent (`harness.options`,
   under the chosen profile); the controls fill in as it answers, without
   blocking the page (§6.15). Send while it is still asking waits for
   the answer, at most 10 s (agent-orchestrator's wait for its page),
   saying "Starting once … says what it offers", so the choices the
   machine kept are applied (a choice the agent does not offer fails the
   session, so none is sent unchecked). A start keeps every choice the
   agent offers, one equal to its default today included; one that could
   not check them keeps what was kept, never fewer (`keptChoices`).
4. **"+"**: a profile (an account, or a model route, ADR 0015: each says
   its route) and "Open as terminal" with the arguments the agent's
   command gets (split as a shell would, not expanded), only where the
   machine offers a terminal for the agent (an added agent has none: no
   switch, and no "+" when it has no profile either). An agent that runs
   only in a terminal on that machine is opened as one.
5. **Send** creates the session with the choices (`session.create`, the
   options in the spec, and its title from the prompt's first line, so it
   is named in the sidebar and the board at once) and opens it in place of this page as soon as it
   exists (not for someone who has left the page meanwhile), as
   agent-orchestrator does. The prompt is sent without waiting on it (the
   node answers once the agent has started, up to a minute): the
   session's log shows it at its end, "sending", from the first frame,
   until the log has it (`data/openings.ts`), and a message typed
   meanwhile is queued behind it ("Queue a follow-up while Claude Code
   gets ready"). A first prompt the machine refuses goes back into the
   session's box (before anything typed there meanwhile), and the box
   says "The first message was not sent: …" until the next message goes,
   as GOAT and agent-orchestrator return a refused brief to the
   composer. A terminal takes no prompt: the box's first line becomes
   its title.
   A refusal is said under the box, in the danger colour; the page keeps
   what was typed. In a new worktree, Send first makes it: the commands
   are asked about when the set is not approved (§6.18), setup's output
   is shown under the box as it runs, a failed setup waits for the
   person, and the session starts once setup is no longer running.

The decisions are `data/new-session.ts`'s; the page draws them.

### 7.6 Adding a machine

One dialog over the bridge's ops (track E), opened from Settings ›
Machines' "Add machine", the welcome and the command palette; Paseo's
order and GOAT's checklist (`data/pairing.ts` decides every line):

1. **Add a machine.** "On the other machine, run" `vessel node pair` (a
   CommandLine; Vessel must be running there, `vessel node start`), then
   "Paste the link it printed". Continue reads it with `inspect`, which
   sends nothing; text that is not a link says so under the field.
2. **Connect to this machine?** The key's fingerprint (13px mono, the
   first line), how it will be reached (each direct address, then each
   relay), the machine's id and when the link expires; one sentence: the
   key should match the `key` line of `vessel node status` there. Then
   "Name it", checked as typed against the book's rules and its names
   (`nameProblem`); Connect is off until the name is good. An expired link
   says so and offers no Connect.
3. **The checklist** (`pair`, then what the bridge pushes): Paired,
   Connected, Agents found, each a spinner, then a check in the ready
   colour or a cross in the danger colour with one line of result; a row
   after a failure is skipped (a dashed circle, "Not tried"). A failed row
   says what to do and, where a command helps, gives it to copy
   (`vessel node pair` for a used or expired link, `vessel node start`
   for a machine that does not answer, `vessel harnesses` for one with no
   agent). A busy page or a lost connection says to wait, not to make a
   new link: the op's typed reason decides, not its message.
4. **Ready.** "<name> is ready", Close, and "Start a session there",
   which opens a new session on it. A refused link offers "Use another
   link", back to the first stage with the field empty.

The link is a credential until used: it lives in the dialog's state only,
goes to `inspect` and `pair`, and is never put in an address, a log, a
toast or a test's failure message.

---

### 7.7 Notifications

What the page tells the person about a session they are not looking at
(`data/notices.ts`, `features/notifications/`): a session that starts to
need them (a permission or a question waits: the board's `blocked`
family, ADR 0019) or finishes its work (it was working and is now
neither working nor waiting). A change between two polls of a machine's
sessions is news; a session already waiting when the page first sees it
is not. While the page is not in front, by Settings › Notifications: the
browser's notification (titled with the session's name and why, tagged
per session and kind so two tabs show it once; clicking it opens the
session), a short tone made by the page (after its first click or key
while a sound is chosen: browsers allow no sound before, and no audio
is opened while none is chosen), and a count in the tab's title, cleared
when the page is in front again. While notices are on, a hidden page
keeps polling; browsers slow a hidden tab's timers, so a notice can come
up to about a minute late. The browser grants notifications per address,
and each launch has a new one, so permission is asked again every launch,
from the section's button; the section says what the browser answered.

## 8. Accessibility

- **Focus.** `:focus-visible` is a 2px `--ring` ring, offset 2px, on
  every control (the reference shows none; §12). A field draws it on its
  shell (`:focus-within`, over the edge); an option row reached from the
  keyboard draws it inside itself; a popup's own surface does not draw
  it. Do not remove outlines. A mouse click on a button does not leave a
  ring (`:focus-visible` only).
- **Hit targets.** 32px desktop, 44px compact (below 600px) for the primary
  controls (nav, composer send, dialog buttons).
- **Landmarks.** `role="banner"` topbar, `role="navigation"` sidebar,
  `role="main"`, inspector `role="complementary"`, composer is a
  `form`. Skip link "Skip to sessions" is the first focusable node.
- **Lists.** The board is a group of four labelled regions (the lanes),
  each a list of cards; a card's link names the session, its title, its
  phrase and its node. Tables stay `<table>` with `th scope="col"`.
- **One heading.** The page's `h1` is the last crumb in the frame's title
  bar; pages do not repeat it.
- **Live regions.** Reconnecting banner and incoming permission:
  `aria-live="assertive"`. New log lines in the timeline:
  `aria-live="polite"` on the list, not on every chunk (or the reader
  is flooded). Toasts as in §6.10.
- **Contrast.** §4.3. Status never colour-only.
- **Reduced motion.** `prefers-reduced-motion: reduce` disables the
  starting/reconnecting pulse, skeleton shimmer, dialog rise (opacity
  only), and any type-in. Instant state, no delay.
- **Theme.** System, light or dark; system by default, as the references
  have it (ADR 0021, replacing ADR 0018's light on first run). CSS
  decides: `:root` is `color-scheme: light dark`, so with no choice stored
  the page follows the system from its first paint, and live, with no
  script (the content security policy forbids an inline one). A stored
  choice sets `data-theme="light"` or `"dark"` on `<html>` before React's
  first render, which pins the scheme. The switch is in three places:
  Settings (a segmented control), the sidebar's foot (an icon button
  showing the scheme in use, with a menu), and the command palette
  ("Theme: system / light / dark"). The terminal stays dark in both
  (§4.4); xterm.js gets resolved colours and takes them again when the
  scheme changes.
- **Language.** UI copy is English, matching the CLI. `lang="en"` on
  `<html>`. Pretendard still covers Korean in session markdown and
  paths.
- **Terminal.** xterm must expose a text alternative (the live buffer
  as a visually-hidden `aria-live="off"` log the operator can reach
  with a "Screen reader buffer" toggle, default off). Structured mode
  is the accessible default for that reason.

---

## 9. Mapping onto the protocol

So M7 does not have to guess names.

| UI | Protocol / store |
| --- | --- |
| Machine name | `NodeInfo.name` (book name if it differs, book wins for display) |
| Node id (inspector) | `NodeInfo.id` |
| Version | `NodeInfo.version` |
| OS / arch | `NodeInfo.os` `arch` |
| Serving / draining | `NodeInfo.state` |
| Online | client connection state, not a daemon field |
| Reach | the route this client used: direct, SSH, relay |
| Home / member | fleet membership; home is the node that publishes |
| Sync generation | fleet manifest generation the home is on; member's applied generation from `fleet.status` |
| Conflict | local file kept, incoming at `<name>.vessel-incoming` |
| Agent | `HarnessKind`, by its product name (`data/agents.ts`) |
| Title | `SessionInfo.title` (protocol 9), a `renamed` record when it changes; `session.rename` |
| Model, effort, permissions, settings | the agent's options (`SessionOption`, by category): `harness.options` before a session, `options_changed` in its log, `session.set_option` |
| Profile | `ProfileName`; current is `SessionInfo.profile` |
| Mode | `SessionMode` |
| Model | `SessionInfo.model` (route and pinned model, protocol 7), else `SessionMode::Structured.model` |
| Route | `ModelRoute` from `model.list`; a profile names one (`ProfileInfo.model`) |
| Model call | `SessionEvent::ModelCall`: route, model, tokens, stop, duration; never content |
| Cwd | `SessionSpec.cwd` |
| Status | `SessionStatus` (+ turn from last `TurnStarted`/`TurnEnded`) |
| Created | `SessionInfo.created_at` |
| Creator | `SessionInfo.created_by` / `origin` / `via` |
| Timeline | `AgentEvent` in `EventSeq` order |
| Permission | `PermissionId` + `PermissionOption`; Approve and Decline by `PermissionOptionKind` |
| Devices | `device.list`; scopes `read`, `operate`, `admin` |
| Pairing link | `vessel://pair#…` (fragment; never sent to a server) |
| Add a machine | bridge ops `inspect` (reads the link), `pair` (book entry and credential), then `nodes`, `harness.list` |
| Rename, Make default, Forget | bridge ops `rename`, `default`, `forget`; each answers with the book |
| An agent's status | `harness.list` (`Harness.structured`); absent from it is "Not found" |
| What an agent offers | `harness.options` (`SessionOption`, grouped by `OptionCategory`) |
| Kill / move / copy | `session.kill`, `session.move_profile`, export/import |
| Send during a turn | `session.prompt` with `delivery` `queue`, `steer` or `interrupt`; between turns `start` |
| Queued prompts | `SessionConversation.queue` from the log's queue records; `session.queue` `edit` (with its revision), `move`, `remove`, `send_now`, `resume` |
| Files | `attachment.begin`, `write`, `finish`; a prompt's `ContentBlock::Attachment`; `attachment.read` for an image shown; bounds from `session.conversation`'s `limits` |
| Commands | `commands_changed` (`AgentCommand`) |
| `@` files | `workspace.find` with one search key per field (`data/workspace/find.ts`) |
| Question | `question_asked`, `question_resolved`; `session.send` `answer_question` |
| Context meter, a turn's tokens | `usage` records; `turn_ended.usage` |
| Pin, archive, unread | `SessionInfo.marks`; `session.mark` |
| Start again, delete | `session.resume`; `session.delete` (`SessionDeleted`: the worktree tidied or kept, and why) |

Secret values never appear in the UI. Sync may list secret *names*.
The app renders a QR of the returned pairing link and expires it in the
same 10 minutes as the CLI.

---

## 10. Sample data in the screens

`npm run screens` runs a small fleet of real nodes with fake harnesses
(`web/e2e/fleet.ts`). Generic names, no real hostnames.

| Node | Role |
| --- | --- |
| `laptop` | home |
| `gpu-box` | member |
| `build-server` | member |

A three-deep agent tree: a person on laptop starts a session, its agent
starts one on gpu-box, that agent starts one on build-server.

---

## 11. Decisions worth keeping

- **Purpose and intent:** a control room for a personal fleet of
  harnesses, readable in a glance, operable from the keyboard, honest
  about reach and attribution.
- **Existing constraints:** protocol 6 is already shipped; the UI must
  not rename statuses, scopes, or fleet conflict behaviour. Pretendard
  first is a host-wide font rule.
- **Alternatives considered:** a chat-first shell (hides the fleet); a
  card-grid node dashboard (too marketing, too little data); a dark,
  blue-accented palette with tinted status chips (milestone 7's: the
  accent and the chips competed with state); sessions as a table (every
  row's status had to be read); lanes by attention (the node does not
  report it yet); a top-tab layout (nodes are a hierarchy, they belong on
  the left).
- **Chosen:** agent-orchestrator's shell, values and board: a framed page
  in a grey shell, the ink as the accent, one colour per lane, the node
  deciding each session's lane and phrase, the archive apart from the
  lanes (ADRs 0018, 0019, 0020).
- **Controls (milestone 10, ADR 0021):** Base UI for behaviour, the look
  ours; Geist then Pretendard; the system's theme by default.
- **Trade-offs:** denser than consumer chat, so new users will need
  the empty states to be plain-spoken. With no hue in the chrome,
  structure rests on hairlines and type weight alone, so a careless extra
  border or a second bold shows at once. Three of the four lanes stay
  empty until a node can see pull requests.

---

## 12. Deliberate differences from the references

Where this page is not agent-orchestrator (or, where it has no such
control, GOAT), on purpose:

1. **Keyboard focus is always visible**: a 2px `--ring` on
   `:focus-visible`, on the field's shell, inside an option row reached
   from the keyboard. The reference removes focus outlines.
2. **Text meets WCAG AA** (4.5:1) at this UI's sizes: the tertiary grey
   and the in-review and ready colours are darkened (§4.3), and the
   menus' small labels and shortcuts use the tertiary colour where the
   reference uses a lighter grey.
3. **Pretendard after Geist** in every stack, ahead of any generic
   family: Geist has no Hangul, and the owner's font rule (§5.1).
4. **No glass, no blur.** The command palette and the scrims are solid
   where the reference's palette is translucent and blurred.
5. **A segmented control's chosen segment** is GOAT's lifted thumb: the
   reference's (its menu's 7% fill on the field fill) cannot be told
   apart on a white card.
6. **The permissions menu's rows share one shield.** agent-orchestrator
   maps the modes of the agents it knows to four icons and its own
   wording ("Accept edits", "Bypass permissions", in the warning colour).
   Vessel keeps no knowledge per agent (ADR 0022): the rows carry the
   agent's own names and descriptions under one icon.
7. **No prompt chips under the new-session heading** ("Review my
   changes", "Add tests"…): they assume a project and a git worktree the
   page knows nothing about, and would put words in a prompt for an agent
   Vessel cannot vouch for.
8. **No search or favourite setups in the agent menu.** The reference
   searches across every agent's model catalogue and keeps starred
   setups; an agent here offers a handful of values, in its own submenu,
   and the page keeps no favourites store.
9. **The "+" opens a popover, not a menu**: it holds the terminal's
   arguments, a text field a menu's typeahead would fight. On the
   new-session page files are attached from it ("Attach files to this
   message"); on a running session, where the "+" has nothing else to
   hold, from a paperclip in its place.
10. **Terminal or timeline is chosen once, at creation** ("Open as
    terminal"). The reference switches a session's interface later; a
    Vessel session's mode is fixed by the protocol.
11. **The inspector has three tabs** (Summary, Changes, Files) where the
    reference has more (Reviews, Browser, Side chat, Artifacts: other
    tracks' or not built), and Summary keeps 340px, the reference's
    narrowest, so the timeline's column keeps 768px beside it at 1440px;
    Changes and Files take the reference's 500px. It is not resizable.
12. **A session is named in the frame's title bar**, as the page's
    heading, where the reference has a strip of session tabs.
13. **Settings is a column, not the full pane.** agent-orchestrator's
    settings body is up to 1152px wide; this one is GOAT's 896px reading
    width, so a row's control stays near its label on a wide window.
14. **An agent's card is a list, not tabs.** agent-orchestrator's harness
    page has Account, Models and Health tabs; Vessel shows what the agent
    offers in one list, since the node reports no account or health of
    its own for an agent.
15. **No machine switch on the row.** GOAT's machine row carries an
    on/off switch; a book entry has no such state, so the row has none.
16. **The confirmation names the machine by its id.** Paseo's "Connect to
    this host?" shows the host's name; a Vessel pairing link carries the
    node's id and key but not its name, which the checklist shows once the
    machine has connected.
17. **Preferences live with `vessel ui`, not the browser** (ADR 0028).
    The references keep them in the app or the browser's storage; a
    Vessel launch has a new origin each time, so the bridge keeps them in
    a file and every tab follows it.
18. **No notifications inbox.** GOAT lists past notifications; this page
    tells about a change once (the browser's own list keeps it), and the
    board is where a waiting session is found.
19. **Shortcuts are physical keys.** agent-orchestrator records the
    character typed; Vessel records the key's place (`code`), so `Alt+[`
    and `G S` stay where they are on any layout, and shows them as a US
    keyboard prints them.
20. **One page-wide width for the conversation** (Comfortable, Wide,
    Full) where GOAT has a slider: three widths cover its range and keep
    the composer and the log aligned.
21. **A repository's commands are approved first** (ADR 0027). None of
    the references asks: Paseo and GOAT run a repository's setup file,
    agent-orchestrator only commands typed into its page. The approval
    compares with the set the node says was approved last (protocol 11),
    and with the project folder's approved set only where the node does
    not say.
22. **The diff is the page's own rows, not a diff library.** The
    references draw with `@pierre/diffs` (agent-orchestrator, GOAT) or a
    canvas (Paseo); Vessel draws the node's typed hunks in the timeline's
    window, one number column (agent-orchestrator's fallback view), no
    split view and no word-level marks. Its syntax colour is a small lexer,
    not Shiki's grammars.
23. **No line comments, no "viewed" marks, no image or rich previews.**
    agent-orchestrator's review pane sends a comment on a line and marks
    files viewed; a selection sent to the draft is what Vessel offers.
24. **"Add to chat" has no shortcut.** agent-orchestrator's is ⌘L; in a
    browser Ctrl+L is the address bar's.
25. **The editor is a text area.** agent-orchestrator edits in its diff
    library's editor with colour; Vessel's editor is plain mono text, the
    colour back once saved. GOAT's conflict choices (Reload from disk,
    Overwrite) are taken, with both sides' state, where agent-orchestrator
    has none.
26. **The new-session tray names the place** (Local, New worktree,
    Existing branch, and the base) where agent-orchestrator makes a
    worktree for every task without asking; GOAT's selector is the model.
27. **Stash is in the git menu** (Paseo's); GOAT has none.
28. **Files are uploaded when the message is sent**, not as they are
    attached (agent-orchestrator's): the node keeps what was uploaded
    against the session's 100 MiB until the session goes and has no call
    to give an upload back, so a file removed before sending would still
    count. Send waits for the uploads, each chip showing its progress.
29. **A long paste stays text.** GOAT turns a long paste into a file; the
    queue's 128 KiB and the draft's bound already say when text is too
    long, and the agent reads text as the person sees it.
30. **No "Compact context" button by the meter.** The agent's own
    `/compact`, where it offers one, is in the `/` list; Vessel adds no
    command of its own there.
31. **No Undo after archiving.** The archive's Restore is one click, and
    the confirmation already says it can be restored.
32. **Pinned sessions lead their machine's group**, not a "Pinned"
    section of their own (Paseo's and GOAT's): the sidebar is grouped by
    machine, and a pinned session's machine is part of what it is.
33. **An ended session waits on the board until archived** (with
    protocol 10) where agent-orchestrator archives a session when it
    ends: a Vessel session can be started again, and archiving is the
    person's choice.


### Shared quota meter

The read-only quota bar (`ui/kit/quota-meter`) follows Paseo's 6px track,
3px radius and clipped full-radius fill, with no minimum fill at zero. Track
and quiet fill use the named `quota-surface-3/4` tokens; normal fill uses the
existing muted/success/warning/danger roles. Those colors remain paired with
a numeric label supplied by the parent, so color is not the only value.
The validated percentage value is a dependency-free leaf shared by the data
projection and kit. The `/kit` specimen is the only initial consumer until
its light/dark comparison is accepted; account screens and wire calls remain
inactive. This addition makes no new accessibility conformance claim.
