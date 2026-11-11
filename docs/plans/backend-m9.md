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

## Track A: progress (2026-11-11)

Built on `backend/m9-brain`, in `vessel-session`, `vessel-store`,
`vessel-node` (tests only) and the docs this track owns. `vessel-proto`
is untouched: the contract held as written.

What was built:

- `vessel-session/src/board.rs` and `board/facts.rs`: the typed facts
  (`SessionFacts` with `WorkerFacts` and `Policy`; `PullRequestFacts` with
  `CiState`, `ReviewDecision`, `Mergeability`, `ReviewRunFacts`,
  `ExternalReviewFacts`, `PullRequestUrl`) and `derive(session,
  pull_requests, now, no_signal_grace) -> (KanbanColumn, DisplayStatus)`,
  the port of `DeriveKanbanPresentation` with its per-pull-request lane
  rule, its ranking and its phrase functions. `NO_SIGNAL_GRACE` is 90 s.
  Public (`vessel_session::board`) so the next milestone's node can feed
  it, and so a node test can hold the store to it.
- `board/tests.rs`: every case of `kanban_test.go`, under its own name,
  plus Vessel's own: the `Unseen` cases, the silence boundary, and the
  whole ranking (ready over in review over validating), which the
  reference's table never exercises. A few mutations of the reducer (the
  ranking, the tie breaks, the live-pool filter, the phrase order, the own
  approval rule) were each caught.
- `vessel-session/src/activity.rs`: the activity tracker, a projection of
  the records the publisher commits (the plan's rules), and
  `publish.rs`: the board re-derived on every commit and published in
  the same `send_if_modified` as the status, waking watchers only on a new
  lane, phrase or activity. The runtime sets the first board at creation.
- `vessel-session/src/terminal.rs`: the quiet timer (`QUIET`, 5 s), one
  `Sleep` pushed back on output and disarmed once it fires, in the
  actor's `select!`; unit-tested on tokio's paused clock (`test-util`, a
  dev-dependency feature other crates already enable).
- `vessel-store/src/codec.rs`: a stored session that has ended reads back
  `archive` / `terminated`, `exited`, its end as last activity; a stored
  row still live gets no board.
- Tests through the real fakes: a structured session through Ready, a
  permission round trip, a cancelled turn and the next turn, the end; a
  starting session; a failed one; a forced move (the board shows the
  cancelled turn, not one left running); a terminal printing, going quiet
  (waited for on the info watch, about 5 s of real time) and printing
  again; the node's hand-over (stored board equal to the session's own
  last board and to the reducer); a node restart over the wire.
- Docs: `docs/PROTOCOL.md` version 8, ADR 0019, the session and store
  crates' module docs.

Gate, run in the worktree on the code as committed:

- `cargo fmt --all`: no changes.
- `cargo clippy --workspace --all-targets -- -D warnings`: `Finished`,
  no warnings.
- `cargo test --workspace`: 80 test binaries, 995 passed, 0 failed, 7
  ignored (all existing: six need rust-analyzer on `PATH`, one is a
  documented limit in `vessel-node`'s milestone 5 tests).

Where this differs from the plan or the reference, and why:

- The plan's "the node passes `false` and an empty slice" became
  `PullRequests::Unseen`, a value of its own rather than an empty slice
  plus a flag: "none" and "cannot tell" stay apart in the type, and the
  `AwaitingPr` / `Idle` difference reads off it.
- The reference's `blocked` activity has no Vessel counterpart; its
  phrase functions treat it exactly as `waiting_input`, and its cases that
  use it are ported with `waiting_input`. Its zero activity (none
  reported) is `idle` here; no reference case tells the two apart.
- A pull request's lane is a three-valued internal type, so the
  reference's fall-through arms (a priority for no lane, the building
  phrase for a pull-request lane) are gone. Same results.
- `last_activity_at` moves on the handshake (`Ready`) and on the end as
  well as on the contract's list (turns, permissions, terminal output);
  message and tool-call chunks inside a turn do not move it.
- The activity is tracked in the publisher, which both actors already
  own through `Lifecycle`, rather than in each actor's loop: one copy of
  the rules, and exactly the records the log holds. The terminal's timer
  is the one piece in the actor, as the plan says.

Left open:

- `docs/ARCHITECTURE.md` is not in this track's list and was not edited.
  Its `vessel-session` paragraph does not yet mention the board, and the
  milestone list has no milestone 9 line; nothing in it is wrong.
- `DisplayStatus::Exited` (a worker gone while its session lives on) and
  `NoSignal` are reachable only in the reducer's tests: a Vessel session
  ends with its process, and none is expected to report on its own.
- A time-based change of the board (`NoSignal` after the grace) would
  need a timer to derive it again; none is needed while
  `signal_expected` is false.
- A stored row that is still live (only this run's sessions, which the
  node serves from its runtime; a `session.list` racing a hand-over could
  see one) has no board, and a client places it by status.
