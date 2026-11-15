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
exit 0. Existing Vessel clap parse errors are 2; this is a stated baseline difference to fix.
Paseo --host and --home are mutually exclusive. Vessel uses --node and --data-dir and a local
node book; daemon host registrations must not be emulated by mutating that local book.

Data flow: clap validates input → feature handler resolves selectors → existing NodeClient
request → typed response → presentation → stdout. Stream/attach paths retain raw output.
No live node, real provider, credential store or production configuration is used for checks.
Existing CLI modules remain the authority for lifecycle/daemon/scripts.

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
| `logs <id>` | `--since <time>`: Show logs since timestamp<br>`--filter <type>`: Filter by event type (tools, text, errors, permissions)<br>`--tail <n>`: Show last n entries<br>`-f, --follow`: Follow log output (streaming) | log ID | W17 | Different argument/output contract; not full parity. | R: human transcript/event text only; --json/--format do not change this handler; 0/1 | `packages/cli/src/cli.ts:93` |
| `stop [id]` | `--cwd <path>`: Stop all agents in directory<br>`--all`: Stop all agents | send ID TEXT --interrupt (requires a new prompt) | W17 | Different argument/output contract; not full parity. | W; stopSchema; 0/1 | `packages/cli/src/cli.ts:97` |
| `delete [id]` | `--cwd <path>`: Delete all agents in directory<br>`--all`: Delete all agents | session delete ID --yes | W17 | Different argument/output contract; not full parity. | W; deleteSchema; 0/1 | `packages/cli/src/cli.ts:101` |
| `send [prompt] <id>` | `--no-wait`: Return immediately without waiting for completion<br>`new Option("--behavior <behavior>", "Active-turn behavior (agent default: steer; otherwise: interrupt)").choices(["interrupt", "steer"])`<br>`--image <path>`: Attach image(s) to the message; default=[]<br>`--prompt-file <path>`: Read the message from a UTF-8 text file<br>`--prompt <text>`: Provide the message inline as a flag | send ID TEXT | W17 | Different argument/output contract; not full parity. | W; agentSendSchema; 0/1 | `packages/cli/src/cli.ts:105` |
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
| `agent send [prompt] <id>` | `--no-wait`: Return immediately without waiting for completion<br>`new Option("--behavior <behavior>", "Active-turn behavior (agent default: steer; otherwise: interrupt)").choices(["interrupt", "steer"])`<br>`--image <path>`: Attach image(s) to the message; default=[]<br>`--prompt-file <path>`: Read the message from a UTF-8 text file<br>`--prompt <text>`: Provide the message inline as a flag | send ID TEXT | W17 | Different argument/output contract; not full parity. | W; agentSendSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:60` |
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
| `host ls ` | — | nodes list (local node book; not daemon host registrations) | W17 | Different argument/output contract; not full parity. | W; { ...schema, renderHuman: () => JSON.stringify(host, null, 2) }; 0/1 | `packages/cli/src/commands/host/index.ts:85` |
| `host add <connection>` | `--label <name>`<br>`--meta <key=value>`: Set metadata; default=[] | nodes add NAME ADDRESS (local node book; not daemon host registrations) | W17 | Different argument/output contract; not full parity. | W; { ...schema, renderHuman: () => JSON.stringify(host, null, 2) }; 0/1 | `packages/cli/src/commands/host/index.ts:88` |
| `host update <registration-id>` | `--label <name>`<br>`--connection <connection>`<br>`--replace-identity`: Accept a new destination identity<br>`--meta <key=value>`: Replace supplied metadata keys; default=[]<br>`--remove-meta <key>`: Remove a metadata key; default=[] | none (local node book; not daemon host registrations) | W17 | Missing entry point or backend contract; see dependencies below. | W; { ...schema, renderHuman: () => JSON.stringify(host, null, 2) }; 0/1 | `packages/cli/src/commands/host/index.ts:99` |
| `host inspect <registration-id>` | — | none (local node book; not daemon host registrations) | W17 | Missing entry point or backend contract; see dependencies below. | W; { ...schema, renderHuman: () => JSON.stringify(host, null, 2) }; 0/1 | `packages/cli/src/commands/host/index.ts:112` |
| `host enable <registration-id>` | — | none (local node book; not daemon host registrations) | W17 | Missing entry point or backend contract; see dependencies below. | W; { ...schema, renderHuman: () => JSON.stringify(host, null, 2) }; 0/1 | `packages/cli/src/commands/host/index.ts:112` |
| `host disable <registration-id>` | — | none (local node book; not daemon host registrations) | W17 | Missing entry point or backend contract; see dependencies below. | W; { ...schema, renderHuman: () => JSON.stringify(host, null, 2) }; 0/1 | `packages/cli/src/commands/host/index.ts:112` |
| `host rm <registration-id>` | — | nodes remove NAME (local node book; not daemon host registrations) | W17 | Different argument/output contract; not full parity. | W; { ...schema, renderHuman: () => JSON.stringify(host, null, 2) }; 0/1 | `packages/cli/src/commands/host/index.ts:112` |
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
  retain their original Vessel syntax; their Paseo invocation forms are not yet aliases.
- Agent list uses current-directory scope by default, excludes archived, supports global/all,
  validated label filters and advertised thinking choices. The provider field currently uses
  Vessel's HarnessRef; created is an RFC3339 timestamp, not Paseo's relative-time string.
  Inspect currently returns typed SessionInfo, not Paseo's capitalized Inspect projection.
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
node model client/listener setup moves unchanged to node/model_setup.rs. This behavior-preserving
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

Root `run PROMPT --provider PROVIDER` and prompt-first `send PROMPT ID` now decode into the
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
