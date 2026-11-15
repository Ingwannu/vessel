# ADR 0057: Codex, Claude Code and OpenCode over their own protocols; the rest stay on ACP

Status: accepted, 2026-11-14. The owner made the decision ("let us do it
exactly as Paseo does; Claude Code, OpenCode and the other providers too,
where possible"). This ADR records how it is done for each agent, and what
it costs. It partly supersedes ADR 0003 (every structured agent over ACP)
and replaces ADR 0055's wait for an adapter change. The research notes
are summarised below; the plan is `docs/plans/agents-own-protocols.md`.
Slice 0 (the seam made selectable, protocol 18) is built (see "Slice 0,
as built"), and Codex's driver through its slice 2 is built and offered
(see "Codex slice 2, as built").

## Context

**What Paseo does**
(`packages/server/src/server/agent/provider-registry.ts`, lines 188 to
222):

| Agent | How Paseo drives it | Where |
| --- | --- | --- |
| Claude Code | Anthropic's Agent SDK, loaded in process, which drives the `claude` executable | `providers/claude/agent.ts` (7,106 lines) |
| Codex | its own client of `codex app-server` (JSON-RPC over stdio) | `providers/codex-app-server-agent.ts` (7,927 lines), `codex/` (`app-server-transport.ts`, `options.ts`, `rewind.ts`, `thread-history.ts`, `async-questions.ts`, `tool-call-mapper.ts`), `codex-feature-definitions.ts` |
| OpenCode | `@opencode-ai/sdk/v2` against an `opencode serve` it starts and manages | `providers/opencode-agent.ts` (5,538 lines), `opencode/` (`server-manager.ts`, `bridge.ts`, `event-consumer.ts`) |
| Pi | its RPC mode | |
| Copilot, Cursor, Kimi, Kiro, Trae, user-defined | ACP | `*-acp-agent.ts`, `generic-acp-agent.ts` |

"As Paseo" therefore means that three agents leave ACP: Codex, Claude
Code and OpenCode. Everything else stays on ACP: Gemini, every agent
added on a node (ADR 0032), and whatever is not one of the three.

**What Vessel has** (ADR 0003): every structured agent speaks ACP, either
natively or through an adapter (`vessel-harness/src/harnesses/mod.rs`,
`Structured::{Native, Adapter}`):

- Claude Code goes through `claude-agent-acp`, a Node program that hosts
  the same Agent SDK.
- Codex goes through `codex-acp`, a Node program that is itself a client
  of `codex app-server`.
- OpenCode runs `opencode acp`, which is its own first-party ACP.

`vessel-agent` connects to one of these and turns it into a protocol-free
seam:

- `AgentSetup`: folder, model, resume, fork, choices, MCP servers,
  handshake timeout, questions.
- Commands: prompt, cancel, steer, answer a permission, answer a question,
  turn running, options, set an option.
- Outputs: Vessel's `AgentEvent`s, options, commands, usage, the agent's
  introduction, and the disconnect.
- Two calls besides the session: `probe` and `history`.

The session actors, the store and the wire to clients see only this
seam.

**The node is Rust.** Paseo loads the vendors' TypeScript SDKs in its own
process; the node cannot. For each of the three agents, three ways were
weighed:

- (a) speak the agent's own wire from Rust;
- (b) host the vendor's SDK in a small sidecar process;
- (c) stay on the ACP adapter.

## Evidence, per agent

### Codex

- **The protocol** is OpenAI's own: the desktop app, the IDE extension
  and `codex-acp` are all clients of it. The binary generates its schema
  itself (`codex app-server generate-json-schema [--experimental]`),
  stable and experimental apart. Codex 0.166.0 has 106 stable client
  requests and 179 counting the experimental ones.
- **What Vessel needs is stable**: `initialize`, `model/list`,
  `config/read`, `thread/start`, `resume`, `fork`, `read`, `list`,
  `turns/list`, `revert`, `archive`, `metadata/update`, `name/set`,
  `compact/start`, `turn/start`, `steer`, `interrupt`, `skills/list`,
  `account/read`, `account/rateLimits/read`, `permissionProfile/list`.
  So are the server's approval requests (`item/commandExecution/
  requestApproval`, `item/fileChange/requestApproval`,
  `item/permissions/requestApproval`), `item/tool/requestUserInput` and
  `mcpServer/elicitation/request`.
- **What is experimental**:
  - `turn/start.cyberAccessProgram`, `daybreakEnabled`;
  - `thread/settings/update`, `turn/settings/update`;
  - `collaborationMode/list` (plan mode);
  - `thread/queue/*`.
- **It moves.** Paseo's rewind calls `thread/rollback`
  (`codex/rewind.ts`), which Codex 0.166.0 no longer has; it has
  `thread/revert`. An update can remove what a driver uses.
- (b) is what `codex-acp` already is, so it would add nothing.
- **Recommendation: (a).** Codex's own protocol, from Rust, against the
  generated schema.

### Claude Code

- **What the SDK speaks.** The Agent SDK (0.3.246) starts
  `claude --input-format stream-json --output-format stream-json
  --verbose`, with `--permission-prompt-tool stdio` when the host answers
  permissions. It exchanges `control_request` / `control_response` /
  `control_cancel_request` frames with the CLI:
  - from the host: `initialize`, `interrupt`, `set_permission_mode`,
    `set_model`, `set_max_thinking_tokens`, `rewind_files`,
    `get_context_usage`, `mcp_*`, `generate_session_title` and more;
  - from the CLI: `can_use_tool`, `hook_callback`, `mcp_message`.
- **What is documented.** Anthropic's CLI reference documents
  `--input-format` and `--output-format stream-json`,
  `--include-partial-messages`, `--replay-user-messages`, and
  permissions through an MCP tool (`--permission-prompt-tool
  mcp__server__tool`, `--permission-prompts host|none`). It does not
  document the control frames: they are the SDKs' internal wire. They are
  visible only in the SDK's type declarations and in the open-source
  Python SDK.
- **The wire tolerates version drift in practice.** The SDK pins a CLI
  version (`claudeCodeVersion` 2.1.246 in its `package.json`, a binary per
  platform with checksums in `manifest.json`). Paseo nevertheless passes
  the user's own `claude` (`resolveClaudeBinary`, `agent.ts` 1904 and
  3676-3704), so it runs SDK 0.3.246's wire against whatever CLI is
  installed (2.1.322 here).
- (b) would be a second `claude-agent-acp` speaking a protocol of Vessel's
  own. (c) is the adapter Vessel already has.
- **Recommendation: (a)**, built in two layers:
  - **The documented layer**: stream-json both ways, and the node's
    permission model, whose safety never depends on the undocumented wire
    being understood: what is not understood is denied (amended
    2026-11-14 with the coordinator and A2, below).
  - **The control frames, opt-in**: interrupt, live model and mode
    changes, `rewind_files`, context usage. They are checked by a contract
    test against the SDK's declarations of the version Vessel follows,
    and the driver turns them off where that check fails.
  - **Permissions** (coordinator's decision, 2026-11-14): inside the CLI
    versions the contract test covers, every tool request comes over the
    control protocol's `can_use_tool`, which carries the agent's own
    choices ("always allow …") and their withdrawal. Outside that range,
    or when the control handshake does not come up as the contract
    expects, the driver falls back by itself to the documented
    `--permission-prompt-tool mcp__…` route (allow or deny only), and the
    session says once that the agent's own choices are unavailable on
    that version. On both routes, tested on the fake CLI: every tool
    request is answered by the node or denied; a frame the driver does
    not understand is a denial and ends the turn with a typed error,
    never an allow; no permission-skipping flag unless the session's mode
    is bypass; the mode is always passed (Paseo's default, `auto`, when the
    session chose none), so a setting in the person's own Claude folder,
    which the node cannot see, never decides it; MCP configuration and
    anything secret go in a file only the node's user can read, never on
    the command line.
  - **Credentials**: the driver never reads, copies or proxies the agent's
    credential files or tokens, and never presents a login of its own: it
    runs the person's own installed `claude` under the person's own login
    (the profile's home), as the ACP adapter does.

  This is "where possible" taken at its word: possible from Rust, but
  only partly a public contract, so the rest is fenced.

### OpenCode

- **The protocol** is first-party and documented
  (`opencode.ai/docs/server`):
  - `opencode serve` on loopback;
  - an OpenAPI document at `/doc`;
  - basic auth with `OPENCODE_SERVER_PASSWORD`;
  - server-sent events at `/event` and `/global/event`;
  - sessions: create, list, `message`, `prompt_async`, `abort`,
    `permissions/:id` (with `remember`), `revert`, `fork`.
- **ACP is also first-party here**: OpenCode speaks ACP itself
  (`opencode acp`), so (c) is not a third party's adapter.
- **Recommendation: (a)**, last in order. It is the owner's choice, and
  the HTTP API carries what ACP does not: revert and fork natively,
  permissions with "remember", the server's own session list. Because
  OpenCode's ACP is first-party, keeping it as the fallback costs least
  here.

## Decision

- **Drivers.** `vessel-agent` gets a driver per protocol behind the seam
  it already has: `acp` (today's), `codex_app_server`, `claude_stream`
  and `opencode_http`. Each turns its agent's wire into the same
  commands and outputs. Nothing above the seam changes. On the wire to
  clients, the only change is what an agent may now advertise, as options
  and abilities.
- **Selection.** `vessel-harness` describes how each harness's structured
  sessions start: `Structured::{Native, Adapter}` gains the direct
  protocols. A node setting, per agent, selects `direct` or `acp`
  (`node.settings`, ADR 0038's file). ACP stays the default for each
  agent until its direct driver has run on real agents with the owner's
  leave. Rollback is that setting, not a revert.
- **Versions.** Each direct driver states the agent versions it was
  checked against, and reads the installed version at start
  (`codex --version` and `initialize.userAgent`; `claude --version`;
  OpenCode's `/doc` `info.version`):
  - below the minimum, the session runs on ACP and says why;
  - above the newest checked, the session runs, the node logs it once,
    and the contract test is the place to look.
- **Contract tests.** Kept in the repository and run with the gate:
  - **Codex**: the subset of the generated schema the driver uses,
    checked in, with a test that regenerates it from an installed `codex`
    when one is present and fails on any change to that subset.
  - **Claude Code**: the control frames the driver uses, checked against
    the SDK's `sdk.d.ts` of the version followed.
  - **OpenCode**: the operations used, against `/doc`.
- **Security.** The node's permission model holds on every driver:
  nothing runs without asking that ACP would have asked about.
  - **Codex**: `approvalPolicy: on-request` with a sandbox from the
    session's mode; never `never` with `danger-full-access` unless the
    session's mode is full access (Paseo's presets, `codex-app-server-
    agent.ts` 321-347).
  - **Claude Code**: every tool request goes through the node
    (`can_use_tool`, or the documented MCP permission tool). The driver
    passes `--allow-dangerously-skip-permissions` only for a session
    whose mode is bypass. Paseo passes it always (`agent.ts` 3701); Vessel
    does not.
  - **OpenCode**: the server bound to loopback with a password of the
    node's own per server, and the session's permission rules set to
    "ask" for what ACP asked about.
  - Credentials stay the agent's own home's. A read-only device sees what
    it sees today.
- **What is gained** (from Paseo's code and the schemas):
  - **Codex**:
    - the speed tier as a select of the model's `serviceTiers`, beyond
      on/off (`codex-feature-definitions.ts`);
    - Daybreak, with ADR 0055's option shape (`daybreak`: `off`, `blue`,
      `red`). It is stored with `thread/metadata/update { daybreakEnabled
      }`, and each turn carries `cyberAccessProgram`: Red when the model
      offers it, else Blue, and `standard` when off. It is refused for a
      provider other than OpenAI, as upstream PR #737 and the Codex app
      do; the app lowers Red to Blue for a model that is not a cyber
      model.
    - native thread history (`thread/list`, `thread/turns/list`) and
      revert;
    - plan mode (collaboration mode);
    - goals, steering, questions (`item/tool/requestUserInput`);
    - rate limits.
  - **Claude Code**:
    - restoring files on a rewind (`rewind_files`, which ACP lacks: ADR
      0052 says files are not put back);
    - live model and permission-mode changes;
    - context usage, subagent transcripts, background tasks, hooks.
  - **OpenCode**:
    - native revert and fork at a message;
    - permissions with "remember";
    - the server's session list.
- **What must be rebuilt per direct driver** (everything ACP gives today):
  - permissions, with the agent's own choices;
  - questions, plans, steering, cancel and its settling, usage, commands,
    options and choices;
  - the abilities flags;
  - fork and rewind (now on each agent's native calls; ADR 0052's points
    become each agent's own ids);
  - `agent.history` (ADR 0053) and `probe`;
  - install and sign-in checks;
  - MCP servers handed to the agent;
  - the replay on resume, which parking (ADR 0038), moves (ADR 0011) and
    rewinds rely on.

  Each driver passes the existing session tests that exercise those, run
  against its fake.
- **Tests without real agents.** The testkit gains `fake_codex_app_server`,
  `fake_claude_stream` and `fake_opencode_server`, as `fake_acp` is for
  ACP. Each replays shapes taken from the schemas (Codex's generated
  JSON Schema, the SDK's declarations, OpenCode's `/doc`), never invented
  ones. Each driver's list of what only a real turn can verify is in the
  plan, to be run with the owner's leave.
- **Order.** Codex first (the owner's Daybreak and speed tiers), then
  Claude Code, then OpenCode, each in slices that gate on their own (the
  plan).

## Why

- **Codex**: its app-server is the vendor's own, documented by a
  generated schema, and used by the vendor's own app. Going through an
  adapter has cost Vessel the features the owner asked for: Daybreak (ADR
  0055) and the speed tiers.
- **Claude Code**: a direct driver removes two layers (Node and the
  adapter) and reaches what the adapter does not expose, such as file
  restore and live mode changes. Fencing the undocumented frames keeps a
  CLI update from breaking the documented core.
- **OpenCode**: done for the owner's consistency, but last and cheapest
  to fall back from, since its ACP is its own.

## Consequences

- **Size.** The node owns three protocol clients in Rust. In Paseo they
  are 5,500 to 7,900 lines each, plus mappers. This is the largest piece
  of work in the milestone, and each driver needs upkeep at each agent
  release.
- **The ACP driver stays**: for every other agent, and as the fallback
  for these three.
- **ADR 0003 is amended**: ACP is the protocol for structured agents
  except where a direct driver is selected.
- **ADRs 0052 and 0053 are rebuilt per driver** on native calls. Their
  wire does not change.
- **ADR 0055's option** becomes real on the Codex driver. Its "no
  configuration key" sentence stands until verified: a third party reports
  that `codex exec -c daybreak=true` sets the thread's choice, which has
  not been checked here.

## Slice 0, as built (2026-11-14, protocol 18)

Where the build differs from the decision above, this section is what
holds.

- **The harness table** is `vessel_harness::direct`, beside
  `Structured::{Native, Adapter}` rather than in it: a harness's ACP
  launch and its own protocol are both kept, since ACP is the fallback.
  Each entry names the protocol, the arguments that switch the agent's
  own program to it (`codex app-server`; `claude -p --input-format
  stream-json --output-format stream-json --verbose`; `opencode serve
  --hostname 127.0.0.1 --port 0`), the oldest and newest versions
  checked, and whether it is offered (`selectable`: false for all three
  in slice 0; Codex's true from its slice 2).
  `launch_direct` builds the command with the account, profile and route
  of an ACP launch; what a session adds (resume, MCP configuration,
  permission flags) is the driver's to add from `AgentSetup`, in its
  `launch` step before the agent is spawned (below).
- **The seam** is `vessel_agent::Driver` and three entries that take it:
  `AgentProcess::spawn_with`, `probe_with`, `history_with`. A direct
  driver is a module (`codex`, `claude`, `opencode`) with the ACP
  driver's two entry points, `run` and `history(command, cwd, from,
  limit)`, and a third, `launch(command, setup) -> (command, Launched)`,
  which shapes the agent's command for one launch before `spawn_with`
  spawns it (added the same day: Claude Code takes resume, fork, its
  permission route and its MCP file only on its command line, and
  `OpenCode` needs its server's password set before it starts). `run`
  gets the setup and the `Launched` and keeps it for the session's life.
  `AgentSetup::private_dir` carries the node's owner-only MCP directory
  to a session's launches (never to a probe or a listing), the only place
  a driver writes a launch's private files, through the harness's
  `McpFile`, which checks at every write that the directory is the
  writer's own, owner-only and not a link (Unix; Windows relies on the
  data directory's ACL). A module whose `BUILT` is false is refused
  before its agent starts (`AgentError::NoDriver`, `unsupported`); a
  launch a built driver cannot honour is refused there too
  (`AgentError::Refused(LaunchRefusal)`: a choice not offered, a fork it
  cannot make, a conversation id it cannot have made, its private file),
  answered as the ACP path answers the same problem.
- **The choice** is the session runtime's (`runtime/drivers`), at a
  session's first launch, its start again, a probe and a listing: the
  node's setting, then whether the build offers the driver, then the
  agent's `--version` (kept per program until the file changes). The
  session keeps its driver for its life (`Launch::driver`,
  `SessionHandle::driver`): a move, a park and a rewind restart it with
  the same one, since its options are that driver's. Below the oldest
  version or unreadable: ACP and a line in the session's log; above the
  newest: the driver and a line in the log (each session's, not once a
  node, so it is seen where it matters).
- **The setting** is `NodeSettings.drivers` (protocol 18): `acp`,
  `direct` or `unknown` for Claude Code, Codex and `OpenCode`. `direct` is
  refused while not offered; a newer build's saved choice runs as ACP and
  stays in the file. The CLI sets it with `vessel node settings --driver
  codex=direct`.
- **Fakes**: `fake-codex-app-server` and `fake-claude-stream` answer
  `--version` and their protocol's first exchange (`initialize`; a
  `system` `init` and a `result` frame). `OpenCode`'s fake needs an HTTP
  server and comes with its first slice.

## Codex slice 1, as built (2026-11-14)

The Codex driver starts and resumes a thread, runs turns, streams their
messages, reasoning summaries, plans, tool-like items and tokens, steers
and interrupts a turn, and declines every approval with a line in the
session's log. It is built (`codex::BUILT`) and not offered
(`selectable` false), because a driver that cannot ask a person must not
run a session. Choices made here:

- **No JSON-RPC library**: Codex's lines leave out `jsonrpc`, which the
  ACP SDK requires; the driver reads and writes the four shapes itself
  (`codex/wire.rs`), one reader and one writer task, the loop matching
  answers to its requests by id and never awaiting one in place.
- **Typed subset, checked**: the messages used are types
  (`codex/protocol.rs`) that ignore unknown fields and keep unknown kinds
  as `Other`; `schema/app-server.json` holds the 43 definitions of Codex
  0.166.0's schema they follow; a test checks every field and value used
  is there, and another regenerates them from an installed Codex of at
  least 0.166.0 and fails on any change.
- **Tokens**: a turn's usage is the growth of the thread's total over
  the turn; the context's fullness is the last request against
  `modelContextWindow`.
- **Until slice 2**, the thread runs `on-request` in a `workspace-write`
  sandbox (Paseo's default).

## Codex slice 2, as built (2026-11-14)

The Codex driver asks a person what Codex asks, and the node may select
it (`selectable` true). Choices made here:

- **The mode is an option** (`mode`), with codex-acp's id, values and
  presets (`read-only`, `workspace-write`, `agent` the default with
  Codex's own reviewer first, `agent-full-access` the only one that never
  asks and leaves the sandbox), so a session's choices mean the same
  whichever driver runs it. It goes with the thread's start or resume; a
  change goes with the next `turn/start` (`approvalPolicy`,
  `approvalsReviewer`, `sandboxPolicy`), as codex-acp applies it. Until
  slice 3 it is the one option; any other choice fails the handshake.
- **Approvals are permission requests with Codex's own choices**: a
  command's `availableDecisions` (each sent back as Codex gave it, a rule
  and all; one this build does not know left out), a file change's four
  decisions, more permissions granted as asked for the turn or the
  session, or none. The request's tool call is the item's, announced from
  the request when Codex has not announced the item, then updated by it.
- **Questions**: Codex's own (`item/tool/requestUserInput`) become one
  question of a field each (a choice where it offers options and no free
  text, otherwise text); a tool server's form is read as ACP's by the ACP
  driver's own mapper, so one reader holds the bounds and checks. A
  secret question (`isSecret`) is never shown, since an answer is kept in
  the session's timeline; with nobody watching, a question is answered
  empty and a form declined at once, each with a line in the log.
- **Withdrawal**: a cancel withdraws what is open before it interrupts
  (Codex hears `cancel`, no permissions, no answers), and what Codex asks
  until the turn ends is withdrawn at once; the turn's end and the
  connection's withdraw the rest; Codex's `serverRequest/resolved`
  settles one it no longer waits on. At most 64 wait, as over ACP.
- **Another thread's approvals** (a sub-agent's) are asked too: unasked,
  Codex would wait on them for good, and nothing is allowed unasked.
- **Checked**: `schema/app-server.json` grows to the 61 definitions the
  driver follows; the contract test checks each field and value used,
  and that a granted permission profile has the requested one's shape.

## Codex native MCP increment (2026-11-15, ADR 0081)

The earlier slice checkpoints above describe their original state. Native MCP
is now implemented for both thread/start and thread/resume as
config.mcp_servers, using a native-only serializer of ResolvedMcpServers.
Stdio env and HTTP headers stay secret-wrapped until explicit serialization;
Debug prints only the server count. An empty set omits config to retain native
defaults. This reuses the resolved launch snapshot and does not create a
second secret resolver or environment transport. The positive test checks
exact synthetic native payloads for both start/resume and empty omission.
The old MCP-omission assertion narrowed only after that positive passed;
unsupported option refusals remain. Source review and the combined package
gate are still owed; no real helper or endpoint was executed.
