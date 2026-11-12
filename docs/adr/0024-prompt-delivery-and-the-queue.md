# ADR 0024: How a prompt is delivered while a turn runs, and the queue

Status: accepted, 2026-11-11 (milestone 11, track A,
`docs/plans/backend-m11.md`, "Track A: contract" and its review; the
wire is protocol 10, `crates/vessel-proto/src/conversation/queue.rs`).

## Context

A person writes the next message while the agent still works: to wait
its turn, to correct the turn that runs, or to replace it. All three
references offer the three (agent-orchestrator's queue and steer routes,
GOAT's `queue_after_active`, `steer_active` and `restart_active`, Paseo's
`interrupt`, `steer` and its composer's queue). Before protocol 10 a
Vessel prompt sent during a turn was refused (`conflict`).

What the agents do with ACP's `session/prompt` during a turn differs, read
from their code on 2026-11-11: Claude's and Codex's adapters queue it,
Gemini cancels the running turn, OpenCode refuses it. Steering exists in
ACP only as an extension: an agent whose `initialize` answer carries
`_meta.steering.supported: true` takes `_session/steering { sessionId,
prompt, _meta.steering.idleBehavior }` and answers `injected`,
`promptRequired` (no turn runs) or `startedNewTurn`. Claude's adapter
0.100, `codex-acp` and OpenCode advertise it; Gemini does not.

Questions:

1. Who keeps the queue: a client, the node, or the agent?
2. What is "steer" for an agent that cannot take input mid-turn?
3. What happens to waiting prompts when a turn is cancelled, fails, or
   the session ends?
4. How are the races between a turn ending, the queue moving, a cancel
   and a steer settled?

## Options

**The queue**

1. In the client (Paseo): lost with the browser, invisible to another
   device, and its drain races each client's view of the turn.
2. In the agent: the adapters disagree on what a prompt mid-turn does,
   and a profile move replaces the agent process (and its queue).
3. In the node, as session state every client sees, kept by the store
   (agent-orchestrator and GOAT).

**Steering for an agent without the extension**

1. Interrupt and resend under the name "steer" (Paseo falls back to this
   silently).
2. Not offered; interrupt-and-send offered as what it is.

**Waiting prompts after a stop**

1. Drop them at a cancel (agent-orchestrator).
2. Hold them at a cancel, a failure and a restart, and let a person
   resume (GOAT).
3. Keep sending them.

## Decision

- **The node keeps the queue**, in the session's actor (`vessel-session`),
  logged whole as `queue_changed` and projected by the store, so it
  survives the page and the node. The actor is the one place every
  delivery passes, one at a time, with the turn state it publishes:
  nothing else can start a turn, so "is a turn running" and "start the
  next entry" never disagree.
- **Four deliveries** (`session.prompt`): `start` (refused during a turn,
  what `session.send` always did), `queue`, `steer` and `interrupt`.
  `steer` uses only the ACP extension and is refused (`unsupported`) for
  an agent without it, before anything is sent; the client knows from
  `AgentAbilities.steer`. `interrupt` cancels, waits (10 s) for the turn
  to end, then starts the prompt.
- **One entry per turn**, as GOAT does; agent-orchestrator merges every
  waiting entry into one turn, which loses where each prompt ends.
- **Held, never dropped**: a person's cancel, a failed turn, and a
  session ending with entries hold the queue (`QueueHold`), and a
  `resume` lets it go; an empty queue is never held, so a message typed
  after a stop starts at once. A cancel names the turn it means, so a
  cancel that lands after the queue started the next turn only holds the
  queue.
- **Steering races**: `promptRequired` (the turn ended first) starts the
  prompt if nothing runs, or puts it at the head of the queue if the
  queue already started a turn; `startedNewTurn` despite
  `promptRequired` is an error (its turn would never be logged).
  `steered` is logged when the agent says `injected`.
- **Bounded**: 32 entries and 128 KiB of content; none of the references
  bounds its queue.

## Why

The node is where every client and agent tool already meets, and the
only place a queue outlives a browser and agrees across devices. Holding
rather than dropping keeps the rule a person relies on: nothing waiting
is sent after they said stop, and nothing they wrote is lost. Steering
only where the agent takes it keeps milestone 11's rule (truth over
neatness): an interrupt under another name restarts the agent's work
cold.

## Consequences

- Every prompt goes through the session's actor; while it waits for the
  agent's answer the actor keeps publishing the agent's output, or a
  full output queue would stall the agent that has to answer.
- The log carries the whole queue at each change (at most 128 KiB per
  record); retention counts it like any record.
- Any turn that ends `cancelled` holds what waits, not only a person's
  cancel: a forced move's cancel, or an agent cancelling on its own,
  also stops the queue, since nobody asked for the next entry then.
  An interrupt's cancel is the exception: its own prompt starts next.
- The actor counts the turns it started and the ends it published
  rather than keeping a flag: a turn's end can be in the log before the
  agent's answer to its prompt reaches the actor.
- An entry is checked again when it is sent; one an agent no longer
  takes (after a move or a start again) holds the queue with a log line
  instead of failing a turn.
- No idempotency key: a call that may or may not have run is reported so
  to its caller, who sees the queue and decides.
