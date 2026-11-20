# ADR 0116: terminal list long poll and resize claim

Status: accepted by the coordinator on 2026-11-20 (long poll instead of a `terminal.watch` stream; claim and update as
drafted). Implementer T2, packet N1. Protocol 37 (`SHARED_TERMINALS_PROTOCOL`), assigned at its node-line merge on
2026-11-20. TRACKER 481 (list sync), 483 (resize and focus claim); catalogue F-G44, F-G46, F-G49,
F-G51. Paseo reference 0.15.4.

## Context

Paseo shares a workspace's terminals between clients in two ways.

- **The list is pushed.** A client sends `subscribe_terminals_request {cwd}` and gets `terminals_changed` at once and
  again after every create, exit, rename or close, one refresh in flight with a pending flag
  (`packages/server/src/terminal/terminal-session-controller.ts` 124, 203, 370-400).
- **One client owns a terminal's size** (`terminal/terminal-size-ownership.ts`). A resize carries `intent: claim |
  update`; none is a claim (`COMPAT(terminalSizeOwnership)`). A claim makes the sender the owner and applies; an update
  applies only from the owner and is otherwise dropped (`false`, no error). A pane claims on focus and when it
  attaches (`terminal-pane-focus-claim.ts`, `terminal-stream-controller.ts` 74-79); its later fits are updates.

Vessel before this (ADR 0031, protocol 11):

- `terminal.list` answered once. The native app's terminal tabs (TRACKER 481) asked on every 2 s fleet read and after
  their own create, close or end, so a shell another client opened took up to 2 s to appear.
- `TerminalInput::Resize {size}` applied whatever arrived last, from any connection. The app sends only from its
  focused pane, so its own panes did not fight, but the app and the page (or two apps) on one shell did.

## Decision

### The list as a long poll

- `ListTerminals` gains `changed_from: Option<u64>`; `TerminalList` gains `revision: u64` (0 from an older node).
- With `changed_from`, `terminal.list` waits at most 30 s, inside the caller's wait shares, until the revision differs,
  then answers the list; a forwarded call answers at once. This is `terminal.get`'s and `port.list`'s shape.
- The revision is one `watch` counter per node (`terminals/registry/listed.rs`). The registry moves it when a terminal
  is kept or forgotten; one small task per terminal moves it when the terminal's status, title, service state or end
  changes. A resize or output never moves it, so a typing shell wakes no waiter.
- The revision is read before the list, so a change between the two answers the next poll at once.
- Clients: ask again at once with the answer's revision when it moved (no timers of their own beyond the poll); one
  refresh per burst, as Paseo coalesces. Below protocol 37 they keep their previous behaviour (the app's 2 s read).

### The size claim

- `TerminalInput::Resize` gains `intent: Option<ResizeIntent>` (`claim | update`); absent is a claim.
- The node keeps the owner of each terminal in its registry entry, and of each live terminal-mode session in
  `NodeService::session_sizes`: the claiming connection's identity (`connection_life::Owner`, compared by pointer and
  kept alive by the slot, so a later connection never matches a gone one).
- A claim applies and takes the slot. An update applies only from the owner; otherwise it is dropped and answered
  `Ack`. Nobody owning it yet, an update is dropped too (Paseo's rule).
- Clients send a claim on focus and on attach (with the preferred size) and updates for later fits while focused.

### Version rule

`Call::since` makes a call that sets `changed_from` or names an `intent` need protocol 37. An older node would answer
the poll at once (a client asking again on each answer would spin) and apply another client's update.

## Alternatives considered

- **A `terminal.watch` subscription** (Paseo's push shape). The node's subscriptions carry a log's `EventRecord`s keyed
  by `EventSeq`, with replay (`session.subscribe`, `terminal.subscribe`); a list snapshot is not a record. It would
  need a new frame kind, its own unsubscribe and its own lifetime rules. The long poll gives the same latency, one round
  trip after the change, with none of that, and is how the node already serves `terminal.get`, `terminal.read` and
  `port.list`. Rejected; a stated difference from Paseo (push vs long poll).
- **Keep the 2 s read.** Another client's shell takes up to 2 s to appear (the gap G44 and G49 name).
- **A revision per place.** Fewer wakeups, but a terminal can change place membership only by being created or
  forgotten, and the node keeps at most 96, so one counter is simpler; a waiter woken by another place's change reads
  the same list and asks again.
- **Owner by client name.** Two windows of one app would share an owner and fight again; Paseo keys by socket.
- **Refuse a non-owner's update with an error.** Every fit would need a refusal path; Paseo drops it silently and the
  next claim settles the size.

## Consequences

- Protocol 37 (assigned at the node-line merge), `PROTOCOL_VERSION` 37; both new request fields are optional, the answer field defaults to 0.
  The minimum protocol stays 2; the store schema is unchanged.
- The connection life gets its first consumer: the size owner reads `Connection::life().owner()`; the close observer
  (`closed()`) keeps its dead-code expectation until workspace D2 Preview or W14.
- Cost: a call forwarded through a home node carries that home's connection, so the clients behind one home share its
  ownership (they can still fight among themselves, not with clients connected directly).
- Cost: one task per kept terminal (at most 96) watches its info, ending with the terminal or the node.
- Clients: the native app's terminal tabs move from the 2 s read to the long poll and send claims and updates at 37+
  (packet N1's consumer change, on the app line); the page does the same in P2.
- Stated differences from Paseo: the list is a long poll, not a push; it is asked per `TerminalPlace` (a session or a
  project), not per workspace folder.
