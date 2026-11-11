# ADR 0019: The node places each session on the board

Status: accepted, 2026-11-11 (milestone 9, track A,
`docs/plans/backend-m9.md`; the contract is protocol 8,
`crates/vessel-proto/src/board.rs`).

## Context

The owner looked at milestone 8's board and asked for
[agent-orchestrator](https://github.com/OrchestratorInc/agent-orchestrator)'s
(Apache-2.0) as it is. Its board has lanes by where a session is in
delivery (Building, Validating, In review, Ready, and an archive apart),
and on each card one phrase for what is happening in that lane ("Working",
"Blocked", "Fixing CI failures", "Mergeable"). Its daemon decides both
with one pure reducer, `DeriveKanbanPresentation` in
`backend/pkg/contract/kanban.go`, specified by the table of cases in
`kanban_test.go`.

Milestone 8 placed cards by `SessionStatus` alone, because that was all a
node reported (ADR 0018, "Lanes by status, not by attention"). A session's
actor already sees every turn start and end and every permission request,
so the facts behind "working" and "blocked" exist; they never left the
node. Nothing in Vessel knows a session's pull requests yet, which is what
the three right-hand lanes are about.

Four questions:

1. Who decides a session's lane and phrase: the node, or each client?
2. Is the reducer Vessel's own design, or agent-orchestrator's ported?
3. How does the phrase cross the wire?
4. What does a node say about what it cannot see (pull requests), and
   where do ended sessions go?

## Options

**Who decides**

1. Each client, from the session's event log (replay it, track turns).
2. Each client, from raw facts the node sends (activity, flags).
3. The node, sending the result.

**The reducer**

1. Vessel's own, shaped by what a Vessel session reports today.
2. A port of `DeriveKanbanPresentation`, checked by its test table.

**The phrase**

1. Display text, as agent-orchestrator's daemon sends it.
2. A closed enum.

**What a node cannot see**

1. Leave the reference as is: between turns with nothing delivered is
   "Awaiting PR".
2. Say "Idle" when the node cannot see pull requests, "Awaiting PR" only
   when it can.

**Ended sessions**

1. Keep the reference's "exited but not archived" (the worker gone while
   the session lives on), archiving only on request.
2. Every ended session is the archive, whatever its status.

## Decision

- **The node decides (option 3).** `SessionInfo.board` carries the lane,
  the phrase, the activity they were derived from and the time of the last
  activity. Clients render it and do not reduce anything.
- **A port (option 2).** `vessel_session::board::derive` is
  `DeriveKanbanPresentation` in Rust over typed facts: the session's (the
  worker's activity, last activity, signal expected and seen, terminated,
  interrupted, an artifact, three fleet policy flags) and each pull
  request's (draft, merged, closed, checks, review decision, mergeability,
  updated at, the fleet's own review passes, people's reviews). The lane
  is chosen first, per pull request with the most actionable winning;
  then the phrase, from the facts that lane cares about. Every case of
  `kanban_test.go` is ported under its own name
  (`crates/vessel-session/src/board/tests.rs`).
- **A closed enum (option 2)**: `DisplayStatus`, matched exhaustively by
  clients.
- **"Idle" where the node cannot see (option 2).** The reducer takes
  `PullRequests::Unseen` or `PullRequests::Seen(&[..])`; between turns with
  nothing delivered is `awaiting_pr` only for `Seen`. No node can see pull
  requests yet, so every live session is in Building.
- **Ended sessions are the archive (option 2)**, as `archive` /
  `terminated` with activity `exited` and their end as the last activity,
  whether `exited`, `failed` or `interrupted`, live in this run or read
  back from the store.

How the facts are gathered:

- What the agent is doing is a projection of the session's records, as
  the store's `sessions` row is: every record the actor commits passes an
  activity tracker (`vessel-session/src/activity.rs`). `starting` is
  active. Structured: `Ready` and a turn's end are idle, `TurnStarted` is
  active, a permission request is `waiting_input` until every open request
  is resolved, a turn that ended `cancelled` is interrupted until the next
  one starts. Any final status is `exited` and terminated.
- A terminal has no turns: output is active, and one resettable timer in
  the actor's loop (`terminal::QUIET`, five seconds) makes it idle.
- The info watch wakes on a new lane, phrase or activity only;
  `last_activity_at` moves silently, as `next_seq` does.
- A stored session's board is derived when its row is read
  (`vessel-store`, `codec::stored_board`), never stored.
- No session expects a signal (`signal_expected` is false) and no fleet
  policy is on, so `no_signal` and the policy phrases are reachable only in
  the reducer's tests.

## Why

- **The node decides because only it has the facts, and once.** Replaying
  a log per client per card is a full read per session and still a guess
  for terminal sessions; sending raw facts lets two clients a version apart
  disagree on the same session. agent-orchestrator's daemon decides for the
  same reasons.
- **A port because the owner asked for theirs, and because their rules
  are already worked out.** Three of the four lanes are about pull
  requests. A reducer of Vessel's own today would have rules only for
  Building, and the next milestone would design the other three again,
  without the cases agent-orchestrator wrote for the traps it met (a merged
  pull request hiding live work, a stale review pass deciding a lane, a
  policy flag claiming work an idle agent is not doing). Their test table
  checks the port, so "faithful" is a test result, not a reading.
- **An enum because a phrase is one of a known set** (AGENTS.md section
  2): the client matches it exhaustively and a new phrase breaks every
  place that must show it, which text cannot do. The client owns the
  words; the enum keeps them in step with the lanes.
- **"Idle" because "Awaiting PR" would be a claim the node cannot make.**
  agent-orchestrator always has a repository and its pull requests; a
  Vessel session may be in no repository, and no node gathers pull
  requests yet. `Unseen` as its own value (rather than an empty slice and a
  flag) keeps "none" and "cannot tell" apart in the type.
- **The archive for every end because a Vessel session does not outlive
  its process.** agent-orchestrator's "exited but not archived" is a worker
  that died inside a session that still exists (its worktree, its pull
  request); a Vessel session ends with its process, so there is nothing
  left to show in a lane. The status word on the archived card still says
  how it ended.
- **A projection of the records because every record already passes the
  publisher.** Hooks in each actor would be two copies of the rules, and a
  move's dropped replay or an output drained after exit could slip past
  one. Reading what is committed gives exactly what the log holds, in
  order.
- **A silent `last_activity_at` because output is constant.** Waking every
  info watcher on each burst of terminal output would turn the watch into
  an output stream.

## Consequences

- Protocol 8 (`docs/PROTOCOL.md`): `SessionInfo.board`, optional, so no
  version gate. A client of an older node places sessions by status, as
  milestone 8 did.
- ADR 0018's follow-up "lanes by attention" is answered differently than
  it proposed: not Working / Needs you / Idle lanes, but agent-orchestrator's
  lanes with attention in the phrase ("Working", "Blocked", "Idle").
- Differences from the reference, stated in `board.rs` and none changing
  one of its cases: `Idle` where pull requests are unseen; Vessel's four
  activities (the reference's `blocked` is treated exactly as
  `waiting_input` there, and is `waiting_input` here); a pull request's lane
  is its own three-valued type, so the reference's fall-through arms do not
  exist. A case beyond the reference pins the whole ranking (its table
  never ranks a ready pull request against a live one in another lane).
- The store writes the ended-session rule itself, since it may not depend
  on `vessel-session` (ADR 0001). A node test holds it equal to the
  session's own last board and to the reducer; if the reducer ever places
  an ended session elsewhere, both change together.
- `last_activity_at` for a structured session moves on turns and
  permissions, not on message or tool-call chunks inside a turn: the turn's
  start already says the agent is at work.
- The terminal's five seconds are a judgement about real harnesses (a
  spinner, a pause to think), a constant rather than a setting.
- Next (milestone 9's "Left open"): the node gathers each session's pull
  requests, checks and reviews and passes them as `PullRequests::Seen`;
  Validating, In review and Ready fill with no change to the reducer, and
  `SessionBoard` gains the evidence for the card. An archive action, to
  take an ended session off the board, is not decided.
