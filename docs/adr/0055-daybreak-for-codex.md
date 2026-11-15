# ADR 0055: Daybreak for Codex: not possible through the adapter today

Status: proposed, blocked, 2026-11-14 (milestone 11, track A2, the
owner's request: a Daybreak switch for Codex in the model detail
settings, beside a Fast switch for every agent that offers fast). Nothing
is built. This records why, and what the node does once it becomes
possible.

## Context

In the Codex desktop app, Daybreak is a switch in the model picker. It is
shown when a listed model's `availableAccessPrograms.cyber` contains
`daybreakBlue` or `daybreakRed` (Codex app-server `model/list`). When it
is on, every `turn/start` carries `cyberAccessProgram` (`standard`,
`daybreakBlue` or `daybreakRed`). Red is lowered to Blue for a model that
is not a cyber model.

Vessel drives Codex only through its ACP adapter, `codex-acp` (ADR 0003).
What was read, without a prompt to a real agent:

- **codex-acp 2.6.0**:
  - It advertises the config options `mode`, `collaboration_mode`,
    `model`, `reasoning_effort` and `fast-mode`. None of them is an access
    programme.
  - Its `turn/start` is built from a fixed list: thread, input, approval
    policy and reviewer, sandbox or permissions, summary, effort, model,
    service tier, client message id. There is no `cyberAccessProgram`.
  - It reads a prompt's `_meta` only for agent file-change reports.
  - It never sends `daybreakEnabled` and never calls
    `thread/metadata/update`.
- **The Codex CLI it bundles (0.166.0), app-server schema** (`codex
  app-server generate-json-schema --experimental`):
  - `turn/start.cyberAccessProgram` is an experimental field ("Request an
    authorized cyber program for this turn. Omission preserves automatic
    behavior. This does not grant access.").
  - `thread/start.daybreakEnabled` and
    `thread/metadata/update.daybreakEnabled` store a client's choice for a
    persistent thread. By their own description they do "not select a
    turn's `cyberAccessProgram` or grant access".
  - `model/list` has `availableAccessPrograms.cyber`.
- **Configuration and environment**:
  - Codex's configuration has no key for an access programme. The
    config-read schema names none; the binary's strings show
    `cyber_access_program` only in stored turn context and rollouts, and
    `daybreak_enabled` only as a thread column.
  - So neither `-c key=value` nor codex-acp's `CODEX_CONFIG` (which
    becomes `thread/start`'s `config`) can set it.
  - No environment variable for it was found in codex-acp or in the CLI's
    help.
- **ACP** has no field for it. The adapter would have to offer it as a
  config option or read it from `_meta`, and it does neither.

**Not confirmed without a real turn**:

- what "omission preserves automatic behavior" picks for an account that
  has Daybreak access;
- whether a thread's `daybreakEnabled`, saved by another client such as
  the Codex app, changes the turns codex-acp starts. The schema says it
  does not.

## Options considered

- **Drive Codex's app-server directly** for Codex sessions, to send
  `cyberAccessProgram`. Refused: Vessel speaks ACP to every agent (ADR
  0003), and a second client stack for one harness is what ADR 0003
  exists to avoid.
- **Set it through configuration at launch** (`-c`, `CODEX_CONFIG`). Not
  possible: Codex has no such configuration key.
- **Show a switch that saves a preference only.** Refused: a control that
  changes nothing a turn does misleads, and the coordinator's rule is
  that nothing shown may do nothing.
- **Wait for the adapter to carry it.** Taken.

## Decision

- **Not offered now.** The node advertises nothing for Daybreak, and the
  app draws nothing for it. Fast needs nothing new: the agents already
  advertise it as an option (`fast` on|off for Claude Code, `fast-mode`
  off|on for Codex).
- **When codex-acp carries it**, the node's part is fixed in advance:
  - **How it is learned**: from the agent alone, never from a model's name.
    Either codex-acp advertises a config option that it forwards to
    `turn/start.cyberAccessProgram` (which it would offer only for models
    whose `availableAccessPrograms.cyber` lists a Daybreak programme), or
    an ACP `_meta` field that codex-acp maps to it and advertises in
    `initialize`.
  - **How it is shown**: an adapter option reaches clients as every
    option does (`SessionOption`, protocol 9), under the adapter's id and
    values. For a `_meta` field, the node would add one option of the
    existing kind: id `daybreak`, a select with the values `off` and
    `blue` (and `red` when the model list offers Red). An app built
    before it shows one more select; the lead's app draws it as a switch.
  - **How it is applied**: per session, carried on every turn, as Codex's
    own app does. It is changed with `session.set_option` (operate). A
    read-only device sees the option and its value and cannot change it.
  - **If the session's adapter cannot carry it**: the option is not
    advertised.

## Why

The decision is to ship nothing. A Daybreak switch in Vessel would have
to reach `turn/start`, and the only road from Vessel to Codex is
codex-acp, which has no lane for it today.

## Consequences

- The owner's Fast switch ships without Daybreak; Daybreak waits on an
  upstream change to codex-acp, or a later adapter version, which the
  node then follows as above without a protocol change of its own (an
  adapter option) or with one option (a `_meta` field).
- When codex-acp is upgraded, this is checked again by the same reading:
  its advertised options, its `turn/start` parameters, its `_meta`.
