# Milestone 8: the web UI's design (white and black, a board)

Status: built on `backend/m8-board`, 2026-11-11. Not merged: it reaches
`main` by a pull request once the owner has looked at it.

Read first: `AGENTS.md`, `docs/design/web-ui/DESIGN.md`, ADR 0017 (the web
UI) and ADR 0018 (this milestone's decisions and their reasons).

## Goal

The owner's direction after using milestone 7's UI:

1. White and black, clean.
2. One design system.
3. Sessions as a kanban board, in the manner of
   [agent-orchestrator](https://github.com/OrchestratorInc/agent-orchestrator)'s.

## Out of scope

- Any crate, the wire, the bridge, the security model of `vessel ui`. This
  milestone is `web/` and the docs.
- Lanes by attention (working, needs you, idle). They need the node to
  report each session's turn and pending permission; see "Left open".
- Drag and drop. A lane is a fact the node reports, not a column a person
  moves a card into.

## Before writing code

1. **What exists.** The app's tokens, stylesheet, session tree
   (`data/tree.ts`), status vocabulary (`data/format.ts`) and every route.
   All reused.
2. **Reuse or new.** Reuse. The board is a new view over the existing
   tree; the tokens are re-valued, not replaced; the class names stay.
3. **Outside.** agent-orchestrator's board and design guide (Apache-2.0).
   Its structure and rules are taken; its code is not, because it is
   Tailwind and shadcn and this UI is plain CSS on tokens (ADR 0018).
4. **Decision: modify.** No new dependency.

## What was built

- **Tokens** (`web/src/ui/tokens.css`): light is `:root` and is white and
  black; dark is `:root[data-theme="dark"]`, the inversion. The accent is
  the ink. New: `--t-card`, `--lane-head-h`, `--lane-min-w`, and the
  terminal's own `--term-frame`, `--term-dim`, `--term-border`,
  `--term-cursor`, `--term-selection`.
- **Theme** (`data/theme.ts`): light until the operator chooses; the
  system preference is no longer read.
- **Board** (`data/board.ts`, `routes/sessions.tsx`): lanes Running,
  Interrupted, Failed, Exited from `SessionStatus`; cards; a child card
  under its parent in the same lane; "agent of" on every agent-started
  card; Kill from a card with a confirmation; 20 cards a lane, then more.
- **Tree** (`data/tree.ts`): rows carry `parentNode`; `liveOnly` is gone
  with the Live/Recent switch.
- **Shell** (`ui/shell.tsx`): no section labels; a node's live sessions
  listed under it.
- **Fleet** (`routes/fleet.tsx`): the four summary tiles are one line; the
  page's own New session button is gone (the topbar's New is on every
  page); opening a node selects it, so the topbar's New opens on it.
- **Stylesheet** (`ui/app.css`): no uppercase, no tinted chips (a badge is
  an outlined word with a state dot), filled wells for search and
  segmented controls, ink links underlined in running text, the board and
  card rules.
- **Docs**: ADR 0018; `DESIGN.md` rewritten where the design changed, with
  the contrast table measured again; the milestone 7 draft's mockups,
  token file and stylesheet removed from `docs/design/web-ui/`.

## Two defects of milestone 7 found and fixed on the way

- **A terminal session's page was painted black, with its header inset.**
  `.term` styled the PTY surface, but `.session.term` is also the session
  page's container. The surface rule is now on `.xterm-host` alone.
- **The terminal's cursor and selection used the page's accent.** With a
  black accent on a black terminal neither would show. They have terminal
  tokens now.

## Checked

Automated, on Linux (2026-11-11):

- `web/`: `npm run lint` (ESLint, Prettier, the font rule: 29 stacks),
  `npm run typecheck`, `npm test` (10 files, 38 tests), `npm run build`.
- `npm run e2e`: 16 passed, against real nodes with the fake harnesses.
  One is new: the board holds a session in its status's lane, kills it
  from its card after the confirmation, and the card opens the session.
- `npm run screens`: every main screen in both themes.
- The Rust gate (`cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace`): no crate
  changed; run because `vessel-web` embeds the bundle.

By eye, from the screens: Fleet, the board (light and dark), a structured
session with a pending permission, a terminal session, Devices.

Not checked:

- macOS and Windows. Nothing here is platform code, but CI has not run on
  this branch.
- A board with Interrupted, Failed and many Exited cards from real use.
  The test fleet produces Running and Exited.
- Widths under 1280px (the icon rail and the stacked lanes) by eye.
- A screen reader.

## Left open

- **Lanes by attention.** The session actor already knows whether a turn
  is running and which permissions wait. Reporting that on `SessionInfo`
  (a protocol version) lets the board have Working, Needs you and Idle.
  Only `data/board.ts` `laneOf` and its labels change. Terminal sessions
  stay in a lane by status.
- **An archive.** Ended sessions stay on the board for as long as the node
  keeps their logs.
- **The sidebar on a session's page** highlights both Sessions and the
  session's own row (the router marks a link active on any path under it).
  It reads as "in Sessions, this one"; a single highlight would be
  cleaner.
