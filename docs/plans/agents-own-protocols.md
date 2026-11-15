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

- `vessel_agent`: `AgentSetup` (with `private_dir`, added 2026-11-14
  by the launch-step commit below, and closed again); the commands
  (`input::Command` and `AgentInput`'s methods); `AgentOutput` and
  `AgentOutputs`; `Disconnect`; `AgentError::NoDriver`; `Driver` and
  `Driver::built`; each direct driver's entries `launch(&CommandSpec,
  &AgentSetup) -> Result<(CommandSpec, Launched), AgentError>` (its own
  `Launched`), `run(ChildStdin, ChildStdout, mpsc::Receiver<Command>,
  mpsc::Sender<AgentOutput>, AgentSetup, Launched)` and
  `history(&CommandSpec, Option<&Path>, HistoryFrom, Duration) ->
  Result<HistoryPage, HistoryError>`; `HistoryFrom`, `HistoryPage`,
  `HistoryError`, `Probed`.
- **The launch step** (2026-11-14, agreed with the coordinator and A2):
  Claude Code takes resume, fork, its permission route and its MCP
  configuration file only on its command line, so a direct driver shapes
  its command in `launch`, from `setup`, before `AgentProcess::spawn_with`
  spawns it, and keeps what that leaves (`Launched`: Claude's MCP file)
  for the session's life in `run`. Codex's and `OpenCode`'s `launch` are
  the identity today; `OpenCode`'s will put the node's own server
  password into its environment there. `history` spawns inside each
  module, so a listing shapes its own command there and writes no file;
  a probe goes through `spawn_with` with `private_dir: None`.
  `AgentSetup::private_dir` is the node's owner-only MCP directory
  (`DataDir::prepare_mcp_dir`, emptied at every node start), set by the
  session runtime on a new session, a fork, a start again and a move (a
  wake and a rewind reuse the session's launch), and `None` for a probe;
  a driver writes there only through `vessel_harness::McpFile`.
- **Launch refusals** (2026-11-14, agreed with the coordinator and A2):
  `AgentError::Refused(LaunchRefusal)`, what a direct driver's `launch`
  returns for a launch it cannot honour, before anything is spawned:
  `Option` (a choice it does not offer: `invalid_request`), `Fork` (a
  fork it cannot make: the session answers `ForkRefused`, the adapters'
  `unsupported`), `Conversation` (an id the agent cannot have made:
  `invalid_request`), `NoPrivateDir` and `Mcp` (the private file:
  `spawn_failed`). `McpFile::write` checks its directory at every write
  on Unix (a directory of the writer's, owner-only, not a link, the same
  across the create); Windows relies on the data directory's ACL.
  Codex moves its fork refusal into `codex::launch` in its next slice.
- **A side question's context** (2026-11-14, with N5's side questions,
  protocol 19, ADR 0045): `Command::Prompt` carries `context`. A direct
  driver carries it (first, only to the agent, never kept for a later
  prompt, the request bounded by `PROMPT_BYTES` before anything is sent,
  `ContextLimit` past it) or refuses the prompt typed before sending
  (`InputError::ContextNotTaken`, `unsupported`); it never drops it. The
  Claude driver carries it; Codex refuses it until A2's next commit
  carries it.
- `vessel_harness`: `Direct` (its fields), `DirectProtocol`, `Version`,
  `VersionFit`, `launch_direct`, `version_check`, `claude_mcp_file`.
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
  history,drivers,lib}.rs` (the dispatch already names every module and
  its launch step; these files are closed to driver slices since the
  launch-step commit, and a `pub use` a slice needs is agreed first), the ACP
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
   - Every approval request is declined, with a log line (until slice
     2). That is why the setting could not select the driver yet.
   - `fake_codex_app_server` replays schema-shaped messages.
2. **Permissions and questions.**
   - **The mode** is an option of its own here (it decides what is
     asked), with codex-acp's id and values so a session's choices carry
     over between drivers: `mode` = `read-only` (on-request, read-only
     sandbox), `workspace-write` (on-request, workspace-write),
     `agent` (on-request, workspace-write, reviewer `auto_review`; the
     default, as codex-acp's) and `agent-full-access` (never,
     danger-full-access; the only one that sets either). It goes with
     `thread/start` and `thread/resume`, and a change goes with the next
     `turn/start` (`approvalPolicy`, `approvalsReviewer`,
     `sandboxPolicy`).
   - The approval requests (`item/commandExecution/requestApproval`,
     `item/fileChange/requestApproval`,
     `item/permissions/requestApproval`) become permission requests on
     the item's tool call (announced from the request when `item/started`
     has not come), with the agent's own choices: a command's
     `availableDecisions` (accept, accept for the session, accept with
     the proposed rule, decline, cancel), a file change's four, more
     permissions for the turn or the session, or none. The answer goes
     back as the decision chosen; a cancel, the turn's end and the
     connection's withdraw what is open (`cancel`, no permissions).
   - `item/tool/requestUserInput` and `mcpServer/elicitation/request`
     (form mode) become questions: a question with options is a choice,
     one without (or with `isOther`) is text; an elicitation's form is
     read as ACP's (both are MCP's schema). With questions off (nobody
     watches), they are answered empty or declined at once, with a line
     in the log; other elicitation modes are declined. A secret question
     (`isSecret`) is never shown (the timeline keeps answers): answered
     empty, with a line in the log.
   - Codex's `serverRequest/resolved` settles a request it no longer
     waits on; a sub-agent thread's approvals are asked too.
   - After this slice the setting may select `direct` for Codex
     (`selectable`).
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
   As built (2026-11-14, `BUILT` stays false until slice 2):
   - `launch` adds only enumerated flags, checked ids and a path:
     `--include-partial-messages`, `--permission-prompt-tool=stdio`,
     `--disallowedTools=AskUserQuestion` (always, until slice 2 makes it
     follow `setup.questions`), `--session-id=<new UUID>` or
     `--resume=<UUID>` (anything else refused: `--resume` would read a
     name or a path), `--permission-mode=<mode>` always, from one
     exhaustive match: Paseo's five (`plan`, `default`, `acceptEdits`,
     `auto`, `bypassPermissions`) with Paseo's default `auto` for a
     session that chose none, so the node always knows the mode and a
     setting in the person's Claude folder never picks it (a stated
     difference from the ACP path); `dontAsk` is accepted, not offered;
     under Bedrock or Vertex `auto` is refused before spawning and the
     default is `default`, as Paseo does; only `bypassPermissions` adds
     `--allow-dangerously-skip-permissions`), `--model=<id>`,
     `--mcp-config=<file>` in
     the node's private directory (`vessel_harness::claude_mcp_file`).
     Value flags use `=`. Nothing is added to the environment. A fork, an
     option other than the mode, or MCP servers without the private
     directory are refused before spawning (`LaunchRefusal`).
   - The session opens the control protocol (`initialize`) within the
     handshake time and is ready with the launched conversation; a CLI
     that does not answer fails the handshake (slice 2 adds the MCP-tool
     fallback for that case). Text and reasoning come from the stream,
     a message never streamed from its whole frame, tool calls from whole
     frames, their results from `user` frames, the turn's end from
     `result`; a subagent's frames are left out until slice 5.
   - The floor: every `can_use_tool` is denied, with a line in the log; a
     control request it does not serve or cannot read is refused and the
     turn is interrupted and ends failed, saying why; a `result` that does
     not read ends the turn; a line that is not JSON ends the connection.
   - Attachments and opaque blocks are refused (`AttachmentNotTaken`,
     `InvalidContent`); steering is `CannotSteer`; options are empty.
   - The contract test (`claude::contract`, ignored unless
     `VESSEL_CLAUDE_SDK_TYPES` names the installed SDK's `sdk.d.ts`) checks
     every type, field and literal the wire uses against the SDK's
     declarations, and its `claudeCodeVersion` against `direct`'s.
2. **Permissions and questions.**
   - Every tool request comes through `can_use_tool` (`--permission-prompt-
     tool stdio`), with its suggestions as the agent's choices.
   - `AskUserQuestion` becomes a question; plan approval (`ExitPlanMode`)
     as Paseo shows it.
   - `--allow-dangerously-skip-permissions` is passed only for a bypass
     session.
   - After this slice the setting may select `direct` for Claude Code.
3. **Options.** Model, effort, fast and permission mode, changed live
   with `set_model` and `set_permission_mode`, or by restarting with
   `--resume` where the contract fails. As Paseo has them: the models
   from `initialize`'s list, each gated by its minimum Claude Code version
   (Opus 5.5 2.1.280, Sonnet 5.5 2.1.284, Haiku 5.5 2.1.293) against
   `init`'s `claude_code_version`; efforts low to max and `ultracode`;
   fast with `apply_flag_settings`; a live switch into bypass restarts
   with `--resume`, since only a launch for a bypass session carries the
   skip-permissions flag (a stated difference: Paseo always passes it).
   Steering (a prompt into the running turn) and images (jpeg, png, gif,
   webp) come here too.
4. **History, rewind, fork.** `--fork-session`, and resuming at a message.
   `rewind_files` restores files, which ADR 0052 could not; it needs
   `CLAUDE_CODE_ENABLE_SDK_FILE_CHECKPOINTING` at launch, the one
   variable the launch's allow-list then admits.
5. **The rest.** `probe`, the sign-in check (`claude auth status`, already
   used), context usage, subagent transcripts and text, an interrupt that
   spares background tasks, slash commands, MCP servers (`--mcp-config`,
   built in slice 1). Usage from the stream's own `rate_limit_event`
   frames: never from the agent's credential files, which Paseo's usage
   plugin reads (a stated difference, the credential invariant). No
   system-prompt append (Paseo sends one).

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
    (codex-acp's bundled CLI). Below 0.166.0 the node runs Codex over
    ACP (the version gate), and the contract test that regenerates the
    schema skips; both run once a Codex of at least 0.166.0 is installed;
  - (slice 1) a thread started with `approvalPolicy: on-request` and
    `sandbox: workspace-write`, and resumed with `excludeTurns`; which
    items a real turn sends with deltas and which whole; the order of
    `turn/start`'s answer and `turn/started`; the token usage a real turn
    reports (`modelContextWindow` above all); `userAgent`'s form, which the
    introduced version is read from.
  - (slice 2) each mode's `approvalPolicy`, `approvalsReviewer` and
    `sandbox` taken at a thread's start and resume, and a change taken
    with `turn/start`'s `sandboxPolicy`; whether a command's approval
    comes before or after its `item/started`; the `availableDecisions` a
    real command offers, and a rule sent back as offered; a permissions
    grant echoing the request; `item/tool/requestUserInput` (experimental)
    and an MCP form's real payloads; whether `serverRequest/resolved`
    comes before `turn/completed` for what an interrupt ends, and whether
    an answer to a request already resolved is harmless; a sub-agent
    thread's approvals.
- **Claude Code**:
  - each control frame on the installed CLI (2.1.322 here, the SDK
    follows 2.1.246): `initialize` answered before any prompt and what
    its answer holds; `interrupt` and the `result` an interrupted turn
    writes (its subtype, whether `stop_reason` is null);
  - that `--session-id` gives `init` the same id, and `--resume` keeps
    it (the driver logs a difference, never follows it);
  - that `--permission-prompt-tool=stdio` sends every tool request as
    `can_use_tool` in every mode but bypass, that a denial lets the turn
    go on, and that a refused unknown request (an elicitation) stops
    nothing on the CLI's side but the request;
  - a wake before the session's first turn: Claude Code writes the
    transcript only once a turn starts, so `--resume=<launched id>` may
    find no conversation (A2's read). What the CLI does then is not
    guessed; the driver fails the handshake at once when the CLI exits
    before it is ready, naming the resume (tested with the fake), so the
    wake fails typed and the session stays parked to be tried again;
  - whether prompts the node sends should carry `origin: {kind: human}`
    (the SDK asks hosts wrapping keyboard input to say so; scheduled
    prompts are not typed by a person);
  - a synthetic or local message (`/context`) shown once, and a
    subagent's frames;
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
- Codex slice 1 (start, prompt, stream): built, `BUILT` true, not offered
  (`selectable` false until slice 2). `crates/vessel-agent/src/codex/`:
  `wire` (JSON-RPC lines without `jsonrpc`, at most 64 MiB a line, a
  writer task), `protocol` (the subset as types, read leniently),
  `mapper` (items, deltas, plans, tokens, turn ends; prompts as text and
  mentions), `session` (the loop: `initialize`, `initialized`,
  `thread/start` or `thread/resume` with `excludeTurns`, then `Options`
  empty and `Ready`; turns with `turn/start`, `turn/steer`,
  `turn/interrupt`; every approval declined with a line in the log;
  other threads' notifications ignored), `contract` (the fields and
  values used are in `schema/app-server.json`, the 43 definitions of
  0.166.0's schema the driver follows; regenerated from an installed
  Codex of at least 0.166.0 and compared). A fork is refused at the
  handshake (`CannotFork`) and a listing answers `CannotList` until slice
  4; options are empty until slice 3, so a session asked for a choice
  fails its handshake (as for an option an ACP agent does not offer);
  MCP servers are not passed until slice 3 (`config`), so a direct Codex
  session has no Vessel tools yet, and its log says which it runs
  without.
  The thread runs `on-request` in a `workspace-write` sandbox, Paseo's
  default; slice 2 maps the session's mode instead. Checked against
  `fake-codex-app-server` (its scenarios in its file): the handshake's
  order and a prompt sent before it, a streamed reply (another thread's
  delta left out), usage and the turn's tokens, reasoning, a command, a
  plan, a message without deltas, a declined approval and its log line,
  a steer and a cancel, a failed turn, an agent that exits mid-turn, a
  resume, a missing thread and a fork refused at the handshake, a probe.
- Codex slice 1, after the integrator's review (2026-11-14): choices and
  tool servers are never dropped unannounced (above); the running turn's
  id is `turn/start`'s answer's alone, and only its `turn/completed` ends
  it (another client's turn on the thread does not), one that does not
  read ending it failed; a message that does not serialize is not sent
  and its caller is answered as for a closed input; a `file:` link's
  percent escapes are decoded. The contract test that runs the installed
  `codex` is opt-in (`VESSEL_CODEX_CONTRACT`, set only by the gate's step
  in AGENTS.md section 1, the one exception in section 8), in an empty
  environment but `PATH` with temporary homes, its output in a temporary
  directory, and says on standard error when it skips.
- Codex slice 1, two more from the review: the loop takes commands
  before the agent's lines, so a cancel (and, from slice 2, a
  permission's answer) never waits for a streaming reply to end
  (checked by a unit test that blocks the old order, and end to end with
  a cancel mid-stream); and an answer to `turn/start` that fails or does
  not read ends the turn failed, since only that answer names the turn.
- Codex slice 2 (permissions and questions): built and offered
  (`selectable` true). `codex/mode.rs` (the `mode` option, codex-acp's
  values and presets; the one option until slice 3, another choice fails
  the handshake), `codex/asks.rs` (approvals and questions held under
  node-minted ids, at most 64 each, the response each answer becomes, the
  withdrawal each refusal sends), `session` (permission requests on the
  item's tool call, announced from the request when the item has not
  come and updated by it; questions only when someone watches; a cancel
  withdraws before it interrupts and what is asked meanwhile is withdrawn
  at once; the turn's end and the connection's withdraw the rest;
  `serverRequest/resolved` settles). The schema subset grows to 61
  definitions; the contract test checks the new fields and values and
  that a granted permission profile is shaped as the requested one.
  Checked against `fake-codex-app-server`: a command's approval with a
  rule among its decisions (sent back whole, answered once), a cancel and
  Codex's own resolution withdrawing one, a file change and more
  permissions, Codex's questions and a tool server's form asked, checked
  and answered, a secret not asked, nobody watching, the mode at the
  thread's start, at each turn and changed between turns; unit tests for
  the bookkeeping (offered decisions, the bound, withdrawal, resolution)
  and the mode's choice. The node's setting accepts `direct` for Codex
  (e2e), and the runtime chooses it (unit).

### Later native MCP checkpoint (2026-11-15)

The historical slice1 MCP omission above is superseded by the ADR0081 native
MCP increment: start and resume now serialize resolved stdio/HTTP servers as
config.mcp_servers. Empty snapshots omit the override; the exact synthetic
transport and redacted Debug tests passed in the remote agent selection.
This does not enable other still unsupported choices or constitute a package
review, real MCP execution or installed-vendor compatibility result.
