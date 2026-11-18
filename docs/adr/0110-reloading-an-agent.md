# ADR 0110: Reloading a session's agent, as Paseo's "Reload agent"

Status: accepted (number reserved by the coordinator on 2026-11-18 for TRACKER 803; rulings A, B and C the
same day); impl U. The protocol number is drafted as 31 (`RELOAD_PROTOCOL`) and assigned at the node-line merge.
Supersedes ADR 0063's "Reload" section (its refusals of a session that is not live or is parked, and its
dropped replay); ADR 0063's other sections stand.

## Context

Paseo's tab menu has "Reload agent" (`workspace-tab-menu.ts`, icon `rotate-cw`, tooltip "Reload agent to
update skills, MCPs or login status."). Its handler (`workspace-screen.tsx` `handleReloadAgent`) shows
"Reloading agent..." until the answer, then "Reloaded agent", or the error's message, else "Failed to reload
agent". It sends `refresh_agent_request`; the daemon (`session.ts` `handleRefreshAgentRequest`):

- unarchives the agent (`unarchiveAgentState`);
- for a loaded agent, interrupts a running turn and calls `reloadAgentSession(..., { rehydrateFromDisk: true })`:
  a running turn is cancelled first, the old process is closed, a new one resumes the persisted session with
  a fresh launch, and the in-memory timeline is wiped so `hydrateTimelineFromProvider` streams the provider's
  history again under a new epoch;
- for a closed agent, loads it from storage (`ensureAgentLoaded`) and hydrates it the same way;
- answers `agent_refreshed` with `timelineSize`, or `rpc_error` `agent_refresh_failed`.

Vessel already restarts a structured session's agent in place: a move (ADR 0011), a wake (ADR 0038) and a
start again (`session.resume`, ADR 0026) all stop the process and start another with `session/load`, and
drop the agent's replay because the log has it. ADR 0063 had specified a reload on that machinery that
refused a session that was not live or was parked, and dropped the replay.

## Decision

[Decision Log]
- Purpose: a person restarts a session's agent so it picks up skills, MCP servers and its login, and sees
  what the agent's own history has beyond the log (turns made outside Vessel), as in Paseo.
- Existing assets reused: the actor's stop and `session/load` start (`moving.rs` `stop_agent`, `start_agent`,
  `until_turn_over` with the forced move's 10 s grace), parking's state and its failed-wake path, the start
  again (`again.rs`, the node's `service/again.rs`), the launcher's start-again launch (`Launcher::again`),
  and the move's credential and token minting (`Agents::begin_move`, `Models::begin_move`).
- Alternatives considered:
  - ADR 0063's reload: refuse ended, archived and parked sessions, and drop the replay. Rejected by the
    coordinator (ruling A): Paseo unarchives and loads a closed agent.
  - A forced `session.move_profile` to the same profile. Rejected as in ADR 0063: a failed move starts the old
    process again and publishes `profile_changed`, and a forced move stops a turn that did not end.
  - For the history (ruling C), drop the replay (R1): turns made outside Vessel never show. Or append a reset
    marker and the whole replay again (R4): an older client sees the conversation twice, and the log grows
    with every reload. Or append only what the history has beyond the log (R2, chosen).
- Choice:
  - One call, `session.reload { session }`: `operate`, forwardable, unary, since `RELOAD_PROTOCOL`. Its reply is
    the `SessionInfo`; its `next_seq` stands for Paseo's `timelineSize`.
  - Refusals: an agent principal is `forbidden` (a person reloads; Paseo has no MCP tool for it). A terminal
    session, one with no harness session yet, and one whose agent cannot `session/load` are `unsupported`.
    An unknown session is `not_found`. A session starting or moving is `conflict`.
  - Order (Paseo's handler):
    1. Unarchive. This is kept even if the reload then fails, as in Paseo.
    2. A live session: input is held. A running turn is cancelled and waited on for the forced move's 10 s
       grace (ruling B). If it does not end, the call is `conflict`, nothing is stopped and the turn goes on.
       The driver settles pending permissions and questions as cancelled on the cancel and on the stop. The
       process is stopped (`unavailable` if it cannot be signalled, nothing new started) and a new one
       started with `session/load` and a fresh launch: the current profile, the fleet's MCP servers as they
       are now, a new `vessel` credential and a routed profile's new token, the session's choices.
    3. A parked session: started with that launch.
    4. An ended or archived session: started again as `session.resume` starts it.
  - The record: `reloaded { harness_session, appended }` once the agent is ready again. An older peer reads it
    as `vessel: the agent was reloaded` (with the count when records were added), at the same sequence.
  - The history (ruling C, R2):
    - Instead of being dropped, the replay is gathered, bounded by 60 s and 20,000 events.
    - Its user messages are lined up with the log's user prompts, compared exactly as text.
    - Only when the log's prompts are exactly the first ones are the records from the next user message on
      appended after `reloaded`: whole messages, at most 2,000 records.
    - Otherwise nothing is appended, and one log line says why: a different history, the gather's bounds, a
      log whose first records were pruned, or a message longer than the cap. When records remain past the
      cap, a line says to reload again.
  - Failure (typed `ReloadFailed`, `spawn_failed`): a new agent that does not become ready, or becomes ready
    on another conversation, ends a live session `failed` with its log kept; `session.resume` or another
    reload is the retry. A parked session stays parked with the reason, as after a failed wake.
- Why this over the others: it is Paseo's behaviour on Vessel's terms. The log is every client's cursor and is
  never rewritten, so rehydration can only add. Adding only after an exact prefix match cannot duplicate a
  turn the log has, and a second reload after an addition adds nothing.
- Pros, cons and effects:
  - A reload picks up configuration a move or a wake would not re-resolve (MCP servers, credentials).
  - Turns continued in the harness's own interface appear in Vessel after a reload.
  - A reload waits for the whole replay, as a wake does, and holds up to the gather's bound in memory until
    the agent is ready.
  - An adapter whose replay alters user text (wrappers around slash commands, injected context) never lines
    up: such a reload adds nothing and says so. That is safe, but its outside turns stay out of the log.
  - Each reload reads the store's records before the current run to fold their prompts; a session started in
    this run reads none.

## How the log's prompts are counted (`vessel-session` `reload::prompts`)

- In the log, one prompt is:
  - a turn's prompt (`turn_started`);
  - a steer (`steered`);
  - outside a turn, a run of user message chunks: an import's records, or an earlier reload's.
- A user chunk inside a turn is the agent echoing its prompt and is not counted again. Only agent events
  take part, so a node record between two chunks does not split a message.
- The node folds the stored records before the run (`Sessions::prompts_before`); the publisher folds this
  run's records as it commits them. They meet at the run's start, where no turn runs, so a record that
  arrives while the store is read is neither missed nor counted twice.
- Text is the blocks' text joined with nothing between them; attachments and links have none.
- The fold gives up past 4 MiB of text.

## Costs

- `RELOAD_PROTOCOL` is a new number (drafted 31). The page's copy (`web/src/data/reload.ts`) must follow
  the number assigned at merge.
- The page offers the row on every structured session whose agent can load a conversation, on machines
  speaking this protocol, as Paseo offers it on every agent tab; the node's refusal is the failure toast. The
  row sits in the session's actions menu, since the page has no tab menu. The native row is the visual lane's,
  after this node op lands.
- No store schema change. A parked state is still not persisted.

## Tests

- `vessel-session` `reload/tests.rs`:
  - the fold (turns, steers, outside runs, echoes, joins, the byte bound);
  - nothing new;
  - turns made outside Vessel added once, with no duplicate on a second reload;
  - a diverged history adding nothing;
  - the unknown log, the cap in whole messages, and the item and time bounds on a paused clock.
- `vessel-node` e2e `reload.rs`, over the wire with `fake-acp` (`FAKE_ACP_REPLAY_FIRST`, `FAKE_ACP_REPLAY_USER`,
  the `deaf` prompt):
  - the three history cases;
  - an archived session unarchived and started again;
  - a turn that does not end kept with `conflict`;
  - an agent `forbidden`, a terminal `unsupported`, an unknown session `not_found`.
- `vessel-proto`:
  - `reload/tests.rs`: the request's wire form, `Call::since`, the older peer's line;
  - the event admission table;
  - the method version, scope and forwarding pins.
- Page: `data/reload.test.ts`.
