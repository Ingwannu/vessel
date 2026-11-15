# ADR 0026: Ended, archived, started again; marks kept by the node

Status: accepted, 2026-11-11 (milestone 11, track A,
`docs/plans/backend-m11.md`, "Track A: contract" and its review; the
wire is protocol 10, `crates/vessel-proto/src/conversation/marks.rs`).
Amends ADR 0019's rule for ended sessions.

## Context

Until protocol 10 an ended session could not continue (except by moving
it to another profile while it ran), so ADR 0019 put every ended session
in the board's archive lane. Track A adds `session.resume`: an ended
structured session starts again with its conversation, as
agent-orchestrator's restore, GOAT's resume and Paseo's resume-on-prompt
do. All three also let a person pin, archive and bring back, delete and
mark sessions unread; GOAT and Paseo keep these on the server,
agent-orchestrator keeps unread in each browser.

agent-orchestrator, whose board reducer Vessel ported, separates the two:
a worker whose process exited is in building as `exited` until someone
restores or archives it; archiving kills it and puts it in the archive
(`terminated`).

## Options

**The board**

1. Keep "ended is archived": a session started again would leave the
   archive and come back, and a person's "archive" would change nothing
   on the board.
2. agent-orchestrator's rule: archived is in the archive; ended and not
   archived is in building as `exited`.

**Unread**

1. A flag the node sets when a turn ends and a client clears: a `read`
   and a turn ending at the same moment disagree depending on order.
2. A comparison of positions: the last record asking for attention
   against the point the last `read` named.

**Starting again**

1. A new session resuming the agent's conversation (already possible
   with `SessionMode::Structured.resume`).
2. The same session, its log continued, on the profile-move machinery
   (ADR 0011).

## Decision

- The board follows agent-orchestrator: `SessionFacts.worker.is_terminated`
  means archived. An ended session that is not archived is in building as
  `exited`. The store's schema 7 archives every session that ended before
  it, so the board looks as it did. A version 8-9 peer still gets every
  ended session in the archive.
- `session.mark` sets `pin`, `unpin`, `archive` (a live session is stopped
  first), `unarchive`, `read { through }` and `unread`; the store keeps
  them. Unread is derived: marked unread, or a record asking for
  attention (a turn's end or failure, a permission or a question asked,
  the session's end) at or after the last `read`'s `through`, which only
  moves forward. Agents may not mark read or unread.
- `session.resume` starts the same session again: same id, same log
  continued after its final status, its title, choices and held queue
  kept, `session/load` under its current profile with the agent's replay
  dropped, as a profile move does. It answers once the agent is ready, or
  with the reason it is not (`unsupported` when the agent cannot resume,
  `spawn_failed` when it did not start). The store reopens the row before
  anything is published, so retention never prunes it mid-start; a
  subscription served from the store while it reopened ends `lagged`.
- `session.delete` removes an ended session's log, marks, queue and
  attachments; a live one is `conflict`. Retention prunes archived
  sessions first, then other ended ones, pinned ones last. Since the
  merge with track B, deleting a session, or retention dropping it, also
  tidies the worktree made for it by ADR 0027's rule, and the delete's
  answer says what became of it (`SessionDeleted`); archiving does not,
  so a session started again finds its folder.

## Why

One board for agent-orchestrator's rule and Vessel's: a person's archive
is what puts a card away, and a card that can start again is not put
away. Positions instead of a flag make unread commute with the log, so
every client agrees whatever order a `read` and a turn's end arrive in.
The same session continuing keeps its id, its subscribers, its marks and
its board place, which is what the references' restore means.

## Consequences

- Sessions that end on their own stay in building as `exited` until
  someone archives or starts them again; sessions agents create pile up
  there as agent-orchestrator's workers do. A page can group or hide
  them; the node does not archive on its own.
- "A final status is the last record" holds per run, not per log; the
  docs and the CLI read it so.
- A resumed session's agent tools get a new credential, granted as a
  session the caller creates would be (ADR 0013); `created_by` stays.
- Unread before any `read`: every record asking for attention is unread
  (a `read` through 0), as a new message is; a session with no such
  record, one stored before protocol 10 included, is not.
- `read`'s `through` may not pass the end of the log as published
  (`invalid_request`): `read` only moves forward, so a `through` past the
  end could never be taken back. For a session the runtime has, the end
  is its ring's, which a record a follower received never passes.
- While the runtime has a session, its marks are the store's copy: a mark
  is applied in the store first, and its answer handed to the session's
  handle under one node lock, so the two cannot disagree. The info adds
  them (`SessionHandle::info`) with the attention point the publisher
  reads off the records it commits (the store's projection uses the same
  rule, `SessionEvent::asks_for_attention`). The info watch carries no
  marks: its sender closes with the session, and a session is archived
  after its end.
- The session runtime's reducer never sees an archived session (no record
  says so, and archiving comes after the end), so its facts say
  `is_terminated: false`; an archived session's info is placed in the
  archive (`SessionBoard::archived`) where the marks are added, which is
  what the reducer's first rule gives.
- `archive` keeps the first time it was archived; any `read` clears a
  marked `unread`, whatever its `through`. `archive` on a live session
  answers once the session's final status is published, not once the
  store has its whole log.
- `session.delete` of a session whose final status is out but whose log
  is still being written is `conflict` ("try again"), as the review said;
  the store refuses a row it still shows live too.
- Retention's order is an index (schema 8, `sessions_retention`, which
  replaces schema 1's `sessions_ended`): the pruning walks it instead of
  sorting every ended session each time one ends. A pruned or deleted
  session's attachment folder is removed by the store's writer right
  after the commit that removed its row.

## As built: starting again (slice 5, 2026-11-11)

The decision stands; these refine it where it said nothing.

- **Order.** The runtime registers the session, its agent already
  starting, and publishes nothing until a gate opens
  (`vessel_session::Gate`); under the marks' lock the node makes it
  visible, the store reopens its row (`Store::reopen`: `live`, `starting`,
  no end, out of the archive, only for an ended row whose log ends where
  it was read) and the handle takes the store's marks; then the gate
  opens. A row the store will not reopen (deleted or pruned meanwhile)
  drops the gate, and the session ends without a record. Reopening first
  would leave a moment where the store shows a live row the runtime does
  not have, which a subscription the store serves cannot tell from a row
  a failed store left live.
- **What leaves a record.** An agent that cannot be executed (its spawn
  fails) is refused before the row is reopened, so that attempt is not in
  the log; one that starts and fails its handshake or `session/load`
  leaves `starting`, then `failed` with the reason, and the call answers
  `spawn_failed` with that reason.
- **The queue** comes back held `ended`, also when the node stopped
  before the session's end could hold it, whatever held it before (as a
  session's end does); `session.queue resume` lets it move.
- **One at a time.** A start again claims its session for the call; a
  second is `conflict`. The agent's credential and model token are now
  retired in the hand-over to the store (they already refused once the
  end was visible), so the end of one run never takes the places of the
  next.
- **Marks.** A stored session's mark takes the marks' lock and looks the
  session up again under it, so it lands either before the reopening,
  whose answer carries it, or on the running session after.
- **Subscriptions** the store serves end `lagged` from their cursor when
  the runtime has the session again, read on when its stored log grew (a
  run started and ended meanwhile), and end `session_ended` otherwise.
- **Abilities.** A live session's info now carries the abilities of its
  agent's last `ready`, as the store's row does; a session started again
  shows its stored ones until its new `ready`.
- **Routes.** A routed session keeps its route (ADR 0015), as a move
  does: if its profile's route changed since, starting it again is
  `unsupported`.

## Amendment (2026-11-15): waiting for the hand-over

- Between a session's end and the store serving it, its log is still being
  written. `session.delete` and starting a session again were refused at
  once in that gap (`conflict`, "try again"), which a client that has just
  seen the end cannot avoid: an agent that archives a session and then
  resumes it met it (the shared MCP test, on a loaded build machine).
- Both now wait for the hand-over, at most `HAND_OVER_WAIT` (10 s, the
  same budget a side question's reset already used): delete until the store
  serves the session, start-again until the store serves it and its last
  run's entry is released (`hand_over` publishes the end after both). Past
  the wait the answer is `conflict` as before, so a store that never
  answers still gets a typed reply rather than a hung call.
- Reference: Paseo's delete closes the agent and flushes its queued
  persistence before removing it, so it waits rather than refusing.
