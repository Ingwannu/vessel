# ADR 0052: Rewinding and forking a conversation, only where the agent can

Status: accepted, 2026-11-13 (milestone 11, track A2,
`docs/plans/backend-m11.md`, "Track A2: contract, 3." and its review).
The wire is protocol 14's `session.fork`, `session.rewind`,
`AgentAbilities.fork`, `TurnEnded.point` and the `forked` and `rewound`
records (`crates/vessel-proto/src/forking.rs`, `agent.rs`, `event.rs`).
Built as protocol 12 on track A2's branch; 14 on milestone 12's node
line, where 13 was already the schedule preview's.

## Context

GOAT, Paseo and agent-orchestrator let a person take a conversation back
to an earlier turn (rewind) or continue it from there beside the original
(fork). They do it through each provider's own API: Claude's SDK
(`forkSession` with `upToMessageId`, `resumeSessionAt`), Codex's
app-server (`thread/fork`, `thread/rollback`), OpenCode's
`session.revert`. Vessel drives every agent over ACP, and over ACP:

- No adapter rewinds in place: neither `claude-agent-acp` nor
  `codex-acp` has a rollback, revert or truncation method.
- Both fork (`sessionCapabilities.fork`, ACP's unstable `session/fork`),
  and both read an anchor in the request's `_meta`
  (`jetbrains.air.fork { version: 1, messageId }`, an agent message's ACP
  id): Claude forks up to that message, Codex after the turn that holds
  it; an id neither finds is `invalid_params`. The anchor is advertised
  nowhere, so an agent that does not read it forks the whole conversation
  without saying so.
- Files cannot be put back over ACP (Claude's `rewindFiles` is used only
  for a dry run).

## Options considered

- **A per-provider rewind** (GOAT, Paseo): Vessel would reach around ACP
  into each provider's SDK or files. Refused: Vessel speaks ACP to every
  agent (ADR 0003), and a rewritten transcript is the provider's format.
- **A fork that carries the transcript as context** to a fresh session
  (agent-orchestrator's approximate replay, Paseo's "fork into a new
  tab"): works for any agent but is not the agent's conversation (no tool
  state, a budget of text). Not taken now; listed as not offered.
- **An id on every message chunk** (the contract's first form). Replaced
  by the review: a point is a turn's, so the turn's end carries it
  (records stay their size; one rule in one place).

## Decision

- **A point is a turn's end**: `TurnEnded.point?: TurnPoint { message,
  conversation }`, set by `vessel-agent`'s driver when the turn's last
  agent message carried an id and nothing but the turn's end followed it
  (a thought, a plan or a tool call after it would be cut off: Claude's
  `TodoWrite` arrives as a plan). It carries the conversation it is in,
  so a fork from a turn a rewind left behind takes the conversation that
  still holds it.
- **Fork** (`session.fork { session, after? }`, operate): a new session
  of the caller's, through `session.create`'s own path (refusals, grants,
  an agent's limits and credential, a routed profile's token; its MCP
  servers resolved for it), in the source's folder, with the source's
  current choices, titled "<title> (fork)". Its agent starts with
  `session/fork` instead of `session/new` (the whole conversation without
  `after`, refused while a turn runs). Its log starts with `forked {
  from, after? }`; the earlier conversation is the source's log, not
  copied. It answers at creation; a fork that fails its check ends
  `failed`.
- **Rewind** (`session.rewind { session, after }`, operate): between
  turns, a parked session woken first; the actor holds input as for a
  move, stops the agent, starts it on a fork at the point, and goes on
  with the fork. The log gets `rewound { after, harness_session,
  files_changed }`; what came after stays in the log and in the agent's
  history. On any failure the old conversation starts again (with the
  session's current choices) and the call says why; the log then has no
  `rewound` record.
- **Checked, not trusted**: a fork at a point is followed, on the same
  connection, by `session/load` of the fork (both adapters load a session
  they just forked), and the driver reads its replay: it must end with
  the anchor's message and no user message after it. Otherwise the
  handshake fails "the agent did not fork at the point" (`unsupported`).
- **The new conversation id is in the record**: a restarted agent's
  `ready` is dropped with its replay, so `rewound.harness_session` is
  what the store's projection and the session's info take, and what a
  later move, resume, export or fork reads.

## Why

ACP is what reaches Vessel, and a fork at an agent message is the one
operation both adapters offer; a rewind built on it is what Paseo does
for Claude (a fork at the message, then the session swapped to it). The
check costs one replay on a person's explicit request and turns an
adapter that ignores the anchor from a silent wrong answer into a refusal.

## Consequences

- Only agents that advertise `session/fork` (and `session/load`, for the
  check) fork or rewind; sessions whose turns were recorded before 14
  have no points, so they fork whole only.
- Files are not put back. `rewound.files_changed` counts the tool calls
  left behind that edited, deleted or moved files, so a client can say so.
- A fork's history is in its source's log; once the source is deleted, or
  retention prunes the fork's first records (past 64 MiB of its log), a
  client shows the fork's own records only. Its agent keeps the history.
- The anchor is another client's `_meta` extension
  (`jetbrains.air.fork`); both installed adapters read it from any
  client. If a later ACP version names a point, Vessel moves to it.
- Not offered: a point inside a turn; a failed turn as a point; restoring
  files; a rewind that deletes what came after; merging a fork back
  (GOAT); a fork for agents without `session/fork` that carries the
  transcript as context.
