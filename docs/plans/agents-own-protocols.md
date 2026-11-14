# Plan: agents over their own protocols (ADR 0057)

The owner's decision of 2026-11-14 ("as Paseo does"): Codex, Claude Code
and OpenCode get direct drivers; every other agent stays on ACP. This plan
splits the work into slices that each pass the gate on their own. No
driver code is written before the coordinator has read ADR 0057 and this
plan.

## The seam (exists)

`vessel-agent` already hides the protocol behind one seam, and every
direct driver implements that same seam:

- **What a session is started with**: `AgentSetup` (folder, model, resume,
  fork, choices, MCP servers, handshake timeout, questions).
- **Commands**: `Prompt`, `CancelTurn`, `Steer`, `Answer`,
  `AnswerQuestion`, `TurnRunning`, `Options`, `SetOption`.
- **Outputs**: `AgentOutput::{Event, Options, Commands, Usage, Introduced,
  Disconnected}`.
- **Calls besides a session**: `probe`, `history`.

The session actors, the store and the wire do not change.

## Slice 0: the seam made selectable (built, protocol 18)

- `vessel-harness` (`direct.rs`): the table of each harness's own
  protocol, its arguments, the versions checked and whether it is
  offered (`selectable`, false for all three); `launch_direct`;
  `version_check` (`<binary> --version`).
- `vessel-agent`: `Driver { Acp, CodexAppServer, ClaudeStream,
  OpencodeHttp }`; `AgentProcess::spawn_with`, `probe_with`,
  `history_with` dispatch on it; the modules `codex/`, `claude/`,
  `opencode/` hold `BUILT = false`, `run` and `history` stubs; a driver
  not built is `AgentError::NoDriver` before anything starts.
- `vessel-session` (`runtime/drivers.rs`): the choice at a session's
  first launch (setting, offered, version), kept for its life in
  `Launch::driver` and on its handle; the reasons go in the log.
- Wire (protocol 18): `NodeSettings.drivers` and
  `SetNodeSettings.drivers` (`AgentDriver`: `acp`, `direct`, `unknown`).
  `direct` is refused (`unsupported`) while not offered. CLI: `vessel
  node settings --driver AGENT=acp|direct`.
- Testkit: `fake-codex-app-server` and `fake-claude-stream` skeletons
  (`--version`, the first exchange). `OpenCode`'s fake comes with its
  slice 1 (it needs an HTTP server).

### Frozen by slice 0 (change only by agreement first)

Every driver implements or reads these; a change to one is agreed with
the coordinator and the other driver's worker before it is made.

- `vessel_agent`: `AgentSetup`; the commands (`input::Command` and
  `AgentInput`'s methods); `AgentOutput` and `AgentOutputs`;
  `Disconnect`; `AgentError::NoDriver`; `Driver` and `Driver::built`;
  the entry signatures `run(ChildStdin, ChildStdout,
  mpsc::Receiver<Command>, mpsc::Sender<AgentOutput>, AgentSetup)` and
  `history(&CommandSpec, Option<&Path>, HistoryFrom, Duration) ->
  Result<HistoryPage, HistoryError>`; `HistoryFrom`, `HistoryPage`,
  `HistoryError`, `Probed`.
- `vessel_harness`: `Direct` (its fields), `DirectProtocol`, `Version`,
  `VersionFit`, `launch_direct`, `version_check`.
- `vessel_proto::settings`: `AgentDriver`, `AgentDrivers`,
  `SetAgentDrivers`.

### What each driver's worker changes, and what not

- **Changes**: its own module (`crates/vessel-agent/src/codex/`,
  `claude/` or `opencode/`), its own fake
  (`crates/vessel-testkit/src/bin/fake_codex_app_server.rs`,
  `fake_claude_stream.rs`, later `fake_opencode_server.rs`) and its
  tests, its own entry in `vessel_harness::direct` (`selectable`, the
  versions checked) once its permissions slice lands, its section of
  this plan and of ADR 0057's notes.
- **Does not touch**: `crates/vessel-agent/src/{process,probe,
  history,drivers,lib}.rs` (the dispatch already names every module; the
  one exception is a `pub use` its slice needs, agreed first), the ACP
  driver (`crates/vessel-agent/src/driver/`), the other drivers' modules
  and fakes, `vessel-session` (the session actors, `runtime/drivers.rs`),
  `vessel-node`'s settings, and the wire. A shared session test is run
  with its fake, not edited for it.
- **The Claude Code worker** in particular (the node integrator, in
  parallel with Codex): `crates/vessel-agent/src/claude/`,
  `fake_claude_stream.rs`, the `CLAUDE` entry in
  `crates/vessel-harness/src/direct.rs`, and nothing under `codex/`.

## Codex (first)

The wire is JSON-RPC over stdio to `codex app-server`, using the user's
own `codex`. Its schema comes from `codex app-server
generate-json-schema --experimental`. The subset the driver uses is
checked in under `crates/vessel-agent/src/codex/schema/`, and a contract
test regenerates it when `codex` is installed.

1. **Start, prompt, stream.**
   - `initialize` (with `experimentalApi`), the version gate, then
     `thread/start` or `thread/resume` and `turn/start`.
   - Notifications become `AgentEvent`s: agent message deltas and
     reasoning summaries; items (command execution, file change, MCP tool
     call) as tool calls and their updates; `turn/plan/updated` as plans;
     `turn/completed` with usage.
   - `turn/interrupt` cancels and `turn/steer` steers.
   - Every approval request is declined, with a log line. That is why the
     setting cannot select the driver yet.
   - `fake_codex_app_server` replays schema-shaped messages.
2. **Permissions and questions.**
   - The approval requests (`item/commandExecution/requestApproval`,
     `item/fileChange/requestApproval`,
     `item/permissions/requestApproval`) become permission requests, with
     the agent's own choices.
   - `item/tool/requestUserInput` and `mcpServer/elicitation/request`
     become questions.
   - The session's mode maps to approval policy and sandbox, as in
     Paseo's presets. Full access is never set unless the mode is full
     access.
   - After this slice the setting may select `direct` for Codex.
3. **Options.**
   - `model/list` gives models and efforts.
   - Speed is a select of the model's `serviceTiers` plus the default.
   - Plan mode is a collaboration mode (experimental: offered only where
     the schema has it).
   - Daybreak is ADR 0055's option `daybreak` (`off`, `blue`, `red`, only
     what the model offers). It is stored with `thread/metadata/update`,
     and `cyberAccessProgram` goes on every turn: Red before Blue,
     `standard` when off. It is refused for a provider other than OpenAI.
   - MCP servers go in the thread's `config`.
4. **History, rewind, fork.**
   - `thread/list` and `thread/turns/list` serve `agent.history` and the
     replay on resume.
   - `thread/fork` serves `session.fork`, and `thread/revert`
     `session.rewind`: ADR 0052's points become turn and item ids, and the
     wire is unchanged.
5. **The rest.**
   - `probe` without a session (`model/list`, `config/read`).
   - The sign-in check (`account/read`), rate limits, skills as commands,
     compaction, goals.

Each slice has its section in ADR 0057's follow-up notes and in this
plan's progress, and passes the existing session tests against the fake.

## Claude Code (second)

The wire is `claude -p --input-format stream-json --output-format
stream-json --verbose --include-partial-messages`, using the user's own
`claude`. The control frames (`control_request` and `control_response`)
are opt-in: a contract test checks the ones the driver uses against the
Agent SDK's `sdk.d.ts` of the version followed (0.3.246 now, CLI
2.1.246; the installed CLI here is 2.1.322).

1. **Start, prompt, stream.** `--resume` and `--session-id`; assistant
   and user messages, partial deltas, tool use and result, and the result
   frame with usage. Cancel is the `interrupt` control frame where the
   contract holds, otherwise stopping the process and resuming.
   `fake_claude_stream` replays frames from the SDK's declared types.
2. **Permissions and questions.**
   - Every tool request comes through `can_use_tool` (`--permission-prompt-
     tool stdio`), with its suggestions as the agent's choices.
   - `AskUserQuestion` becomes a question.
   - `--allow-dangerously-skip-permissions` is passed only for a bypass
     session.
   - After this slice the setting may select `direct` for Claude Code.
3. **Options.** Model, effort, fast and permission mode, changed live
   with `set_model` and `set_permission_mode`, or by restarting with
   `--resume` where the contract fails.
4. **History, rewind, fork.** `--fork-session`, and resuming at a message.
   `rewind_files` restores files, which ADR 0052 could not.
5. **The rest.** `probe`, the sign-in check (`claude auth status`, already
   used), context usage, subagent transcripts, MCP servers
   (`--mcp-config`, as today).

## OpenCode (third)

The wire is HTTP and server-sent events to `opencode serve`: one server
per profile home, on loopback, started by the node with a random
`OPENCODE_SERVER_PASSWORD` that is never logged. The operations used are
checked against `/doc`.

1. **The server and a turn.** The server's lifecycle; `POST /session`,
   `prompt_async`, `/event`, `abort`.
2. **Permissions and questions.** `permissions/:id` with `remember`, and
   the session's rules set to ask for what ACP asked about.
3. **Options.** Providers, models, agents and modes.
4. **History, rewind, fork.** `GET /session`, `revert`, `fork`.
5. **The rest.** `probe`, the sign-in check, usage, commands.

## Who can work in parallel

- Slice 0 first, by one worker: it touches the harness descriptor, the
  driver dispatch and the settings, which every driver needs. Done.
- After it, Codex (`crates/vessel-agent/src/codex/`,
  `crates/vessel-testkit/src/bin/fake_codex_app_server.rs`) and Claude
  Code (`crates/vessel-agent/src/claude/`, `fake_claude_stream.rs`)
  touch no common file but one line each in
  `crates/vessel-harness/src/direct.rs` (their own entry). Two workers
  can take them at once.
- The shared session tests are run with each driver's fake, not edited
  by both.
- OpenCode is third, possibly by the worker that finishes first.

## What only a real turn can verify (with the owner's leave, later)

- **Codex**:
  - the approval requests' real payloads;
  - `cyberAccessProgram` accepted by the server for an account with
    Daybreak, and what "omission" picks;
  - the real service tier ids;
  - `thread/revert` on a real thread;
  - steering;
  - the user's `codex` (0.156.1 here) against the schema of 0.166.0
    (codex-acp's bundled CLI).
- **Claude Code**:
  - each control frame on the installed CLI (2.1.322 here, the SDK
    follows 2.1.246);
  - `can_use_tool` payloads and suggestions;
  - `rewind_files` on real file history.
- **OpenCode**:
  - 1.18's event shapes;
  - permission ids and `remember`;
  - revert on a real session.

## Progress

- ADR 0057 and this plan: accepted by the coordinator, 2026-11-14.
- Slice 0: built (protocol 18). Checked: the setting's wire and its
  version (`a_settings_change_naming_drivers_needs_protocol_eighteen`),
  its round trip and the agents it names; the node refuses `direct`
  (`unsupported`) and `unknown` (`invalid_request`), keeps a newer
  build's choice in the file and runs it as ACP (e2e); the runtime's
  choice before and after the version (unit, with a driver offered in
  the test); the version read from the agent's own program (with the
  fake); `launch_direct` and `version_check` (harness); every driver not
  built refused before its agent starts, for a session, a probe and a
  listing; the CLI's `--driver`; the fakes' first exchange.
- Not reachable yet, so not checked end to end: a session that runs a
  direct driver. The first driver slice that sets `BUILT` checks it.
