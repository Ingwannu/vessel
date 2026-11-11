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
  sessions first, then other ended ones, pinned ones last.

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
