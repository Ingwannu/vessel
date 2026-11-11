# Vessel web UI — design

This file is the contract for `vessel ui`'s page: why it is shaped this
way, how it maps onto the protocol, and what must not be improvised.

- The tokens are `web/src/ui/tokens.css`, the components
  `web/src/ui/app.css`. There is no other token file or stylesheet
  (ADR 0018).
- The pictures are the app's own screens: `npm run screens` in `web/`
  captures every main screen in both themes.
- Milestone 7 built the first version from a dark, blue-accented draft
  with static mockups. Milestone 8 (ADR 0018) made it white and black, one
  design system, with sessions as a board. Milestone 9 (ADRs 0019 and
  0020) took the rest from agent-orchestrator, the product the owner wants
  this one to resemble: its shell, its values, its lanes, and the node
  deciding each session's lane. The draft's mockups are in the history,
  not in the tree.
- **The reference is agent-orchestrator** (Apache-2.0): its `DESIGN.md`,
  its token file and its board view. Where this file gives a number, it is
  theirs unless it says otherwise. Two things differ on purpose: text
  colours that fall short of WCAG AA on white are darkened, and the face
  is Pretendard (the owner's font rule), at their sizes.

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
   states, explanations — are Pretendard.
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
| **Fleet** | Every node the book knows, plus live facts from each daemon | node, reach, role, harnesses, profiles, live count |
| **Sessions** | Every session, one node or all, as a board | session card, status lane, creator |
| **Session** | One session, terminal or structured | timeline or PTY, actions |
| **New session** | Create dialog | node, harness, profile, model, cwd, mode |
| **Sync** | Fleet configuration as the home publishes it | generation, members, conflicts, MCP, secrets (names only) |
| **Devices** | Pairing and revocation, per the selected node | device, scopes, invite link |
| **Settings** | This client | node book, theme |

There is no "home dashboard" distinct from Fleet. Fleet *is* the home.

### 2.2 Chrome

```
┌─ shell (grey) ────────────────────────────────────────────────────────┐
│ ▲ Vessel        ┌─ frame (the page, 12px radius, a hairline) ───────┐ │
│ ⌕ Search    ⌘K  │ Sessions                    client · connected New │ │
│ Fleet           ├────────────────────────────────────────────────────┤ │
│ Sessions      5 │                                                    │ │
│ Sync            │  main                                              │ │
│ Devices         │                                                    │ │
│ Nodes         + │                                                    │ │
│ ● laptop      3 │                                                    │ │
│   ○ vessel 3f2a │                                                    │ │
│   ◌ docs   9aa0 │                                                    │ │
│ ● gpu-box     2 │                                                    │ │
│ ─────────────── │                                                    │ │
│ Settings        └────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────────────┘
```

- **One shell, one frame.** The sidebar (240px) and the margin around the
  page are one grey surface, so the sidebar has no edge. The page is a
  framed surface set into it: 12px radius, a 4% hairline, inset 14px top
  and bottom and 16px on the right, flush with the sidebar on the left.
- **The title bar is inside the frame**: 36px, the page's one heading
  (16px semibold) after its crumbs, then this client's link and New. There
  is no bar across the window.
- **The sidebar, top to bottom**: the brand; Search (a filled row with its
  shortcut as text); the places (Fleet, Sessions with the fleet's live
  count, Sync, Devices); "Nodes" with a + that starts a session; each node
  with its live sessions under it; Settings at the foot.
- **A row** is 32px, 10px radius, 14px medium, secondary text. Its fill is
  a pill that grows from 4px inside the row on hover (150ms) and stays, at
  7% instead of 4%, on the current row. No bar marks the current row.
- **A node's row** leads with its state dot and ends with its live count
  (an em dash when offline, so it does not look idle-but-fine).
- **A session's row** leads with a mark: a spinner while its agent works,
  a filled needs-you dot while it waits on its owner, else a hollow dot;
  then its title and its id. Up to five per node, then "N more" to the
  board scoped to that node. Ended sessions are not listed.
- **Search** (`/` or `⌘K`) is a palette over nodes and sessions, opening
  under its row.
- **New** is the one primary action, in the title bar on every page, and
  opens the create dialog on the selected node (§2.5).

### 2.3 Routes

Names are UI, ids are protocol.

```
/                         → /fleet
/fleet                    all nodes
/fleet/:node              one node (its book key)
/sessions                 the board, all nodes
/sessions?node=:node      the board, one node
/sessions/:node/:id       one session, structured or terminal by its mode
/sync
/sync?node=:node
/devices
/devices?node=:node
/settings
```

Creating a session is a dialog over the current route, not a page.

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

Devices, pairing, drain/stop, and the node book entry are per node.
Sync conflicts are per member. The selected node is:

1. the node in the URL (opening a node's page selects it), else
2. the last node the operator selected this session, else
3. the book's `default_node`, else
4. the local node if the book has one.

The sidebar highlights that node. Switching it does not leave the
current place (Sync stays Sync).

---

## 3. Layout grid

A 12-column grid inside `main`, not including the sidebar.

| Breakpoint | Sidebar | Main | Columns | Gutter |
| --- | --- | --- | --- | --- |
| ≥ 1280px | 240px fixed | fluid | 12 | 16px |
| 1100–1279 | 52px icon rail | fluid | 12 | 16px |
| 768–1099 | hidden, drawer | fluid | 8 | 12px |
| < 768 | hidden, drawer | fluid | 4 | 8px |

Page padding inside main: 16px (12px below 768). Max content width is
the pane, not a centred 720px reading column — this is an app.

### 3.1 Common page frames

**Fleet.** Main is one column: one line of summary facts (nodes, live
sessions, harnesses), then a full-width node table. Selecting a row opens
a 320px inspector with harnesses, profiles, reach, last error. The summary
is a sentence of numbers, not tiles: the table under it is the fleet.

**Sessions.** The board fills the frame under the title bar: four lanes,
each a 48px heading over its own scrolling column of cards, then a 36px
bar with the archive's count. When the book has more than one node a slim
row above the lanes scopes the board to one. No inspector; a card opens
the session. Below 1024px of pane the board scrolls sideways (lanes keep
256px); below 768px the lanes stack.

**Structured session.**

```
header 40px
┌───────────────  minmax(0, 1fr)  ──────────────┬─ 320px ─┐
│ timeline                                      │ inspect │
│                                               │ plan    │
│                                               │ files   │
│                                               │ meta    │
├───────────────────────────────────────────────┤         │
│ permission (if any), then composer 88–120px   │         │
└───────────────────────────────────────────────┴─────────┘
```

The inspector hides below 1100px (toggle `⌘I`). The composer is sticky.

**Terminal session.** Header 40px, reconnecting banner if needed, then
the terminal consumes all remaining height. No inspector.

**Dialogs** (new session, pair, confirm kill or revoke): 480px wide,
centred, 16px padding, dimmer `--overlay` at 32% black (light) / 56%
(dark). New session can grow to 560px because of the model list.

**Sync.** Two stacked sections, not tabs: Sources (home) then Members.
Conflicts, when any exist, pin to the top of Members as a list, not a
toast.

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
min 72px. Touch targets below 768px inflate to 44px; on desktop we
keep 32px because this is a pointer-first operator tool, with visible
focus for keyboard.

---

## 4. Design tokens

All tokens live in `web/src/ui/tokens.css` as CSS custom properties on
`:root` (light, the design) and `:root[data-theme="dark"]` (its
inversion). `web/src/ui/app.css` uses tokens only; a literal colour there
is a defect, except the pairing QR code, which stays dark on light to
scan.

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
| `--r-2`, `--r-3` | 8 | buttons, inputs, menus |
| `--r-4` | 10 | sidebar rows, board cards |
| `--r-5` | 12 | the frame; the terminal's inner frame |
| `--r-6` | 16 | the terminal's outer frame |

The terminal uses the concentric pair 16 / 12 with a 4px gap
(inner = outer − gap). Nothing else is framed that way.

### 4.6 Elevation

Shadows are for what floats: dialogs and menus,
`0 4px 12px -4px` of the ink at 12% (black at 40% in dark) over a hairline.
Cards, rows and tables use hairlines, never shadows.

A filled button's only depth is inside it (`--shadow-control`): a top edge
lit at 6% and a bottom edge shaded at 10%. No outer shadow, no glow.

`--overlay`: the ink at 32% light, black at 56% dark.

### 4.7 Motion

| Token | Value |
| --- | --- |
| `--ease` | `cubic-bezier(0.16, 1, 0.3, 1)` |
| `--d-1` | `120ms` |
| `--d-2` | `180ms` |
| `--d-3` | `240ms` |

Animate `opacity` and `transform` only. Dialogs: fade + 8px rise.
Inspector and sidebar: width, not a bounce. Tool-call expand: the
`<details>` disclosure, no extra animation. Live status dots do not
pulse unless the state is `starting` or `reconnecting`, and even then
the pulse is opacity 1 → 0.45 → 1 over 1.6s, stopped under
`prefers-reduced-motion`.

---

## 5. Typography

### 5.1 Families

```css
--font-sans: Pretendard, -apple-system, BlinkMacSystemFont, "Segoe UI",
             sans-serif;
--font-mono: Consolas, "JetBrains Mono", "Cascadia Mono", Pretendard, monospace;
```

**Mandatory host rule, applied everywhere:**

- Pretendard is first in every `font-family`, including the mono stack
  (after the real monospace faces, before the generic `monospace`).
- Never put a bare `monospace` or `serif` first.
- Never use Gulim, Dotum, Batang, or any family that renders as a pixel
  font on Korean Windows Chrome.
- **The app self-hosts Pretendard** (woff2, `font-display: swap`). No
  font is loaded from a CDN.

Two families only. No display serif, no third mono.

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
| `--t-mono-lg` | 13px / 22 / 400 | mono | composer, tool I/O |
| `--t-term` | 13px / 20.8 (1.6) / 400 | mono | terminal |

Headlines heavier than 600 are forbidden. Markdown in the timeline may
use 600 for `h3`/`h4` equivalents; there is no `h1` inside a session.

---

## 6. Component inventory

Every component below is in `web/src/ui/app.css` under the same class
name. States
to implement: default, hover, active, focus-visible, disabled,
loading, error, empty.

### 6.1 Buttons

| Class | Look | Use |
| --- | --- | --- |
| `.btn` | a quiet grey fill (`#F0F0F0`), no edge | default |
| `.btn.primary` | the ink as a fill (`#1F1F1F`, white text, weight 500) with the inset edge | one primary per view (New, Create, Send, Pair) |
| `.btn.danger` | `--danger` text on no fill, a 12% tint on hover; solid `--danger` only on the confirm step | Kill, Revoke |
| `.btn.ghost` | no fill, secondary text; the sidebar's growing pill on hover | icon buttons, quiet actions |
| `.btn.sm`, `.btn.xs` | 28px and 24px tall, 12px type | row actions, small controls |

Buttons are filled, not outlined, 32px tall with an 8px radius. Hover
deepens the fill; nothing scales or moves.

The primary button is the one dark shape on the page; that is why there
is one per view.

Icon + label. Icon-only buttons need `aria-label`. Gap 6px. Radius
`--r-2`. Disabled at 50% opacity, `pointer-events: none`.

### 6.2 Inputs

Text fields, selects, the composer. Surface fill, 1px `--border-subtle`,
focus: 1px `--accent` (the ink, not a glow). A search well and a segmented
control's track are filled `--bg-hover` with no edge; the chosen segment
is a white (black, in dark) thumb with a `--border-strong` ring. Height 32px. Placeholder
`--text-tertiary`. Labels sit above, `--t-meta`, never inside.

Selects use the native control styled; the app may replace with a
listbox later. Model picker is a listbox grouped by provider, not a
flat 40-item `<select>` once routing exists.

Checkbox / radio: 14px square/circle, accent when on.

### 6.3 List rows and tables

`.table` is a `<table>` with hairline row borders, no vertical rules, no
zebra. Column headers are sentence case in `--t-kicker`. Hover
`--bg-hover`. Selected `--bg-active` plus a 2px ink bar on the left.

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
| `TurnStarted` | filled accent tick | the prompt, mono 13px, on `--bg-elevated` |
| `UserMessageChunk` | same | resume replay; muted, labelled "replayed" |
| `MessageChunk` | empty circle | markdown, sans 13px, max 78ch |
| `ThoughtChunk` | dashed circle | collapsed `<details>`, tertiary |
| `ToolCall` | square by `ToolKind` | `.tool` card, §6.6 |
| `ToolCallUpdate` | (same card, in place) | do not append a second card |
| `Plan` | checklist | inspector, and a compact strip in-timeline |
| `PermissionRequested` | amber diamond | §6.7; also sticky above composer |
| `PermissionResolved` | check or x | one line under the request |
| `TurnEnded` / `TurnFailed` | rule | reason in meta |
| `Ready` | meta | harness session id, mono |
| `HarnessLog` | meta | stderr, mono, tertiary; error colour if it looks like a login failure |

User and agent are not two chat bubbles on opposite sides. This is a
log, left-aligned, like an editor's output channel.

Markdown: Pretendard, 13/20. Inline code and fences use `--font-mono`
on `--bg-elevated` with a hairline. Headings inside a turn start at
the `h3` equivalent (14px / 600). Lists and blockquotes get a 1px
accent-left only for blockquotes.

### 6.6 Tool-call cards

`<details class="tool">`. Summary: kind icon, title, status badge,
location path (mono, truncated from the left). Body: input then output,
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
screen.

- In-timeline card, `--warn` hairline, listing the tool title and the
  harness's options in order (`AllowOnce`, `AllowAlways`, `RejectOnce`,
  `RejectAlways` — use the harness labels, not our own).
- Sticky bar above the composer, same actions, so it cannot scroll off.
- First answer wins (protocol). After `PermissionResolved`, both copies
  freeze showing the chosen option and who (this device / another
  follower). Later clicks error with conflict — show a toast, do not
  hide the history.
- Composer `disabled` while a permission is pending, with helper text
  "Answer the permission to continue".

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

`.dialog` 480–560px, `--bg-elevated`, radius `--r-4`, padding `--s-4`.
Header 13px / 600, close ghost button. Footer right-aligned, ghost
Cancel then primary. `role="dialog"` `aria-modal="true"`
`aria-labelledby`. Focus moves in on open and back to the opener on
close.

Dimmer click and Escape cancel, unless a pairing invite is live and
uncopied — then confirm.

### 6.10 Toasts

Bottom-right, 320px, stack up. 4s for success, stay for error until
dismissed. `--bg-elevated` + hairline + 3px status bar on the left.
`role="status"` for success, `role="alert"` for error.

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
- **The archive is not a lane.** Ended sessions (`archive`) are counted on
  a 36px bar under the lanes; the count opens them as the same cards,
  three to a row, newest first, with a search. An archived card reads its
  own status word, and a failed one keeps `--danger`.

**Card.** One session. In order:

1. *Origin*, only when it was not started with the owner token: "agent of
   `3f2a`" (a link; "on gpu-box" when the parent runs elsewhere), or
   "device `ab12cd`".
2. *Title row*: the agent's mark (a letter in an 18px tile: C for Claude
   Code, X for Codex, G, O), then the title, 11.5px semibold, two lines at
   most. The title is the last segment of the working directory; a session
   has no name in the protocol yet.
3. *Where*: the mode's icon and the working directory, mono 10.5px, cut
   from the left so its end stays.
4. *Who*: the id's first four characters, then the profile (or the
   harness) and the model when there is one.
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
  without moving the title: Kill, for a live session. It asks first.

---

## 7. Interaction details

### 7.1 Keyboard

The operator is in this UI for hours. Shortcuts are first-class.

| Keys | Action |
| --- | --- |
| `⌘K` or `/` | command palette (focus search) |
| `⌘N` | new session |
| `⌘Enter` | send prompt (structured) |
| `Escape` | close dialog, blur composer, collapse palette |
| `j` / `k` | next / previous row (when not in an input) |
| `Enter` | open the selected row |
| `⌘.` | kill the open session (confirm if running) |
| `⌘⇧C` | copy session id |
| `⌘I` | toggle inspector |
| `⌘B` | toggle sidebar |
| `[` / `]` | collapse / expand tool calls in view |
| `g f` | Fleet |
| `g s` | Sessions |
| `g y` | Sync |
| `g d` | Devices |
| `1`–`9` | select nth node in the sidebar |
| `⌘1` / `⌘2` | structured composer vs (future) editor pane — reserved |

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

Available on both modes, from the session header overflow and from
the list row:

| Action | Notes |
| --- | --- |
| Kill | confirm; `session.kill` |
| Move profile | structured only, between turns; list profiles of the same harness that are logged in |
| Copy to node | `session.export` / `import`; pick node, profile, cwd; optional start |
| Copy id | |
| Open log | future |

Move and copy are dialogs, not inline selects, because they have
consequences (the conversation continues elsewhere).

### 7.5 New session

Fields in order, because each one filters the next:

1. **Node** — book names, with online/offline. Offline nodes disabled.
2. **Harness** — those the node discovered. If structured is
   unavailable, the mode toggle still shows structured but disabled
   with the install hint from `StructuredSupport::Unavailable`.
3. **Profile** — that harness's profiles on that node (`claude`,
   `claude-work`, …). Default profile labelled.
4. **Model** — "harness default", then the node's routes
   (`model.list`) grouped by provider. A route reaches a session only
   through a profile naming it, so picking a route picks that harness's
   profile on it, and picking a routed profile shows its route; a route
   no profile of the harness names is listed disabled. (Built 2026-11-07
   with milestone 5, ADR 0017.)
5. **Working directory** — text, placeholder the node's home, with a
   recent list.
6. **Mode** — structured (default when available) or terminal.

Create is the only primary button. The dialog does not close until
the RPC returns; the button shows a spinner. Failure sits inline,
not only as a toast.

---

## 8. Accessibility

- **Focus.** `:focus-visible` is a 2px `--accent` ring, offset 2px,
  on every control. Do not remove outlines. Mouse click does not
  leave a ring (`:focus-visible` only).
- **Hit targets.** 32px desktop, 44px below 768px for the primary
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
- **Theme.** Light on first run, whatever the system prefers: white is
  the product's look (ADR 0018). The Settings toggle writes the choice.
  Do not flash one theme then the other: set the attribute before paint.
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
| Node name | `NodeInfo.name` (book name if it differs, book wins for display) |
| Node id (inspector) | `NodeInfo.id` |
| Version | `NodeInfo.version` |
| OS / arch | `NodeInfo.os` `arch` |
| Serving / draining | `NodeInfo.state` |
| Online | client connection state, not a daemon field |
| Reach | the route this client used: direct, SSH, relay |
| Home / member | fleet membership; home is the node that publishes |
| Sync generation | fleet manifest generation the home is on; member's applied generation from `fleet.status` |
| Conflict | local file kept, incoming at `<name>.vessel-incoming` |
| Harness | `HarnessKind` |
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
| Permission | `PermissionId` + `PermissionOption` |
| Devices | `device.list`; scopes `read`, `operate`, `admin` |
| Pairing link | `vessel://pair#…` (fragment; never sent to a server) |
| Kill / move / copy | `session.kill`, `session.move_profile`, export/import |

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
- **Trade-offs:** denser than consumer chat, so new users will need
  the empty states to be plain-spoken. With no hue in the chrome,
  structure rests on hairlines and type weight alone, so a careless extra
  border or a second bold shows at once. Three of the four lanes stay
  empty until a node can see pull requests.
