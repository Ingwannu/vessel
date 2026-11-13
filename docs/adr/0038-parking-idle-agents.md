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
  without activity, when no turn runs, nothing waits on an answer, its
  queue is empty or held, and nobody follows it live (a page that has it
  open, `vessel attach`, an agent's `wait_turn`: agent-orchestrator's view
  lease). `session.park` parks it now, whoever follows it.
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
