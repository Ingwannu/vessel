# Milestone 9: agent-orchestrator's board, in Rust and strict TypeScript

Status: in progress, 2026-11-11. Built on `backend/m9`, which is stacked on
`backend/m8-board` (milestone 8 is not merged yet).

Read first: `AGENTS.md`, `docs/plans/backend-m8.md`, ADR 0018,
`docs/design/web-ui/DESIGN.md`, and in the reference clone of
[agent-orchestrator](https://github.com/OrchestratorInc/agent-orchestrator)
(Apache-2.0): `DESIGN.md`, `backend/pkg/contract/kanban.go`,
`backend/pkg/contract/status.go`, `backend/pkg/contract/kanban_test.go`,
`packages/product-ui/src/SessionsBoardView.tsx`.

## Goal

The owner looked at milestone 8 and said it did not resemble
agent-orchestrator: bring it over as closely as possible, in Rust where
that is possible and in the strictest TypeScript where a UI needs it; and
the product should run as a desktop app, in a browser and on a phone, as
Paseo does.

Milestone 8 took the structure of that board and kept Vessel's own shell
and lanes by `SessionStatus`. This milestone takes the rest:

1. **The board's brain, in Rust.** agent-orchestrator's daemon decides
   each session's lane and the phrase on its card with one pure reducer
   (`DeriveKanbanPresentation`). The node gets the same reducer, ported,
   fed with what a Vessel session can report.
2. **The shell and the board as theirs are**, in the web UI: the framed
   centre panel, the sidebar rows, the lanes Building, Validating, In
   review, Ready, the archive apart from the lanes, the card.
3. **One UI on three surfaces**: decided as a direction here (ADR 0020,
   proposed), built in a later milestone.

## Out of scope

- Pull requests, checks and reviews (the facts behind Validating, In
  review and Ready). The reducer takes them as input and the tests cover
  them; no node gathers them yet. That is the next milestone ("Left
  open").
- The desktop and phone shells themselves.
- agent-orchestrator's other surfaces: the task composer, the session
  inspector's files, reviews and browser tabs, automations, cloud.

## Before writing code

1. **What exists.** The session actors already see every agent event
   (`vessel-session/src/structured.rs`) and publish `SessionInfo` through
   a watch (`publish.rs`). The web UI has the board of milestone 8, the
   tree, the tokens.
2. **Reuse or new.** The reducer is new code, a port. The activity facts
   come from the existing actors. The web UI's board is restyled and
   re-laned, not rewritten: `data/board.ts` stays the one place a lane is
   decided.
3. **Outside.** agent-orchestrator's reducer and its tests are the
   specification. Its UI is Tailwind and shadcn; its values are taken, its
   components are not (ADR 0018's reasons hold).
4. **Decision: port and modify.** No new dependency.

## Contract (protocol 8; the coordinator's, already on `backend/m9`)

`crates/vessel-proto/src/board.rs`, commit `13e56d1`:

- `KanbanColumn`: `building`, `validating`, `needs_review`, `ready`,
  `archive`.
- `DisplayStatus`: agent-orchestrator's phrases as a closed enum, plus
  `idle` (between turns on a node that cannot see pull requests).
- `Activity`: `active`, `idle`, `waiting_input`, `exited`.
- `SessionBoard { column, display_status, activity, last_activity_at }`.
- `SessionInfo.board: Option<SessionBoard>`, absent from older nodes.
- `PROTOCOL_VERSION` is 8. Nothing else on the wire changes.

A track that needs the contract changed asks the coordinator; it does not
edit `vessel-proto`.

## Track A: the board's brain (`backend/m9-brain`)

Owns: `crates/vessel-session`, `crates/vessel-node`, `crates/vessel-store`
where a stored session is turned into a `SessionInfo`, `docs/PROTOCOL.md`,
ADR 0019, the module docs of what it touches. Does not touch `web/` or
`vessel-proto`.

1. **The reducer**, a new module `vessel-session/src/board.rs`:
   - The fact types, as agent-orchestrator has them, typed: the session's
     facts (activity, last activity, signal expected and seen, terminated,
     interrupted, and the three policy flags `auto_review`,
     `auto_inject_review`, `auto_inject_ci` with `has_artifact_output`),
     and per pull request: draft, merged, closed, `CiState`,
     `ReviewDecision`, `Mergeability`, updated at, the review-run facts
     and the external-review facts. None of these is a wire type.
   - `derive(session, pull_requests, now, no_signal_grace) ->
     (KanbanColumn, DisplayStatus)`: a faithful port of
     `DeriveKanbanPresentation`, its per-pull-request column rule, its
     ranking (ready, then needs review, then validating; ties by the most
     recently updated, then by URL) and its four phrase functions.
   - One difference, stated in the code: a session between turns with
     nothing delivered is `AwaitingPr` only when the node can see pull
     requests; otherwise it is `Idle`. Until a node gathers pull requests
     it never can.
   - Every case of `kanban_test.go` ported as table tests. The port is
     checked by them, not by eye.
2. **Activity**, from what the actors already see:
   - Structured: `TurnStarted` is `active`; an unanswered
     `PermissionRequested` is `waiting_input` until every pending request
     is resolved; `TurnEnded` and `TurnFailed` are `idle`. `interrupted` is
     true while the last turn ended cancelled and none has started since.
   - Terminal: output makes it `active`; a quiet stretch (a constant,
     about five seconds) makes it `idle`. One resettable timer in the
     actor's loop, not a poll. Only a change of state touches the watch:
     output alone must not wake watchers.
   - `starting` is `active`. An ended session is `exited` and terminated.
3. **Publishing.** `SessionInfo.board` is kept current through the same
   watch as `status`. A change of lane, phrase or activity wakes watchers;
   nothing else about it does.
4. **Stored sessions.** A session read back from the store after a restart
   is ended: its board is `archive` / `terminated`, activity `exited`, last
   activity its end.
5. **Docs.** `docs/PROTOCOL.md` version 8; ADR 0019 (why the node derives
   the placement, why the reducer is a port, why the phrase is an enum,
   what `idle` is, why ended sessions are the archive); the session
   crate's module docs.
6. **Gate.** `cargo fmt --all`, `cargo clippy --workspace --all-targets --
   -D warnings`, `cargo test --workspace`. Report all three.

Decisions already made for this track, so it does not have to stop:

- An ended session is in the archive whatever its status (`exited`,
  `failed`, `interrupted`). The client shows the status word on the
  archived card. agent-orchestrator's "exited but not archived" has no
  counterpart: a Vessel session does not outlive its process.
- `signal_expected` is false for every Vessel session today (a structured
  session that never becomes ready already fails at the handshake
  timeout), so `NoSignal` is reachable only in tests. It is ported so the
  reducer stays the reference's.
- No policy flag is ever true and no pull request is ever passed, until
  the next milestone. The node passes `false` and an empty slice.

## Track B: the shell and the board (`backend/m9`, the coordinator)

Owns: `web/`, `docs/design/web-ui/`, ADR 0020.

1. **Tokens** from agent-orchestrator's light theme: canvas `#FAFAFA`,
   sidebar `#F4F4F4`, card `#FFFFFF`, hairlines as black at 8% and 4%, the
   lane colours (building blue, validating plum, in review amber, ready
   green, needs-you red), its type scale and radii. Pretendard stays the
   face (the owner's font rule); the sizes are theirs.
2. **Shell**: a 240px sidebar with no edge, and the page as a framed
   surface (12px radius, a hairline) inset in the sidebar's colour, with a
   36px title bar inside it. Rows 32px with the growing hover pill.
3. **Board**: lanes Building, Validating, In review, Ready; a 48px lane
   heading (8px dot, the lane's colour on its name, the count); the card
   (title two lines at 11.5px semibold, a line under it, one status line
   in the lane's colour with a spinner while working, the age as "6m
   ago"); the needs-you card (its edge, its tint, its slow pulse); the
   archive under the lanes as a count that opens the ended sessions.
4. **Lane and phrase from the node**: `data/board.ts` reads
   `SessionInfo.board`; for a node older than protocol 8 it keeps the
   placement `SessionStatus` implies, as agent-orchestrator's client does
   for an old daemon.
5. **Types.** The phrase and the lane are matched exhaustively. `tsconfig`
   gains `noUncheckedIndexedAccess` and `exactOptionalPropertyTypes` if
   the code can take them in this milestone; if not, the plan says what
   stopped it.
6. **Gate.** `web/`'s lint, typecheck, unit tests, build, e2e and screens.

## One UI on three surfaces (direction; ADR 0020, proposed)

What the two references do:

- **Paseo**: one Expo / React Native tree for phone, web
  (`react-native-web`) and desktop (Electron loading the web build). The
  terminal is a native grid on the phone and xterm.js on the web. Phones
  reach a daemon through a relay with a Noise channel, paired by a link.
- **agent-orchestrator**: an Electron desktop app in React DOM and a
  separate Expo phone app with its own screens. No UI is shared; the
  phone mirrors the wire types by hand.

What Vessel already has: the relay, the Noise channel, pairing and device
credentials, all in Rust (`vessel-client`); a web UI in React DOM with
types generated from the wire; a bridge (`vessel-web`) generic over where
its nodes come from.

Proposed: **one web UI in three shells**. The browser keeps `vessel ui`
(the loopback bridge). The desktop and phone apps are Tauri 2 shells whose
Rust side links `vessel-client` and the bridge directly and is a paired
device of its own; the page talks to it over Tauri's IPC with the same
typed envelope it sends the websocket today. The UI gets one seam (the
transport) and a phone layout. The owner decides (ADR 0020 lists the
alternative, an Expo rewrite as Paseo did).

## Left open

- **Pull request facts** (the next milestone): the session's branch, its
  pull requests, checks and reviews, gathered by the node, so Validating,
  In review and Ready fill. The reducer and the contract are ready for
  them; `SessionBoard` gains the evidence for the card.
- **An archive action.** Ended sessions are the archive; nothing removes
  one.
- **The shells** (ADR 0020).
