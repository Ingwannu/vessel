# ADR 0053: Importing from an agent's own history, as the agent lists it

Status: accepted, 2026-11-13 (milestone 11, track A2,
`docs/plans/backend-m11.md`, "Track A2: contract, 4." and its review).
The wire is protocol 17's `agent.history` (`crates/vessel-proto/src/history.rs`):
track A2 wrote it at 14 beside forking, and on milestone 12's node line
15 (fleet controls) and 16 (`fleet.published`) came first, so it was
numbered 17 when merged there.

## Context

A person has conversations with their agents outside Vessel: in the
agent's own terminal, in another client. Paseo, agent-orchestrator and
GOAT each offer to bring them in:

- **Paseo** asks the agent over ACP: a probe process, `initialize`,
  `session/list` paged by cursor, and for each listed conversation a
  `session/load` to read its first and last prompts as previews (up to
  60 s a listing); an import resumes the conversation, whose timeline is
  read in.
- **agent-orchestrator** and **GOAT** read each harness's files with a
  reader of their own (thirteen and three of them), within a time window,
  and copy the transcript into their store.

Over ACP, both installed adapters list (`sessionCapabilities.list`):
Claude's in pages of 1,000 behind an offset cursor, matching the folder
exactly; Codex's filtering the folder after paging, so a page can be
empty with a next one.

Vessel already imports, without a list: `session.create` with
`Structured { resume }` starts the agent with `session/load`, and a new
session publishes the load's replay as its log.

## Options considered

- **Readers of each harness's files** (agent-orchestrator, GOAT): break
  with every change of a harness's format, read another program's files,
  and cannot see an added agent's. Refused.
- **Paseo's listing with previews**: a load of every listed conversation
  (seconds each) to show its first and last prompts. Not taken: the
  agent's title and time are enough to choose, and the import shows the
  whole conversation.
- **A list of the node's own** (an index of conversations): a second
  truth beside the agent's. Refused.

## Decision

- **`agent.history { harness, profile?, cwd?, cursor? }`** answers a page
  of the agent's own conversations: the node starts the agent as a probe
  is started (the profile's account and environment, the probe folder, a
  routed profile's model token), `initialize`, and `session/list`; no
  session is created or loaded. Only an agent that advertises
  `sessionCapabilities.list` and `loadSession` lists (`unsupported`
  otherwise): a conversation it cannot load cannot be imported.
- **Pages**: the agent's page whole up to 1,000 entries behind a cursor
  the node mints (the agent's cursor and how many of its page were
  given), so a larger page comes in parts and nothing is skipped; pages
  that hold nothing are asked past on the same connection (up to 10), so
  an empty page with `next` is rare and not the end.
- **Entries**: `harness_session`, an absolute `cwd`, the title cut as a
  prompt's title is, `updated_at` typed, and `in_vessel`, the newest
  session on this node on that conversation (live or kept), matched on
  harness and conversation.
- **Import is `session.create`** with `resume`, the entry's folder, the
  profile it was listed under and its title; nothing new on the wire.
- **One agent per conversation**: a start that resumes a conversation
  (`session.create`, `session.resume`) is refused `conflict` while
  another live session is on it under the same profile or starting on
  it, so a double click or a stale `in_vessel` cannot put two agents on
  one transcript. Per profile, because the transcript is in the
  profile's home: another profile's copy is another file, and `session
  move` resumes the copy before it stops the source. A move between
  profiles links the files instead, so an import under the profile a
  session moved away from is not caught (its entry names the session).
- **Who**: `operate`, as the import; forwardable as `session.create` is;
  refused to agents. Two listings at once on a node, `unavailable`
  beyond.

## Why

The agent's own list is the only one that is right for every agent that
speaks ACP, including agents added on the node, and it costs one agent
start per page. Making the list a page of the agent's (not a fixed size
of Vessel's) is what keeps every conversation reachable whatever the
adapter's page size; the claim is what keeps an import that became a
click from corrupting a transcript.

## Consequences

- Agents that do not list over ACP offer nothing to import (the older
  path, `vessel run --structured --resume ID`, still works for an id a
  person knows).
- An import's replay must fit the agent's handshake (60 s): measured at
  2.3 s for 10,000 replayed messages on a debug build; a longer one ends
  the session `failed` with what was replayed.
- Imported history has no fork points (ADR 0052) until its first turn in
  Vessel: the replay carries no turn's end.
- Each page costs an agent start (seconds); nothing is cached, so a
  conversation just had shows.
- What is listed is the adapter's choice: Codex leaves `codex exec` and
  sub-agent threads out, Claude its sub-agent transcripts; conversations
  with nothing in them are listed (Paseo drops them by loading each).
- Refusing agents is a person's view, not a boundary: an agent can read
  its harness's files with its own tools.
- Not offered: previews; a search over the agent's list (a client's
  filter over pages it loaded); copying a transcript into Vessel without
  the agent.
