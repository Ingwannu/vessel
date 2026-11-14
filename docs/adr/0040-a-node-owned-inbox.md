# ADR 0040: A node-owned inbox, folded by each client

Status: accepted, 2026-11-13.

## Context

The session log already records permissions, questions and their resolution,
turn ends and failures. A person must currently open each session to find
them. Paseo keeps attention on its server and clears finished attention when
seen, but never clears an unanswered permission just because it was seen.
GOAT's `DecisionInbox.tsx` folds environments, reuses the conversation's
approval panel, disables answers without operate scope, and keeps failed
environments visible instead of pretending the list is complete.

## Options

1. Derive everything in each client from every session log: no migration,
   but a reload needs every log, retention loses requests and devices disagree.
2. Put the whole fleet's inbox on its home: one read, but duplicates state,
   needs forwarding when the home is away and changes the trust boundary.
3. Each node projects its own log transactionally; clients fold those lists.

## Decision

Option 3. A stored entry identifies its node, session and exact event
sequence. The node supplies the session's current title and project reference;
the approval carries its own tool title/detail and choices, and a question
carries its existing typed form. The store updates its projection in the
same transaction as the event: an answered/withdrawn request disappears;
a new turn removes the previous outcome; an ended session withdraws all
pending requests. A cancelled turn is not successful completion. Normal
completion, failure and token/request limits are distinct kinds.

Read/unread is one shared attention state per entry, like the existing
session marks. It is not an authorization grant and is not an answer.
Two devices with different scopes see the same read state. A read-only
device may read and mark that shared attention read, but it cannot answer;
answers use the existing operate-scoped session calls and first answer wins.
Agents, including forwarded or delegated agent origins, cannot mutate a person's read state. Revoking a device closes its
connections and removes its push subscription; it does not undo attention
the owner already read, or hide entries from the remaining devices. Pairing
a new device reveals the current node state rather than creating unread
copies. Read state is shared only among devices paired to that node; no
cross-node replication is introduced.

Lists are bounded pages with a stable descending cursor. Clients preserve a
machine's error/offline state and never claim its missing entries were
resolved. The app's inbox, waiting lane and sidebar use one UI-independent
view model; only the inbox draws its page. Approval and question answers
reuse the conversation's controls and words. Opening an item carries its
session and sequence to the existing conversation navigation seam.

## Why

The log and projection cannot disagree after a crash. No client must read
every log to discover attention, and the relay remains a blind transport.
Reusing the session actor's answer path keeps simultaneous devices subject
to the same form validation and first-answer rule.

## Consequences

- Schema adds inbox rows and bounded tool projections. Session deletion and
  retention remove their rows. Outcome retention is bounded; pending requests
  remain until their actor resolves them, within its existing request limits.
- Protocol 19 adds list/read calls. Minimum protocol stays unchanged. Old
  clients receive no new unsolicited frame; old nodes reject new methods
  before transmission through `Call::since`.
- Per-node pages are folded by the client; partial and disconnected machines
  must be explicit. Shared read state means viewing on a restricted device
  clears the unread dot elsewhere, while the waiting request remains.
- This deliberately does not add GOAT's separate durable recommendations,
  parent-agent routing or expiry decisions: Vessel has no such request kind.


## Projection lifecycle and bounds

The appended migration reconstructs only retained outcomes of already ended,
unarchived sessions. It uses the last retained turn end/failure, or the
session's retained attention record; pruned history is not fabricated.
Existing read-through and explicit unread marks are preserved. Migration and
backfill commit together and run once. Startup recovery's `Interrupted`
withdraws pending requests without inventing a successful completion.
Reopening a session clears its old outcome. Rewinding clears the abandoned
branch's attention and tool cache while preserving its log. Backfill considers
only outcomes after the latest retained turn start or rewind, so recovery
cannot revive a prior branch's limit or completion. Session deletion/retention uses
foreign-key cascade; archive removes its inbox entries. Read-through updates
only older entries, so a late read never consumes newer attention.

Each response has at most 16 entries; each complete request payload is at
most 256 KiB. Complete pending payloads share a 32 MiB node pool. An overflow
keeps an exact `InSession` request reference rather than clipping a question
or dropping answer options. The actor already limits each live session to
64 permission requests and 64 questions; compact references scale with the
number of live sessions rather than introducing a hidden global request cap.
Outcomes retain the newest 2,048 entries. Tool snapshots have a 32 KiB body
bound, 128 per session and 512 globally; truncated details are explicitly
`InSession`. Pending requests keep their own bounded tool snapshot after
cache eviction. Source records remain subject to the existing log retention.
Full pending forms remain independently in the live actor. Opt-in
`inbox.get {key,fetch:"live_request"}` recovers only that exact form, so
opening a large request never clones every pending form into one frame.
The response budget is 15 MiB below the 16 MiB frame limit; an over-limit
form stays an explicit compact reference, never clipped options. A gone
actor request returns absence. Ordinary background confirmation uses the
default summary mode and never expands large content. The CLI exposes this
selected recovery as `inbox show ENTRY`; list pages stay bounded and inspecting
one form does not mark it read. Tool details are a separate bounded snapshot:
when an update no longer fits, the prior snapshot is invalidated, not presented
as current. Omitted tool content can only be inspected if session history still
retains it; this is not a promise that pruned tool content can be recovered.

`before` is an exclusive, monotonically assigned descending cursor. Reads
never reorder entries. Deletion may shorten a page; new insertions are newer
than its boundary and appear on a head refresh, without duplicating prior
pages. An empty terminal page ends traversal. These are moving pages, not a
frozen historical snapshot. Counts are exact for one node's read transaction,
not the number of loaded entries. `inbox.get` rechecks an exact source when
an answer is pending: slipping off a partial page is not proof of resolution.

Initial/reconnected snapshots are silent. Desktop delivery uses per-node
cursor watermarks, not unread count changes. The push worker subscribes to
committed attention changes before reading its initial highest-assigned
cursor; existing/backfilled entries never replay on startup. A 250 ms burst
coalesces into one generic push opportunity; there is no durable retry queue.
Before delivery it rechecks current unread/live attention, device existence,
revocation, effective Read scope, current subscription, lease and presence.
A 404/410 removes only the exact attempted subscription, so a late response
cannot delete its replacement. Read/answer/revoke during coalescing suppresses
it; if a different eligible source remains inside the captured burst, the
worker admits that exact source instead. Failed delivery leaves the durable inbox available for reconnect.

## References

Paseo: `packages/server/src/server/agent-attention-policy.ts`,
`packages/app/src/utils/agent-attention.ts`, `packages/cli/src/commands/permit/`.
GOAT: `apps/web/src/components/decisions/DecisionInbox.tsx` and
`apps/web/src/components/decisions/useDecisionInbox.ts`.

## Marks ahead of persistence

The runtime may publish attention before its durable channel drains. Archive
waits for publication to end; it does not serialize that drain. Therefore each
inbox insert reads the stored session marks in its append transaction. An
archived session admits no new entry. Other entries inherit read state through
`KeptMarks::marks(Some(source_seq))`, including explicit unread and the kept
exclusive read watermark. Read marks update existing entries with that same
kept watermark, even when an older read clears an explicit unread mark. The
backfill uses this same insertion rule. A second mark engine or a wait for all
persistence would add ordering and latency without strengthening this keeper.

Every fleet hop must decode its full typed call. The common recursive
`Call::since` rule applies to the inbox too; checking only the envelope's
version would expose protocol 18 relays to protocol 19 methods. No scope or
forwardability policy changes with this correction.
