# ADR 0033: Schedules, time, who a run acts as, the starter told, saved prompts

Status: accepted, 2026-11-12 (milestone 11, track D,
`docs/plans/backend-m11.md`, "Track D: contract" and its review; the wire
is protocol 11, `crates/vessel-proto/src/automation/`; the node's part is
`crates/vessel-node/src/automation/`).

## Context

The owner asked that Vessel do nearly everything agent-orchestrator, GOAT
and Paseo do. All three run agents on their own: agent-orchestrator's
automations (RRULE or a narrow cron, a time zone, run history, bounded
catch-up), GOAT's scheduled tasks (an interval, a time of day, a webhook;
a launch-watch that tells the launching thread when a launched one ends),
Paseo's schedules and heartbeats (cron, a time zone, a run cap, an expiry,
run now; finish subscriptions). agent-orchestrator alone saves prompts
(Cues). Before protocol 11 Vessel had none of these: an agent that started
a session had to wait on it.

Running something later, unattended, raises questions the references
answer differently or not at all:

1. What grammar says when, and which code computes the next time?
2. What happens at a daylight-saving gap or fold, when the node was
   stopped across due times, when the clock steps?
3. Whom does a run act as, and how is an agent kept from using a schedule
   to outlive or outgrow its session?
4. What does a run leave behind, and what if nobody is there?
5. How is a session's starter told, without two sessions waking each other
   for ever?
6. Where do saved prompts live, and is a prompt in a repository's file
   someone else's code?

## Options

**Grammar**

1. RRULE (agent-orchestrator, through `rrule-go`): the most expressive; a
   large grammar (RFC 5545 with DTSTART, rebasing on a zone change), used
   by one reference, which itself narrows cron to daily and weekly times.
2. Five-field cron and an elapsed interval (Paseo, GOAT's interval): what
   two references and every cron agree on.

**Computing the next time**

1. A cron crate: `cron` needs a seconds field and has open reports of
   daylight-saving mistakes; `croner` brings `chrono-tz`'s generated zone
   tables and its own rules for gaps and folds.
2. Paseo's way: step a real instant one minute at a time and read the
   wall clock (a year of minutes per search; a time a gap skips never
   runs).
3. Walk wall-clock minutes that match, field by field, and turn each into
   an instant by the zone's rules, with a maintained zone database.

**The zone database**: `chrono-tz` (code generated at build), the
operating system's (`/usr/share/zoneinfo`; nothing on Windows), or
`jiff`'s bundled copy.

**Waking**: a ticker (agent-orchestrator 15 s, GOAT 5 s, Paseo 1 s reading
every schedule's file), or a sleep until the earliest due time.

**Missed due times**: several runs (agent-orchestrator, up to 3), one
(GOAT, Paseo during a run), none (Paseo at start).

**Who a run acts as**: nothing (agent-orchestrator, Paseo), the creator
and a ceiling stored (GOAT), the definer rebuilt at each run.

**The starter told**: steered into the starter's running turn (Paseo),
queued after it (GOAT); per prompt (Paseo) or per launched thread (GOAT);
loop guard by ignoring the reverse watch's own run (GOAT) or none (Paseo).

**Saved prompts**: in the node (agent-orchestrator), in a file of the
repository.

## Decision

**1. Grammar.** Five-field cron (lists, ranges, steps on `*` or a range,
month and day names, the `@daily`-style shorthands; no seconds, `L`, `W`,
`#`, `?`, no step on a single value, no wrapping range) with an IANA zone,
and `every` (60 s to 366 days of elapsed time, from an anchor the node
fills when it is left out). No RRULE: cron says what agent-orchestrator's
templates say, `every` the intervals cron cannot ("every 14 days"), and a
later `Cadence` variant can add RRULE without changing the rest (`Cadence`
has `unknown`). Parsed in `vessel-proto` into bit sets, so a client knows
before sending whether the node will take an expression.

**2. Computing.** Option 3, written here (two hundred lines and their
tests): `vessel-node`'s `automation::time` walks matching wall-clock
minutes and asks `jiff`, with its bundled IANA database, how each maps to
an instant. The rules:

- A time a gap skips runs once, at the instant of the jump (Vixie cron's
  rule: `30 2 * * *` in New York on the spring night runs at 03:00 EDT);
  several times in one gap, and the hour after it, make one run, since
  the next due time is always strictly after the last.
- A time a fold repeats runs once, the first time, for every expression
  (Cronie runs a `*`-hour expression in both passes; here a cron is about
  the wall clock and `every` is the elapsed rhythm).
- A cron with no matching day within 8 years is refused (`0 0 30 2 *`);
  every expression that matches some day matches one within 8 years.
- The bundled database, not the system's: the same times on every
  platform and in every test, and the node reads no zone file. The cost:
  a zone whose rules change after a release is computed by the old rules
  until the next.

**3. Waking and missing.**

- The scheduler is one task owning the schedules; due times are instants
  in an ordered index. It sleeps until the first, at most 10 minutes at a
  time (a clock stepped forward is noticed within 10 minutes), and wakes
  on every change. A wake looks at the index's first entry, never at
  every schedule; with nothing active there is no timer.
- A schedule keeps its next due time and the due time of its last run.
  The clock stepped back under 24 hours never runs a due time twice; a
  last due time more than 24 hours ahead of the clock means the clock was
  wrong when it ran, and the next is computed from now.
- Missed due times make one run at most, however many: with `catch_up:
  once` (the default) for the latest, if it is less than 24 hours old;
  otherwise, or with `skip`, one `skipped { missed }` record. A due time
  noticed less than 60 s late is simply due.
- A run is recorded `making`, in the same write as the next due time,
  before anything is sent; a node stopped meanwhile finds it at start and
  marks it `interrupted`, never making it twice.
- At most 4 runs are made at once on a node, one per schedule; a slot is
  freed when the run has its outcome.

**4. Who a run acts as.** The schedule's `defined_by` (whoever last wrote
its definition), rebuilt **as it is when the run comes**: the owner; a
device with its scopes now (revoked: the schedule ends); an agent whose
session is still live, with its grant. The run goes through the checks a
call of theirs would: the scope, ADR 0013's ownership and limits, ADR
0027's place rules. Updating a schedule makes the updater its definer, so
a device cannot leave the owner's name on a prompt it wrote.

Agents: at most 8 schedules per agent session; a heartbeat (a schedule
whose target is the agent's own session) is the one exception to "an
agent does not own its own session", for a prompt through its own queue.
Every schedule an agent made is deleted when its session ends, and at the
node's start; agents cannot run a schedule now (they would pump their own
heartbeat past its cadence). A delegated caller (`fleet.forward`) cannot
create or update a schedule: what it may do depends on the home's grant
and on an agent of another node, which cannot be rebuilt when the run
comes.

**5. What a run leaves.** A new session's run, by default, archives the
session (ADR 0026) once its first turn ends and nothing waits in its
queue (Paseo's `archiveOnFinish`): without it, every run of an hourly
schedule would leave one more agent running. `keep` leaves it live, and
the schedule's next run waits for it to end (agent-orchestrator's one live
session per automation). A scheduled session's agent is offered no
questions; a permission makes the run `waiting` until a person answers or
archives it, and the schedule's later runs skip meanwhile. Nothing
automatic (a scheduled prompt, a heartbeat, a notice) starts a session
whose last turn a person stopped (its board says `interrupted`): ADR
0024's rule, kept for prompts that come after the stop.

**6. The starter told.** `session.prompt { notify: true }`, asked by a
session's agent, tells that agent's own session about this prompt's turn,
exactly: the session's actor says which turn the prompt started (at once,
or when its queue entry starts). Each notice is a prompt queued into the
told session (never steered into a running turn), once per event: each
permission or question the turn asks (`needs_you`, at most 8), then its
end or the session's (`finished`, `ended`). The told session is always
the caller's own and the watched one descends from it, so notices only go
up the tree of who started whom, which has no cycle: notices alone never
wake two sessions in turn, and a loop needs an agent to prompt again each
time, as any agent loop does. Asked for, not automatic: an agent that will
call `wait_turn` anyway would pay for a second turn to read a report it
has. Bounded: 32 waiting per told session, 256 per node, 60 notices an
hour into one session.

**7. Saved prompts** live in the node's store, per project or for the
machine, with a `SessionSpec` saying what a session started from one runs
with. Not in a repository's file: a prompt steers an agent that may run
without asking, so a prompt the repository supplies would be someone
else's instructions in the sense ADR 0027 gives commands, and would need a
second approval flow. Agents may list them, not write them: a saved prompt
is later sent by a person who may not read it again.

## Why

- A walk over wall-clock minutes decides every daylight-saving edge by a
  rule we state and test, where a crate's rule would be another document
  to keep in step; the zone database is the one part not worth writing.
- Sleeping until the next due time is what a node at rest can afford (a
  node used 3 s of CPU in six hours; a 1 s ticker over every schedule
  would be most of it).
- One run for any number of missed times, and none for a day-old one,
  because a prompt written for nine o'clock is not wanted at nine the next
  evening, nor thirty times after a sleep.
- Rebuilding the definer at each run keeps ADR 0010's and 0013's promises
  in time: a revoked device stops acting, an agent's schedule dies with its
  session, and grants only shrink.
- Archiving after the first turn and offering no questions are what make
  a run unattended in fact, not only in name.
- Tying a notice to the turn a prompt started, through the actor that
  starts it, is the only way the report is about the right turn when the
  prompt was queued behind another.

## Consequences

- Protocol 11: the `automation` module, thirteen methods for schedules
  and saved prompts (the six for hooks are designed in ADR 0034 and not
  built), `PromptSession.notify`. Schema 12 (13 on the track's branch,
  renumbered at the join).
- `vessel-node` depends on `jiff` (bundled database; Unlicense or MIT);
  `vessel-cli` too, with `tz-system`, to name the zone the machine is in
  when `--tz` is left out.
- `vessel-session`'s actor reports which turn a delivered prompt started
  (a small addition to its delivery: a ticket per prompt).
- A wall-clock rule that differs from Cronie's for `*`-hour expressions in
  a fold; `every` is the alternative, and the ADR says so.
- RRULE, a new worktree per run, and the starter told across nodes are not
  built (the plan's "Not in this track").

## Amended at the join (2026-11-13, "Integration 3 (node)")

Where track D met the node join (pull requests, terminals) and track G
(added agents), and what its second review found:

- **Room for runs, in order** (decision 3). The scheduler hands out room
  itself: a run recorded `making` waits, keyed by its due time and its
  schedule, until one of the 4 slots is free and, for a run that starts a
  session, one of 8 first turns: at most 8 scheduled sessions are in
  their first turn and not waiting for a person at once, so a catch-up
  after a night off cannot start every schedule's session within seconds.
  The first waiting run that fits goes first (oldest due, then by id), so
  a prompt through a queue is not held behind sessions waiting for a
  turn. An asked run waits 30 s at most on the scheduler's clock, as due
  times are measured, then is recorded `failed` and answered
  `unavailable`. Track D's tasks each waited on a semaphore, whose order
  was the runtime's.
- **What a run leaves** (decision 5). A run's session is put away by the
  service's one archive path, which also ends the terminals and services
  beside it and the watch of its pull request (ADRs 0029 and 0031), as a
  person's archive does; a session that started but got no prompt is put
  away at once. A run is the definer's own `session.create` and prompt,
  so it opens no terminal, merge or approval they could not get
  directly; no run makes a worktree or runs setup or a script. A
  follower sees the run `Finished`, and the session's final status,
  before the archive mark is stored (the archive ends the session first,
  so "archived" always implies "ended"); it must not read `archived_at`
  at either, but wait for the mark (2026-11-14, the e2e test that did).
- **An agent's schedules** (decision 4). A make that waited while the
  agent's session ended is refused; ends the scheduler fell behind on are
  made up by checking every agent's schedule against its agent; a
  session started again (`session.resume`) deletes its last run's agent
  schedules before its new grant, so a heartbeat never outlives the run
  that set it. An agent's `every` is anchored by the node when it writes
  it, so rewriting the anchor cannot run a schedule at once. A person
  taking an agent's schedule over counts toward the people's bound.
- **Bounds the wire needed.** A spec kept with a schedule or a saved
  prompt is at most 16 KiB encoded, as the prompt beside it; an `after`
  the node does not know is refused, not stored (it would neither
  archive nor wait).
- **The starter told** (decision 6). A notice is a prompt the told agent
  reads as from a person, so the session's own words in it (its reply's
  end, what it asks, why its turn failed) go between tags whose name is
  random, as ADR 0034 fences a hook's body, with a line saying they are
  that session's output. Without randomness the words are left out. A
  notice's ticket names the turn by its number among those the actor
  started, so an interrupt whose cancelled turn's record arrives late is
  never reported as the prompt's own.
- **Added agents** (ADR 0032). A new session's spec names its agent by
  `HarnessRef`; a run whose added agent was removed fails `not_found`, one
  whose secret is missing or (one of the four) is being installed fails
  `conflict`, and the schedule goes on.

## Read-only previews and machine zone (2026-11-13, N3, protocol 13)

A preview calls the existing `Due` calculation on the node. Duplicating it
in a client was rejected: the database and DST policy could diverge. The
reply includes the effective cadence, with the interval anchor filled at
one clock reading, so create/update can preserve it. A historical preview
is about that cursor; saving later still schedules only future instants.

Four blocking workers at most, admitted before enqueueing, hold permits
until they exit even if the caller disconnects or cancels. Ten requested
runs at most, eight years per search, no partial result on failure.
Read scope suffices because nothing is saved or started. The optional typed
error detail distinguishes zone, cadence, time-range and no-match failures.

`NodeInfo.timezone` is a best-effort startup observation from jiff's
`try_system` with `tz-system` (Unix system configuration and Windows system
zone mapping). A zone without an IANA name or a detection failure is absent;
there is no UTC fallback. This does not modify the system's settings. The
bundled IANA database still determines all schedule calculations. Testing
Windows system detection itself requires a Windows runner; it is not
claimed from Linux. Older handshakes and node.info replies omit the field.

Preview time inputs are checked against jiff's `Timestamp::MIN` and `MAX`,
including subsecond validity. These bounds reserve room for every legal
UTC offset, so the first and last civil days of the extreme years are not
all supported instants. A request outside those bounds is `time_out_of_range`;
a search with no remaining cron instant is `no_future_run`. Neither returns
a partial run list.

## Proposed amendment: durable finish subscriptions (2026-11-15, row 618)

**Status: proposed, not accepted or implemented.** The accepted decisions above
remain in force until this security-relevant amendment is reviewed. W18 owns
subscription intent and the existing notice watcher; W23 owns submission
receipts, authoritative actor-ticket binding and internal notice admission.
W10 renders the canonical timeline notification. W7 retains OS notifications
and Inbox. This proposal registers no RPC or schema and assigns no number.

### Evidence, problem and alternatives

Paseo 0.15.4, source revision `1c00b365c9e0`, restores durable finish
subscriptions. `packages/server/src/server/agent/finish-subscriptions/index.ts`
lines 16–61 define its record, observation and dispatch acknowledgement
boundaries; startup groups saved records and attaches one observation per
caller/child/destination. Its `internal/json-store.ts`, `observation.ts` and
`observation-buffer.ts` preserve unacknowledged intent, wait for recovery and
dispatch, and deduplicate permission checkpoints. Accepted replacement and
rejected candidate are distinct; a late acknowledgement removes only the exact
record it captured. Shutdown leaves unacknowledged records for recovery.

Vessel's `automation/notices.rs` currently explains that nothing is stored
because sessions do not outlive the node. That implementation assumption is
insufficient once structured sessions can recover under ADR 0059. Simply
starting another log follower on boot can miss the actual queued turn, consume
a transient startup interruption as a finish, or duplicate a parent prompt
whose acknowledgement was lost.

Options considered:

- Keep notices transient: small and consistent with the original runtime, but
  loses a requested finish subscription across a restart.
- Copy Paseo's JSON catalogue and add a second watcher: local file recovery is
  possible, but cannot atomically deduplicate Vessel's parent queue/event and
  submission receipt, and duplicates the authoritative observation owner.
- Extend the existing SQLite Store actor and existing notices/notify watcher,
  sharing W23's receipt owner for delivery and exact-turn binding. This is the
  proposed choice: no dependency, second observer or second receipt store.
  It requires explicit transaction and recovery work rather than treating
  current in-memory tickets or a saved file as proof of durable admission.

### Intent identity, source identity and finite storage

`FinishIntentId` is one node-minted UUID, allocated once before registration
and never reminted to retry an uncertain effect. It is not the stream RPC's
`SubscriptionId`. A durable intent is bound to the local owner `NodeId` and
contains these typed fields:

| Field | Meaning and invariant |
| --- | --- |
| parent | Exact local `SessionId`, derived from a direct authenticated local Agent principal; never caller-selected or defaulted to Owner. |
| target | Exact `{ NodeId, SessionId }`; a route name is never target identity or durable authority. |
| ownership | Descendant ownership for prompt follow-up, or exact creation-parent ownership for creation callbacks; rechecked against authoritative parentage, including detach. |
| grant ceiling | Captured Read/Operate scopes only, intersected with the current authenticated parent grant at every effect. No credential, route token or provider secret is saved. |
| binding | Original bounded canonical W23 request, authenticated creator identity and fingerprint, then the actor's authoritative queued-entry/actual-turn binding. A remote binding also retains its exact destination and origin. |
| phase | PendingDispatch, Active, CapturedNotice or Acknowledged, with retained uncertain observations distinct from confirmed refusal/admission. |
| checkpoint | Authoritative source record/request identity and immutable captured notice payload, not a scan heuristic or next-turn guess. |

Replacement uses compare-and-set of the exact intent id for the pair
(parent, target). A pending candidate does not suppress its predecessor until
its original dispatch is authoritatively accepted; a rejected candidate leaves
the predecessor intact. Recovery coalesces an accepted replacement and its
predecessor into one observation. An old task or late acknowledgement cannot
remove, activate or overwrite a newer intent. An id collision is refused.

Existing named limits remain authoritative: `ON_NODE` (256), `PER_SESSION`
(32), `HOURLY` (60) and `NEEDS_YOU_MAX` (8). Pending and uncertain candidates
count against those finite admission limits too. Retained permission payloads
are bounded by `NEEDS_YOU_MAX`, plus one terminal/unavailable checkpoint per
intent; the checked aggregate follows from those limits. Captured content uses
the existing bounded prompt type and `REPLY_TAIL` (1500 characters), with a
separately checked aggregate encoded-byte bound beside its storage owner.
Durable rate accounting must not reset the hourly allowance on restart.

The proposed canonical reservation definitions belong beside W23's existing
`timeline/admission.rs` limits, not in a second accounting store:

| Named definition | Proposed value / charge |
| --- | --- |
| `MAX_NOTICE_KEY_BYTES` | 512 encoded UTF-8 bytes, checked before persistence. The actual key contains only fixed-size UUIDs, an EventSeq and closed enum tags. |
| `MAX_NOTICE_PAYLOAD_BYTES` | 16 KiB of canonical encoded captured payload, including its fixed text and randomized fence. Reject excess before capture/admission, never truncate an id or synthesize a source. |
| `MAX_NOTICE_STATE_BYTES` | 32 KiB for the complete key, source, payload, fingerprint, lifetime/provenance metadata and largest accepted/refused/uncertain/tombstone result. |
| `MAX_FINISH_ORIGINAL_REQUEST_BYTES` | 64 KiB of the FULL canonical encoded original request, checked before finish registration or its original dispatch. This is an internal finish-registration resource floor beyond Paseo, not an existing generic SubmitPrompt validation or a new limit on ordinary submissions without finish registration. |
| `MAX_FINISH_INTENT_STATE_BYTES` | 128 KiB for that bounded original request, parent/target/grant/binding and every non-payload intent field. The stored input permits exact read-only recovery without reconstructing pruned content. |
| `MAX_FINISH_CHECKPOINTS` | `NEEDS_YOU_MAX + 1`, using checked arithmetic: all allowed needs-you checkpoints plus one terminal OR unavailable checkpoint. Capturing ObservationUnavailable irrevocably ends this intent's observation; it can never later capture Terminal. |

Before any intent storage or original prompt effect, the same writer transaction
reserves the full intent envelope plus `MAX_FINISH_CHECKPOINTS` complete notice
envelopes, each with `ADMISSION_ROW_CHARGE_BYTES`. It charges one intent record
and that many checkpoint record slots against the existing
`MAX_SESSION_ADMISSIONS`/`MAX_NODE_ADMISSIONS` and corresponding byte limits.
Unused slots are reserved, not invented checkpoint keys; later capture binds a
slot to one exact key. All captured payload/receipt growth must fit those
already charged envelopes before any queue/event effect. Overflow refuses with
typed Capacity and leaves the original work unadmitted. Existing-key replay or
observation still works when fresh admission is full. Shared accounting may
refuse below the separate 32/256 active watcher maxima; those are ceilings,
not guarantees of free durable capacity.

Acknowledgement, uncertainty, replacement and log pruning never release these
retained reservations. Settled state becomes a compact replay tombstone within
its original charge. Only committed authorized WHOLE parent-session removal may cascade its intent/
checkpoint rows and release accounting atomically. Retention qualifies only if
it performs that same whole-session removal; ordinary log pruning, replacement,
revocation or an accepted acknowledgement never releases the charge. No new TTL
or unbounded retention choice is introduced.
On startup verify the worst-case encoded envelope and derived accounting before
admission; inconsistent rows refuse rather than reset/drop replay protection.
SQLite page/WAL overhead and the ordinary queue's existing content limits remain
separate; these charges bound logical retained admission state. The integrator
assigns migration/protocol numbers at wiring, not this proposal.

### Exact private notice key and W23 admission boundary

Proposed internal key, consisting only of bounded typed values:

```text
NoticeKey {
    owner: NodeId,
    intent: FinishIntentId,
    checkpoint: NoticeCheckpoint,
}
NoticeCheckpoint =
    NeedsYou { request: PermissionId | QuestionId, source: EventSeq }
  | Terminal { source: EventSeq }
  | ObservationUnavailable
```

The request alternatives are a closed Rust enum, not a string union. Source
sequences belong to the intent's exact target and authoritative actual turn;
`ObservationUnavailable` is a singleton for that intent and irrevocably ends
its observation. Its retained delivery may still be unacknowledged, but a later
reconnection/terminal record cannot restart the observer or add another terminal
checkpoint under that intent. It is an explicit unknown outcome, never a
fabricated terminal sequence or a claim of finish.
The first captured payload, reason and randomized output fence are persisted
within the reserved slot before admission; retries use that same payload/
fingerprint and key. A pruned current snapshot with no original source EventSeq
cannot mint a NeedsYou checkpoint: use an already retained exact checkpoint or
an explicitly uncertain/unavailable outcome. No synthetic sequence is allowed.

This is a **trusted internal namespace in W23's existing receipt/store owner**.
Public `SubmitPrompt` must neither accept nor select it. It must not reuse the
session/client-message user namespace or synthesize `Creator::Owner`.
A private producer entry point accepts the key, captured source/provenance,
immutable payload and an authenticated current parent-lifetime fence. A forged
public call cannot obtain that fence or turn a caller-supplied notice key into
authority. The new fence is an integration requirement, not an existing API.

W23 must retain the actor ticket's actual `EventSeq` or `QueueEntryId` binding,
including the later actual turn for a queued prompt, in its receipt owner.
Current `Started`/`Steered` are unit variants and interrupted disposition names
only the interrupted turn. Echoed client ids in history cannot recover this
binding after pruning. No history scan, tool-name branch or next-event guess
substitutes for it. Likewise, `SubmitPrompt` is read-or-admit: observing a
missing result must remain Unobserved, never authorize resend. The coordinator separately approved W23's one staged read-only
`ObservePromptReceipt { request: SubmitPrompt }` under `TIMELINE_PARITY=None`.
Its complete original request and authenticated Creator must match. The reply
is Unobserved, OutcomeUnknown, KeyConflict, Refused or Accepted with the
immutable original receipt and separate binding. That binding is Unobserved,
Queued, Turn, Withdrawn or Command; none authorizes resending. This amendment
adds no further node RPC and does not consume that unreviewed implementation.

Internal admission must durably return or replay **both the same canonical
parent notice EventSeq and the exact accepted QueueEntryId** for the same
NoticeKey and fingerprint. Subsequent queue-to-turn binding is separate metadata
and cannot rewrite that immutable acceptance. The same key with a different
fingerprint returns typed KeyConflict without overwrite or effects. Fresh
reservation exhaustion returns typed Capacity/no-admission, never a retained
Accepted or an instruction to resend. It must record queue/event,
receipt outcome and internal provenance atomically, or provide an equivalent
recoverable transaction in the same owner. The result must distinguish
Accepted, Refused, Unobserved and OutcomeUnknown; no timeout is confirmed
absence of an effect. This durable transaction is new work, not a statement
about the pending W23 runtime. The private key is not a field added to
`TimelineNotification`.

### Parent lifetime, captured grant and recovery admission

Registration is available only through the existing direct local-agent path,
after current Read/Operate, descendant/exact-parent and count checks. A
Delegated or foreign-agent principal cannot leave intent on another node or
inherit its owner. A saved record is intent, not authorization or a capability.
Every later enqueue rechecks the current parent grant intersected with its
saved ceiling and current parentage, after awaits and at atomic admission.

The runtime captures the exact parent execution lifetime before asynchronous
work and subscribes to parent termination before reading its live state.
Old work cannot cross an explicit new grant/session execution. Across restart,
only the lifecycle owner's authoritative recovery of the same committed parent
execution may rebind the intent to its new runtime fence. An explicit resume,
archive, delete, person stop or detach is not such a rebind. Never resurrect a
parent or child, unarchive it or turn a startup snapshot into admission.
Current `Agents::scopes`/`is_live` are observations, not a durable lifetime
fence; the shared atomic fence remains an implementation dependency.

Load and validate bounded intents before cleanup/retention or a query can
conclude that no subscription is pending. During load/reconciliation the
pending answer is conservative. Wait for the lifecycle recovery and dispatch
barriers before following exact accepted work or treating an interruption as
terminal. No ad hoc startup sleep is a barrier. Start one existing observer per
accepted pair and feed it the retained actual-turn binding and checkpoints.

If a binding is Unobserved or unavailable after pruning/remote expiry, retain
uncertainty or emit the single explicit unavailable outcome after the bounded
owner policy. Never attach to a later turn or reissue the original prompt with
a fresh id. The current expiring remote receipt is not by itself a restart
contract. Remote durable recovery requires W23's checked retained binding at
that exact node; losing/forgetting the route gives unknown, not worker finish.

Shutdown cancels active observation but preserves unacknowledged durable
intent. Parent termination revokes future admission and retires only matching
intent generations. The cancellation-vs-ack transaction must preserve an
already admitted callback outcome while preventing admission after revocation.
This ordering belongs to the shared Store/actor owner, not a stale UI or RPC
snapshot.

### Delivery, recovery and presentation

The existing `automation::notices`, `Teller`/`RemoteTeller` and notify watcher
remain the observation and notification path. Persist intent before following,
and capture a checkpoint before admission. Await the exact original dispatch
acknowledgement before selecting the active replacement. Checkpoint dedupe
belongs to the retained request/source identity, including permission answers
that arrive before recovery's first current snapshot.

Callback prompts retain ADR 0033's **Queue**, never steer/interrupt, policy and
its no-automatic-restart-after-person-stop rule. Queue acknowledgement means
durable admission, not that the model consumed it. After lost acknowledgement,
observe/replay only the same internal key through W23's owner. Once accepted,
a failed intent-removal commit retries that commit alone, not another callback.
After the final acknowledgement retire only the captured intent id. Permission
acknowledgements retain the terminal subscription and their dedupe checkpoints.

All waits use named finite owner budgets. Reuse the reviewed remote setup and
post-follow supervision boundaries (120 seconds and 5 seconds); setup timeout
or failed notice/close remains uncertain. Offline observation retries use one
named bounded policy, distinct from delivery/storage retries. As in Paseo,
connecting alone does not spend the unreachable budget; exhausted unreachable
attempts or a removed host produce unknown. No retry spins on all records or
creates a second periodic watcher. Persisted retry/rate state is bounded and
cannot be reset to gain effects by restarting.

W23's canonical `TimelineNotification { level, message }` remains the body.
The producer's stable `EventRecord.seq` identifies the timeline item; internal
provenance prevents an extra ordinary-user bubble. W10 uses its single alert
renderer and literal selectable text. No new SessionEvent presentation union,
body id, OS notification or Inbox owner is introduced here. Captured worker
output retains the randomized fence and safe diagnostic categories; malformed
records do not expose private payloads through Display/Debug/source chains.

### Required proof and integration holds

Before implementation, W23 must align its actual-turn binding, read-only
observation disposition and internal namespace/admission outcome with this key
and lifetime contract; V3 must review the amendment. Producer readiness, not a
None constant or a values carrier, controls later wiring. Row 618 stays pending.
No RPC/table row, schema, persistence or second watcher is added by this text.

Implementation acceptance requires actual isolated-home regressions for:

- Restart/process loss before and after intent commit, original dispatch,
  checkpoint capture, parent queue/event receipt, acknowledgement and removal;
  no duplicate callback or ordinary-user bubble, same canonical EventSeq.
- Pending startup recovery, accepted/rejected/unknown candidates, queued entry
  later becoming a turn, pruned history and remote binding expiry; no resend
  or falsely reported next-turn completion.
- Exact supersession and stale acknowledgement, repeated permission/current
  snapshots, grant shrink, parent end, detach, explicit resume/person stop,
  foreign-node records and removed routes.
- All named slot/payload/storage/rate/retention limits across restart; full
  capacity refusal, cancellation and held setup/notice/close futures; malformed
  versioned storage and canary diagnostics without silent empty-state reset.

Paused futures and pure model tests prove mechanisms only. Actual crash/restart
and cross-node tests, the changed-source strict/functional/whole gates, canonical
producer integration and final timeline consumer review remain separate proof.
The benefit is durable requested notification without new authority or duplicate
watchers. The cost is coordinated actor/store recovery and finite retained
state; until that work is checked, restart parity must not be advertised.
