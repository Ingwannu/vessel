# ADR 0018: A white and black design system, and sessions as a board

Status: accepted, 2026-11-11 (the owner's direction on seeing milestone 7's
UI: white and black, one design system, sessions as a kanban board in the
manner of agent-orchestrator's).

## Context

Milestone 7 built the web UI from a design draft (`docs/design/web-ui/`):
dark first with a blue accent, a light theme derived from it, tinted status
chips, uppercase column labels, and Sessions as a table that nests the
sessions agents start under the sessions that started them.

Three things were wrong with that once the owner used it:

- **The look.** The owner wants a white page with black ink and nothing
  else in the chrome. The draft's light theme was grey-blue surfaces under
  a blue accent.
- **Two sources of truth.** The draft kept its own `tokens.css`, its own
  `app.css` and seven static HTML mockups beside the app's `tokens.css` and
  `app.css`. They had already drifted (the draft's README listed five
  corrections "the mockups may not yet show"), and every later change
  would have to be made twice or leave the draft wrong.
- **Sessions as a table.** The question an operator brings to that page is
  "what is running, and what stopped and why". A table ordered by start
  time answers it only after every row's status is read.

[agent-orchestrator](https://github.com/OrchestratorInc/agent-orchestrator)
(Apache-2.0) solves the same supervision problem with a board: sessions are
cards in lanes, a lane is a state, and its design guide (`DESIGN.md` there)
states the rules that keep such a board calm. The owner named it as the
model.

## Options

**The look**

1. Keep dark first and retune the light theme.
2. White and black as the design, with the dark theme as its inversion.

**The design sources**

1. Keep the draft's mockups and tokens, and update them with every change.
2. One token file and one stylesheet, in the app; `DESIGN.md` is the
   contract; the app's own screens (`npm run screens`) are the pictures.

**Sessions**

1. Keep the table and add a status filter.
2. A board with lanes by attention (working, needs you, idle), as
   agent-orchestrator has.
3. A board with one lane per `SessionStatus`.
4. Both a board and a table, behind a toggle.

**Taking from agent-orchestrator**

1. Copy its board components (React with Tailwind and shadcn primitives).
2. Take its structure and its rules, and write them in Vessel's own CSS and
   components.

## Decision

**White and black (the look, option 2).**

- The canvas is white (`#ffffff`) and the ink near-black (`#0a0a0a`). The
  sidebar is one step off white; hover and selection are greys.
- The accent is the ink. Selection is a grey row with a black bar, focus is
  a black ring, the primary button is black with white text, and a link in
  running text is underlined, since colour no longer marks it.
- Hue is kept for two things: state (running green, starting amber, failed
  red, as a dot and a word) and syntax inside code, diffs and the terminal.
- The dark theme is the same system inverted: black canvas, white ink.
- A first visit is light whatever the system prefers; the operator's choice
  in Settings is kept. The white page is the product's look, not one of two
  equal skins.
- The terminal stays black in both themes. Harness TUIs draw for a dark
  terminal, and a black panel on a white page is the look itself. It has
  tokens of its own (`--term-*`), because the page's accent is black there.

**One design system (the sources, option 2).**

- `web/src/ui/tokens.css` is the only token file and `web/src/ui/app.css`
  the only stylesheet. A literal colour in `app.css` is a defect, with one
  stated exception: the pairing QR code, which must stay dark on light to
  scan.
- `docs/design/web-ui/DESIGN.md` is the contract: the rules, the token
  tables with their measured contrast, the component inventory.
- The draft's mockups, its `tokens.css` and its `app.css` are removed. They
  stay in the history (`400baae` and before).
- Rules that now hold on every screen: labels are sentence case and never
  uppercased; a border separates durable regions and is not put around a
  thing because it is rectangular; state is a dot and a word, not a tinted
  chip; a summary is a line of facts, not a row of tiles.

**Sessions are a board (option 3).**

- Lanes, left to right: Running (`starting` and `running`), Interrupted,
  Failed, Exited. `data/board.ts` `laneOf` is the only place a status
  becomes a lane, and it matches exhaustively.
- A card is a session: its working directory's last segment as the title,
  its id and path, who runs it (profile or harness, mode, model), then one
  status line with the node and the time.
- Attribution (ADR 0013) survives the move from a tree to lanes two ways.
  A card an agent started names the session that started it, linked,
  whatever lanes the two are in. When both are in one lane the child still
  follows its parent, indented, with the tree's connector.
- The whole card opens the session. One action (Kill, for a live session)
  appears on hover or focus, and asks before it kills.
- Lanes are one grid divided by hairlines. Each scrolls by itself and shows
  20 cards before "Show more", since the node keeps every ended session.
- The sidebar lists each node's live sessions under it, to switch between
  them from any page.

**From agent-orchestrator: structure and rules, not code (option 2).**

No file of theirs is copied. What is taken is what a design guide is for:
lanes as one divided grid with a dot, a name and a count; the card as the
one bordered object; one status line per card; sentence case; borders as
structure; no tiles of repeated metrics. Their components are Tailwind and
shadcn; Vessel's UI is plain CSS on tokens with no component library, and
adding one for four lanes would cost more than writing them.

## Why

- **White and black is a decision about noise.** With no hue in the chrome,
  the only coloured thing on a page is a state, so a failed session is the
  first thing seen. The blue accent competed with exactly that.
- **One source cannot drift.** The mockups did their job in milestone 7,
  which was to settle the design before code existed. Kept after that, they
  are a second design that is wrong by default.
- **Lanes by status, not by attention, because the node does not report
  attention.** `SessionInfo` carries `SessionStatus` and nothing about a
  turn or a pending permission. The page could find those only by replaying
  each live session's event log: a full read per card, and still a guess
  for terminal sessions, which have no turns. The design's first principle
  is that every label maps to something the node says. Status lanes are
  true today; attention lanes would be a second vocabulary invented in the
  browser.
- **Not a table and a board.** Two views of one list are two things to keep
  right, and the table's one advantage, the tree, is kept on the board.
- **Not their code.** A dependency needs a reason (AGENTS.md section 9).
  The reason would be four lanes and a card.

## Consequences

- `web/src/ui/tokens.css`: light is `:root`, dark is
  `:root[data-theme="dark"]` (it was the other way round). `data/theme.ts`
  no longer reads `prefers-color-scheme`.
- Every text and background pair in `DESIGN.md` section 4.3 was measured
  again at WCAG AA against the new values. A token change must re-measure
  its pairs in the same change.
- `data/tree.ts` loses `liveOnly` (the board shows every status in its
  lane) and its rows gain `parentNode`.
- The Sessions page has no Live/Recent switch. Ended sessions are the
  board's right-hand lanes.
- `docs/design/web-ui/` holds `DESIGN.md` and a README only.
- Follow-up, not scheduled: **lanes by attention**. When the node reports,
  per session, whether a turn is running and whether a permission is
  waiting (the session actor already knows both; it would be a field on
  `SessionInfo` and a protocol version), the board can have Working, Needs
  you and Idle lanes. Only `laneOf` and its labels change. Terminal
  sessions would stay in a lane by status, since a PTY has no turns.
- Follow-up, not scheduled: an archive. Nothing removes an ended session
  from the board; "Show more" is the only relief for a long Exited lane.
