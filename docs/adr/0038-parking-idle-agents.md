# ADR 0038: Parking an idle agent and waking it

Status: accepted, 2026-11-13 (milestone 11, track A2,
`docs/plans/backend-m11.md`, "Track A2: contract, 2." and its review).
The wire is protocol 12's `session.park`, `session.wake`,
`SessionInfo.parked` and the `parked` and `woken` records
(`crates/vessel-proto/src/parking.rs`); the behaviour is the session
runtime's (`crates/vessel-session/src/parking.rs`).

## Context

A structured session's agent is a process that holds hundreds of MiB
whether or not it works. All three references stop an idle one and start
it again when it is needed: agent-orchestrator after 10 minutes (off by
default), GOAT after 30, Paseo after an hour (on by default). Vessel's
session actor ends when its agent exits, and a move to another profile
(ADR 0011) already knows how to stop one agent and start another on the
same conversation with ACP `session/load`.

The questions:

1. Which sessions, after how long, and what prevents it?
2. What does a client see, and does an older one break?
3. What wakes it, and what happens when waking fails?

## Options considered

- **End the session and let `session.resume` start it again** (ADR
  0026's start again). Refused: the session would leave the running lane,
  its followers would see it end, and its queue would be held.
- **A new `SessionStatus::Parked`.** The contract's first form. Refused
  by its review: older peers would receive it through replies no
  rewriting covers (a mark's answer, an export's manifest, an import) and
  through status records, and fail to decode them; every "is it running"
  check, the page's composer among them, would have to learn it.
- **A sweep over every session**, as agent-orchestrator and Paseo run.
  Refused: a loop that wakes for nothing; each actor can keep its own
  deadline.
- **Asking the agent whether it has background work.**
  agent-orchestrator does, through an extension of its own
  (`_ao/session/can_hibernate`) that the stock adapters lack; ACP has no
  such call. Not available.

## Decision

- A structured session whose agent can resume (its handshake said
  `loadSession`) and has a harness session id is parked after the node's
  `park_after` (an hour by default, `vesseld --park-after`, 0 never)
  without activity, when no turn runs, nothing waits on an answer and its
  queue is empty or held. Whoever follows it does not matter (amended
  2026-11-14, below). `session.park` parks it now.
- Its process is stopped as a move stops the old agent, without ACP
  `session/close` (which may delete what resume needs). The session stays
  live and `running`; `SessionInfo.parked` says since when and why; the log
  has `parked { reason }`.
- Whatever needs the agent wakes it: a prompt of any delivery, a queue
  change that starts one, an option change, a move, `session.wake`. The
  agent is started with the session's launch, its harness session id and
  its current choices, with `session/load`; its replay is dropped; the log
  has `woken` once it is ready. Reads (options, the conversation, the log,
  a search) and a cancel do not wake it.
- A wake that fails leaves the session parked (`reason: wake_failed`) and
  fails the call that woke it (`spawn_failed`). After it, a prompt
  delivered `queue` (pull request feedback, a schedule, a starter's
  notice) waits in the queue, held, instead of trying again; a person's
  prompt, `session.wake` or the queue's `resume` tries again.
- A kill or the node stopping ends a parked session at once. A node
  draining for an update does not wait for parked sessions. A restart
  interrupts them as any live session.

## Why

The route a handle sends through says whether the agent is live, being
replaced or parked, so orders reach a parked session's actor (which
wakes it) while what needs the agent itself is answered from what was
kept: nothing else in the node had to learn the state. Keeping `running`
makes the feature invisible to older peers, who see a prompt take a
moment longer. One timer per actor, reset only when it fires, costs
nothing per event.

## The setting (2b, 2026-11-13)

Paseo turns idle cleanup on and off from a card in a host's settings,
kept by the daemon and applied at once; a flag alone left parking out of
the app. `node.settings` (read) answers the settings in force and
`node.set_settings` (admin) changes them: `park_after` is `never` or
`idle { minutes }` (1 to 10,080; a longer time would be a new `kind`,
since an older client refuses a number out of its range). Neither is
forwardable: they are the node's own administration, as `node.stop` is.
What a person changed is kept in `settings.json` in the data directory
(only what was changed, owner-only, replaced atomically) and wins over
`--park-after` at start, field by field, so a service started with the
flag does not undo a change at its next restart; the node logs when it
does. A saved choice this build does not know leaves the flag's value,
and fields it does not know are ignored at every level, so a file from a
later build still parses after a downgrade; such a file (anything this
build did not read from it, found by comparing the two at start) is
copied to `settings.json.later-<UTC time>` before the first change this
build writes, so the later build's settings can be recovered from the
copy (nothing is merged back on its own). A file that cannot be read or
parsed leaves the flag's value too, and is first moved aside as
`settings.json.unreadable-<UTC time>` (owner-only, never over an earlier
copy), so the next change cannot replace it (2026-11-14; before, it did,
and every saved setting was lost). If the move fails, the node refuses
any change that would replace the file (`conflict`, with the reason)
rather than destroy it. A change is written first and
applied after, under one lock; a change that changes nothing writes
nothing. Applied live: the session runtime holds the value in a watch,
and every structured session's actor re-arms its idle timer from its
last activity when it changes (one idle for longer than a new, shorter
time is parked at once; off disarms the timers, and parked sessions stay
parked until something wakes them). Unlike Paseo, the node does not tell
connected clients that the setting changed; a client reads it when it
shows it.

## Consequences

- An agent's background work (a shell it left running, a dev server) is
  stopped with it; Vessel cannot ask. That is why the default is an hour
  and why a node can turn it off. A port the agent's processes listen on
  would be a real sign of such work; the node's port scan is not wired to
  sessions' actors yet.
- A parked session keeps its actor, its record ring and its last options
  and commands; it counts as live toward an agent's eight sessions
  (ADR 0013), and `session.resume` on it is `conflict`.
- The first prompt after a park waits for the agent's handshake and
  `session/load` (bounded by the handshake timeout).
- A parent session is parked while sessions it started work, unlike
  Paseo, which keeps a family awake; a starter's notice wakes it.

## Amendment, 2026-11-14: followers do not keep an agent awake

- Was: nobody following the session live (a page that had it open,
  `vessel attach`, an agent's `wait_turn`: agent-orchestrator's view
  lease) was a condition of the idle park.
- Found staging the node line: sessions an app had shown stayed awake for
  minutes after the app was gone. Releasing depended on every client's
  subscriptions ending, and the node's token `/rpc` connections had no
  keepalive, so a client gone without closing (asleep, its network
  dropped) held its subscriptions, and the sessions they followed, for
  good.
- Paseo, the owner's reference ("the setup exactly as Paseo's"), collects
  an idle runtime whether or not a client watches it: its eligibility
  (`AgentManager.runtimeHasNoPendingWork` and the idle-runtime-cleanup
  module) is the foreground turn, permissions and their answers, queued
  deliveries, lifecycle operations and running children, plus each
  provider's veto; client subscriptions are not among them.
- Decision: the same. A follower loses nothing: the session stays live,
  its subscription goes on through `parked` and `woken`, and its next
  prompt wakes the agent. What still holds the agent: a running turn, a
  permission or question waiting, a moving queue, a move, an agent that
  cannot resume, and activity within `park_after`.
- And the node pings `/rpc` connections as it pings `/secure` ones (10 s,
  dropped after 30 s of silence, `vessel_relay::Liveness`), so a vanished
  client's subscriptions end within the silence limit.
- `park_after` is whole minutes on the wire and in the flag
  (`ParkAfter`), one at least: a test of the idle park below a minute is
  the session runtime's, not the node's.

## Amendment: settings-file diagnostics (2026-11-15)

Purpose: a mistaken inline credential in an invalid settings block must not
become a startup log entry. Serde's invalid-type messages can quote the value;
printing a fixed outer message alone still exposes it through Debug or sources.
The file decoder therefore retains the serde cause privately behind a safe
diagnostic. Display, Debug and the public source chain expose only the file
path, error category and a bounded field-path shape. Map keys and enum names
are placeholders, since they can themselves be credentials. List indices remain.
The cost is less precise repair advice; the original file is still set aside
under the existing recovery rule. No parsing acceptance or storage policy changes.

Reuse decision: extend the store's single startup reader and use the existing
workspace dependency serde_path_to_error 0.1.20 (MIT OR Apache-2.0, 106,364 bytes
of unpacked source in the inspected copy; linked-size delta not measured). It
already belongs to the RPC graph and adds no package/version to the lock.
Serde JSON alone has no structured field path; reparsing or writing another
visitor would duplicate validation. RPC's decoder deliberately discards its raw
cause; the file boundary instead keeps that cause private, without a public
raw-cause accessor. Explicit strict reload retains its separate acceptance rule.
A marker test covers error formatting and the complete public source chain;
a captured startup recovery log covers both move success and move failure.
