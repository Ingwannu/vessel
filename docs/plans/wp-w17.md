# W17: CLI parity package

Base: `d954fa3a7a534ace48c45a184f45be5ee437add8` (node, protocol 23).
Reference: Paseo 0.15.4, source commit `1c00b365c9e0b74c1580eba33d905d693125ae21`.
This inventory records baseline facts, not a claim that names or handlers imply parity.
117 executable invocation paths, including the `plugin add` compatibility alias are listed below.
The `speech` group and discovery-only group nodes have no action; they are not missing operations.

[Decision Log]
- Purpose and intent: make the existing typed node client usable with familiar CLI paths while retaining precise refusals for missing contracts.
- Existing implementation and constraints: `vessel-cli` already implements node/session/project/worktree/terminal/schedule operations; the base lacks workspace entities, subagent detach and host-registration APIs. Other tracks own lifecycle logs/labels/reload, daemon operations and workspace scripts. Quality B has not been announced.
- Alternatives considered: duplicate a second CLI/client, forward raw JSON RPC, or add aliases without matching semantics.
- Chosen approach: modify the existing clap CLI; separate typed command parsing, presentation, session operations and node-dependency refusals into feature modules. Reuse NodeClient calls; add no wire methods or crate dependencies.
- Why: reuse preserves token/identity checks and RPC version/scope enforcement; typed refusals prevent a command from silently performing a different operation.
- Advantages, costs and effects: existing workflows remain available; CLI-only changes can proceed before Quality B. Some paths remain dependency-blocked and their parser acceptance must never be counted as functional parity.

## Shared contract and structure

Paseo shared options and exits: `packages/cli/src/cli.ts:48-60`, `output/index.ts`,
`utils/command-options.ts:46`, `utils/daemon-target.ts:6`, `run.ts:53-59`.
Paseo table/json/yaml, --json precedence, quiet IDs, no-headers/no-color; help/version 0,
parse errors 1, runtime errors 1. Ordinary returned agent timeout/permission/error states are
exit 0. Baseline Vessel clap parse errors were 2; the current main edge maps parser
failures to1 and help/version to0. An exact process exit-code regression is now drafted;
no executed latest-head exit-code claim is made.
Paseo --host and --home are mutually exclusive. Vessel uses --node and --data-dir and a local
node book; daemon host registrations must not be emulated by mutating that local book.

Data flow: clap validates input → feature handler resolves selectors → existing NodeClient
request → typed response → presentation → stdout. Stream/attach paths retain raw output.
No live node, real provider, credential store or production configuration is used for checks.
Existing CLI modules remain the authority for lifecycle/daemon/scripts.

## Current integration state (2026-11-15 13:22 UTC)

The table below is the initial 117-path inventory, not a current green checklist.
The [current invocation map](wp-w17-current.md) records source adapters and refusals
for every path without converting them into pass claims.
The development tree merged announced node head82ae7c91 (gate46g) at4ce46b45e.
Its Rust/e2e/forge passes and coordinator-counted web load timeout describe that
base only. Latest W17 changes and the combined head remain unverified. No whole
W17 gate, package V3 pass, protocol assignment or merge handoff has been claimed.

| Boundary | Current state / owner |
| --- | --- |
| Shared output for W4 permit / W16 plugin | V3 scoped PASS for source5305: 6/0/0 root/process regressions and strict CLI alltarget Clippy0. Evidence-only handoff5b3ca1d is now on announced node head82ae7c91; W4/W16 were told to merge that head. No full W17 claim. |
| Existing-client CLI adapters | Scoped agent lists, text runs, sends with attachment staging, turn waits/cancel, forced archive/delete, mode options, project create/list/clone, schedules/heartbeat and terminal keys have source implementations. Exact0656 CLI units now134/0/0; older process/host checks are in the verification matrix. The public-host/dd33 strict run is queued, and later receipt/base changes still need checks. No latest-head/full-parity claim. |
| Run/send output | Typed command receipts now map idle completion to completed. Standalone wait remains idle. Wait/mutation tables and schedule/heartbeat deletion status are source-corrected; update/provider notices and detailed schedule logs retain documented native differences. |
| W17 node host registry / capture | Six pending rows and one None feature constant; source-owned reference metadata, separately typed bounded public local-agent inventory and capture actor glue staged. Paired destinations lacking node-side trust lookup and direct/SSH creation remain typed refusals. Capture must consume C2's one reviewed pure engine before handoff; no second parser may merge. |
| Workspace placement / project reset/cascade | W11 reviewed registry/operation contract awaited; WorkspaceId alone is not a workspace call. |
| Detach / atomic archive / open | W2 detach and W21 idle archive/deep links awaited; Creator is never treated as current parentage. |
| Provider / initial schema/image/env | W22 atomic preflight and per-turn contracts awaited. Pre-create refusal remains. The 64 KiB offline schema adapter and initial+2 retry loop passed scoped0656 units under cfg(test), not runtime wiring; Ajv extension equivalence remains owed. |
| Reports / terminal hooks / client book | W5 report identities, W7 authenticated attribution/ingestion, E2 canonical route/upsert seam awaited. No replacement stores/transports/endpoints are created in CLI. |
| Other owners | Lifecycle logs/labels/reload and daemon/workspace-script handlers remain with their owners. Permit W4 and plugins W16 remain separate modules, using the shared formatter after review. |

## Complete command inventory (baseline)

Flags/defaults and reference output/exits are recorded per path; global options above apply
in addition. W means Paseo withOutput (table, JSON, YAML, quiet); R means handler-owned raw.
The Vessel column names the baseline equivalent, including meaningful differences.

| Paseo command / arguments | Paseo flags/defaults | Vessel baseline command or none | Owner | Difference | Paseo output/exits | Source registration |
|---|---|---|---|---|---|---|
| `ls ` | `--thinking <id>`: Filter by thinking option ID<br>`--label <key=value>`: Filter by label (can be used multiple times); default=[]<br>`--background`: Include background workspace agents<br>`-g, --global`: List agents across all directories<br>`-a, --all`: Include archived and background agents | sessions | W17 | Different argument/output contract; not full parity. | W; agentLsSchema; 0/1 | `packages/cli/src/cli.ts:65` |
| `run <prompt>` | `--output-schema <schema>`: Output JSON matching the provided schema file path or inline JSON schema<br>`--wait-timeout <duration>`: Maximum time to wait for agent to finish (e.g., 30s, 5m, 1h). Default: no limit<br>`--label <key=value>`: Add label(s) to the agent (can be used multiple times); default=[]<br>`--env <key=value>`: Set environment variable(s) for the agent process (can be used multiple times); default=[]<br>`--cwd <path>`: Working directory (default: current)<br>`--image <path>`: Attach image(s) to the initial prompt (can be used multiple times); default=[]<br>`--workspace <id>`: Run in an existing workspace (defaults to the caller workspace when agent-scoped)<br>`--forge <forge>`: Forge for checkout-pr mode<br>`--pr-number <n>`: Pull request or change request number for checkout-pr mode<br>`--branch <name>`: Existing branch for checkout-branch mode<br>`--base <ref>`: Base ref for branch-off mode<br>`--new-branch <name>`: New branch name for branch-off mode<br>`--worktree-slug <slug>`: Managed worktree path slug<br>`--worktree-mode <mode>`: Worktree mode: branch-off, checkout-branch, or checkout-pr<br>`new Option("--worktree <name>", "Legacy workspace isolation alias").hideHelp()`<br>`--new-workspace <local\|worktree>`: Create a separate local or worktree workspace<br>`--mode <mode>`: Provider-specific mode (e.g., plan, default, bypass)<br>`--thinking <id>`: Thinking option ID to use for this run<br>`--model <model>`: Model to use (e.g., claude-sonnet-4-20250514, claude-3-5-haiku-20241022)<br>`--provider <provider>`: Agent provider, or provider/model (e.g. codex or codex/gpt-5.4)<br>`new Option("--name <name>", "Hidden alias for --title").hideHelp()`<br>`--title <title>`: Assign a title to the agent<br>`new Option("--detach", "Legacy alias for --no-wait").hideHelp()`<br>`new Option("-d, --background", "Legacy alias for --no-wait").hideHelp()`<br>`--no-wait`: Return immediately without waiting for completion | run HARNESS [--structured] | W17 | Different argument/output contract; not full parity. | W; outputSchema, structuredRunSchema(output), agentRunSchema; 0/1 | `packages/cli/src/cli.ts:67` |
| `import <id>` | `--label <key=value>`: Add label(s) to the agent (can be used multiple times); default=[]<br>`--cwd <path>`: Working directory for providers that require it<br>`--provider <provider>` **required**: Agent provider id | run PROVIDER --structured --resume ID | W17 | Different argument/output contract; not full parity. | W; agentRunSchema; 0/1 | `packages/cli/src/cli.ts:71` |
| `clone <repo>` | `--dir <path>` **required**: Parent directory to clone into (for example: ~/workspace)<br>`new Option("--protocol <protocol>", "Protocol for owner/repo shorthand repositories").choices(["https", "ssh"])` | project clone URL PARENT | W17 | Different argument/output contract; not full parity. | W; cloneSchema; 0/1 | `packages/cli/src/cli.ts:75` |
| `attach <id>` | — | attach ID | W17 | Different argument/output contract; not full parity. | R: human transcript/event text only; --json/--format do not change this handler; 0/1 | `packages/cli/src/cli.ts:89` |
| `logs <id>` | `--since <time>`: Show logs since timestamp<br>`--filter <type>`: Filter by event type (tools, text, errors, permissions)<br>`--tail <n>`: Show last n entries<br>`-f, --follow`: Follow log output (streaming) | log ID | lifecycle (logs) | Owned elsewhere; W17 does not modify this handler. | R: human transcript/event text only; --json/--format do not change this handler; 0/1 | `packages/cli/src/cli.ts:93` |
| `stop [id]` | `--cwd <path>`: Stop all agents in directory<br>`--all`: Stop all agents | send ID TEXT --interrupt (requires a new prompt) | W17 | Different argument/output contract; not full parity. | W; stopSchema; 0/1 | `packages/cli/src/cli.ts:97` |
| `delete [id]` | `--cwd <path>`: Delete all agents in directory<br>`--all`: Delete all agents | session delete ID --yes | W17 | Different argument/output contract; not full parity. | W; deleteSchema; 0/1 | `packages/cli/src/cli.ts:101` |
| `send <id> [prompt]` | `--no-wait`: Return immediately without waiting for completion<br>`new Option("--behavior <behavior>", "Active-turn behavior (agent default: steer; otherwise: interrupt)").choices(["interrupt", "steer"])`<br>`--image <path>`: Attach image(s) to the message; default=[]<br>`--prompt-file <path>`: Read the message from a UTF-8 text file<br>`--prompt <text>`: Provide the message inline as a flag | send ID TEXT | W17 | Different argument/output contract; not full parity. | W; agentSendSchema; 0/1 | `packages/cli/src/cli.ts:105` |
| `inspect <id>` | — | none | W17 | Missing entry point or backend contract; see dependencies below. | W; createInspectSchema(inspectData); 0/1 | `packages/cli/src/cli.ts:109` |
| `wait <id>` | `--timeout <seconds>`: Maximum wait time (default: no limit) | none | W17 | Missing entry point or backend contract; see dependencies below. | W; agentWaitSchema; 0/1 | `packages/cli/src/cli.ts:113` |
| `archive <id>` | `--force`: Force archive running agent (interrupts active run first) | session archive ID | W17 | Different argument/output contract; not full parity. | W; archiveSchema; 0/1 | `packages/cli/src/cli.ts:117` |
| `hooks <agent> <event>` | — | none | W17 | Missing entry point or backend contract; see dependencies below. | R: no stdout; missing env/unrecognized activity/network failure silently returns 0 | `packages/cli/src/commands/hooks.ts:22` |
| `onboard ` | `--listen <listen>`: Listen target (host:port, port, or unix socket path)<br>`--port <port>`: Port to listen on (default: 6767)<br>`--relay`: Enable relay connection without prompting<br>`--no-relay`: Disable relay connection<br>`--no-mcp`: Disable the Agent MCP HTTP endpoint<br>`--hostnames <hosts>`: 'Daemon hostnames (comma-separated, e.g. "myhost,.example.com" or "true" for any)'<br>`new Option("--allowed-hosts <hosts>").hideHelp()`<br>`--timeout <seconds>`: Max time to wait for daemon readiness (default: 600)<br>`--voice <mode>`: Voice setup mode: ask, enable, disable; default="ask" | init | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | R: interactive notes/progress/QR; cancel 0, failure 1 | `packages/cli/src/commands/onboard.ts:152` |
| `pair ` | `--relay`: Enable relay without prompting | node pair | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | R: QR/link + guidance; --json pairing offer; 0/1 | `packages/cli/src/commands/daemon/pair.ts:61` |
| `reload ` | — | node reload | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | W; daemonReloadSchema; 0/1 | `packages/cli/src/commands/daemon/reload.ts:64` |
| `restart ` | removed launch flags (listed below)<br>`--timeout <seconds>`: Replacement readiness deadline (default: 600)<br>`--kill-terminals`: Close every terminal instead of keeping them across the restart | none | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | W: object {action,target,supervisorPid,previousWorkerPid,workerPid,acknowledged}; human replacement confirmation; 0/1 | `packages/cli/src/commands/daemon/restart.ts:18` |
| `start ` | removed launch flags (listed below)<br>`--timeout <seconds>`: Readiness deadline (default: 600) | node start (foreground) | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | W: object {action,home,pid,listen,logPath}; human Started/Already running and log path; quiet PID; 0/1 | `packages/cli/src/commands/daemon/start.ts:8` |
| `status ` | — | status | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | W: local metadata + observed reachability/provider/relay status; human key:value lines; quiet daemon; 0/1 | `packages/cli/src/commands/daemon/status.ts:16` |
| `agent ls ` | `--thinking <id>`: Filter by thinking option ID<br>`--label <key=value>`: Filter by label (can be used multiple times); default=[]<br>`--background`: Include background workspace agents<br>`-g, --global`: List agents across all directories<br>`-a, --all`: Include archived and background agents | sessions | W17 | Different argument/output contract; not full parity. | W; agentLsSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:30` |
| `agent run <prompt>` | `--output-schema <schema>`: Output JSON matching the provided schema file path or inline JSON schema<br>`--wait-timeout <duration>`: Maximum time to wait for agent to finish (e.g., 30s, 5m, 1h). Default: no limit<br>`--label <key=value>`: Add label(s) to the agent (can be used multiple times); default=[]<br>`--env <key=value>`: Set environment variable(s) for the agent process (can be used multiple times); default=[]<br>`--cwd <path>`: Working directory (default: current)<br>`--image <path>`: Attach image(s) to the initial prompt (can be used multiple times); default=[]<br>`--workspace <id>`: Run in an existing workspace (defaults to the caller workspace when agent-scoped)<br>`--forge <forge>`: Forge for checkout-pr mode<br>`--pr-number <n>`: Pull request or change request number for checkout-pr mode<br>`--branch <name>`: Existing branch for checkout-branch mode<br>`--base <ref>`: Base ref for branch-off mode<br>`--new-branch <name>`: New branch name for branch-off mode<br>`--worktree-slug <slug>`: Managed worktree path slug<br>`--worktree-mode <mode>`: Worktree mode: branch-off, checkout-branch, or checkout-pr<br>`new Option("--worktree <name>", "Legacy workspace isolation alias").hideHelp()`<br>`--new-workspace <local\|worktree>`: Create a separate local or worktree workspace<br>`--mode <mode>`: Provider-specific mode (e.g., plan, default, bypass)<br>`--thinking <id>`: Thinking option ID to use for this run<br>`--model <model>`: Model to use (e.g., claude-sonnet-4-20250514, claude-3-5-haiku-20241022)<br>`--provider <provider>`: Agent provider, or provider/model (e.g. codex or codex/gpt-5.4)<br>`new Option("--name <name>", "Hidden alias for --title").hideHelp()`<br>`--title <title>`: Assign a title to the agent<br>`new Option("--detach", "Legacy alias for --no-wait").hideHelp()`<br>`new Option("-d, --background", "Legacy alias for --no-wait").hideHelp()`<br>`--no-wait`: Return immediately without waiting for completion | run HARNESS [--structured] | W17 | Different argument/output contract; not full parity. | W; outputSchema, structuredRunSchema(output), agentRunSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:32` |
| `agent import <id>` | `--label <key=value>`: Add label(s) to the agent (can be used multiple times); default=[]<br>`--cwd <path>`: Working directory for providers that require it<br>`--provider <provider>` **required**: Agent provider id | run PROVIDER --structured --resume ID | W17 | Different argument/output contract; not full parity. | W; agentRunSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:36` |
| `agent attach <id>` | — | attach ID | W17 | Different argument/output contract; not full parity. | R: human transcript/event text only; --json/--format do not change this handler; 0/1 | `packages/cli/src/commands/agent/index.ts:40` |
| `agent logs <id>` | `--since <time>`: Show logs since timestamp<br>`--filter <type>`: Filter by event type (tools, text, errors, permissions)<br>`--tail <n>`: Show last n entries<br>`-f, --follow`: Follow log output (streaming) | log ID | lifecycle (labels/logs/reload); W17 only non-label update fields | Owned elsewhere / excluded; W17 does not modify its handler. | R: human transcript/event text only; --json/--format do not change this handler; 0/1 | `packages/cli/src/commands/agent/index.ts:44` |
| `agent open <agent-id>` | `--server <server-id>`: Server ID (defaults to the local daemon) | none | W17 | Missing entry point or backend contract; see dependencies below. | W; openAgentSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:48` |
| `agent stop [id]` | `--cwd <path>`: Stop all agents in directory<br>`--all`: Stop all agents | send ID TEXT --interrupt (requires a new prompt) | W17 | Different argument/output contract; not full parity. | W; stopSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:52` |
| `agent delete [id]` | `--cwd <path>`: Delete all agents in directory<br>`--all`: Delete all agents | session delete ID --yes | W17 | Different argument/output contract; not full parity. | W; deleteSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:56` |
| `agent send <id> [prompt]` | `--no-wait`: Return immediately without waiting for completion<br>`new Option("--behavior <behavior>", "Active-turn behavior (agent default: steer; otherwise: interrupt)").choices(["interrupt", "steer"])`<br>`--image <path>`: Attach image(s) to the message; default=[]<br>`--prompt-file <path>`: Read the message from a UTF-8 text file<br>`--prompt <text>`: Provide the message inline as a flag | send ID TEXT | W17 | Different argument/output contract; not full parity. | W; agentSendSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:60` |
| `agent inspect <id>` | — | none | W17 | Missing entry point or backend contract; see dependencies below. | W; createInspectSchema(inspectData); 0/1 | `packages/cli/src/commands/agent/index.ts:64` |
| `agent wait <id>` | `--timeout <seconds>`: Maximum wait time (default: no limit) | none | W17 | Missing entry point or backend contract; see dependencies below. | W; agentWaitSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:68` |
| `agent mode <id> [mode]` | `--list`: List available modes for this agent | session option (none; options are client calls only) | W17 | Missing entry point or backend contract; see dependencies below. | W; modeListSchema, setModeSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:73` |
| `agent archive <id>` | `--force`: Force archive running agent (interrupts active run first) | session archive ID | W17 | Different argument/output contract; not full parity. | W; archiveSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:82` |
| `agent reload <id>` | — | node reload | lifecycle (labels/logs/reload); W17 only non-label update fields | Owned elsewhere / excluded; W17 does not modify its handler. | W; reloadSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:86` |
| `agent detach <id>` | — | none | W17 | Missing entry point or backend contract; see dependencies below. | W; detachSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:90` |
| `agent update <id>` | `--name <name>`: Update the agent's display name<br>`--thinking <id>`: Update the agent's thinking option ID<br>`--label <label>`: Add/set label(s) on the agent (can be used multiple times or comma-separated); default=[] | none | lifecycle (labels/logs/reload); W17 only non-label update fields | Owned elsewhere / excluded; W17 does not modify its handler. | W; updateSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:97` |
| `daemon set-password ` | — | none | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | W; setPasswordResultSchema; 0/1 | `packages/cli/src/commands/daemon/index.ts:26` |
| `daemon pair ` | `--relay`: Enable relay without prompting | node pair | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | R: QR/link + guidance; --json pairing offer; 0/1 | `packages/cli/src/commands/daemon/pair.ts:61` |
| `daemon reload ` | — | node reload | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | W; daemonReloadSchema; 0/1 | `packages/cli/src/commands/daemon/reload.ts:64` |
| `daemon restart ` | removed launch flags (listed below)<br>`--timeout <seconds>`: Replacement readiness deadline (default: 600)<br>`--kill-terminals`: Close every terminal instead of keeping them across the restart | none | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | W: object {action,target,supervisorPid,previousWorkerPid,workerPid,acknowledged}; human replacement confirmation; 0/1 | `packages/cli/src/commands/daemon/restart.ts:18` |
| `daemon start ` | removed launch flags (listed below)<br>`--timeout <seconds>`: Readiness deadline (default: 600) | node start | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | W: object {action,home,pid,listen,logPath}; human Started/Already running and log path; quiet PID; 0/1 | `packages/cli/src/commands/daemon/start.ts:8` |
| `daemon run ` | removed launch flags (listed below) | node run | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | W: lifecycle result; propagates result.exitCode (default 0); wrapper failure 1 | `packages/cli/src/commands/daemon/start.ts:38` |
| `daemon status ` | — | node status | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | W: local metadata + observed reachability/provider/relay status; human key:value lines; quiet daemon; 0/1 | `packages/cli/src/commands/daemon/status.ts:16` |
| `daemon stop ` | `--timeout <seconds>`: Graceful exit deadline (default: 15)<br>`--force`: Permit forced local process cleanup<br>`--keep-terminals`: Leave terminals running; the next daemon started for this home takes them back<br>`--kill-timeout <seconds>`: Forced exit deadline (default: 3) | node stop | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | W: action/home-or-host/pid/forced/usedLifecycleRpc as available; human local result or remote shutdown requested; 0/1 | `packages/cli/src/commands/daemon/stop.ts:14` |
| `daemon config get [path] ` | — | none | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | W; { idField: () => "configuration", columns: [], renderHuman: () => JSON.stringify(data, null, 2), }; 0/1 | `packages/cli/src/commands/daemon/config.ts:78` |
| `daemon config set <path> <value> ` | `--string`: Interpret value literally as a string | none | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | W; { idField: () => "configuration", columns: [], renderHuman: () => JSON.stringify(data, null, 2), }; 0/1 | `packages/cli/src/commands/daemon/config.ts:93` |
| `daemon config unset <path> ` | — | none | nodectl | Owned elsewhere / excluded; W17 does not modify its handler. | W; { idField: () => "configuration", columns: [], renderHuman: () => JSON.stringify(data, null, 2), }; 0/1 | `packages/cli/src/commands/daemon/config.ts:109` |
| `host ls ` | — | nodes list (local node book; not daemon host registrations) | W17 | Different argument/output contract; not full parity. | W; host schema (ID/LABEL/SERVER/ENABLED/STATUS/SOURCE); 0/1 | `packages/cli/src/commands/host/index.ts:85` |
| `host add <connection>` | `--label <name>`<br>`--meta <key=value>`: Set metadata; default=[] | nodes add NAME ADDRESS (local node book; not daemon host registrations) | W17 | Different argument/output contract; not full parity. | W; host schema (ID/LABEL/SERVER/ENABLED/STATUS/SOURCE); 0/1 | `packages/cli/src/commands/host/index.ts:88` |
| `host update <registration-id>` | `--label <name>`<br>`--connection <connection>`<br>`--replace-identity`: Accept a new destination identity<br>`--meta <key=value>`: Replace supplied metadata keys; default=[]<br>`--remove-meta <key>`: Remove a metadata key; default=[] | none (local node book; not daemon host registrations) | W17 | Missing entry point or backend contract; see dependencies below. | W; host schema (ID/LABEL/SERVER/ENABLED/STATUS/SOURCE); 0/1 | `packages/cli/src/commands/host/index.ts:99` |
| `host inspect <registration-id>` | — | none (local node book; not daemon host registrations) | W17 | Missing entry point or backend contract; see dependencies below. | W; { ...schema, renderHuman: () => JSON.stringify(host, null, 2) }; 0/1 | `packages/cli/src/commands/host/index.ts:112` |
| `host enable <registration-id>` | — | none (local node book; not daemon host registrations) | W17 | Missing entry point or backend contract; see dependencies below. | W; host schema (ID/LABEL/SERVER/ENABLED/STATUS/SOURCE); 0/1 | `packages/cli/src/commands/host/index.ts:112` |
| `host disable <registration-id>` | — | none (local node book; not daemon host registrations) | W17 | Missing entry point or backend contract; see dependencies below. | W; host schema (ID/LABEL/SERVER/ENABLED/STATUS/SOURCE); 0/1 | `packages/cli/src/commands/host/index.ts:112` |
| `host rm <registration-id>` | — | nodes remove NAME (local node book; not daemon host registrations) | W17 | Different argument/output contract; not full parity. | W; host schema (ID/LABEL/SERVER/ENABLED/STATUS/SOURCE); 0/1 | `packages/cli/src/commands/host/index.ts:112` |
| `project create [path]` | — | project add ABS_PATH | W17 | Different argument/output contract; not full parity. | W; projectSchema; 0/1 | `packages/cli/src/commands/project/index.ts:12` |
| `project ls ` | — | project list | W17 | Different argument/output contract; not full parity. | W; projectSchema; 0/1 | `packages/cli/src/commands/project/index.ts:19` |
| `project rename <project-id> [name]` | `--reset`: Clear the custom name and use the directory name | none | W17 | Missing entry point or backend contract; see dependencies below. | W; projectRenameSchema; 0/1 | `packages/cli/src/commands/project/index.ts:23` |
| `project delete <project-id>` | — | project remove PROJECT | W17 | Different argument/output contract; not full parity. | W; projectDeleteSchema; 0/1 | `packages/cli/src/commands/project/index.ts:33` |
| `workspace setup <workspace-id>` | — | none (workspace entity absent on base; do not alias worktree/project IDs) | W17 | Missing entry point or backend contract; see dependencies below. | W; workspaceSetupSchema; 0/1 | `packages/cli/src/commands/workspace/index.ts:13` |
| `workspace create ` | `--isolation <local\|worktree>` **required**: Workspace isolation<br>`--path <path>`: Local directory or source checkout (default: current)<br>`--project <id>`: Existing project id<br>`--title <title>`: Workspace title<br>`--mode <mode>`: Worktree mode: branch-off, checkout-branch, or checkout-pr (default: branch-off)<br>`--worktree-slug <slug>`: Managed worktree path slug<br>`--new-branch <name>`: New branch name (--mode branch-off)<br>`--base <ref>`: Base ref (--mode branch-off)<br>`--branch <name>`: Existing branch (--mode checkout-branch)<br>`--pr-number <n>`: Pull request or change request number (--mode checkout-pr)<br>`--forge <forge>`: Forge for --mode checkout-pr (default: source checkout)<br>`--background`: Create a background workspace<br>`--no-background`: Create a visible workspace instead of inheriting background | none (workspace entity absent on base; do not alias worktree/project IDs) | W17 | Missing entry point or backend contract; see dependencies below. | W; workspaceSchema; 0/1 | `packages/cli/src/commands/workspace/index.ts:20` |
| `workspace ls ` | `--background`: Include background workspaces | none (workspace entity absent on base; do not alias worktree/project IDs) | W17 | Missing entry point or backend contract; see dependencies below. | W; options.background ? workspaceWithBackgroundSchema : workspaceSchema; 0/1 | `packages/cli/src/commands/workspace/index.ts:42` |
| `workspace rename <workspace-id> [title]` | `--reset`: Clear the title and revert to the branch or directory name | none (workspace entity absent on base; do not alias worktree/project IDs) | W17 | Missing entry point or backend contract; see dependencies below. | W; workspaceRenameSchema; 0/1 | `packages/cli/src/commands/workspace/index.ts:49` |
| `workspace archive <workspace-id>` | — | none (workspace entity absent on base; do not alias worktree/project IDs) | W17 | Missing entry point or backend contract; see dependencies below. | W; workspaceArchiveSchema; 0/1 | `packages/cli/src/commands/workspace/index.ts:61` |
| `worktree ls ` (hidden) | `--cwd <path>`: Repository directory (default: current) | worktree list [--project PROJECT] | W17 | Different argument/output contract; not full parity. | W; worktreeLsSchema; 0/1 | `packages/cli/src/commands/worktree/index.ts:11` |
| `worktree create ` (hidden) | `--mode <mode>`: Creation mode: branch-off, checkout-branch, or checkout-pr<br>`--new-branch <name>`: New branch name (--mode branch-off)<br>`--base <ref>`: Base ref for new branch (--mode branch-off, defaults to repo default)<br>`--branch <name>`: Existing branch to check out (--mode checkout-branch)<br>`--pr-number <n>`: Pull request number (--mode checkout-pr)<br>`--cwd <path>`: Repository directory (default: current) | worktree new PROJECT | W17 | Different argument/output contract; not full parity. | W; createSchema; 0/1 | `packages/cli/src/commands/worktree/index.ts:18` |
| `worktree archive <name>` (hidden) | `--cwd <path>`: Repository directory (default: current) | worktree remove ID | W17 | Different argument/output contract; not full parity. | W; archiveSchema; 0/1 | `packages/cli/src/commands/worktree/index.ts:33` |
| `script ls ` | `--workspace <workspace-id>`: Workspace ID (required when a directory has multiple workspaces)<br>`--cwd <path>`: Workspace directory (default: current directory) | script list (different selectors/status) | workspace | Owned elsewhere / excluded; W17 does not modify its handler. | W; workspaceScriptSchema; 0/1 | `packages/cli/src/commands/script/index.ts:20` |
| `script start <name>` | `--workspace <workspace-id>`: Workspace ID (required when a directory has multiple workspaces)<br>`--cwd <path>`: Workspace directory (default: current directory) | script run <name> (different selectors/status) | workspace | Owned elsewhere / excluded; W17 does not modify its handler. | W; workspaceScriptSchema; 0/1 | `packages/cli/src/commands/script/index.ts:26` |
| `script stop <name>` | `--workspace <workspace-id>`: Workspace ID (required when a directory has multiple workspaces)<br>`--cwd <path>`: Workspace directory (default: current directory) | script stop <name> (different selectors/status) | workspace | Owned elsewhere / excluded; W17 does not modify its handler. | W; workspaceScriptSchema; 0/1 | `packages/cli/src/commands/script/index.ts:32` |
| `terminal ls ` | `new Option("--all", "List terminals across all workspaces").conflicts(["cwd", "workspace"])`<br>`--workspace <id>`: Workspace ID<br>`--cwd <path>`: Working directory | shell list | W17 | Different argument/output contract; not full parity. | W; terminalSchema; 0/1 | `packages/cli/src/commands/terminal/index.ts:17` |
| `terminal create ` | `--workspace <id>`: Workspace ID<br>`--cwd <path>`: Working directory<br>`--name <name>`: Terminal name | shell --session ID|--project PROJECT --detached | W17 | Different argument/output contract; not full parity. | W; terminalSchema; 0/1 | `packages/cli/src/commands/terminal/index.ts:28` |
| `terminal kill <terminal-id>` | — | shell kill ID | W17 | Different argument/output contract; not full parity. | W; terminalKillSchema; 0/1 | `packages/cli/src/commands/terminal/index.ts:37` |
| `terminal capture <terminal-id>` | `--start <n>`: Capture start line<br>`--end <n>`: Capture end line<br>`-S, --scrollback`: Capture from the beginning of scrollback<br>`--ansi`: Preserve ANSI escape codes | none (event bytes do not implement a screen capture) | W17 | Missing entry point or backend contract; see dependencies below. | R: captured text or --json result; failure 1 | `packages/cli/src/commands/terminal/index.ts:44` |
| `terminal send-keys <terminal-id> <keys...>` | `-l, --literal`: Send raw keys without interpreting special tokens | none (terminal.send exists) | W17 | Missing entry point or backend contract; see dependencies below. | R: silent by default; --json {terminalId,keysSent}; failure 1 | `packages/cli/src/commands/terminal/index.ts:56` |
| `schedule create <prompt>` | `--every <duration>`: Cron-compatible cadence preset (for example: 5m, 1h)<br>`--cron <expr>`: Cron cadence expression<br>`--timezone <iana>`: IANA time zone for cron cadence (default: UTC)<br>`--name <name>`: Optional schedule name<br>`new Option("--target <target>", "Legacy schedule target").hideHelp()`<br>`--provider <provider>`: Agent provider, or provider/model (e.g. codex or codex/gpt-5.4)<br>`--mode <mode>`: Provider-specific mode (e.g. claude bypassPermissions, opencode build)<br>`--thinking <id>`: Thinking option ID for new-agent runs<br>`--cwd <path>`: Working directory (default: current; required with --host)<br>`--run-now`: Fire one immediate run on creation<br>`--max-runs <n>`: Maximum number of runs<br>`--expires-in <duration>`: Time to live for the schedule | schedule create (different flags/defaults/output) | W17 | Different argument/output contract; not full parity. | W; scheduleSchema; 0/1 | `packages/cli/src/commands/schedule/index.ts:17` |
| `schedule ls ` | — | schedule list (different flags/defaults/output) | W17 | Different argument/output contract; not full parity. | W; scheduleSchema; 0/1 | `packages/cli/src/commands/schedule/index.ts:42` |
| `schedule inspect <id>` | — | schedule show ID (different flags/defaults/output) | W17 | Different argument/output contract; not full parity. | W; createScheduleInspectSchema(payload.schedule); 0/1 | `packages/cli/src/commands/schedule/index.ts:46` |
| `schedule logs <id>` | — | schedule runs ID (different flags/defaults/output) | W17 | Different argument/output contract; not full parity. | W; scheduleLogSchema; 0/1 | `packages/cli/src/commands/schedule/index.ts:50` |
| `schedule pause <id>` | — | schedule pause (different flags/defaults/output) | W17 | Different argument/output contract; not full parity. | W; scheduleSchema; 0/1 | `packages/cli/src/commands/schedule/index.ts:57` |
| `schedule resume <id>` | — | schedule resume (different flags/defaults/output) | W17 | Different argument/output contract; not full parity. | W; scheduleSchema; 0/1 | `packages/cli/src/commands/schedule/index.ts:61` |
| `schedule delete <id>` | — | schedule delete (different flags/defaults/output) | W17 | Different argument/output contract; not full parity. | W; scheduleDeleteSchema; 0/1 | `packages/cli/src/commands/schedule/index.ts:68` |
| `schedule run-once <id>` | — | schedule run ID (different flags/defaults/output) | W17 | Different argument/output contract; not full parity. | W; scheduleSchema; 0/1 | `packages/cli/src/commands/schedule/index.ts:72` |
| `schedule update <id>` | `--every <duration>`: Cron-compatible cadence preset (for example: 5m, 1h)<br>`--cron <expr>`: Switch to cron cadence expression<br>`--timezone <iana>`: IANA time zone for cron cadence (requires --cron)<br>`--name <name>`: Rename the schedule (empty string clears the name)<br>`--prompt <text>`: Replace the schedule prompt<br>`--provider <provider>`: New agent provider, or provider/model (only for new-agent target)<br>`--model <model>`: New agent model (only for new-agent target)<br>`--mode <mode>`: New agent provider mode (only for new-agent target)<br>`--cwd <path>`: New working directory (only for new-agent target)<br>`--max-runs <n>`: Set or change maximum number of runs<br>`--no-max-runs`: Clear the max-runs limit<br>`--expires-in <duration>`: Set or change time to live for the schedule<br>`--no-expires-in`: Clear the expiration | schedule edit ID (different flags/defaults/output) | W17 | Different argument/output contract; not full parity. | W; createScheduleInspectSchema(payload.schedule); 0/1 | `packages/cli/src/commands/schedule/index.ts:79` |
| `heartbeat create <prompt>` | `--cron <expr>` **required**: Five-field cron cadence<br>`--timezone <iana>`: IANA time zone<br>`--name <name>`: Heartbeat name<br>`--max-runs <n>`: Maximum number of runs<br>`--expires-in <duration>`: Time to live | schedule create --session CALLER (no heartbeat ownership wrapper) | W17 | Different argument/output contract; not full parity. | W; scheduleSchema; 0/1 | `packages/cli/src/commands/heartbeat/index.ts:157` |
| `heartbeat update <id>` | `--cron <expr>` **required**: Five-field cron cadence<br>`--timezone <iana>`: IANA time zone | schedule update --session CALLER (no heartbeat ownership wrapper) | W17 | Different argument/output contract; not full parity. | W; scheduleSchema; 0/1 | `packages/cli/src/commands/heartbeat/index.ts:168` |
| `heartbeat delete <id>` | — | schedule delete --session CALLER (no heartbeat ownership wrapper) | W17 | Different argument/output contract; not full parity. | W; heartbeatDeleteSchema; 0/1 | `packages/cli/src/commands/heartbeat/index.ts:176` |
| `permit ls ` | — | inbox list / session.conversation (permission options differ; no acknowledgement call) | W4 permissions CLI | Owned elsewhere; W17 supplies the common output seam. | W; permitLsSchema; 0/1 | `packages/cli/src/commands/permit/index.ts:11` |
| `permit allow <agent> [req_id]` | `--all`: Allow all pending permissions for this agent<br>`--input <json>`: Modified input parameters (JSON) | session answer ID REQUEST OPTION (permission options differ; no acknowledgement call) | W4 permissions CLI | Owned elsewhere; W17 supplies the common output seam. | W: PermissionResponseItem[] (requestId,agentId,agentShortId,name,result); 0/1 | `packages/cli/src/commands/permit/index.ts:15` |
| `permit deny <agent> [req_id]` | `--all`: Deny all pending permissions for this agent<br>`--message <msg>`: Denial reason message<br>`--interrupt`: Stop agent after denial | session answer ID REQUEST --decline (permission options differ; no acknowledgement call) | W4 permissions CLI | Owned elsewhere; W17 supplies the common output seam. | W: PermissionResponseItem[] (requestId,agentId,agentShortId,name,result); 0/1 | `packages/cli/src/commands/permit/index.ts:25` |
| `provider ls ` | — | agents / harnesses / profiles (different projection) | W17 | Different argument/output contract; not full parity. | W; providerLsSchema; 0/1 | `packages/cli/src/commands/provider/index.ts:11` |
| `provider models <provider>` | `--thinking`: Include thinking option IDs for each model | harness.options (client call only) | W17 | Different argument/output contract; not full parity. | W; options.thinking ? providerModelsWithThinkingSchema : providerModelsSchema; 0/1 | `packages/cli/src/commands/provider/index.ts:15` |
| `provider diagnostic <provider>` | — | agents status PROVIDER (readiness only) | W17 | Different argument/output contract; not full parity. | W; providerDiagnosticSchema; 0/1 | `packages/cli/src/commands/provider/index.ts:23` |
| `usage ls ` | `--refresh`: Refresh usage instead of using the host cache<br>`--agent <id>`: Show account quota associated with this agent (not agent token consumption) | usage account/session (quota limits) or usage machine (token use); report-ID API absent | W17 | Different argument/output contract; not full parity. | W: UsageReportEntry[] via usageSchema; table/JSON/YAML; 0/1 | `packages/cli/src/commands/usage/index.ts:80` |
| `usage inspect <report-id>` | `--refresh`: Refresh the selected report instead of using the host cache | usage account/session (quota limits) or usage machine (token use); report-ID API absent | W17 | Different argument/output contract; not full parity. | W; usageSchema; 0/1 | `packages/cli/src/commands/usage/index.ts:83` |
| `usage ` | `--refresh`: Refresh usage instead of using the host cache<br>`--agent <id>`: Show account quota associated with this agent (not agent token consumption) | usage account/session (quota limits) or usage machine (token use); report-ID API absent | W17 | Different argument/output contract; not full parity. | W: UsageReportEntry[] via usageSchema; table/JSON/YAML; 0/1 | `packages/cli/src/commands/usage/index.ts:79` |
| `plugin init <directory>` | `--id <id>`: Manifest plugin ID (defaults to the directory name) | none | W16 plugins | Owned elsewhere / excluded; W17 does not modify its handler. | W; scaffoldSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:212` |
| `plugin ls [id]` | — | none | W16 plugins | Owned elsewhere / excluded; W17 does not modify its handler. | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:219` |
| `plugin status [id]` (hidden) | — | none | W16 plugins | Owned elsewhere / excluded; W17 does not modify its handler. | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:222` |
| `plugin logs <id>` | — | none | W16 plugins | Owned elsewhere / excluded; W17 does not modify its handler. | W; pluginLogsSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:225` |
| `plugin install <source>` | `--id <id>`: Runtime plugin ID (defaults to paseo-plugin.json id)<br>`--ref <ref>`: Git branch, tag, or commit<br>`--path <path>`: Legacy form of the :plugin/path source suffix | none | W16 plugins | Owned elsewhere / excluded; W17 does not modify its handler. | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:228` |
| `plugin update [id]` | `--all`: Review all configured plugins<br>`--check`: Show available updates without installing<br>`--yes`: Apply displayed updates without asking<br>`--ref <ref>`: Apply a Git branch, tag, or commit without asking<br>`--version <version>`: Apply an npm version, tag, or range without asking | none | W16 plugins | Owned elsewhere / excluded; W17 does not modify its handler. | W pluginUpdateSchema; 1 if any outcome=error, otherwise 0 | `packages/cli/src/commands/plugin/index.ts:243` |
| `plugin reload <id>` | — | none | W16 plugins | Owned elsewhere / excluded; W17 does not modify its handler. | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:255` |
| `plugin enable <id>` | — | none | W16 plugins | Owned elsewhere / excluded; W17 does not modify its handler. | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:255` |
| `plugin disable <id>` | — | none | W16 plugins | Owned elsewhere / excluded; W17 does not modify its handler. | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:255` |
| `plugin remove <id>` | — | none | W16 plugins | Owned elsewhere / excluded; W17 does not modify its handler. | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:263` |
| `plugin add <source>` (alias) | `--id <id>`: Runtime plugin ID (defaults to paseo-plugin.json id)<br>`--ref <ref>`: Git branch, tag, or commit<br>`--path <path>`: Legacy form of the :plugin/path source suffix | none | W16 plugins | Owned elsewhere / excluded; W17 does not modify its handler. | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:228` |
| `hub connect [origin]` | `--api-key <secret>`: Organization API key<br>`--permission <permission...>`: Grant daemon permission during connection | none (Paseo Hub excluded) | out of scope by plan §4b | Owned elsewhere / excluded; W17 does not modify its handler. | W: HubRow[] via hubStatusResult; table or JSON/YAML; 0/1 | `packages/cli/src/commands/hub/connect.ts:61` |
| `hub deploy ` | `-p, --project <slug>`: Deploy a legacy bundle to this project slug<br>`--hub <origin>`: Paseo Hub origin<br>`--api-key <secret>`: Organization API key<br>`--dry-run`: Validate without installing or activating | none (Paseo Hub excluded) | out of scope by plan §4b | Owned elsewhere / excluded; W17 does not modify its handler. | W; triggerValidationSchema, triggerResultSchema, validationSchema, resultSchema; 0/1 | `packages/cli/src/commands/hub/deploy.ts:249` |
| `hub disconnect ` | `--force`: Remove local authority without notifying the Hub | none (Paseo Hub excluded) | out of scope by plan §4b | Owned elsewhere / excluded; W17 does not modify its handler. | W: HubRow[] plus possible warning; table or JSON/YAML; 0/1 | `packages/cli/src/commands/hub/disconnect.ts:43` |
| `hub export [directory]` | `--hub <origin>`: Paseo Hub origin<br>`--api-key <secret>`: Organization API key<br>`--force`: Replace trigger files with different contents | none (Paseo Hub excluded) | out of scope by plan §4b | Owned elsewhere / excluded; W17 does not modify its handler. | W; ; 0/1 | `packages/cli/src/commands/hub/export.ts:101` |
| `hub status ` | — | none (Paseo Hub excluded) | out of scope by plan §4b | Owned elsewhere / excluded; W17 does not modify its handler. | W: HubRow[] via hubStatusResult; table or JSON/YAML; 0/1 | `packages/cli/src/commands/hub/index.ts:73` |
| `hub init ` | — | none (Paseo Hub excluded) | out of scope by plan §4b | Owned elsewhere / excluded; W17 does not modify its handler. | R: Clack prompts/progress; cancel 0, failure exitCode=1 | `packages/cli/src/commands/hub/init.ts:91` |
| `hub login [origin]` | — | none (Paseo Hub excluded) | out of scope by plan §4b | Owned elsewhere / excluded; W17 does not modify its handler. | W; ; 0/1 | `packages/cli/src/commands/hub/login.ts:59` |
| `hub logout ` | `--disconnect-daemon`: Also disconnect a daemon related to the same Hub<br>`--force`: Remove daemon authority without notifying the Hub | none (Paseo Hub excluded) | out of scope by plan §4b | Owned elsewhere / excluded; W17 does not modify its handler. | W; ; 0/1 | `packages/cli/src/commands/hub/logout.ts:93` |
| `hub projects ` | `--hub <origin>`: Paseo Hub origin<br>`--api-key <secret>`: Organization API key | none (Paseo Hub excluded) | out of scope by plan §4b | Owned elsewhere / excluded; W17 does not modify its handler. | W; ; 0/1 | `packages/cli/src/commands/hub/projects.ts:77` |
| `hub permissions list ` | — | none (Paseo Hub excluded) | out of scope by plan §4b | Owned elsewhere / excluded; W17 does not modify its handler. | W; ; 0/1 | `packages/cli/src/commands/hub/permissions.ts:82` |
| `hub permissions grant <permission>` | — | none (Paseo Hub excluded) | out of scope by plan §4b | Owned elsewhere / excluded; W17 does not modify its handler. | W; ; 0/1 | `packages/cli/src/commands/hub/permissions.ts:90` |
| `hub permissions revoke <permission>` | — | none (Paseo Hub excluded) | out of scope by plan §4b | Owned elsewhere / excluded; W17 does not modify its handler. | W; ; 0/1 | `packages/cli/src/commands/hub/permissions.ts:90` |

## Node dependencies (one report to the coordinator)

- Workspace entities and create/list/title/archive/setup with background visibility and ownership; `project`/`worktree` IDs are not workspace IDs. This blocks workspace paths and full run/import/terminal placement. A reviewed workspace stack exists separately, but it is not part of the announced node base.
- Project title/reset and cascading delete semantics. `project.remove` alone does not promise Paseo workspace archival.
- Subagent parentage/detach; none of the base calls removes a parent relation. No fake detach success.
- Launch environment and provider creation configuration/atomic apply; `SessionSpec` cannot carry all Paseo --env/--mode/--thinking creation settings. Output-schema execution needs a separate bounded schema validator; it is not an RPC alias.
- Provider snapshot/model-thinking/diagnostic projection: existing agent readiness and harness option calls cover only parts, not the complete Paseo diagnostic contract.
- Terminal screen capture (line offsets, ANSI/scrollback semantics): reading event bytes is not a substitute for rendered screen state. `terminal.send` is available for key input.
- Daemon-owned host registrations (metadata, enabled/disabled, identity replacement) differ from the local node book. Require a contract/ruling before implementing them.
- Quota report IDs/discovery differ from `usage.limits` account/session scopes; do not reinterpret report IDs as harness/profile IDs.
- Permission modified-input and denial-message/acknowledgement semantics are absent from `AnswerPermission` (an option or cancelled only). Basic option-based replies can reuse existing calls; richer flags must refuse before sending anything.
- Hub paths are explicitly outside this plan, not pending implementations. Plugin CLI belongs to W16. Lifecycle logs/labels/reload, daemon, and scripts remain with their owners.

No new calls are registered before Quality B; no provisional method numbers are invented.

## Verification and handoff

Use fake-node process tests for stdout shape, error/exit contracts, scoped listing, interrupt
versus kill, archive protection, option validation before mutations, permission selection,
finite waits and broken pipes. Source-only audit is distinct from a passing narrow check.
Per-commit checks use the remote short lane; the package gets one root whole gate through
A12, no native gate/captures. Reviewer V3 and the node integrator receive the final range,
run ID/steps/totals, works/blocked/unverified.

## Node host registry and capture decision (coordinator ruling)

The final owner table assigns W17 the host registry and node-side terminal capture.
W5 owns the report/source DTOs; W17 wires their CLI only. W4 owns CLI permit; W2
owns detach. W11 owns workspace identity, W22 provider creation, lifecycle logs/labels/reload,
nodectl daemon, workspace scripts, W16 plugins and W19b desktop installation remain separate.

[Decision Log]
- Purpose and intent: persist user-visible host registrations without creating a second transport or credential authority; capture rendered terminal lines rather than log bytes.
- Existing implementation and constraints: fleet nodes, paired devices/links and relay routes already establish trust and connectivity; `nodes.toml` is client-local. Terminal actors own output but their bounded event ring is not a screen and cannot reconstruct pruned output or historical resize geometry reliably.
- Alternatives considered: duplicate host credentials in a registry; treat any URL as trusted; replay raw bytes per capture; reuse a native-only grid in the node; implement another ANSI parser by hand.
- Chosen approach: host entries reference existing paired/fleet destinations and retain source ownership, metadata, enabled state and explicit identity-replacement checks. Use one owner-only atomic host registry file, reusing store file-replacement helpers (no SQLite migration). A bounded terminal screen owned beside the terminal output actor provides snapshots; evaluate the established `vt100` parser within `vessel-session`, with no parser types crossing its API.
- Why: credentials stay in existing authorities; file persistence avoids a competing store migration for a small bounded directory; persistent actor screen state stays correct after event-ring pruning and resize. Native UI parser remains C2's responsibility.
- Advantages, costs and effects: registry disable/removal never revokes a paired device or forgets a fleet node. Direct/SSH/pair offers must resolve to a trusted destination first; the initial reference-only contract must explicitly refuse unresolved targets. A screen engine adds bounded memory per terminal and one external parser dependency owned solely by `vessel-session` if adopted.

Reference host bounds: `packages/protocol/src/hosts.ts:3-36` (label/key 256,
metadata value 4096, connection text 16384). The initial reference-only form accepts
`fleet:<NodeId>` or `paired:<NodeId>`; unresolved address forms remain a stated pending
connectivity integration, not a successful registration. Source ownership derives from the
authenticated principal, never a caller-provided source ID.
Reference capture: `packages/server/src/terminal/terminal-capture.ts:14-59`: zero-based,
inclusive start/end, negative indices count from the end, clamped to available rows;
reversed ranges are empty. xterm translateToString omits style information even with
stripAnsi=false, so --ansi must not synthesize style that Paseo itself does not return.

Parser choice verified from the upstream `vt100` 0.18.1 crate source and manifest
(MIT, Rust 1.85): https://github.com/doy/vt100-rust and
https://docs.rs/vt100/0.18.1/vt100/ . `Parser` accepts split UTF-8/ANSI bytes and exposes
scrollback and line projections without exporting its types across our crate boundary.
Only three new lockfile packages were resolved: vt100, vte, unicode-width; no existing
locked dependency version was updated.

## Implemented/staged status over the baseline

- Added typed `agent` handlers plus root ls/inspect/wait/stop/delete/archive. Root run/send
  retain their original Vessel syntax; Paseo root run/send forms route through the agent grammar.
- Agent list uses current-directory scope by default, excludes archived, supports global/all,
  validated label filters and advertised thinking choices. The provider field currently uses
  Vessel's HarnessRef rather than W22's canonical ProviderId; model IDs and relative created labels follow the reference.
  Inspect projects the capitalized reference fields from typed session/log facts; unavailable source facts remain null.
- Stop reconstructs the logged turn and cancels that exact turn; it does not call process kill.
  Live archive without --force refuses until an atomic archive-if-idle contract is available;
  the current mark call unconditionally stops a live session and is unsafe for that promise.
- Agent run/import support existing structured-session creation; missing workspace/provider
  creation fields and schema-output/image staging refuse before creation. No env value is logged.
- Wait reconstructs the subscription snapshot, follows the stream and returns timeout/permission/
  error as data with exit 0. The default is a named 24-hour safety ceiling (strict finite-wait
  rule), a stated difference from Paseo's unbounded default. No recent-activity preview yet.
- Global formats apply to the new typed handlers. Legacy command handlers have not yet all been
  converted; their global-format behavior remains a tracked gap, not complete parity.
  Single-value human rendering currently uses pretty structured text; list rendering is a table.
- Terminal ls/kill/send-keys use existing calls. Name lookup follows exact ID, unique ID prefix,
  exact case-insensitive name, then unique partial name. Key expansion follows only Paseo's
  documented tokens, and keysSent counts UTF-16 units. Default ls filters terminal folder until
  real workspace identity lands; create refuses rather than substituting project/session IDs.
- Terminal capture has typed range/options/results and an actor-owned vt100 screen, independent
  of the event ring. Shell/script actors answer bounded snapshot requests; final snapshots remain
  readable after process exit. Node handler reuses terminal_for ownership. RPC registration remains
  pending, as does the client/CLI capture call.
- Capture memory: 10,000 scrollback lines by default, 4,000,000 cells per screen maximum, 1 MiB
  per response, 8 queued capture/resize requests, 5-second response deadline. Very wide initial
  geometry lowers retained rows; a later resize that exceeds existing capacity's cell budget
  explicitly disables capture rather than pretending discarded screen state can be recovered.
- Host types, bounded text/metadata and a source-owned actor/file store are staged. Tests cover
  identity replacement, stale updates, other-source refusal, canceled reply receiver, persisted
  state, private modes and symlink rejection. Actual host RPC registration/routing remains pending.

## Structural extraction before pending registration

The unchanged Request/UnexpectedReply contracts move from rpc.rs to rpc/contract.rs;
node model client/listener setup moves unchanged to the existing node/setup.rs. This behavior-preserving
refactor keeps the method table and node assembly within their existing shrinking budgets
without raising either record. Public imports and tests retain their original meaning.

Shared-seam delivery is the clean pair `1c8d0b93`, `ed3a3323` directly on Quality B
`29eebb8b` in branch `backend/m12-wp-cli-seams`. An earlier combined local extraction
commit is draft history only; integration uses that clean pair. The feature source aligns
with it. The coordinator explicitly approved adding only host_registry.add/update/remove
to the existing exact Admin-method assertion; all prior expectations remain unchanged.

Quality B `29eebb8b` was announced and merged before registration. Five rows now use
`(pending CLI_PARITY_PROTOCOL)` in the single method table: host_registry.list/add/update/remove
and terminal.capture. No number is invented. The client has typed wrappers; CLI performs
an additional unassigned-feature refusal before connecting. Node and client admission
refuse None requirements, including wrapped/old-peer paths. Host administration is local-only
(no fleet forwarding), source-owned, and denied to agents/delegated callers. Screen capture
follows terminal ownership and may forward like terminal.read. Browser policy explicitly
allows the unary requests subject to the node's normal admission/scope checks.

Root `run PROMPT --provider PROVIDER` and flagged `send ID PROMPT` decode into the
same agent grammar. Legacy `run HARNESS` and hex-ID-first `send ID TEXT` remain available;
a prompt consisting only of hex/dashes is ambiguous in the latter spelling, so use
`agent send` or `send ID --prompt TEXT` to select the Paseo form explicitly. Root selectors'
values and arguments after `--` are excluded from compatibility grammar selection.
Heartbeat create/update/delete reuse the existing schedule client, require VESSEL_AGENT_SESSION,
and verify the selected schedule targets that caller before update/deletion. Existing schedule
and project/worktree list paths gained familiar aliases; unchanged legacy flag/output differences
remain in the inventory and are not counted as full parity.

Host label/metadata per-field lengths use UTF-16 code units, matching Zod string lengths;
aggregate metadata is capped at 16 KiB and 256 entries as a stated resource admission floor.
Control characters in host annotations are a stated CLI/terminal-safety refusal. Capture line
indices accept signed 64-bit integers and reject trailing nonnumeric text (Paseo parseInt
would accept it); this is a deliberate strict-input difference.

## Capture engine handoff and first Quality B check

Coordinator ADR0095 ruling assigns the sole pure root `vessel-terminal-screen` engine to C2.
The staged vt100 adapter and tests at c0b9d71c are measurement input only; the final node
actor must consume C2's shared core and retain only channel/PTY ordering glue. Native view,
restore/query and CLI capture must share that one interpretation. No engine has passed the
comparison yet. This supersedes the provisional direct session dependency described above.

The first Quality B narrow check on c0b9d71c failed before downstream compilation: the
CaptureLineCount accessor accidentally declared Option despite its total u32 value, and two
older-peer reply projections lacked the five new pending variants. Correcting these changes
no wire shape; pending admission still refuses the calls until integration assigns a version.

## Existing project adapters and pending placement grammar

[Decision Log]
- Purpose: parse the complete workspace creation forms without confusing their identity with a legacy worktree.
- Existing assets: AddProject/ListProjects/CloneProject and the clone revision long-poll already implement project registration and Git authority.
- Alternatives: a second registry or local Git subprocess would duplicate node authority; neither is needed.
- Choice: reuse those typed calls for project create/list and root clone, retain explicit pending refusals for W11's rename/reset/cascade and workspace operations.
- Why: consumers can inspect the real parser while missing placement semantics fail before effects. Provider/model shorthand decodes once into existing supported driver/model inputs; full provider identity remains W22's contract.
- Impact: GitHub owner/repo shorthand requires an explicit transport; clone completion has a ten-minute ceiling and preserves the project ID on failure.

Root import routes to the same typed agent handler. Run now accepts every workspace source flag
and legacy background/detach alias; placement flags refuse before creation until W11's reviewed
API arrives. File and positional prompts share a 64 KiB admission floor; file reads have a
30-second deadline. These are stated resource limits. List reconstructs OptionsChanged from
the same history snapshot as turn state, so listing ended agents does not call the live-only
session.options method.

## Schedule invocation and output adapter

Paseo reference: commands/schedule/index.ts:17-105, shared.ts:140-427 and protocol/src/schedule/cadence.ts.
The existing revision-checked schedule service owns execution, persistence and limits; the CLI
normalizes positional prompt/provider-model/cwd/timezone/expiry flags into its existing definition.
Calendar-compatible --every values compile to five-field cron in UTC; 7m, 5h, 30s and 2d refuse
rather than changing cadence at calendar boundaries. This changes the old create/edit elapsed
interval interpretation; schedule preview retains its explicitly documented elapsed behavior.
Explicit remote creation requires cwd; updates preserve the old cwd unless a new one is given.
Empty --name and paired no-max-runs/no-expires-in flags clear those fields in one revision-checked
update. Provider mode/thinking and the hidden legacy --target refuse before connecting until
their reviewed contracts exist. Run-now reuses the existing call after creation; a failed run
reports the created schedule ID so retrying cannot silently duplicate it. It is not atomic
creation+run, unlike a future node-level create option.

List/create/pause/resume/run-once share a typed schedule row with all global formats. Inspect
and logs currently serialize native ScheduleInfo/ScheduleRun rather than pretending node data
contains Paseo's run output/error report or target discriminator. Those projection differences
remain explicit. No scheduler/node RPC is added by this adapter.

## Pending provider and usage consumers

Provider ls/models/diagnostic and usage default/ls/inspect now have dedicated clap grammar.
Provider selectors are CLI-local text only; no provisional ProviderId/ReportId wire DTO is
invented. Until W22/W5 deliver reviewed contracts they return Unsupported before connecting.
In particular, manifest availability, generic harness options, account/profile identities and
token consumption are not mislabeled as provider snapshots or quota reports. Usage parent
--agent/--refresh options conflict with subcommands, so usage --agent ID inspect REPORT cannot
silently choose one scope. Legacy explicit usage scopes remain available with their existing
output. The full reference table above is the baseline; these status sections describe staged
work and do not claim 117 paths have passed parity checks.

The second narrow check (70385962) compiled the product protocol and reached downstream
crates, then stopped on two denied integer-division lints in the UTF-16 boundary test.
The fixture now uses checked arithmetic. This is compilation evidence only; unit tests
and package review have not passed yet.

Hooks CLI grammar is staged against W7 ADR0089 terminal-activity attribution: a missing
VESSEL_TERMINAL_ID / VESSEL_ACTIVITY_TOKEN / VESSEL_TERMINAL_ACTIVITY_URL is a silent no-op,
with no stdin read or connection. A configured target currently returns Unsupported before
reading a token value or payload. W7 confirmed it owns hook ingestion in addition to normal
authenticated terminal.activity; no duplicate endpoint is introduced here. This configured
unsupported exit1 is a stated temporary difference from Paseo's best-effort hook exit0.

The schedule process test now checks the requested cron/UTC and typed row/delete receipts
rather than asserting the superseded elapsed-interval text. It still creates, lists, runs,
reads the recorded Asked trigger, updates, keeps the existing session spec, pauses, deletes
and checks invalid timezone refusal against the fake-only node. Three further narrow lint
findings (borrowed registry error, optional boot admission and a test Ack qualification)
were corrected without changing registry behavior. Input tests cover bounded UTF-8 files
and logged option snapshots after process exit.

Hidden worktree create/archive and worktree ls --cwd now parse their reference flags.
Creation mode conflicts are checked before connecting. They remain explicit pending
W11 registry/ownership consumers: a legacy WorktreeId is not substituted for a WorkspaceId,
and removing an owned worktree cannot bypass workspace reference archival. Existing
project-ID based worktree list/new/remove remain available with their native contract.

Registry source review found that metadata-only update/disable unnecessarily queried the
destination trust provider. Those edits now bypass destination discovery when no replacement
ref is supplied, so an offline or removed destination can still be disabled/annotated. A
replacement still requires the existing authority proof, explicit identity-change consent
and revision comparison. The actor test disables with an empty trusted-destination set.

Agent list display now appends the latest advertised model (then the pinned/launch model)
and uses the reference relative-time boundaries, including future timestamps as just now.
This is presentation only: it does not invent provider identity or change model selection.

CLI-only narrow check 55b342d8 reached the full executable/tests and reported only lints,
including long command dispatch functions and one oversized schedule future. Terminal capture,
key-send effects, preview cadence and usage report formatting now have separate typed helpers;
the schedule future is boxed at dispatch. No lint suppression or file budget was raised.

Delete now emits deletedCount/agentIds only after each node deletion acknowledgement.
Archive emits agentId/status=archived/archivedAt from the acknowledged marks and refuses an
already archived session. Missing archivedAt is an invalid node receipt, not a fabricated
local timestamp. Live idle archival without force remains pending atomic node support.

The first local TS generator run completed compilation but found a global declaration-name
collision with forge::HostStatus. The registry enum keeps its Rust namespace and wire values
and exports the distinct TypeScript name RegisteredHostStatus. No existing forge declaration
or RPC serialization shape changes. Generator outputs remain uncommitted until rerun succeeds.

Mode requires either a choice or --list. Available modes project only actual Mode select
values, with ID/label/description rows. A successful set returns agentId/mode only after
the acknowledgement reports that choice selected. Internal archive/mode intent uses enums
rather than carrying clap booleans between handler modules.

## Image send over the existing attachment protocol

[Decision Log]
- Purpose: implement --image for an existing agent without inventing inline RPCs or a second uploader.
- Existing assets: conversation::FileInput already reads bounded chunks and send::upload already uses attachment.begin/write/finish.
- Alternatives: read/base64 the whole image in the CLI, or implement a second upload loop; both duplicate the existing attachment protocol and retain more memory.
- Choice: keep file input private to conversation, expose an opaque Images batch, and extract upload_file with unchanged chunk behavior. Pre-open/validate every image, check the node's actual attachment limits and advertised image ability, then upload and send references.
- Why: invalid local files fail before staging; provider ability and node limits fail before the first upload; all transports continue using the same client calls.
- Impact: only png/jpg/jpeg/gif/webp extensions are accepted (Paseo guesses JPEG for unknown extensions); default attachment admission is 16 images, 10 MiB each and 25 MiB inline. Node limits may tighten these. Staging has a two-minute deadline; unfinished uploads use the existing idle cleanup because no abandon call exists. Initial run images still await atomic creation integration.

CLI unit run 6910fd61 passed 116/0/0. Process run 3e23c196 passed two tests and failed
one fixture assertion: fake ACP's generic hello prompt asks permission, while the test
expected idle. The idle fixture now uses the explicitly idle plain command; the observed
permission result was correct. A new process case checks actual delivered image bytes.

The image batch stays in conversation alongside FileInput/upload_file; raw file byte counts
are not exposed to the command orchestration layer. W16 reserves root plugin registration
and owns args::plugin / plugin::run over the same Presentation/output helper surface.

## Current scoped verification

| Check | Exact source | Result | Evidence |
|---|---|---|---|
| Shared seam clippy, proto/node | ed3a3323 | pass | /tmp/work/wp-cli/seams-check-recovery |
| CLI unit tests | 6910fd61 | 116 passed, 0 failed, 0 ignored | wp-cli-narrow-20261115T075605Z-6910fd61; /tmp/work/wp-cli/cli-unit-check |
| CLI process parity tests | 16428482 | 4 passed, 0 failed, 0 ignored | wp-cli-narrow-20261115T081105Z-16428482; /tmp/work/wp-cli/cli-process-check2 |
| Full affected-crate narrow clippy | 19205c28 | failed on one unused extraction import, corrected in 6910fd61 | /tmp/work/wp-cli/qualityb-check6 |
| Local TS generation | before1252d8dc | failed on the forge/registry declaration-name clash; corrected, rerun pending | /tmp/work/wp-cli/typescript-generation*.log |

These are scoped results, not a whole root gate or V3 package pass. All runtime checks use
fake ACP and isolated temporary nodes, under ubuntu UID/GID1000; no live user agent or
service configuration is touched. No permission/ownership setting was changed. Package
base3 6c782cb6 was merged as instructed (8467e96c); the exact approved e2e race fix c5da5305
was merged first, with no local edits to that test. The pure screen engine and the listed
external contracts still await their owners' reviewed consumer heads.

Inspect now serializes the reference capitalized field names and renders key/value rows.
It reuses the same bounded history snapshot for model/mode/thinking, last log timestamp and
pending permission IDs, including ended agents. LastUsage/Capabilities remain null because
Vessel's current snapshot does not supply those equivalent facts; unknown tools are named
unknown. Worktree is the actual Vessel WorktreeId, not an invented workspace ID or a Paseo
label alias. ParentAgentId remains null until W2's actual attachment contract is available;
Creator provenance is deliberately never used to infer current parentage.

Host registry scoped run e638bc46 passed 5/0/0 (wp-cli-narrow-20261115T081853Z-e638bc46,
/tmp/work/wp-cli/host-registry-tests). The second TS attempt overlapped the required base3
merge and compiled mixed old/new ScriptEntry APIs; current source matches base3's entry
API. That attempt is invalid generation evidence, and generation is rerun on stable source.
W22 confirmed output-schema belongs to atomic create+initial prompt/per-turn transport,
not durable SessionSpec configuration; W17 retains CLI semantic validation/retry ownership.

Mode setting resolves a value across all advertised options in its category and retains
the selected OptionId for acknowledgement validation. A duplicated value across settings
refuses before mutation; list uses logged options so ended sessions remain inspectable.
The pure selection regression includes a value present only in the second option, which
the earlier first-category lookup incorrectly refused.

## Node head40 integration

Merged announced ba45a6e3 after the scoped checks finished. Shared Request/UnexpectedReply
contracts and the existing boot helpers follow the head, including its preserved logging
target. W17's five pending rows move to the END of rpc/table.rs; no version/schema is
assigned. Existing Admin expectations remain unchanged, with only the three approved host
Admin names relocated to the table's new trailing order. W17 has no new project/worktree/
git/file method to add to WORKSPACE_FAMILIES_LATER. Head WorkspaceId/WorkspaceOperationId
and their tests are retained alongside HostRegistrationId. The smaller node.rs budget684
is retained rather than raised to691. The provisional screen adapter remains blocked on
C2 ADR0095 and is explicitly not mergeable as a second parser.

Remote named ts run dc7d55d71 completed generator4/0/0 and exported a one-file expected
tracked diff (overall step rc1 because generated declarations changed). Its manifest and
artifacts are /tmp/work/wp-cli/ts-export/artifacts/ts. Since head40 updates wire types too,
that older generated file will not overwrite the head's declarations; generation will be
repeated on the resolved merge. Verified local waiting PID3674024 was terminated before
submitting the remote ts job; no concurrent duplicate generator was started.


### Head40 generated wire and output boundary

The exact cd9bb4bf remote TS generator passed four tests. Its tracked-diff exit
was 1 because host/capture DTOs changed proto.ts, not because generation failed.
Applied only the regenerated proto.ts after verifying artifact commit and every
SHA256SUMS entry; bridge.ts and calls.ts were unchanged. Evidence:
`/tmp/work/wp-cli/ts-export-head40`.

[Decision Log]
- Purpose: let permit/plugin owners share one typed formatter without depending
  on W17 host/provider error variants or connection aliases.
- Existing asset: args::parity::output and parity::output already own the flags
  and Row/list/render_list/single surface. Reuse them instead of copying output.
- Alternatives: waiting for the full CLI package blocks independent consumers;
  moving formatting into a new crate adds an unnecessary dependency boundary.
- Choice: separate connection aliases and runtime failure classification into
  their own leaf modules. The existing output API and presentation precedence
  remain unchanged. No new RPC, store, transport or dependency is introduced.
- Impact: a standalone early shared slice can be reviewed independently; its
  consumer readiness still requires review. The full package is not yet gated.


### Bounded structured-output staging

Coordinator approved the single jsonschema Value edge, recorded in AGENTS.md and
ADR0001. Reused RawJson and RawValue, pinned cached jsonschema 0.60.2 (MIT), with
no default resolvers and offline compilation. All schema/response diagnostics
are content-free. Added cfg(test) semantic validation, internal/external ref and
size tests, raw/fenced response decoding, maxRetries2 orchestration and fake
caller failure/no-retry checks. This is source staging only pending remote checks
and W22's reviewed per-turn native contract; the product still refuses the flag
before creating an agent. It is not runtime output-schema parity.

Approved once-only local offline cargo metadata resolved 24 added packages;
a parsed lock comparison preserves all 448 existing package identities/checksums,
and every addition is in jsonschema's dependency closure. No compilation or lane
was used. This does not constitute adapter test evidence.


### Run/send result projections

Paseo commands/agent/run.ts95-116 and719-745 use a run receipt, with idle wait
mapped to completed; send.ts15-31,155-175,211-234 uses a send receipt. W17 now
projects typed create/prompt acknowledgements into those CLI field names, status
sets, table columns and ID-only quiet output, rather than exposing SessionInfo
or Delivered. Standalone wait still returns idle; timeout/permission/error remain
result data. A queued ack says queued without claiming Paseo's two-second timer.
Provider remains the actual HarnessRef until W22 exposes canonical ProviderId;
run title/cwd come from the creation acknowledgement, not a fabricated final
snapshot. Inspect now orders modes before permissions and shows [] for no
pending permissions, matching inspect.ts. Process expectations cover the changed
completed status; latest narrow execution remains owed.


### Shared output integration staging

Merged own early output source 872ea64 into the W17 development tree, preserving
its isolated immutable review/test head. This is internal source staging only;
no consumer-ready hash or integrator merge is claimed. Admission now precedes
connection aliases and connection work. Explicit global output is consumed only
by parity adapters (excluding raw hooks/attach), project parity paths, schedule
non-preview paths, configured models, and typed report preflight refusals. Other
legacy paths refuse instead of ignoring the request. Compare retains its local
JSON switch. Terminal capture/send-keys quiet output uses terminal IDs; their
plain default remains raw capture/silent send respectively. Full package gate,
corrected shared strict lint and targeted execution remain owed.


### Announced node gate42 merge

Merged announced d432ef0f into the W17 development tree after its immutable
early-output test was submitted from the separate source tree. Preserved both
terminal lifetime regressions and W17 ring-pruning capture regression, and both
architecture sections. ProcessLifetime observation is only a read-only carrier;
no additional socket/signal authority is inferred. SDK check_call is now present.
Lifecycle still owns the separately reserved legacy run_session preflight line;
W17 does not duplicate it. No owned AgentSetup literal required a new field;
base literals retain the incoming system_prompt field. Shrinking size allowances
use the new lower limits; no record was raised. This is not a latest-head gate.

Staged schema dialect correction: fixed draft-07 (Paseo Ajv8 default), typed
$schema declaration gate, and format annotations disabled (no ajv-formats in
reference). Added tuple-items/dialect/format fixtures; not executed yet. OpenAPI
nullable and other Ajv extensions remain an explicit runtime-promotion debt, not
a claim of complete validator equivalence.

Import now reuses the run-receipt schema (reference import.ts toImportResult),
reporting created from the acknowledged import without claiming a new turn. Mode
and thinking updates share exact option-ID/value acknowledgement verification; a
returned list that omits the requested selection is an invalid receipt, never
success. Added stale/wrong-setting acknowledgement fixtures. Update's full result
projection and provider notices remain pending; no latest execution claim.


### Atomic steer fallback and remote interruption

The existing session actor returns CannotSteer while a running driver lacks
steering (delivery.rs303); a client read then interrupt would race, and generic
Unsupported is not a typed no-mutation receipt. Coordinator assigned the atomic
SteerOrInterrupt policy and typed outcome to W23's prompt-receipt transport. W17
and W12b consume the reviewed contract; no local fallback, speculative retry or
new delivery enum is added here. Existing Unsupported remains an honest temporary
difference until that actor contract lands.

Exact shared output5305 targeted regression passed6/0/0. Its strict run101545
was cancelled143 while queued at10:35:42, with no cargo step. T/coordinator
confirmed an operator-approved VM resize/restart and no live old runners. Original
logs remain output-seam-clippy2; one same-source replacement is submitted in a
new output-seam-clippy3 folder. Cancellation is neither source failure nor PASS.
All future Cargo/rustc and compilation/checks are VM-only; no new local Cargo job
is permitted, including metadata (the one earlier metadata exception completed).


### Destination path interpretation

[Decision Log]
- Intent: never interpret a selected destination's project path using the CLI's
  cwd merely because its address is loopback (it may be a tunnel).
- Existing asset: book::Origin already distinguishes implicit local Default
  from Named/Project/User selection. Reuse this type; no new resolver or store.
- Choice: only implicit-local Default may supply/resolve the CLI cwd. Explicit
  selected targets require an absolute host path under the current node API.
- Tradeoff: relative remote paths are a typed refusal until W11's host-root
  registry contract can resolve them on the destination; no local guess.
- Validation drafted: actual CLI against a fake node selected through a loopback
  book default refuses missing/relative paths without adding a project, then
  succeeds with an explicit absolute host path. Latest execution remains owed.
Legacy native workspace helpers are not changed by this adapter correction.

Machine-readable runtime failures now reuse CallError::code() for both remote
and local SDK admission errors. UnassignedProtocol/NotInProtocol remain
Unsupported rather than being misclassified as Internal after the new SDK
preflight seam. Added a nested CLI-error classification regression; it does not
claim a frame was sent or that a pending call is available. Latest execution owed.


### Early output handoff closed, package still in progress

V3 scoped PASS at11:12 covers source5305/evidence-only5b3ca1d: targeted6/0/0
and strictClippy0. Passed head handed to integrator, W4 and W16. The integrator
took exactly5b (nothing from the W17 feature tree), staged30e1e1bb overc399e072,
and told consumers to merge its announced green head aftergate46 rather than
the development branch. Combined-head/consumer checks remain required.

The W17 development sourcecd1a12d7 was separately submitted for all-target strict
Clippy across CLI/proto/RPC/client/store/session/node/web, run
wp-cli-narrow-20261115T111143Z-cd1a12d7, /tmp/work/wp-cli/main-clippy-head42.
No result yet. Latest actor/CLI/schema/process tests, reviewed external contracts,
shared-core consumption and the final whole root gate remain owed.


### Host registry local-agent read policy — security ADR addendum (design reviewed; implementation pending checks)

Status: V3 row202 SOURCE PASSED / GATE OWED on the docs-only 2bea422d
security design, before implementation began. This is not a runtime or final
implementation review pass.
This section is W17's feature ADR (the initial package instructions allow an
ADR section in the plan). It supersedes only the earlier blanket Agent read
refusal; existing source-bound management and Admin writes remain unchanged.

[Decision Log]
- Purpose: Paseo MCP list_hosts serves all agents of its daemon, acting for the
  node owner. W18 needs the same inventory without inventing an Agent→Owner
  source mapping or copying a client book.
- Existing assets/constraints: HostRegistration persists destination references
  and display annotations; HostSource controls Owner/Device management.
  service/host_registry.rs currently rejects Agent/Delegated at source(). The
  existing HostDirectory/HostRegistration response exposes source/revision and
  must not be reused wholesale as a public agent response. All W17 calls remain
  behind CLI_PARITY_PROTOCOL=None; this registry is not a released writable API.
- Alternatives: inherit a root source by walking Creator/attachment ancestry
  (ambiguous foreign/deleted chains, unnecessary auth storage); treat Agent as
  Owner (incorrect authority); expose complete records (needless private data).
- Choice: an explicit node-wide authenticated Read projection for local agents.
  No source inheritance or parent/creator traversal. New read projection policy
  is independent of source ownership; it does not manufacture HostSource::Owner.
- Reason/tradeoff: intentionally share display inventory while leaving mutable
  management scoped. Agents learn only the approved public fields, never gain
  transport credentials or the ability to register/rebind/remove destinations.

**Proposed allowed fields (explicit projection; no flatten/full-record serde):**

| Field | Why shareable / source |
| --- | --- |
| registration ID | Non-secret stable inventory identity; not an authentication token. |
| destination NodeId + Fleet/Paired kind | Existing typed address/reference only, not a new route or trust grant. |
| destination display name (optional) | Bounded public name from existing destination observations. |
| registration label (optional) | Admin-authored public display text. |
| bounded annotations | The draft contract's display-only/public annotations. CLI/API docs must explicitly say local agents can read them; credentials must never be placed in this field. |

The projection contains **no** HostSource/DeviceId, optimistic revision, private
configuration path, credential/token/key, pairing link/material, credential file
reference, raw connection object, or whole NodeInfo. Enabled/connectivity status
is not added to this field list: W18 may retain its existing fleet observations.
No raw endpoint URL is introduced: this registry currently stores NodeId/kind
references, so address sharing means those typed public references. A future
human endpoint projection must separately exclude URL userinfo/query secrets
and use its owning typed transport API, not a second book/resolver here.

Annotations are deliberately public display input from this unreleased feature.
The implementation must not inspect or copy credential/config stores and must
not claim heuristic redaction can make arbitrary private records shareable.
If legacy/private metadata ever exists outside this unreleased contract, it
requires an explicit migration/visibility decision before projection.

**Proposed admission and delivery:**
- Existing RPC Read-scope enforcement applies first. Direct authenticated
  Owner/Device callers may use the same public projection; a direct local
  Principal::Agent (credential issued for this node) may read it node-wide.
- Principal::Delegated stays refused, including foreign/forwarded agents. Keep
  the new read non-forwardable, as current registry management already is.
  Eligibility comes from authenticated principal kind, never loopback address,
  caller-supplied IDs, an alias, Creator ancestry or current attachment.
- The query has no source-selector argument. Return the public projection of
  records across sources, including retained disabled records; it is inventory,
  not a claim that a route is currently enabled/connected. It starts no connection
  and returns no new authority. Source-scoped full management reads are separate.
- Do not widen current ListHostRegistrations's full response. After approval,
  add a separately typed public Read request/response/client adapter as a pending
  table row at its end, using the existing package Option protocol constant None.
  Exact Rust/wire names await implementation/review, not invented by W18.
- Reuse the existing bounded directory actor and limits. No second store, source
  cache, credential ancestry record or route resolver; no writes on this path.

**Required tests after design review (now drafted, not yet executed):** direct
local Agent+Read sees public entries from Owner/Device sources; missing Read and
all Delegated callers refuse; full source-private management and all Admin writes
remain denied to agents. Explicit typed serialization probes reject extra fields
(source/revision/credential/config); foreign refs do not become routes. Cover
retained disabled/unreachable entries without connectivity claims, pending/older
SDK refusal before frames, and W18's real MCP listing adapter after a reviewed
node head. No current CLI/MCP/runtime PASS is implied by this design.


### C2 staged reply/feed integration plan (not an API handoff)

C2's proposed pure core defaults to Silent. The superseding 13:38 coordinator
ruling keeps capture integration Silent until C2's same-VTE presentation withholding /
answered-query suppression lands and passes; only then may node glue activate
NodeAuthority. CLI renderer completion is a separate D1 follow-up. Feed has a16KiB consumed-prefix budget and typed replies; actor
glue must loop only the unconsumed suffix, check forward progress, and write
reply bytes through its existing ordered PTY writer. Reply errors do not undo
already consumed screen input: never re-feed that prefix. Final readonly
snapshots retain cells/capture independently of the output ring and do not claim
a serializable partial-decoder restore. Reopened the exact server reference:
terminal-capture.ts uses translateToString(true).trimEnd(), then optional ANSI
stripping. --ansi does not regenerate SGR. Proto Plain/Preserve map over the core's
interpreted rows; no second renderer is needed. The earlier extra-renderer request
was incorrect, and the existing staged adapter already documented this rule.

This is a plan against C2's staged sketch, not imported code, a parallel parser,
a compiled adapter or consumer readiness. Actual shared engine/glue integration
and tests wait for the reviewed core hash.

Reference update.ts parseAgentChanges explicitly rejects simultaneous metadata
and thinking changes. Corrected that pre-connect error to Usage/InvalidRequest;
it is not an absent atomic-update contract. The thinking/mode option-ID decoder
also trims outer whitespace as the reference does, retaining its bound/control
checks. No new node call or mutation behavior is added by these edge corrections.

Latest development clippy run111143 (exactcd1a12d7) completed12:02:51 with
three lint errors: one archive receipt struct/field naming prefix and two missing
code backticks in schema-decoder docs. No type error or executed test result was
reported. Renamed the private receipt type (wire field names unchanged) and fixed
those comments. Retained /tmp/work/wp-cli/main-clippy-head42 evidence; a changed
source strict run and latest unit/process tests remain required.


### Context-sensitive --home alias

[Decision Log]
- Intent: preserve native init's existing --home account-scan root while adding
  Paseo's node-home selector. Reusing a global clap id propagates the native
  value into parent options; treating it as data_dir would target the wrong path.
- Choice: reuse the typed Command::Init discriminant at the one connection
  decoding edge. Init keeps its own scan-home value and its selected node/data
  directory. Other commands apply --home as the node data directory.
- Context-dependent home/host/data-dir conflicts are checked at that edge before
  resolution/connection/effects; host/node remains a clap conflict. No argv
  re-parser, second config store or foreign init handler change is introduced.
- Added parser-to-decoder regressions for data-dir + native init home, legacy
  node and new host selection, ordinary home aliases and conflicting selections.
  This is source-drafted evidence until the next updated CLI suite executes.


### Public host inventory implementation (2026-11-15 12:40 UTC)

[Decision Log]
- Purpose: let W18 consume the approved local-agent inventory without source inheritance.
- Existing assets: reuse the durable actor, bounded queue/deadline, typed display fields,
  principal scope admission and table-generated dispatch. No dependency is introduced.
- Alternatives: full-record serialization would leak source/revision; a second registry
  or treating agents as Owner would create incorrect authority.
- Choice: `host_registry.list_shared` is a separately typed, nonforwardable pending
  Read row under the existing CLI_PARITY_PROTOCOL=None. `ListSharedHostRegistrations`
  returns `SharedHostDirectory { node: NodeId, hosts: SharedHostRegistrations }`.
- Reason: the actor projects only `SharedHostRegistration` fields (registration_id,
  destination NodeId/kind, destination_name, label, annotations) across sources.
  The bounded wrapper admits at most MAX_HOST_REGISTRATIONS on construction and decode.
- Consequences: disabled and unavailable records remain inventory; no state/route is
  inferred. Handler scope-checks Read and explicitly compares an Agent's node to this
  node, denying every Delegated principal even for a local-looking origin. It neither
  traverses Creator nor borrows an Owner source. Private management still uses source().

API/CLI annotation write surfaces now explicitly state that local agents can read
annotations; no private config, credential migration or heuristic redaction exists.
Actor cross-source/disabled retention, serialized field allowlist, bounded decoding,
local/foreign/delegated admission and pending client no-frame regressions are drafted.
No compilation/test pass for this implementation or consumer-ready hash is claimed yet.

### Existing-call table rendering correction (2026-11-15 12:50 UTC)

[Decision Log]
- Purpose: explicit table/header choices must reach an actual row renderer, including
  one-result commands. Source inspection found project create and host mutations
  using object serialization in table mode, and terminal ls serializing its entire list.
- Existing assets: reuse Row/render_list and the accepted structured single serializer.
  No dependency, node call, credential or transport change is needed.
- Alternatives: duplicate per-command output branches, or lose object shape by wrapping
  every single structured result in an array.
- Choice: add output::record for table/quiet row emission and object-shaped JSON/YAML.
  Project creation, host mutation rows and terminal kill consume it; existing run/send and
  schedule receipt branches reuse it without changing their result shapes.
- Reason/consequences: terminal ls now has ID/NAME/CWD/WORKSPACE columns, quiet still
  emits actual IDs, and its structured output deliberately retains native TerminalInfo
  fields until the reviewed workspace terminal contract exists. WORKSPACE is explicitly
  unknown (`-`), never a substituted project/session ID. This is not terminal schema parity.

Reference terminal/schema.ts defines the four list columns and kill ID/SUCCESS table;
client/src/terminals/index.ts resolves exactly the eleven existing CLI key tokens (not
MCP's richer key grammar), which W17 already matches. A real fake-node process regression
creates four projects and checks default table, headerless table, object JSON and quiet
against node acknowledgements. It is drafted, not an executed latest-head pass.

### CLI unit evidence after lint corrections (2026-11-15 12:55 UTC)

Exact source0656af27d416232f2fb864d22805cd2a4d014d20 completed remote
wp-cli-narrow-20261115T120656Z-0656af27 at 12:54 UTC, all four steps rc0.
`cargo test -p vessel-cli --bin vessel --features vessel-proto/ts,vessel-web/ts`:
134 passed / 0 failed / 0 ignored; actual tests took 5.44 seconds after compilation.
Retained /tmp/work/wp-cli/cli-unit-head42 includes spec, cargo.log and steps.tsv.
This executes the offline schema/retry, parser, output, option ACK and typed-error
regressions on that source. It excludes later native init --home, gate44 merge,
public host inventory and table corrections, and does not execute CLI process tests.

Latest product-source dd33c5d7af96 (includes public inventory c6bd390be + field
rejection02305b878) has exactly one strict affected-crate run queued:
wp-cli-narrow-20261115T125525Z-dd33c5d7, public-host-cli-clippy. No result claimed.

### Raw attach reply-authority integration boundary (2026-11-15 12:55 UTC)

Read-only audit of C2's staged core e792e217 (not adopted) found a consumer boundary
for NodeAuthority query replies: native interpreters default Silent, but existing
`vessel shell attach` writes raw output to a real terminal and forwards raw input
(attach/mod.rs::attach_terminal, attach/terminal.rs). A real emulator can therefore
also answer DSR/DA unless one policy suppresses those replies. Sent exact paths to
C2 before activation; no second parser/filter or new protocol was invented. The
Windows PTY startup query is already answered and removed by pty/startup.rs, so
that startup case is distinct. Actor glue still waits for the reviewed core/API
and a single reply-authority policy for this legacy raw consumer.


### Reply authority ruling settled (2026-11-15 13:00 UTC)

C2 confirmed the raw TerminalId attach path and obtained the coordinator ruling:
W17's capture consumer uses Screen::new Silent after the reviewed core handoff.
C2 owns the separate CLI attach renderer and typed mode/restore consumer on that
same engine in Silent posture; it must avoid query-bearing raw stdout and must
not feed emitted ANSI back into the interpreter. NodeAuthority activates only
after that renderer lands and passes. Existing raw attach is not refused here.
No additional parser/regex/filter is introduced by W17. Thus query policy does
not block pure capture adoption once C2's core source/gate/review is ready.


### Preserve host inspect's intentional JSON human renderer (2026-11-15 13:05 UTC)

Reopened exact host/index.ts39-81: inspect alone overrides renderHuman with
JSON.stringify; ls/add/update/enable/disable/remove use the six-column schema.
The blanket one-row table change at dd33 was too broad. Restored inspect's JSON
human renderer, retaining structured object/quiet behavior. Host lists/mutations
now use ID/LABEL/SERVER/ENABLED/STATUS/SOURCE; source text explicitly describes the
actual Owner/Device enum, not invented Paseo id/key or authority inheritance.
The earlier baseline inventory's host output shorthand was overgeneralized;
only inspect has the human JSON override. No pending call was activated.

### D1 authoritative frame follow-up proposal (2026-11-15 13:11 UTC)

Design coordination only: no new DTO, method row, client API or mode parser exists
from this proposal. It does not block the separately approved Silent capture
consumer. C2 requested node/consumer cursor ownership before live D1 wiring.

[Decision Log]
- Purpose: restore a CLI/native renderer without guessing parser state from visible
  cells or replaying a possibly pruned, mid-CSI/mid-UTF8 raw event suffix.
- Existing assets/constraints: one actor owns Screen and TermLog numbering; the core's
  Snapshot is readonly and is not a serializable complete parser/decoder checkpoint.
  TerminalInfo.revision and raw EventSeq alone do not version a screen and its modes.
- Alternatives: full engine serialization, including both buffers, saved region/cursor,
  modes/kitty stack and decoder carry (not exposed by the frozen engine); or client
  replay/preamble reconstruction (cannot recover pruned parser carry; rejected).
- Choice: demand-driven authoritative full typed frames. The node keeps the only
  mutable decoder; consumers render cells/modes and never feed a raw suffix or generated
  ANSI back into an engine. C2 extends the same pure engine for mode observation.
- Reason: full replacement frames may coalesce without a delta/replay continuity claim.
  A readonly grid is sufficient for rendering, but is never called parser restoration.
- Consequences: frame, viewport, input modes and cursor are obtained by one command at
  the output actor. A separate opaque epoch changes whenever that authority is replaced.
  A checked FrameRevision advances on every fully consumed output chunk, admitted resize,
  mode/state change and finalization; exhaustion refuses, never wraps.

Proposed cursor is (screen epoch, frame revision, next_event_seq). The last component
is TermLog.next at the same actor admission/response boundary: every raw output record
below it has already been fed completely, including decoder carry, and its event has
been published. Partial feed prefixes never escape as an event-boundary checkpoint.
Non-screen notices may advance the event cursor without a frame revision; long-read
change admission compares epoch+frame revision, and even Unchanged returns the fresh
cursor. A resize may advance the frame revision without advancing the raw event cursor.
Renderers follow frame revision, never replay raw records starting at that event cursor.

A proposed ReadFrame(terminal, after, viewport) is an ownership-checked Read long-read.
It returns an authoritative full frame, Unchanged with cursor, or a typed unavailable /
epoch-replaced / viewport-too-large outcome. Ended frames remain readable with explicit
final state; a restarted node with no retained authority refuses rather than inventing
an empty parser. The actor registers a bounded set of waiting readers or answers at
once, with cancellation/deadline cleanup and a finite wait. Snapshot/projection work is
only on demand, not a full grid clone on every output chunk. Slow consumers receive a
later complete replacement; no dropped-delta recovery is necessary.

Both cell count and encoded frame bytes need named bounds before code. Bounded typed
viewports let a large terminal/history refuse or page explicitly rather than silently
truncate an allegedly complete frame. A history page must match the authority epoch and
frame revision or report changed/pruned; it never seeds a decoder. No eager per-output
full-grid fan-out or unbounded subscriber/restore-token map is proposed.

Modes must be supplied by C2's same-engine D1 extension, following
protocol/terminal-input-mode.ts (kitty flags/stack, win32, application cursor and
bracketed paste). W17 will explicitly map typed flags/mode enums into proto; it will
not copy the reference regex tracker, inherit a source, or substitute guessed defaults.
Resize admission/PTY writer ordering and frame publication must be proved together;
current capture inbox ordering alone is not proof of a new D1 event cursor contract.
Query replies remain Silent until the later approved same-parser presentation suppression
lands and passes; the 13:00 renderer prerequisite was superseded at13:38.

C2 accepted authoritative frames for D1 and added the stable viewport invariant:
pinned history must preserve inspected text through growth/front trimming, using
its pure buffer identity/epoch, retained origin and stable anchor/marker. Node glue
must not derive an anchor from EventSeq or distance behind the tail. FollowLive
and Pinned(anchor) are distinct intents; eviction/clamp/reset reports the requested
and actual anchor plus a typed reason. Primary/alternate transitions never reuse
an anchor in a different buffer; RIS/history clear invalidate its epoch. Reflow
remapping is the pure engine's rule. Actor epoch/anchor identity never substitutes
for authenticated NodeId+TerminalId routing, pairing or connection authority. The
frozen e792 core exposes neither modes nor markers; those remain C2's follow-up.

### Remaining existing-call receipt tables (2026-11-15 13:20 UTC)

[Decision Log]
- Purpose: consume explicit human table/header requests for real acknowledgements,
  without changing the node operations or confusing idle waits with completed runs.
- Existing assets: Row, output::record, typed wait/mutation receipts and ScheduleRow.
  Reuse them; no dependency or protocol method is added.
- Alternatives: repeated output branches or returning native object JSON for table
  selection. Neither preserves the declared reference table contract.
- Choice: wait uses AGENT ID/STATUS/MESSAGE; stop/delete use INTERRUPTED/DELETED count
  columns; archive uses AGENT ID/STATUS/ARCHIVED AT; mode-set uses AGENT ID/MODE.
  Object JSON fields stay unchanged. Quiet single records retain empty-ID suppression,
  so an idle stop with no interrupted agents emits zero bytes rather than a blank line.
- Reason/consequences: schedule and heartbeat delete now return acknowledged id plus
  typed status=deleted, matching both reference delete schemas; the former deleted:true
  field was a native difference. Heartbeat create/update reuse ScheduleRow instead of
  independently serializing native snapshots. Existing permission, mode-ACK and ownership
  checks remain the authority for every result; no success is manufactured from input.

The real CLI parity regression now additionally checks idle stop quiet is empty and
headerless stop reports count0; automation's actual deletion JSON expectation is updated
from the old boolean to a closed status enum. These source changes follow the already
queued dd33 strict head and have not executed yet. Latest-head unit/process/gate proof
remains owed. Update/provider notices and detailed schedule-log projections still have
explicitly documented native differences and are not covered by this table correction.

### Send prompt-source admission (2026-11-15 13:26 UTC)

Paseo send.ts102-141 resolves exactly one prompt source before connecting; an
image flag does not replace a prompt argument/--prompt/--prompt-file. Removed
W17's image-only exception at clap and runtime edges. The normal typed image
staging/sending path still accepts a real prompt plus images. Empty/whitespace
content continues to refuse before opening images or connecting, now consistently
with images too. This intentionally remains stricter than Paseo's unvalidated
empty-file return, retaining the existing CLI empty-input admission rule. Parser
regressions cover missing source vs a prompt+image without touching the file.

### Repository worktree list admission (2026-11-15 13:31 UTC)

[Decision Log]
- Purpose: Paseo worktree ls defaults to the current repository; it must not quietly
  list every registered worktree when the repository/ownership contract is missing.
- Existing assets: the native worktree list accepts an optional project and intentionally
  lists broadly without one. An ls alias discarded which contract the caller requested.
- Alternatives: guess a workspace from cwd/project IDs, or change native list's scope.
- Choice: split typed WorktreeCommand::Ls from the native List variant. Both default and
  explicit --cwd ls require W11's canonical registry/ownership before connection; native
  worktree list retains its established behavior.
- Reason/consequences: no broader read is substituted for a repository query. The new
  root/dispatch regression uses an invalid node URI to prove the dependency refusal
  precedes connection parsing, and checks native list remains admitted. No new call,
  workspace ID inference or parser-only parity claim is made.


### Explicit process exit evidence fixture (2026-11-15 13:35 UTC)

The shared main edge already uses try_parse_from and explicitly returns1 for parse
failures,0 for help/version, but the older baseline wording still said this was
unfixed. Corrected that wording. Test Ran now retains the actual OS ExitStatus,
so checks can distinguish1 from merely nonzero. A new real executable regression
uses a fresh private home without starting a node: help/version0, missing/unknown
arguments1, typed provider dependency refusal1 with empty stdout and a decoded
Unsupported stderr envelope. It also asserts the home stays empty. This is a drafted
regression, not a claimed new runtime pass; the current dd33 strict run predates it.


### Same-parser legacy presentation ruling (2026-11-15 13:38 UTC)

The coordinator approved C2's compatibility route for new nodes with old raw clients.
C2 owns a read-only token-boundary hook in the SAME VTE parser and pure-core presentation
withholding/suppression, only for non-rendering query tokens the node answers. Preserve
all other complete bytes; do not filter stdin (reply shapes overlap actual key encodings).
W17 owns node glue: publish presentation bytes in actor order and preserve EventSeq even
when a record's presentation bytes are empty. Replayed history must contain neither held
query prefixes nor finals that could trigger late duplicate responses.

This supersedes the 13:00 activation dependency: NodeAuthority now waits for this suppression
landing+PASS, not for the CLI renderer. C2's frozen e792 core and current capture remain
Silent and unchanged. Split query, held-history/old-raw replay, empty-record sequence and
no-stdin-key-suppression regressions are required at the actual follow-up glue integration.
No new scanner/parser, published filter, input suppression or authority switch exists now.

### No-force archive also fences reopening (2026-11-15 13:43 UTC)

[Decision Log]
- Purpose: --force must be explicit even when a session looked ended at the last read.
- Existing constraints: CLI previously refused a live idle no-force archive but sent
  legacy Mark::Archive for ended sessions. Node service::conversation::resume_session
  reopens ended structured sessions; sessions/marks.rs re-finds them and mark_live
  kills a live process. A concurrent reopen/prompt therefore defeats that snapshot.
- Alternatives: another status read (still races), client locks (do not bind other
  callers), or a duplicate node admission (W21 owns the atomic archive contract).
- Choice: withhold ALL no-force archive effects until W21's reviewed atomic admission,
  preserving the active-turn diagnostic and the explicit Force path. No Mark::Archive
  is inferred from an ended snapshot. Tell W21 to fence stored/reopen admission too.
- Reason/consequences: this is a temporary Unsupported boundary, not fake idle safety.
  W21's real IfIdle call will replace it; the CLI creates no new delivery/mark method.

A deterministic actual-CLI regression kills a fake structured session, waits for its
recorded ended status, verifies no-force archive returns1/Unsupported without marking,
then proves that exact identity can reopen unarchived. It fails under the former ended
fallback and has not executed at the current source. Whole/changed-source tests remain owed.

### Public inventory strict return (2026-11-15 13:46 UTC)

Exact dd33c5d7af964ada7168944c6b0e26212774f2d2 run
wp-cli-narrow-20261115T125525Z-dd33c5d7 completed13:44 UTC rc1. Strict Clippy
stopped at one items_after_statements diagnostic in proto host_registry/shared/tests.rs:
its independent Allowed response schema was declared after fixture statements.
Moved that test-only type to module scope; no lint suppression, wire change or
allowlist weakening. No test execution or strict pass is claimed. Retained
/tmp/work/wp-cli/public-host-cli-clippy contains exact spec, cargo.log and steps.tsv.
Later base/CLI receipt/no-force changes also need the changed-source check.

### Public read guard/service/actor regression (2026-11-15 13:58 UTC)

Added a real node-unit chain test using the existing test gateway and memory handshake
(no new production test hook). A fake already-authenticated context enters the actual
Guarded Handler, public service method and persisted registry actor: local Agent+Read
sees Owner/Device public rows, private management still denies that agent, each direct
management source sees only its own row, and missing Read/foreign Agent refuse. The
saved registry must remain byte-semantically unchanged. No harness/provider starts; all
node/user paths are private temporary homes and the lookup is empty.

This intentionally calls Guarded directly after creating a real Peer/Outbound: the None
wire gate is not disabled or assigned a pretend protocol. It tests admission and handler
composition, not credential authentication or a live pending RPC. The separate client
no-frame test keeps pending admission covered. This fixture is drafted after queued8545
strict and still needs execution with the host_ suite; no new runtime pass is claimed.


### Pinned-source send argument correction (2026-11-15 14:19 UTC)

[Decision Log]
- Purpose: the first argument must remain the recipient even when the prompt looks like
  another session ID. Source takes precedence over an erroneous survey/table row.
- Existing constraints: area-7 survey rows49/75 had send [prompt] <id>; W17 followed
  that table. Exact pinned1c00b365 agent/send.ts39-43 instead registers <id> then
  [prompt], reused by agent/index.ts60 and cli.ts105. The coordinator confirmed and
  corrected the survey; its docs/parity mirror follows the normal sync.
- Alternatives: guess argument roles from whether text is hexadecimal (can mutate the
  wrong recipient), or keep a source-incompatible group (rejected).
- Choice: AgentSend is ID-first, then optional prompt, with the same single-source
  prompt validation. Remove text-based root guessing. Explicit prompt/behavior/image/
  no-wait or presentation flags select the parity handler, including global output
  flags before the root command; unflagged root send retains native behavior.
- Reason/consequences: native root delivery/output remains a declared compatibility
  difference until W23's atomic policy is ready, as the coordinator approved. No
  new delivery call, fallback, source authority or recipient inference is added.

Updated README, both full/current117 path spellings and parser assertions. The real
CLI regression uses two fake sessions and sends one session's full UUID as the other
session's literal prompt, through both agent send and flagged root send; it checks the
actual recipient receipt AND recorded TurnStarted content. It fails under the inverted
order. The fixture now clears inherited VESSEL_AGENT_SESSION in its CHILD process, so
an unrelated caller environment cannot steer default policy. These are unexecuted
changed-source tests; earlier134 units and queued8545 strict do not validate this fix.

### Version short flag (2026-11-15 14:23 UTC)

Paseo cli.ts53 declares lowercase -v/--version; native clap generated uppercase -V.
Reuse clap's existing Version action with one explicit root argument: -v is primary,
-V is a visible compatibility alias, and --version/metadata stay unchanged. Disable
only the duplicate generated flag, not version handling, and add no argv scanner or
runtime version path. Root clap and actual process exit regressions now cover all
three spellings (source only; execution owed). This uses the already pinned builder
API, without a new dependency or node interaction.

### Canonical W7 values consumed without ingress (2026-11-15 14:30 UTC)

W7 handed off reviewed values-only c2dcebea058968d207da0a7a6b1ff604af883bc7
(V3 row162 supplemental SOURCE PASS, integrator batch12 a5311bc2). Merged exactly
that leaf on its ede base, not the unreviewed runtime branch. It adds only the two
proto modules and declarations; no row, SDK, LocalCredential, field installation
or protocol assignment. Its None constant remains None. W7's f887 source tests4/0/0
are scoped producer evidence, not a W17 or live-hook pass.

The CLI presence check now uses canonical TERMINAL_ACTIVITY_ID_ENV,
TERMINAL_ACTIVITY_SECRET_ENV and TERMINAL_ACTIVITY_SOCKET_ENV instead of invented
ID/token/URL names. Missing attribution remains silent0 without reading stdin or
connecting. Presence alone does not read/log a secret or grant an action; configured
paths explicitly await the restricted IPC SDK/producer. No listener, URL ingestion,
normal node credential, TerminalInfo activity field or semantic-state inference is
added. Its own changed-source check is still owed.

### Public inventory import correction (2026-11-15 14:37 UTC)

Exact85454788 run wp-cli-narrow-20261115T134749Z-85454788 completed14:35 UTC
rc1. The earlier declaration-position lint is gone; compilation reached vessel-node
and stopped on E0603: service/host_registry/shared.rs imported Method through the
private rpc module. Use the existing public vessel_proto::Method re-export instead.
The proto crate's own internal tests may still name crate::rpc. No wire/authority
or data shape changes and no lint waiver. Retained public-host-cli-clippy2 spec/log/
steps are source-red evidence, not an executed test or latest strict pass.
