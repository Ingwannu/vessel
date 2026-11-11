# ADR 0022: What a session runs with, and what it is called

Status: accepted, 2026-11-11 (milestone 10, track C,
`docs/plans/backend-m10.md`; the contract is protocol 9,
`crates/vessel-proto/src/options.rs` and `title.rs`).

## Context

The owner looked at the web UI and found nothing to choose a model, an
effort or the permissions with, and a board of cards named after
folders. All three choices are the agent's own: ACP reports them as
session config options in the handshake (`session/new`'s
`configOptions`, each with a category: `mode`, `model`, `model_config`,
`thought_level`, or none), and an older agent reports only `modes`
(`SessionModeState`), changed with `session/set_mode`. What each agent
offers differs and changes with every release. On this machine on
2026-11-11, `claude-agent-acp` 0.100.0 offered a `mode`, a `model`, an
`effort` (`thought_level`), two on/off selects (`model_config`) and an
uncategorized one; `opencode acp` 1.18 a `model` and a `mode`, no effort;
Codex had no ACP adapter installed.

Protocol 8 carried one model name at creation (`SessionMode::Structured.
model`), matched against the agent's model option; nothing else of the
options left `vessel-agent`, and the two updates that say they changed
were dropped.

Questions:

1. Does Vessel pass the agent's options on, or define its own fields?
2. How does a new-session form know what an agent offers before a
   session exists?
3. When are choices applied, and what happens to one the agent does not
   offer?
4. How do clients learn the options of a running session and their
   changes?
5. Where does a session's title come from, and which value is the truth?
6. What happens to the choices when a session continues elsewhere (a
   profile move, an import on another node)?

## Options

**What crosses the wire**

1. Vessel's own fields (`model`, `effort`, `permissions`) with values
   mapped per harness.
2. ACP's option objects forwarded as raw JSON.
3. Vessel's own shapes of ACP's options: ids and names as the agent gave
   them, a closed category, a closed kind.

**Before a session exists**

1. A list per harness written into Vessel.
2. Reading each harness's configuration files.
3. Starting the agent as a session would, reading the handshake, and
   stopping it (a probe), cached.

**Titles**

1. A free `String`, checked where the node receives it.
2. A validated type, built only by parsing (a typed title) or from a
   prompt (cut, not refused).

## Decision

- **Options are passed on (3).** `SessionOption { id, name,
  description?, category, kind }`, `kind` a `select { current, values }`
  (values flattened in order, each with its group's name) or a `toggle {
  on }`, and `unknown` for a later kind, which a client skips (it keeps
  the rest of the list decodable). `OptionCategory` is ACP's five, an
  unknown one read as `other`. An agent with only the older `modes` gets
  a mode option synthesized from them (`session/set_mode` sets it). Ids
  are opaque and echoed back. Nothing is invented: what an agent does not
  offer is not offered.
- **A probe (3), cached.** `harness.options { harness, profile? }`
  starts the agent under that profile in the node's probe directory, runs
  `initialize` and `session/new`, takes the options and stops it; no
  prompt is ever sent. Concurrent callers for one harness and profile
  share one probe; a success is kept ten minutes, or until the harness
  reports another version; a failure is shared only with those who waited
  for it. A harness that cannot run structured sessions on the node
  offers an empty list; one not installed, or whose agent does not start,
  is a typed error with the agent's last lines in the message. The probe
  is bounded by the handshake timeout.
- **Choices in the handshake.** `SessionMode::Structured.options` are
  applied after `session/new` or `session/load`, in order, after
  `model`, each checked against what the agent reported after the changes
  before it. One the agent does not offer, or refuses, fails the session,
  as an unknown model does: a session must not run in a permissions mode
  nobody asked for.
- **One report after the handshake, then every change.** The driver
  reports the options right before `Ready`, as they stand after the
  choices, empty when the agent offers none (a client tells "nothing
  offered" from "not known yet" by it), then whenever they differ from its
  last report: a `session.set_option` (the agent's answer is the reply;
  its refusal the error), or the agent's own `config_option_update` or
  `current_mode_update`. The session logs each as `options_changed`, the
  whole list. `session.options` answers from the driver.
- **Titles are `SessionTitle` (2).** A typed title is trimmed, one line,
  at most 120 characters, refused otherwise; a title from a prompt is its
  first non-empty line, cut at 80. `SessionInfo.title` is the one truth
  for the current title: `spec.title` when asked for at creation, then
  the first prompt's line for an untitled structured session, then each
  `session.rename`; each change is a `renamed` record, and the store's
  `title` column follows them, so a title survives the node's restart.
  `spec.title` stays what was asked.
- **Choices travel.** A session continuing elsewhere keeps what it runs
  with: its last report's values as choices (before any report, its
  spec's own). A profile move applies them to the restarted agent; an
  export carries them (`TransferDetail::Session.options`, with the title
  in its info), and the import applies them after the load, failing
  loudly where the other agent does not offer one.

## Why

- Passing the options on is the only way the controls stay true as
  agents change: the efforts Claude offers depend on its model, OpenCode
  offers no effort at all, and Vessel learns of a new model the day the
  agent does. Vessel's own shapes keep ACP's version out of the wire and
  let clients match categories and kinds exhaustively.
- A probe is the agent's own answer for the account that will run the
  session; a cache keeps it to one agent start per ten minutes.
- Failing a session on a choice not offered is the rule protocol 7 set
  for models, for the same reason: running in a different mode than
  asked is worse than not running.
- The report before `Ready` means no client ever sees a running session
  whose options are unknown.
- One truth for the title, kept by the store from the log, is what made
  a rename survive a restart; parsing it once keeps a newline out of every
  card, tab and terminal title.
- Choices dropped at a move would change a session's permissions without
  anyone asking.

## Consequences

- A probe starts the agent: a few seconds, and whatever `session/new`
  leaves in the harness's home (Claude Code makes an empty project folder
  for the probe directory; `opencode` records a session). It runs in one
  directory of the node's, so the trace is always the same one.
- An older node never receives options or a title it would ignore:
  `Call::since` makes a `session.create` carrying either need protocol 9.
- `options_changed` and `renamed` reach protocol 2-8 clients as
  `harness_log` lines (`SessionOption::summary`).
- A stored (ended) session has no agent: `session.options`,
  `session.set_option` and `session.rename` on it are `conflict`; its
  log's last `options_changed` is what it ran with.
- A rename during a profile move waits for the move to finish: the
  actor publishes it once the move is over.
- Options the agent reports of a kind ACP adds later are left out until
  Vessel maps them.
