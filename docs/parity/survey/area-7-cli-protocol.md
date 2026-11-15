# Paseo 0.15.4 CLI and protocol reference

Source: `/home/dev/refs/paseo-latest`, commit `1c00b365c9e0b74c1580eba33d905d693125ae21`; surveyed 2026-11-15. Static source inspection only; no builds, daemon invocation, network calls, or source modifications. Paths below are relative to that checkout.

This is a reference surface: command groups and protocol feature areas replace screen-oriented features. Counts distinguish canonical executable command paths, compatibility aliases, session discriminators, transport envelopes, and nested payload variants. Tables were saved incrementally and checked against static command registrations and protocol union membership.

[Decision Log]
- Purpose: provide a precise Rust-port inventory using the existing command registrations and Zod wire contracts.
- Existing assets: `packages/cli/src/cli.ts`, `packages/protocol/src/messages.ts` and imported RPC schemas, `packages/client/src/index.ts` and `daemon-client.ts`.
- Alternatives: runtime `--help` exploration or generated declarations.
- Choice: reuse source registrations/schemas and trace server dispatch; avoid runtime initialization and stale generated outputs.
- Tradeoff: proves source contracts, not runtime correctness or service/account permissions; no live service was accessed and no permissions were changed.

## CLI connection, parsing, output and exit contract

Source entry points: `packages/cli/src/cli.ts:48`, `run.ts:59`, `utils/command-options.ts:46`, `utils/daemon-target.ts:6`, `utils/client.ts`. This inventory is a static registration audit; no CLI action was run.

| Shared surface | Exact behavior/default |
|---|---|
| `-h, --help`, `help [command]` | Commander-generated help; parser help/version exits 0; malformed/unknown arguments exit 1. Group-only commands display Commander help and have no daemon action (except `usage`). |
| `-v, --version` | Package/installed CLI version; root and top-level commands receive it, except plugin so `plugin update --version` remains an update option. |
| `-o, --format <format>` | `table` default; json/yaml; `cli` normalizes to table in withOutput; invalid format gives INVALID_FORMAT. |
| `--json` | Alias choosing JSON even when --format differs. |
| `-q, --quiet` | ID-only lines; takes precedence over format unless --output-schema forces structured JSON. |
| `--no-headers`, `--no-color` | Headers/color default enabled; commands rendering their own text may not use these. |
| `--host <host>` / `--home <path>` | Mutually exclusive explicit selectors. Home defaults through PASEO_HOME to ~/.paseo; PASEO_HOST selects an endpoint when no explicit selector. Both environment selectors together are ambiguous; conflicting duplicate selectors across levels are rejected. Local-only commands reject --host and ignore PASEO_HOST. |
| Endpoint syntax | host:port, tcp/http/ws URLs, Unix/socket path, SSH, pairing/deep-link offers; parsed by protocol/host-target.ts and connector. SSH uses system ssh -T -W, BatchMode, 10 s ConnectTimeout and default daemon port 6767 (`packages/protocol/src/ssh-transport.ts:1`). |
| W: withOutput | Table/human renderer by default; JSON is result.data (object for single, array for list), YAML same; schema.serialize may collapse identical per-row serializations; quiet uses schema.idField. Errors to stderr are `{error:{code,message,details?}}` for JSON/YAML; human Error text otherwise. Success 0, thrown validation/connection/RPC/render failure 1. |
| R: custom/raw output | Logs/attach, pairing, onboarding, hooks, capture and send-keys implement output themselves; global format support is only what their handlers implement. See command-specific rows. |
| Exit-state distinction | Ordinary agent run/send/wait can report timeout/permission/error as returned state with exit 0; run --output-schema instead throws OUTPUT_SCHEMA_FAILED when no valid structured response is produced. Do not infer success of an agent turn from CLI exit 0. Foreground daemon run propagates worker/supervisor result; plugin update sets 1 for failed outcomes. |
| Closed stdout | EPIPE is ignored so a consumer such as head can close stdout without an uncaught exception (`run.ts:53`). |

Flags below are **in addition to** the inherited global surface. An unprovided positive boolean is false/absent; a lone negated option defaults its positive form to true; an ordinary value is absent unless a default is listed. Paired positive/negative options can intentionally remain undefined so inheritance works. `collectMultiple, []` means repeated values accumulate from an empty list. References name the output schema so field-level serialization can be inspected without executing the command.

## CLI root

Primary commands share their implementations with `agent`; root `stop` stops agents, while `daemon stop` controls the daemon. `paseo` without arguments invokes onboarding; initial `--relay`/`--no-relay` also select onboarding (`run.ts:13`). An existing directory argument opens the desktop project through a deep link (`classify.ts:51`, `commands/open.ts:108`); that is an invocation form, not a registered command.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `ls ` | `--thinking <id>`: Filter by thinking option ID<br>`--label <key=value>`: Filter by label (can be used multiple times); default=[]<br>`--background`: Include background workspace agents<br>`-g, --global`: List agents across all directories<br>`-a, --all`: Include archived and background agents | List agents. By default excludes archived agents.; `fetchAgents` | W; agentLsSchema; 0/1 | `packages/cli/src/cli.ts:65` |
| `run <prompt>` | `--output-schema <schema>`: Output JSON matching the provided schema file path or inline JSON schema<br>`--wait-timeout <duration>`: Maximum time to wait for agent to finish (e.g., 30s, 5m, 1h). Default: no limit<br>`--label <key=value>`: Add label(s) to the agent (can be used multiple times); default=[]<br>`--env <key=value>`: Set environment variable(s) for the agent process (can be used multiple times); default=[]<br>`--cwd <path>`: Working directory (default: current)<br>`--image <path>`: Attach image(s) to the initial prompt (can be used multiple times); default=[]<br>`--workspace <id>`: Run in an existing workspace (defaults to the caller workspace when agent-scoped)<br>`--forge <forge>`: Forge for checkout-pr mode<br>`--pr-number <n>`: Pull request or change request number for checkout-pr mode<br>`--branch <name>`: Existing branch for checkout-branch mode<br>`--base <ref>`: Base ref for branch-off mode<br>`--new-branch <name>`: New branch name for branch-off mode<br>`--worktree-slug <slug>`: Managed worktree path slug<br>`--worktree-mode <mode>`: Worktree mode: branch-off, checkout-branch, or checkout-pr<br>`new Option("--worktree <name>", "Legacy workspace isolation alias").hideHelp()`<br>`--new-workspace <local\|worktree>`: Create a separate local or worktree workspace<br>`--mode <mode>`: Provider-specific mode (e.g., plan, default, bypass)<br>`--thinking <id>`: Thinking option ID to use for this run<br>`--model <model>`: Model to use (e.g., claude-sonnet-4-20250514, claude-3-5-haiku-20241022)<br>`--provider <provider>`: Agent provider, or provider/model (e.g. codex or codex/gpt-5.4)<br>`new Option("--name <name>", "Hidden alias for --title").hideHelp()`<br>`--title <title>`: Assign a title to the agent<br>`new Option("--detach", "Legacy alias for --no-wait").hideHelp()`<br>`new Option("-d, --background", "Legacy alias for --no-wait").hideHelp()`<br>`--no-wait`: Return immediately without waiting for completion | Create and start an agent with a task; `fetchAgent, fetchWorkspaces, createWorkspace, createAgent, sendMessage, waitForFinish` | W; outputSchema, structuredRunSchema(output), agentRunSchema; 0/1 | `packages/cli/src/cli.ts:67` |
| `import <id>` | `--label <key=value>`: Add label(s) to the agent (can be used multiple times); default=[]<br>`--cwd <path>`: Working directory for providers that require it<br>`--provider <provider>` **required**: Agent provider id | Import an existing provider session as a Paseo agent; `importAgent` | W; agentRunSchema; 0/1 | `packages/cli/src/cli.ts:71` |
| `clone <repo>` | `--dir <path>` **required**: Parent directory to clone into (for example: ~/workspace)<br>`new Option("--protocol <protocol>", "Protocol for owner/repo shorthand repositories").choices(["https", "ssh"])` | Clone a GitHub repo and register it as a Paseo workspace; `cloneGithubProject` | W; cloneSchema; 0/1 | `packages/cli/src/cli.ts:75` |
| `attach <id>` | — | Attach to a running agent's output stream; `fetchAgent, subscribeAgentTimeline` | R: human transcript/event text only; --json/--format do not change this handler; 0/1 | `packages/cli/src/cli.ts:89` |
| `logs <id>` | `--since <time>`: Show logs since timestamp<br>`--filter <type>`: Filter by event type (tools, text, errors, permissions)<br>`--tail <n>`: Show last n entries<br>`-f, --follow`: Follow log output (streaming) | View agent activity/timeline; `fetchAgent, subscribeAgentTimeline` | R: human transcript/event text only; --json/--format do not change this handler; 0/1 | `packages/cli/src/cli.ts:93` |
| `stop [id]` | `--cwd <path>`: Stop all agents in directory<br>`--all`: Stop all agents | Interrupt an agent if it is running (no-op for idle agents); `fetchAgents, fetchAgent, cancelAgent` | W; stopSchema; 0/1 | `packages/cli/src/cli.ts:97` |
| `delete [id]` | `--cwd <path>`: Delete all agents in directory<br>`--all`: Delete all agents | Delete an agent (interrupt if running, then hard-delete); `fetchAgentHistory, fetchAgent, cancelAgent, deleteAgent` | W; deleteSchema; 0/1 | `packages/cli/src/cli.ts:101` |
| `send [prompt] <id>` | `--no-wait`: Return immediately without waiting for completion<br>`new Option("--behavior <behavior>", "Active-turn behavior (agent default: steer; otherwise: interrupt)").choices(["interrupt", "steer"])`<br>`--image <path>`: Attach image(s) to the message; default=[]<br>`--prompt-file <path>`: Read the message from a UTF-8 text file<br>`--prompt <text>`: Provide the message inline as a flag | Send a message/task to an existing agent; `sendAgentMessage, waitForFinish` | W; agentSendSchema; 0/1 | `packages/cli/src/cli.ts:105` |
| `inspect <id>` | — | Show detailed information about an agent; `fetchAgent` | W; createInspectSchema(inspectData); 0/1 | `packages/cli/src/cli.ts:109` |
| `wait <id>` | `--timeout <seconds>`: Maximum wait time (default: no limit) | Wait for an agent to become idle; `waitForFinish` | W; agentWaitSchema; 0/1 | `packages/cli/src/cli.ts:113` |
| `archive <id>` | `--force`: Force archive running agent (interrupts active run first) | Archive an agent (soft-delete); `fetchAgents, archiveAgent` | W; archiveSchema; 0/1 | `packages/cli/src/cli.ts:117` |
| `hooks <agent> <event>` | — | Record agent hook activity; `HTTP POST activity target from PASEO_TERMINAL_* / PASEO_ACTIVITY_TOKEN; no session RPC` | R: no stdout; missing env/unrecognized activity/network failure silently returns 0 | `packages/cli/src/commands/hooks.ts:22` |
| `onboard ` | `--listen <listen>`: Listen target (host:port, port, or unix socket path)<br>`--port <port>`: Port to listen on (default: 6767)<br>`--relay`: Enable relay connection without prompting<br>`--no-relay`: Disable relay connection<br>`--no-mcp`: Disable the Agent MCP HTTP endpoint<br>`--hostnames <hosts>`: 'Daemon hostnames (comma-separated, e.g. "myhost,.example.com" or "true" for any)'<br>`new Option("--allowed-hosts <hosts>").hideHelp()`<br>`--timeout <seconds>`: Max time to wait for daemon readiness (default: 600)<br>`--voice <mode>`: Voice setup mode: ask, enable, disable; default="ask" | Run first-time setup, start daemon, and print pairing instructions; `reloadDaemonConfig, getDaemonPairingOffer, patchDaemonConfig` | R: interactive notes/progress/QR; cancel 0, failure 1 | `packages/cli/src/commands/onboard.ts:152` |
| `pair ` | `--relay`: Enable relay without prompting | Print the daemon pairing QR code and link; `getDaemonPairingOffer, patchDaemonConfig; same as daemon pair` | R: QR/link + guidance; --json pairing offer; 0/1 | `packages/cli/src/commands/daemon/pair.ts:61` |
| `reload ` | — | Reload config.json without restarting; `reloadDaemonConfig; same as daemon reload` | W; daemonReloadSchema; 0/1 | `packages/cli/src/commands/daemon/reload.ts:64` |
| `restart ` | removed launch flags (listed below)<br>`--timeout <seconds>`: Replacement readiness deadline (default: 600)<br>`--kill-terminals`: Close every terminal instead of keeping them across the restart | Restart the selected daemon worker, retaining its supervisor launch; `getDaemonStatus, restartServer; same as daemon restart` | W: object {action,target,supervisorPid,previousWorkerPid,workerPid,acknowledged}; human replacement confirmation; 0/1 | `packages/cli/src/commands/daemon/restart.ts:18` |
| `start ` | removed launch flags (listed below)<br>`--timeout <seconds>`: Readiness deadline (default: 600) | Start the local daemon from persistent configuration (local operation); `No daemon client method; local operation; same as daemon start` | W: object {action,home,pid,listen,logPath}; human Started/Already running and log path; quiet PID; 0/1 | `packages/cli/src/commands/daemon/start.ts:8` |
| `status ` | — | Observe the selected daemon and its published endpoint; `getDaemonStatus; same as daemon status` | W: local metadata + observed reachability/provider/relay status; human key:value lines; quiet daemon; 0/1 | `packages/cli/src/commands/daemon/status.ts:16` |

## CLI agent

IDs generally accept IDs/prefixes; exact resolution differs by command and may fall back to name. `run`/`send` wait by default, but a reported timeout, permission, or agent-error state is returned data and is not automatically a nonzero process exit. `run --background` is the old no-wait spelling and does not mean a background workspace.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `agent ls ` | `--thinking <id>`: Filter by thinking option ID<br>`--label <key=value>`: Filter by label (can be used multiple times); default=[]<br>`--background`: Include background workspace agents<br>`-g, --global`: List agents across all directories<br>`-a, --all`: Include archived and background agents | List agents. By default excludes archived agents.; `fetchAgents` | W; agentLsSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:30` |
| `agent run <prompt>` | `--output-schema <schema>`: Output JSON matching the provided schema file path or inline JSON schema<br>`--wait-timeout <duration>`: Maximum time to wait for agent to finish (e.g., 30s, 5m, 1h). Default: no limit<br>`--label <key=value>`: Add label(s) to the agent (can be used multiple times); default=[]<br>`--env <key=value>`: Set environment variable(s) for the agent process (can be used multiple times); default=[]<br>`--cwd <path>`: Working directory (default: current)<br>`--image <path>`: Attach image(s) to the initial prompt (can be used multiple times); default=[]<br>`--workspace <id>`: Run in an existing workspace (defaults to the caller workspace when agent-scoped)<br>`--forge <forge>`: Forge for checkout-pr mode<br>`--pr-number <n>`: Pull request or change request number for checkout-pr mode<br>`--branch <name>`: Existing branch for checkout-branch mode<br>`--base <ref>`: Base ref for branch-off mode<br>`--new-branch <name>`: New branch name for branch-off mode<br>`--worktree-slug <slug>`: Managed worktree path slug<br>`--worktree-mode <mode>`: Worktree mode: branch-off, checkout-branch, or checkout-pr<br>`new Option("--worktree <name>", "Legacy workspace isolation alias").hideHelp()`<br>`--new-workspace <local\|worktree>`: Create a separate local or worktree workspace<br>`--mode <mode>`: Provider-specific mode (e.g., plan, default, bypass)<br>`--thinking <id>`: Thinking option ID to use for this run<br>`--model <model>`: Model to use (e.g., claude-sonnet-4-20250514, claude-3-5-haiku-20241022)<br>`--provider <provider>`: Agent provider, or provider/model (e.g. codex or codex/gpt-5.4)<br>`new Option("--name <name>", "Hidden alias for --title").hideHelp()`<br>`--title <title>`: Assign a title to the agent<br>`new Option("--detach", "Legacy alias for --no-wait").hideHelp()`<br>`new Option("-d, --background", "Legacy alias for --no-wait").hideHelp()`<br>`--no-wait`: Return immediately without waiting for completion | Create and start an agent with a task; `fetchAgent, fetchWorkspaces, createWorkspace, createAgent, sendMessage, waitForFinish` | W; outputSchema, structuredRunSchema(output), agentRunSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:32` |
| `agent import <id>` | `--label <key=value>`: Add label(s) to the agent (can be used multiple times); default=[]<br>`--cwd <path>`: Working directory for providers that require it<br>`--provider <provider>` **required**: Agent provider id | Import an existing provider session as a Paseo agent; `importAgent` | W; agentRunSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:36` |
| `agent attach <id>` | — | Attach to a running agent's output stream; `fetchAgent, subscribeAgentTimeline` | R: human transcript/event text only; --json/--format do not change this handler; 0/1 | `packages/cli/src/commands/agent/index.ts:40` |
| `agent logs <id>` | `--since <time>`: Show logs since timestamp<br>`--filter <type>`: Filter by event type (tools, text, errors, permissions)<br>`--tail <n>`: Show last n entries<br>`-f, --follow`: Follow log output (streaming) | View agent activity/timeline; `fetchAgent, subscribeAgentTimeline` | R: human transcript/event text only; --json/--format do not change this handler; 0/1 | `packages/cli/src/commands/agent/index.ts:44` |
| `agent open <agent-id>` | `--server <server-id>`: Server ID (defaults to the local daemon) | Open an existing agent in Paseo Desktop; `fetchAgent` | W; openAgentSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:48` |
| `agent stop [id]` | `--cwd <path>`: Stop all agents in directory<br>`--all`: Stop all agents | Interrupt an agent if it is running (no-op for idle agents); `fetchAgents, fetchAgent, cancelAgent` | W; stopSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:52` |
| `agent delete [id]` | `--cwd <path>`: Delete all agents in directory<br>`--all`: Delete all agents | Delete an agent (interrupt if running, then hard-delete); `fetchAgentHistory, fetchAgent, cancelAgent, deleteAgent` | W; deleteSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:56` |
| `agent send [prompt] <id>` | `--no-wait`: Return immediately without waiting for completion<br>`new Option("--behavior <behavior>", "Active-turn behavior (agent default: steer; otherwise: interrupt)").choices(["interrupt", "steer"])`<br>`--image <path>`: Attach image(s) to the message; default=[]<br>`--prompt-file <path>`: Read the message from a UTF-8 text file<br>`--prompt <text>`: Provide the message inline as a flag | Send a message/task to an existing agent; `sendAgentMessage, waitForFinish` | W; agentSendSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:60` |
| `agent inspect <id>` | — | Show detailed information about an agent; `fetchAgent` | W; createInspectSchema(inspectData); 0/1 | `packages/cli/src/commands/agent/index.ts:64` |
| `agent wait <id>` | `--timeout <seconds>`: Maximum wait time (default: no limit) | Wait for an agent to become idle; `waitForFinish` | W; agentWaitSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:68` |
| `agent mode <id> [mode]` | `--list`: List available modes for this agent | Change an agent's operational mode; `fetchAgent, setAgentMode` | W; modeListSchema, setModeSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:73` |
| `agent archive <id>` | `--force`: Force archive running agent (interrupts active run first) | Archive an agent (soft-delete); `fetchAgents, archiveAgent` | W; archiveSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:82` |
| `agent reload <id>` | — | Reload an agent (restarts the underlying process); `fetchAgents, refreshAgent` | W; reloadSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:86` |
| `agent detach <id>` | — | Make a subagent independent without stopping or moving it; `fetchAgents, detachAgent` | W; detachSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:90` |
| `agent update <id>` | `--name <name>`: Update the agent's display name<br>`--thinking <id>`: Update the agent's thinking option ID<br>`--label <label>`: Add/set label(s) on the agent (can be used multiple times or comma-separated); default=[] | Update an agent's settings or metadata; `fetchAgent, setAgentThinkingOption, updateAgent` | W; updateSchema; 0/1 | `packages/cli/src/commands/agent/index.ts:97` |

## CLI daemon

`start`, `run`, `set-password`, and `config` are local-home operations. Start/restart readiness default is 600 s. Foreground `daemon run` owns deployment process lifetime and propagates its exit code. Status probes the selected endpoint without treating remote process liveness as verified. `stop --force` requires a local home; it conflicts with `--keep-terminals`. Terminal preservation requires `serverInfo.features.terminalsSurviveRestart` on the same connection carrying the lifecycle request.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `daemon set-password ` | — | Save a hashed daemon password (local operation); `No daemon client method; local operation` | W; setPasswordResultSchema; 0/1 | `packages/cli/src/commands/daemon/index.ts:26` |
| `daemon pair ` | `--relay`: Enable relay without prompting | Print the daemon pairing QR code and link; `getDaemonPairingOffer, patchDaemonConfig` | R: QR/link + guidance; --json pairing offer; 0/1 | `packages/cli/src/commands/daemon/pair.ts:61` |
| `daemon reload ` | — | Reload config.json without restarting; `reloadDaemonConfig` | W; daemonReloadSchema; 0/1 | `packages/cli/src/commands/daemon/reload.ts:64` |
| `daemon restart ` | removed launch flags (listed below)<br>`--timeout <seconds>`: Replacement readiness deadline (default: 600)<br>`--kill-terminals`: Close every terminal instead of keeping them across the restart | Restart the selected daemon worker, retaining its supervisor launch; `getDaemonStatus, restartServer` | W: object {action,target,supervisorPid,previousWorkerPid,workerPid,acknowledged}; human replacement confirmation; 0/1 | `packages/cli/src/commands/daemon/restart.ts:18` |
| `daemon start ` | removed launch flags (listed below)<br>`--timeout <seconds>`: Readiness deadline (default: 600) | Start the local daemon from persistent configuration (local operation); `No daemon client method; local operation` | W: object {action,home,pid,listen,logPath}; human Started/Already running and log path; quiet PID; 0/1 | `packages/cli/src/commands/daemon/start.ts:8` |
| `daemon run ` | removed launch flags (listed below) | Run a local daemon in the foreground with deployment environment overrides; `No daemon client method; local operation` | W: lifecycle result; propagates result.exitCode (default 0); wrapper failure 1 | `packages/cli/src/commands/daemon/start.ts:38` |
| `daemon status ` | — | Observe the selected daemon and its published endpoint; `getDaemonStatus` | W: local metadata + observed reachability/provider/relay status; human key:value lines; quiet daemon; 0/1 | `packages/cli/src/commands/daemon/status.ts:16` |
| `daemon stop ` | `--timeout <seconds>`: Graceful exit deadline (default: 15)<br>`--force`: Permit forced local process cleanup<br>`--keep-terminals`: Leave terminals running; the next daemon started for this home takes them back<br>`--kill-timeout <seconds>`: Forced exit deadline (default: 3) | Stop the selected daemon; `shutdownServer; local stopDaemonInstance` | W: action/home-or-host/pid/forced/usedLifecycleRpc as available; human local result or remote shutdown requested; 0/1 | `packages/cli/src/commands/daemon/stop.ts:14` |

## CLI daemon config

Edits configured state in `<home>/config.json`, not an arbitrary remote file. `get` recursively redacts secrets. `set` parses JSON, falling back to string; `--string` forces literal text. Saved config is reloaded if a local daemon is reachable; restart-required fields are reported, not silently applied. See `packages/cli/src/commands/daemon/config.ts:34`.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `daemon config get [path] ` | — | ; `No daemon client method; local operation` | W; { idField: () => "configuration", columns: [], renderHuman: () => JSON.stringify(data, null, 2), }; 0/1 | `packages/cli/src/commands/daemon/config.ts:78` |
| `daemon config set <path> <value> ` | `--string`: Interpret value literally as a string | ; `reloadDaemonConfig` | W; { idField: () => "configuration", columns: [], renderHuman: () => JSON.stringify(data, null, 2), }; 0/1 | `packages/cli/src/commands/daemon/config.ts:93` |
| `daemon config unset <path> ` | — | ; `reloadDaemonConfig` | W; { idField: () => "configuration", columns: [], renderHuman: () => JSON.stringify(data, null, 2), }; 0/1 | `packages/cli/src/commands/daemon/config.ts:109` |

## CLI host

IDs here are registration IDs, not server IDs. `inspect` reads the full directory entry; enable/disable patch the same registration. Metadata repeats key=value; removal merges from current metadata. Replacing a destination identity is explicit. Host source ownership comes from the authenticated session.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `host ls ` | — | List saved hosts, including disabled and failed entries; `listHosts` | W; { ...schema, renderHuman: () => JSON.stringify(host, null, 2) }; 0/1 | `packages/cli/src/commands/host/index.ts:85` |
| `host add <connection>` | `--label <name>`<br>`--meta <key=value>`: Set metadata; default=[] | ; `addHost` | W; { ...schema, renderHuman: () => JSON.stringify(host, null, 2) }; 0/1 | `packages/cli/src/commands/host/index.ts:88` |
| `host update <registration-id>` | `--label <name>`<br>`--connection <connection>`<br>`--replace-identity`: Accept a new destination identity<br>`--meta <key=value>`: Replace supplied metadata keys; default=[]<br>`--remove-meta <key>`: Remove a metadata key; default=[] | ; `listHosts, updateHost` | W; { ...schema, renderHuman: () => JSON.stringify(host, null, 2) }; 0/1 | `packages/cli/src/commands/host/index.ts:99` |
| `host inspect <registration-id>` | — | ; `listHosts` | W; { ...schema, renderHuman: () => JSON.stringify(host, null, 2) }; 0/1 | `packages/cli/src/commands/host/index.ts:112` |
| `host enable <registration-id>` | — | ; `listHosts, updateHost` | W; { ...schema, renderHuman: () => JSON.stringify(host, null, 2) }; 0/1 | `packages/cli/src/commands/host/index.ts:112` |
| `host disable <registration-id>` | — | ; `listHosts, updateHost` | W; { ...schema, renderHuman: () => JSON.stringify(host, null, 2) }; 0/1 | `packages/cli/src/commands/host/index.ts:112` |
| `host rm <registration-id>` | — | ; `listHosts, removeHost` | W; { ...schema, renderHuman: () => JSON.stringify(host, null, 2) }; 0/1 | `packages/cli/src/commands/host/index.ts:112` |

## CLI project

Registry operations retain the project directory. Create defaults to current directory only for a local-home target; explicit endpoints require a path. Delete removes the registry project and archives its active workspaces. Rename requires a name or --reset.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `project create [path]` | — | Register a project directory; `addProject` | W; projectSchema; 0/1 | `packages/cli/src/commands/project/index.ts:12` |
| `project ls ` | — | List projects; `listProjects` | W; projectSchema; 0/1 | `packages/cli/src/commands/project/index.ts:19` |
| `project rename <project-id> [name]` | `--reset`: Clear the custom name and use the directory name | Set a project's user-visible name; `renameProject` | W; projectRenameSchema; 0/1 | `packages/cli/src/commands/project/index.ts:23` |
| `project delete <project-id>` | — | Delete a project and its workspaces; `removeProject` | W; projectDeleteSchema; 0/1 | `packages/cli/src/commands/project/index.ts:33` |

## CLI workspace

Create requires isolation local/worktree. Worktree source mode is branch-off (default), checkout-branch or checkout-pr, with mutually valid source-specific fields. Omitted background inherits caller workspace visibility; explicit --no-background selects visible. Setup is an explicit run of potentially blocked setup. Rename --reset clears the custom title; archive retains a local directory and may remove an owned worktree only after its final reference.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `workspace setup <workspace-id>` | — | Allow and run setup for a workspace; `runWorkspaceSetup` | W; workspaceSetupSchema; 0/1 | `packages/cli/src/commands/workspace/index.ts:13` |
| `workspace create ` | `--isolation <local\|worktree>` **required**: Workspace isolation<br>`--path <path>`: Local directory or source checkout (default: current)<br>`--project <id>`: Existing project id<br>`--title <title>`: Workspace title<br>`--mode <mode>`: Worktree mode: branch-off, checkout-branch, or checkout-pr (default: branch-off)<br>`--worktree-slug <slug>`: Managed worktree path slug<br>`--new-branch <name>`: New branch name (--mode branch-off)<br>`--base <ref>`: Base ref (--mode branch-off)<br>`--branch <name>`: Existing branch (--mode checkout-branch)<br>`--pr-number <n>`: Pull request or change request number (--mode checkout-pr)<br>`--forge <forge>`: Forge for --mode checkout-pr (default: source checkout)<br>`--background`: Create a background workspace<br>`--no-background`: Create a visible workspace instead of inheriting background | Create a workspace; `fetchAgent, createWorkspace` | W; workspaceSchema; 0/1 | `packages/cli/src/commands/workspace/index.ts:20` |
| `workspace ls ` | `--background`: Include background workspaces | List active workspaces; `fetchWorkspaces` | W; options.background ? workspaceWithBackgroundSchema : workspaceSchema; 0/1 | `packages/cli/src/commands/workspace/index.ts:42` |
| `workspace rename <workspace-id> [title]` | `--reset`: Clear the title and revert to the branch or directory name | Set a workspace's user-visible title; `setWorkspaceTitle` | W; workspaceRenameSchema; 0/1 | `packages/cli/src/commands/workspace/index.ts:49` |
| `workspace archive <workspace-id>` | — | Archive a workspace and everything it owns; `archiveWorkspace` | W; workspaceArchiveSchema; 0/1 | `packages/cli/src/commands/workspace/index.ts:61` |

## CLI worktree

Hidden legacy group retained for compatibility (`cli.ts:164`); planned removal after 2027-01-17. Uses managed-worktree RPCs, not the newer workspace.create flow.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `worktree ls ` (hidden) | `--cwd <path>`: Repository directory (default: current) | List Paseo-managed git worktrees; `fetchAgents, getPaseoWorktreeList` | W; worktreeLsSchema; 0/1 | `packages/cli/src/commands/worktree/index.ts:11` |
| `worktree create ` (hidden) | `--mode <mode>`: Creation mode: branch-off, checkout-branch, or checkout-pr<br>`--new-branch <name>`: New branch name (--mode branch-off)<br>`--base <ref>`: Base ref for new branch (--mode branch-off, defaults to repo default)<br>`--branch <name>`: Existing branch to check out (--mode checkout-branch)<br>`--pr-number <n>`: Pull request number (--mode checkout-pr)<br>`--cwd <path>`: Repository directory (default: current) | Create a Paseo-managed git worktree; `createPaseoWorktree` | W; createSchema; 0/1 | `packages/cli/src/commands/worktree/index.ts:18` |
| `worktree archive <name>` (hidden) | `--cwd <path>`: Repository directory (default: current) | Archive a worktree (removes worktree and associated branch); `getPaseoWorktreeList, archivePaseoWorktree` | W; archiveSchema; 0/1 | `packages/cli/src/commands/worktree/index.ts:33` |

## CLI script

Configured `paseo.json` scripts run under the daemon supervisor. Select one workspace by --workspace or --cwd/current directory; explicit workspace and cwd conflict. List/start/stop use status-rich RPCs (lifecycle, health, port/proxy URLs, terminal ID).

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `script ls ` | `--workspace <workspace-id>`: Workspace ID (required when a directory has multiple workspaces)<br>`--cwd <path>`: Workspace directory (default: current directory) | List configured workspace scripts; `fetchWorkspaces, listWorkspaceScripts` | W; workspaceScriptSchema; 0/1 | `packages/cli/src/commands/script/index.ts:20` |
| `script start <name>` | `--workspace <workspace-id>`: Workspace ID (required when a directory has multiple workspaces)<br>`--cwd <path>`: Workspace directory (default: current directory) | Start a configured workspace script; `fetchWorkspaces, startWorkspaceScriptWithStatus` | W; workspaceScriptSchema; 0/1 | `packages/cli/src/commands/script/index.ts:26` |
| `script stop <name>` | `--workspace <workspace-id>`: Workspace ID (required when a directory has multiple workspaces)<br>`--cwd <path>`: Workspace directory (default: current directory) | Stop a running workspace script; `fetchWorkspaces, stopWorkspaceScript` | W; workspaceScriptSchema; 0/1 | `packages/cli/src/commands/script/index.ts:32` |

## CLI terminal

Workspace-owned terminals use the public typed SDK. ID/prefix/name resolution lists terminals first; names must be unambiguous. Capture defaults to stripping ANSI; --scrollback sets the capture start to earliest retained history. Send-keys expands named keys unless --literal; writing is not a shell-completion acknowledgement.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `terminal ls ` | `new Option("--all", "List terminals across all workspaces").conflicts(["cwd", "workspace"])`<br>`--workspace <id>`: Workspace ID<br>`--cwd <path>`: Working directory | List terminals; `client.terminals.list` | W; terminalSchema; 0/1 | `packages/cli/src/commands/terminal/index.ts:17` |
| `terminal create ` | `--workspace <id>`: Workspace ID<br>`--cwd <path>`: Working directory<br>`--name <name>`: Terminal name | Create a terminal; `client.workspaces.open, client.terminals.create` | W; terminalSchema; 0/1 | `packages/cli/src/commands/terminal/index.ts:28` |
| `terminal kill <terminal-id>` | — | Kill a terminal; `client.terminals.list, client.terminals.ref(resolvedId).kill, client.terminals.ref` | W; terminalKillSchema; 0/1 | `packages/cli/src/commands/terminal/index.ts:37` |
| `terminal capture <terminal-id>` | `--start <n>`: Capture start line<br>`--end <n>`: Capture end line<br>`-S, --scrollback`: Capture from the beginning of scrollback<br>`--ansi`: Preserve ANSI escape codes | Capture terminal output; `client.terminals.list, client.terminals.ref(resolvedId).capture, client.terminals.ref` | R: captured text or --json result; failure 1 | `packages/cli/src/commands/terminal/index.ts:44` |
| `terminal send-keys <terminal-id> <keys...>` | `-l, --literal`: Send raw keys without interpreting special tokens | Send keys to a terminal; `terminals.list/ref → handle.sendKeys/write → sendTerminalInput` | R: silent by default; --json {terminalId,keysSent}; failure 1 | `packages/cli/src/commands/terminal/index.ts:56` |

## CLI schedule

Create chooses a cadence (`--every` cron-compatible preset or `--cron`) and new-agent target; hidden `--target` is legacy. The hidden --target still accepts an arbitrary existing agent ID, new-agent, or self; self requires PASEO_AGENT_ID. Provider/mode/thinking options are rejected for existing-agent targets. Heartbeat is the canonical caller-agent surface (`packages/cli/src/commands/schedule/shared.ts:102`). Update can clear limits with negated options. Inspect/log/pause/resume/run-once first resolve schedule IDs using shared inspection.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `schedule create <prompt>` | `--every <duration>`: Cron-compatible cadence preset (for example: 5m, 1h)<br>`--cron <expr>`: Cron cadence expression<br>`--timezone <iana>`: IANA time zone for cron cadence (default: UTC)<br>`--name <name>`: Optional schedule name<br>`new Option("--target <target>", "Legacy schedule target").hideHelp()`<br>`--provider <provider>`: Agent provider, or provider/model (e.g. codex or codex/gpt-5.4)<br>`--mode <mode>`: Provider-specific mode (e.g. claude bypassPermissions, opencode build)<br>`--thinking <id>`: Thinking option ID for new-agent runs<br>`--cwd <path>`: Working directory (default: current; required with --host)<br>`--run-now`: Fire one immediate run on creation<br>`--max-runs <n>`: Maximum number of runs<br>`--expires-in <duration>`: Time to live for the schedule | Create a schedule; `scheduleCreate` | W; scheduleSchema; 0/1 | `packages/cli/src/commands/schedule/index.ts:17` |
| `schedule ls ` | — | List schedules; `scheduleList` | W; scheduleSchema; 0/1 | `packages/cli/src/commands/schedule/index.ts:42` |
| `schedule inspect <id>` | — | Inspect a schedule; `scheduleInspect` | W; createScheduleInspectSchema(payload.schedule); 0/1 | `packages/cli/src/commands/schedule/index.ts:46` |
| `schedule logs <id>` | — | Show recent schedule run logs; `scheduleInspect, scheduleLogs` | W; scheduleLogSchema; 0/1 | `packages/cli/src/commands/schedule/index.ts:50` |
| `schedule pause <id>` | — | Pause a schedule; `scheduleInspect, schedulePause` | W; scheduleSchema; 0/1 | `packages/cli/src/commands/schedule/index.ts:57` |
| `schedule resume <id>` | — | Resume a paused schedule; `scheduleInspect, scheduleResume` | W; scheduleSchema; 0/1 | `packages/cli/src/commands/schedule/index.ts:61` |
| `schedule delete <id>` | — | Delete a schedule; `scheduleDelete` | W; scheduleDeleteSchema; 0/1 | `packages/cli/src/commands/schedule/index.ts:68` |
| `schedule run-once <id>` | — | Manually trigger a single run of a schedule without affecting cadence; `scheduleInspect, scheduleRunOnce` | W; scheduleSchema; 0/1 | `packages/cli/src/commands/schedule/index.ts:72` |
| `schedule update <id>` | `--every <duration>`: Cron-compatible cadence preset (for example: 5m, 1h)<br>`--cron <expr>`: Switch to cron cadence expression<br>`--timezone <iana>`: IANA time zone for cron cadence (requires --cron)<br>`--name <name>`: Rename the schedule (empty string clears the name)<br>`--prompt <text>`: Replace the schedule prompt<br>`--provider <provider>`: New agent provider, or provider/model (only for new-agent target)<br>`--model <model>`: New agent model (only for new-agent target)<br>`--mode <mode>`: New agent provider mode (only for new-agent target)<br>`--cwd <path>`: New working directory (only for new-agent target)<br>`--max-runs <n>`: Set or change maximum number of runs<br>`--no-max-runs`: Clear the max-runs limit<br>`--expires-in <duration>`: Set or change time to live for the schedule<br>`--no-expires-in`: Clear the expiration | Update an existing schedule in place; `scheduleInspect, scheduleUpdate` | W; createScheduleInspectSchema(payload.schedule); 0/1 | `packages/cli/src/commands/schedule/index.ts:79` |

## CLI heartbeat

All three commands require `PASEO_AGENT_ID`. Update/delete inspect ownership first: target must be that agent. These are schedules with `target: {type:"agent",agentId}`; the CLI has an update operation even though some tool surfaces expose only create/delete. Cron has five fields; timezone defaults through the shared cadence schema to UTC.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `heartbeat create <prompt>` | `--cron <expr>` **required**: Five-field cron cadence<br>`--timezone <iana>`: IANA time zone<br>`--name <name>`: Heartbeat name<br>`--max-runs <n>`: Maximum number of runs<br>`--expires-in <duration>`: Time to live | Create a recurring prompt for this agent; `scheduleCreate` | W; scheduleSchema; 0/1 | `packages/cli/src/commands/heartbeat/index.ts:157` |
| `heartbeat update <id>` | `--cron <expr>` **required**: Five-field cron cadence<br>`--timezone <iana>`: IANA time zone | Change a heartbeat cron cadence; `scheduleInspect, scheduleUpdate` | W; scheduleSchema; 0/1 | `packages/cli/src/commands/heartbeat/index.ts:168` |
| `heartbeat delete <id>` | — | Delete a heartbeat; `scheduleInspect, scheduleDelete` | W; heartbeatDeleteSchema; 0/1 | `packages/cli/src/commands/heartbeat/index.ts:176` |

## CLI permit

Lists pending agent permissions. Allow/deny require request ID unless `--all`; allow parses optional modified-input JSON. Deny optionally interrupts after the response. Both use the response-and-wait client path so resolution can be acknowledged.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `permit ls ` | — | List all pending permissions; `fetchAgents` | W; permitLsSchema; 0/1 | `packages/cli/src/commands/permit/index.ts:11` |
| `permit allow <agent> [req_id]` | `--all`: Allow all pending permissions for this agent<br>`--input <json>`: Modified input parameters (JSON) | Allow a permission request; `fetchAgent, respondToPermissionAndWait` | W: PermissionResponseItem[] (requestId,agentId,agentShortId,name,result); 0/1 | `packages/cli/src/commands/permit/index.ts:15` |
| `permit deny <agent> [req_id]` | `--all`: Deny all pending permissions for this agent<br>`--message <msg>`: Denial reason message<br>`--interrupt`: Stop agent after denial | Deny a permission request; `fetchAgent, respondToPermissionAndWait, cancelAgent (with --interrupt)` | W: PermissionResponseItem[] (requestId,agentId,agentShortId,name,result); 0/1 | `packages/cli/src/commands/permit/index.ts:25` |

## CLI provider

Provider availability/models/diagnostics are discovered by the daemon. No provider mode-setting or feature-setting command exists here beyond agent mode/update. CLI models optionally expands thinking options.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `provider ls ` | — | List available providers and status; `getProvidersSnapshot` | W; providerLsSchema; 0/1 | `packages/cli/src/commands/provider/index.ts:11` |
| `provider models <provider>` | `--thinking`: Include thinking option IDs for each model | List models for a provider; `listProviderModels` | W; options.thinking ? providerModelsWithThinkingSchema : providerModelsSchema; 0/1 | `packages/cli/src/commands/provider/index.ts:15` |
| `provider diagnostic <provider>` | — | Show provider installation, environment, and availability diagnostics; `getProviderDiagnostic` | W; providerDiagnosticSchema; 0/1 | `packages/cli/src/commands/provider/index.ts:23` |

## CLI usage

`usage` and `usage ls` are the same action. Requires `usageSources`; --agent selects associated account quotas, not agent token usage. Inspect uses an exact opaque report ID, first discovers known accounts, and refreshes that selected account if requested.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `usage ls ` | `--refresh`: Refresh usage instead of using the host cache<br>`--agent <id>`: Show account quota associated with this agent (not agent token consumption) | List account quotas and balances; `listUsageReports` | W: UsageReportEntry[] via usageSchema; table/JSON/YAML; 0/1 | `packages/cli/src/commands/usage/index.ts:80` |
| `usage inspect <report-id>` | `--refresh`: Refresh the selected report instead of using the host cache | Show a complete account usage report; `listUsageReports` | W; usageSchema; 0/1 | `packages/cli/src/commands/usage/index.ts:83` |
| `usage ` | `--refresh`: Refresh usage instead of using the host cache<br>`--agent <id>`: Show account quota associated with this agent (not agent token consumption) | List account quotas and balances; `listUsageReports` | W: UsageReportEntry[] via usageSchema; table/JSON/YAML; 0/1 | `packages/cli/src/commands/usage/index.ts:79` |

## CLI plugin

Plugin code is trusted and unsandboxed. `init` only scaffolds locally. Install accepts registry, directory, Git or npm references. Update previews changes; structured/noninteractive output requires explicit consent (`--yes`) for application. Per-plugin update failures set exit code 1. `status` is hidden; `add` aliases install. Reload/enable/disable are dynamic loop registrations.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `plugin init <directory>` | `--id <id>`: Manifest plugin ID (defaults to the directory name) | Create a typecheckable local plugin; `No daemon client method; local operation` | W; scaffoldSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:212` |
| `plugin ls [id]` | — | List configured plugins; `listPlugins` | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:219` |
| `plugin status [id]` (hidden) | — | ; `listPlugins` | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:222` |
| `plugin logs <id>` | — | Show recent plugin output; `getPluginLogs` | W; pluginLogsSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:225` |
| `plugin install <source>` | `--id <id>`: Runtime plugin ID (defaults to paseo-plugin.json id)<br>`--ref <ref>`: Git branch, tag, or commit<br>`--path <path>`: Legacy form of the :plugin/path source suffix | Trust and install a plugin from a registry, directory, Git repository, or npm package; `installPluginSource` | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:228` |
| `plugin update [id]` | `--all`: Review all configured plugins<br>`--check`: Show available updates without installing<br>`--yes`: Apply displayed updates without asking<br>`--ref <ref>`: Apply a Git branch, tag, or commit without asking<br>`--version <version>`: Apply an npm version, tag, or range without asking | Review and update installed plugins; `previewPluginUpdates, applyPluginUpdates` | W pluginUpdateSchema; 1 if any outcome=error, otherwise 0 | `packages/cli/src/commands/plugin/index.ts:243` |
| `plugin reload <id>` | — | `${action} a plugin`; `reloadPlugin` | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:255` |
| `plugin enable <id>` | — | `${action} a plugin`; `enablePlugin` | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:255` |
| `plugin disable <id>` | — | `${action} a plugin`; `disablePlugin` | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:255` |
| `plugin remove <id>` | — | Remove plugin configuration; `listPlugins, removePlugin` | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:263` |
| `plugin add <source>` (alias) | `--id <id>`: Runtime plugin ID (defaults to paseo-plugin.json id)<br>`--ref <ref>`: Git branch, tag, or commit<br>`--path <path>`: Legacy form of the :plugin/path source suffix | Trust and install a plugin from a registry, directory, Git repository, or npm package; `installPluginSource` | W; pluginSchema; 0/1 | `packages/cli/src/commands/plugin/index.ts:228` |

## CLI hub

Hub HTTP authentication/configuration APIs are a separate transport from daemon session RPCs. CLI credentials and daemon authority are distinct. Login/init can continue guided setup; `init` is interactive and cancellation returns normally. Export writes configuration files; deploy validates, previews, then installs on Hub (see exact flags). Hub credential values are not copied into this reference.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `hub connect [origin]` | `--api-key <secret>`: Organization API key<br>`--permission <permission...>`: Grant daemon permission during connection | Enroll this daemon with a Paseo Hub; `connectHub + HubHttpClient/register authority` | W: HubRow[] via hubStatusResult; table or JSON/YAML; 0/1 | `packages/cli/src/commands/hub/connect.ts:61` |
| `hub deploy ` | `-p, --project <slug>`: Deploy a legacy bundle to this project slug<br>`--hub <origin>`: Paseo Hub origin<br>`--api-key <secret>`: Organization API key<br>`--dry-run`: Validate without installing or activating | Deploy .paseo organization triggers or a legacy project bundle; `HubHttpClient.validateConfiguration/installConfiguration (or trigger equivalents)` | W; triggerValidationSchema, triggerResultSchema, validationSchema, resultSchema; 0/1 | `packages/cli/src/commands/hub/deploy.ts:249` |
| `hub disconnect ` | `--force`: Remove local authority without notifying the Hub | ; `getHubStatus, disconnectHub` | W: HubRow[] plus possible warning; table or JSON/YAML; 0/1 | `packages/cli/src/commands/hub/disconnect.ts:43` |
| `hub export [directory]` | `--hub <origin>`: Paseo Hub origin<br>`--api-key <secret>`: Organization API key<br>`--force`: Replace trigger files with different contents | Export active Hub triggers as one YAML file per trigger; `Hub HTTP export API; no daemon RPC` | W; ; 0/1 | `packages/cli/src/commands/hub/export.ts:101` |
| `hub status ` | — | ; `getHubStatus` | W: HubRow[] via hubStatusResult; table or JSON/YAML; 0/1 | `packages/cli/src/commands/hub/index.ts:73` |
| `hub init ` | — | Create and optionally deploy a safe starter Hub trigger; `Hub HTTP guided init + connectHub` | R: Clack prompts/progress; cancel 0, failure exitCode=1 | `packages/cli/src/commands/hub/init.ts:91` |
| `hub login [origin]` | — | Log in to a Paseo Hub for CLI access; `Hub HTTP CLI-login flow; optional daemon connect` | W; ; 0/1 | `packages/cli/src/commands/hub/login.ts:59` |
| `hub logout ` | `--disconnect-daemon`: Also disconnect a daemon related to the same Hub<br>`--force`: Remove daemon authority without notifying the Hub | Remove the active stored Hub CLI login; `getHubStatus/disconnectHub when selected; credential store logout` | W; ; 0/1 | `packages/cli/src/commands/hub/logout.ts:93` |
| `hub projects ` | `--hub <origin>`: Paseo Hub origin<br>`--api-key <secret>`: Organization API key | List projects for the authenticated Hub organization; `HubHttpClient.listProjects` | W; ; 0/1 | `packages/cli/src/commands/hub/projects.ts:77` |

## CLI hub permissions

Permission changes read current daemon Hub status and update its authority grants. `list` is the only read action; grant/revoke share the same runner with an operation discriminant.

| Command / arguments | Flags and defaults | Action / client methods | Output / exits | Registration |
|---|---|---|---|---|
| `hub permissions list ` | — | ; `getHubStatus` | W; ; 0/1 | `packages/cli/src/commands/hub/permissions.ts:82` |
| `hub permissions grant <permission>` | — | ; `getHubStatus, updateHubPermissions` | W; ; 0/1 | `packages/cli/src/commands/hub/permissions.ts:90` |
| `hub permissions revoke <permission>` | — | ; `getHubStatus, updateHubPermissions` | W; ; 0/1 | `packages/cli/src/commands/hub/permissions.ts:90` |

## CLI speech placeholder and discovery-only groups

`packages/cli/src/commands/speech/index.ts:3` returns `new Command("speech").description("Speech commands")` with **no subcommands, arguments, options or action**. Speech model management exists in the protocol/client but is not registered in the CLI. The `agent`, `daemon`, `host`, `hub`, `hub permissions`, `permit`, `plugin`, `project`, `provider`, `schedule`, `heartbeat`, `script`, `terminal`, `workspace`, hidden `worktree`, and nested `daemon config` group nodes are discovery/help surfaces. They do not themselves call client methods. `usage` is the exception: it has its own list action as well as ls/inspect.

`daemon start` and `daemon run` register removed flags solely to reject them with REMOVED_LAUNCH_OPTION: --port, --listen, --relay, --no-relay, --relay-use-tls, --no-mcp, --no-inject-mcp, --web-ui, --no-web-ui, --hostnames, --allowed-hosts, --foreground. Persist these in daemon config instead, or use daemon run for foreground deployment (`commands/daemon/local-daemon.ts:80`). They are accepted by the parser but are not functioning launch settings.

## Protocol contract and scope

Canonical unions: `packages/protocol/src/messages.ts:3283` (inbound) and `messages.ts:7033` (outbound). **219 C→S session variants + 238 S→C session variants = 457 distinct session discriminators.** Tables list every union member separately so response payloads and pushed events keep their exact fields. Browser automation is reverse RPC: its request travels S→C and response C→S. Request/response naming is intentionally mixed (snake_case, dot namespaces, slash RPC); preserve wire spellings.

Field contracts use exact compact **Zod notation** to retain types, optionality, defaults, validation and nested discriminants: `z.string()` = string; `z.number()` = number; `z.boolean()` = boolean; `.optional()` = omitted allowed; `.nullable()` = explicit null allowed; `z.array(T)` = T[]; `z.record(K,V)` = map; `z.literal`/`z.enum` = exact allowed values. Every referenced protocol schema is expanded in the shared-type dictionary later in this document; this avoids silently dropping validation refinements in a hand-translated TypeScript type. A handler entry is a dispatch or emission site, not a claim that its entire downstream service was runtime-tested. All C/S directions refer to a daemon session; routed connections carry the same protocol.

## Protocol: Connected hosts and routing

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `hosts.list.request`<br>`HostsListRequestSchema`<br>`packages/protocol/src/hosts.ts:39` | C→S | hosts.list.response; Connected hosts and routing | `z.object({ type: z.literal("hosts.list.request"), requestId: z.string(), observe: z.boolean().optional(), })` | `packages/server/src/server/connected-hosts/session.ts:99` `execute` (dispatch) |
| `hosts.add.request`<br>`HostsAddRequestSchema`<br>`packages/protocol/src/hosts.ts:44` | C→S | hosts.add.response; Connected hosts and routing | `z.object({ type: z.literal("hosts.add.request"), requestId: z.string(), input: HostAddSchema, })` | `packages/server/src/server/connected-hosts/session.ts:101` `execute` (dispatch) |
| `hosts.update.request`<br>`HostsUpdateRequestSchema`<br>`packages/protocol/src/hosts.ts:49` | C→S | hosts.update.response; Connected hosts and routing | `z.object({ type: z.literal("hosts.update.request"), requestId: z.string(), registrationId: z.string(), patch: HostUpdateSchema, })` | `packages/server/src/server/connected-hosts/session.ts:110` `execute` (dispatch) |
| `hosts.remove.request`<br>`HostsRemoveRequestSchema`<br>`packages/protocol/src/hosts.ts:55` | C→S | hosts.remove.response; Connected hosts and routing | `z.object({ type: z.literal("hosts.remove.request"), requestId: z.string(), registrationId: z.string(), })` | `packages/server/src/server/connected-hosts/session.ts:119` `execute` (dispatch) |
| `hosts.link.open.request`<br>`HostsLinkOpenRequestSchema`<br>`packages/protocol/src/hosts.ts:115` | C→S | hosts.link.open.response; Connected hosts and routing | `z.object({ type: z.literal("hosts.link.open.request"), requestId: z.string(), channelId: z.string().min(1).max(256), })` | `packages/server/src/server/connected-hosts/link.ts:78` `handles` (dispatch)<br>`packages/server/src/server/connected-hosts/link.ts:86` `dispatch` (dispatch) |
| `hosts.route.open.request`<br>`HostsRouteOpenRequestSchema`<br>`packages/protocol/src/hosts.ts:60` | C→S | hosts.route.open.response; Connected hosts and routing | `z.object({ type: z.literal("hosts.route.open.request"), requestId: z.string(), serverId: z.string(), scopeId: z.string().min(1).max(256), })` | `packages/server/src/server/connected-hosts/session.ts:123` `execute` (dispatch) |
| `hosts.route.frame.request`<br>`HostsRouteFrameRequestSchema`<br>`packages/protocol/src/hosts.ts:66` | C→S | hosts.route.frame (asynchronously routed frame; no per-send acknowledgement); Connected hosts and routing | `z.object({ type: z.literal("hosts.route.frame.request"), routeId: z.string(), data: z.string().max(100 * 1024 * 1024), binary: z.literal(false), })` | `packages/server/src/server/connected-hosts/link.ts:79` `handles` (dispatch)<br>`packages/server/src/server/connected-hosts/link.ts:187` `dispatch` (dispatch) |
| `hosts.route.close.request`<br>`HostsRouteCloseRequestSchema`<br>`packages/protocol/src/hosts.ts:72` | C→S | hosts.route.closed (channel closure; no separate close response); Connected hosts and routing | `z.object({ type: z.literal("hosts.route.close.request"), routeId: z.string().optional(), scopeId: z.string().optional(), })` | `packages/server/src/server/connected-hosts/link.ts:80` `handles` (dispatch)<br>`packages/server/src/server/connected-hosts/link.ts:191` `dispatch` (dispatch) |
| `hosts.list.response`<br>`HostsListResponseSchema`<br>`packages/protocol/src/hosts.ts:77` | S→C | hosts.list.request; Connected hosts and routing | `z.object({ type: z.literal("hosts.list.response"), payload: z.object({ requestId: z.string(), directory: HostDirectorySchema, subscriptionId: z.string().optional(), }), })` | `packages/server/src/server/connected-hosts/session.ts:147` `list` (emit) |
| `hosts.changed`<br>`HostsChangedSchema`<br>`packages/protocol/src/hosts.ts:85` | S→C | push / shared status or error channel; Connected hosts and routing | `z.object({ type: z.literal("hosts.changed"), payload: z.object({ subscriptionId: z.string(), directory: HostDirectorySchema }), })` | `packages/server/src/server/connected-hosts/session.ts:159` (emit) |
| `hosts.add.response`<br>`HostsAddResponseSchema`<br>`packages/protocol/src/hosts.ts:89` | S→C | hosts.add.request; Connected hosts and routing | `z.object({ type: z.literal("hosts.add.response"), payload: z.object({ requestId: z.string(), host: HostSummarySchema }), })` | `packages/server/src/server/connected-hosts/session.ts:103` `execute` (emit) |
| `hosts.update.response`<br>`HostsUpdateResponseSchema`<br>`packages/protocol/src/hosts.ts:93` | S→C | hosts.update.request; Connected hosts and routing | `z.object({ type: z.literal("hosts.update.response"), payload: z.object({ requestId: z.string(), host: HostSummarySchema }), })` | `packages/server/src/server/connected-hosts/session.ts:112` `execute` (emit) |
| `hosts.remove.response`<br>`HostsRemoveResponseSchema`<br>`packages/protocol/src/hosts.ts:97` | S→C | hosts.remove.request; Connected hosts and routing | `z.object({ type: z.literal("hosts.remove.response"), payload: z.object({ requestId: z.string() }), })` | `packages/server/src/server/connected-hosts/session.ts:121` `execute` (emit) |
| `hosts.link.open.response`<br>`HostsLinkOpenResponseSchema`<br>`packages/protocol/src/hosts.ts:120` | S→C | hosts.link.open.request; Connected hosts and routing | `z.object({ type: z.literal("hosts.link.open.response"), payload: z.object({ requestId: z.string(), routeId: z.string() }), })` | `packages/server/src/server/connected-hosts/link.ts:178` `dispatch` (emit) |
| `hosts.route.open.response`<br>`HostsRouteOpenResponseSchema`<br>`packages/protocol/src/hosts.ts:101` | S→C | hosts.route.open.request; Connected hosts and routing | `z.object({ type: z.literal("hosts.route.open.response"), payload: z.object({ requestId: z.string(), routeId: z.string() }), })` | `packages/server/src/server/connected-hosts/session.ts:244` `open` (emit) |
| `hosts.route.frame`<br>`HostsRouteFrameSchema`<br>`packages/protocol/src/hosts.ts:105` | S→C | push / shared status or error channel; Connected hosts and routing | `z.object({ type: z.literal("hosts.route.frame"), payload: z.object({ subscriptionId: z.string(), data: z.string(), binary: z.literal(false) }), })` | `packages/server/src/server/connected-hosts/link.ts:118` (emit)<br>`packages/server/src/server/connected-hosts/session.ts:213` (emit) |
| `hosts.route.closed`<br>`HostsRouteClosedSchema`<br>`packages/protocol/src/hosts.ts:109` | S→C | push / shared status or error channel; Connected hosts and routing | `z.object({ type: z.literal("hosts.route.closed"), payload: z.object({ subscriptionId: z.string(), code: z.string(), message: z.string() }), })` | `packages/server/src/server/connected-hosts/link.ts:146` (emit)<br>`packages/server/src/server/connected-hosts/session.ts:187` (emit) |

## Protocol: Browser host and automation

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `browser.host.register.request`<br>`BrowserHostRegisterRequestSchema`<br>`packages/protocol/src/messages.ts:3262` | C→S | browser.host.register.response; Browser host and automation | `z.object({ type: z.literal("browser.host.register.request"), requestId: z.string(), hostKind: z.string().min(1), supportedCommands: z.array(z.enum(BROWSER_AUTOMATION_COMMAND_NAMES)).min(1), })` | `packages/server/src/server/session.ts:2420` `dispatchSubscriptionMessage` (dispatch) |
| `browser.automation.execute.response`<br>`BrowserAutomationExecuteResponseSchema`<br>`packages/protocol/src/browser-automation/rpc-schemas.ts:510` | C→S | browser.automation.execute.request; Browser host and automation | `z.object({ type: z.literal("browser.automation.execute.response"), payload: z.discriminatedUnion("ok", [ z.object({ requestId: z.string().min(1), ok: z.literal(true), result: BrowserAutomationResultSchema, dialogs: z.array(BrowserAutomationDialogEventSchema).optional(), }), z.object({ requestId: z.string().min(1), ok: z.literal(false), error: BrowserAutomationErrorSchema, dialogs: z.array(BrowserAutomationDialogEventSchema).optional(), }), ]), })` | `packages/server/src/server/session.ts:2421` `dispatchSubscriptionMessage` (dispatch) |
| `browser.host.register.response`<br>`BrowserHostRegisterResponseSchema`<br>`packages/protocol/src/messages.ts:3268` | S→C | browser.host.register.request; Browser host and automation | `z.object({ type: z.literal("browser.host.register.response"), payload: z.object({ requestId: z.string(), subscriptionId: z.string() }), })` | `packages/server/src/server/session.ts:2367` `registerBrowserHost` (emit) |
| `browser.automation.execute.request`<br>`BrowserAutomationExecuteRequestSchema`<br>`packages/protocol/src/browser-automation/rpc-schemas.ts:498` | S→C | browser.automation.execute.response; Browser host and automation | `z .object({ type: z.literal("browser.automation.execute.request"), subscriptionId: z.string().optional(), requestId: z.string().min(1), agentId: z.string().min(1).optional(), cwd: z.string().min(1).optional(), workspaceId: z.string().min(1).optional(), command: BrowserAutomationCommandSchema, }) .strict()` | `packages/server/src/server/browser-tools/broker.ts:117` `execute` (emit) |

## Protocol: Connection, daemon lifecycle and diagnostics

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `subscription.release.request`<br>`SubscriptionReleaseRequestSchema`<br>`packages/protocol/src/messages.ts:3273` | C→S | subscription.release.response; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("subscription.release.request"), requestId: z.string(), subscriptionId: z.string(), })` | `packages/server/src/server/session.ts:2429` `dispatchSubscriptionMessage` (dispatch) |
| `session.events.set_subscription.request`<br>`SessionEventsSetSubscriptionRequestSchema`<br>`packages/protocol/src/messages.ts:3251` | C→S | session.events.set_subscription.response; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("session.events.set_subscription.request"), requestId: z.string(), events: z.array(SessionEventSubscriptionSchema), notifications: z.boolean().optional(), })` | `packages/server/src/server/session.ts:2820` `dispatchAgentTimelineMessage` (dispatch) |
| `daemon.get_status.request`<br>`DaemonGetStatusRequestSchema`<br>`packages/protocol/src/messages.ts:1446` | C→S | daemon.get_status.response; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("daemon.get_status.request"), requestId: z.string(), })` | `packages/server/src/server/session.ts:2957` `dispatchAgentConfigMessage` (dispatch) |
| `daemon.get_pairing_offer.request`<br>`DaemonGetPairingOfferRequestSchema`<br>`packages/protocol/src/messages.ts:1451` | C→S | daemon.get_pairing_offer.response; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("daemon.get_pairing_offer.request"), requestId: z.string(), })` | `packages/server/src/server/session.ts:2959` `dispatchAgentConfigMessage` (dispatch) |
| `daemon.config.reload.request`<br>`DaemonConfigReloadRequestSchema`<br>`packages/protocol/src/messages.ts:1456` | C→S | daemon.config.reload.response; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("daemon.config.reload.request"), requestId: z.string(), })` | `packages/server/src/server/session.ts:2961` `dispatchAgentConfigMessage` (dispatch) |
| `diagnostics.request`<br>`DiagnosticsRequestSchema`<br>`packages/protocol/src/messages.ts:1484` | C→S | diagnostics.response; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("diagnostics.request"), requestId: z.string(), })` | `packages/server/src/server/session.ts:2968` `dispatchAgentConfigMessage` (dispatch) |
| `get_daemon_config_request`<br>`GetDaemonConfigRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1684` | C→S | get_daemon_config_response; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("get_daemon_config_request"), requestId: z.string(), })` | `packages/server/src/server/session.ts:2951` `dispatchAgentConfigMessage` (dispatch) |
| `set_daemon_config_request`<br>`SetDaemonConfigRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1689` | C→S | set_daemon_config_response; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("set_daemon_config_request"), requestId: z.string(), config: MutableDaemonConfigPatchSchema, })` | `packages/server/src/server/session.ts:2972` `dispatchAgentConfigMessage` (dispatch) |
| `shutdown_server_request`<br>`ShutdownServerRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1923` | C→S | status (shutdown_requested); Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("shutdown_server_request"), requestId: z.string(), keepTerminals: z.boolean().optional(), })` | `packages/server/src/server/session.ts:2757` `dispatchVoiceAndControlMessage` (dispatch)<br>`packages/server/src/server/websocket-server.ts:3280` `getControlRpcLogInfo` (dispatch) |
| `restart_server_request`<br>`RestartServerRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1914` | C→S | status (restart_requested); Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("restart_server_request"), reason: z.string().optional(), requestId: z.string(), killTerminals: z.boolean().optional(), })` | `packages/server/src/server/session.ts:2753` `dispatchVoiceAndControlMessage` (dispatch)<br>`packages/server/src/server/websocket-server.ts:3287` `getControlRpcLogInfo` (dispatch) |
| `daemon.update.request`<br>`DaemonUpdateRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1931` | C→S | daemon.update.response; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("daemon.update.request"), requestId: z.string(), })` | `packages/server/src/server/session.ts:2970` `dispatchAgentConfigMessage` (dispatch)<br>`packages/server/src/server/websocket-server.ts:3295` `getControlRpcLogInfo` (dispatch) |
| `ping`<br>`PingMessageSchema`<br>`packages/protocol/src/messages.ts:2971` | C→S | pong; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("ping"), requestId: z.string(), clientSentAt: z.number().int().optional(), })` | `packages/server/src/server/session.ts:2764` `dispatchVoiceAndControlMessage` (dispatch)<br>`packages/server/src/server/websocket-server.ts:2580` `handleRawMessage` (dispatch) |
| `subscription.release.response`<br>`SubscriptionReleaseResponseSchema`<br>`packages/protocol/src/messages.ts:3278` | S→C | subscription.release.request; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("subscription.release.response"), payload: z.object({ requestId: z.string(), subscriptionId: z.string() }), })` | `packages/server/src/server/session.ts:2432` (emit) |
| `session.events.set_subscription.response`<br>`SessionEventsSetSubscriptionResponseSchema`<br>`packages/protocol/src/messages.ts:3257` | S→C | session.events.set_subscription.request; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("session.events.set_subscription.response"), payload: z.object({ requestId: z.string(), subscriptionId: z.string().optional() }), })` | `packages/server/src/server/session.ts:2834` `dispatchAgentTimelineMessage` (emit) |
| `activity_log`<br>`ActivityLogMessageSchema`<br>`packages/protocol/src/messages.ts:3521` | S→C | push / shared status or error channel; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("activity_log"), payload: ActivityLogPayloadSchema, })` | `packages/server/src/server/session/agent-config/agent-config-session.ts:211` `applyConfigChange` (emit)<br>`packages/server/src/server/session/voice/voice-session.ts:611` (emit) |
| `status`<br>`StatusMessageSchema`<br>`packages/protocol/src/messages.ts:3890` | S→C | push / shared status or error channel; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("status"), payload: z .object({ status: z.string(), }) .passthrough(), })` | `packages/server/src/server/session/owned-subscriptions/index.ts:232` `protocolFailure` (emit)<br>`packages/server/src/server/session.ts:2728` (emit) |
| `pong`<br>`PongMessageSchema`<br>`packages/protocol/src/messages.ts:3899` | S→C | ping; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("pong"), payload: z.object({ requestId: z.string(), clientSentAt: z.number().int().optional(), serverReceivedAt: z.number().int(), serverSentAt: z.number().int(), }), })` | `packages/server/src/server/session.ts:2767` `dispatchVoiceAndControlMessage` (emit)<br>`packages/server/src/server/websocket-server.ts:2586` `handleRawMessage` (emit) |
| `rpc_error`<br>`RpcErrorMessageSchema`<br>`packages/protocol/src/messages.ts:3909` | S→C | push / shared status or error channel; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("rpc_error"), payload: z.object({ requestId: z.string(), requestType: z.string().optional(), error: z.string(), code: z.string().optional(), }), })` | `packages/server/src/server/session/daemon/daemon-self-update-session-controller.ts:98` `handleDaemonUpdateRequest` (emit)<br>`packages/server/src/server/session/daemon/daemon-session.ts:168` `handleHubRelationshipRequest` (emit) |
| `daemon.get_status.response`<br>`DaemonGetStatusResponseSchema`<br>`packages/protocol/src/messages.ts:5155` | S→C | daemon.get_status.request; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("daemon.get_status.response"), payload: z .object({ requestId: z.string(), serverId: z.string(), version: z.string().nullable().optional(), pid: z.number(), nodePath: z.string(), startedAt: z.string().nullable().optional(), listen: z.string().nullable(), relay: z .object({ enabled: z.boolean(), endpoint: z.string(), publicEndpoint: z.string(), useTls: z.boolean(), publicUseTls: z.boolean(), }) .nullable() .optional(), providers: z.array(z.object({ provider: z.string(), available: z.boolean(), error: z.string().nullable().optional(), })), terminalCleanupUnconfirmed: z .array(z.object({ pid: z.number(), role: z.string(), reason: z.string(), detail: z.string(), })) .optional(), terminalsUnavailable: z.string().optional(), }) .passthrough(), })` | `packages/server/src/server/session/daemon/daemon-session.ts:196` `handleGetStatusRequest` (emit) |
| `daemon.get_pairing_offer.response`<br>`DaemonGetPairingOfferResponseSchema`<br>`packages/protocol/src/messages.ts:5239` | S→C | daemon.get_pairing_offer.request; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("daemon.get_pairing_offer.response"), payload: z .object({ requestId: z.string(), url: z.string(), qr: z.string().nullable().optional(), relayEnabled: z.boolean(), }) .passthrough(), })` | `packages/server/src/server/session/daemon/daemon-session.ts:250` `handleGetPairingOfferRequest` (emit) |
| `daemon.config.reload.response`<br>`DaemonConfigReloadResponseSchema`<br>`packages/protocol/src/messages.ts:5251` | S→C | daemon.config.reload.request; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("daemon.config.reload.response"), payload: z .object({ requestId: z.string(), appliedPaths: z.array(z.string()), restartRequiredPaths: z.array(z.string()), overrideControlledPaths: z.array(z.string()), }) .passthrough(), })` | `packages/server/src/server/session/daemon/daemon-session.ts:276` `handleConfigReloadRequest` (emit) |
| `diagnostics.response`<br>`DiagnosticsResponseSchema`<br>`packages/protocol/src/messages.ts:5263` | S→C | diagnostics.request; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("diagnostics.response"), payload: z .object({ requestId: z.string(), diagnostic: z.string(), }) .passthrough(), })` | `packages/server/src/server/session/daemon/daemon-session.ts:311` `handleDiagnosticsRequest` (emit) |
| `get_daemon_config_response`<br>`GetDaemonConfigResponseMessageSchema`<br>`packages/protocol/src/messages.ts:5145` | S→C | get_daemon_config_request; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("get_daemon_config_response"), payload: z .object({ requestId: z.string(), config: MutableDaemonConfigSchema, }) .passthrough(), })` | `packages/server/src/server/session.ts:2953` `dispatchAgentConfigMessage` (emit) |
| `set_daemon_config_response`<br>`SetDaemonConfigResponseMessageSchema`<br>`packages/protocol/src/messages.ts:5273` | S→C | set_daemon_config_request; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("set_daemon_config_response"), payload: z .object({ requestId: z.string(), config: MutableDaemonConfigSchema, }) .passthrough(), })` | `packages/server/src/server/session.ts:2975` (emit) |
| `daemon.update.progress`<br>`DaemonUpdateProgressMessageSchema`<br>`packages/protocol/src/messages.ts:6715` | S→C | daemon.update.request (intermediate progress); Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("daemon.update.progress"), payload: z.object({ requestId: z.string(), phase: z.enum(["starting", "downloading", "installing", "complete"]), }), })` | `packages/server/src/server/session/daemon/daemon-self-update-session-controller.ts:121` `emitProgress` (emit) |
| `daemon.update.response`<br>`DaemonUpdateResponseSchema`<br>`packages/protocol/src/messages.ts:6702` | S→C | daemon.update.request; Connection, daemon lifecycle and diagnostics | `z.object({ type: z.literal("daemon.update.response"), payload: z.object({ requestId: z.string(), success: z.boolean(), error: z.string().nullable(), previousVersion: z.string().nullable(), newVersion: z.string().nullable(), }), })` | `packages/server/src/server/session/daemon/daemon-self-update-session-controller.ts:137` `emitResponse` (emit) |

## Protocol: Hub management and execution

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `hub.execution.agent.create.request`<br>`HubExecutionAgentCreateRequestSchema`<br>`packages/protocol/src/messages.ts:3157` | C→S | hub.execution.agent.create.response; Hub management and execution | `z.object({ type: z.literal("hub.execution.agent.create.request"), requestId: z.string(), executionId: z.string(), provider: z.string(), cwd: z.string(), prompt: z.string(), workspaceId: z.string().optional(), model: z.string().optional(), modeId: z.string().optional(), thinkingOptionId: z.string().optional(), featureValues: z.record(z.string(), z.unknown()).optional(), providerOptions: ProviderOptionsSchema.optional(), toolPolicy: ToolPolicySchema.optional(), env: z.record(z.string(), z.string()).optional(), mcpServers: z.record(z.string(), McpServerConfigSchema).optional(), worktree: CreateAgentWorktreeTargetSchema.optional(), })` | `packages/server/src/server/session.ts:2866` `dispatchHubExecutionMessage` (dispatch) |
| `hub.execution.agent.validate.request`<br>`HubExecutionAgentValidateRequestSchema`<br>`packages/protocol/src/messages.ts:3179` | C→S | hub.execution.agent.validate.response; Hub management and execution | `z.object({ type: z.literal("hub.execution.agent.validate.request"), requestId: z.string(), provider: z.string(), model: z.string().optional(), modeId: z.string().optional(), thinkingOptionId: z.string().optional(), providerOptions: ProviderOptionsSchema.optional(), })` | `packages/server/src/server/session.ts:2869` `dispatchHubExecutionMessage` (dispatch) |
| `hub.execution.control.request`<br>`HubExecutionControlRequestSchema`<br>`packages/protocol/src/messages.ts:3221` | C→S | hub.execution.control.response; Hub management and execution | `z.object({ type: z.literal("hub.execution.control.request"), requestId: z.string(), executionId: z.string(), action: HubExecutionControlActionSchema, })` | `packages/server/src/server/session.ts:2872` `dispatchHubExecutionMessage` (dispatch) |
| `hub.management.daemon.connect.request`<br>`HubManagementDaemonConnectRequestSchema`<br>`packages/protocol/src/messages.ts:1461` | C→S | hub.management.daemon.connect.response; Hub management and execution | `z.object({ type: z.literal("hub.management.daemon.connect.request"), requestId: z.string(), hubUrl: z.string(), token: z.string(), permissions: z.array(DaemonPermissionSchema).default([]), })` | `packages/server/src/server/session/daemon/daemon-session.ts:130` `handleHubRelationshipRequest` (dispatch)<br>`packages/server/src/server/session.ts:2963` `dispatchAgentConfigMessage` (dispatch) |
| `hub.management.daemon.get_status.request`<br>`HubManagementDaemonGetStatusRequestSchema`<br>`packages/protocol/src/messages.ts:1468` | C→S | hub.management.daemon.get_status.response; Hub management and execution | `z.object({ type: z.literal("hub.management.daemon.get_status.request"), requestId: z.string(), })` | `packages/server/src/server/session.ts:2964` `dispatchAgentConfigMessage` (dispatch) |
| `hub.management.daemon.disconnect.request`<br>`HubManagementDaemonDisconnectRequestSchema`<br>`packages/protocol/src/messages.ts:1472` | C→S | hub.management.daemon.disconnect.response; Hub management and execution | `z.object({ type: z.literal("hub.management.daemon.disconnect.request"), requestId: z.string(), force: z.boolean().optional(), })` | `packages/server/src/server/session/daemon/daemon-session.ts:153` `handleHubRelationshipRequest` (dispatch)<br>`packages/server/src/server/session.ts:2965` `dispatchAgentConfigMessage` (dispatch) |
| `hub.management.daemon.permissions.update.request`<br>`HubManagementDaemonPermissionsUpdateRequestSchema`<br>`packages/protocol/src/messages.ts:1477` | C→S | hub.management.daemon.permissions.update.response; Hub management and execution | `z.object({ type: z.literal("hub.management.daemon.permissions.update.request"), requestId: z.string(), grant: z.array(DaemonPermissionSchema).default([]), revoke: z.array(DaemonPermissionSchema).default([]), })` | `packages/server/src/server/session/daemon/daemon-session.ts:142` `handleHubRelationshipRequest` (dispatch)<br>`packages/server/src/server/session.ts:2966` `dispatchAgentConfigMessage` (dispatch) |
| `hub.execution.agent.create.response`<br>`HubExecutionAgentCreateResponseSchema`<br>`packages/protocol/src/messages.ts:6723` | S→C | hub.execution.agent.create.request; Hub management and execution | `z.object({ type: z.literal("hub.execution.agent.create.response"), payload: z.object({ requestId: z.string(), executionId: z.string(), agentId: z.string().nullable(), agent: AgentSnapshotPayloadSchema.nullable(), success: z.boolean(), toolPolicyApplied: z.literal(true).optional(), error: HubExecutionAgentCreateErrorSchema.nullable(), }), })` | `packages/server/src/server/hub/execution-controller.ts:179` `createAgentWithResponse` (emit) |
| `hub.execution.agent.validate.response`<br>`HubExecutionAgentValidateResponseSchema`<br>`packages/protocol/src/messages.ts:6745` | S→C | hub.execution.agent.validate.request; Hub management and execution | `z.object({ type: z.literal("hub.execution.agent.validate.response"), payload: z.object({ requestId: z.string(), valid: z.boolean(), issues: z.array(HubExecutionAgentValidationIssueSchema), error: z.string().nullable(), }), })` | `packages/server/src/server/hub/execution-controller.ts:99` `validateAgentWithResponse` (emit) |
| `hub.execution.control.response`<br>`HubExecutionControlResponseSchema`<br>`packages/protocol/src/messages.ts:6755` | S→C | hub.execution.control.request; Hub management and execution | `z.object({ type: z.literal("hub.execution.control.response"), payload: z.object({ requestId: z.string(), executionId: z.string(), action: HubExecutionControlActionSchema, success: z.boolean(), error: z.string().nullable(), }), })` | `packages/server/src/server/hub/execution-controller.ts:134` `controlExecutionWithResponse` (emit) |
| `hub.execution.agent.update`<br>`HubExecutionAgentUpdateSchema`<br>`packages/protocol/src/messages.ts:6766` | S→C | push / shared status or error channel; Hub management and execution | `z.object({ type: z.literal("hub.execution.agent.update"), payload: z.object({ subscriptionId: z.string().optional(), executionId: z.string(), agentId: z.string(), agent: AgentSnapshotPayloadSchema, }), })` | `packages/server/src/server/hub/execution-controller.ts:210` `sendOwnedEvent` (emit) |
| `hub.execution.agent.stream`<br>`HubExecutionAgentStreamSchema`<br>`packages/protocol/src/messages.ts:6776` | S→C | push / shared status or error channel; Hub management and execution | `z.object({ type: z.literal("hub.execution.agent.stream"), payload: z.object({ subscriptionId: z.string().optional(), executionId: z.string(), agentId: z.string(), event: AgentStreamEventPayloadSchema, }), })` | `packages/server/src/server/hub/execution-controller.ts:220` `sendOwnedEvent` (emit) |
| `hub.management.daemon.connect.response`<br>`HubManagementDaemonConnectResponseSchema`<br>`packages/protocol/src/messages.ts:5218` | S→C | hub.management.daemon.connect.request; Hub management and execution | `z.object({ type: z.literal("hub.management.daemon.connect.response"), payload: z.object({ requestId: z.string(), status: HubRelationshipStatusSchema }), })` | `packages/server/src/server/session/daemon/daemon-session.ts:137` `handleHubRelationshipRequest` (emit) |
| `hub.management.daemon.get_status.response`<br>`HubManagementDaemonGetStatusResponseSchema`<br>`packages/protocol/src/messages.ts:5222` | S→C | hub.management.daemon.get_status.request; Hub management and execution | `z.object({ type: z.literal("hub.management.daemon.get_status.response"), payload: z.object({ requestId: z.string(), status: HubRelationshipStatusSchema }), })` | `packages/server/src/server/session/daemon/daemon-session.ts:162` `handleHubRelationshipRequest` (emit) |
| `hub.management.daemon.disconnect.response`<br>`HubManagementDaemonDisconnectResponseSchema`<br>`packages/protocol/src/messages.ts:5226` | S→C | hub.management.daemon.disconnect.request; Hub management and execution | `z.object({ type: z.literal("hub.management.daemon.disconnect.response"), payload: z.object({ requestId: z.string(), status: HubRelationshipStatusSchema, warning: z.string().optional(), }), })` | `packages/server/src/server/session/daemon/daemon-session.ts:156` `handleHubRelationshipRequest` (emit) |
| `hub.management.daemon.permissions.update.response`<br>`HubManagementDaemonPermissionsUpdateResponseSchema`<br>`packages/protocol/src/messages.ts:5234` | S→C | hub.management.daemon.permissions.update.request; Hub management and execution | `z.object({ type: z.literal("hub.management.daemon.permissions.update.response"), payload: z.object({ requestId: z.string(), status: HubRelationshipStatusSchema }), })` | `packages/server/src/server/session/daemon/daemon-session.ts:148` `handleHubRelationshipRequest` (emit) |

## Protocol: Voice, dictation and local speech models

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `voice_audio_chunk`<br>`VoiceAudioChunkMessageSchema`<br>`packages/protocol/src/messages.ts:942` | C→S | voice_input_state / transcription_result / audio_output; Voice, dictation and local speech models | `z.object({ type: z.literal("voice_audio_chunk"), audio: z.string(), format: z.string(), isLast: z.boolean(), })` | `packages/server/src/server/session/voice/index.ts:105` `dispatch` (dispatch)<br>`packages/server/src/server/session.ts:2744` `dispatchVoiceAndControlMessage` (dispatch) |
| `abort_request`<br>`AbortRequestMessageSchema`<br>`packages/protocol/src/messages.ts:949` | C→S | no separate acknowledgement; Voice, dictation and local speech models | `z.object({ type: z.literal("abort_request"), })` | `packages/server/src/server/session/voice/index.ts:107` `dispatch` (dispatch)<br>`packages/server/src/server/session.ts:2745` `dispatchVoiceAndControlMessage` (dispatch) |
| `audio_played`<br>`AudioPlayedMessageSchema`<br>`packages/protocol/src/messages.ts:953` | C→S | no separate acknowledgement; Voice, dictation and local speech models | `z.object({ type: z.literal("audio_played"), id: z.string(), })` | `packages/server/src/server/session/voice/index.ts:109` `dispatch` (dispatch)<br>`packages/server/src/server/session.ts:2746` `dispatchVoiceAndControlMessage` (dispatch) |
| `set_voice_mode`<br>`SetVoiceModeMessageSchema`<br>`packages/protocol/src/messages.ts:1098` | C→S | set_voice_mode_response; Voice, dictation and local speech models | `z.object({ type: z.literal("set_voice_mode"), enabled: z.boolean(), agentId: z.string().optional(), requestId: z.string().optional(), })` | `packages/server/src/server/session/voice/index.ts:111` `dispatch` (dispatch)<br>`packages/server/src/server/session.ts:2747` `dispatchVoiceAndControlMessage` (dispatch) |
| `dictation_stream_start`<br>`DictationStreamStartMessageSchema`<br>`packages/protocol/src/messages.ts:1713` | C→S | dictation_stream_ack / dictation_stream_error; Voice, dictation and local speech models | `z.object({ type: z.literal("dictation_stream_start"), dictationId: z.string(), format: z.string(), })` | `packages/server/src/server/session/voice/index.ts:113` `dispatch` (dispatch)<br>`packages/server/src/server/session.ts:2748` `dispatchVoiceAndControlMessage` (dispatch) |
| `dictation_stream_chunk`<br>`DictationStreamChunkMessageSchema`<br>`packages/protocol/src/messages.ts:1719` | C→S | dictation_stream_partial / dictation_stream_error; Voice, dictation and local speech models | `z.object({ type: z.literal("dictation_stream_chunk"), dictationId: z.string(), seq: z.number().int().nonnegative(), audio: z.string(), format: z.string(), })` | `packages/server/src/server/session/voice/index.ts:115` `dispatch` (dispatch)<br>`packages/server/src/server/session.ts:2749` `dispatchVoiceAndControlMessage` (dispatch) |
| `dictation_stream_finish`<br>`DictationStreamFinishMessageSchema`<br>`packages/protocol/src/messages.ts:1727` | C→S | dictation_stream_finish_accepted → dictation_stream_final/error; Voice, dictation and local speech models | `z.object({ type: z.literal("dictation_stream_finish"), dictationId: z.string(), finalSeq: z.number().int().nonnegative(), })` | `packages/server/src/server/session/voice/index.ts:122` `dispatch` (dispatch)<br>`packages/server/src/server/session.ts:2750` `dispatchVoiceAndControlMessage` (dispatch) |
| `dictation_stream_cancel`<br>`DictationStreamCancelMessageSchema`<br>`packages/protocol/src/messages.ts:1733` | C→S | no separate acknowledgement; Voice, dictation and local speech models | `z.object({ type: z.literal("dictation_stream_cancel"), dictationId: z.string(), })` | `packages/server/src/server/session/voice/index.ts:124` `dispatch` (dispatch)<br>`packages/server/src/server/session.ts:2751` `dispatchVoiceAndControlMessage` (dispatch) |
| `speech.local_models.list.request`<br>`SpeechLocalModelsListRequestSchema`<br>`packages/protocol/src/speech/rpc-schemas.ts:161` | C→S | speech.local_models.list.response; Voice, dictation and local speech models | `z.object({ type: z.literal("speech.local_models.list.request"), requestId: z.string(), })` | `packages/server/src/server/session/speech/speech-models-session.ts:24` `dispatch` (dispatch) |
| `speech.local_models.download.request`<br>`SpeechLocalModelsDownloadRequestSchema`<br>`packages/protocol/src/speech/rpc-schemas.ts:166` | C→S | speech.local_models.download.response; Voice, dictation and local speech models | `z.object({ type: z.literal("speech.local_models.download.request"), requestId: z.string(), modelId: z.string(), })` | `packages/server/src/server/session/speech/speech-models-session.ts:26` `dispatch` (dispatch) |
| `speech.local_models.delete.request`<br>`SpeechLocalModelsDeleteRequestSchema`<br>`packages/protocol/src/speech/rpc-schemas.ts:172` | C→S | speech.local_models.delete.response; Voice, dictation and local speech models | `z.object({ type: z.literal("speech.local_models.delete.request"), requestId: z.string(), modelId: z.string(), })` | `packages/server/src/server/session/speech/speech-models-session.ts:29` `dispatch` (dispatch) |
| `audio_output`<br>`AudioOutputMessageSchema`<br>`packages/protocol/src/messages.ts:3533` | S→C | push / shared status or error channel; Voice, dictation and local speech models | `z.object({ type: z.literal("audio_output"), payload: z.object({ audio: z.string(), format: z.string(), id: z.string(), isVoiceMode: z.boolean(), groupId: z.string().optional(), chunkIndex: z.number().int().nonnegative().optional(), isLastChunk: z.boolean().optional(), }), })` | `packages/server/src/server/agent/tts-manager.ts:402` `emitPreparedSegment` (emit) |
| `transcription_result`<br>`TranscriptionResultMessageSchema`<br>`packages/protocol/src/messages.ts:3546` | S→C | push / shared status or error channel; Voice, dictation and local speech models | `z.object({ type: z.literal("transcription_result"), payload: z.object({ text: z.string(), language: z.string().optional(), duration: z.number().optional(), requestId: z.string(), avgLogprob: z.number().optional(), isLowConfidence: z.boolean().optional(), byteLength: z.number().optional(), format: z.string().optional(), debugRecordingPath: z.string().optional(), }), })` | `packages/server/src/server/session/voice/voice-session.ts:970` `handleTranscriptionResultPayload` (emit) |
| `voice_input_state`<br>`VoiceInputStateMessageSchema`<br>`packages/protocol/src/messages.ts:3561` | S→C | push / shared status or error channel; Voice, dictation and local speech models | `z.object({ type: z.literal("voice_input_state"), payload: z.object({ isSpeaking: z.boolean(), }), })` | `packages/server/src/server/session/voice/voice-session.ts:594` (emit)<br>`packages/server/src/server/session/voice/voice-session.ts:679` `handleVoiceSpeechStopped` (emit) |
| `dictation_stream_ack`<br>`DictationStreamAckMessageSchema`<br>`packages/protocol/src/messages.ts:3568` | S→C | dictation_stream_start (streamId); Voice, dictation and local speech models | `z.object({ type: z.literal("dictation_stream_ack"), payload: z.object({ dictationId: z.string(), ackSeq: z.number().int(), }), })` | `packages/server/src/server/dictation/dictation-stream-manager.ts:491` `emitDictationAck` (emit) |
| `dictation_stream_finish_accepted`<br>`DictationStreamFinishAcceptedMessageSchema`<br>`packages/protocol/src/messages.ts:3576` | S→C | dictation_stream_finish (streamId); Voice, dictation and local speech models | `z.object({ type: z.literal("dictation_stream_finish_accepted"), payload: z.object({ dictationId: z.string(), timeoutMs: z.number().int().positive(), }), })` | `packages/server/src/server/dictation/dictation-stream-manager.ts:465` `handleFinish` (emit) |
| `dictation_stream_partial`<br>`DictationStreamPartialMessageSchema`<br>`packages/protocol/src/messages.ts:3584` | S→C | dictation_stream_chunk (streamId); Voice, dictation and local speech models | `z.object({ type: z.literal("dictation_stream_partial"), payload: z.object({ dictationId: z.string(), text: z.string(), }), })` | `packages/server/src/server/dictation/dictation-stream-manager.ts:495` `emitDictationPartial` (emit) |
| `dictation_stream_final`<br>`DictationStreamFinalMessageSchema`<br>`packages/protocol/src/messages.ts:3592` | S→C | dictation_stream_finish (streamId); Voice, dictation and local speech models | `z.object({ type: z.literal("dictation_stream_final"), payload: z.object({ dictationId: z.string(), text: z.string(), debugRecordingPath: z.string().optional(), }), })` | `packages/server/src/server/dictation/dictation-stream-manager.ts:750` (emit) |
| `dictation_stream_error`<br>`DictationStreamErrorMessageSchema`<br>`packages/protocol/src/messages.ts:3601` | S→C | dictation stream operations (streamId); Voice, dictation and local speech models | `z.object({ type: z.literal("dictation_stream_error"), payload: z.object({ dictationId: z.string(), error: z.string(), retryable: z.boolean(), reasonCode: z.string().optional(), missingModelIds: z.array(z.string()).optional(), debugRecordingPath: z.string().optional(), }), })` | `packages/server/src/server/session/voice/voice-session.ts:282` `handleDictationStreamStart` (emit)<br>`packages/server/src/server/dictation/dictation-stream-manager.ts:533` `failDictationStream` (emit) |
| `set_voice_mode_response`<br>`SetVoiceModeResponseMessageSchema`<br>`packages/protocol/src/messages.ts:2222` | S→C | set_voice_mode; Voice, dictation and local speech models | `z.object({ type: z.literal("set_voice_mode_response"), payload: z.object({ requestId: z.string(), enabled: z.boolean(), agentId: z.string().nullable(), accepted: z.boolean(), error: z.string().nullable(), reasonCode: z.string().optional(), retryable: z.boolean().optional(), missingModelIds: z.array(z.string()).optional(), }), })` | `packages/server/src/server/session/voice/voice-session.ts:421` `handleSetVoiceMode` (emit) |
| `speech.local_models.list.response`<br>`SpeechLocalModelsListResponseSchema`<br>`packages/protocol/src/speech/rpc-schemas.ts:178` | S→C | speech.local_models.list.request; Voice, dictation and local speech models | `z.object({ type: z.literal("speech.local_models.list.response"), payload: z.object({ requestId: z.string(), models: z.array(LocalSpeechModelSchema), error: z.string().nullable(), }), })` | `packages/server/src/server/session/speech/speech-models-session.ts:42` `handleList` (emit) |
| `speech.local_models.download.response`<br>`SpeechLocalModelsDownloadResponseSchema`<br>`packages/protocol/src/speech/rpc-schemas.ts:188` | S→C | speech.local_models.download.request; Voice, dictation and local speech models | `z.object({ type: z.literal("speech.local_models.download.response"), payload: z.object({ requestId: z.string(), error: z.string().nullable(), }), })` | `packages/server/src/server/session/speech/speech-models-session.ts:59` `handleDownload` (emit) |
| `speech.local_models.delete.response`<br>`SpeechLocalModelsDeleteResponseSchema`<br>`packages/protocol/src/speech/rpc-schemas.ts:196` | S→C | speech.local_models.delete.request; Voice, dictation and local speech models | `z.object({ type: z.literal("speech.local_models.delete.response"), payload: z.object({ requestId: z.string(), error: z.string().nullable(), }), })` | `packages/server/src/server/session/speech/speech-models-session.ts:76` `handleDelete` (emit) |

## Protocol: Agents and runtime configuration

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `fetch_agents_request`<br>`FetchAgentsRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1291` | C→S | fetch_agents_response; Agents and runtime configuration | `z.object({ type: z.literal("fetch_agents_request"), requestId: z.string(), scope: z.enum(["active"]).optional(), includeBackground: z.boolean().optional(), filter: AgentDirectoryFilterSchema.optional(), sort: z .array(z.object({ key: z.enum(["status_priority", "created_at", "updated_at", "title"]), direction: z.enum(["asc", "desc"]), })) .optional(), page: z .object({ limit: z.number().int().positive().max(200), cursor: z.string().min(1).optional(), }) .optional(), subscribe: z .object({ subscriptionId: z.string().optional(), }) .optional(), sync: DirectorySyncRequestSchema.optional(), })` | `packages/server/src/server/session.ts:2896` `dispatchAgentLifecycleMessage` (dispatch)<br>`packages/server/src/server/session.ts:5559` `listFetchAgentsEntries` (dispatch) |
| `fetch_agent_history_request`<br>`FetchAgentHistoryRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1382` | C→S | fetch_agent_history_response; Agents and runtime configuration | `z.object({ type: z.literal("fetch_agent_history_request"), requestId: z.string(), filter: AgentDirectoryFilterSchema.optional(), search: z.string().optional(), sort: z .array(z.object({ key: z.enum(["status_priority", "created_at", "updated_at", "title"]), direction: z.enum(["asc", "desc"]), })) .optional(), page: z .object({ limit: z.number().int().positive().max(200), cursor: z.string().min(1).optional(), }) .optional(), })` | `packages/server/src/server/session.ts:369` `agentDirectorySearchQuery` (dispatch)<br>`packages/server/src/server/session.ts:2898` `dispatchAgentLifecycleMessage` (dispatch) |
| `fetch_agent_request`<br>`FetchAgentRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1415` | C→S | fetch_agent_response; Agents and runtime configuration | `z.object({ type: z.literal("fetch_agent_request"), requestId: z.string(), agentId: z.string(), })` | `packages/server/src/server/session.ts:2902` `dispatchAgentLifecycleMessage` (dispatch) |
| `delete_agent_request`<br>`DeleteAgentRequestMessageSchema`<br>`packages/protocol/src/messages.ts:970` | C→S | agent_deleted; Agents and runtime configuration | `z.object({ type: z.literal("delete_agent_request"), agentId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2904` `dispatchAgentLifecycleMessage` (dispatch) |
| `archive_agent_request`<br>`ArchiveAgentRequestMessageSchema`<br>`packages/protocol/src/messages.ts:976` | C→S | agent_archived; Agents and runtime configuration | `z.object({ type: z.literal("archive_agent_request"), agentId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2906` `dispatchAgentLifecycleMessage` (dispatch) |
| `close_items_request`<br>`CloseItemsRequestMessageSchema`<br>`packages/protocol/src/messages.ts:982` | C→S | close_items_response; Agents and runtime configuration | `z.object({ type: z.literal("close_items_request"), agentIds: z.array(z.string()).default([]), terminalIds: z.array(z.string()).default([]), requestId: z.string(), })` | `packages/server/src/server/session.ts:2908` `dispatchAgentLifecycleMessage` (dispatch) |
| `update_agent_request`<br>`UpdateAgentRequestMessageSchema`<br>`packages/protocol/src/messages.ts:989` | C→S | update_agent_response; Agents and runtime configuration | `z.object({ type: z.literal("update_agent_request"), agentId: z.string(), name: z.string().optional(), labels: z.record(z.string(), z.string()).optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2910` `dispatchAgentLifecycleMessage` (dispatch) |
| `send_agent_message_request`<br>`SendAgentMessageRequestSchema`<br>`packages/protocol/src/messages.ts:1422` | C→S | send_agent_message_response; Agents and runtime configuration | `z.object({ type: z.literal("send_agent_message_request"), requestId: z.string(), agentId: z.string(), text: z.string(), sourceAgentId: z.string().min(1).optional(), sourceAgentServerId: z.string().min(1).optional(), sessionMode: z.string().optional(), messageId: z.string().optional(), activeTurnBehavior: ActiveTurnBehaviorSchema.optional(), images: z.array(ImageAttachmentSchema).optional(), attachments: AgentAttachmentsSchema, })` | `packages/server/src/server/session.ts:2916` `dispatchAgentLifecycleMessage` (dispatch) |
| `wait_for_finish_request`<br>`WaitForFinishRequestSchema`<br>`packages/protocol/src/messages.ts:1438` | C→S | wait_for_finish_response; Agents and runtime configuration | `z.object({ type: z.literal("wait_for_finish_request"), requestId: z.string(), agentId: z.string(), timeoutMs: z.number().int().positive().optional(), })` | `packages/server/src/server/session.ts:2918` `dispatchAgentLifecycleMessage` (dispatch) |
| `create_agent_request`<br>`CreateAgentRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1773` | C→S | status (agent_created / agent_create_failed); Agents and runtime configuration | `z.object({ type: z.literal("create_agent_request"), idempotencyKey: z.string().min(1).max(512).optional(), config: AgentSessionConfigSchema, env: z.record(z.string(), z.string()).optional(), workspaceId: z.string().optional(), callerAgentId: z.string().optional(), worktreeName: z.string().optional(), initialPrompt: z.string().optional(), clientMessageId: z.string().optional(), outputSchema: z.record(z.string(), z.unknown()).optional(), images: z.array(ImageAttachmentSchema).optional(), attachments: AgentAttachmentsSchema, git: GitSetupOptionsSchema.optional(), worktree: CreateAgentWorktreeTargetSchema.optional(), autoArchive: z.boolean().optional(), internal: z.boolean().optional(), background: z.boolean().optional(), labels: z.record(z.string(), z.string()).default({}), requestId: z.string(), })` | `packages/server/src/server/session.ts:2920` `dispatchAgentLifecycleMessage` (dispatch) |
| `resume_agent_request`<br>`ResumeAgentRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1883` | C→S | status (agent_resumed); failures use rpc_error; Agents and runtime configuration | `z.object({ type: z.literal("resume_agent_request"), handle: AgentPersistenceHandleSchema, overrides: AgentSessionConfigSchema.partial().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2922` `dispatchAgentLifecycleMessage` (dispatch) |
| `import_agent_request`<br>`ImportAgentRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1890` | C→S | status (agent_resumed / agent_create_failed); Agents and runtime configuration | `z.object({ type: z.literal("import_agent_request"), provider: AgentProviderSchema.optional(), providerId: z.string().optional(), sessionId: z.string().optional(), providerHandleId: z.string().optional(), cwd: z.string().optional(), workspaceId: z.string().optional(), labels: z.record(z.string(), z.string()).optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2924` `dispatchAgentLifecycleMessage` (dispatch) |
| `refresh_agent_request`<br>`RefreshAgentRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1902` | C→S | status (agent_refreshed); failures use rpc_error; Agents and runtime configuration | `z.object({ type: z.literal("refresh_agent_request"), agentId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2926` `dispatchAgentLifecycleMessage` (dispatch) |
| `cancel_agent_request`<br>`CancelAgentRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1908` | C→S | cancel_agent_response; Agents and runtime configuration | `z.object({ type: z.literal("cancel_agent_request"), agentId: z.string(), requestId: z.string().optional(), })` | `packages/server/src/server/session.ts:2928` `dispatchAgentLifecycleMessage` (dispatch) |
| `set_agent_mode_request`<br>`SetAgentModeRequestMessageSchema`<br>`packages/protocol/src/messages.ts:2001` | C→S | set_agent_mode_response; Agents and runtime configuration | `z.object({ type: z.literal("set_agent_mode_request"), agentId: z.string(), modeId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2941` `dispatchAgentConfigMessage` (dispatch) |
| `set_agent_model_request`<br>`SetAgentModelRequestMessageSchema`<br>`packages/protocol/src/messages.ts:2021` | C→S | set_agent_model_response; Agents and runtime configuration | `z.object({ type: z.literal("set_agent_model_request"), agentId: z.string(), modelId: z.string().nullable(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2943` `dispatchAgentConfigMessage` (dispatch) |
| `set_agent_thinking_request`<br>`SetAgentThinkingRequestMessageSchema`<br>`packages/protocol/src/messages.ts:2033` | C→S | set_agent_thinking_response; Agents and runtime configuration | `z.object({ type: z.literal("set_agent_thinking_request"), agentId: z.string(), thinkingOptionId: z.string().nullable(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2947` `dispatchAgentConfigMessage` (dispatch) |
| `set_agent_feature_request`<br>`SetAgentFeatureRequestMessageSchema`<br>`packages/protocol/src/messages.ts:2045` | C→S | set_agent_feature_response; Agents and runtime configuration | `z.object({ type: z.literal("set_agent_feature_request"), agentId: z.string(), featureId: z.string(), value: z.unknown(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2945` `dispatchAgentConfigMessage` (dispatch) |
| `agent.config.apply.request`<br>`AgentConfigApplyRequestMessageSchema`<br>`packages/protocol/src/messages.ts:2078` | C→S | agent.config.apply.response; Agents and runtime configuration | `z.object({ type: z.literal("agent.config.apply.request"), agentId: z.string(), config: AgentConfigApplySchema, requestId: z.string(), })` | `packages/server/src/server/session.ts:2949` `dispatchAgentConfigMessage` (dispatch) |
| `agent.detach.request`<br>`AgentDetachRequestMessageSchema`<br>`packages/protocol/src/messages.ts:2090` | C→S | agent.detach.response; Agents and runtime configuration | `z.object({ type: z.literal("agent.detach.request"), agentId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2796` `dispatchAgentRelationshipMessage` (dispatch) |
| `agent.rewind.request`<br>`AgentRewindRequestMessageSchema`<br>`packages/protocol/src/messages.ts:2103` | C→S | agent.rewind.response; Agents and runtime configuration | `z.object({ type: z.literal("agent.rewind.request"), agentId: z.string(), messageId: z.string(), mode: AgentRewindModeSchema, requestId: z.string(), })` | `packages/server/src/server/session.ts:2787` `dispatchAgentRewindMessage` (dispatch) |
| `list_available_editors_request`<br>`LegacyListAvailableEditorsRequestSchema`<br>`packages/protocol/src/messages.ts:2645` | C→S | list_available_editors_response; Agents and runtime configuration | `z.object({ type: z.literal("list_available_editors_request"), requestId: z.string(), })` | `packages/server/src/server/session.ts:3075` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `open_in_editor_request`<br>`LegacyOpenInEditorRequestSchema`<br>`packages/protocol/src/messages.ts:2650` | C→S | open_in_editor_response; Agents and runtime configuration | `z.object({ type: z.literal("open_in_editor_request"), path: z.string(), editorId: z.string().trim().min(1), mode: z.enum(["open", "reveal"]).optional(), cwd: z.string().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3077` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `fs.entry.create.request`<br>`FileEntryCreateRequestSchema`<br>`packages/protocol/src/messages.ts:2895` | C→S | fs.entry.create.response; Agents and runtime configuration | `z.object({ type: z.literal("fs.entry.create.request"), cwd: z.string(), parentPath: z.string(), name: z.string(), kind: z.enum(["file", "directory"]), requestId: z.string(), })` | `packages/server/src/server/session.ts:3142` `dispatchWorkspaceFileMessage` (dispatch) |
| `fs.entry.rename.request`<br>`FileEntryRenameRequestSchema`<br>`packages/protocol/src/messages.ts:2904` | C→S | fs.entry.rename.response; Agents and runtime configuration | `z.object({ type: z.literal("fs.entry.rename.request"), cwd: z.string(), path: z.string(), name: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3144` `dispatchWorkspaceFileMessage` (dispatch) |
| `fs.entry.duplicate.request`<br>`FileEntryDuplicateRequestSchema`<br>`packages/protocol/src/messages.ts:2912` | C→S | fs.entry.duplicate.response; Agents and runtime configuration | `z.object({ type: z.literal("fs.entry.duplicate.request"), cwd: z.string(), path: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3146` `dispatchWorkspaceFileMessage` (dispatch) |
| `fs.entry.delete.request`<br>`FileEntryDeleteRequestSchema`<br>`packages/protocol/src/messages.ts:2919` | C→S | fs.entry.delete.response; Agents and runtime configuration | `z.object({ type: z.literal("fs.entry.delete.request"), cwd: z.string(), path: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3148` `dispatchWorkspaceFileMessage` (dispatch) |
| `list_commands_request`<br>`ListCommandsRequestSchema`<br>`packages/protocol/src/messages.ts:2992` | C→S | list_commands_response; Agents and runtime configuration | `z.object({ type: z.literal("list_commands_request"), agentId: z.string(), draftConfig: ListCommandsDraftConfigSchema.optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3246` `dispatchMiscMessage` (dispatch) |
| `assistant_chunk`<br>`AssistantChunkMessageSchema`<br>`packages/protocol/src/messages.ts:3526` | S→C | push / shared status or error channel; Agents and runtime configuration | `z.object({ type: z.literal("assistant_chunk"), payload: z.object({ chunk: z.string(), }), })` | not found: packages/server/src (non-test source) |
| `artifact`<br>`ArtifactMessageSchema`<br>`packages/protocol/src/messages.ts:4000` | S→C | push / shared status or error channel; Agents and runtime configuration | `z.object({ type: z.literal("artifact"), payload: z.object({ type: z.enum(["markdown", "diff", "image", "code"]), id: z.string(), title: z.string(), content: z.string(), isBase64: z.boolean(), }), })` | not found: packages/server/src (non-test source) |
| `agent_update`<br>`AgentUpdateMessageSchema`<br>`packages/protocol/src/messages.ts:4228` | S→C | push / shared status or error channel; Agents and runtime configuration | `z.object({ type: z.literal("agent_update"), payload: z.discriminatedUnion("kind", [ z.object({ subscriptionId: z.string().optional(), kind: z.literal("upsert"), agent: AgentSnapshotPayloadSchema, project: ProjectPlacementPayloadSchema.nullable().optional(), generation: z.string().optional(), seq: z.number().int().positive().optional(), }), z.object({ subscriptionId: z.string().optional(), kind: z.literal("remove"), agentId: z.string(), generation: z.string().optional(), seq: z.number().int().positive().optional(), }), ]), })` | `packages/server/src/server/session/agent-updates/agent-updates-service.ts:189` `bufferOrEmit` (emit)<br>`packages/server/src/server/session.ts:7717` `handleWorkspaceClearAttentionRequest` (emit) |
| `agent_status`<br>`AgentStatusMessageSchema`<br>`packages/protocol/src/messages.ts:4262` | S→C | push / shared status or error channel; Agents and runtime configuration | `z.object({ type: z.literal("agent_status"), payload: z.object({ agentId: z.string(), status: z.string(), info: AgentSnapshotPayloadSchema, }), })` | not found: packages/server/src (non-test source) |
| `fetch_agents_response`<br>`FetchAgentsResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4310` | S→C | fetch_agents_request; Agents and runtime configuration | `z.object({ type: z.literal("fetch_agents_response"), payload: z.object({ requestId: z.string(), subscriptionId: z.string().nullable().optional(), entries: z.array(AgentDirectoryResponseEntrySchema), pageInfo: AgentDirectoryPageInfoSchema, sync: DirectorySyncMetadataSchema.optional(), }), })` | `packages/server/src/server/session.ts:6331` `handleFetchAgents` (emit) |
| `fetch_agent_history_response`<br>`FetchAgentHistoryResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4321` | S→C | fetch_agent_history_request; Agents and runtime configuration | `z.object({ type: z.literal("fetch_agent_history_response"), payload: z.object({ requestId: z.string(), entries: z.array(AgentDirectoryResponseEntrySchema), pageInfo: AgentDirectoryPageInfoSchema, searchTruncated: z.boolean().optional(), }), })` | `packages/server/src/server/session.ts:6367` `handleFetchAgentHistory` (emit) |
| `list_available_editors_response`<br>`LegacyListAvailableEditorsResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4729` | S→C | list_available_editors_request; Agents and runtime configuration | `z.object({ type: z.literal("list_available_editors_response"), payload: z.object({ requestId: z.string(), editors: z.array(z.object({ id: z.string().trim().min(1), label: z.string(), })), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:7442` `handleLegacyListAvailableEditorsRequest` (emit) |
| `open_in_editor_response`<br>`LegacyOpenInEditorResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4743` | S→C | open_in_editor_request; Agents and runtime configuration | `z.object({ type: z.literal("open_in_editor_response"), payload: z.object({ requestId: z.string(), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:7455` `handleLegacyOpenInEditorRequest` (emit) |
| `fetch_agent_response`<br>`FetchAgentResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4761` | S→C | fetch_agent_request; Agents and runtime configuration | `z.object({ type: z.literal("fetch_agent_response"), payload: z.object({ requestId: z.string(), agent: AgentSnapshotPayloadSchema.nullable(), project: ProjectPlacementPayloadSchema.nullable().optional(), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:7833` `handleFetchAgent` (emit) |
| `cancel_agent_response`<br>`CancelAgentResponseMessageSchema`<br>`packages/protocol/src/messages.ts:5008` | S→C | cancel_agent_request; Agents and runtime configuration | `z.object({ type: z.literal("cancel_agent_response"), payload: z.object({ requestId: z.string(), agentId: z.string(), agent: AgentSnapshotPayloadSchema.nullable(), error: z.string().nullable().optional(), }), })` | `packages/server/src/server/session.ts:4850` `handleCancelAgentRequest` (emit) |
| `send_agent_message_response`<br>`SendAgentMessageResponseMessageSchema`<br>`packages/protocol/src/messages.ts:5123` | S→C | send_agent_message_request; Agents and runtime configuration | `z.object({ type: z.literal("send_agent_message_response"), payload: z.object({ requestId: z.string(), agentId: z.string(), accepted: z.boolean(), deliveryStatus: z.literal("queued").optional(), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:8310` `handleSendAgentMessageRequest` (emit) |
| `set_agent_mode_response`<br>`SetAgentModeResponseMessageSchema`<br>`packages/protocol/src/messages.ts:2016` | S→C | set_agent_mode_request; Agents and runtime configuration | `z.object({ type: z.literal("set_agent_mode_response"), payload: AgentActionResponsePayloadSchema, })` | `packages/server/src/server/session/agent-config/agent-config-session.ts:83` (emit) |
| `set_agent_model_response`<br>`SetAgentModelResponseMessageSchema`<br>`packages/protocol/src/messages.ts:2028` | S→C | set_agent_model_request; Agents and runtime configuration | `z.object({ type: z.literal("set_agent_model_response"), payload: AgentActionResponsePayloadSchema, })` | `packages/server/src/server/session/agent-config/agent-config-session.ts:101` (emit) |
| `set_agent_thinking_response`<br>`SetAgentThinkingResponseMessageSchema`<br>`packages/protocol/src/messages.ts:2040` | S→C | set_agent_thinking_request; Agents and runtime configuration | `z.object({ type: z.literal("set_agent_thinking_response"), payload: AgentActionResponsePayloadSchema, })` | `packages/server/src/server/session/agent-config/agent-config-session.ts:134` (emit) |
| `set_agent_feature_response`<br>`SetAgentFeatureResponseMessageSchema`<br>`packages/protocol/src/messages.ts:2053` | S→C | set_agent_feature_request; Agents and runtime configuration | `z.object({ type: z.literal("set_agent_feature_response"), payload: AgentActionResponsePayloadSchema, })` | `packages/server/src/server/session/agent-config/agent-config-session.ts:119` (emit) |
| `agent.config.apply.response`<br>`AgentConfigApplyResponseMessageSchema`<br>`packages/protocol/src/messages.ts:2085` | S→C | agent.config.apply.request; Agents and runtime configuration | `z.object({ type: z.literal("agent.config.apply.response"), payload: AgentActionResponsePayloadSchema, })` | `packages/server/src/server/session/agent-config/agent-config-session.ts:149` (emit) |
| `agent.detach.response`<br>`AgentDetachResponseMessageSchema`<br>`packages/protocol/src/messages.ts:2096` | S→C | agent.detach.request; Agents and runtime configuration | `z.object({ type: z.literal("agent.detach.response"), payload: AgentActionResponsePayloadSchema, })` | `packages/server/src/server/session.ts:3464` `handleDetachAgentRequest` (emit) |
| `agent.rewind.response`<br>`AgentRewindResponseMessageSchema`<br>`packages/protocol/src/messages.ts:2111` | S→C | agent.rewind.request; Agents and runtime configuration | `z.object({ type: z.literal("agent.rewind.response"), payload: z.object({ requestId: z.string(), agentId: z.string(), ok: z.boolean(), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:4891` `handleAgentRewindRequest` (emit) |
| `update_agent_response`<br>`UpdateAgentResponseMessageSchema`<br>`packages/protocol/src/messages.ts:2121` | S→C | update_agent_request; Agents and runtime configuration | `z.object({ type: z.literal("update_agent_response"), payload: AgentActionResponsePayloadSchema, })` | `packages/server/src/server/session.ts:3590` `handleUpdateAgentRequest` (emit) |
| `wait_for_finish_response`<br>`WaitForFinishResponseMessageSchema`<br>`packages/protocol/src/messages.ts:5134` | S→C | wait_for_finish_request; Agents and runtime configuration | `z.object({ type: z.literal("wait_for_finish_response"), payload: z.object({ requestId: z.string(), status: z.enum(["idle", "error", "permission", "timeout"]), final: AgentSnapshotPayloadSchema.nullable(), error: z.string().nullable(), lastMessage: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:8441` `handleWaitForFinish` (emit) |
| `agent_deleted`<br>`AgentDeletedMessageSchema`<br>`packages/protocol/src/messages.ts:5346` | S→C | delete_agent_request; Agents and runtime configuration | `z.object({ type: z.literal("agent_deleted"), payload: z.object({ agentId: z.string(), requestId: z.string(), }), })` | `packages/server/src/server/session.ts:3391` `handleDeleteAgentRequest` (emit) |
| `agent_archived`<br>`AgentArchivedMessageSchema`<br>`packages/protocol/src/messages.ts:5354` | S→C | archive_agent_request; Agents and runtime configuration | `z.object({ type: z.literal("agent_archived"), payload: z.object({ agentId: z.string(), archivedAt: z.string(), requestId: z.string(), }), })` | `packages/server/src/server/session.ts:3405` `handleArchiveAgentRequest` (emit) |
| `close_items_response`<br>`CloseItemsResponseSchema`<br>`packages/protocol/src/messages.ts:5373` | S→C | close_items_request; Agents and runtime configuration | `z.object({ type: z.literal("close_items_response"), payload: z.object({ agents: z.array(CloseItemsAgentResultSchema), terminals: z.array(CloseItemsTerminalResultSchema), requestId: z.string(), }), })` | `packages/server/src/server/session.ts:3521` `handleCloseItemsRequest` (emit) |
| `fs.entry.create.response`<br>`FileEntryCreateResponseSchema`<br>`packages/protocol/src/messages.ts:6211` | S→C | fs.entry.create.request; Agents and runtime configuration | `z.object({ type: z.literal("fs.entry.create.response"), payload: z.object({ cwd: z.string(), parentPath: z.string(), path: z.string().nullable(), success: z.boolean(), error: z.string().nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/files/workspace-files-session.ts:181` `handleFileEntryCreateRequest` (emit) |
| `fs.entry.rename.response`<br>`FileEntryRenameResponseSchema`<br>`packages/protocol/src/messages.ts:6223` | S→C | fs.entry.rename.request; Agents and runtime configuration | `z.object({ type: z.literal("fs.entry.rename.response"), payload: z.object({ cwd: z.string(), path: z.string(), renamedPath: z.string().nullable(), success: z.boolean(), error: z.string().nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/files/workspace-files-session.ts:200` `handleFileEntryRenameRequest` (emit) |
| `fs.entry.duplicate.response`<br>`FileEntryDuplicateResponseSchema`<br>`packages/protocol/src/messages.ts:6235` | S→C | fs.entry.duplicate.request; Agents and runtime configuration | `z.object({ type: z.literal("fs.entry.duplicate.response"), payload: z.object({ cwd: z.string(), path: z.string(), duplicatedPath: z.string().nullable(), success: z.boolean(), error: z.string().nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/files/workspace-files-session.ts:218` `handleFileEntryDuplicateRequest` (emit) |
| `fs.entry.delete.response`<br>`FileEntryDeleteResponseSchema`<br>`packages/protocol/src/messages.ts:6247` | S→C | fs.entry.delete.request; Agents and runtime configuration | `z.object({ type: z.literal("fs.entry.delete.response"), payload: z.object({ cwd: z.string(), path: z.string(), success: z.boolean(), error: z.string().nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/files/workspace-files-session.ts:236` `handleFileEntryDeleteRequest` (emit) |
| `list_commands_response`<br>`ListCommandsResponseSchema`<br>`packages/protocol/src/messages.ts:6539` | S→C | list_commands_request; Agents and runtime configuration | `z.object({ type: z.literal("list_commands_response"), payload: z.object({ agentId: z.string(), commands: z.array(AgentSlashCommandSchema), error: z.string().nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session.ts:5120` `handleListCommandsRequest` (emit) |

## Protocol: Providers, commands and usage

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `fetch_recent_provider_sessions_request`<br>`FetchRecentProviderSessionsRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1405` | C→S | fetch_recent_provider_sessions_response; Providers, commands and usage | `z.object({ type: z.literal("fetch_recent_provider_sessions_request"), requestId: z.string(), cwd: z.string().optional(), providers: z.array(z.string()).optional(), since: z.string().optional(), limit: z.number().int().positive().max(200).optional(), query: z.string().optional(), })` | `packages/server/src/server/session.ts:2900` `dispatchAgentLifecycleMessage` (dispatch) |
| `list_provider_models_request`<br>`ListProviderModelsRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1830` | C→S | list_provider_models_response; Providers, commands and usage | `z.object({ type: z.literal("list_provider_models_request"), provider: AgentProviderSchema, cwd: z.string().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3181` `dispatchProviderMessage` (dispatch) |
| `list_provider_modes_request`<br>`ListProviderModesRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1837` | C→S | list_provider_modes_response; Providers, commands and usage | `z.object({ type: z.literal("list_provider_modes_request"), provider: AgentProviderSchema, cwd: z.string().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3183` `dispatchProviderMessage` (dispatch) |
| `list_provider_features_request`<br>`ListProviderFeaturesRequestMessageSchema`<br>`packages/protocol/src/messages.ts:2986` | C→S | list_provider_features_response; Providers, commands and usage | `z.object({ type: z.literal("list_provider_features_request"), draftConfig: ListCommandsDraftConfigSchema, requestId: z.string(), })` | `packages/server/src/server/session.ts:3185` `dispatchProviderMessage` (dispatch) |
| `list_available_providers_request`<br>`ListAvailableProvidersRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1844` | C→S | list_available_providers_response; Providers, commands and usage | `z.object({ type: z.literal("list_available_providers_request"), requestId: z.string(), })` | `packages/server/src/server/session.ts:3187` `dispatchProviderMessage` (dispatch) |
| `get_providers_snapshot_request`<br>`GetProvidersSnapshotRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1849` | C→S | get_providers_snapshot_response; Providers, commands and usage | `z.object({ type: z.literal("get_providers_snapshot_request"), cwd: z.string().optional(), ifNoneMatch: z.string().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3189` `dispatchProviderMessage` (dispatch) |
| `refresh_providers_snapshot_request`<br>`RefreshProvidersSnapshotRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1857` | C→S | refresh_providers_snapshot_response; Providers, commands and usage | `z.object({ type: z.literal("refresh_providers_snapshot_request"), cwd: z.string().optional(), providers: z.array(AgentProviderSchema).optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3191` `dispatchProviderMessage` (dispatch) |
| `provider_diagnostic_request`<br>`ProviderDiagnosticRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1864` | C→S | provider_diagnostic_response; Providers, commands and usage | `z.object({ type: z.literal("provider_diagnostic_request"), provider: AgentProviderSchema, requestId: z.string(), })` | `packages/server/src/server/session.ts:3193` `dispatchProviderMessage` (dispatch) |
| `provider.usage.list.request`<br>`ProviderUsageListRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1870` | C→S | provider.usage.list.response; Providers, commands and usage | `z.object({ type: z.literal("provider.usage.list.request"), requestId: z.string(), })` | `packages/server/src/server/session.ts:3195` `dispatchProviderMessage` (dispatch) |
| `usage.list_reports.request`<br>`UsageListReportsRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1875` | C→S | usage.list_reports.response; Providers, commands and usage | `z.object({ type: z.literal("usage.list_reports.request"), agentId: z.string().optional(), requestId: z.string(), reportIds: z.array(z.string()).optional(), forceRefresh: z.boolean().optional(), })` | `packages/server/src/server/session.ts:3197` `dispatchProviderMessage` (dispatch) |
| `fetch_recent_provider_sessions_response`<br>`FetchRecentProviderSessionsResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4332` | S→C | fetch_recent_provider_sessions_request; Providers, commands and usage | `z.object({ type: z.literal("fetch_recent_provider_sessions_response"), payload: z.object({ requestId: z.string(), entries: z.array(RecentProviderSessionDescriptorPayloadSchema), filteredAlreadyImportedCount: z.number().int().nonnegative().optional(), providerErrors: z .array(z.object({ provider: z.string(), message: z.string(), })) .optional(), }), })` | `packages/server/src/server/session.ts:6400` `handleFetchRecentProviderSessions` (emit) |
| `list_provider_models_response`<br>`ListProviderModelsResponseMessageSchema`<br>`packages/protocol/src/messages.ts:6314` | S→C | list_provider_models_request; Providers, commands and usage | `z.object({ type: z.literal("list_provider_models_response"), payload: z.object({ provider: AgentProviderSchema, models: z.array(AgentModelDefinitionSchema).optional(), error: z.string().nullable().optional(), fetchedAt: z.string(), requestId: z.string(), }), })` | `packages/server/src/server/session/provider/provider-catalog-session.ts:198` `emitProviderDisabledResponse` (emit)<br>`packages/server/src/server/session/provider/provider-catalog-session.ts:214` `handleListProviderModelsRequest` (emit) |
| `list_provider_modes_response`<br>`ListProviderModesResponseMessageSchema`<br>`packages/protocol/src/messages.ts:6325` | S→C | list_provider_modes_request; Providers, commands and usage | `z.object({ type: z.literal("list_provider_modes_response"), payload: z.object({ provider: AgentProviderSchema, modes: z.array(AgentModeSchema).optional(), error: z.string().nullable().optional(), fetchedAt: z.string(), requestId: z.string(), }), })` | `packages/server/src/server/session/provider/provider-catalog-session.ts:200` `emitProviderDisabledResponse` (emit)<br>`packages/server/src/server/session/provider/provider-catalog-session.ts:269` `handleListProviderModesRequest` (emit) |
| `list_provider_features_response`<br>`ListProviderFeaturesResponseMessageSchema`<br>`packages/protocol/src/messages.ts:6336` | S→C | list_provider_features_request; Providers, commands and usage | `z.object({ type: z.literal("list_provider_features_response"), payload: z.object({ provider: AgentProviderSchema, features: z.array(AgentFeatureSchema).optional(), error: z.string().nullable().optional(), fetchedAt: z.string(), requestId: z.string(), }), })` | `packages/server/src/server/session/provider/provider-catalog-session.ts:362` `handleListProviderFeaturesRequest` (emit) |
| `list_available_providers_response`<br>`ListAvailableProvidersResponseSchema`<br>`packages/protocol/src/messages.ts:6353` | S→C | list_available_providers_request; Providers, commands and usage | `z.object({ type: z.literal("list_available_providers_response"), payload: z.object({ providers: z.array(ProviderAvailabilitySchema), error: z.string().nullable().optional(), fetchedAt: z.string(), requestId: z.string(), }), })` | `packages/server/src/server/session/provider/provider-catalog-session.ts:397` `handleListAvailableProvidersRequest` (emit) |
| `get_providers_snapshot_response`<br>`GetProvidersSnapshotResponseMessageSchema`<br>`packages/protocol/src/messages.ts:6364` | S→C | get_providers_snapshot_request; Providers, commands and usage | `z.object({ type: z.literal("get_providers_snapshot_response"), payload: z.object({ cwd: z.string().optional(), entries: z.array(ProviderSnapshotEntrySchema), compactSnapshot: CompactProviderSnapshotSchema.optional(), snapshotHash: z.string().optional(), fetchedAt: z.record(z.string(), z.string()).optional(), notModified: z.boolean().optional(), generatedAt: z.string(), requestId: z.string(), }), })` | `packages/server/src/server/session/provider/provider-catalog-session.ts:425` `handleGetProvidersSnapshotRequest` (emit) |
| `providers_snapshot_update`<br>`ProvidersSnapshotUpdateMessageSchema`<br>`packages/protocol/src/messages.ts:6379` | S→C | push / shared status or error channel; Providers, commands and usage | `z.object({ type: z.literal("providers_snapshot_update"), payload: z.object({ subscriptionId: z.string().optional(), cwd: z.string().optional(), entries: z.array(ProviderSnapshotEntrySchema), compactSnapshot: CompactProviderSnapshotSchema.optional(), snapshotHash: z.string().optional(), fetchedAt: z.record(z.string(), z.string()).optional(), generatedAt: z.string(), }), })` | `packages/server/src/server/session/provider/provider-catalog-session.ts:91` (emit) |
| `refresh_providers_snapshot_response`<br>`RefreshProvidersSnapshotResponseMessageSchema`<br>`packages/protocol/src/messages.ts:6393` | S→C | refresh_providers_snapshot_request; Providers, commands and usage | `z.object({ type: z.literal("refresh_providers_snapshot_response"), payload: z.object({ requestId: z.string(), acknowledged: z.boolean(), }), })` | `packages/server/src/server/session/provider/provider-catalog-session.ts:447` `handleRefreshProvidersSnapshotRequest` (emit) |
| `provider_diagnostic_response`<br>`ProviderDiagnosticResponseMessageSchema`<br>`packages/protocol/src/messages.ts:6402` | S→C | provider_diagnostic_request; Providers, commands and usage | `z.object({ type: z.literal("provider_diagnostic_response"), payload: z.object({ provider: AgentProviderSchema, diagnostic: z.string(), requestId: z.string(), }), })` | `packages/server/src/server/session/provider/provider-catalog-session.ts:461` `handleProviderDiagnosticRequest` (emit) |
| `provider.usage.list.response`<br>`ProviderUsageListResponseMessageSchema`<br>`packages/protocol/src/messages.ts:6464` | S→C | provider.usage.list.request; Providers, commands and usage | `z.object({ type: z.literal("provider.usage.list.response"), payload: z.object({ requestId: z.string(), fetchedAt: z.string(), providers: z.array(ProviderUsageSchema), }), })` | `packages/server/src/server/session/usage/usage-session.ts:70` `handleLegacyList` (emit) |
| `usage.list_reports.update`<br>`UsageListReportsUpdateMessageSchema`<br>`packages/protocol/src/messages.ts:6523` | S→C | usage.list_reports.request (intermediate report update); Providers, commands and usage | `z.object({ type: z.literal("usage.list_reports.update"), payload: z.object({ requestId: z.string(), report: UsageReportEntrySchema }), })` | `packages/server/src/server/session/usage/usage-session.ts:31` (emit) |
| `usage.list_reports.response`<br>`UsageListReportsResponseMessageSchema`<br>`packages/protocol/src/messages.ts:6527` | S→C | usage.list_reports.request; Providers, commands and usage | `z.object({ type: z.literal("usage.list_reports.response"), payload: z.object({ requestId: z.string(), error: z.string().nullable() }), })` | `packages/server/src/server/session/usage/usage-session.ts:36` `handleListReports` (emit) |

## Protocol: Projects, workspaces, labels and recovery

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `fetch_workspaces_request`<br>`FetchWorkspacesRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1328` | C→S | fetch_workspaces_response; Projects, workspaces, labels and recovery | `z.object({ includeBackground: z.boolean().optional(), type: z.literal("fetch_workspaces_request"), requestId: z.string(), filter: z .object({ query: z.string().optional(), projectId: z.string().optional(), idPrefix: z.string().optional(), includeInternal: z.boolean().optional(), includeBackground: z.boolean().optional(), }) .optional(), sort: z .array(z.object({ key: z.enum(["status_priority", "activity_at", "name", "project_id"]), direction: z.enum(["asc", "desc"]), })) .optional(), page: z .object({ limit: z.number().int().positive().max(200), cursor: z.string().min(1).optional(), }) .optional(), subscribe: z .object({ subscriptionId: z.string().optional(), }) .optional(), sync: DirectorySyncRequestSchema.optional(), })` | `packages/server/src/server/session.ts:3062` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `project.list.request`<br>`ProjectListRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1366` | C→S | project.list.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.list.request"), requestId: z.string(), sync: DirectorySyncRequestSchema.optional(), })` | `packages/server/src/server/session.ts:3064` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `project.suggestions.request`<br>`ProjectSuggestionsRequestSchema`<br>`packages/protocol/src/messages.ts:1373` | C→S | project.suggestions.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.suggestions.request"), requestId: z.string(), providers: z.array(z.string()).optional(), since: z.string().optional(), limit: z.number().int().positive().max(100).optional(), })` | `packages/server/src/server/session.ts:3066` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `project.rename.request`<br>`ProjectRenameRequestSchema`<br>`packages/protocol/src/messages.ts:1004` | C→S | project.rename.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.rename.request"), projectId: z.string(), customName: z.string().nullable(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2912` `dispatchAgentLifecycleMessage` (dispatch) |
| `project.icon.set.request`<br>`ProjectIconSetRequestSchema`<br>`packages/protocol/src/messages.ts:1012` | C→S | project.icon.set.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.icon.set.request"), projectId: z.string(), source: ProjectIconSourceSchema, requestId: z.string(), })` | `packages/server/src/server/session.ts:2914` `dispatchAgentLifecycleMessage` (dispatch) |
| `project.remove.request`<br>`ProjectRemoveRequestSchema`<br>`packages/protocol/src/messages.ts:1019` | C→S | project.remove.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.remove.request"), projectId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3091` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `workspace.title.set.request`<br>`WorkspaceTitleSetRequestSchema`<br>`packages/protocol/src/messages.ts:1025` | C→S | workspace.title.set.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.title.set.request"), workspaceId: z.string(), title: z.string().nullable(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3093` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `workspace.pin.set.request`<br>`WorkspacePinSetRequestSchema`<br>`packages/protocol/src/messages.ts:1033` | C→S | workspace.pin.set.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.pin.set.request"), workspaceId: z.string(), pinned: z.boolean(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3095` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `workspace.label.list.request`<br>`WorkspaceLabelListRequestSchema`<br>`packages/protocol/src/messages.ts:1050` | C→S | workspace.label.list.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.label.list.request"), requestId: z.string(), subscribe: z.object({ subscriptionId: z.string().optional() }).optional(), sync: WorkspaceLabelSyncCursorSchema.optional(), })` | `packages/server/src/server/session.ts:3114` `dispatchWorkspaceLabelMessage` (dispatch) |
| `workspace.label.assignment.set.request`<br>`WorkspaceLabelAssignmentSetRequestSchema`<br>`packages/protocol/src/messages.ts:1056` | C→S | workspace.label.assignment.set.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.label.assignment.set.request"), requestId: z.string(), workspaceId: z.string(), label: WorkspaceLabelDefinitionSchema, assigned: z.boolean(), })` | `packages/server/src/server/session.ts:3116` `dispatchWorkspaceLabelMessage` (dispatch) |
| `workspace.label.update.request`<br>`WorkspaceLabelUpdateRequestSchema`<br>`packages/protocol/src/messages.ts:1068` | C→S | workspace.label.update.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.label.update.request"), requestId: z.string(), name: z.string(), newName: z.string().optional(), color: WorkspaceLabelColorSchema.optional(), })` | `packages/server/src/server/session.ts:3118` `dispatchWorkspaceLabelMessage` (dispatch) |
| `workspace.label.delete.request`<br>`WorkspaceLabelDeleteRequestSchema`<br>`packages/protocol/src/messages.ts:1075` | C→S | workspace.label.delete.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.label.delete.request"), requestId: z.string(), name: z.string(), })` | `packages/server/src/server/session.ts:3120` `dispatchWorkspaceLabelMessage` (dispatch) |
| `workspace.label.delete.inspect.request`<br>`WorkspaceLabelDeleteInspectRequestSchema`<br>`packages/protocol/src/messages.ts:1080` | C→S | workspace.label.delete.inspect.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.label.delete.inspect.request"), requestId: z.string(), name: z.string(), })` | `packages/server/src/server/session.ts:3122` `dispatchWorkspaceLabelMessage` (dispatch) |
| `workspace.recovery.inspect.request`<br>`WorkspaceRecoveryInspectRequestSchema`<br>`packages/protocol/src/messages.ts:1086` | C→S | workspace.recovery.inspect.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.recovery.inspect.request"), workspaceId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3166` `dispatchWorkspaceStateMessage` (dispatch) |
| `workspace.recovery.restore.request`<br>`WorkspaceRecoveryRestoreRequestSchema`<br>`packages/protocol/src/messages.ts:1092` | C→S | workspace.recovery.restore.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.recovery.restore.request"), workspaceId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3168` `dispatchWorkspaceStateMessage` (dispatch) |
| `open_project_request`<br>`OpenProjectRequestSchema`<br>`packages/protocol/src/messages.ts:2659` | C→S | open_project_response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("open_project_request"), cwd: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3079` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `project.add.request`<br>`ProjectAddRequestSchema`<br>`packages/protocol/src/messages.ts:2669` | C→S | project.add.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.add.request"), cwd: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3081` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `project.create_directory.request`<br>`ProjectCreateDirectoryRequestSchema`<br>`packages/protocol/src/messages.ts:2675` | C→S | project.create_directory.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.create_directory.request"), parentPath: z.string(), name: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3083` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `workspace.github.search_repositories.request`<br>`WorkspaceGithubSearchRepositoriesRequestSchema`<br>`packages/protocol/src/messages.ts:2692` | C→S | workspace.github.search_repositories.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.github.search_repositories.request"), query: z.string(), limit: z.number().int().min(1).max(50).optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3085` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `project.github.clone.request`<br>`ProjectGithubCloneRequestSchema`<br>`packages/protocol/src/messages.ts:2701` | C→S | project.github.clone.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.github.clone.request"), repo: z.string().trim().min(MIN_REPOSITORY_PATH_LENGTH), cloneProtocol: ProjectGithubCloneProtocolSchema.optional(), targetDirectory: z.string().trim().min(1), requestId: z.string(), })` | `packages/server/src/server/session.ts:3087` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `archive_workspace_request`<br>`ArchiveWorkspaceRequestSchema`<br>`packages/protocol/src/messages.ts:2709` | C→S | archive_workspace_response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("archive_workspace_request"), workspaceId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3089` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `workspace.clear_attention.request`<br>`WorkspaceClearAttentionRequestSchema`<br>`packages/protocol/src/messages.ts:2764` | C→S | workspace.clear_attention.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.clear_attention.request"), workspaceId: z.union([z.string(), z.array(z.string())]), requestId: z.string(), })` | `packages/server/src/server/session.ts:3170` `dispatchWorkspaceStateMessage` (dispatch) |
| `workspace.mark_unread.request`<br>`WorkspaceMarkUnreadRequestSchema`<br>`packages/protocol/src/messages.ts:2770` | C→S | workspace.mark_unread.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.mark_unread.request"), workspaceId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3172` `dispatchWorkspaceStateMessage` (dispatch) |
| `project.icon.get.request`<br>`ProjectIconGetRequestSchema`<br>`packages/protocol/src/messages.ts:2932` | C→S | project.icon.get.response; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.icon.get.request"), projectId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3152` `dispatchWorkspaceFileMessage` (dispatch) |
| `workspace_update`<br>`WorkspaceUpdateMessageSchema`<br>`packages/protocol/src/messages.ts:4390` | S→C | push / shared status or error channel; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace_update"), payload: z.discriminatedUnion("kind", [ z.object({ subscriptionId: z.string().optional(), kind: z.literal("upsert"), workspace: WorkspaceDescriptorPayloadSchema, generation: z.string().optional(), seq: z.number().int().positive().optional(), }), z.object({ subscriptionId: z.string().optional(), kind: z.literal("remove"), id: z.string(), emptyProject: WorkspaceProjectDescriptorPayloadSchema.optional(), removedProjectId: z.string().optional(), generation: z.string().optional(), seq: z.number().int().positive().optional(), }), ]), })` | `packages/server/src/server/session.ts:1495` `emitCreatedWorkspaceUpdate` (emit)<br>`packages/server/src/server/session.ts:5919` `bufferOrEmitWorkspaceUpdate` (emit) |
| `workspace.label.list.response`<br>`WorkspaceLabelListResponseSchema`<br>`packages/protocol/src/messages.ts:4426` | S→C | workspace.label.list.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.label.list.response"), payload: z.object({ requestId: z.string(), subscriptionId: z.string().optional(), labels: z.array(WorkspaceLabelDefinitionSchema), sync: WorkspaceLabelSyncMetadataSchema, }), })` | `packages/server/src/server/session.ts:6618` `handleWorkspaceLabelList` (emit) |
| `workspace.label.update`<br>`WorkspaceLabelUpdateSchema`<br>`packages/protocol/src/messages.ts:4435` | S→C | push / shared status or error channel; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.label.update"), payload: z.discriminatedUnion("kind", [ z.object({ subscriptionId: z.string().optional(), kind: z.literal("upsert"), label: WorkspaceLabelDefinitionSchema, previousName: z.string().optional(), generation: z.string(), seq: z.number().int().positive(), }), z.object({ subscriptionId: z.string().optional(), kind: z.literal("remove"), name: z.string(), generation: z.string(), seq: z.number().int().positive(), }), ]), })` | `packages/server/src/server/session.ts:6636` (emit) |
| `workspace.label.assignment.set.response`<br>`WorkspaceLabelAssignmentSetResponseSchema`<br>`packages/protocol/src/messages.ts:4455` | S→C | workspace.label.assignment.set.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.label.assignment.set.response"), payload: z.object({ requestId: z.string(), label: WorkspaceLabelDefinitionSchema, workspaceLabels: z.array(z.string()), }), })` | `packages/server/src/server/session.ts:6676` `handleWorkspaceLabelAssignment` (emit) |
| `workspace.label.update.response`<br>`WorkspaceLabelUpdateResponseSchema`<br>`packages/protocol/src/messages.ts:4463` | S→C | workspace.label.update.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.label.update.response"), payload: z.object({ requestId: z.string(), label: WorkspaceLabelDefinitionSchema, affectedWorkspaceCount: z.number().int().nonnegative(), }), })` | `packages/server/src/server/session.ts:6690` `handleWorkspaceLabelUpdate` (emit) |
| `workspace.label.delete.response`<br>`WorkspaceLabelDeleteResponseSchema`<br>`packages/protocol/src/messages.ts:4471` | S→C | workspace.label.delete.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.label.delete.response"), payload: z.object({ requestId: z.string(), affectedWorkspaceCount: z.number().int().nonnegative(), }), })` | `packages/server/src/server/session.ts:6704` `handleWorkspaceLabelDelete` (emit) |
| `workspace.label.delete.inspect.response`<br>`WorkspaceLabelDeleteInspectResponseSchema`<br>`packages/protocol/src/messages.ts:4478` | S→C | workspace.label.delete.inspect.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.label.delete.inspect.response"), payload: z.object({ requestId: z.string(), affectedWorkspaceCount: z.number().int().nonnegative(), }), })` | `packages/server/src/server/session.ts:6720` `handleWorkspaceLabelDeleteInspection` (emit) |
| `project.update`<br>`ProjectUpdateMessageSchema`<br>`packages/protocol/src/messages.ts:4486` | S→C | push / shared status or error channel; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.update"), payload: z.discriminatedUnion("kind", [ z.object({ subscriptionId: z.string().optional(), kind: z.literal("upsert"), project: WorkspaceProjectDescriptorPayloadSchema, generation: z.string().optional(), seq: z.number().int().positive().optional(), }), z.object({ subscriptionId: z.string().optional(), kind: z.literal("remove"), projectId: z.string(), generation: z.string().optional(), seq: z.number().int().positive().optional(), }), ]), })` | `packages/server/src/server/session.ts:1459` `publishProjectUpdate` (emit) |
| `project.list.response`<br>`ProjectListResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4506` | S→C | project.list.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.list.response"), payload: z.object({ requestId: z.string(), projects: z.array(WorkspaceProjectDescriptorPayloadSchema), sync: DirectorySyncMetadataSchema.optional(), }), })` | `packages/server/src/server/session.ts:6539` `handleProjectListRequest` (emit) |
| `project.suggestions.response`<br>`ProjectSuggestionsResponseSchema`<br>`packages/protocol/src/messages.ts:4524` | S→C | project.suggestions.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.suggestions.response"), payload: z.object({ requestId: z.string(), suggestions: z.array(ProjectSuggestionSchema), providerErrors: z.array(z.object({ provider: z.string(), message: z.string() })), }), })` | `packages/server/src/server/session.ts:6570` `handleProjectSuggestionsRequest` (emit) |
| `fetch_workspaces_response`<br>`FetchWorkspacesResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4371` | S→C | fetch_workspaces_request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("fetch_workspaces_response"), payload: z.object({ requestId: z.string(), subscriptionId: z.string().nullable().optional(), entries: z.array(WorkspaceDescriptorPayloadSchema), emptyProjects: z.array(WorkspaceProjectDescriptorPayloadSchema).optional().default([]), pageInfo: z.object({ nextCursor: z.string().nullable(), prevCursor: z.string().nullable(), hasMore: z.boolean(), }), sync: DirectorySyncMetadataSchema.optional(), }), })` | `packages/server/src/server/session.ts:6497` `handleFetchWorkspacesRequest` (emit) |
| `project.add.response`<br>`ProjectAddResponseSchema`<br>`packages/protocol/src/messages.ts:4615` | S→C | project.add.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.add.response"), payload: z.object({ requestId: z.string(), project: WorkspaceProjectDescriptorPayloadSchema.nullable(), error: z.string().nullable(), errorCode: z.enum(["directory_not_found"]).nullish().catch(null), }), })` | `packages/server/src/server/session.ts:7084` `handleProjectAddRequest` (emit) |
| `project.create_directory.response`<br>`ProjectCreateDirectoryResponseSchema`<br>`packages/protocol/src/messages.ts:4634` | S→C | project.create_directory.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.create_directory.response"), payload: z.object({ requestId: z.string(), directoryPath: z.string().nullable(), project: WorkspaceProjectDescriptorPayloadSchema.nullable(), error: z.string().nullable(), errorCode: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:7147` `handleProjectCreateDirectoryRequest` (emit) |
| `open_project_response`<br>`OpenProjectResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4604` | S→C | open_project_request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("open_project_response"), payload: z.object({ requestId: z.string(), workspace: WorkspaceDescriptorPayloadSchema.nullable(), error: z.string().nullable(), errorCode: z.enum(["directory_not_found"]).nullish().catch(null), }), })` | `packages/server/src/server/session.ts:6995` `handleOpenProjectRequest` (emit) |
| `workspace.github.search_repositories.response`<br>`WorkspaceGithubSearchRepositoriesResponseSchema`<br>`packages/protocol/src/messages.ts:4647` | S→C | workspace.github.search_repositories.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.github.search_repositories.response"), payload: z.discriminatedUnion("status", [ z.object({ status: z.literal("success"), requestId: z.string(), repositories: z.array(GithubRepositorySchema), available: z.literal(true), error: z.null(), }), z.object({ status: z.literal("unavailable"), requestId: z.string(), repositories: z.array(GithubRepositorySchema), reason: z.literal("gh_missing"), available: z.literal(false), error: z.string(), }), z.object({ status: z.literal("unauthenticated"), requestId: z.string(), repositories: z.array(GithubRepositorySchema), available: z.literal(false), error: z.string(), }), z.object({ status: z.literal("error"), requestId: z.string(), repositories: z.array(GithubRepositorySchema), available: z.literal(true), error: z.string(), }), ]), })` | `packages/server/src/server/session.ts:7203` `handleWorkspaceGithubSearchRepositoriesRequest` (emit) |
| `project.github.clone.response`<br>`ProjectGithubCloneResponseSchema`<br>`packages/protocol/src/messages.ts:4682` | S→C | project.github.clone.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.github.clone.response"), payload: z.object({ requestId: z.string(), repo: z.string().trim().min(MIN_REPOSITORY_PATH_LENGTH), checkoutPath: z.string().nullable(), project: WorkspaceProjectDescriptorPayloadSchema.nullable(), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:7311` `handleProjectGithubCloneRequest` (emit) |
| `archive_workspace_response`<br>`ArchiveWorkspaceResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4751` | S→C | archive_workspace_request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("archive_workspace_response"), payload: z.object({ requestId: z.string(), workspaceId: z.string(), archivedAt: z.string().nullable(), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:7608` `handleArchiveWorkspaceRequest` (emit) |
| `workspace.clear_attention.response`<br>`WorkspaceClearAttentionResponseSchema`<br>`packages/protocol/src/messages.ts:5093` | S→C | workspace.clear_attention.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.clear_attention.response"), payload: z.object({ requestId: z.string(), workspaceId: z.union([z.string(), z.array(z.string())]), clearedAgentIds: z.array(z.string()), results: z.array(z.object({ workspaceId: z.string(), clearedAgentIds: z.array(z.string()), success: z.boolean(), error: z.string().nullable(), })), success: z.boolean(), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:7651` `handleWorkspaceClearAttentionRequest` (emit) |
| `workspace.mark_unread.response`<br>`WorkspaceMarkUnreadResponseSchema`<br>`packages/protocol/src/messages.ts:5112` | S→C | workspace.mark_unread.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.mark_unread.response"), payload: z.object({ requestId: z.string(), workspaceId: z.string(), markedAgentId: z.string().nullable(), success: z.boolean(), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:7803` `handleWorkspaceMarkUnreadRequest` (emit) |
| `project.rename.response`<br>`ProjectRenameResponseSchema`<br>`packages/protocol/src/messages.ts:2134` | S→C | project.rename.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.rename.response"), payload: ProjectRenameResponsePayloadSchema, })` | `packages/server/src/server/session.ts:3645` `handleProjectRenameRequest` (emit) |
| `project.icon.set.response`<br>`ProjectIconSetResponseSchema`<br>`packages/protocol/src/messages.ts:2139` | S→C | project.icon.set.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.icon.set.response"), payload: z.object({ requestId: z.string(), projectId: z.string(), accepted: z.boolean(), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:3731` `handleProjectIconSetRequest` (emit) |
| `project.remove.response`<br>`ProjectRemoveResponseSchema`<br>`packages/protocol/src/messages.ts:2157` | S→C | project.remove.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.remove.response"), payload: ProjectRemoveResponsePayloadSchema, })` | `packages/server/src/server/session.ts:3836` `handleProjectRemoveRequest` (emit) |
| `workspace.title.set.response`<br>`WorkspaceTitleSetResponseSchema`<br>`packages/protocol/src/messages.ts:2170` | S→C | workspace.title.set.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.title.set.response"), payload: WorkspaceTitleSetResponsePayloadSchema, })` | `packages/server/src/server/session.ts:3893` `handleWorkspaceTitleSetRequest` (emit) |
| `workspace.pin.set.response`<br>`WorkspacePinSetResponseSchema`<br>`packages/protocol/src/messages.ts:2183` | S→C | workspace.pin.set.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.pin.set.response"), payload: WorkspacePinSetResponsePayloadSchema, })` | `packages/server/src/server/session.ts:3953` (emit) |
| `workspace.recovery.inspect.response`<br>`WorkspaceRecoveryInspectResponseSchema`<br>`packages/protocol/src/messages.ts:2204` | S→C | workspace.recovery.inspect.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.recovery.inspect.response"), payload: z.object({ requestId: z.string(), state: WorkspaceRecoveryStateSchema, }), })` | `packages/server/src/server/session.ts:3995` `handleWorkspaceRecoveryInspectRequest` (emit) |
| `workspace.recovery.restore.response`<br>`WorkspaceRecoveryRestoreResponseSchema`<br>`packages/protocol/src/messages.ts:2212` | S→C | workspace.recovery.restore.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("workspace.recovery.restore.response"), payload: z.object({ requestId: z.string(), workspaceId: z.string(), accepted: z.boolean(), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:4009` `handleWorkspaceRecoveryRestoreRequest` (emit) |
| `project.icon.get.response`<br>`ProjectIconGetResponseSchema`<br>`packages/protocol/src/messages.ts:6281` | S→C | project.icon.get.request; Projects, workspaces, labels and recovery | `z.object({ type: z.literal("project.icon.get.response"), payload: z.object({ projectId: z.string(), icon: ProjectIconSchema.nullable(), error: z.string().nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session.ts:3762` `handleProjectIconGetRequest` (emit) |

## Protocol: Plugins

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `plugin.catalog.get.request`<br>`PluginCatalogGetRequestSchema`<br>`packages/protocol/src/messages.ts:1489` | C→S | plugin.catalog.get.response; Plugins | `z.object({ type: z.literal("plugin.catalog.get.request"), requestId: z.string(), cachedClientBundles: z .array(z.object({ id: PluginIdSchema, clientBundleHash: z.string() })) .optional(), })` | `packages/server/src/server/session.ts:2610` `dispatchPluginMessage` (dispatch) |
| `plugin.list.request`<br>`PluginListRequestSchema`<br>`packages/protocol/src/messages.ts:1498` | C→S | plugin.list.response; Plugins | `z.object({ type: z.literal("plugin.list.request"), requestId: z.string(), })` | `packages/server/src/server/session.ts:2552` `dispatchPluginMessage` (dispatch) |
| `plugin.logs.get.request`<br>`PluginLogsGetRequestSchema`<br>`packages/protocol/src/messages.ts:1503` | C→S | plugin.logs.get.response; Plugins | `z.object({ type: z.literal("plugin.logs.get.request"), requestId: z.string(), pluginId: PluginIdSchema, })` | `packages/server/src/server/session.ts:2561` `dispatchPluginMessage` (dispatch) |
| `plugin.directory.install.request`<br>`PluginDirectoryInstallRequestSchema`<br>`packages/protocol/src/messages.ts:1509` | C→S | plugin.directory.install.response; Plugins | `z.object({ type: z.literal("plugin.directory.install.request"), requestId: z.string(), path: z.string().min(1), id: PluginIdSchema.optional(), })` | `packages/server/src/server/session.ts:2700` `dispatchPluginDirectoryMessage` (dispatch) |
| `plugin.directory.inspect.request`<br>`PluginDirectoryInspectRequestSchema`<br>`packages/protocol/src/messages.ts:1516` | C→S | plugin.directory.inspect.response; Plugins | `z.object({ type: z.literal("plugin.directory.inspect.request"), requestId: z.string(), path: z.string().min(1), })` | `packages/server/src/server/session.ts:2710` `dispatchPluginDirectoryMessage` (dispatch) |
| `plugin.source.install.request`<br>`PluginSourceInstallRequestSchema`<br>`packages/protocol/src/messages.ts:1522` | C→S | plugin.source.install.response; Plugins | `z.object({ type: z.literal("plugin.source.install.request"), requestId: z.string(), source: z.string().min(1), id: PluginIdSchema.optional(), ref: z.string().min(1).optional(), pluginPath: z.string().min(1).optional(), })` | `packages/server/src/server/session.ts:2641` `dispatchPluginDirectoryMessage` (dispatch) |
| `plugin.source.status.request`<br>`PluginSourceStatusRequestSchema`<br>`packages/protocol/src/messages.ts:1589` | C→S | plugin.source.status.response; Plugins | `z.object({ type: z.literal("plugin.source.status.request"), requestId: z.string(), pluginId: PluginIdSchema.optional(), })` | `packages/server/src/server/session.ts:2658` `dispatchPluginDirectoryMessage` (dispatch) |
| `plugin.source.update.request`<br>`PluginSourceUpdateRequestSchema`<br>`packages/protocol/src/messages.ts:1595` | C→S | plugin.source.update.response; Plugins | `z.object({ type: z.literal("plugin.source.update.request"), requestId: z.string(), pluginId: PluginIdSchema.optional(), })` | `packages/server/src/server/session.ts:2690` `dispatchPluginDirectoryMessage` (dispatch) |
| `plugin.source.update.preview.request`<br>`PluginUpdatePreviewRequestSchema`<br>`packages/protocol/src/messages.ts:1577` | C→S | plugin.source.update.preview.response; Plugins | `z.object({ type: z.literal("plugin.source.update.preview.request"), requestId: z.string(), pluginId: PluginIdSchema.optional(), target: PluginUpdateSelectionSchema.optional(), })` | `packages/server/src/server/session.ts:2668` `dispatchPluginDirectoryMessage` (dispatch) |
| `plugin.source.update.apply.request`<br>`PluginUpdateApplyRequestSchema`<br>`packages/protocol/src/messages.ts:1583` | C→S | plugin.source.update.apply.response; Plugins | `z.object({ type: z.literal("plugin.source.update.apply.request"), requestId: z.string(), proposals: z.array(PluginUpdateProposalSchema).min(1), })` | `packages/server/src/server/session.ts:2680` `dispatchPluginDirectoryMessage` (dispatch) |
| `plugin.reload.request`<br>`PluginReloadRequestSchema`<br>`packages/protocol/src/messages.ts:1605` | C→S | plugin.reload.response; Plugins | `z.object({ type: z.literal("plugin.reload.request"), requestId: z.string(), pluginId: PluginIdSchema })` | `packages/server/src/server/session.ts:2573` `dispatchPluginMessage` (dispatch) |
| `plugin.enable.request`<br>`PluginEnableRequestSchema`<br>`packages/protocol/src/messages.ts:1606` | C→S | plugin.enable.response; Plugins | `z.object({ type: z.literal("plugin.enable.request"), requestId: z.string(), pluginId: PluginIdSchema })` | `packages/server/src/server/session.ts:2583` `dispatchPluginMessage` (dispatch) |
| `plugin.disable.request`<br>`PluginDisableRequestSchema`<br>`packages/protocol/src/messages.ts:1607` | C→S | plugin.disable.response; Plugins | `z.object({ type: z.literal("plugin.disable.request"), requestId: z.string(), pluginId: PluginIdSchema })` | `packages/server/src/server/session.ts:2593` `dispatchPluginMessage` (dispatch) |
| `plugin.remove.request`<br>`PluginRemoveRequestSchema`<br>`packages/protocol/src/messages.ts:1608` | C→S | plugin.remove.response; Plugins | `z.object({ type: z.literal("plugin.remove.request"), requestId: z.string(), pluginId: PluginIdSchema })` | `packages/server/src/server/session.ts:2603` `dispatchPluginMessage` (dispatch) |
| `plugin.rpc.invoke.request`<br>`PluginRpcInvokeRequestSchema`<br>`packages/protocol/src/messages.ts:1610` | C→S | plugin.rpc.invoke.response; Plugins | `z.object({ type: z.literal("plugin.rpc.invoke.request"), requestId: z.string(), pluginId: PluginIdSchema, method: z.string().min(1), input: z.unknown(), })` | `packages/server/src/server/session.ts:2625` `dispatchPluginMessage` (dispatch) |
| `plugin.catalog.get.response`<br>`PluginCatalogGetResponseSchema`<br>`packages/protocol/src/messages.ts:6828` | S→C | plugin.catalog.get.request; Plugins | `z.object({ type: z.literal("plugin.catalog.get.response"), payload: z.object({ requestId: z.string(), plugins: z.array(z.object({ id: PluginIdSchema, clientBundle: z.string().optional(), clientBundleHash: z.string().optional(), requirements: PluginRequirementsSchema.optional(), })), }), })` | `packages/server/src/server/session.ts:2620` `dispatchPluginMessage` (emit) |
| `plugin.list.response`<br>`PluginListResponseSchema`<br>`packages/protocol/src/messages.ts:6882` | S→C | plugin.list.request; Plugins | `z.object({ type: z.literal("plugin.list.response"), payload: z.object({ requestId: z.string(), plugins: z.array(PluginListItemSchema) }), })` | `packages/server/src/server/session.ts:2555` (emit) |
| `plugin.logs.get.response`<br>`PluginLogsGetResponseSchema`<br>`packages/protocol/src/messages.ts:6887` | S→C | plugin.logs.get.request; Plugins | `z.object({ type: z.literal("plugin.logs.get.response"), payload: z.object({ requestId: z.string(), pluginId: PluginIdSchema, entries: z.array(PluginLogEntrySchema), }), })` | `packages/server/src/server/session.ts:2564` `dispatchPluginMessage` (emit) |
| `plugin.directory.install.response`<br>`PluginDirectoryInstallResponseSchema`<br>`packages/protocol/src/messages.ts:6896` | S→C | plugin.directory.install.request; Plugins | `z.object({ type: z.literal("plugin.directory.install.response"), payload: z.object({ requestId: z.string(), plugin: PluginListItemSchema }), })` | `packages/server/src/server/session.ts:2704` (emit) |
| `plugin.directory.inspect.response`<br>`PluginDirectoryInspectResponseSchema`<br>`packages/protocol/src/messages.ts:6901` | S→C | plugin.directory.inspect.request; Plugins | `z.object({ type: z.literal("plugin.directory.inspect.response"), payload: z.object({ requestId: z.string(), id: PluginIdSchema }), })` | `packages/server/src/server/session.ts:2714` (emit) |
| `plugin.source.install.response`<br>`PluginSourceInstallResponseSchema`<br>`packages/protocol/src/messages.ts:6906` | S→C | plugin.source.install.request; Plugins | `z.object({ type: z.literal("plugin.source.install.response"), payload: z.object({ requestId: z.string(), plugin: PluginListItemSchema }), })` | `packages/server/src/server/session.ts:2652` (emit) |
| `plugin.source.status.response`<br>`PluginSourceStatusResponseSchema`<br>`packages/protocol/src/messages.ts:6926` | S→C | plugin.source.status.request; Plugins | `z.object({ type: z.literal("plugin.source.status.response"), payload: z.object({ requestId: z.string(), plugins: z.array(PluginSourceStatusItemSchema) }), })` | `packages/server/src/server/session.ts:2662` (emit) |
| `plugin.source.update.response`<br>`PluginSourceUpdateResponseSchema`<br>`packages/protocol/src/messages.ts:6940` | S→C | plugin.source.update.request; Plugins | `z.object({ type: z.literal("plugin.source.update.response"), payload: z.object({ requestId: z.string(), plugins: z.array(PluginSourceUpdateItemSchema) }), })` | `packages/server/src/server/session.ts:2694` (emit) |
| `plugin.source.update.preview.response`<br>`PluginUpdatePreviewResponseSchema`<br>`packages/protocol/src/messages.ts:6963` | S→C | plugin.source.update.preview.request; Plugins | `z.object({ type: z.literal("plugin.source.update.preview.response"), payload: z.object({ requestId: z.string(), plugins: z.array(PluginUpdatePreviewSchema) }), })` | `packages/server/src/server/session.ts:2674` (emit) |
| `plugin.source.update.apply.response`<br>`PluginUpdateApplyResponseSchema`<br>`packages/protocol/src/messages.ts:6967` | S→C | plugin.source.update.apply.request; Plugins | `z.object({ type: z.literal("plugin.source.update.apply.response"), payload: z.object({ requestId: z.string(), plugins: z.array(PluginUpdateResultSchema) }), })` | `packages/server/src/server/session.ts:2684` (emit) |
| `plugin.reload.response`<br>`PluginReloadResponseSchema`<br>`packages/protocol/src/messages.ts:6979` | S→C | plugin.reload.request; Plugins | `z.object({ type: z.literal("plugin.reload.response"), payload: z.object({ requestId: z.string(), plugin: PluginListItemSchema }) })` | `packages/server/src/server/session.ts:2577` (emit) |
| `plugin.enable.response`<br>`PluginEnableResponseSchema`<br>`packages/protocol/src/messages.ts:6980` | S→C | plugin.enable.request; Plugins | `z.object({ type: z.literal("plugin.enable.response"), payload: z.object({ requestId: z.string(), plugin: PluginListItemSchema }) })` | `packages/server/src/server/session.ts:2587` (emit) |
| `plugin.disable.response`<br>`PluginDisableResponseSchema`<br>`packages/protocol/src/messages.ts:6981` | S→C | plugin.disable.request; Plugins | `z.object({ type: z.literal("plugin.disable.response"), payload: z.object({ requestId: z.string(), plugin: PluginListItemSchema }) })` | `packages/server/src/server/session.ts:2597` (emit) |
| `plugin.remove.response`<br>`PluginRemoveResponseSchema`<br>`packages/protocol/src/messages.ts:6982` | S→C | plugin.remove.request; Plugins | `z.object({ type: z.literal("plugin.remove.response"), payload: z.object({ requestId: z.string() }).strict(), })` | `packages/server/src/server/session.ts:2606` (emit) |
| `plugin.rpc.invoke.response`<br>`PluginRpcInvokeResponseSchema`<br>`packages/protocol/src/messages.ts:6987` | S→C | plugin.rpc.invoke.request; Plugins | `z.object({ type: z.literal("plugin.rpc.invoke.response"), payload: z.object({ requestId: z.string(), output: z.unknown(), }), })` | `packages/server/src/server/session.ts:2631` (emit) |

## Protocol: Agent timelines and search

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `agent.timeline.append.request`<br>`AgentTimelineAppendRequestSchema`<br>`packages/protocol/src/messages.ts:1618` | C→S | agent.timeline.append.response; Agent timelines and search | `z.object({ type: z.literal("agent.timeline.append.request"), requestId: z.string(), agentId: z.string(), item: z.object({ type: z.literal("plugin"), id: z.string(), kind: z.string(), version: z.number().int().positive(), data: JsonWireValueSchema, }), })` | `packages/server/src/server/session.ts:2810` `dispatchAgentTimelineMessage` (dispatch) |
| `fetch_agent_timeline_request`<br>`FetchAgentTimelineRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1941` | C→S | fetch_agent_timeline_response; Agent timelines and search | `z.object({ wholeTurns: z.boolean().optional(), type: z.literal("fetch_agent_timeline_request"), agentId: z.string(), requestId: z.string(), direction: z.enum(["tail", "before", "after"]).optional(), cursor: AgentTimelineCursorSchema.optional(), limit: z.number().int().nonnegative().optional(), projection: z.enum(["projected", "canonical"]).optional(), mergeWindow: z.boolean().optional(), })` | `packages/server/src/server/session.ts:2808` `dispatchAgentTimelineMessage` (dispatch) |
| `agent.timeline.search.request`<br>`AgentTimelineSearchRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1957` | C→S | agent.timeline.search.response; Agent timelines and search | `z.object({ type: z.literal("agent.timeline.search.request"), agentId: z.string(), requestId: z.string(), query: z.string(), cursor: z.number().int().nonnegative().optional(), })` | `packages/server/src/server/session.ts:2812` `dispatchAgentTimelineMessage` (dispatch) |
| `agent.timeline.list_prompts.request`<br>`AgentTimelineListPromptsRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1965` | C→S | agent.timeline.list_prompts.response; Agent timelines and search | `z.object({ type: z.literal("agent.timeline.list_prompts.request"), agentId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2814` `dispatchAgentTimelineMessage` (dispatch) |
| `agent.timeline.set_subscription.request`<br>`SetAgentTimelineSubscriptionRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1987` | C→S | agent.timeline.set_subscription.response; Agent timelines and search | `z.object({ type: z.literal("agent.timeline.set_subscription.request"), agentIds: z.array(z.string()), requestId: z.string(), })` | `packages/server/src/server/session.ts:2846` `dispatchAgentTimelineMessage` (dispatch) |
| `agent.fork_context.request`<br>`AgentForkContextRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1993` | C→S | agent.fork_context.response; Agent timelines and search | `z.object({ type: z.literal("agent.fork_context.request"), agentId: z.string(), boundaryCursor: AgentTimelineCursorSchema.optional(), boundaryMessageId: z.string().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2858` `dispatchAgentTimelineMessage` (dispatch) |
| `pull_request_timeline_request`<br>`PullRequestTimelineRequestSchema`<br>`packages/protocol/src/messages.ts:2446` | C→S | pull_request_timeline_response; Agent timelines and search | `z.object({ type: z.literal("pull_request_timeline_request"), cwd: z.string(), prNumber: z.number(), repoOwner: z.string(), repoName: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3042` `dispatchCheckoutMessage` (dispatch) |
| `agent.timeline.append.response`<br>`AgentTimelineAppendResponseSchema`<br>`packages/protocol/src/messages.ts:6995` | S→C | agent.timeline.append.request; Agent timelines and search | `z.object({ type: z.literal("agent.timeline.append.response"), payload: z.object({ requestId: z.string(), seq: z.number().int().nonnegative(), epoch: z.string(), }), })` | `packages/server/src/server/session.ts:7997` `handleAgentTimelineAppendRequest` (emit) |
| `agent_stream`<br>`AgentStreamMessageSchema`<br>`packages/protocol/src/messages.ts:4249` | S→C | push / shared status or error channel; Agent timelines and search | `z.object({ type: z.literal("agent_stream"), payload: z.object({ subscriptionId: z.string().optional(), agentId: z.string(), event: AgentStreamEventPayloadSchema, timestamp: z.string(), seq: z.number().int().nonnegative().optional(), epoch: z.string().optional(), }), })` | `packages/server/src/server/session.ts:1386` `forwardAgentStream` (emit)<br>`packages/server/src/server/session.ts:4979` `emitReconstructedTimelineRows` (emit) |
| `fetch_agent_timeline_response`<br>`FetchAgentTimelineResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4787` | S→C | fetch_agent_timeline_request; Agent timelines and search | `z.object({ type: z.literal("fetch_agent_timeline_response"), payload: z.object({ requestId: z.string(), agentId: z.string(), agent: AgentSnapshotPayloadSchema.nullable(), direction: z.enum(["tail", "before", "after"]), projection: z.enum(["projected", "canonical"]), epoch: z.string(), reset: z.boolean(), staleCursor: z.boolean(), gap: z.boolean(), window: z.object({ minSeq: z.number().int().nonnegative(), maxSeq: z.number().int().nonnegative(), nextSeq: z.number().int().nonnegative(), }), startCursor: AgentTimelineCursorSchema.nullable(), endCursor: AgentTimelineCursorSchema.nullable(), hasOlder: z.boolean(), hasNewer: z.boolean(), mergeWindow: z.boolean().optional(), entries: z.array(AgentTimelineEntryPayloadSchema), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:7911` `handleFetchAgentTimelineRequest` (emit) |
| `agent.timeline.replacement`<br>`AgentTimelineReplacementMessageSchema`<br>`packages/protocol/src/messages.ts:4814` | S→C | push / shared status or error channel; Agent timelines and search | `z.object({ type: z.literal("agent.timeline.replacement"), payload: z.object({ subscriptionId: z.string().optional(), agentId: z.string(), epoch: z.string(), }), })` | `packages/server/src/server/session.ts:4942` `deliverTimelineReplacement` (emit) |
| `agent.timeline.search.response`<br>`AgentTimelineSearchResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4823` | S→C | agent.timeline.search.request; Agent timelines and search | `z.object({ type: z.literal("agent.timeline.search.response"), payload: z.object({ requestId: z.string(), agentId: z.string(), epoch: z.string(), locations: z.array(z.object({ seq: z.number().int().nonnegative(), role: z.enum(["user", "assistant"]), count: z.number().int().positive().optional(), })), nextCursor: z.number().int().nonnegative().nullable(), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:8026` `handleAgentTimelineSearchRequest` (emit) |
| `agent.timeline.list_prompts.response`<br>`AgentTimelineListPromptsResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4843` | S→C | agent.timeline.list_prompts.request; Agent timelines and search | `z.object({ type: z.literal("agent.timeline.list_prompts.response"), payload: z.object({ requestId: z.string(), agentId: z.string(), epoch: z.string(), prompts: z.array(z.object({ seq: z.number().int().nonnegative(), timestamp: z.string(), preview: z.string(), })), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:8073` `handleAgentTimelineListPromptsRequest` (emit) |
| `agent.timeline.set_subscription.response`<br>`SetAgentTimelineSubscriptionResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4963` | S→C | agent.timeline.set_subscription.request; Agent timelines and search | `z.object({ type: z.literal("agent.timeline.set_subscription.response"), payload: z.object({ subscriptionId: z.string().optional(), agentIds: z.array(z.string()), requestId: z.string(), }), })` | `packages/server/src/server/session.ts:2851` `dispatchAgentTimelineMessage` (emit) |
| `agent.fork_context.response`<br>`AgentForkContextResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4995` | S→C | agent.fork_context.request; Agent timelines and search | `z.object({ type: z.literal("agent.fork_context.response"), payload: z.object({ requestId: z.string(), agentId: z.string(), attachment: TextAttachmentSchema.nullable(), itemCount: z.number().int().nonnegative(), boundaryMessageId: z.string().nullable(), boundaryCursor: AgentTimelineCursorSchema.nullable().optional(), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:8260` `handleAgentForkContextRequest` (emit) |
| `pull_request_timeline_response`<br>`PullRequestTimelineResponseSchema`<br>`packages/protocol/src/messages.ts:5965` | S→C | pull_request_timeline_request; Agent timelines and search | `z.object({ type: z.literal("pull_request_timeline_response"), payload: z .object({ cwd: z.string().optional().default(""), prNumber: z.number().nullable().optional().default(null), items: z.array(PullRequestTimelineItemSchema).optional().default([]), truncated: z.boolean().optional().default(false), error: PullRequestTimelineErrorSchema.nullable().optional().default(null), requestId: z.string().optional().default(""), githubFeaturesEnabled: z.boolean().optional().default(true), authState: ForgeAuthStateSchema, }) .optional() .prefault({}), })` | `packages/server/src/server/session/checkout/checkout-session.ts:1174` `handlePullRequestTimelineRequest` (emit) |

## Protocol: Orchestration skills

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `agent.skills.get_status.request`<br>`AgentSkillsGetStatusRequestSchema`<br>`packages/protocol/src/messages.ts:1659` | C→S | agent.skills.get_status.response; Orchestration skills | `z.object({ type: z.literal("agent.skills.get_status.request"), requestId: z.string() }).strict()` | `packages/server/src/server/session.ts:2508` `dispatchOrchestrationSkillsMessage` (dispatch) |
| `agent.skills.reconcile.request`<br>`AgentSkillsReconcileRequestSchema`<br>`packages/protocol/src/messages.ts:1662` | C→S | agent.skills.reconcile.response; Orchestration skills | `z.object({ type: z.literal("agent.skills.reconcile.request"), requestId: z.string() }).strict()` | `packages/server/src/server/session.ts:2514` `dispatchOrchestrationSkillsMessage` (dispatch) |
| `agent.skills.uninstall.request`<br>`AgentSkillsUninstallRequestSchema`<br>`packages/protocol/src/messages.ts:1665` | C→S | agent.skills.uninstall.response; Orchestration skills | `z.object({ type: z.literal("agent.skills.uninstall.request"), requestId: z.string() }).strict()` | `packages/server/src/server/session.ts:2520` `dispatchOrchestrationSkillsMessage` (dispatch) |
| `agent.skills.save_selection.request`<br>`AgentSkillsSaveSelectionRequestSchema`<br>`packages/protocol/src/messages.ts:1668` | C→S | agent.skills.save_selection.response; Orchestration skills | `z .object({ type: z.literal("agent.skills.save_selection.request"), requestId: z.string(), selection: AgentSkillSelectionSchema, confirmedRemovals: z.array(z.string()).optional(), }) .strict()` | `packages/server/src/server/session.ts:2526` `dispatchOrchestrationSkillsMessage` (dispatch) |
| `agent.skills.import_legacy_selection.request`<br>`AgentSkillsImportLegacySelectionRequestSchema`<br>`packages/protocol/src/messages.ts:1676` | C→S | agent.skills.import_legacy_selection.response; Orchestration skills | `z .object({ type: z.literal("agent.skills.import_legacy_selection.request"), requestId: z.string(), selection: AgentSkillSelectionSchema, }) .strict()` | `packages/server/src/server/session.ts:2536` `dispatchOrchestrationSkillsMessage` (dispatch) |
| `agent.skills.get_status.response`<br>`AgentSkillsGetStatusResponseSchema`<br>`packages/protocol/src/messages.ts:7011` | S→C | agent.skills.get_status.request; Orchestration skills | `z.object({ type: z.literal("agent.skills.get_status.response"), payload: AgentSkillsStatusSchema.extend({ requestId: z.string() }) })` | `packages/server/src/server/session.ts:2495` emitStatus helper; selected at `session.ts:2510` |
| `agent.skills.reconcile.response`<br>`AgentSkillsReconcileResponseSchema`<br>`packages/protocol/src/messages.ts:7014` | S→C | agent.skills.reconcile.request; Orchestration skills | `z.object({ type: z.literal("agent.skills.reconcile.response"), payload: AgentSkillsStatusSchema.extend({ requestId: z.string() }) })` | `packages/server/src/server/session.ts:2495` emitStatus helper; selected at `session.ts:2516` |
| `agent.skills.uninstall.response`<br>`AgentSkillsUninstallResponseSchema`<br>`packages/protocol/src/messages.ts:7017` | S→C | agent.skills.uninstall.request; Orchestration skills | `z.object({ type: z.literal("agent.skills.uninstall.response"), payload: AgentSkillsStatusSchema.extend({ requestId: z.string() }) })` | `packages/server/src/server/session.ts:2495` emitStatus helper; selected at `session.ts:2522` |
| `agent.skills.save_selection.response`<br>`AgentSkillsSaveSelectionResponseSchema`<br>`packages/protocol/src/messages.ts:7020` | S→C | agent.skills.save_selection.request; Orchestration skills | `z.object({ type: z.literal("agent.skills.save_selection.response"), payload: AgentSkillsSaveResultSchema.extend({ requestId: z.string() }), })` | `packages/server/src/server/session.ts:2531` (emit) |
| `agent.skills.import_legacy_selection.response`<br>`AgentSkillsImportLegacySelectionResponseSchema`<br>`packages/protocol/src/messages.ts:7024` | S→C | agent.skills.import_legacy_selection.request; Orchestration skills | `z.object({ type: z.literal("agent.skills.import_legacy_selection.response"), payload: z.object({ requestId: z.string(), imported: z.boolean(), selection: AgentSkillSelectionSchema, }), })` | `packages/server/src/server/session.ts:2541` (emit) |

## Protocol: Project configuration

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `read_project_config_request`<br>`ReadProjectConfigRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1695` | C→S | read_project_config_response; Project configuration | `z.object({ type: z.literal("read_project_config_request"), requestId: z.string(), repoRoot: z.string(), })` | `packages/server/src/server/session.ts:2982` `dispatchAgentConfigMessage` (dispatch) |
| `write_project_config_request`<br>`WriteProjectConfigRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1701` | C→S | write_project_config_response; Project configuration | `z.object({ type: z.literal("write_project_config_request"), requestId: z.string(), repoRoot: z.string(), config: PaseoConfigRawSchema, expectedRevision: PaseoConfigRevisionSchema.nullable(), })` | `packages/server/src/server/session.ts:2984` `dispatchAgentConfigMessage` (dispatch) |
| `read_project_config_response`<br>`ReadProjectConfigResponseMessageSchema`<br>`packages/protocol/src/messages.ts:5283` | S→C | read_project_config_request; Project configuration | `z.object({ type: z.literal("read_project_config_response"), payload: z.union([ z.object({ requestId: z.string(), repoRoot: z.string(), ok: z.literal(true), config: PaseoConfigRawSchema.nullable(), revision: PaseoConfigRevisionSchema.nullable(), hasUncommittedWorktreeSetupChanges: z.boolean().optional(), }), z.object({ requestId: z.string(), repoRoot: z.string(), ok: z.literal(false), error: ProjectConfigRpcErrorSchema, }), ]), })` | `packages/server/src/server/session/project-config/project-config-session.ts:70` `handleReadProjectConfigRequest` (emit)<br>`packages/server/src/server/session/project-config/project-config-session.ts:149` `emitProjectConfigReadFailure` (emit) |
| `write_project_config_response`<br>`WriteProjectConfigResponseMessageSchema`<br>`packages/protocol/src/messages.ts:5305` | S→C | write_project_config_request; Project configuration | `z.object({ type: z.literal("write_project_config_response"), payload: z.union([ z.object({ requestId: z.string(), repoRoot: z.string(), ok: z.literal(true), config: PaseoConfigRawSchema, revision: PaseoConfigRevisionSchema, hasUncommittedWorktreeSetupChanges: z.boolean().optional(), }), z.object({ requestId: z.string(), repoRoot: z.string(), ok: z.literal(false), error: ProjectConfigRpcErrorSchema, }), ]), })` | `packages/server/src/server/session/project-config/project-config-session.ts:117` `handleWriteProjectConfigRequest` (emit)<br>`packages/server/src/server/session/project-config/project-config-session.ts:165` `emitProjectConfigWriteFailure` (emit) |

## Protocol: Provider subagents

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `agent.provider_subagents.list.request`<br>`ProviderSubagentListRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1971` | C→S | agent.provider_subagents.list.response; Provider subagents | `z.object({ type: z.literal("agent.provider_subagents.list.request"), parentAgentId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2816` `dispatchAgentTimelineMessage` (dispatch) |
| `agent.provider_subagents.timeline.get.request`<br>`ProviderSubagentTimelineRequestMessageSchema`<br>`packages/protocol/src/messages.ts:1977` | C→S | agent.provider_subagents.timeline.get.response; Provider subagents | `z.object({ type: z.literal("agent.provider_subagents.timeline.get.request"), parentAgentId: z.string(), subagentId: z.string(), requestId: z.string(), direction: z.enum(["tail", "before", "after"]).optional(), cursor: AgentTimelineCursorSchema.optional(), limit: z.number().int().nonnegative().optional(), })` | `packages/server/src/server/session.ts:2818` `dispatchAgentTimelineMessage` (dispatch) |
| `agent.provider_subagents.list.response`<br>`ProviderSubagentListResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4882` | S→C | agent.provider_subagents.list.request; Provider subagents | `z.object({ type: z.literal("agent.provider_subagents.list.response"), payload: z.object({ requestId: z.string(), parentAgentId: z.string(), subagents: z.array(ProviderSubagentDescriptorPayloadSchema), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:8114` `handleProviderSubagentListRequest` (emit) |
| `agent.provider_subagents.timeline.get.response`<br>`ProviderSubagentTimelineResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4892` | S→C | agent.provider_subagents.timeline.get.request; Provider subagents | `z.object({ type: z.literal("agent.provider_subagents.timeline.get.response"), payload: z.object({ requestId: z.string(), parentAgentId: z.string(), subagentId: z.string(), provider: AgentProviderSchema.nullable(), direction: z.enum(["tail", "before", "after"]), epoch: z.string(), projection: z.literal("projected").optional(), startCursor: AgentTimelineCursorSchema.nullable().optional(), endCursor: AgentTimelineCursorSchema.nullable().optional(), reset: z.boolean(), staleCursor: z.boolean(), gap: z.boolean(), window: z.object({ minSeq: z.number().int().nonnegative(), maxSeq: z.number().int().nonnegative(), nextSeq: z.number().int().nonnegative(), }), hasOlder: z.boolean(), hasNewer: z.boolean(), rows: z.array(z.object({ item: AgentTimelineItemPayloadSchema, timestamp: z.string(), seq: z.number().int().nonnegative(), seqStart: z.number().int().nonnegative().optional(), seqEnd: z.number().int().nonnegative().optional(), sourceSeqRanges: z .array(z.object({ startSeq: z.number().int().nonnegative(), endSeq: z.number().int().nonnegative(), })) .optional(), })), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:8166` `handleProviderSubagentTimelineRequest` (emit) |
| `agent.provider_subagents.update`<br>`ProviderSubagentUpdateMessageSchema`<br>`packages/protocol/src/messages.ts:4935` | S→C | push / shared status or error channel; Provider subagents | `z.object({ type: z.literal("agent.provider_subagents.update"), payload: z.discriminatedUnion("kind", [ z.object({ subscriptionId: z.string().optional(), kind: z.literal("upsert"), subagent: ProviderSubagentDescriptorPayloadSchema, }), z.object({ subscriptionId: z.string().optional(), kind: z.literal("timeline"), parentAgentId: z.string(), subagentId: z.string(), provider: AgentProviderSchema, item: AgentTimelineItemPayloadSchema, timestamp: z.string(), seq: z.number().int().nonnegative(), epoch: z.string(), }), z.object({ subscriptionId: z.string().optional(), kind: z.literal("remove"), parentAgentId: z.string(), subagentId: z.string(), }), ]), })` | `packages/server/src/server/session.ts:1924` `forwardProviderSubagentUpdate` (emit) |

## Protocol: Agent permissions

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `agent_permission_response`<br>`AgentPermissionResponseMessageSchema`<br>`packages/protocol/src/messages.ts:2236` | C→S | agent_permission_request; Agent permissions | `z.object({ type: z.literal("agent_permission_response"), agentId: z.string(), requestId: z.string(), response: AgentPermissionResponseSchema, })` | `packages/server/src/server/session.ts:2930` `dispatchAgentLifecycleMessage` (dispatch) |
| `agent_permission_request`<br>`AgentPermissionRequestMessageSchema`<br>`packages/protocol/src/messages.ts:5327` | S→C | agent_permission_response; Agent permissions | `z.object({ type: z.literal("agent_permission_request"), payload: z.object({ subscriptionId: z.string().optional(), agentId: z.string(), request: AgentPermissionRequestPayloadSchema, }), })` | `packages/server/src/server/session.ts:2087` (emit) |
| `agent_permission_resolved`<br>`AgentPermissionResolvedMessageSchema`<br>`packages/protocol/src/messages.ts:5336` | S→C | agent_permission_response; Agent permissions | `z.object({ type: z.literal("agent_permission_resolved"), payload: z.object({ subscriptionId: z.string().optional(), agentId: z.string(), requestId: z.string(), resolution: AgentPermissionResponseSchema, }), })` | `packages/server/src/server/session.ts:2096` (emit)<br>`packages/server/src/server/session.ts:5199` `handleAgentPermissionResponse` (emit) |

## Protocol: Checkout, Git and forge

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `checkout_status_request`<br>`CheckoutStatusRequestSchema`<br>`packages/protocol/src/messages.ts:2261` | C→S | checkout_status_response; Checkout, Git and forge | `z.object({ type: z.literal("checkout_status_request"), cwd: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2994` `dispatchCheckoutMessage` (dispatch) |
| `checkout.diff.get.request`<br>`CheckoutDiffGetRequestSchema`<br>`packages/protocol/src/messages.ts:2275` | C→S | checkout.diff.get.response; Checkout, Git and forge | `z.object({ type: z.literal("checkout.diff.get.request"), cwd: z.string(), compare: CheckoutDiffCompareSchema, requestId: z.string(), })` | `packages/server/src/server/session.ts:3006` `dispatchCheckoutMessage` (dispatch) |
| `subscribe_checkout_diff_request`<br>`SubscribeCheckoutDiffRequestSchema`<br>`packages/protocol/src/messages.ts:2267` | C→S | subscribe_checkout_diff_response; Checkout, Git and forge | `z.object({ type: z.literal("subscribe_checkout_diff_request"), subscriptionId: z.string().optional(), cwd: z.string(), compare: CheckoutDiffCompareSchema, requestId: z.string(), })` | `packages/server/src/server/session.ts:3008` `dispatchCheckoutMessage` (dispatch) |
| `unsubscribe_checkout_diff_request`<br>`UnsubscribeCheckoutDiffRequestSchema`<br>`packages/protocol/src/messages.ts:2282` | C→S | no separate acknowledgement; Checkout, Git and forge | `z.object({ type: z.literal("unsubscribe_checkout_diff_request"), subscriptionId: z.string(), })` | `packages/server/src/server/session.ts:3010` `dispatchCheckoutMessage` (dispatch) |
| `checkout_commit_request`<br>`CheckoutCommitRequestSchema`<br>`packages/protocol/src/messages.ts:2287` | C→S | checkout_commit_response; Checkout, Git and forge | `z.object({ type: z.literal("checkout_commit_request"), cwd: z.string(), message: z.string().optional(), addAll: z.boolean().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3016` `dispatchCheckoutMessage` (dispatch) |
| `checkout_merge_request`<br>`CheckoutMergeRequestSchema`<br>`packages/protocol/src/messages.ts:2295` | C→S | checkout_merge_response; Checkout, Git and forge | `z.object({ type: z.literal("checkout_merge_request"), cwd: z.string(), baseRef: z.string().optional(), strategy: z.enum(["merge", "squash"]).optional(), requireCleanTarget: z.boolean().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3018` `dispatchCheckoutMessage` (dispatch) |
| `checkout_merge_from_base_request`<br>`CheckoutMergeFromBaseRequestSchema`<br>`packages/protocol/src/messages.ts:2304` | C→S | checkout_merge_from_base_response; Checkout, Git and forge | `z.object({ type: z.literal("checkout_merge_from_base_request"), cwd: z.string(), baseRef: z.string().optional(), requireCleanTarget: z.boolean().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3020` `dispatchCheckoutMessage` (dispatch) |
| `checkout_pull_request`<br>`CheckoutPullRequestSchema`<br>`packages/protocol/src/messages.ts:2312` | C→S | checkout_pull_response; Checkout, Git and forge | `z.object({ type: z.literal("checkout_pull_request"), cwd: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3022` `dispatchCheckoutMessage` (dispatch) |
| `checkout_push_request`<br>`CheckoutPushRequestSchema`<br>`packages/protocol/src/messages.ts:2318` | C→S | checkout_push_response; Checkout, Git and forge | `z.object({ type: z.literal("checkout_push_request"), cwd: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3024` `dispatchCheckoutMessage` (dispatch) |
| `checkout.refresh.request`<br>`CheckoutRefreshRequestSchema`<br>`packages/protocol/src/messages.ts:2324` | C→S | checkout.refresh.response; Checkout, Git and forge | `z.object({ type: z.literal("checkout.refresh.request"), cwd: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3026` `dispatchCheckoutMessage` (dispatch) |
| `checkout.discard_changes.request`<br>`CheckoutDiscardChangesRequestSchema`<br>`packages/protocol/src/messages.ts:2330` | C→S | checkout.discard_changes.response; Checkout, Git and forge | `z.object({ type: z.literal("checkout.discard_changes.request"), cwd: z.string(), paths: z.array(z.string()).min(1), requestId: z.string(), })` | `packages/server/src/server/session.ts:3028` `dispatchCheckoutMessage` (dispatch) |
| `checkout_pr_create_request`<br>`CheckoutPrCreateRequestSchema`<br>`packages/protocol/src/messages.ts:2337` | C→S | checkout_pr_create_response; Checkout, Git and forge | `z.object({ type: z.literal("checkout_pr_create_request"), cwd: z.string(), title: z.string().optional(), body: z.string().optional(), baseRef: z.string().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3030` `dispatchCheckoutMessage` (dispatch) |
| `checkout_pr_merge_request`<br>`CheckoutPrMergeRequestSchema`<br>`packages/protocol/src/messages.ts:2346` | C→S | checkout_pr_merge_response; Checkout, Git and forge | `z.object({ type: z.literal("checkout_pr_merge_request"), cwd: z.string(), mergeMethod: z.enum(["merge", "squash", "rebase"]), requestId: z.string(), })` | `packages/server/src/server/session.ts:3032` `dispatchCheckoutMessage` (dispatch) |
| `checkout.forge.set_auto_merge.request`<br>`CheckoutForgeSetAutoMergeRequestSchema`<br>`packages/protocol/src/messages.ts:2353` | C→S | checkout.forge.set_auto_merge.response; Checkout, Git and forge | `z.object({ type: z.literal("checkout.forge.set_auto_merge.request"), cwd: z.string(), enabled: z.boolean(), mergeMethod: z.enum(["merge", "squash", "rebase"]).optional(), requestId: z.string(), })` | `packages/server/src/server/session/checkout/checkout-session.ts:1058` `handleCheckoutForgeSetAutoMergeRequest` (dispatch)<br>`packages/server/src/server/session.ts:3034` `dispatchCheckoutMessage` (dispatch) |
| `checkout.github.set_auto_merge.request`<br>`CheckoutGithubSetAutoMergeRequestSchema`<br>`packages/protocol/src/messages.ts:2364` | C→S | checkout.github.set_auto_merge.response; Checkout, Git and forge | `z.object({ type: z.literal("checkout.github.set_auto_merge.request"), cwd: z.string(), enabled: z.boolean(), mergeMethod: z.enum(["merge", "squash", "rebase"]).optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3035` `dispatchCheckoutMessage` (dispatch) |
| `checkout.commits.list.request`<br>`CheckoutCommitsListRequestSchema`<br>`packages/protocol/src/messages.ts:2391` | C→S | checkout.commits.list.response; Checkout, Git and forge | `z.object({ type: z.literal("checkout.commits.list.request"), cwd: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2996` `dispatchCheckoutMessage` (dispatch) |
| `checkout.commits.file_diff.request`<br>`CheckoutCommitFileDiffRequestSchema`<br>`packages/protocol/src/messages.ts:2397` | C→S | checkout.commits.file_diff.response; Checkout, Git and forge | `z.object({ type: z.literal("checkout.commits.file_diff.request"), cwd: z.string(), sha: z.string(), path: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2998` `dispatchCheckoutMessage` (dispatch) |
| `checkout.forge.get_check_details.request`<br>`CheckoutForgeGetCheckDetailsRequestSchema`<br>`packages/protocol/src/messages.ts:2427` | C→S | checkout.forge.get_check_details.response; Checkout, Git and forge | `CheckoutCheckDetailsRequestPayloadSchema.extend({ type: z.literal("checkout.forge.get_check_details.request"), })` | `packages/server/src/server/session/checkout/checkout-session.ts:1299` `handleCheckoutForgeGetCheckDetailsRequest` (dispatch)<br>`packages/server/src/server/session.ts:3037` `dispatchCheckoutMessage` (dispatch) |
| `checkout.github.get_check_details.request`<br>`CheckoutGithubGetCheckDetailsRequestSchema`<br>`packages/protocol/src/messages.ts:2435` | C→S | checkout.github.get_check_details.response; Checkout, Git and forge | `CheckoutCheckDetailsRequestPayloadSchema.extend({ type: z.literal("checkout.github.get_check_details.request"), })` | `packages/server/src/server/session.ts:3038` `dispatchCheckoutMessage` (dispatch) |
| `checkout_pr_status_request`<br>`CheckoutPrStatusRequestSchema`<br>`packages/protocol/src/messages.ts:2440` | C→S | checkout_pr_status_response; Checkout, Git and forge | `z.object({ type: z.literal("checkout_pr_status_request"), cwd: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3040` `dispatchCheckoutMessage` (dispatch) |
| `checkout_switch_branch_request`<br>`CheckoutSwitchBranchRequestSchema`<br>`packages/protocol/src/messages.ts:2462` | C→S | checkout_switch_branch_response; Checkout, Git and forge | `z.object({ type: z.literal("checkout_switch_branch_request"), cwd: z.string(), branch: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3012` `dispatchCheckoutMessage` (dispatch) |
| `checkout.rename_branch.request`<br>`CheckoutRenameBranchRequestSchema`<br>`packages/protocol/src/messages.ts:2469` | C→S | checkout.rename_branch.response; Checkout, Git and forge | `z.object({ type: z.literal("checkout.rename_branch.request"), cwd: z.string(), branch: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3014` `dispatchCheckoutMessage` (dispatch) |
| `stash_save_request`<br>`StashSaveRequestSchema`<br>`packages/protocol/src/messages.ts:2476` | C→S | stash_save_response; Checkout, Git and forge | `z.object({ type: z.literal("stash_save_request"), cwd: z.string(), branch: z.string().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3047` `dispatchCheckoutMessage` (dispatch) |
| `stash_pop_request`<br>`StashPopRequestSchema`<br>`packages/protocol/src/messages.ts:2484` | C→S | stash_pop_response; Checkout, Git and forge | `z.object({ type: z.literal("stash_pop_request"), cwd: z.string(), stashIndex: z.number().int().min(0), requestId: z.string(), })` | `packages/server/src/server/session.ts:3049` `dispatchCheckoutMessage` (dispatch) |
| `stash_list_request`<br>`StashListRequestSchema`<br>`packages/protocol/src/messages.ts:2492` | C→S | stash_list_response; Checkout, Git and forge | `z.object({ type: z.literal("stash_list_request"), cwd: z.string(), paseoOnly: z.boolean().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3051` `dispatchCheckoutMessage` (dispatch) |
| `validate_branch_request`<br>`ValidateBranchRequestSchema`<br>`packages/protocol/src/messages.ts:2455` | C→S | validate_branch_response; Checkout, Git and forge | `z.object({ type: z.literal("validate_branch_request"), cwd: z.string(), branchName: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3000` `dispatchCheckoutMessage` (dispatch) |
| `branch_suggestions_request`<br>`BranchSuggestionsRequestSchema`<br>`packages/protocol/src/messages.ts:2500` | C→S | branch_suggestions_response; Checkout, Git and forge | `z.object({ type: z.literal("branch_suggestions_request"), cwd: z.string(), query: z.string().optional(), limit: z.number().int().min(1).max(200).optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3002` `dispatchCheckoutMessage` (dispatch) |
| `forge.search.request`<br>`ForgeSearchRequestSchema`<br>`packages/protocol/src/messages.ts:2540` | C→S | forge.search.response; Checkout, Git and forge | `z.object({ type: z.literal("forge.search.request"), cwd: z.string(), query: z.string(), limit: z.number().int().min(1).max(50).optional(), kinds: z.array(ForgeSearchKindSchema).optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3044` `dispatchCheckoutMessage` (dispatch) |
| `github_search_request`<br>`GitHubSearchRequestSchema`<br>`packages/protocol/src/messages.ts:2552` | C→S | github_search_response; Checkout, Git and forge | `z.object({ type: z.literal("github_search_request"), cwd: z.string(), query: z.string(), limit: z.number().int().min(1).max(50).optional(), kinds: z.array(GitHubSearchKindSchema).optional(), requestId: z.string(), })` | `packages/server/src/server/session/checkout/checkout-session.ts:1364` `handleForgeSearchRequest` (dispatch)<br>`packages/server/src/server/session.ts:3045` `dispatchCheckoutMessage` (dispatch) |
| `checkout_status_response`<br>`CheckoutStatusResponseSchema`<br>`packages/protocol/src/messages.ts:5444` | S→C | checkout_status_request; Checkout, Git and forge | `z.object({ type: z.literal("checkout_status_response"), payload: z.union([ CheckoutStatusNotGitSchema, CheckoutStatusGitNonPaseoSchema, CheckoutStatusGitPaseoSchema, ]), })` | `packages/server/src/server/session/checkout/checkout-session.ts:243` `handleStatusRequest` (emit) |
| `checkout_status_update`<br>`CheckoutStatusUpdateSchema`<br>`packages/protocol/src/messages.ts:5584` | S→C | push / shared status or error channel; Checkout, Git and forge | `z.object({ type: z.literal("checkout_status_update"), payload: z .union([ CheckoutStatusNotGitSchema, CheckoutStatusGitNonPaseoSchema, CheckoutStatusGitPaseoSchema, ]) .and(CheckoutStatusUpdateMetadataSchema), })` | `packages/server/src/server/session/checkout/checkout-session.ts:538` `emitStatusUpdate` (emit) |
| `checkout.diff.get.response`<br>`CheckoutDiffGetResponseSchema`<br>`packages/protocol/src/messages.ts:5611` | S→C | checkout.diff.get.request; Checkout, Git and forge | `z.object({ type: z.literal("checkout.diff.get.response"), payload: CheckoutDiffSubscriptionPayloadSchema.omit({ subscriptionId: true }).extend({ requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:420` `handleReadDiffRequest` (emit) |
| `subscribe_checkout_diff_response`<br>`SubscribeCheckoutDiffResponseSchema`<br>`packages/protocol/src/messages.ts:5604` | S→C | subscribe_checkout_diff_request; Checkout, Git and forge | `z.object({ type: z.literal("subscribe_checkout_diff_response"), payload: CheckoutDiffSubscriptionPayloadSchema.extend({ requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:463` `handleSubscribeDiffRequest` (emit) |
| `checkout_diff_update`<br>`CheckoutDiffUpdateSchema`<br>`packages/protocol/src/messages.ts:5618` | S→C | push / shared status or error channel; Checkout, Git and forge | `z.object({ type: z.literal("checkout_diff_update"), payload: CheckoutDiffSubscriptionPayloadSchema, })` | `packages/server/src/server/session/checkout/checkout-session.ts:445` (emit) |
| `checkout_commit_response`<br>`CheckoutCommitResponseSchema`<br>`packages/protocol/src/messages.ts:5623` | S→C | checkout_commit_request; Checkout, Git and forge | `z.object({ type: z.literal("checkout_commit_response"), payload: z.object({ cwd: z.string(), success: z.boolean(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:764` `handleCheckoutCommitRequest` (emit) |
| `checkout_merge_response`<br>`CheckoutMergeResponseSchema`<br>`packages/protocol/src/messages.ts:5633` | S→C | checkout_merge_request; Checkout, Git and forge | `z.object({ type: z.literal("checkout_merge_response"), payload: z.object({ cwd: z.string(), success: z.boolean(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:825` `handleCheckoutMergeRequest` (emit) |
| `checkout_merge_from_base_response`<br>`CheckoutMergeFromBaseResponseSchema`<br>`packages/protocol/src/messages.ts:5643` | S→C | checkout_merge_from_base_request; Checkout, Git and forge | `z.object({ type: z.literal("checkout_merge_from_base_response"), payload: z.object({ cwd: z.string(), success: z.boolean(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:871` `handleCheckoutMergeFromBaseRequest` (emit) |
| `checkout_pull_response`<br>`CheckoutPullResponseSchema`<br>`packages/protocol/src/messages.ts:5653` | S→C | checkout_pull_request; Checkout, Git and forge | `z.object({ type: z.literal("checkout_pull_response"), payload: z.object({ cwd: z.string(), success: z.boolean(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:903` `handleCheckoutPullRequest` (emit) |
| `checkout_push_response`<br>`CheckoutPushResponseSchema`<br>`packages/protocol/src/messages.ts:5663` | S→C | checkout_push_request; Checkout, Git and forge | `z.object({ type: z.literal("checkout_push_response"), payload: z.object({ cwd: z.string(), success: z.boolean(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:933` `handleCheckoutPushRequest` (emit) |
| `checkout.refresh.response`<br>`CheckoutRefreshResponseSchema`<br>`packages/protocol/src/messages.ts:5673` | S→C | checkout.refresh.request; Checkout, Git and forge | `z.object({ type: z.literal("checkout.refresh.response"), payload: z.object({ cwd: z.string(), success: z.boolean(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:498` `handleRefreshRequest` (emit) |
| `checkout.discard_changes.response`<br>`CheckoutDiscardChangesResponseSchema`<br>`packages/protocol/src/messages.ts:5729` | S→C | checkout.discard_changes.request; Checkout, Git and forge | `z.object({ type: z.literal("checkout.discard_changes.response"), payload: z.object({ cwd: z.string(), success: z.boolean(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:661` `handleCheckoutDiscardChangesRequest` (emit) |
| `checkout_pr_create_response`<br>`CheckoutPrCreateResponseSchema`<br>`packages/protocol/src/messages.ts:5683` | S→C | checkout_pr_create_request; Checkout, Git and forge | `z.object({ type: z.literal("checkout_pr_create_response"), payload: z.object({ cwd: z.string(), url: z.string().nullable(), number: z.number().nullable(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:983` `handleCheckoutPrCreateRequest` (emit) |
| `checkout_pr_merge_response`<br>`CheckoutPrMergeResponseSchema`<br>`packages/protocol/src/messages.ts:5694` | S→C | checkout_pr_merge_request; Checkout, Git and forge | `z.object({ type: z.literal("checkout_pr_merge_response"), payload: z.object({ cwd: z.string(), success: z.boolean(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:1027` `handleCheckoutPrMergeRequest` (emit) |
| `checkout.forge.set_auto_merge.response`<br>`CheckoutForgeSetAutoMergeResponseSchema`<br>`packages/protocol/src/messages.ts:5704` | S→C | checkout.forge.set_auto_merge.request; Checkout, Git and forge | `z.object({ type: z.literal("checkout.forge.set_auto_merge.response"), payload: z.object({ cwd: z.string(), enabled: z.boolean(), success: z.boolean(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:1048` handleCheckoutForgeSetAutoMergeRequest; responseType selected at :1059 |
| `checkout.github.set_auto_merge.response`<br>`CheckoutGithubSetAutoMergeResponseSchema`<br>`packages/protocol/src/messages.ts:5718` | S→C | checkout.github.set_auto_merge.request; Checkout, Git and forge | `z.object({ type: z.literal("checkout.github.set_auto_merge.response"), payload: z.object({ cwd: z.string(), enabled: z.boolean(), success: z.boolean(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:1048` handleCheckoutForgeSetAutoMergeRequest; responseType selected at :1060 |
| `checkout.commits.list.response`<br>`CheckoutCommitsListResponseSchema`<br>`packages/protocol/src/messages.ts:5739` | S→C | checkout.commits.list.request; Checkout, Git and forge | `z.object({ type: z.literal("checkout.commits.list.response"), payload: z.object({ cwd: z.string(), baseRef: z.string().nullable(), commits: z.array(CheckoutCommitSchema), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:282` `handleCommitsListRequest` (emit) |
| `checkout.commits.file_diff.response`<br>`CheckoutCommitFileDiffResponseSchema`<br>`packages/protocol/src/messages.ts:5750` | S→C | checkout.commits.file_diff.request; Checkout, Git and forge | `z.object({ type: z.literal("checkout.commits.file_diff.response"), payload: z.object({ cwd: z.string(), sha: z.string(), path: z.string(), file: ParsedDiffFileSchema.nullable(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:306` `handleCommitFileDiffRequest` (emit) |
| `checkout.forge.get_check_details.response`<br>`CheckoutForgeGetCheckDetailsResponseSchema`<br>`packages/protocol/src/messages.ts:5842` | S→C | checkout.forge.get_check_details.request; Checkout, Git and forge | `z.object({ type: z.literal("checkout.forge.get_check_details.response"), payload: z.object({ cwd: z.string(), success: z.boolean(), details: CheckoutCheckDetailsSchema.nullable().optional().default(null), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:1286` handleCheckoutForgeGetCheckDetailsRequest; responseType selected at :1300 |
| `checkout.github.get_check_details.response`<br>`CheckoutGithubGetCheckDetailsResponseSchema`<br>`packages/protocol/src/messages.ts:5856` | S→C | checkout.github.get_check_details.request; Checkout, Git and forge | `z.object({ type: z.literal("checkout.github.get_check_details.response"), payload: z.object({ cwd: z.string(), success: z.boolean(), details: CheckoutCheckDetailsSchema.nullable().optional().default(null), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:1286` handleCheckoutForgeGetCheckDetailsRequest; responseType selected at :1301 |
| `checkout_pr_status_response`<br>`CheckoutPrStatusResponseSchema`<br>`packages/protocol/src/messages.ts:5867` | S→C | checkout_pr_status_request; Checkout, Git and forge | `z.object({ type: z.literal("checkout_pr_status_response"), payload: CheckoutPrStatusPayloadSchema, })` | `packages/server/src/server/session/checkout/checkout-session.ts:1143` `handleCheckoutPrStatusRequest` (emit) |
| `checkout_switch_branch_response`<br>`CheckoutSwitchBranchResponseSchema`<br>`packages/protocol/src/messages.ts:5985` | S→C | checkout_switch_branch_request; Checkout, Git and forge | `z.object({ type: z.literal("checkout_switch_branch_response"), payload: z.object({ cwd: z.string(), success: z.boolean(), branch: z.string(), source: z.enum(["local", "remote"]).optional(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:572` `handleCheckoutSwitchBranchRequest` (emit) |
| `checkout.rename_branch.response`<br>`CheckoutRenameBranchResponseSchema`<br>`packages/protocol/src/messages.ts:5997` | S→C | checkout.rename_branch.request; Checkout, Git and forge | `z.object({ type: z.literal("checkout.rename_branch.response"), payload: z.object({ requestId: z.string(), success: z.boolean(), cwd: z.string(), currentBranch: z.string().nullable(), error: CheckoutErrorSchema.nullable(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:602` `handleCheckoutRenameBranchRequest` (emit) |
| `stash_save_response`<br>`StashSaveResponseSchema`<br>`packages/protocol/src/messages.ts:6015` | S→C | stash_save_request; Checkout, Git and forge | `z.object({ type: z.literal("stash_save_response"), payload: z.object({ cwd: z.string(), success: z.boolean(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:688` `handleStashSaveRequest` (emit) |
| `stash_pop_response`<br>`StashPopResponseSchema`<br>`packages/protocol/src/messages.ts:6025` | S→C | stash_pop_request; Checkout, Git and forge | `z.object({ type: z.literal("stash_pop_response"), payload: z.object({ cwd: z.string(), success: z.boolean(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:711` `handleStashPopRequest` (emit) |
| `stash_list_response`<br>`StashListResponseSchema`<br>`packages/protocol/src/messages.ts:6035` | S→C | stash_list_request; Checkout, Git and forge | `z.object({ type: z.literal("stash_list_response"), payload: z.object({ cwd: z.string(), entries: z.array(StashEntrySchema), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:731` `handleStashListRequest` (emit) |
| `validate_branch_response`<br>`ValidateBranchResponseSchema`<br>`packages/protocol/src/messages.ts:6045` | S→C | validate_branch_request; Checkout, Git and forge | `z.object({ type: z.literal("validate_branch_response"), payload: z.object({ exists: z.boolean(), resolvedRef: z.string().nullable(), isRemote: z.boolean(), error: z.string().nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:328` `handleValidateBranchRequest` (emit) |
| `branch_suggestions_response`<br>`BranchSuggestionsResponseSchema`<br>`packages/protocol/src/messages.ts:6056` | S→C | branch_suggestions_request; Checkout, Git and forge | `z.object({ type: z.literal("branch_suggestions_response"), payload: z.object({ branches: z.array(z.string()), branchDetails: z .array(z.object({ name: z.string(), committerDate: z.number(), hasLocal: z.boolean().optional(), hasRemote: z.boolean().optional(), localAhead: z.number().int().nonnegative().optional(), localBehind: z.number().int().nonnegative().optional(), })) .optional(), error: z.string().nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/checkout/checkout-session.ts:391` `handleBranchSuggestionsRequest` (emit) |
| `forge.search.response`<br>`ForgeSearchResponseSchema`<br>`packages/protocol/src/messages.ts:6093` | S→C | forge.search.request; Checkout, Git and forge | `z.object({ type: z.literal("forge.search.response"), payload: ForgeSearchResponsePayloadSchema, })` | `packages/server/src/server/session/checkout/checkout-session.ts:1383` `handleForgeSearchRequest` (emit) |
| `github_search_response`<br>`GitHubSearchResponseSchema`<br>`packages/protocol/src/messages.ts:6101` | S→C | github_search_request; Checkout, Git and forge | `z.object({ type: z.literal("github_search_response"), payload: GitHubSearchResponsePayloadSchema, })` | `packages/server/src/server/session/checkout/checkout-session.ts:1370` `handleForgeSearchRequest` (emit) |

## Protocol: Legacy managed worktrees and directories

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `directory_suggestions_request`<br>`DirectorySuggestionsRequestSchema`<br>`packages/protocol/src/messages.ts:2561` | C→S | directory_suggestions_response; Legacy managed worktrees and directories | `z.object({ type: z.literal("directory_suggestions_request"), query: z.string(), cwd: z.string().optional(), includeFiles: z.boolean().optional(), includeDirectories: z.boolean().optional(), matchMode: z.enum(["fuzzy", "suffix"]).optional(), limit: z.number().int().min(1).max(100).optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3004` `dispatchCheckoutMessage` (dispatch) |
| `paseo_worktree_list_request`<br>`PaseoWorktreeListRequestSchema`<br>`packages/protocol/src/messages.ts:2572` | C→S | paseo_worktree_list_response; Legacy managed worktrees and directories | `z.object({ type: z.literal("paseo_worktree_list_request"), cwd: z.string().optional(), repoRoot: z.string().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3068` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `paseo_worktree_archive_request`<br>`PaseoWorktreeArchiveRequestSchema`<br>`packages/protocol/src/messages.ts:2579` | C→S | paseo_worktree_archive_response; Legacy managed worktrees and directories | `z.object({ type: z.literal("paseo_worktree_archive_request"), worktreePath: z.string().optional(), repoRoot: z.string().optional(), branchName: z.string().optional(), workspaceId: z.string().optional(), scope: z.enum(["workspace", "worktree"]).optional().default("workspace"), deleteWorktreeFromDisk: z.boolean().optional().default(false), requestId: z.string(), })` | `packages/server/src/server/session.ts:3070` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `create_paseo_worktree_request`<br>`CreatePaseoWorktreeRequestSchema`<br>`packages/protocol/src/messages.ts:2608` | C→S | create_paseo_worktree_response; Legacy managed worktrees and directories | `z.object({ type: z.literal("create_paseo_worktree_request"), cwd: z.string(), projectId: z.string().optional(), worktreeSlug: z.string().optional(), nameContext: z.string().optional(), attachments: AgentAttachmentsSchema.optional(), firstAgentContext: FirstAgentContextSchema.optional(), refName: z.string().min(1).optional(), action: z.enum(["branch-off", "checkout"]).optional(), checkoutSource: ChangeRequestCheckoutSourceSchema.optional(), githubPrNumber: z.number().int().positive().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3072` `dispatchWorkspaceAndProjectMessage` (dispatch) |
| `directory_suggestions_response`<br>`DirectorySuggestionsResponseSchema`<br>`packages/protocol/src/messages.ts:6106` | S→C | directory_suggestions_request; Legacy managed worktrees and directories | `z.object({ type: z.literal("directory_suggestions_response"), payload: z.object({ directories: z.array(z.string()), entries: z .array(z.object({ path: z.string(), kind: z.enum(["file", "directory"]), })) .optional() .default([]), error: z.string().nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session.ts:5254` `handleDirectorySuggestionsRequest` (emit) |
| `paseo_worktree_list_response`<br>`PaseoWorktreeListResponseSchema`<br>`packages/protocol/src/messages.ts:6131` | S→C | paseo_worktree_list_request; Legacy managed worktrees and directories | `z.object({ type: z.literal("paseo_worktree_list_response"), payload: z.object({ worktrees: z.array(PaseoWorktreeSchema), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/worktree-session.ts:153` `handlePaseoWorktreeListRequest` (emit) |
| `paseo_worktree_archive_response`<br>`PaseoWorktreeArchiveResponseSchema`<br>`packages/protocol/src/messages.ts:6140` | S→C | paseo_worktree_archive_request; Legacy managed worktrees and directories | `z.object({ type: z.literal("paseo_worktree_archive_response"), payload: z.object({ success: z.boolean(), removedAgents: z.array(z.string()).optional(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), }), })` | `packages/server/src/server/worktree-session.ts:220` `handlePaseoWorktreeArchiveRequest` (emit) |
| `create_paseo_worktree_response`<br>`CreatePaseoWorktreeResponseSchema`<br>`packages/protocol/src/messages.ts:6150` | S→C | create_paseo_worktree_request; Legacy managed worktrees and directories | `z.object({ type: z.literal("create_paseo_worktree_response"), payload: z.object({ workspace: WorkspaceDescriptorPayloadSchema.nullable(), error: z.string().nullable(), errorCode: z.string().optional(), setupTerminalId: z.string().nullable(), setupSkippedReason: z.string().optional(), requestId: z.string(), }), })` | `packages/server/src/server/worktree-session.ts:285` `handleCreatePaseoWorktreeRequest` (emit) |

## Protocol: Workspace setup

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `workspace_setup_status_request`<br>`WorkspaceSetupStatusRequestSchema`<br>`packages/protocol/src/messages.ts:2626` | C→S | workspace_setup_status_response; Workspace setup | `z.object({ type: z.literal("workspace_setup_status_request"), workspaceId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3103` `dispatchWorkspaceSetupMessage` (dispatch) |
| `workspace.setup.subscribe.request`<br>`WorkspaceSetupSubscribeRequestSchema`<br>`packages/protocol/src/messages.ts:2632` | C→S | workspace.setup.subscribe.response; Workspace setup | `z.object({ type: z.literal("workspace.setup.subscribe.request"), workspaceId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:2419` `dispatchSubscriptionMessage` (dispatch) |
| `workspace.setup.run.request`<br>`WorkspaceSetupRunRequestSchema`<br>`packages/protocol/src/messages.ts:2638` | C→S | workspace.setup.run.response; Workspace setup | `z.object({ type: z.literal("workspace.setup.run.request"), workspaceId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3106` `dispatchWorkspaceSetupMessage` (dispatch) |
| `workspace_setup_progress`<br>`WorkspaceSetupProgressMessageSchema`<br>`packages/protocol/src/messages.ts:4542` | S→C | push / shared status or error channel; Workspace setup | `z.object({ type: z.literal("workspace_setup_progress"), payload: z.object({ subscriptionId: z.string().optional(), workspaceId: z.string(), status: z.enum(["running", "completed", "failed", "blocked"]), detail: WorktreeSetupDetailPayloadSchema, error: z.string().nullable(), blockedSource: z .object({ kind: z.literal("change_request"), forge: z.string(), number: z.number().int().positive(), headRepository: z.string(), }) .optional(), }), })` | `packages/server/src/server/session.ts:1210` (emit)<br>`packages/server/src/server/worktree-session.ts:374` `createPaseoWorktreeWorkflow` (emit) |
| `workspace_setup_status_response`<br>`WorkspaceSetupStatusResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4595` | S→C | workspace_setup_status_request; Workspace setup | `z.object({ type: z.literal("workspace_setup_status_response"), payload: z.object({ requestId: z.string(), workspaceId: z.string(), snapshot: WorkspaceSetupSnapshotSchema.nullable(), }), })` | `packages/server/src/server/worktree-session.ts:443` `handleWorkspaceSetupStatusRequest` (emit) |
| `workspace.setup.subscribe.response`<br>`WorkspaceSetupSubscribeResponseSchema`<br>`packages/protocol/src/messages.ts:4575` | S→C | workspace.setup.subscribe.request; Workspace setup | `z.object({ type: z.literal("workspace.setup.subscribe.response"), payload: z.object({ requestId: z.string(), subscriptionId: z.string().optional(), workspaceId: z.string(), snapshot: WorkspaceSetupSnapshotSchema.nullable(), }), })` | `packages/server/src/server/session.ts:2405` `subscribeWorkspaceSetup` (emit) |
| `workspace.setup.run.response`<br>`WorkspaceSetupRunResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4585` | S→C | workspace.setup.run.request; Workspace setup | `z.object({ type: z.literal("workspace.setup.run.response"), payload: z.object({ requestId: z.string(), workspaceId: z.string(), started: z.boolean(), error: z.string().nullable(), }), })` | `packages/server/src/server/worktree-session.ts:486` `handleWorkspaceSetupRunRequest` (emit) |

## Protocol: Creation lifecycle and receipts

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `workspace.create.request`<br>`WorkspaceCreateRequestSchema`<br>`packages/protocol/src/messages.ts:2718` | C→S | workspace.create.response; Creation lifecycle and receipts | `z.object({ type: z.literal("workspace.create.request"), workspaceId: z .string() .regex(/^wks_[a-f0-9]{16}$/) .optional(), agent: WorkspaceInitialAgentSchema.optional(), subscribe: z.boolean().optional(), requestId: z.string(), idempotencyKey: z.string().min(1).max(512).optional(), title: z.string().optional(), firstAgentContext: FirstAgentContextSchema.optional(), internal: z.boolean().optional(), background: z.boolean().optional(), callerAgentId: z.string().optional(), source: z.discriminatedUnion("kind", [ z.object({ kind: z.literal("directory"), path: z.string(), projectId: z.string().optional(), }), z.object({ kind: z.literal("worktree"), cwd: z.string().optional(), projectId: z.string().optional(), action: z.enum(["branch-off", "checkout"]).optional(), refName: z.string().min(1).optional(), baseBranch: z.string().optional(), branchName: z.string().min(1).optional(), checkoutSource: ChangeRequestCheckoutSourceSchema.optional(), githubPrNumber: z.number().int().positive().optional(), worktreeSlug: z.string().optional(), }), ]), })` | `packages/server/src/server/session.ts:2887` `dispatchCreationMessage` (dispatch) |
| `agent.create.request`<br>`AgentCreateRequestSchema`<br>`packages/protocol/src/messages.ts:1804` | C→S | agent.create.response; Creation lifecycle and receipts | `CreateAgentRequestMessageSchema.extend({ type: z.literal("agent.create.request"), agentId: z.uuid().optional(), attachments: z.array(AgentAttachmentSchema).optional(), subscribe: z.boolean().optional(), })` | `packages/server/src/server/session.ts:2883` `dispatchCreationMessage` (dispatch) |
| `creation.subscribe.request`<br>`CreationSubscribeRequestSchema`<br>`packages/protocol/src/messages.ts:1822` | C→S | creation.subscribe.response; Creation lifecycle and receipts | `z.object({ type: z.literal("creation.subscribe.request"), requestId: z.string(), kind: z.enum(["workspace", "agent"]), idempotencyKey: z.string().min(1).max(512), subscribe: z.boolean().optional(), })` | `packages/server/src/server/session.ts:2885` `dispatchCreationMessage` (dispatch) |
| `workspace.create.response`<br>`WorkspaceCreateResponseSchema`<br>`packages/protocol/src/messages.ts:5079` | S→C | workspace.create.request; Creation lifecycle and receipts | `z.object({ type: z.literal("workspace.create.response"), payload: z.object({ agent: AgentSnapshotPayloadSchema.optional(), creation: CreationSnapshotSchema.optional(), workspace: WorkspaceDescriptorPayloadSchema.nullable(), setupTerminalId: z.string().nullable(), setupSkippedReason: z.string().optional(), error: z.string().nullable(), errorCode: z.string().optional(), requestId: z.string(), }), })` | `packages/server/src/server/session.ts:4315` `handleWorkspaceCreation` (emit) |
| `agent.create.response`<br>`AgentCreateResponseSchema`<br>`packages/protocol/src/messages.ts:5069` | S→C | agent.create.request; Creation lifecycle and receipts | `z.object({ type: z.literal("agent.create.response"), payload: z.object({ requestId: z.string(), agent: AgentSnapshotPayloadSchema.nullable(), error: z.string().nullable(), creation: CreationSnapshotSchema.optional(), }), })` | `packages/server/src/server/session.ts:4208` `handleAgentCreation` (emit) |
| `workspace.create.update`<br>`WorkspaceCreationUpdateSchema`<br>`packages/protocol/src/messages.ts:5053` | S→C | push / shared status or error channel; Creation lifecycle and receipts | `z.object({ type: z.literal("workspace.create.update"), payload: CreationSnapshotSchema.extend({ subscriptionId: z.string().optional() }), })` | `packages/server/src/server/session.ts:4197` (dispatch) |
| `agent.create.update`<br>`AgentCreationUpdateSchema`<br>`packages/protocol/src/messages.ts:5057` | S→C | push / shared status or error channel; Creation lifecycle and receipts | `WorkspaceCreationUpdateSchema.extend({ type: z.literal("agent.create.update"), })` | `packages/server/src/server/session.ts:4197` (dispatch) |
| `creation.subscribe.response`<br>`CreationSubscribeResponseSchema`<br>`packages/protocol/src/messages.ts:5060` | S→C | creation.subscribe.request; Creation lifecycle and receipts | `z.object({ type: z.literal("creation.subscribe.response"), payload: z.object({ requestId: z.string(), subscriptionId: z.string().optional(), snapshot: CreationSnapshotSchema.nullable(), error: z.string().nullable(), }), })` | `packages/server/src/server/session.ts:4123` `handleCreationSubscription` (emit) |

## Protocol: Files and binary transfer

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `file_explorer_request`<br>`FileExplorerRequestSchema`<br>`packages/protocol/src/messages.ts:2833` | C→S | file_explorer_response; Files and binary transfer | `z.object({ type: z.literal("file_explorer_request"), cwd: z.string(), path: z.string().optional(), mode: z.enum(["list", "file"]), requestId: z.string(), acceptBinary: z.boolean().optional(), maxBytes: z.number().int().positive().optional(), })` | `packages/server/src/server/session/owned-subscriptions/index.ts:402` `authorizeFileReply` (dispatch)<br>`packages/server/src/server/session.ts:3134` `dispatchWorkspaceFileMessage` (dispatch) |
| `fs.file.subscribe.request`<br>`FileSubscribeRequestSchema`<br>`packages/protocol/src/messages.ts:2869` | C→S | fs.file.subscribe.response; Files and binary transfer | `z.object({ type: z.literal("fs.file.subscribe.request"), cwd: z.string(), path: z.string(), subscriptionId: z.string().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3136` `dispatchWorkspaceFileMessage` (dispatch) |
| `fs.file.unsubscribe.request`<br>`FileUnsubscribeRequestSchema`<br>`packages/protocol/src/messages.ts:2877` | C→S | fs.file.unsubscribe.response; Files and binary transfer | `z.object({ type: z.literal("fs.file.unsubscribe.request"), subscriptionId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3138` `dispatchWorkspaceFileMessage` (dispatch) |
| `fs.file.write.request`<br>`FileWriteRequestSchema`<br>`packages/protocol/src/messages.ts:2883` | C→S | fs.file.write.response; Files and binary transfer | `z.object({ type: z.literal("fs.file.write.request"), cwd: z.string(), path: z.string(), content: z.string(), contentEncoding: z.enum(["utf-8", "base64"]).optional(), expectedModifiedAt: z.string(), expectedRevision: z.string().optional(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3140` `dispatchWorkspaceFileMessage` (dispatch) |
| `project_icon_request`<br>`ProjectIconRequestSchema`<br>`packages/protocol/src/messages.ts:2926` | C→S | project_icon_response; Files and binary transfer | `z.object({ type: z.literal("project_icon_request"), cwd: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3150` `dispatchWorkspaceFileMessage` (dispatch) |
| `file_download_token_request`<br>`FileDownloadTokenRequestSchema`<br>`packages/protocol/src/messages.ts:2938` | C→S | file_download_token_response; Files and binary transfer | `z.object({ type: z.literal("file_download_token_request"), cwd: z.string(), path: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3154` `dispatchWorkspaceFileMessage` (dispatch) |
| `file.upload.request`<br>`FileUploadRequestSchema`<br>`packages/protocol/src/messages.ts:2945` | C→S | file.upload.response; Files and binary transfer | `z.object({ type: z.literal("file.upload.request"), fileName: z.string().min(1), mimeType: z.string().min(1), size: z.number().int().nonnegative(), modifiedAt: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3156` `dispatchWorkspaceFileMessage` (dispatch) |
| `file_explorer_response`<br>`FileExplorerResponseSchema`<br>`packages/protocol/src/messages.ts:6162` | S→C | file_explorer_request; Files and binary transfer | `z.object({ type: z.literal("file_explorer_response"), payload: z.object({ cwd: z.string(), path: z.string(), mode: z.enum(["list", "file"]), directory: FileExplorerDirectorySchema.nullable(), file: FileExplorerFileSchema.nullable(), error: z.string().nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/files/workspace-files-session.ts:253` `handleFileExplorerRequest` (emit) |
| `fs.file.subscribe.response`<br>`FileSubscribeResponseSchema`<br>`packages/protocol/src/messages.ts:6175` | S→C | fs.file.subscribe.request; Files and binary transfer | `z.object({ type: z.literal("fs.file.subscribe.response"), payload: z.object({ subscriptionId: z.string(), initial: FileVersionSchema, requestId: z.string(), }), })` | `packages/server/src/server/session/files/workspace-files-session.ts:120` `handleFileSubscribeRequest` (emit) |
| `fs.file.unsubscribe.response`<br>`FileUnsubscribeResponseSchema`<br>`packages/protocol/src/messages.ts:6184` | S→C | fs.file.unsubscribe.request; Files and binary transfer | `z.object({ type: z.literal("fs.file.unsubscribe.response"), payload: z.object({ subscriptionId: z.string(), requestId: z.string(), }), })` | `packages/server/src/server/session/files/workspace-files-session.ts:153` `handleFileUnsubscribeRequest` (emit) |
| `fs.file.write.response`<br>`FileWriteResponseSchema`<br>`packages/protocol/src/messages.ts:6203` | S→C | fs.file.write.request; Files and binary transfer | `z.object({ type: z.literal("fs.file.write.response"), payload: z.object({ result: FileWriteResultSchema, requestId: z.string(), }), })` | `packages/server/src/server/session/files/workspace-files-session.ts:168` `handleFileWriteRequest` (emit) |
| `fs.file.update`<br>`FileUpdateSchema`<br>`packages/protocol/src/messages.ts:6258` | S→C | push / shared status or error channel; Files and binary transfer | `z.object({ type: z.literal("fs.file.update"), payload: z.object({ subscriptionId: z.string(), version: FileVersionSchema, }), })` | `packages/server/src/server/session/files/workspace-files-session.ts:102` (emit) |
| `project_icon_response`<br>`ProjectIconResponseSchema`<br>`packages/protocol/src/messages.ts:6271` | S→C | project_icon_request; Files and binary transfer | `z.object({ type: z.literal("project_icon_response"), payload: z.object({ cwd: z.string(), icon: ProjectIconSchema.nullable(), error: z.string().nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/files/workspace-files-session.ts:405` `handleProjectIconRequest` (emit) |
| `file_download_token_response`<br>`FileDownloadTokenResponseSchema`<br>`packages/protocol/src/messages.ts:6291` | S→C | file_download_token_request; Files and binary transfer | `z.object({ type: z.literal("file_download_token_response"), payload: z.object({ cwd: z.string(), path: z.string(), token: z.string().nullable(), fileName: z.string().nullable(), mimeType: z.string().nullable(), size: z.number().nullable(), error: z.string().nullable(), requestId: z.string(), }), })` | `packages/server/src/server/session/files/workspace-files-session.ts:431` `handleFileDownloadTokenRequest` (emit) |
| `file.upload.response`<br>`FileUploadResponseSchema`<br>`packages/protocol/src/messages.ts:6305` | S→C | file.upload.request; Files and binary transfer | `z.object({ type: z.literal("file.upload.response"), payload: z.object({ requestId: z.string(), file: UploadedFileAttachmentSchema.nullable(), error: z.string().nullable(), }), })` | `packages/server/src/server/file-upload/index.ts:205` `buildUploadResponse` (emit) |

## Protocol: Presence, attention and notifications

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `clear_agent_attention`<br>`ClearAgentAttentionMessageSchema`<br>`packages/protocol/src/messages.ts:2954` | C→S | clear_agent_attention_response; Presence, attention and notifications | `z.object({ type: z.literal("clear_agent_attention"), agentId: z.union([z.string(), z.array(z.string())]), requestId: z.string().optional(), })` | `packages/server/src/server/session.ts:2932` `dispatchAgentLifecycleMessage` (dispatch) |
| `client_heartbeat`<br>`ClientHeartbeatMessageSchema`<br>`packages/protocol/src/messages.ts:2960` | C→S | no separate acknowledgement; Presence, attention and notifications | `z.object({ type: z.literal("client_heartbeat"), deviceType: z.enum(["web", "mobile"]), focusedAgentId: z.string().nullable(), focusedTerminalId: z.string().nullable().optional().default(null), lastActivityAt: z.string(), appVisible: z.boolean(), appVisibilityChangedAt: z.string().optional(), })` | `packages/server/src/server/session.ts:2761` `dispatchVoiceAndControlMessage` (dispatch) |
| `register_push_token`<br>`RegisterPushTokenMessageSchema`<br>`packages/protocol/src/messages.ts:2999` | C→S | no separate acknowledgement; Presence, attention and notifications | `z.object({ type: z.literal("register_push_token"), token: z.string(), })` | `packages/server/src/server/session.ts:3249` `dispatchMiscMessage` (dispatch) |
| `push.unregister.request`<br>`PushUnregisterRequestSchema`<br>`packages/protocol/src/messages.ts:3004` | C→S | push.unregister.response; Presence, attention and notifications | `z.object({ type: z.literal("push.unregister.request"), token: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3252` `dispatchMiscMessage` (dispatch) |
| `push.unregister.response`<br>`PushUnregisterResponseSchema`<br>`packages/protocol/src/messages.ts:3010` | S→C | push.unregister.request; Presence, attention and notifications | `z.object({ type: z.literal("push.unregister.response"), payload: z.object({ requestId: z.string(), }), })` | `packages/server/src/server/session.ts:3258` `dispatchMiscMessage` (emit) |
| `agent_attention_required`<br>`AgentAttentionRequiredMessageSchema`<br>`packages/protocol/src/messages.ts:4972` | S→C | push / shared status or error channel; Presence, attention and notifications | `z.object({ type: z.literal("agent_attention_required"), payload: z.object({ subscriptionId: z.string().optional(), agentId: z.string(), reason: z.enum(["finished", "error", "permission"]), timestamp: z.string(), shouldNotify: z.boolean(), notification: z .object({ title: z.string(), body: z.string(), data: z.object({ serverId: z.string(), workspaceId: z.string().optional(), agentId: z.string(), reason: z.enum(["finished", "error", "permission"]), }), }) .optional(), }), })` | `packages/server/src/server/session.ts:1375` `forwardAgentStream` (emit)<br>`packages/server/src/server/websocket-server.ts:2929` `broadcastAgentAttention` (emit) |
| `clear_agent_attention_response`<br>`ClearAgentAttentionResponseMessageSchema`<br>`packages/protocol/src/messages.ts:5018` | S→C | clear_agent_attention; Presence, attention and notifications | `z.object({ type: z.literal("clear_agent_attention_response"), payload: z.object({ requestId: z.string(), agentId: z.string().or(z.array(z.string())), agents: z.array(AgentSnapshotPayloadSchema), }), })` | `packages/server/src/server/session.ts:5026` `handleClearAgentAttention` (emit) |

## Protocol: Terminals

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `list_terminals_request`<br>`ListTerminalsRequestSchema`<br>`packages/protocol/src/messages.ts:3021` | C→S | list_terminals_response; Terminals | `z.object({ type: z.literal("list_terminals_request"), cwd: z.string().optional(), workspaceId: z.string().optional(), requestId: z.string(), })` | `packages/server/src/terminal/terminal-session-controller.ts:209` `dispatch` (dispatch) |
| `subscribe_terminals_request`<br>`SubscribeTerminalsRequestSchema`<br>`packages/protocol/src/messages.ts:3028` | C→S | terminals_changed (initial and updates); Terminals | `z.object({ type: z.literal("subscribe_terminals_request"), requestId: z.string().optional(), cwd: z.string(), workspaceId: z.string().optional(), })` | `packages/server/src/terminal/terminal-session-controller.ts:203` `dispatch` (dispatch) |
| `unsubscribe_terminals_request`<br>`UnsubscribeTerminalsRequestSchema`<br>`packages/protocol/src/messages.ts:3035` | C→S | no separate acknowledgement; Terminals | `z.object({ type: z.literal("unsubscribe_terminals_request"), cwd: z.string(), workspaceId: z.string().optional(), })` | `packages/server/src/terminal/terminal-session-controller.ts:205` `dispatch` (dispatch) |
| `create_terminal_request`<br>`CreateTerminalRequestSchema`<br>`packages/protocol/src/messages.ts:3041` | C→S | create_terminal_response; Terminals | `z.object({ type: z.literal("create_terminal_request"), cwd: z.string(), workspaceId: z.string().optional(), name: z.string().optional(), agentId: z.string().optional(), command: z.string().optional(), args: z.array(z.string()).optional(), size: z .object({ rows: z.number().int().positive(), cols: z.number().int().positive(), }) .optional(), requestId: z.string(), })` | `packages/server/src/terminal/terminal-session-controller.ts:211` `dispatch` (dispatch) |
| `terminal.rename.request`<br>`RenameTerminalRequestSchema`<br>`packages/protocol/src/messages.ts:3062` | C→S | terminal.rename.response; Terminals | `z.object({ type: z.literal("terminal.rename.request"), terminalId: z.string(), title: z.string(), requestId: z.string(), })` | `packages/server/src/terminal/terminal-session-controller.ts:225` `dispatch` (dispatch) |
| `subscribe_terminal_request`<br>`SubscribeTerminalRequestSchema`<br>`packages/protocol/src/messages.ts:3096` | C→S | subscribe_terminal_response; Terminals | `z.object({ type: z.literal("subscribe_terminal_request"), terminalId: z.string(), requestId: z.string(), restore: z .object({ mode: z.enum(["live", "visible-snapshot", "full-snapshot"]), scrollbackLines: z.number().int().nonnegative().optional(), size: z .object({ rows: z.number().int().positive(), cols: z.number().int().positive(), }) .optional(), }) .optional(), })` | `packages/server/src/terminal/terminal-session-controller.ts:213` `dispatch` (dispatch) |
| `unsubscribe_terminal_request`<br>`UnsubscribeTerminalRequestSchema`<br>`packages/protocol/src/messages.ts:3114` | C→S | no separate acknowledgement; Terminals | `z.object({ type: z.literal("unsubscribe_terminal_request"), terminalId: z.string(), })` | `packages/server/src/terminal/terminal-session-controller.ts:215` `dispatch` (dispatch) |
| `terminal_input`<br>`TerminalInputSchema`<br>`packages/protocol/src/messages.ts:3136` | C→S | no JSON acknowledgement; terminal output binary; Terminals | `z.object({ type: z.literal("terminal_input"), terminalId: z.string(), message: TerminalClientMessageSchema, })` | `packages/server/src/terminal/terminal-session-controller.ts:217` `dispatch` (dispatch) |
| `kill_terminal_request`<br>`KillTerminalRequestSchema`<br>`packages/protocol/src/messages.ts:3142` | C→S | kill_terminal_response; Terminals | `z.object({ type: z.literal("kill_terminal_request"), terminalId: z.string(), requestId: z.string(), })` | `packages/server/src/terminal/terminal-session-controller.ts:221` `dispatch` (dispatch) |
| `capture_terminal_request`<br>`CaptureTerminalRequestSchema`<br>`packages/protocol/src/messages.ts:3148` | C→S | capture_terminal_response; Terminals | `z.object({ type: z.literal("capture_terminal_request"), terminalId: z.string(), start: z.number().int().optional(), end: z.number().int().optional(), stripAnsi: z.boolean().default(true), requestId: z.string(), })` | `packages/server/src/terminal/terminal-session-controller.ts:223` `dispatch` (dispatch) |
| `list_terminals_response`<br>`ListTerminalsResponseSchema`<br>`packages/protocol/src/messages.ts:6602` | S→C | list_terminals_request; Terminals | `z.object({ type: z.literal("list_terminals_response"), payload: z.object({ cwd: z.string().optional(), terminals: z.array(TerminalInfoSchema.partial({ cwd: true })), requestId: z.string(), }), })` | `packages/server/src/terminal/terminal-session-controller.ts:414` `handleListTerminalsRequest` (emit) |
| `terminals_changed`<br>`TerminalsChangedSchema`<br>`packages/protocol/src/messages.ts:6611` | S→C | subscribe_terminals_request; Terminals | `z.object({ type: z.literal("terminals_changed"), payload: z.object({ subscriptionId: z.string().optional(), requestId: z.string().optional(), workspaceId: z.string().optional(), cwd: z.string(), terminals: z.array(TerminalInfoSchema.omit({ cwd: true })), }), })` | `packages/server/src/terminal/terminal-session-controller.ts:392` (emit) |
| `create_terminal_response`<br>`CreateTerminalResponseSchema`<br>`packages/protocol/src/messages.ts:6622` | S→C | create_terminal_request; Terminals | `z.object({ type: z.literal("create_terminal_response"), payload: z.object({ terminal: TerminalInfoSchema.nullable(), error: z.string().nullable(), requestId: z.string(), }), })` | `packages/server/src/terminal/terminal-session-controller.ts:551` `handleCreateTerminalRequest` (emit) |
| `terminal.rename.response`<br>`RenameTerminalResponseSchema`<br>`packages/protocol/src/messages.ts:6631` | S→C | terminal.rename.request; Terminals | `z.object({ type: z.literal("terminal.rename.response"), payload: z.object({ requestId: z.string(), success: z.boolean(), error: z.string().nullable(), }), })` | `packages/server/src/terminal/terminal-session-controller.ts:661` (emit) |
| `subscribe_terminal_response`<br>`SubscribeTerminalResponseSchema`<br>`packages/protocol/src/messages.ts:6640` | S→C | subscribe_terminal_request; Terminals | `z.object({ type: z.literal("subscribe_terminal_response"), payload: z.union([ z.object({ terminalId: z.string(), slot: z.number().int().min(0).max(255), subscriptionId: z.string().optional(), error: z.null(), requestId: z.string(), }), z.object({ terminalId: z.string(), error: z.string(), requestId: z.string(), }), ]), })` | `packages/server/src/terminal/terminal-session-controller.ts:690` `handleSubscribeTerminalRequest` (emit) |
| `kill_terminal_response`<br>`KillTerminalResponseSchema`<br>`packages/protocol/src/messages.ts:6658` | S→C | kill_terminal_request; Terminals | `z.object({ type: z.literal("kill_terminal_response"), payload: z.object({ terminalId: z.string(), success: z.boolean(), requestId: z.string(), }), })` | `packages/server/src/terminal/terminal-session-controller.ts:809` `handleKillTerminalRequest` (emit) |
| `capture_terminal_response`<br>`CaptureTerminalResponseSchema`<br>`packages/protocol/src/messages.ts:6667` | S→C | capture_terminal_request; Terminals | `z.object({ type: z.literal("capture_terminal_response"), payload: z.object({ terminalId: z.string(), lines: z.array(z.string()), totalLines: z.number().int().nonnegative(), requestId: z.string(), }), })` | `packages/server/src/terminal/terminal-session-controller.ts:821` `handleCaptureTerminalRequest` (emit) |
| `terminal_stream_exit`<br>`TerminalStreamExitSchema`<br>`packages/protocol/src/messages.ts:6677` | S→C | push / shared status or error channel; Terminals | `z.object({ type: z.literal("terminal_stream_exit"), payload: z.object({ subscriptionId: z.string().optional(), terminalId: z.string(), error: z.string().optional(), }), })` | `packages/server/src/terminal/terminal-session-controller.ts:970` (emit)<br>`packages/server/src/terminal/terminal-session-controller.ts:1064` `sendSnapshot` (emit) |
| `terminal_attention_required`<br>`TerminalAttentionRequiredSchema`<br>`packages/protocol/src/messages.ts:6687` | S→C | push / shared status or error channel; Terminals | `z.object({ type: z.literal("terminal_attention_required"), payload: z.object({ subscriptionId: z.string().optional(), serverId: z.string().optional(), terminalId: z.string(), cwd: z.string(), workspaceId: z.string().optional(), reason: z.enum(["finished", "needs_input"]), title: z.string(), body: z.string(), shouldNotify: z.boolean(), }), })` | `packages/server/src/server/websocket-server.ts:3029` `broadcastTerminalAttention` (emit) |

## Protocol: Workspace scripts

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `start_workspace_script_request`<br>`StartWorkspaceScriptRequestSchema`<br>`packages/protocol/src/messages.ts:3069` | C→S | start_workspace_script_response; Workspace scripts | `z.object({ type: z.literal("start_workspace_script_request"), workspaceId: z.string(), scriptName: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3206` `dispatchTerminalMessage` (dispatch) |
| `workspace.script.list.request`<br>`WorkspaceScriptListRequestSchema`<br>`packages/protocol/src/messages.ts:3076` | C→S | workspace.script.list.response; Workspace scripts | `z.object({ type: z.literal("workspace.script.list.request"), workspaceId: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3208` `dispatchTerminalMessage` (dispatch) |
| `workspace.script.start.request`<br>`WorkspaceScriptStartRequestSchema`<br>`packages/protocol/src/messages.ts:3082` | C→S | workspace.script.start.response; Workspace scripts | `z.object({ type: z.literal("workspace.script.start.request"), workspaceId: z.string(), scriptName: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3210` `dispatchTerminalMessage` (dispatch) |
| `workspace.script.stop.request`<br>`WorkspaceScriptStopRequestSchema`<br>`packages/protocol/src/messages.ts:3089` | C→S | workspace.script.stop.response; Workspace scripts | `z.object({ type: z.literal("workspace.script.stop.request"), workspaceId: z.string(), scriptName: z.string(), requestId: z.string(), })` | `packages/server/src/server/session.ts:3212` `dispatchTerminalMessage` (dispatch) |
| `script_status_update`<br>`ScriptStatusUpdateMessageSchema`<br>`packages/protocol/src/messages.ts:4533` | S→C | push / shared status or error channel; Workspace scripts | `z.object({ type: z.literal("script_status_update"), payload: z.object({ subscriptionId: z.string().optional(), workspaceId: z.string(), scripts: z.array(WorkspaceScriptPayloadSchema), }), })` | `packages/server/src/server/session/workspace-scripts/workspace-scripts-service.ts:139` `emitStatusUpdate` (emit)<br>`packages/server/src/server/script-status-projection.ts:267` `buildScriptStatusUpdateMessage` (emit) |
| `start_workspace_script_response`<br>`StartWorkspaceScriptResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4693` | S→C | start_workspace_script_request; Workspace scripts | `z.object({ type: z.literal("start_workspace_script_response"), payload: z.object({ requestId: z.string(), workspaceId: z.string(), scriptName: z.string(), terminalId: z.string().nullable(), error: z.string().nullable(), }), })` | `packages/server/src/server/session/workspace-scripts/workspace-scripts-service.ts:249` `start` (emit) |
| `workspace.script.list.response`<br>`WorkspaceScriptListResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4713` | S→C | workspace.script.list.request; Workspace scripts | `z.object({ type: z.literal("workspace.script.list.response"), payload: WorkspaceScriptOperationPayloadSchema, })` | `packages/server/src/server/session.ts:7358` `handleWorkspaceScriptListRequest` (emit) |
| `workspace.script.start.response`<br>`WorkspaceScriptStartResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4718` | S→C | workspace.script.start.request; Workspace scripts | `z.object({ type: z.literal("workspace.script.start.response"), payload: WorkspaceScriptOperationPayloadSchema, })` | `packages/server/src/server/session.ts:7385` `handleWorkspaceScriptStartRequest` (emit) |
| `workspace.script.stop.response`<br>`WorkspaceScriptStopResponseMessageSchema`<br>`packages/protocol/src/messages.ts:4723` | S→C | workspace.script.stop.request; Workspace scripts | `z.object({ type: z.literal("workspace.script.stop.response"), payload: WorkspaceScriptOperationPayloadSchema, })` | `packages/server/src/server/session.ts:7414` `handleWorkspaceScriptStopRequest` (emit) |

## Protocol: Chat orchestration

**Schema-only feature in this checkout:** requests/responses belong to the active protocol unions and permission tables, but no server dispatcher or response producer was found in packages/server/src. No public client method or CLI command was found. Do not interpret the declared contract as an implemented service.

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `chat/create`<br>`ChatCreateRequestSchema`<br>`packages/protocol/src/chat/rpc-schemas.ts:6` | C→S | chat/create/response; Chat orchestration | `z.object({ type: z.literal("chat/create"), requestId: z.string(), name: z.string(), purpose: z.string().optional(), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `chat/list`<br>`ChatListRequestSchema`<br>`packages/protocol/src/chat/rpc-schemas.ts:13` | C→S | chat/list/response; Chat orchestration | `z.object({ type: z.literal("chat/list"), requestId: z.string(), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `chat/inspect`<br>`ChatInspectRequestSchema`<br>`packages/protocol/src/chat/rpc-schemas.ts:18` | C→S | chat/inspect/response; Chat orchestration | `z.object({ type: z.literal("chat/inspect"), requestId: z.string(), room: z.string(), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `chat/delete`<br>`ChatDeleteRequestSchema`<br>`packages/protocol/src/chat/rpc-schemas.ts:24` | C→S | chat/delete/response; Chat orchestration | `z.object({ type: z.literal("chat/delete"), requestId: z.string(), room: z.string(), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `chat/post`<br>`ChatPostRequestSchema`<br>`packages/protocol/src/chat/rpc-schemas.ts:30` | C→S | chat/post/response; Chat orchestration | `z.object({ type: z.literal("chat/post"), requestId: z.string(), room: z.string(), body: z.string(), authorAgentId: z.string().optional(), replyToMessageId: z.string().optional(), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `chat/read`<br>`ChatReadRequestSchema`<br>`packages/protocol/src/chat/rpc-schemas.ts:39` | C→S | chat/read/response; Chat orchestration | `z.object({ type: z.literal("chat/read"), requestId: z.string(), room: z.string(), limit: z.number().int().nonnegative().optional(), since: z.string().optional(), authorAgentId: z.string().optional(), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `chat/wait`<br>`ChatWaitRequestSchema`<br>`packages/protocol/src/chat/rpc-schemas.ts:48` | C→S | chat/wait/response; Chat orchestration | `z.object({ type: z.literal("chat/wait"), requestId: z.string(), room: z.string(), afterMessageId: z.string().optional(), timeoutMs: z.number().int().nonnegative().optional(), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `chat/create/response`<br>`ChatCreateResponseSchema`<br>`packages/protocol/src/chat/rpc-schemas.ts:56` | S→C | chat/create; Chat orchestration | `z.object({ type: z.literal("chat/create/response"), payload: z.object({ requestId: z.string(), room: ChatRoomDetailSchema.nullable(), error: z.string().nullable(), }), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `chat/list/response`<br>`ChatListResponseSchema`<br>`packages/protocol/src/chat/rpc-schemas.ts:65` | S→C | chat/list; Chat orchestration | `z.object({ type: z.literal("chat/list/response"), payload: z.object({ requestId: z.string(), rooms: z.array(ChatRoomDetailSchema), error: z.string().nullable(), }), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `chat/inspect/response`<br>`ChatInspectResponseSchema`<br>`packages/protocol/src/chat/rpc-schemas.ts:74` | S→C | chat/inspect; Chat orchestration | `z.object({ type: z.literal("chat/inspect/response"), payload: z.object({ requestId: z.string(), room: ChatRoomDetailSchema.nullable(), error: z.string().nullable(), }), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `chat/delete/response`<br>`ChatDeleteResponseSchema`<br>`packages/protocol/src/chat/rpc-schemas.ts:83` | S→C | chat/delete; Chat orchestration | `z.object({ type: z.literal("chat/delete/response"), payload: z.object({ requestId: z.string(), room: ChatRoomDetailSchema.nullable(), error: z.string().nullable(), }), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `chat/post/response`<br>`ChatPostResponseSchema`<br>`packages/protocol/src/chat/rpc-schemas.ts:92` | S→C | chat/post; Chat orchestration | `z.object({ type: z.literal("chat/post/response"), payload: z.object({ requestId: z.string(), message: ChatMessageSchema.nullable(), error: z.string().nullable(), }), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `chat/read/response`<br>`ChatReadResponseSchema`<br>`packages/protocol/src/chat/rpc-schemas.ts:101` | S→C | chat/read; Chat orchestration | `z.object({ type: z.literal("chat/read/response"), payload: z.object({ requestId: z.string(), messages: z.array(ChatMessageSchema), error: z.string().nullable(), }), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `chat/wait/response`<br>`ChatWaitResponseSchema`<br>`packages/protocol/src/chat/rpc-schemas.ts:110` | S→C | chat/wait; Chat orchestration | `z.object({ type: z.literal("chat/wait/response"), payload: z.object({ requestId: z.string(), messages: z.array(ChatMessageSchema), timedOut: z.boolean(), error: z.string().nullable(), }), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |

## Protocol: Schedules and heartbeats

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `schedule/create`<br>`ScheduleCreateRequestSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:27` | C→S | schedule/create/response; Schedules and heartbeats | `z.object({ type: z.literal("schedule/create"), requestId: z.string(), prompt: z.string().min(1), name: z.string().optional(), cadence: ScheduleCadenceSchema, target: ScheduleCreateTargetSchema, maxRuns: z.number().int().positive().optional(), expiresAt: z.string().optional(), runOnCreate: z.boolean().optional(), })` | `packages/server/src/server/session.ts:3221` `dispatchScheduleMessage` (dispatch) |
| `schedule/list`<br>`ScheduleListRequestSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:39` | C→S | schedule/list/response; Schedules and heartbeats | `z.object({ type: z.literal("schedule/list"), requestId: z.string(), })` | `packages/server/src/server/session.ts:3223` `dispatchScheduleMessage` (dispatch) |
| `schedule/inspect`<br>`ScheduleInspectRequestSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:44` | C→S | schedule/inspect/response; Schedules and heartbeats | `z.object({ type: z.literal("schedule/inspect"), requestId: z.string(), scheduleId: z.string(), })` | `packages/server/src/server/session.ts:3225` `dispatchScheduleMessage` (dispatch) |
| `schedule/logs`<br>`ScheduleLogsRequestSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:50` | C→S | schedule/logs/response; Schedules and heartbeats | `z.object({ type: z.literal("schedule/logs"), requestId: z.string(), scheduleId: z.string(), })` | `packages/server/src/server/session.ts:3227` `dispatchScheduleMessage` (dispatch) |
| `schedule/pause`<br>`SchedulePauseRequestSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:56` | C→S | schedule/pause/response; Schedules and heartbeats | `z.object({ type: z.literal("schedule/pause"), requestId: z.string(), scheduleId: z.string(), })` | `packages/server/src/server/session.ts:3229` `dispatchScheduleMessage` (dispatch) |
| `schedule/resume`<br>`ScheduleResumeRequestSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:62` | C→S | schedule/resume/response; Schedules and heartbeats | `z.object({ type: z.literal("schedule/resume"), requestId: z.string(), scheduleId: z.string(), })` | `packages/server/src/server/session.ts:3231` `dispatchScheduleMessage` (dispatch) |
| `schedule/delete`<br>`ScheduleDeleteRequestSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:68` | C→S | schedule/delete/response; Schedules and heartbeats | `z.object({ type: z.literal("schedule/delete"), requestId: z.string(), scheduleId: z.string(), })` | `packages/server/src/server/session.ts:3233` `dispatchScheduleMessage` (dispatch) |
| `schedule/run-once`<br>`ScheduleRunOnceRequestSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:74` | C→S | schedule/run-once/response; Schedules and heartbeats | `z.object({ type: z.literal("schedule/run-once"), requestId: z.string(), scheduleId: z.string(), })` | `packages/server/src/server/session.ts:3235` `dispatchScheduleMessage` (dispatch) |
| `schedule/update`<br>`ScheduleUpdateRequestSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:90` | C→S | schedule/update/response; Schedules and heartbeats | `z.object({ type: z.literal("schedule/update"), requestId: z.string(), scheduleId: z.string(), name: z.string().nullable().optional(), prompt: z.string().min(1).optional(), cadence: ScheduleCadenceSchema.optional(), newAgentConfig: ScheduleUpdateNewAgentConfigSchema.optional(), maxRuns: z.number().int().positive().nullable().optional(), expiresAt: z.string().nullable().optional(), })` | `packages/server/src/server/session.ts:3237` `dispatchScheduleMessage` (dispatch) |
| `schedule/create/response`<br>`ScheduleCreateResponseSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:102` | S→C | schedule/create; Schedules and heartbeats | `z.object({ type: z.literal("schedule/create/response"), payload: z.object({ requestId: z.string(), schedule: ScheduleSummarySchema.nullable(), error: z.string().nullable(), }), })` | `packages/server/src/server/session/schedule/schedule-session.ts:85` `handleScheduleCreateRequest` (emit) |
| `schedule/list/response`<br>`ScheduleListResponseSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:111` | S→C | schedule/list; Schedules and heartbeats | `z.object({ type: z.literal("schedule/list/response"), payload: z.object({ requestId: z.string(), schedules: z.array(ScheduleSummarySchema), error: z.string().nullable(), }), })` | `packages/server/src/server/session/schedule/schedule-session.ts:103` `handleScheduleListRequest` (emit) |
| `schedule/inspect/response`<br>`ScheduleInspectResponseSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:120` | S→C | schedule/inspect; Schedules and heartbeats | `z.object({ type: z.literal("schedule/inspect/response"), payload: z.object({ requestId: z.string(), schedule: StoredScheduleSchema.nullable(), error: z.string().nullable(), }), })` | `packages/server/src/server/session/schedule/schedule-session.ts:121` `handleScheduleInspectRequest` (emit) |
| `schedule/logs/response`<br>`ScheduleLogsResponseSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:129` | S→C | schedule/logs; Schedules and heartbeats | `z.object({ type: z.literal("schedule/logs/response"), payload: z.object({ requestId: z.string(), runs: z.array(ScheduleRunSchema), error: z.string().nullable(), }), })` | `packages/server/src/server/session/schedule/schedule-session.ts:139` `handleScheduleLogsRequest` (emit) |
| `schedule/pause/response`<br>`SchedulePauseResponseSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:138` | S→C | schedule/pause; Schedules and heartbeats | `z.object({ type: z.literal("schedule/pause/response"), payload: z.object({ requestId: z.string(), schedule: ScheduleSummarySchema.nullable(), error: z.string().nullable(), }), })` | `packages/server/src/server/session/schedule/schedule-session.ts:157` `handleSchedulePauseRequest` (emit) |
| `schedule/resume/response`<br>`ScheduleResumeResponseSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:147` | S→C | schedule/resume; Schedules and heartbeats | `z.object({ type: z.literal("schedule/resume/response"), payload: z.object({ requestId: z.string(), schedule: ScheduleSummarySchema.nullable(), error: z.string().nullable(), }), })` | `packages/server/src/server/session/schedule/schedule-session.ts:175` `handleScheduleResumeRequest` (emit) |
| `schedule/delete/response`<br>`ScheduleDeleteResponseSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:156` | S→C | schedule/delete; Schedules and heartbeats | `z.object({ type: z.literal("schedule/delete/response"), payload: z.object({ requestId: z.string(), scheduleId: z.string(), error: z.string().nullable(), }), })` | `packages/server/src/server/session/schedule/schedule-session.ts:193` `handleScheduleDeleteRequest` (emit) |
| `schedule/run-once/response`<br>`ScheduleRunOnceResponseSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:165` | S→C | schedule/run-once; Schedules and heartbeats | `z.object({ type: z.literal("schedule/run-once/response"), payload: z.object({ requestId: z.string(), schedule: StoredScheduleSchema.nullable(), error: z.string().nullable(), }), })` | `packages/server/src/server/session/schedule/schedule-session.ts:211` `handleScheduleRunOnceRequest` (emit) |
| `schedule/update/response`<br>`ScheduleUpdateResponseSchema`<br>`packages/protocol/src/schedule/rpc-schemas.ts:174` | S→C | schedule/update; Schedules and heartbeats | `z.object({ type: z.literal("schedule/update/response"), payload: z.object({ requestId: z.string(), schedule: StoredScheduleSchema.nullable(), error: z.string().nullable(), }), })` | `packages/server/src/server/session/schedule/schedule-session.ts:237` `handleScheduleUpdateRequest` (emit) |

## Protocol: Loops

**Schema-only feature in this checkout:** active protocol/permission declarations exist, but no server dispatch/controller/response producer was found. The typed SDK and CLI also have no dedicated loop method/command.

| Message / schema source | Direction | Pair / use | Fields and typed validation | Server handler / emission site |
|---|---|---|---|---|
| `loop/run`<br>`LoopRunRequestSchema`<br>`packages/protocol/src/loop/rpc-schemas.ts:89` | C→S | loop/run/response; Loops | `z.object({ type: z.literal("loop/run"), requestId: z.string(), prompt: z.string().trim().min(1), cwd: z.string(), provider: AgentProviderSchema.optional(), model: z.string().trim().min(1).optional(), modeId: z.string().trim().min(1).optional(), workerProvider: AgentProviderSchema.optional(), workerModel: z.string().trim().min(1).optional(), verifierProvider: AgentProviderSchema.optional(), verifierModel: z.string().trim().min(1).optional(), verifierModeId: z.string().trim().min(1).optional(), verifyPrompt: z.string().trim().min(1).optional(), verifyChecks: z.array(z.string().trim().min(1)).optional(), archive: z.boolean().optional(), name: z.string().trim().min(1).optional(), sleepMs: z.number().int().nonnegative().optional(), maxIterations: z.number().int().positive().optional(), maxTimeMs: z.number().int().positive().optional(), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `loop/list`<br>`LoopListRequestSchema`<br>`packages/protocol/src/loop/rpc-schemas.ts:111` | C→S | loop/list/response; Loops | `z.object({ type: z.literal("loop/list"), requestId: z.string(), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `loop/inspect`<br>`LoopInspectRequestSchema`<br>`packages/protocol/src/loop/rpc-schemas.ts:116` | C→S | loop/inspect/response; Loops | `z.object({ type: z.literal("loop/inspect"), requestId: z.string(), id: z.string().trim().min(1), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `loop/logs`<br>`LoopLogsRequestSchema`<br>`packages/protocol/src/loop/rpc-schemas.ts:122` | C→S | loop/logs/response; Loops | `z.object({ type: z.literal("loop/logs"), requestId: z.string(), id: z.string().trim().min(1), afterSeq: z.number().int().nonnegative().optional(), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `loop/stop`<br>`LoopStopRequestSchema`<br>`packages/protocol/src/loop/rpc-schemas.ts:129` | C→S | loop/stop/response; Loops | `z.object({ type: z.literal("loop/stop"), requestId: z.string(), id: z.string().trim().min(1), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `loop/run/response`<br>`LoopRunResponseSchema`<br>`packages/protocol/src/loop/rpc-schemas.ts:135` | S→C | loop/run; Loops | `z.object({ type: z.literal("loop/run/response"), payload: z.object({ requestId: z.string(), loop: LoopRecordSchema.nullable(), error: z.string().nullable(), }), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `loop/list/response`<br>`LoopListResponseSchema`<br>`packages/protocol/src/loop/rpc-schemas.ts:144` | S→C | loop/list; Loops | `z.object({ type: z.literal("loop/list/response"), payload: z.object({ requestId: z.string(), loops: z.array(LoopListItemSchema), error: z.string().nullable(), }), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `loop/inspect/response`<br>`LoopInspectResponseSchema`<br>`packages/protocol/src/loop/rpc-schemas.ts:153` | S→C | loop/inspect; Loops | `z.object({ type: z.literal("loop/inspect/response"), payload: z.object({ requestId: z.string(), loop: LoopRecordSchema.nullable(), error: z.string().nullable(), }), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `loop/logs/response`<br>`LoopLogsResponseSchema`<br>`packages/protocol/src/loop/rpc-schemas.ts:162` | S→C | loop/logs; Loops | `z.object({ type: z.literal("loop/logs/response"), payload: z.object({ requestId: z.string(), loop: LoopRecordSchema.nullable(), entries: z.array(LoopLogEntrySchema), nextCursor: z.number().int().nonnegative(), error: z.string().nullable(), }), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |
| `loop/stop/response`<br>`LoopStopResponseSchema`<br>`packages/protocol/src/loop/rpc-schemas.ts:173` | S→C | loop/stop; Loops | `z.object({ type: z.literal("loop/stop/response"), payload: z.object({ requestId: z.string(), loop: LoopRecordSchema.nullable(), error: z.string().nullable(), }), })` | not found: searched packages/server/src non-test dispatch/controller sources; only authorization/operation-permissions.ts names it |

## Protocol shared payload types and constraints

This dictionary expands all named schema references reachable from the session message contracts. Source expressions such as .extend/.omit/.pick/.partial and imported constants retain their original semantics. They are payload definitions, **not additional top-level messages**. Non-Schema symbol dependencies are listed afterwards where relevant.

| Schema | Source | Exact typed contract |
|---|---|---|
| `ActiveTurnBehaviorSchema` | `packages/protocol/src/messages.ts:1254` | `z.enum(["interrupt", "steer"])` |
| `ActivityLogPayloadSchema` | `packages/protocol/src/messages.ts:3512` | `z.object({ subscriptionId: z.string().optional(), id: z.string(), timestamp: z.coerce.date(), type: z.enum(["transcript", "assistant", "tool_call", "tool_result", "error", "system"]), content: z.string(), metadata: z.record(z.string(), z.unknown()).optional(), })` |
| `AgentActionResponsePayloadSchema` | `packages/protocol/src/messages.ts:2008` | `z.object({ requestId: z.string(), agentId: z.string(), accepted: z.boolean(), error: z.string().nullable(), notice: AgentProviderNoticeSchema.nullable().optional(), })` |
| `AgentActiveTurnPayloadSchema` | `packages/protocol/src/messages.ts:858` | `z.object({ turnId: z.string(), startedAt: z.string().nullable(), })` |
| `AgentAttachmentSchema` | `packages/protocol/src/messages.ts:1216` | `z.discriminatedUnion("type", [ ForgeChangeRequestAttachmentSchema, ForgeIssueAttachmentSchema, GitHubPrAttachmentSchema, GitHubIssueAttachmentSchema, TextAttachmentSchema, ReviewAttachmentSchema, UploadedFileAttachmentSchema, ])` |
| `AgentAttachmentsSchema` | `packages/protocol/src/messages.ts:1240` | `z.unknown().transform(normalizeAgentAttachments).optional()` |
| `AgentCapabilityFlagsSchema` | `packages/protocol/src/messages.ts:416` | `z .object({ supportsStreaming: z.boolean(), supportsSessionPersistence: z.boolean(), supportsSessionListing: z.boolean().optional(), supportsDynamicModes: z.boolean(), supportsMcpServers: z.boolean(), supportsReasoningStream: z.boolean(), supportsToolInvocations: z.boolean(), supportsRewindConversation: z.boolean().optional().default(false), supportsRewindFiles: z.boolean().optional().default(false), supportsRewindBoth: z.boolean().optional().default(false), }) .catchall(z.boolean())` |
| `AgentConfigApplySchema` | `packages/protocol/src/messages.ts:2063` | `z.object({ modelId: z.string().nullable().optional(), modeId: z.string().optional(), thinkingOptionId: z.string().nullable().optional(), featureValues: z.record(z.string(), z.unknown()).optional(), })` |
| `AgentDirectoryFilterSchema` | `packages/protocol/src/messages.ts:958` | `z.object({ labels: z.record(z.string(), z.string()).optional(), projectKeys: z.array(z.string()).optional(), statuses: z.array(AgentStatusSchema).optional(), includeArchived: z.boolean().optional(), requiresAttention: z.boolean().optional(), thinkingOptionId: z.string().nullable().optional(), includeInternal: z.boolean().optional(), includeBackground: z.boolean().optional(), })` |
| `AgentDirectoryPageInfoSchema` | `packages/protocol/src/messages.ts:4304` | `z.object({ nextCursor: z.string().nullable(), prevCursor: z.string().nullable(), hasMore: z.boolean(), })` |
| `AgentDirectoryResponseEntrySchema` | `packages/protocol/src/messages.ts:4292` | `z.object({ agent: AgentSnapshotPayloadSchema, project: ProjectPlacementPayloadSchema, searchScore: z.number().optional(), searchMatches: z.array(AgentSearchMatchSchema).optional(), syncSeq: z.number().int().positive().optional(), })` |
| `AgentFeatureSchema` | `packages/protocol/src/messages.ts:359` | `z.discriminatedUnion("type", [ AgentFeatureToggleSchema, AgentFeatureSelectSchema, ])` |
| `AgentFeatureSelectSchema` | `packages/protocol/src/messages.ts:347` | `z.object({ type: z.literal("select"), id: z.string(), label: z.string(), description: z.string().optional(), tooltip: z.string().optional(), icon: z.string().optional(), desktopTrigger: z.enum(["icon", "label"]).optional(), value: z.string().nullable(), options: z.array(AgentSelectOptionSchema), })` |
| `AgentFeatureToggleSchema` | `packages/protocol/src/messages.ts:337` | `z.object({ type: z.literal("toggle"), id: z.string(), label: z.string(), description: z.string().optional(), tooltip: z.string().optional(), icon: z.string().optional(), value: z.boolean(), })` |
| `AgentMessageSchema` | `packages/protocol/src/agent-message.ts:4` | `z.object({ event: z.enum(["message", "finished", "errored", "permission-required", "closed"]), sender: z.object({ id: z.string().min(1), serverId: z.string().min(1).optional(), title: z.string().min(1).optional(), }), text: z.string(), })` |
| `AgentModeSchema` | `packages/protocol/src/messages.ts:308` | `z.object({ id: z.string(), label: z.string(), description: z.string().optional(), icon: z.string().optional(), colorTier: z.string().optional(), })` |
| `AgentModelDefinitionSchema` | `packages/protocol/src/messages.ts:364` | `z.object({ provider: AgentProviderSchema, id: z.string(), aliases: z.array(z.string()).optional(), isSelectable: z.boolean().optional(), label: z.string(), description: z.string().optional(), isDefault: z.boolean().optional(), metadata: z.record(z.string(), z.unknown()).optional(), contextWindowMaxTokens: z.number().optional(), thinkingOptions: z.array(AgentSelectOptionSchema).optional(), defaultThinkingOptionId: z.string().optional(), }) satisfies z.ZodType<AgentModelDefinition>` |
| `AgentPermissionActionSchema` | `packages/protocol/src/messages.ts:502` | `z.object({ id: z.string(), label: z.string(), behavior: z.enum(["allow", "deny"]), variant: z.enum(["primary", "secondary", "danger"]).optional(), intent: z.enum(["implement", "implement_resume", "dismiss"]).optional(), })` |
| `AgentPermissionRequestPayloadSchema` | `packages/protocol/src/messages.ts:526` | `z.object({ id: z.string(), provider: AgentProviderSchema, name: z.string(), kind: z.enum(["tool", "plan", "question", "mode", "other"]), title: z.string().optional(), description: z.string().optional(), input: z.record(z.string(), z.unknown()).optional(), detail: z.lazy(() => ToolCallDetailPayloadSchema).optional(), suggestions: z.array(AgentPermissionUpdateSchema).optional(), actions: z.array(AgentPermissionActionSchema).optional(), metadata: z.record(z.string(), z.unknown()).optional(), })` |
| `AgentPermissionResponseSchema` | `packages/protocol/src/messages.ts:510` | `z.discriminatedUnion("behavior", [ z.object({ behavior: z.literal("allow"), selectedActionId: z.string().optional(), updatedInput: z.record(z.string(), z.unknown()).optional(), updatedPermissions: z.array(AgentPermissionUpdateSchema).optional(), }), z.object({ behavior: z.literal("deny"), selectedActionId: z.string().optional(), message: z.string().optional(), interrupt: z.boolean().optional(), }), ])` |
| `AgentPermissionUpdateSchema` | `packages/protocol/src/messages.ts:501` | `z.record(z.string(), z.unknown())` |
| `AgentPersistenceHandleSchema` | `packages/protocol/src/messages.ts:840` | `z .object({ provider: AgentProviderSchema, sessionId: z.string(), nativeHandle: z.string().optional(), metadata: z.record(z.string(), z.unknown()).optional(), }) .nullable()` |
| `AgentProfileSchema` | `packages/protocol/src/agent-profile.ts:12` | `z .object({ id: z.string(), name: z.string(), icon: z.string().optional(), color: z.string().optional(), provider: z.string(), model: z.string().optional(), modeId: z.string().optional(), thinkingOptionId: z.string().optional(), featureValues: z.record(z.string(), z.unknown()).optional(), notes: z.string().optional(), }) .passthrough()` |
| `AgentProviderNoticeSchema` | `packages/protocol/src/messages.ts:331` | `z.discriminatedUnion("type", [ z.object({ type: z.literal("info"), message: z.string() }), z.object({ type: z.literal("warning"), message: z.string() }), z.object({ type: z.literal("error"), message: z.string() }), ])` |
| `AgentProviderSchema` | `packages/protocol/src/provider-manifest.ts:295` | `z.string()` |
| `AgentRewindModeSchema` | `packages/protocol/src/messages.ts:2101` | `z.enum(["conversation", "files", "both"])` |
| `AgentRuntimeInfoSchema` | `packages/protocol/src/messages.ts:849` | `z.object({ provider: AgentProviderSchema, sessionId: z.string().nullable(), model: z.string().nullable().optional(), thinkingOptionId: z.string().nullable().optional(), modeId: z.string().nullable().optional(), extra: z.record(z.string(), z.unknown()).optional(), })` |
| `AgentSearchMatchFieldSchema` | `packages/protocol/src/messages.ts:4278` | `z.enum(["workspace", "title", "branch", "project"])` |
| `AgentSearchMatchSchema` | `packages/protocol/src/messages.ts:4280` | `z.object({ field: AgentSearchMatchFieldSchema, ranges: z.array(z.object({ start: z.number().int().nonnegative(), length: z.number().int().positive(), })), })` |
| `AgentSelectOptionSchema` | `packages/protocol/src/messages.ts:323` | `z.object({ id: z.string(), label: z.string(), description: z.string().optional(), isDefault: z.boolean().optional(), metadata: z.record(z.string(), z.unknown()).optional(), })` |
| `AgentSessionConfigSchema` | `packages/protocol/src/messages.ts:487` | `z.object({ provider: AgentProviderSchema, cwd: z.string(), modeId: z.string().optional(), model: z.string().optional(), thinkingOptionId: z.string().optional(), featureValues: z.record(z.string(), z.unknown()).optional(), title: z.string().trim().min(1).max(MAX_EXPLICIT_AGENT_TITLE_CHARS).optional().nullable(), providerOptions: ProviderOptionsSchema.optional(), toolPolicy: ToolPolicySchema.optional(), systemPrompt: z.string().optional(), mcpServers: z.record(z.string(), McpServerConfigSchema).optional(), })` |
| `AgentSkillOperationSchema` | `packages/protocol/src/messages.ts:1631` | `z.discriminatedUnion("kind", [ z.object({ kind: z.literal("add"), name: z.string() }).strict(), z.object({ kind: z.literal("update"), name: z.string() }).strict(), z.object({ kind: z.literal("delete"), name: z.string() }).strict(), ])` |
| `AgentSkillSelectionSchema` | `packages/protocol/src/agent-profile.ts:32` | `z.discriminatedUnion("mode", [ z.object({ mode: z.literal("all") }).strict(), z.object({ mode: z.literal("custom"), skills: z.array(z.string()) }).strict(), ])` |
| `AgentSkillsConfirmationSchema` | `packages/protocol/src/messages.ts:1647` | `z.object({ removals: z.array(z.string()) }).strict()` |
| `AgentSkillsSaveResultSchema` | `packages/protocol/src/messages.ts:1650` | `AgentSkillsStatusSchema.extend({ confirmationRequired: AgentSkillsConfirmationSchema.nullable(), })` |
| `AgentSkillsStatusSchema` | `packages/protocol/src/messages.ts:1638` | `z.object({ state: z.enum(["not-installed", "up-to-date", "drift"]), ops: z.array(AgentSkillOperationSchema), available: z.array(z.string()), installed: z.array(z.string()), selection: AgentSkillSelectionSchema, })` |
| `AgentSlashCommandSchema` | `packages/protocol/src/messages.ts:6532` | `z.object({ name: z.string(), description: z.string(), argumentHint: z.string(), kind: z.enum(["command", "skill"]).optional().catch("command"), })` |
| `AgentSnapshotPayloadSchema` | `packages/protocol/src/messages.ts:863` | `z.object({ id: z.string(), provider: AgentProviderSchema, cwd: z.string(), workspaceId: z.string().optional(), model: z.string().nullable(), features: z.array(AgentFeatureSchema).optional(), thinkingOptionId: z.string().nullable().optional(), effectiveThinkingOptionId: z.string().nullable().optional(), createdAt: z.string(), updatedAt: z.string(), lastUserMessageAt: z.string().nullable(), status: AgentStatusSchema, activeTurn: AgentActiveTurnPayloadSchema.nullable().optional(), capabilities: AgentCapabilityFlagsSchema, currentModeId: z.string().nullable(), availableModes: z.array(AgentModeSchema), pendingPermissions: z.array(AgentPermissionRequestPayloadSchema), persistence: AgentPersistenceHandleSchema.nullable(), runtimeInfo: AgentRuntimeInfoSchema.optional(), lastUsage: AgentUsageSchema.optional(), lastError: z.string().optional(), title: z.string().nullable(), labels: z.record(z.string(), z.string()).default({}), requiresAttention: z.boolean().optional(), attentionReason: z.enum(["finished", "error", "permission"]).nullable().optional(), attentionTimestamp: z.string().nullable().optional(), archivedAt: z.string().nullable().optional(), providerUnavailable: z.boolean().optional(), internal: z.boolean().optional(), })` |
| `AgentStatusSchema` | `packages/protocol/src/messages.ts:306` | `z.enum(AGENT_LIFECYCLE_STATUSES)` |
| `AgentStreamEventPayloadSchema` | `packages/protocol/src/messages.ts:771` | `z.discriminatedUnion("type", [ z.object({ type: z.literal("thread_started"), sessionId: z.string(), provider: AgentProviderSchema, }), z.object({ type: z.literal("turn_started"), provider: AgentProviderSchema, turnId: z.string().optional(), }), z.object({ type: z.literal("turn_completed"), provider: AgentProviderSchema, turnId: z.string().optional(), usage: AgentUsageSchema.optional(), }), z.object({ type: z.literal("turn_failed"), provider: AgentProviderSchema, turnId: z.string().optional(), error: z.string(), code: z.string().optional(), diagnostic: z.string().optional(), }), z.object({ type: z.literal("turn_canceled"), provider: AgentProviderSchema, turnId: z.string().optional(), reason: z.string(), }), z.object({ type: z.literal("timeline"), provider: AgentProviderSchema, item: AgentTimelineItemPayloadSchema, turnId: z.string().optional(), }), z.object({ type: z.literal("permission_requested"), provider: AgentProviderSchema, request: AgentPermissionRequestPayloadSchema, }), z.object({ type: z.literal("permission_resolved"), provider: AgentProviderSchema, requestId: z.string(), resolution: AgentPermissionResponseSchema, }), z.object({ type: z.literal("attention_required"), provider: AgentProviderSchema, reason: z.enum(["finished", "error", "permission"]), timestamp: z.string(), shouldNotify: z.boolean(), notification: z .object({ title: z.string(), body: z.string(), data: z.object({ serverId: z.string(), workspaceId: z.string().optional(), agentId: z.string(), reason: z.enum(["finished", "error", "permission"]), }), }) .optional(), }), ])` |
| `AgentTimelineCursorSchema` | `packages/protocol/src/messages.ts:1936` | `z.object({ epoch: z.string(), seq: z.number().int().nonnegative(), })` |
| `AgentTimelineEntryPayloadSchema` | `packages/protocol/src/messages.ts:4776` | `z.object({ provider: AgentProviderSchema, item: AgentTimelineItemPayloadSchema, turnId: z.string().optional(), timestamp: z.string(), seqStart: z.number().int().nonnegative(), seqEnd: z.number().int().nonnegative(), sourceSeqRanges: z.array(AgentTimelineSeqRangeSchema), collapsed: z.array(z.enum(["assistant_merge", "reasoning_merge", "tool_lifecycle", "identity"])), })` |
| `AgentTimelineItemPayloadSchema` | `packages/protocol/src/messages.ts:717` | `z.union([ z.object({ type: z.literal("user_message"), text: z.string(), messageId: z.string().optional(), clientMessageId: z.string().optional(), }), z.object({ type: z.literal("assistant_message"), text: z.string(), messageId: z.string().optional(), }), z.object({ type: z.literal("reasoning"), text: z.string(), }), ToolCallTimelineItemPayloadSchema, z.object({ type: z.literal("todo"), items: z.array(z.object({ text: z.string(), completed: z.boolean(), id: z.string().optional(), status: z.enum(["pending", "in_progress", "completed"]).optional(), activeForm: z.string().optional(), })), }), z.object({ type: z.literal("error"), message: z.string(), }), z.object({ type: z.literal("notification"), level: z.enum(["info", "warning", "error"]), message: z.string(), }), z.object({ type: z.literal("compaction"), status: z.enum(["loading", "completed"]), trigger: z.enum(["auto", "manual"]).optional(), preTokens: z.number().optional(), }), z.object({ type: z.literal("plugin"), id: z.string(), pluginId: PluginIdSchema, kind: z.string(), version: z.number(), data: JsonWireValueSchema, }), ])` |
| `AgentTimelineSeqRangeSchema` | `packages/protocol/src/messages.ts:4771` | `z.object({ startSeq: z.number().int().nonnegative(), endSeq: z.number().int().nonnegative(), })` |
| `AgentUsageSchema` | `packages/protocol/src/messages.ts:434` | `z.object({ inputTokens: z.number().optional(), cachedInputTokens: z.number().optional(), outputTokens: z.number().optional(), totalCostUsd: z.number().optional(), contextWindowMaxTokens: z.number().optional(), contextWindowUsedTokens: z.number().optional(), })` |
| `AheadBehindSchema` | `packages/protocol/src/messages.ts:5382` | `z.object({ ahead: z.number(), behind: z.number(), })` |
| `AvailableUsageReportSchema` | `packages/protocol/src/messages.ts:6486` | `z.object({ status: z.literal("available"), planLabel: z.string().optional(), windows: z.array(ProviderUsageWindowSchema), balances: z.array(ProviderUsageBalanceSchema).optional(), details: z.array(ProviderUsageDetailSchema).optional(), })` |
| `BrowserAutomationBackCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:144` | `z.object({ command: z.literal("back"), args: BrowserAutomationTabTargetSchema, })` |
| `BrowserAutomationBackResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:347` | `z.object({ command: z.literal("back"), browserId: BrowserAutomationBrowserIdSchema, })` |
| `BrowserAutomationBrowserIdSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:50` | `z .string({ error: () => BROWSER_AUTOMATION_BROWSER_ID_MESSAGE }) .min(1, BROWSER_AUTOMATION_BROWSER_ID_MESSAGE) .regex(BROWSER_AUTOMATION_BROWSER_ID_PATTERN, BROWSER_AUTOMATION_BROWSER_ID_MESSAGE)` |
| `BrowserAutomationClickCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:92` | `z.object({ command: z.literal("click"), args: BrowserAutomationTabTargetSchema.extend({ ref: BrowserAutomationRefSchema, button: BrowserAutomationMouseButtonSchema.default("left"), doubleClick: z.boolean().default(false), modifiers: z.array(BrowserAutomationInputModifierSchema).default([]), }), })` |
| `BrowserAutomationClickResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:304` | `z.object({ command: z.literal("click"), browserId: BrowserAutomationBrowserIdSchema, ref: BrowserAutomationRefSchema, x: z.number().optional(), y: z.number().optional(), })` |
| `BrowserAutomationCloseTabCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:229` | `z.object({ command: z.literal("close_tab"), args: BrowserAutomationTabTargetSchema, })` |
| `BrowserAutomationCloseTabResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:453` | `z.object({ command: z.literal("close_tab"), browserId: BrowserAutomationBrowserIdSchema, })` |
| `BrowserAutomationCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:234` | `z.discriminatedUnion("command", [ BrowserAutomationListTabsCommandSchema, BrowserAutomationNewTabCommandSchema, BrowserAutomationSnapshotCommandSchema, BrowserAutomationClickCommandSchema, BrowserAutomationFillCommandSchema, BrowserAutomationWaitCommandSchema, BrowserAutomationTypeCommandSchema, BrowserAutomationKeypressCommandSchema, BrowserAutomationNavigateCommandSchema, BrowserAutomationBackCommandSchema, BrowserAutomationForwardCommandSchema, BrowserAutomationReloadCommandSchema, BrowserAutomationScreenshotCommandSchema, BrowserAutomationUploadCommandSchema, BrowserAutomationSelectCommandSchema, BrowserAutomationHoverCommandSchema, BrowserAutomationDragCommandSchema, BrowserAutomationLogsCommandSchema, BrowserAutomationEvaluateCommandSchema, BrowserAutomationScrollCommandSchema, BrowserAutomationResizeCommandSchema, BrowserAutomationCloseTabCommandSchema, ])` |
| `BrowserAutomationConsoleLogEntrySchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:404` | `z.object({ level: z.string(), message: z.string(), source: z.string().optional(), line: z.number().int().optional(), timestamp: z.number(), })` |
| `BrowserAutomationDialogEventSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:489` | `z.object({ type: z.enum(["alert", "confirm", "prompt", "beforeunload"]), message: z.string(), defaultValue: z.string().optional(), action: z.enum(["accepted", "dismissed"]), promptText: z.string().optional(), timestamp: z.number(), })` |
| `BrowserAutomationDragCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:189` | `z.object({ command: z.literal("drag"), args: BrowserAutomationTabTargetSchema.extend({ sourceRef: BrowserAutomationRefSchema, targetRef: BrowserAutomationRefSchema, }), })` |
| `BrowserAutomationDragResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:393` | `z.object({ command: z.literal("drag"), browserId: BrowserAutomationBrowserIdSchema, sourceRef: BrowserAutomationRefSchema, targetRef: BrowserAutomationRefSchema, sourceX: z.number().optional(), sourceY: z.number().optional(), targetX: z.number().optional(), targetY: z.number().optional(), })` |
| `BrowserAutomationErrorCodeSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:3` | `z.enum([ "browser_disabled", "browser_no_host", "browser_tab_not_found", "browser_tab_closed", "browser_timeout", "screenshot_no_frame", "browser_denied", "browser_unsupported", "browser_stale_ref", "browser_unknown_error", ])` |
| `BrowserAutomationErrorSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:483` | `z.object({ code: BrowserAutomationErrorCodeSchema, message: z.string().min(1), retryable: z.boolean().default(false), })` |
| `BrowserAutomationEvaluateCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:204` | `z.object({ command: z.literal("evaluate"), args: BrowserAutomationTabTargetSchema.extend({ function: z.string().min(1), ref: BrowserAutomationRefSchema.optional(), }), })` |
| `BrowserAutomationEvaluateResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:429` | `z.object({ command: z.literal("evaluate"), browserId: BrowserAutomationBrowserIdSchema, resultJson: z.string(), truncated: z.boolean(), })` |
| `BrowserAutomationFillCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:102` | `z.object({ command: z.literal("fill"), args: BrowserAutomationTabTargetSchema.extend({ ref: BrowserAutomationRefSchema, value: z.string(), }), })` |
| `BrowserAutomationFillResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:312` | `z.object({ command: z.literal("fill"), browserId: BrowserAutomationBrowserIdSchema, ref: BrowserAutomationRefSchema, })` |
| `BrowserAutomationForwardCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:149` | `z.object({ command: z.literal("forward"), args: BrowserAutomationTabTargetSchema, })` |
| `BrowserAutomationForwardResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:352` | `z.object({ command: z.literal("forward"), browserId: BrowserAutomationBrowserIdSchema, })` |
| `BrowserAutomationHoverCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:182` | `z.object({ command: z.literal("hover"), args: BrowserAutomationTabTargetSchema.extend({ ref: BrowserAutomationRefSchema, }), })` |
| `BrowserAutomationHoverResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:385` | `z.object({ command: z.literal("hover"), browserId: BrowserAutomationBrowserIdSchema, ref: BrowserAutomationRefSchema, x: z.number().optional(), y: z.number().optional(), })` |
| `BrowserAutomationHttpUrlSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:64` | `z .string() .url() .refine((value) => { const protocol = new URL(value).protocol; return protocol === "http:" \|\| protocol === "https:"; }, "URL must use http or https")` |
| `BrowserAutomationInputModifierSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:63` | `z.enum(["Alt", "Control", "Meta", "Shift"])` |
| `BrowserAutomationKeypressCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:129` | `z.object({ command: z.literal("keypress"), args: BrowserAutomationTabTargetSchema.extend({ ref: BrowserAutomationRefSchema.optional(), key: z.string().min(1), }), })` |
| `BrowserAutomationKeypressResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:332` | `z.object({ command: z.literal("keypress"), browserId: BrowserAutomationBrowserIdSchema, key: z.string().min(1), ref: BrowserAutomationRefSchema.optional(), x: z.number().optional(), y: z.number().optional(), })` |
| `BrowserAutomationListTabsCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:72` | `z.object({ command: z.literal("list_tabs"), args: z.object({}).strict().default({}), })` |
| `BrowserAutomationListTabsResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:270` | `z.object({ command: z.literal("list_tabs"), tabs: z.array(BrowserAutomationTabInfoSchema), })` |
| `BrowserAutomationLogsCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:197` | `z.object({ command: z.literal("logs"), args: BrowserAutomationTabTargetSchema.extend({ maxEntries: z.number().int().positive().max(200).default(50), }), })` |
| `BrowserAutomationLogsResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:422` | `z.object({ command: z.literal("logs"), browserId: BrowserAutomationBrowserIdSchema, console: z.array(BrowserAutomationConsoleLogEntrySchema), network: z.array(BrowserAutomationNetworkLogEntrySchema), })` |
| `BrowserAutomationMouseButtonSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:62` | `z.enum(["left", "right", "middle"])` |
| `BrowserAutomationNavigateCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:137` | `z.object({ command: z.literal("navigate"), args: BrowserAutomationTabTargetSchema.extend({ url: BrowserAutomationHttpUrlSchema, }), })` |
| `BrowserAutomationNavigateResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:341` | `z.object({ command: z.literal("navigate"), browserId: BrowserAutomationBrowserIdSchema, url: z.string().min(1), })` |
| `BrowserAutomationNetworkLogEntrySchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:412` | `z.object({ url: z.string(), method: z.string().optional(), status: z.number().int().optional(), type: z.string().optional(), startTime: z.number(), duration: z.number(), transferSize: z.number().optional(), })` |
| `BrowserAutomationNewTabCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:77` | `z.object({ command: z.literal("new_tab"), args: z .object({ url: BrowserAutomationHttpUrlSchema.optional(), }) .strict() .default({}), })` |
| `BrowserAutomationNewTabResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:275` | `z.object({ command: z.literal("new_tab"), browserId: BrowserAutomationBrowserIdSchema, workspaceId: z.string().min(1), url: z.string().min(1), })` |
| `BrowserAutomationRefSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:61` | `z.string().regex(/^@e\d+$/)` |
| `BrowserAutomationReloadCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:154` | `z.object({ command: z.literal("reload"), args: BrowserAutomationTabTargetSchema, })` |
| `BrowserAutomationReloadResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:357` | `z.object({ command: z.literal("reload"), browserId: BrowserAutomationBrowserIdSchema, })` |
| `BrowserAutomationResizeCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:221` | `z.object({ command: z.literal("resize"), args: BrowserAutomationTabTargetSchema.extend({ width: z.number().int().positive(), height: z.number().int().positive(), }), })` |
| `BrowserAutomationResizeResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:446` | `z.object({ command: z.literal("resize"), browserId: BrowserAutomationBrowserIdSchema, width: z.number().int().positive(), height: z.number().int().positive(), })` |
| `BrowserAutomationResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:458` | `z.discriminatedUnion("command", [ BrowserAutomationListTabsResultSchema, BrowserAutomationNewTabResultSchema, BrowserAutomationSnapshotResultSchema, BrowserAutomationClickResultSchema, BrowserAutomationFillResultSchema, BrowserAutomationWaitResultSchema, BrowserAutomationTypeResultSchema, BrowserAutomationKeypressResultSchema, BrowserAutomationNavigateResultSchema, BrowserAutomationBackResultSchema, BrowserAutomationForwardResultSchema, BrowserAutomationReloadResultSchema, BrowserAutomationScreenshotResultSchema, BrowserAutomationUploadResultSchema, BrowserAutomationSelectResultSchema, BrowserAutomationHoverResultSchema, BrowserAutomationDragResultSchema, BrowserAutomationLogsResultSchema, BrowserAutomationEvaluateResultSchema, BrowserAutomationScrollResultSchema, BrowserAutomationResizeResultSchema, BrowserAutomationCloseTabResultSchema, ])` |
| `BrowserAutomationScreenshotCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:159` | `z.object({ command: z.literal("screenshot"), args: BrowserAutomationTabTargetSchema.extend({ fullPage: z.boolean().default(false), }), })` |
| `BrowserAutomationScreenshotResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:362` | `z.object({ command: z.literal("screenshot"), browserId: BrowserAutomationBrowserIdSchema, mimeType: z.literal("image/png"), dataBase64: z.string().min(1), width: z.number().int().nonnegative(), height: z.number().int().nonnegative(), })` |
| `BrowserAutomationScrollCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:212` | `z.object({ command: z.literal("scroll"), args: BrowserAutomationTabTargetSchema.extend({ ref: BrowserAutomationRefSchema.optional(), deltaX: z.number(), deltaY: z.number(), }), })` |
| `BrowserAutomationScrollResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:436` | `z.object({ command: z.literal("scroll"), browserId: BrowserAutomationBrowserIdSchema, ref: BrowserAutomationRefSchema.optional(), deltaX: z.number(), deltaY: z.number(), x: z.number().optional(), y: z.number().optional(), })` |
| `BrowserAutomationSelectCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:174` | `z.object({ command: z.literal("select"), args: BrowserAutomationTabTargetSchema.extend({ ref: BrowserAutomationRefSchema, value: z.string(), }), })` |
| `BrowserAutomationSelectResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:378` | `z.object({ command: z.literal("select"), browserId: BrowserAutomationBrowserIdSchema, ref: BrowserAutomationRefSchema, value: z.string(), })` |
| `BrowserAutomationSnapshotCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:87` | `z.object({ command: z.literal("snapshot"), args: BrowserAutomationTabTargetSchema, })` |
| `BrowserAutomationSnapshotResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:292` | `z.object({ command: z.literal("snapshot"), browserId: BrowserAutomationBrowserIdSchema, workspaceId: z.string().min(1).optional(), url: z.string(), title: z.string(), format: z.literal("aria-yaml"), snapshot: z.string(), truncated: z.boolean(), stats: BrowserAutomationSnapshotStatsSchema, })` |
| `BrowserAutomationSnapshotStatsSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:282` | `z .object({ nodeCount: z.number().int().nonnegative(), refCount: z.number().int().nonnegative(), textLength: z.number().int().nonnegative(), iframeCount: z.number().int().nonnegative().optional(), maxDepth: z.number().int().nonnegative().optional(), }) .strict()` |
| `BrowserAutomationTabInfoSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:259` | `z.object({ browserId: BrowserAutomationBrowserIdSchema, workspaceId: z.string().min(1).optional(), url: z.string(), title: z.string(), isActive: z.boolean().default(false), isLoading: z.boolean().default(false), canGoBack: z.boolean().optional(), canGoForward: z.boolean().optional(), })` |
| `BrowserAutomationTabTargetSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:55` | `z .object({ browserId: BrowserAutomationBrowserIdSchema, }) .strict()` |
| `BrowserAutomationTypeCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:121` | `z.object({ command: z.literal("type"), args: BrowserAutomationTabTargetSchema.extend({ ref: BrowserAutomationRefSchema.optional(), text: z.string(), }), })` |
| `BrowserAutomationTypeResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:324` | `z.object({ command: z.literal("type"), browserId: BrowserAutomationBrowserIdSchema, ref: BrowserAutomationRefSchema.optional(), x: z.number().optional(), y: z.number().optional(), })` |
| `BrowserAutomationUploadCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:166` | `z.object({ command: z.literal("upload"), args: BrowserAutomationTabTargetSchema.extend({ ref: BrowserAutomationRefSchema, filePaths: z.array(z.string().min(1)).min(1), }), })` |
| `BrowserAutomationUploadResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:371` | `z.object({ command: z.literal("upload"), browserId: BrowserAutomationBrowserIdSchema, ref: BrowserAutomationRefSchema, filePaths: z.array(z.string().min(1)).min(1), })` |
| `BrowserAutomationWaitCommandSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:110` | `z.object({ command: z.literal("wait"), args: BrowserAutomationTabTargetSchema.extend({ text: z.string().min(1).optional(), url: z.string().min(1).optional(), timeoutMs: z.number().int().positive().max(30000).optional(), }).refine((args) => Number(Boolean(args.text)) + Number(Boolean(args.url)) === 1, { message: BROWSER_AUTOMATION_WAIT_CONDITION_MESSAGE, }), })` |
| `BrowserAutomationWaitResultSchema` | `packages/protocol/src/browser-automation/rpc-schemas.ts:318` | `z.object({ command: z.literal("wait"), browserId: BrowserAutomationBrowserIdSchema, matched: z.enum(["text", "url"]), })` |
| `ChangeRequestCheckoutSourceSchema` | `packages/protocol/src/messages.ts:1242` | `z.object({ kind: z.literal("change_request"), forge: z.string().optional(), number: z.number().int().positive(), projectPath: z.string().optional(), })` |
| `ChatMessageSchema` | `packages/protocol/src/chat/types.ts:15` | `z.object({ id: z.string(), roomId: z.string(), authorAgentId: z.string(), body: z.string(), replyToMessageId: z.string().nullable(), mentionAgentIds: z.array(z.string()), createdAt: z.string(), })` |
| `ChatRoomDetailSchema` | `packages/protocol/src/chat/types.ts:27` | `ChatRoomSchema.extend({ messageCount: z.number().int().nonnegative(), lastMessageAt: z.string().nullable(), })` |
| `ChatRoomSchema` | `packages/protocol/src/chat/types.ts:5` | `z.object({ id: z.string(), name: z.string(), purpose: z.string().nullable(), createdAt: z.string(), updatedAt: z.string(), })` |
| `CheckoutCheckDetailsRequestPayloadSchema` | `packages/protocol/src/messages.ts:2407` | `z.object({ cwd: z.string(), repoOwner: GitHubRepoSegmentSchema.optional(), repoName: GitHubRepoSegmentSchema.optional(), checkRunId: z.number().int().positive().optional(), workflowRunId: z.number().int().positive().optional(), changeRequestNumber: z.number().int().positive().optional(), requestId: z.string(), })` |
| `CheckoutCheckDetailsSchema` | `packages/protocol/src/messages.ts:5840` | `CheckoutGithubCheckDetailsSchema` |
| `CheckoutCommitFileSchema` | `packages/protocol/src/messages.ts:2372` | `z.object({ path: z.string(), additions: z.number(), deletions: z.number(), status: z.enum(["added", "modified", "deleted", "renamed"]).optional(), })` |
| `CheckoutCommitSchema` | `packages/protocol/src/messages.ts:2379` | `z.object({ sha: z.string(), shortSha: z.string(), subject: z.string(), authorName: z.string(), authorDate: z.string(), isOnRemote: z.boolean(), isOnBase: z.boolean().optional(), files: z.array(CheckoutCommitFileSchema), })` |
| `CheckoutDiffCompareSchema` | `packages/protocol/src/messages.ts:2255` | `z.object({ mode: z.enum(["uncommitted", "base", "all"]), baseRef: z.string().optional(), ignoreWhitespace: z.boolean().optional(), })` |
| `CheckoutDiffSubscriptionPayloadSchema` | `packages/protocol/src/messages.ts:5595` | `z.object({ subscriptionId: z.string(), cwd: z.string(), files: z.array(ParsedDiffFileSchema), error: CheckoutErrorSchema.nullable(), diffTooLarge: z.boolean().optional(), })` |
| `CheckoutErrorCodeSchema` | `packages/protocol/src/messages.ts:2243` | `z.enum([ "NOT_GIT_REPO", "NOT_ALLOWED", "MERGE_CONFLICT", "UNKNOWN", ])` |
| `CheckoutErrorSchema` | `packages/protocol/src/messages.ts:2250` | `z.object({ code: CheckoutErrorCodeSchema, message: z.string(), })` |
| `CheckoutGithubCheckAnnotationSchema` | `packages/protocol/src/messages.ts:5764` | `z.object({ path: z.string().optional(), startLine: z.number().optional(), endLine: z.number().optional(), annotationLevel: z.string().optional(), message: z.string().optional(), title: z.string().optional(), rawDetails: z.string().optional(), })` |
| `CheckoutGithubCheckDetailsSchema` | `packages/protocol/src/messages.ts:5817` | `z.object({ checkRunId: z.number(), workflowRunId: z.number().nullable().optional(), name: z.string(), status: z.string().nullable().optional(), conclusion: z.string().nullable().optional(), url: z.string().nullable().optional(), detailsUrl: z.string().nullable().optional(), output: z .object({ title: z.string().nullable().optional(), summary: z.string().nullable().optional(), text: z.string().nullable().optional(), }) .nullable() .optional(), annotations: z.array(CheckoutGithubCheckAnnotationSchema).optional().default([]), failedJobs: z.array(CheckoutGithubCheckJobSchema).optional().default([]), truncated: z.boolean().optional().default(false), pipeline: CheckoutPipelineSchema.nullable().optional(), })` |
| `CheckoutGithubCheckJobSchema` | `packages/protocol/src/messages.ts:5774` | `z.object({ jobId: z.number(), name: z.string(), status: z.string().nullable().optional(), conclusion: z.string().nullable().optional(), url: z.string().nullable().optional(), logTail: z.string().optional(), logTruncated: z.boolean().optional(), })` |
| `CheckoutPipelineJobSchema` | `packages/protocol/src/messages.ts:5785` | `z.object({ id: z.number(), name: z.string(), stage: z.string(), status: z.string(), rawStatus: z.string().optional(), url: z.string().nullable().optional().default(null), allowFailure: z.boolean().optional().default(false), durationSeconds: z.number().nullable().optional().default(null), })` |
| `CheckoutPipelineSchema` | `packages/protocol/src/messages.ts:5806` | `z.object({ id: z.number(), status: z.string(), rawStatus: z.string().optional(), url: z.string().nullable().optional().default(null), ref: z.string().nullable().optional().default(null), sha: z.string().nullable().optional().default(null), stages: z.array(CheckoutPipelineStageSchema).optional().default([]), })` |
| `CheckoutPipelineStageSchema` | `packages/protocol/src/messages.ts:5800` | `z.object({ name: z.string(), status: z.string(), jobs: z.array(CheckoutPipelineJobSchema).optional().default([]), })` |
| `CheckoutPrForgeSpecificSchema` | `packages/protocol/src/messages.ts:5504` | `z.unknown().optional()` |
| `CheckoutPrGithubAutoMergeRequestSchema` | `packages/protocol/src/messages.ts:5453` | `z .object({ enabledAt: z.string().nullable().optional().default(null), mergeMethod: z.string().nullable().optional().default(null), enabledBy: z.string().nullable().optional().default(null), }) .nullable() .optional() .default(null)` |
| `CheckoutPrGithubRepositoryPolicySchema` | `packages/protocol/src/messages.ts:5463` | `z .object({ autoMergeAllowed: z.boolean().optional().default(false), mergeCommitAllowed: z.boolean().optional().default(false), squashMergeAllowed: z.boolean().optional().default(false), rebaseMergeAllowed: z.boolean().optional().default(false), viewerDefaultMergeMethod: z.string().nullable().optional().default(null), }) .optional() .default({ autoMergeAllowed: false, mergeCommitAllowed: false, squashMergeAllowed: false, rebaseMergeAllowed: false, viewerDefaultMergeMethod: null, })` |
| `CheckoutPrGithubStatusObjectSchema` | `packages/protocol/src/messages.ts:5480` | `z.object({ mergeStateStatus: z.string().nullable().optional().default(null), autoMergeRequest: CheckoutPrGithubAutoMergeRequestSchema, viewerCanEnableAutoMerge: z.boolean().optional().default(false), viewerCanDisableAutoMerge: z.boolean().optional().default(false), viewerCanMergeAsAdmin: z.boolean().optional().default(false), viewerCanUpdateBranch: z.boolean().optional().default(false), repository: CheckoutPrGithubRepositoryPolicySchema, isMergeQueueEnabled: z.boolean().optional().default(false), isInMergeQueue: z.boolean().optional().default(false), })` |
| `CheckoutPrGithubStatusSchema` | `packages/protocol/src/messages.ts:5492` | `CheckoutPrGithubStatusObjectSchema.optional()` |
| `CheckoutPrStatusPayloadSchema` | `packages/protocol/src/messages.ts:5564` | `z.object({ cwd: z.string(), status: CheckoutPrStatusSchema.nullable(), githubFeaturesEnabled: z.boolean(), authState: ForgeAuthStateSchema, forge: z.string().optional().default("github"), error: CheckoutErrorSchema.nullable(), requestId: z.string(), })` |
| `CheckoutPrStatusSchema` | `packages/protocol/src/messages.ts:5506` | `z.object({ forge: z.string().optional().default("github"), projectPath: z.string().optional(), number: z.number().optional(), url: z.string(), title: z.string(), state: z.string(), baseRefName: z.string(), headRefName: z.string(), isMerged: z.boolean(), isDraft: z.boolean().optional().default(false), mergeable: z .enum(["MERGEABLE", "CONFLICTING", "UNKNOWN"]) .catch("UNKNOWN") .optional() .default("UNKNOWN"), checks: z .array(z.object({ name: z.string(), status: z.string(), url: z.string().nullable(), workflow: z.string().optional(), duration: z.string().optional(), checkRunId: z.number().optional(), workflowRunId: z.number().optional(), traits: z.array(z.string()).optional(), })) .optional() .default([]), checksStatus: z.string().optional(), reviewDecision: z.string().nullable().optional(), repoOwner: z.string().optional(), repoName: z.string().optional(), github: CheckoutPrGithubStatusSchema, forgeSpecific: CheckoutPrForgeSpecificSchema, })` |
| `CheckoutStatusCommonSchema` | `packages/protocol/src/messages.ts:5387` | `z.object({ cwd: z.string(), error: CheckoutErrorSchema.nullable(), requestId: z.string(), upstreamRef: z.string().nullable().optional(), })` |
| `CheckoutStatusGitNonPaseoSchema` | `packages/protocol/src/messages.ts:5414` | `CheckoutStatusCommonSchema.extend({ isGit: z.literal(true), isPaseoOwnedWorktree: z.literal(false), repoRoot: z.string(), mainRepoRoot: z.string().nullable().optional().default(null), currentBranch: z.string().nullable(), isDirty: z.boolean(), baseRef: z.string().nullable(), aheadBehind: AheadBehindSchema.nullable(), aheadOfOrigin: z.number().nullable(), behindOfOrigin: z.number().nullable(), hasRemote: z.boolean(), remoteUrl: z.string().nullable(), })` |
| `CheckoutStatusGitPaseoSchema` | `packages/protocol/src/messages.ts:5429` | `CheckoutStatusCommonSchema.extend({ isGit: z.literal(true), isPaseoOwnedWorktree: z.literal(true), repoRoot: z.string(), mainRepoRoot: z.string(), currentBranch: z.string().nullable(), isDirty: z.boolean(), baseRef: z.string(), aheadBehind: AheadBehindSchema.nullable(), aheadOfOrigin: z.number().nullable(), behindOfOrigin: z.number().nullable(), hasRemote: z.boolean(), remoteUrl: z.string().nullable(), })` |
| `CheckoutStatusNotGitSchema` | `packages/protocol/src/messages.ts:5400` | `CheckoutStatusCommonSchema.extend({ isGit: z.literal(false), isPaseoOwnedWorktree: z.literal(false), repoRoot: z.null(), currentBranch: z.null(), isDirty: z.null(), baseRef: z.null(), aheadBehind: z.null(), aheadOfOrigin: z.null(), behindOfOrigin: z.null(), hasRemote: z.boolean(), remoteUrl: z.null(), })` |
| `CheckoutStatusUpdateMetadataSchema` | `packages/protocol/src/messages.ts:5579` | `z.object({ subscriptionId: z.string().optional(), prStatus: CheckoutPrStatusPayloadSchema.optional(), })` |
| `CloseItemsAgentResultSchema` | `packages/protocol/src/messages.ts:5363` | `z.object({ agentId: z.string(), archivedAt: z.string(), })` |
| `CloseItemsTerminalResultSchema` | `packages/protocol/src/messages.ts:5368` | `z.object({ terminalId: z.string(), success: z.boolean(), })` |
| `CompactProviderSnapshotEntrySchema` | `packages/protocol/src/messages.ts:405` | `ProviderSnapshotEntrySchema.omit({ models: true, }).extend({ models: z.array(CompactProviderSnapshotModelSchema).optional(), })` |
| `CompactProviderSnapshotModelSchema` | `packages/protocol/src/messages.ts:393` | `AgentModelDefinitionSchema.omit({ provider: true, thinkingOptions: true, }).extend({ thinkingSet: z.number().int().nonnegative().optional(), })` |
| `CompactProviderSnapshotSchema` | `packages/protocol/src/messages.ts:411` | `z.object({ entries: z.array(CompactProviderSnapshotEntrySchema), thinkingSets: z.array(ProviderSnapshotThinkingSetSchema), })` |
| `CreateAgentWorktreeTargetSchema` | `packages/protocol/src/messages.ts:1755` | `z.discriminatedUnion("mode", [ z.object({ mode: z.literal("branch-off"), newBranch: z.string().min(1), base: z.string().min(1).optional(), }), z.object({ mode: z.literal("checkout-branch"), branch: z.string().min(1), }), z.object({ mode: z.literal("checkout-pr"), prNumber: z.number().int().positive(), }), ])` |
| `CreationSnapshotSchema` | `packages/protocol/src/messages.ts:5027` | `z.object({ kind: z.enum(["workspace", "agent"]), idempotencyKey: z.string(), revision: z.number().int().nonnegative(), phase: z.enum([ "accepted", "workspace_ready", "agent_ready", "prompt_started", "completed", "failed", ]), workspaceId: z.string().nullable(), agentId: z.string().nullable(), workspace: WorkspaceDescriptorPayloadSchema.optional(), agent: AgentSnapshotPayloadSchema.optional(), error: z.string().nullable(), setupSkippedReason: z.string().optional(), errorCode: z.string().optional(), failedStage: z.enum(["workspace", "agent", "prompt"]).optional(), outcomeUnknown: z.boolean().optional(), })` |
| `DaemonPermissionSchema` | `packages/protocol/src/messages.ts:158` | `z.enum(DAEMON_PERMISSIONS)` |
| `DiffHunkSchema` | `packages/protocol/src/messages.ts:2789` | `z.object({ oldStart: z.number(), oldCount: z.number(), newStart: z.number(), newCount: z.number(), lines: z.array(DiffLineSchema), })` |
| `DiffLineSchema` | `packages/protocol/src/messages.ts:2783` | `z.object({ type: z.enum(["add", "remove", "context", "header"]), content: z.string(), tokens: z.array(HighlightTokenSchema).optional(), })` |
| `DirectoryPluginSourceSchema` | `packages/protocol/src/plugin-config.ts:8` | `z .object({ source: z.literal("directory"), path: z.string().min(1), enabled: z.boolean().optional(), }) .strict()` |
| `DirectorySyncMetadataSchema` | `packages/protocol/src/messages.ts:1281` | `z.object({ generation: z.string(), headSeq: z.number().int().nonnegative(), mode: z.enum(["snapshot", "changes"]), reason: z .enum(["no_cursor", "generation_changed", "cursor_expired", "changes_too_large"]) .optional(), removals: z.array(DirectorySyncRemovalSchema), })` |
| `DirectorySyncRemovalSchema` | `packages/protocol/src/messages.ts:1276` | `z.object({ id: z.string(), seq: z.number().int().positive(), })` |
| `DirectorySyncRequestSchema` | `packages/protocol/src/messages.ts:1271` | `z.object({ generation: z.string().optional(), afterSeq: z.number().int().nonnegative().optional(), })` |
| `ExternalResourceAttachmentMetadataSchema` | `packages/protocol/src/messages.ts:1155` | `z.object({ provider: z.string(), providerLabel: z.string(), resourceType: z.string(), id: z.string(), identifier: z.string(), title: z.string(), url: z.string(), })` |
| `FileExplorerDirectorySchema` | `packages/protocol/src/messages.ts:2828` | `z.object({ path: z.string(), entries: z.array(FileExplorerEntrySchema), })` |
| `FileExplorerEntrySchema` | `packages/protocol/src/messages.ts:2809` | `z.object({ name: z.string(), path: z.string(), kind: z.enum(["file", "directory"]), size: z.number(), modifiedAt: z.string(), })` |
| `FileExplorerFileSchema` | `packages/protocol/src/messages.ts:2817` | `z.object({ path: z.string(), kind: z.enum(["text", "image", "binary"]), encoding: z.enum(["utf-8", "base64", "none"]), content: z.string().optional(), mimeType: z.string().optional(), size: z.number(), modifiedAt: z.string(), revision: z.string().optional(), })` |
| `FileVersionSchema` | `packages/protocol/src/messages.ts:2843` | `z.discriminatedUnion("status", [ z.object({ status: z.literal("ready"), cwd: z.string(), path: z.string(), size: z.number().int().nonnegative(), modifiedAt: z.string(), revision: z.string().optional(), kind: z.enum(["text", "image", "binary"]).optional(), mimeType: z.string().optional(), }), z.object({ status: z.literal("missing"), cwd: z.string(), path: z.string(), }), z.object({ status: z.literal("error"), cwd: z.string(), path: z.string(), error: z.string(), }), ])` |
| `FileWriteResultSchema` | `packages/protocol/src/messages.ts:6192` | `z.discriminatedUnion("status", [ z.object({ status: z.literal("written"), modifiedAt: z.string(), size: z.number(), revision: z.string().optional(), }), z.object({ status: z.literal("conflict"), version: FileVersionSchema }), z.object({ status: z.literal("error"), error: z.string() }), ])` |
| `FirstAgentContextSchema` | `packages/protocol/src/messages.ts:2603` | `z.object({ prompt: z.string().optional(), attachments: AgentAttachmentsSchema, })` |
| `ForgeAuthStateSchema` | `packages/protocol/src/messages.ts:5562` | `z.unknown().optional()` |
| `ForgeChangeRequestAttachmentSchema` | `packages/protocol/src/messages.ts:1119` | `z.object({ type: z.literal("forge_change_request"), mimeType: z.literal("application/paseo-forge-change-request"), forge: z.string().optional().default("github"), number: z.number().int().positive(), title: z.string(), url: z.string(), body: z.string().nullable().optional(), projectPath: z.string().optional(), baseRefName: z.string().nullable().optional(), headRefName: z.string().nullable().optional(), })` |
| `ForgeIssueAttachmentSchema` | `packages/protocol/src/messages.ts:1144` | `z.object({ type: z.literal("forge_issue"), mimeType: z.literal("application/paseo-forge-issue"), forge: z.string().optional().default("github"), number: z.number().int().positive(), title: z.string(), url: z.string(), body: z.string().nullable().optional(), projectPath: z.string().optional(), })` |
| `ForgeSearchKindSchema` | `packages/protocol/src/messages.ts:2530` | `z.enum([ "issue", "change_request", "github-issue", "github-pr", "pr", ])` |
| `ForgeSearchResponsePayloadSchema` | `packages/protocol/src/messages.ts:6077` | `z.object({ items: z.array(z.unknown()), authState: z.unknown().optional(), error: z.string().nullable(), requestId: z.string(), })` |
| `GitHubIssueAttachmentSchema` | `packages/protocol/src/messages.ts:1135` | `z.object({ type: z.literal("github_issue"), mimeType: z.literal("application/github-issue"), number: z.number().int().positive(), title: z.string(), url: z.string(), body: z.string().nullable().optional(), })` |
| `GitHubPrAttachmentSchema` | `packages/protocol/src/messages.ts:1108` | `z.object({ type: z.literal("github_pr"), mimeType: z.literal("application/github-pr"), number: z.number().int().positive(), title: z.string(), url: z.string(), body: z.string().nullable().optional(), baseRefName: z.string().nullable().optional(), headRefName: z.string().nullable().optional(), })` |
| `GitHubRepoSegmentSchema` | `packages/protocol/src/messages.ts:2405` | `z.string().regex(/^[A-Za-z0-9._-]+$/)` |
| `GitHubSearchKindSchema` | `packages/protocol/src/messages.ts:2538` | `ForgeSearchKindSchema` |
| `GitHubSearchResponsePayloadSchema` | `packages/protocol/src/messages.ts:6084` | `z.object({ items: z.array(z.unknown()), featuresEnabled: z.boolean().optional(), authState: z.unknown().optional(), githubFeaturesEnabled: z.boolean().optional(), error: z.string().nullable(), requestId: z.string(), })` |
| `GitSetupOptionsSchema` | `packages/protocol/src/messages.ts:1738` | `z.object({ baseBranch: z.string().optional(), createNewBranch: z.boolean().optional(), newBranchName: z.string().optional(), createWorktree: z.boolean().optional(), worktreeSlug: z.string().optional(), refName: z.string().min(1).optional(), action: z.enum(["branch-off", "checkout"]).optional(), checkoutSource: ChangeRequestCheckoutSourceSchema.optional(), githubPrNumber: z.number().int().positive().optional(), })` |
| `GithubRepositorySchema` | `packages/protocol/src/messages.ts:2682` | `z.object({ id: z.string().min(1), name: z.string().min(1), nameWithOwner: z.string().min(MIN_REPOSITORY_PATH_LENGTH), description: z.string().nullable(), visibility: z.enum(["public", "private", "internal"]), updatedAt: z.string(), cloneUrl: z.string().min(MIN_REPOSITORY_PATH_LENGTH), })` |
| `HighlightTokenSchema` | `packages/protocol/src/messages.ts:2778` | `z.object({ text: z.string(), style: z.string().nullable(), })` |
| `HostAddSchema` | `packages/protocol/src/hosts.ts:4` | `z.object({ connection: z.string().min(1).max(16384), label: z.string().max(256).optional(), metadata: HostMetadataSchema.optional(), sourceKey: z.string().min(1).max(256).optional(), })` |
| `HostDirectorySchema` | `packages/protocol/src/hosts.ts:30` | `z.object({ self: z.object({ serverId: z.string(), label: z.string() }), hosts: z.array(HostSummarySchema), })` |
| `HostMetadataSchema` | `packages/protocol/src/hosts.ts:3` | `z.record(z.string().max(256), z.string().max(4096))` |
| `HostSummarySchema` | `packages/protocol/src/hosts.ts:17` | `z.object({ registrationId: z.string(), serverId: z.string().nullable(), label: z.string().nullable(), source: z.object({ id: z.string(), key: z.string() }), metadata: HostMetadataSchema, enabled: z.boolean(), status: z.enum(["connecting", "online", "offline", "error", "disabled"]), targetDescription: z.string(), lastOnlineAt: z.string().nullable(), nextRetryAt: z.string().nullable(), error: z.object({ code: z.string(), message: z.string() }).nullable(), })` |
| `HostUpdateSchema` | `packages/protocol/src/hosts.ts:10` | `z.object({ label: z.string().max(256).nullable().optional(), metadata: HostMetadataSchema.optional(), enabled: z.boolean().optional(), connection: z.string().min(1).max(16384).optional(), replaceIdentity: z.boolean().optional(), })` |
| `HubExecutionAgentCreateErrorSchema` | `packages/protocol/src/messages.ts:3193` | `z.discriminatedUnion("code", [ z.object({ code: z.literal("provider_options_invalid"), provider: z.string(), issues: z.array(z.object({ path: z.array(z.union([z.string(), z.number()])), message: z.string(), })), message: z.string(), }), z.object({ code: z.literal("tool_policy_unsupported"), provider: z.string(), message: z.string(), }), z.object({ code: z.literal("create_failed"), message: z.string(), }), ])` |
| `HubExecutionAgentValidationIssueSchema` | `packages/protocol/src/messages.ts:6736` | `z.object({ path: z.array(z.union([z.string(), z.number()])), message: z.string(), })` |
| `HubExecutionControlActionSchema` | `packages/protocol/src/messages.ts:3218` | `z.enum(["interrupt", "archive"])` |
| `HubRelationshipStatusSchema` | `packages/protocol/src/messages.ts:5203` | `z.object({ state: z.enum([ "not_connected", "connecting", "connected", "reconnecting", "disconnecting", "revoked", ]), daemonId: z.string().nullable(), hubOrigin: z.string().nullable(), permissions: z.array(DaemonPermissionSchema), connectedAt: z.string().nullable(), lastError: z.string().nullable(), })` |
| `ImageAttachmentSchema` | `packages/protocol/src/messages.ts:1249` | `z.object({ data: z.string(), mimeType: z.string(), })` |
| `JsonWireValueSchema` | `packages/protocol/src/messages.ts:304` | `z.unknown() as z.ZodType<JsonValue>` |
| `ListCommandsDraftConfigSchema` | `packages/protocol/src/messages.ts:2977` | `z.object({ provider: AgentProviderSchema, cwd: z.string(), modeId: z.string().optional(), model: z.string().optional(), thinkingOptionId: z.string().optional(), featureValues: z.record(z.string(), z.unknown()).optional(), })` |
| `LocalSpeechModelSchema` | `packages/protocol/src/speech/rpc-schemas.ts:139` | `z .object({ id: z.string(), kind: z.string(), label: z.string(), description: z.string(), languages: z.array(z.string()), downloadSizeBytes: z.number(), isDefault: z.boolean(), inUse: z.boolean(), status: z.string(), receivedBytes: z.number().optional(), totalBytes: z.number().optional(), error: z.string().optional(), }) .passthrough()` |
| `LoopIterationRecordSchema` | `packages/protocol/src/loop/rpc-schemas.ts:33` | `z.object({ index: z.number().int().positive(), workerAgentId: z.string().nullable(), workerStartedAt: z.string(), workerCompletedAt: z.string().nullable(), verifierAgentId: z.string().nullable(), status: z.enum(["running", "succeeded", "failed", "stopped"]), workerOutcome: z.enum(["completed", "failed", "canceled"]).nullable(), failureReason: z.string().nullable(), verifyChecks: z.array(LoopVerifyCheckResultSchema), verifyPrompt: LoopVerifyPromptResultSchema.nullable(), })` |
| `LoopListItemSchema` | `packages/protocol/src/loop/rpc-schemas.ts:79` | `z.object({ id: z.string(), name: z.string().nullable(), status: z.enum(["running", "succeeded", "failed", "stopped"]), cwd: z.string(), createdAt: z.string(), updatedAt: z.string(), activeIteration: z.number().int().positive().nullable(), })` |
| `LoopLogEntrySchema` | `packages/protocol/src/loop/rpc-schemas.ts:6` | `z.object({ seq: z.number().int().positive(), timestamp: z.string(), iteration: z.number().int().positive().nullable(), source: z.enum(["loop", "worker", "verifier", "verify-check"]), level: z.enum(["info", "error"]), text: z.string(), })` |
| `LoopRecordSchema` | `packages/protocol/src/loop/rpc-schemas.ts:46` | `z.object({ id: z.string(), name: z.string().nullable(), prompt: z.string(), cwd: z.string(), provider: AgentProviderSchema, model: z.string().nullable(), modeId: z.string().nullable().default(null), workerProvider: AgentProviderSchema.nullable(), workerModel: z.string().nullable(), verifierProvider: AgentProviderSchema.nullable(), verifierModel: z.string().nullable(), verifierModeId: z.string().nullable().default(null), verifyPrompt: z.string().nullable(), verifyChecks: z.array(z.string()), archive: z.boolean(), sleepMs: z.number().int().nonnegative(), maxIterations: z.number().int().positive().nullable(), maxTimeMs: z.number().int().positive().nullable(), status: z.enum(["running", "succeeded", "failed", "stopped"]), createdAt: z.string(), updatedAt: z.string(), startedAt: z.string(), completedAt: z.string().nullable(), stopRequestedAt: z.string().nullable(), iterations: z.array(LoopIterationRecordSchema), logs: z.array(LoopLogEntrySchema), nextLogSeq: z.number().int().positive(), activeIteration: z.number().int().positive().nullable(), activeWorkerAgentId: z.string().nullable(), activeVerifierAgentId: z.string().nullable(), })` |
| `LoopVerifyCheckResultSchema` | `packages/protocol/src/loop/rpc-schemas.ts:15` | `z.object({ command: z.string(), exitCode: z.number().int(), passed: z.boolean(), stdout: z.string(), stderr: z.string(), startedAt: z.string(), completedAt: z.string(), })` |
| `LoopVerifyPromptResultSchema` | `packages/protocol/src/loop/rpc-schemas.ts:25` | `z.object({ passed: z.boolean(), reason: z.string(), verifierAgentId: z.string().nullable(), startedAt: z.string(), completedAt: z.string(), })` |
| `McpHttpServerConfigSchema` | `packages/protocol/src/messages.ts:451` | `z.object({ type: z.literal("http"), url: z.string(), headers: z.record(z.string(), z.string()).optional(), alwaysLoad: z.boolean().optional(), })` |
| `McpServerConfigSchema` | `packages/protocol/src/messages.ts:465` | `z.discriminatedUnion("type", [ McpStdioServerConfigSchema, McpHttpServerConfigSchema, McpSseServerConfigSchema, ])` |
| `McpSseServerConfigSchema` | `packages/protocol/src/messages.ts:458` | `z.object({ type: z.literal("sse"), url: z.string(), headers: z.record(z.string(), z.string()).optional(), alwaysLoad: z.boolean().optional(), })` |
| `McpStdioServerConfigSchema` | `packages/protocol/src/messages.ts:443` | `z.object({ type: z.literal("stdio"), command: z.string(), args: z.array(z.string()).optional(), env: z.record(z.string(), z.string()).optional(), alwaysLoad: z.boolean().optional(), })` |
| `McpToolRefSchema` | `packages/protocol/src/messages.ts:473` | `z .object({ kind: z.literal("mcp"), server: z.string().trim().min(1), tool: z.string().trim().min(1), }) .strict()` |
| `MutableBrowserToolsConfigSchema` | `packages/protocol/src/messages.ts:192` | `z .object({ enabled: z.boolean().default(false), }) .passthrough()` |
| `MutableDaemonConfigPatchSchema` | `packages/protocol/src/messages.ts:257` | `z .object({ relay: MutableRelayConfigSchema.partial().optional(), mcp: z.object({ injectIntoAgents: z.boolean().optional() }).passthrough().optional(), usage: z.object({ enabled: z.boolean() }).optional(), browserTools: MutableBrowserToolsConfigSchema.partial().optional(), providers: z .record(z.string(), MutableDaemonProviderConfigSchema.partial().passthrough()) .optional(), removeProviders: z.array(z.string().min(1)).optional(), metadataGeneration: MutableMetadataGenerationConfigSchema.partial().optional(), autoArchiveAfterMerge: z.boolean().optional(), enableTerminalAgentHooks: z.boolean().optional(), idleRuntimeCleanup: z.boolean().optional(), resumeOnStartup: z.boolean().optional(), resumePrompt: z.string().optional(), appendSystemPrompt: z.string().optional(), terminalProfiles: z.array(TerminalProfileSchema).optional(), agentProfiles: z.array(AgentProfileSchema).optional(), pluginsEnabled: z.boolean().optional(), plugins: z.record(PluginIdSchema, PluginSourceSchema).optional(), features: SpeechSettingsPatchSchema.optional(), }) .partial() .passthrough()` |
| `MutableDaemonConfigSchema` | `packages/protocol/src/messages.ts:206` | `z .object({ relay: MutableRelayConfigSchema.optional(), mcp: z .object({ enabled: z.boolean().optional(), injectIntoAgents: z.boolean(), }) .passthrough(), hostnames: z.union([z.literal(true), z.array(z.string())]).optional(), cors: z .object({ allowedOrigins: z.array(z.string()), }) .passthrough() .optional(), trustedProxies: z.union([z.literal(true), z.array(z.string())]).optional(), git: z .object({ maxProcessesPerSecond: z.number().int().positive(), maxProcessConcurrency: z.number().int().positive(), }) .optional(), app: z.object({ baseUrl: z.string() }).optional(), catalogRefreshTimeoutMs: z.number().int().positive().optional(), usage: z.object({ enabled: z.boolean() }).optional(), browserTools: MutableBrowserToolsConfigSchema.default({ enabled: false }), providers: z.record(z.string(), MutableDaemonProviderConfigSchema).default({}), metadataGeneration: MutableMetadataGenerationConfigSchema.default({ providers: [] }), autoArchiveAfterMerge: z.boolean().default(false), enableTerminalAgentHooks: z.boolean().default(false), idleRuntimeCleanup: z.boolean().optional(), resumeOnStartup: z.boolean().optional(), resumePrompt: z.string().optional(), appendSystemPrompt: z.string().default(""), terminalProfiles: z.array(TerminalProfileSchema).optional(), agentProfiles: z.array(AgentProfileSchema).optional(), skills: z.object({ selection: AgentSkillSelectionSchema.optional() }).strict().optional(), pluginsEnabled: z.boolean().optional(), plugins: z.record(PluginIdSchema, PluginSourceSchema).optional(), features: SpeechSettingsSchema.optional(), envOverrides: z.record(z.string(), z.string()).optional(), }) .passthrough()` |
| `MutableDaemonProviderConfigSchema` | `packages/protocol/src/messages.ts:170` | `z .object({ paseoTools: ProviderPaseoToolsPolicySchema.optional(), enabled: z.boolean().optional(), additionalModels: z.array(MutableDaemonProviderModelSchema).optional(), }) .passthrough()` |
| `MutableDaemonProviderModelSchema` | `packages/protocol/src/messages.ts:161` | `z .object({ id: z.string().min(1), label: z.string().min(1), description: z.string().optional(), isDefault: z.boolean().optional(), }) .passthrough()` |
| `MutableMetadataGenerationConfigSchema` | `packages/protocol/src/messages.ts:186` | `z .object({ providers: z.array(MutableStructuredGenerationProviderSchema).default([]), }) .passthrough()` |
| `MutableRelayConfigSchema` | `packages/protocol/src/messages.ts:197` | `z .object({ enabled: z.boolean(), requireProtectedSecurity: z.boolean().optional(), }) .passthrough()` |
| `MutableStructuredGenerationProviderSchema` | `packages/protocol/src/messages.ts:178` | `z .object({ provider: z.string().min(1), model: z.string().min(1).optional(), thinkingOptionId: z.string().min(1).optional(), }) .passthrough()` |
| `NonNullUnknownSchema` | `packages/protocol/src/messages.ts:550` | `z.union([ z.boolean(), z.number(), z.string(), z.array(z.unknown()), z.object({}).passthrough(), ])` |
| `ParsedDiffFileSchema` | `packages/protocol/src/messages.ts:2797` | `z.object({ path: z.string(), oldPath: z.string().optional(), isNew: z.boolean(), isDeleted: z.boolean(), additions: z.number(), deletions: z.number(), hunks: z.array(DiffHunkSchema), status: z.enum(["ok", "too_large", "binary"]).optional(), })` |
| `PaseoConfigRawSchema` | `packages/protocol/src/paseo-config-schema.ts:74` | `z .object({ worktree: PaseoWorktreeConfigRawSchema.optional(), scripts: z.record(z.string(), PaseoScriptEntryRawSchema).optional(), metadataGeneration: PaseoMetadataGenerationSchema.optional(), }) .passthrough()` |
| `PaseoConfigRevisionSchema` | `packages/protocol/src/paseo-config-schema.ts:99` | `z.object({ mtimeMs: z.number(), size: z.number(), })` |
| `PaseoLifecycleCommandRawSchema` | `packages/protocol/src/paseo-config-schema.ts:36` | `z.union([z.string(), z.array(z.string())])` |
| `PaseoMetadataGenerationEntrySchema` | `packages/protocol/src/paseo-config-schema.ts:55` | `z .object({ instructions: z.string().optional(), }) .passthrough() .catch({})` |
| `PaseoMetadataGenerationSchema` | `packages/protocol/src/paseo-config-schema.ts:62` | `z .object({ title: PaseoMetadataGenerationEntrySchema.optional(), branchName: PaseoMetadataGenerationEntrySchema.optional(), commitMessage: PaseoMetadataGenerationEntrySchema.optional(), pullRequest: PaseoMetadataGenerationEntrySchema.optional(), }) .passthrough() .catch({})` |
| `PaseoScriptEntryRawSchema` | `packages/protocol/src/paseo-config-schema.ts:38` | `z .object({ type: z.unknown().optional(), command: z.unknown().optional(), port: z.unknown().optional(), }) .passthrough()` |
| `PaseoServicePortAllocationSchema` | `packages/protocol/src/paseo-config-schema.ts:5` | `z .object({ range: z.string().trim().regex(TCP_PORT_RANGE_PATTERN).optional(), portScript: z.string().trim().min(1).optional(), }) .strict() .refine((value) => value.range !== undefined \|\| value.portScript !== undefined, "Expected range or portScript") .refine((value) => { if (!value.range) return true; const match = TCP_PORT_RANGE_PATTERN.exec(value.range); if (!match) return false; const start = Number(match[1]); const end = Number(match[2]); return start >= 1 && end <= 65535 && start <= end; }, "Expected an inclusive TCP port range from 1-65535")` |
| `PaseoWorktreeConfigRawSchema` | `packages/protocol/src/paseo-config-schema.ts:46` | `z .object({ setup: PaseoLifecycleCommandRawSchema.optional(), teardown: PaseoLifecycleCommandRawSchema.optional(), terminals: z.unknown().optional(), servicePorts: PaseoServicePortAllocationSchema.optional(), }) .passthrough()` |
| `PaseoWorktreeSchema` | `packages/protocol/src/messages.ts:6124` | `z.object({ worktreePath: z.string(), createdAt: z.string(), branchName: z.string().nullable().optional(), head: z.string().nullable().optional(), })` |
| `PluginIdSchema` | `packages/protocol/src/plugin-config.ts:3` | `z.string().regex(/^[a-z][a-z0-9-]*$/)` |
| `PluginInstallationSchema` | `packages/protocol/src/messages.ts:1547` | `z.object({ identity: PluginSourceIdentitySchema, currentRevision: z.string().optional(), })` |
| `PluginListItemSchema` | `packages/protocol/src/messages.ts:6855` | `z.object({ id: PluginIdSchema, name: z.string().optional(), icon: z.string().optional(), media: z.array(z.string()).optional(), description: z.string().optional(), path: z.string(), enabled: z.boolean(), status: PluginStatusSchema, source: z.enum(["directory", "git"]).optional(), npm: PluginNpmInstallationSchema.optional(), installation: PluginInstallationSchema.optional(), remote: z.string().optional(), ref: z.string().optional(), commit: z.string().optional(), error: z.string().optional(), })` |
| `PluginLogEntrySchema` | `packages/protocol/src/messages.ts:6874` | `z.object({ sequence: z.number().int().nonnegative(), timestamp: z.string().datetime(), stream: z.enum(["stdout", "stderr"]), message: z.string(), })` |
| `PluginNpmInstallationSchema` | `packages/protocol/src/messages.ts:6848` | `z.object({ packageName: z.string(), requestedSpec: z.string(), version: z.string(), integrity: z.string(), })` |
| `PluginRegistryIdSchema` | `packages/protocol/src/plugin-registry.ts:3` | `z .string() .regex(/^[a-z0-9]+(?:-[a-z0-9]+)*\/[a-z0-9]+(?:-[a-z0-9]+)*$/)` |
| `PluginRegistryIdentitySchema` | `packages/protocol/src/plugin-registry.ts:6` | `z.object({ url: z.string().url(), id: PluginRegistryIdSchema, })` |
| `PluginRequirementsSchema` | `packages/protocol/src/plugin-config.ts:5` | `z.object({ paseo: z.string().optional() })` |
| `PluginSourceIdentitySchema` | `packages/protocol/src/messages.ts:1532` | `z.discriminatedUnion("kind", [ z.object({ kind: z.literal("directory"), path: z.string() }), z.object({ kind: z.literal("git"), remote: z.string(), pluginPath: z.string(), registry: PluginRegistryIdentitySchema.optional(), }), z.object({ kind: z.literal("npm"), packageName: z.string(), pluginPath: z.string(), registry: PluginRegistryIdentitySchema.optional(), }), ])` |
| `PluginSourceSchema` | `packages/protocol/src/plugin-config.ts:16` | `z.discriminatedUnion("source", [DirectoryPluginSourceSchema])` |
| `PluginSourceStatusItemSchema` | `packages/protocol/src/messages.ts:6911` | `z.object({ id: PluginIdSchema, source: z.enum(["directory", "git"]), npm: PluginNpmInstallationSchema.optional(), installation: PluginInstallationSchema.optional(), path: z.string(), remote: z.string().optional(), ref: z.string().optional(), currentCommit: z.string().optional(), latestCommit: z.string().optional(), commitsBehind: z.number().int().nonnegative().optional(), updateAvailable: z.boolean().optional(), })` |
| `PluginSourceUpdateItemSchema` | `packages/protocol/src/messages.ts:6931` | `z.object({ id: PluginIdSchema, previousCommit: z.string(), currentCommit: z.string(), commits: z.number().int().nonnegative(), updated: z.boolean(), })` |
| `PluginStatusSchema` | `packages/protocol/src/messages.ts:6845` | `z.enum(["running", "disabled", "failed"])` |
| `PluginUpdatePreviewSchema` | `packages/protocol/src/messages.ts:6945` | `z.object({ id: PluginIdSchema, outcome: z.enum(["update", "current", "installed-newer", "local", "error"]), current: PluginInstallationSchema.optional(), target: PluginUpdateTargetSchema.optional(), links: z.array(z.string()), proposal: PluginUpdateProposalSchema.optional(), error: z.string().optional(), })` |
| `PluginUpdateProposalSchema` | `packages/protocol/src/messages.ts:1562` | `z.object({ id: PluginIdSchema, expected: z.object({ identity: PluginSourceIdentitySchema, installationRoot: z.string(), revision: z.string(), }), target: PluginUpdateTargetSchema, })` |
| `PluginUpdateResultSchema` | `packages/protocol/src/messages.ts:6955` | `z.object({ id: PluginIdSchema, outcome: z.enum(["updated", "error"]), plugin: PluginListItemSchema.optional(), error: z.string().optional(), warning: z.string().optional(), })` |
| `PluginUpdateSelectionSchema` | `packages/protocol/src/messages.ts:1572` | `z.discriminatedUnion("kind", [ z.object({ kind: z.literal("git"), ref: z.string().min(1) }), z.object({ kind: z.literal("npm"), version: z.string().min(1) }), ])` |
| `PluginUpdateTargetSchema` | `packages/protocol/src/messages.ts:1552` | `z.discriminatedUnion("kind", [ z.object({ kind: z.literal("git"), commit: z.string().regex(/^[0-9a-f]{40,64}$/) }), z.object({ kind: z.literal("npm"), version: z.string().min(1), resolved: z.string().url(), integrity: z.string().min(1), }), ])` |
| `ProjectCheckoutLiteGitNonPaseoPayloadSchema` | `packages/protocol/src/messages.ts:4026` | `z .object({ cwd: z.string(), isGit: z.literal(true), currentBranch: z.string().nullable(), remoteUrl: z.string().nullable(), worktreeRoot: z.string().optional(), isPaseoOwnedWorktree: z.literal(false), mainRepoRoot: z.string().nullable().optional().default(null), }) .transform((value) => ({ ...value, worktreeRoot: value.worktreeRoot ?? value.cwd, }))` |
| `ProjectCheckoutLiteGitPaseoPayloadSchema` | `packages/protocol/src/messages.ts:4041` | `z .object({ cwd: z.string(), isGit: z.literal(true), currentBranch: z.string().nullable(), remoteUrl: z.string().nullable(), worktreeRoot: z.string().optional(), isPaseoOwnedWorktree: z.literal(true), mainRepoRoot: z.string(), }) .transform((value) => ({ ...value, worktreeRoot: value.worktreeRoot ?? value.cwd, }))` |
| `ProjectCheckoutLiteNotGitPayloadSchema` | `packages/protocol/src/messages.ts:4011` | `z .object({ cwd: z.string(), isGit: z.literal(false), currentBranch: z.null(), remoteUrl: z.null(), worktreeRoot: z.null().optional(), isPaseoOwnedWorktree: z.literal(false), mainRepoRoot: z.null(), }) .transform((value) => ({ ...value, worktreeRoot: null, }))` |
| `ProjectCheckoutLitePayloadSchema` | `packages/protocol/src/messages.ts:4056` | `z.union([ ProjectCheckoutLiteNotGitPayloadSchema, ProjectCheckoutLiteGitNonPaseoPayloadSchema, ProjectCheckoutLiteGitPaseoPayloadSchema, ])` |
| `ProjectConfigRpcErrorSchema` | `packages/protocol/src/paseo-config-schema.ts:104` | `z.discriminatedUnion("code", [ z.object({ code: z.literal("project_not_found") }), z.object({ code: z.literal("invalid_project_config") }), z.object({ code: z.literal("stale_project_config"), currentRevision: PaseoConfigRevisionSchema.nullable(), }), z.object({ code: z.literal("write_failed") }), ])` |
| `ProjectGithubCloneProtocolSchema` | `packages/protocol/src/messages.ts:2699` | `z.enum(["https", "ssh"])` |
| `ProjectIconSchema` | `packages/protocol/src/messages.ts:6266` | `z.object({ data: z.string(), mimeType: z.string(), })` |
| `ProjectIconSourceSchema` | `packages/protocol/src/messages.ts:999` | `z.discriminatedUnion("type", [ z.object({ type: z.literal("automatic") }), z.object({ type: z.literal("upload"), data: z.string() }), ])` |
| `ProjectPlacementPayloadSchema` | `packages/protocol/src/messages.ts:4062` | `z.object({ projectKey: z.string(), projectName: z.string(), workspaceName: z.string().nullable().optional(), checkout: ProjectCheckoutLitePayloadSchema, })` |
| `ProjectRemoveResponsePayloadSchema` | `packages/protocol/src/messages.ts:2149` | `z.object({ requestId: z.string(), projectId: z.string(), accepted: z.boolean(), removedWorkspaceIds: z.array(z.string()).default([]), error: z.string().nullable(), })` |
| `ProjectRenameResponsePayloadSchema` | `packages/protocol/src/messages.ts:2126` | `z.object({ requestId: z.string(), projectId: z.string(), accepted: z.boolean(), customName: z.string().nullable(), error: z.string().nullable(), })` |
| `ProjectSuggestionSchema` | `packages/protocol/src/messages.ts:4515` | `z.object({ rootPath: z.string(), displayName: z.string(), sessionCount: z.number().int().nonnegative(), providerIds: z.array(z.string()), lastActivityAt: z.string(), })` |
| `ProviderAvailabilitySchema` | `packages/protocol/src/messages.ts:6347` | `z.object({ provider: AgentProviderSchema, available: z.boolean(), error: z.string().nullable().optional(), })` |
| `ProviderOptionsSchema` | `packages/protocol/src/messages.ts:471` | `z.record(z.string(), z.unknown())` |
| `ProviderPaseoToolsPolicySchema` | `packages/protocol/src/provider-config.ts:31` | `z.object({ enabled: z.boolean().optional(), disabledTools: z.array(z.string()).optional(), })` |
| `ProviderSnapshotEntrySchema` | `packages/protocol/src/messages.ts:378` | `z.object({ provider: AgentProviderSchema, status: ProviderStatusSchema, enabled: z.boolean().optional().default(true), source: z.enum(["builtin", "custom"]).optional(), error: z.string().optional(), models: z.array(AgentModelDefinitionSchema).optional(), modes: z.array(AgentModeSchema).optional(), fetchedAt: z.string().optional(), label: z.string().optional(), description: z.string().optional(), iconSvg: z.string().optional(), defaultModeId: z.string().nullable().optional(), })` |
| `ProviderSnapshotThinkingSetSchema` | `packages/protocol/src/messages.ts:400` | `z.object({ options: z.array(AgentSelectOptionSchema), defaultOptionId: z.string().optional(), })` |
| `ProviderStatusSchema` | `packages/protocol/src/messages.ts:316` | `z.enum([ "ready", "loading", "error", "unavailable", ])` |
| `ProviderSubagentDescriptorPayloadSchema` | `packages/protocol/src/messages.ts:4860` | `z.object({ id: z.string(), parentAgentId: z.string(), parentSubagentId: z.string().nullable().optional(), provider: AgentProviderSchema, title: z.string().nullable(), description: z.string().nullable(), status: z.enum(["running", "completed", "failed", "canceled"]), createdAt: z.string(), updatedAt: z.string(), toolCallId: z.string().nullable(), cwd: z.string().nullable().optional(), subtitle: z.string().nullable().optional(), })` |
| `ProviderUsageBalanceSchema` | `packages/protocol/src/messages.ts:6432` | `z.object({ id: z.string(), label: z.string(), used: z.number().nullable().optional(), remaining: z.number().nullable().optional(), limit: z.number().nullable().optional(), unit: z.enum(["usd", "credits", "requests", "tokens"]), resetsAt: z.string().nullable().optional(), tone: ProviderUsageToneSchema.optional(), })` |
| `ProviderUsageDetailSchema` | `packages/protocol/src/messages.ts:6443` | `z.object({ id: z.string(), label: z.string(), value: z.string(), tone: ProviderUsageToneSchema.optional(), })` |
| `ProviderUsageSchema` | `packages/protocol/src/messages.ts:6450` | `z.object({ providerId: z.string(), displayName: z.string(), status: ProviderUsageStatusSchema, planLabel: z.string().nullable(), sourceLabel: z.string().nullable().optional(), fetchedAt: z.string().nullable().optional(), nextRefreshAt: z.string().nullable().optional(), windows: z.array(ProviderUsageWindowSchema), balances: z.array(ProviderUsageBalanceSchema).optional(), details: z.array(ProviderUsageDetailSchema).optional(), error: z.string().nullable().optional(), })` |
| `ProviderUsageStatusSchema` | `packages/protocol/src/messages.ts:6412` | `z.enum(["available", "unavailable", "error"])` |
| `ProviderUsageToneSchema` | `packages/protocol/src/messages.ts:6411` | `z.enum(["default", "ok", "warning", "danger"])` |
| `ProviderUsageWindowSchema` | `packages/protocol/src/messages.ts:6414` | `z.object({ id: z.string(), label: z.string(), shortLabel: z.string().optional(), summary: z.boolean().optional(), usedPct: z.number().nullable().optional(), remainingPct: z.number().nullable().optional(), resetsAt: z.string().nullable().optional(), runsOutAt: z.string().nullable().optional(), shortfallPct: z.number().nullable().optional(), tone: ProviderUsageToneSchema.optional(), })` |
| `PullRequestTimelineCommentItemSchema` | `packages/protocol/src/messages.ts:5913` | `z.object({ id: z.string().optional().default(""), kind: z.literal("comment"), author: z.string().optional().default("unknown"), authorUrl: z.string().nullable().optional(), avatarUrl: z.string().nullable().optional(), body: z.string().optional().default(""), createdAt: z.number().optional().default(0), url: z.string().optional().default(""), reviewId: z.string().optional(), threadId: z.string().optional(), threadIsResolved: z.boolean().optional(), location: z .object({ path: z.string(), line: z.number().optional(), startLine: z.number().optional(), threadId: z.string().optional(), isResolved: z.boolean().optional(), isOutdated: z.boolean().optional(), }) .optional(), })` |
| `PullRequestTimelineErrorSchema` | `packages/protocol/src/messages.ts:5887` | `z.preprocess((value) => { if (!value \|\| typeof value !== "object" \|\| Array.isArray(value)) { return { kind: "unknown", message: "" }; } const error = value as Record<string, unknown>; if (error.kind === "not_found" \|\| error.kind === "forbidden" \|\| error.kind === "unknown") { return error; } return { ...error, kind: "unknown" }; }, PullRequestTimelineKnownErrorSchema)` |
| `PullRequestTimelineItemSchema` | `packages/protocol/src/messages.ts:5948` | `z.preprocess((value) => { if (!value \|\| typeof value !== "object" \|\| Array.isArray(value)) { return value; } const item = value as Record<string, unknown>; if (item.kind === "review" \|\| item.kind === "comment") { return item; } return { ...item, kind: "comment" }; }, z.discriminatedUnion("kind", [ PullRequestTimelineReviewItemSchema, PullRequestTimelineCommentItemSchema, ]))` |
| `PullRequestTimelineKnownErrorSchema` | `packages/protocol/src/messages.ts:5872` | `z.discriminatedUnion("kind", [ z.object({ kind: z.literal("not_found"), message: z.string().optional().default(""), }), z.object({ kind: z.literal("forbidden"), message: z.string().optional().default(""), }), z.object({ kind: z.literal("unknown"), message: z.string().optional().default(""), }), ])` |
| `PullRequestTimelineReviewItemSchema` | `packages/protocol/src/messages.ts:5898` | `z.object({ id: z.string().optional().default(""), kind: z.literal("review"), author: z.string().optional().default("unknown"), authorUrl: z.string().nullable().optional(), avatarUrl: z.string().nullable().optional(), body: z.string().optional().default(""), createdAt: z.number().optional().default(0), url: z.string().optional().default(""), reviewState: z .enum(["approved", "changes_requested", "commented"]) .optional() .default("commented"), })` |
| `RecentProviderSessionDescriptorPayloadSchema` | `packages/protocol/src/messages.ts:923` | `z.object({ providerId: z.string(), providerLabel: z.string(), providerHandleId: z.string(), cwd: z.string(), title: z.string().nullable(), firstPromptPreview: z.string().nullable(), lastPromptPreview: z.string().nullable(), lastActivityAt: z.string(), })` |
| `ReviewAttachmentCommentSchema` | `packages/protocol/src/messages.ts:1186` | `z.object({ filePath: z.string(), side: z.enum(["old", "new"]), lineNumber: z.number().int().positive(), body: z.string(), context: z.object({ hunkHeader: z.string(), targetLine: ReviewAttachmentContextLineSchema, lines: z.array(ReviewAttachmentContextLineSchema), }), })` |
| `ReviewAttachmentContextLineSchema` | `packages/protocol/src/messages.ts:1179` | `z.object({ oldLineNumber: z.number().int().positive().nullable(), newLineNumber: z.number().int().positive().nullable(), type: z.enum(["add", "remove", "context"]), content: z.string(), })` |
| `ReviewAttachmentSchema` | `packages/protocol/src/messages.ts:1198` | `z.object({ type: z.literal("review"), mimeType: z.literal("application/paseo-review"), cwd: z.string(), mode: z.enum(["uncommitted", "base", "all"]), baseRef: z.string().nullable().optional(), comments: z.array(ReviewAttachmentCommentSchema), })` |
| `ScheduleCadenceSchema` | `packages/protocol/src/schedule/types.ts:7` | `z.discriminatedUnion("type", [ z.object({ type: z.literal("every"), everyMs: z.number().int().positive(), }), z.object({ type: z.literal("cron"), expression: z.string().trim().min(1), timezone: z.string().trim().min(1).optional(), }), ])` |
| `ScheduleCreateNewAgentConfigSchema` | `packages/protocol/src/schedule/rpc-schemas.ts:10` | `ScheduleTargetSchema.options[1].shape.config` |
| `ScheduleCreateTargetSchema` | `packages/protocol/src/schedule/rpc-schemas.ts:12` | `z.discriminatedUnion("type", [ z.object({ type: z.literal("self"), agentId: z.guid(), }), z.object({ type: z.literal("agent"), agentId: z.guid(), }), z.object({ type: z.literal("new-agent"), config: ScheduleCreateNewAgentConfigSchema, }), ])` |
| `ScheduleRunSchema` | `packages/protocol/src/schedule/types.ts:45` | `z.object({ id: z.string(), scheduledFor: z.string(), startedAt: z.string(), endedAt: z.string().nullable(), status: z.enum(["running", "succeeded", "failed"]), agentId: z.guid().nullable(), workspaceId: z.string().nullable().optional(), output: z.string().nullable(), error: z.string().nullable(), })` |
| `ScheduleStatusSchema` | `packages/protocol/src/schedule/types.ts:4` | `z.enum(["active", "paused", "completed"])` |
| `ScheduleSummarySchema` | `packages/protocol/src/schedule/types.ts:76` | `StoredScheduleSchema.omit({ runs: true, })` |
| `ScheduleTargetSchema` | `packages/protocol/src/schedule/types.ts:20` | `z.discriminatedUnion("type", [ z.object({ type: z.literal("agent"), agentId: z.guid(), }), z.object({ type: z.literal("new-agent"), config: z.object({ provider: AgentProviderSchema, cwd: z.string().trim().min(1), modeId: z.string().trim().min(1).optional(), model: z.string().trim().min(1).optional(), thinkingOptionId: z.string().trim().min(1).optional(), archiveOnFinish: z.boolean().optional(), isolation: z.enum(["local", "worktree"]).optional(), title: z.string().trim().min(1).nullable().optional(), providerOptions: z.record(z.string(), z.unknown()).optional(), featureValues: z.record(z.string(), z.unknown()).optional(), systemPrompt: z.string().optional(), mcpServers: z.record(z.string(), z.unknown()).optional(), }), }), ])` |
| `ScheduleUpdateNewAgentConfigSchema` | `packages/protocol/src/schedule/rpc-schemas.ts:80` | `z.object({ provider: z.string().trim().min(1).optional(), model: z.string().trim().min(1).nullable().optional(), modeId: z.string().trim().min(1).nullable().optional(), thinkingOptionId: z.string().trim().min(1).nullable().optional(), archiveOnFinish: z.boolean().optional(), isolation: z.enum(["local", "worktree"]).optional(), cwd: z.string().trim().min(1).optional(), })` |
| `ServerCapabilitiesFromUnknownSchema` | `packages/protocol/src/messages.ts:3645` | `z .unknown() .optional() .transform((value): z.infer<typeof ServerCapabilitiesSchema> \| undefined => { if (value === undefined) { return undefined; } const parsed = ServerCapabilitiesSchema.safeParse(value); if (!parsed.success) { return undefined; } return parsed.data; })` |
| `ServerCapabilitiesSchema` | `packages/protocol/src/messages.ts:3623` | `z .object({ voice: ServerVoiceCapabilitiesSchema.optional(), }) .passthrough()` |
| `ServerCapabilityStateSchema` | `packages/protocol/src/messages.ts:3613` | `z.object({ enabled: z.boolean(), reason: z.string(), })` |
| `ServerInfoHostnameSchema` | `packages/protocol/src/messages.ts:3629` | `z.unknown().transform((value): string \| null => { if (typeof value !== "string") { return null; } const trimmed = value.trim(); return trimmed.length > 0 ? trimmed : null; })` |
| `ServerInfoStatusPayloadSchema` | `packages/protocol/src/messages.ts:3659` | `z .object({ status: z.literal("server_info"), protocolVersion: z.number().int().optional(), serverId: z.string().trim().min(1), hostname: ServerInfoHostnameSchema.optional(), version: ServerInfoVersionSchema.optional(), sourceId: z.string().optional(), permissions: z.array(DaemonPermissionSchema).optional(), desktopManaged: z.boolean().optional(), capabilities: ServerCapabilitiesFromUnknownSchema.optional(), features: z .object({ agentRequestReceipts: z.boolean().optional(), workspaceRequestReceipts: z.boolean().optional(), creationLifecycle: z.boolean().optional(), hubAgentRpc: z.boolean().optional(), internalAgents: z.boolean().optional(), internalWorkspaces: z.boolean().optional(), backgroundWorkspaces: z.boolean().optional(), providersSnapshot: z.boolean().optional(), connectedHosts: z.boolean().optional(), hostMultiplexing: z.boolean().optional(), usageSetting: z.boolean().optional(), usageSources: z.boolean().optional(), providersSnapshotCwd: z.boolean().optional(), directorySync: z.boolean().optional(), workspaceLabels: z.boolean().optional(), workspaceSetupRun: z.boolean().optional(), workspaceTerminals: z.boolean().optional(), checkoutForgeSetAutoMerge: z.boolean().optional(), checkoutGithubSetAutoMerge: z.boolean().optional(), githubCheckDetails: z.boolean().optional(), forgeCheckDetails: z.boolean().optional(), forgeSearch: z.boolean().optional(), daemonStatusRpc: z.boolean().optional(), daemonConfigReload: z.boolean().optional(), speechSettings: z.boolean().optional(), relayConfig: z.boolean().optional(), pushTokenRevocation: z.boolean().optional(), plugins: z.boolean().optional(), pluginManagement: z.boolean().optional(), pluginLogs: z.boolean().optional(), pluginGitManagement: z.boolean().optional(), pluginSourceInstallation: z.boolean().optional(), pluginSourceUpdates: z.boolean().optional(), pluginThemes: z.boolean().optional(), pluginSettings: z.boolean().optional(), pluginTimelineItems: z.boolean().optional(), skillManagement: z.boolean().optional(), idleRuntimeCleanupSetting: z.boolean().optional(), resumeOnStartupSetting: z.boolean().optional(), "terminal-restore-modes": z.boolean().optional(), "terminal-input-mode-replay": z.boolean().optional(), "terminal-size-ownership": z.boolean().optional(), "terminal-grapheme-widths": z.boolean().optional(), rewind: z.boolean().optional(), agentTimelinePromptIndex: z.boolean().optional(), agentHistorySearch: z.boolean().optional(), checkoutRefresh: z.boolean().optional(), workspaceMultiplicity: z.boolean().optional(), projectRemove: z.boolean().optional(), projectAdd: z.boolean().optional(), worktreeRestore: z.boolean().optional(), workspaceRecovery: z.boolean().optional(), workspaceFileEditing: z.boolean().optional(), fileByteWrites: z.boolean().optional(), providerUsageList: z.boolean().optional(), agentDetach: z.boolean().optional(), agentThinkingUpdate: z.boolean().optional(), daemonDiagnostics: z.boolean().optional(), daemonSelfUpdate: z.boolean().optional(), terminalsSurviveRestart: z.boolean().optional(), agentForkContext: z.boolean().optional(), agentForkContextCursor: z.boolean().optional(), providerSubagents: z.boolean().optional(), projectedSubagentTimeline: z.boolean().optional(), wholeTurnTimeline: z.boolean().optional(), providerSubagentNesting: z.boolean().optional(), workspacePinning: z.boolean().optional(), workspaceMarkUnread: z.boolean().optional(), hubRelationship: z.boolean().optional(), relayProtectedSecurity: z.boolean().optional(), projectGithubClone: z.boolean().optional(), workspaceGithubRepositorySearch: z.boolean().optional(), projectCreateDirectory: z.boolean().optional(), projectList: z.boolean().optional(), projectSuggestions: z.boolean().optional(), commitsList: z.boolean().optional(), commitBaseClassification: z.boolean().optional(), providerRemoval: z.boolean().optional(), hopliteProvider: z.boolean().optional(), importSessionWorkspaceTarget: z.boolean().optional(), importSessionSearch: z.boolean().optional(), forgeProviders: z.boolean().optional(), selectiveAgentTimeline: z.boolean().optional(), explicitEventSubscriptions: z.boolean().optional(), ownedSubscriptions: z.boolean().optional(), binaryChunkingV1: z.boolean().optional(), workspaceSetupObservation: z.boolean().optional(), workspaceSortTimes: z.boolean().optional(), canonicalSubmittedPrompts: z.boolean().optional(), agentMessageProvenance: z.boolean().optional(), agentMessageBatching: z.boolean().optional(), agentTurnIdentity: z.boolean().optional(), stableProjectIdentity: z.boolean().optional(), workspaceScriptManagement: z.boolean().optional(), projectCustomIcon: z.boolean().optional(), fsEntryOps: z.boolean().optional(), fsEntryDuplicate: z.boolean().optional(), checkoutDiscardChanges: z.boolean().optional(), checkoutDiffAll: z.boolean().optional(), agentProfiles: z.boolean().optional(), agentConfigApply: z.boolean().optional(), }) .optional(), }) .passthrough() .transform((payload) => ({ ...payload, hostname: payload.hostname ?? null, version: payload.version ?? null, }))` |
| `ServerInfoVersionSchema` | `packages/protocol/src/messages.ts:3637` | `z.unknown().transform((value): string \| null => { if (typeof value !== "string") { return null; } const trimmed = value.trim(); return trimmed.length > 0 ? trimmed : null; })` |
| `ServerVoiceCapabilitiesSchema` | `packages/protocol/src/messages.ts:3618` | `z.object({ dictation: ServerCapabilityStateSchema, voice: ServerCapabilityStateSchema, })` |
| `SessionEventSubscriptionSchema` | `packages/protocol/src/messages.ts:3231` | `z.enum([ "project.update", "providers_snapshot_update", "agent_attention_required", "agent_permission_request", "agent_permission_resolved", "checkout_status_update", "script_status_update", "workspace_setup_progress", "agent.provider_subagents.update", "terminal_attention_required", "status.server_info", "status.daemon_config_changed", "status.plugin_catalog_changed", "status.plugin_settings_changed", "activity_log", "hub.execution.agent.update", "hub.execution.agent.stream", ])` |
| `SpeechSettingsPatchSchema` | `packages/protocol/src/speech/rpc-schemas.ts:101` | `z .object({ dictation: z .object({ enabled: z.boolean().nullable().optional(), stt: SpeechSttSettingsPatchSchema.optional(), }) .passthrough() .optional(), voiceMode: z .object({ enabled: z.boolean().nullable().optional(), stt: SpeechSttSettingsPatchSchema.optional(), tts: SpeechTtsSettingsPatchSchema.optional(), llm: z .object({ provider: z.string().nullable().optional(), model: z.string().nullable().optional(), }) .passthrough() .optional(), turnDetection: z .object({ provider: z.string().nullable().optional() }) .passthrough() .optional(), }) .passthrough() .optional(), }) .passthrough()` |
| `SpeechSettingsSchema` | `packages/protocol/src/speech/rpc-schemas.ts:32` | `z .object({ dictation: z.object({ enabled: z.boolean(), stt: SpeechSttSettingsSchema }).passthrough(), voiceMode: z .object({ enabled: z.boolean(), stt: SpeechSttSettingsSchema, tts: SpeechTtsSettingsSchema, llm: z .object({ provider: z.string().optional(), model: z.string().optional() }) .passthrough() .optional(), turnDetection: z.object({ provider: z.string().optional() }).passthrough().optional(), }) .passthrough(), }) .passthrough()` |
| `SpeechSttSettingsPatchSchema` | `packages/protocol/src/speech/rpc-schemas.ts:64` | `z.union([ z .object({ provider: z.literal("local"), ...ModelPatchFields, language: z.string().nullable().optional(), }) .passthrough(), z .object({ provider: z.literal("openai"), ...OpenAiPatchFields, ...SttPatchFields }) .passthrough(), z.object({ provider: z.string().nullable() }).passthrough(), z .object({ provider: z.undefined().optional(), ...OpenAiPatchFields, ...SttPatchFields }) .passthrough(), ])` |
| `SpeechSttSettingsSchema` | `packages/protocol/src/speech/rpc-schemas.ts:17` | `z.union([ z .object({ provider: z.literal("local"), ...ModelFields, language: z.string().optional() }) .passthrough(), z.object({ provider: z.literal("openai"), ...OpenAiReadFields, ...SttReadFields }).passthrough(), z.object({ provider: z.string(), ...ModelFields }).passthrough(), ])` |
| `SpeechTtsSettingsPatchSchema` | `packages/protocol/src/speech/rpc-schemas.ts:80` | `z.union([ z .object({ provider: z.literal("local"), ...ModelPatchFields, ...LocalTtsPatchFields }) .passthrough(), z .object({ provider: z.literal("openai"), ...OpenAiPatchFields, voice: z.string().nullable().optional(), }) .passthrough(), z.object({ provider: z.string().nullable() }).passthrough(), z .object({ provider: z.undefined().optional(), ...OpenAiPatchFields, ...LocalTtsPatchFields, voice: z.string().nullable().optional(), }) .passthrough(), ])` |
| `SpeechTtsSettingsSchema` | `packages/protocol/src/speech/rpc-schemas.ts:24` | `z.union([ z.object({ provider: z.literal("local"), ...ModelFields, ...LocalTtsFields }).passthrough(), z .object({ provider: z.literal("openai"), ...OpenAiReadFields, voice: z.string().optional() }) .passthrough(), z.object({ provider: z.string(), ...ModelFields }).passthrough(), ])` |
| `StashEntrySchema` | `packages/protocol/src/messages.ts:6008` | `z.object({ index: z.number().int().min(0), message: z.string(), branch: z.string().nullable(), isPaseo: z.boolean(), })` |
| `StoredScheduleSchema` | `packages/protocol/src/schedule/types.ts:58` | `z.object({ id: z.string(), name: z.string().nullable(), prompt: z.string().min(1), cadence: ScheduleCadenceSchema, target: ScheduleTargetSchema, status: ScheduleStatusSchema, createdAt: z.string(), updatedAt: z.string(), nextRunAt: z.string().nullable(), lastRunAt: z.string().nullable(), pausedAt: z.string().nullable(), expiresAt: z.string().nullable(), maxRuns: z.number().int().positive().nullable(), runs: z.array(ScheduleRunSchema), })` |
| `TerminalActivitySchema` | `packages/protocol/src/terminal-activity.ts:9` | `z.object({ state: z.enum(TERMINAL_ACTIVITY_STATES).catch("idle"), attentionReason: z.enum(TERMINAL_ACTIVITY_ATTENTION_REASONS).nullable().optional().catch(null), changedAt: z.number(), })` |
| `TerminalCellSchema` | `packages/protocol/src/messages.ts:6562` | `z.object({ char: z.string(), fg: z.number().optional(), bg: z.number().optional(), fgMode: z.number().optional(), bgMode: z.number().optional(), bold: z.boolean().optional(), italic: z.boolean().optional(), underline: z.boolean().optional(), dim: z.boolean().optional(), inverse: z.boolean().optional(), strikethrough: z.boolean().optional(), })` |
| `TerminalClientMessageSchema` | `packages/protocol/src/messages.ts:3119` | `z.discriminatedUnion("type", [ z.object({ type: z.literal("input"), data: z.string() }), z.object({ type: z.literal("resize"), rows: z.number(), cols: z.number(), intent: z.enum(["claim", "update"]).optional(), }), z.object({ type: z.literal("mouse"), row: z.number(), col: z.number(), button: z.number(), action: z.enum(["down", "up", "move"]), }), ])` |
| `TerminalCursorSchema` | `packages/protocol/src/messages.ts:6578` | `z.object({ row: z.number(), col: z.number(), hidden: z.boolean().optional(), style: TerminalCursorStyleSchema.optional(), blink: z.boolean().optional(), })` |
| `TerminalCursorStyleSchema` | `packages/protocol/src/messages.ts:6576` | `z.enum(["block", "underline", "bar"])` |
| `TerminalInfoSchema` | `packages/protocol/src/messages.ts:6553` | `z.object({ id: z.string(), name: z.string(), cwd: z.string(), workspaceId: z.string().optional(), title: z.string().optional(), activity: TerminalActivitySchema.nullable().optional(), })` |
| `TerminalProfileSchema` | `packages/protocol/src/terminal-profile.ts:3` | `z .object({ id: z.string(), name: z.string(), command: z.string(), args: z.array(z.string()).optional(), icon: z.string().optional(), }) .passthrough()` |
| `TerminalStateSchema` | `packages/protocol/src/messages.ts:6586` | `z.object({ rows: z.number(), cols: z.number(), grid: z.array(z.array(TerminalCellSchema)), scrollback: z.array(z.array(TerminalCellSchema)), cursor: TerminalCursorSchema, title: z.string().optional(), gridWrapped: z.array(z.boolean()).optional(), scrollbackWrapped: z.array(z.boolean()).optional(), })` |
| `TextAttachmentSchema` | `packages/protocol/src/messages.ts:1165` | `z .object({ type: z.literal("text"), mimeType: z.literal("text/plain"), contextKind: z.string().optional(), title: z.string().nullable().optional(), text: z.string(), externalResource: ExternalResourceAttachmentMetadataSchema.optional(), }) .transform(({ contextKind, ...attachment }) => ({ ...attachment, ...(contextKind === "chat_history" ? { contextKind } : {}), }))` |
| `ToolCallBasePayloadSchema` | `packages/protocol/src/messages.ts:678` | `z.object({ agentMessage: AgentMessageSchema.optional(), type: z.literal("tool_call"), callId: z.string(), name: z.string(), detail: ToolCallDetailPayloadSchema, metadata: z.record(z.string(), z.unknown()).optional(), })` |
| `ToolCallCanceledPayloadSchema` | `packages/protocol/src/messages.ts:702` | `ToolCallBasePayloadSchema.extend({ status: z.literal("canceled"), error: z.null(), })` |
| `ToolCallCompletedPayloadSchema` | `packages/protocol/src/messages.ts:692` | `ToolCallBasePayloadSchema.extend({ status: z.literal("completed"), error: z.null(), })` |
| `ToolCallDetailPayloadSchema` | `packages/protocol/src/messages.ts:577` | `z.discriminatedUnion("type", [ WorktreeSetupDetailPayloadSchema, z.object({ type: z.literal("shell"), command: z.string(), cwd: z.string().optional(), output: z.string().optional(), exitCode: z.number().nullable().optional(), }), z.object({ type: z.literal("read"), filePath: z.string(), content: z.string().optional(), offset: z.number().optional(), limit: z.number().optional(), }), z.object({ type: z.literal("edit"), filePath: z.string(), oldString: z.string().optional(), newString: z.string().optional(), unifiedDiff: z.string().optional(), }), z.object({ type: z.literal("write"), filePath: z.string(), content: z.string().optional(), }), z.object({ type: z.literal("search"), query: z.string(), toolName: z.enum(["search", "grep", "glob", "web_search"]).optional(), content: z.string().optional(), filePaths: z.array(z.string()).optional(), webResults: z .array(z.object({ title: z.string(), url: z.string(), })) .optional(), annotations: z.array(z.string()).optional(), numFiles: z.number().optional(), numMatches: z.number().optional(), durationMs: z.number().optional(), durationSeconds: z.number().optional(), truncated: z.boolean().optional(), mode: z.enum(["content", "files_with_matches", "count"]).optional(), }), z.object({ type: z.literal("fetch"), url: z.string(), prompt: z.string().optional(), result: z.string().optional(), code: z.number().optional(), codeText: z.string().optional(), bytes: z.number().optional(), durationMs: z.number().optional(), }), z.object({ type: z.literal("sub_agent"), subAgentType: z.string().optional(), description: z.string().optional(), childSessionId: z.string().optional(), log: z.string().optional(), actions: z .array(z.object({ index: z.number().int().positive(), toolName: z.string(), summary: z.string().optional(), })) .optional(), }), z.object({ type: z.literal("plain_text"), label: z.string().optional(), text: z.string().optional(), icon: z.enum(TOOL_CALL_ICON_NAMES).optional(), }), z.object({ type: z.literal("plan"), text: z.string(), }), z.object({ type: z.literal("unknown"), input: UnknownValueSchema, output: UnknownValueSchema, }), ])` |
| `ToolCallFailedPayloadSchema` | `packages/protocol/src/messages.ts:697` | `ToolCallBasePayloadSchema.extend({ status: z.literal("failed"), error: NonNullUnknownSchema, })` |
| `ToolCallRunningPayloadSchema` | `packages/protocol/src/messages.ts:687` | `ToolCallBasePayloadSchema.extend({ status: z.literal("running"), error: z.null(), })` |
| `ToolCallTimelineItemPayloadSchema` | `packages/protocol/src/messages.ts:707` | `z.discriminatedUnion("status", [ ToolCallRunningPayloadSchema, ToolCallCompletedPayloadSchema, ToolCallFailedPayloadSchema, ToolCallCanceledPayloadSchema, ])` |
| `ToolPolicySchema` | `packages/protocol/src/messages.ts:481` | `z .object({ preapproved: z.array(McpToolRefSchema), }) .strict()` |
| `UnknownValueSchema` | `packages/protocol/src/messages.ts:541` | `z.union([ z.null(), z.boolean(), z.number(), z.string(), z.array(z.unknown()), z.object({}).passthrough(), ])` |
| `UploadedFileAttachmentSchema` | `packages/protocol/src/messages.ts:1207` | `z.object({ type: z.literal("uploaded_file"), id: z.string(), fileName: z.string(), mimeType: z.string(), size: z.number().int().nonnegative(), path: z.string(), })` |
| `UsageProblemSchema` | `packages/protocol/src/messages.ts:6473` | `z.discriminatedUnion("kind", [ z.object({ kind: z.literal("expired"), expiresAt: z.iso.datetime(), refreshedBy: z.string().optional(), }), z.object({ kind: z.literal("rejected"), status: z.number().int(), refreshedBy: z.string().optional(), }), z.object({ kind: z.literal("no_quota"), detail: z.string() }), ])` |
| `UsageReportEntrySchema` | `packages/protocol/src/messages.ts:6498` | `z.object({ id: z.string(), account: z.object({ label: z.string().optional() }), fetchedAt: z.string(), sourceId: z.string(), sourceLabel: z.string(), icon: z.string().optional(), report: UsageReportSchema, successfulAt: z.string().optional(), lastSuccessful: z .object({ fetchedAt: z.string(), report: AvailableUsageReportSchema }) .optional(), loginErrors: z .array(z.object({ harness: z.string(), report: z.discriminatedUnion("status", [ z.object({ status: z.literal("unavailable"), problem: UsageProblemSchema }), z.object({ status: z.literal("error"), error: z.string() }), ]), })) .optional(), })` |
| `UsageReportSchema` | `packages/protocol/src/messages.ts:6493` | `z.discriminatedUnion("status", [ AvailableUsageReportSchema, z.object({ status: z.literal("unavailable"), problem: UsageProblemSchema }), z.object({ status: z.literal("error"), error: z.string() }), ])` |
| `WorkspaceDescriptorPayloadSchema` | `packages/protocol/src/messages.ts:4151` | `z .object({ id: z.string(), projectId: z.string(), projectDisplayName: z.string(), projectCustomName: z.string().nullable().optional(), projectCustomIconRevision: z.string().nullable().optional(), projectRootPath: z.string(), workspaceDirectory: z.string().optional(), setupStatus: z.enum(["running", "completed", "failed", "blocked"]).nullable().optional(), worktreeSlug: z.string().optional(), projectKind: z.enum(["git", "non_git", "directory"]), workspaceKind: z.enum(["directory", "local_checkout", "checkout", "worktree"]), name: z.string(), title: z.string().nullable().optional(), pinnedAt: z.string().nullable().optional(), labels: z.array(z.string()).optional(), internal: z.boolean().optional(), background: z.boolean().optional(), archivingAt: z.string().nullable().optional().default(null), status: WorkspaceStateBucketSchema, statusEnteredAt: z .string() .nullish() .transform((value) => value ?? null), activityAt: z.string().nullable(), createdAt: z.string().optional(), lastUserMessageAt: z.string().nullable().optional(), diffStat: z .object({ additions: z.number(), deletions: z.number(), }) .nullable() .optional(), scripts: z.array(WorkspaceScriptPayloadSchema).default([]), gitRuntime: WorkspaceGitRuntimePayloadSchema, githubRuntime: WorkspaceGitHubRuntimePayloadSchema, forge: z.string().optional(), project: ProjectPlacementPayloadSchema.optional(), syncSeq: z.number().int().positive().optional(), }) .transform((workspace) => ({ ...workspace, workspaceDirectory: workspace.workspaceDirectory ?? workspace.projectRootPath, }))` |
| `WorkspaceGitHubRuntimePayloadSchema` | `packages/protocol/src/messages.ts:4105` | `z .object({ featuresEnabled: z.boolean().optional(), pullRequest: z .object({ number: z.number().optional(), url: z.string(), title: z.string(), state: z.string(), baseRefName: z.string(), headRefName: z.string(), isMerged: z.boolean(), isDraft: z.boolean().optional(), mergeable: z.enum(["MERGEABLE", "CONFLICTING", "UNKNOWN"]).catch("UNKNOWN").optional(), checks: z .array(z.object({ name: z.string(), status: z.enum(["success", "failure", "pending", "skipped", "cancelled"]), url: z.string().nullable(), workflow: z.string().optional(), duration: z.string().optional(), traits: z.array(z.string()).optional(), })) .optional(), checksStatus: z.enum(["none", "pending", "success", "failure"]).optional(), reviewDecision: z.enum(["approved", "changes_requested", "pending"]).nullable().optional(), repoOwner: z.string().optional(), repoName: z.string().optional(), github: z.unknown().optional(), }) .nullable() .optional(), error: z .object({ message: z.string(), }) .nullable() .optional(), refreshedAt: z.string().nullable().optional(), }) .optional() .nullable()` |
| `WorkspaceGitRuntimePayloadSchema` | `packages/protocol/src/messages.ts:4086` | `z .object({ currentBranch: z.string().nullable().optional(), remoteUrl: z.string().nullable().optional(), isPaseoOwnedWorktree: z.boolean().optional(), isDirty: z.boolean().nullable().optional(), aheadBehind: z .object({ ahead: z.number(), behind: z.number(), }) .nullable() .optional(), aheadOfOrigin: z.number().nullable().optional(), behindOfOrigin: z.number().nullable().optional(), }) .optional() .nullable()` |
| `WorkspaceInitialAgentSchema` | `packages/protocol/src/messages.ts:1811` | `AgentCreateRequestSchema.omit({ type: true, requestId: true, idempotencyKey: true, subscribe: true, workspaceId: true, worktree: true, worktreeName: true, git: true, })` |
| `WorkspaceLabelColorSchema` | `packages/protocol/src/messages.ts:1040` | `z.enum(WORKSPACE_LABEL_COLORS)` |
| `WorkspaceLabelDefinitionSchema` | `packages/protocol/src/messages.ts:1041` | `z.object({ name: z.string(), color: WorkspaceLabelColorSchema, })` |
| `WorkspaceLabelSyncCursorSchema` | `packages/protocol/src/messages.ts:1045` | `z.object({ generation: z.string(), afterSeq: z.number().int().nonnegative(), })` |
| `WorkspaceLabelSyncMetadataSchema` | `packages/protocol/src/messages.ts:4420` | `z.object({ mode: z.enum(["snapshot", "changes"]), generation: z.string(), headSeq: z.number().int().nonnegative(), removals: z.array(z.object({ name: z.string(), seq: z.number().int().positive() })), })` |
| `WorkspacePinSetResponsePayloadSchema` | `packages/protocol/src/messages.ts:2175` | `z.object({ requestId: z.string(), workspaceId: z.string(), accepted: z.boolean(), pinnedAt: z.string().nullable(), error: z.string().nullable(), })` |
| `WorkspaceProjectDescriptorPayloadSchema` | `packages/protocol/src/messages.ts:4353` | `z.object({ projectId: z.string(), projectKey: z.string().optional(), projectDisplayName: z.string(), projectCustomName: z.string().nullable().optional(), projectCustomIconRevision: z.string().nullable().optional(), projectIconRevision: z.string().optional(), projectRootPath: z.string(), projectKind: z.enum(["git", "non_git", "directory"]), syncSeq: z.number().int().positive().optional(), })` |
| `WorkspaceRecoveryStateSchema` | `packages/protocol/src/messages.ts:2188` | `z.discriminatedUnion("kind", [ z.object({ kind: z.literal("recoverable"), workspaceId: z.string(), workspaceName: z.string(), action: z.string(), branch: z.string().nullable(), }), z.object({ kind: z.literal("unavailable"), workspaceId: z.string(), reason: z.string(), message: z.string(), }), ])` |
| `WorkspaceScriptHealthSchema` | `packages/protocol/src/messages.ts:4070` | `z.enum(["healthy", "unhealthy"])` |
| `WorkspaceScriptLifecycleSchema` | `packages/protocol/src/messages.ts:4069` | `z.enum(["running", "stopped"])` |
| `WorkspaceScriptOperationPayloadSchema` | `packages/protocol/src/messages.ts:4704` | `z.object({ requestId: z.string(), workspaceId: z.string(), scriptName: z.string().optional(), script: WorkspaceScriptPayloadSchema.nullable().optional(), scripts: z.array(WorkspaceScriptPayloadSchema).optional(), error: z.string().nullable(), })` |
| `WorkspaceScriptPayloadSchema` | `packages/protocol/src/messages.ts:4072` | `z.object({ scriptName: z.string(), type: z.enum(["script", "service"]).optional().default("service"), hostname: z.string(), port: z.number().int().positive().nullable(), localProxyUrl: z.string().nullable().optional(), publicProxyUrl: z.string().nullable().optional(), proxyUrl: z.string().nullable().optional().default(null), lifecycle: WorkspaceScriptLifecycleSchema, health: WorkspaceScriptHealthSchema.nullable(), exitCode: z.number().nullable().optional().default(null), terminalId: z.string().nullable().optional().default(null), })` |
| `WorkspaceSetupSnapshotSchema` | `packages/protocol/src/messages.ts:4561` | `z.object({ status: z.enum(["running", "completed", "failed", "blocked"]), detail: WorktreeSetupDetailPayloadSchema, error: z.string().nullable(), blockedSource: z .object({ kind: z.literal("change_request"), forge: z.string(), number: z.number().int().positive(), headRepository: z.string(), }) .optional(), })` |
| `WorkspaceStateBucketSchema` | `packages/protocol/src/messages.ts:1320` | `z.enum([ "needs_input", "failed", "running", "attention", "done", ])` |
| `WorkspaceTitleSetResponsePayloadSchema` | `packages/protocol/src/messages.ts:2162` | `z.object({ requestId: z.string(), workspaceId: z.string(), accepted: z.boolean(), title: z.string().nullable(), error: z.string().nullable(), })` |
| `WorktreeSetupCommandSnapshotSchema` | `packages/protocol/src/messages.ts:558` | `z.object({ index: z.number().int().positive(), command: z.string(), cwd: z.string(), log: z.string().optional().default(""), status: z.enum(["running", "completed", "failed"]), exitCode: z.number().nullable(), durationMs: z.number().nonnegative().optional(), })` |
| `WorktreeSetupDetailPayloadSchema` | `packages/protocol/src/messages.ts:568` | `z.object({ type: z.literal("worktree_setup"), worktreePath: z.string(), branchName: z.string(), log: z.string(), commands: z.array(WorktreeSetupCommandSnapshotSchema), truncated: z.boolean().optional(), })` |

## Typed client layers and lifetime

`packages/client/package.json` exports the public SDK at `.`, the Node connector at `./node`, and lower-level internal subpaths including DaemonClient. CLI uses both layers. `createPaseoClient(config)` (`src/index.ts:518`) constructs a cli-typed DaemonClient with a generated client ID; `createPaseoApi(driver,{signal?})` (`index.ts:570`) wraps an existing connection without owning its close operation. `dispose()` aborts API lifetime and releases owned subscriptions; `close()` disposes then closes transport. Merely obtaining a ref is local and does not fetch a record.

High-level handles cache only snapshots they observe. Missing snapshot getters return null; ref.current() distinguishes absence. Event subscriptions for agents/workspaces are local listeners and need a directory subscription to cause the server to stream data. Project/provider subscriptions establish event demand themselves. All high-level methods below route through the listed low-level method; its exact messages and signatures follow.

## Typed client public SDK methods

| Surface | Method | Low-level client | Protocol / semantics | Implementation reference |
|---|---|---|---|---|
| `PaseoClient` | `connect` | `connect` | `hello handshake` | `packages/client/src/index.ts:512` |
| `PaseoClient` | `close` | `close` | `dispose owned handles then close transport` | `packages/client/src/index.ts:513` |
| `PaseoClient` | `ensureConnected` | `ensureConnected` | `initiate/verify connection` | `packages/client/src/index.ts:514` |
| `PaseoClient` | `getConnectionState` | `getConnectionState` | `local idle/connecting/connected/disconnected/disposed state` | `packages/client/src/index.ts:515` |
| `PaseoApi` | `dispose` | `—` | `release SDK-owned observations, no daemon shutdown` | `packages/client/src/index.ts:501` |
| `PaseoApi` | `observeEvents` | `observeEvents` | `session.events.set_subscription and subscription.release` | `packages/client/src/index.ts:661` |
| `hosts` | `list` | `listHosts` | `hosts.list.request/response` | `packages/client/src/hosts/index.ts:136` |
| `hosts` | `observe` | `observeHosts` | `initial directory + hosts.changed; owned lifetime` | `packages/client/src/hosts/index.ts:140` |
| `hosts` | `add` | `addHost` | `hosts.add.request/response` | `packages/client/src/hosts/index.ts:141` |
| `hosts` | `update` | `updateHost` | `hosts.update.request/response` | `packages/client/src/hosts/index.ts:145` |
| `hosts` | `remove` | `removeHost` | `hosts.remove.request/response` | `packages/client/src/hosts/index.ts:149` |
| `hosts` | `client` | `openHostRoute/openHostLink` | `creates routed PaseoApi for serverId; inner hello; disposal closes route` | `packages/client/src/hosts/index.ts:153` |
| `projects` | `list` | `listProjects` | `project.list.request/response` | `packages/client/src/index.ts:728` |
| `projects` | `subscribe` | `observeEvents` | `project.update event demand` | `packages/client/src/index.ts:730` |
| `workspaces` | `list` | `fetchWorkspaces / observeWorkspaces` | `fetch_workspaces; subscribe:{} owns snapshot + workspace_update` | `packages/client/src/index.ts:677` |
| `workspaces` | `ref` | `—` | `local workspace handle` | `packages/client/src/index.ts:738` |
| `workspaces` | `open` | `openProject` | `open_project request; resolves/creates active workspace for cwd` | `packages/client/src/index.ts:1052` |
| `workspaces` | `create` | `createWorkspace` | `workspace creation lifecycle; optional embedded agent` | `packages/client/src/index.ts:741` |
| `workspaces` | `archive` | `archiveWorkspace` | `archive_workspace request/response` | `packages/client/src/index.ts:763` |
| `workspaces` | `subscribe` | `—` | `local workspace_update listener` | `packages/client/src/index.ts:765` |
| `workspace handle` | `current` | `—` | `cached snapshot` | `packages/client/src/index.ts:872` |
| `workspace handle` | `refresh` | `fetchWorkspaces` | `pages of 200; include background when supported; searches exact workspace ID` | `packages/client/src/index.ts:821` |
| `workspace handle` | `setTitle` | `setWorkspaceTitle` | `workspace.title.set request/response` | `packages/client/src/index.ts:874` |
| `workspace handle` | `archive` | `archiveWorkspace` | `updates cached archivingAt` | `packages/client/src/index.ts:875` |
| `workspace handle` | `subscribe` | `—` | `local listener filtered by workspace ID; applies upserts` | `packages/client/src/index.ts:883` |
| `workspace handle` | `agents.create` | `createAgent` | `uses/refetches workspace directory and workspaceId` | `packages/client/src/index.ts:856` |
| `workspace handle` | `terminals.create` | `createTerminal` | `binds workspaceId` | `packages/client/src/index.ts:868` |
| `workspace handle` | `terminals.list` | `listTerminals` | `binds workspaceId` | `packages/client/src/index.ts:869` |
| `agents` | `list` | `fetchAgents / observeAgents` | `fetch_agents; subscribe:{} owns initial snapshot + agent_update` | `packages/client/src/index.ts:701` |
| `agents` | `ref` | `—` | `local agent handle` | `packages/client/src/index.ts:769` |
| `agents` | `create` | `createAgent` | `provider/model split; parent encoded in callerAgentId/address labels` | `packages/client/src/index.ts:608` |
| `agents` | `subscribe` | `—` | `local agent_update listener` | `packages/client/src/index.ts:771` |
| `agent handle` | `current` | `—` | `cached snapshot` | `packages/client/src/index.ts:993` |
| `agent handle` | `refresh` | `fetchAgent` | `fetch_agent request/response` | `packages/client/src/index.ts:994` |
| `agent handle` | `reload` | `refreshAgent + fetchAgent` | `reopens same provider session; interrupts active turn; refreshes snapshot` | `packages/client/src/index.ts:999` |
| `agent handle` | `send` | `sendAgentMessage` | `dispatch acknowledgement; default active-turn semantics in request` | `packages/client/src/index.ts:1005` |
| `agent handle` | `respondToPermission` | `respondToPermission` | `agent_permission_response; fire-and-forget` | `packages/client/src/index.ts:1008` |
| `agent handle` | `run` | `sendAgentMessage + waitForFinish` | `default timeout 10 min; final snapshot updates cache` | `packages/client/src/index.ts:1011` |
| `agent handle` | `waitForFinish` | `waitForFinish` | `default timeout 10 min (different from low-level 60 s)` | `packages/client/src/index.ts:1026` |
| `agent handle` | `commands` | `listCommands` | `live provider slash commands/skills; payload.error may be non-null` | `packages/client/src/index.ts:1036` |
| `agent handle` | `archive` | `archiveAgent` | `updates cached archivedAt` | `packages/client/src/index.ts:1037` |
| `agent handle` | `detach` | `detachAgent` | `ownership change without move or stop` | `packages/client/src/index.ts:1044` |
| `agent handle` | `subscribe` | `—` | `local listener filtered by agent ID; applies upserts` | `packages/client/src/index.ts:1047` |
| `agent.timeline` | `append` | `appendAgentTimelineItem` | `agent.timeline.append request/response; seq and epoch` | `packages/client/src/index.ts:917` |
| `agent.timeline` | `refetch` | `fetchAgentTimeline` | `paged/projection-aware history; refreshes parent agent snapshot` | `packages/client/src/index.ts:918` |
| `agent.timeline` | `subscribe` | `subscribeAgentTimeline` | `live only; ready/release; replacement/restored/error local lifecycle` | `packages/client/src/index.ts:925` |
| `providers` | `listModels` | `listProviderModels` | `list_provider_models request/response` | `packages/client/src/index.ts:774` |
| `providers` | `listModes` | `listProviderModes` | `list_provider_modes request/response` | `packages/client/src/index.ts:775` |
| `providers` | `listFeatures` | `listProviderFeatures` | `splits provider/model in draft` | `packages/client/src/index.ts:776` |
| `providers` | `listAvailable` | `listAvailableProviders` | `availability snapshot` | `packages/client/src/index.ts:780` |
| `providers` | `snapshot` | `getProvidersSnapshot` | `hash/compact/reference-aware catalog` | `packages/client/src/index.ts:781` |
| `providers` | `waitForReady` | `getProvidersSnapshot + observeEvents` | `waits for lazy discovery completion` | `packages/client/src/index.ts:782` |
| `providers` | `refresh` | `refreshProvidersSnapshot` | `explicit rediscovery` | `packages/client/src/index.ts:790` |
| `providers` | `diagnostic` | `getProviderDiagnostic` | `provider diagnosis` | `packages/client/src/index.ts:791` |
| `providers` | `listUsage` | `listProviderUsage` | `requires providerUsageList` | `packages/client/src/index.ts:792` |
| `providers` | `subscribe` | `observeEvents` | `providers_snapshot_update` | `packages/client/src/index.ts:793` |
| `config` | `get` | `getDaemonConfig` | `typed raw mutable daemon config` | `packages/client/src/index.ts:801` |
| `config` | `patch` | `patchDaemonConfig` | `daemon validates and persists supported patch fields` | `packages/client/src/index.ts:802` |
| `terminals` | `create` | `createTerminal` | `requires workspaceId; resolves default cwd from workspace` | `packages/client/src/terminals/index.ts:151` |
| `terminals` | `list` | `listTerminals` | `workspaceId overrides cwd ownership filter` | `packages/client/src/terminals/index.ts:103` |
| `terminals` | `ref` | `—` | `local terminal handle` | `packages/client/src/terminals/index.ts:113` |
| `terminal handle` | `current` | `—` | `cached terminal snapshot` | `packages/client/src/terminals/index.ts:123` |
| `terminal handle` | `refresh` | `listTerminals` | `find terminal by ID; set null if missing` | `packages/client/src/terminals/index.ts:124` |
| `terminal handle` | `write` | `sendTerminalInput` | `literal UTF-16 input length; no execution wait` | `packages/client/src/terminals/index.ts:116` |
| `terminal handle` | `sendKeys` | `sendTerminalInput` | `expand Enter/Tab/Escape/Space/BSpace/control and arrow tokens` | `packages/client/src/terminals/index.ts:130` |
| `terminal handle` | `capture` | `captureTerminal` | `capture request/response` | `packages/client/src/terminals/index.ts:131` |
| `terminal handle` | `kill` | `killTerminal` | `throws if success=false; clears cached snapshot` | `packages/client/src/terminals/index.ts:135` |

Workspace/agent handle getters: workspace `id, projectId, directory, name, status`; agent `id,address,workspaceId,cwd,status,capabilities,availableModes,pendingPermissions,activeTurn,lastUsage,lastError,features,runtimeInfo,archivedAt`; terminal `id`. `address` requires a connected serverId and returns `{serverId,agentId}`. These getters do not send RPCs.

## Typed low-level client: Connection helpers and local observation

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async connect(): Promise<void>` | `outer hello`<br>`status (server_info)`<br>`outer hello.rejected` | `Delegates to attemptConnect` | `packages/client/src/daemon-client.ts:1492` |
| `async close(): Promise<void>` | — | `Local state/listener/capability helper; Delegates to rejectConnect, resetConnectTimeout, clearWaiters, rejectPendingSendQueue, rejectPingProbe, updateConnectionState` | `packages/client/src/daemon-client.ts:1742` |
| `ensureConnected(options?: { verify?: boolean }): void` | `outer hello / ping (as needed)` | `Delegates to verifyConnection, attemptConnect, connect` | `packages/client/src/daemon-client.ts:1774` |
| `requireConnectedHosts(): void` | — | `Local state/listener/capability helper; No secondary domain call` | `packages/client/src/daemon-client.ts:1817` |
| `subscribeHostFrames(handler: (frame: HostRouteFrame) => void): () => void` | `binary 0x30 (local listener)` | `No secondary domain call` | `packages/client/src/daemon-client.ts:1917` |
| `getConnectionState(): ConnectionState` | — | `Local state/listener/capability helper; No secondary domain call` | `packages/client/src/daemon-client.ts:1930` |
| `subscribeConnectionStatus(listener: (status: ConnectionState) => void): () => void` | — | `Local state/listener/capability helper; No secondary domain call` | `packages/client/src/daemon-client.ts:1934` |
| `get isConnected(): boolean` | — | `Local state/listener/capability helper; No secondary domain call` | `packages/client/src/daemon-client.ts:1942` |
| `get isConnecting(): boolean` | — | `Local state/listener/capability helper; No secondary domain call` | `packages/client/src/daemon-client.ts:1946` |
| `get relaySecurityFloor(): RelaySecurityMode \| undefined` | — | `Local state/listener/capability helper; No secondary domain call` | `packages/client/src/daemon-client.ts:1951` |
| `get lastError(): string \| null` | — | `Local state/listener/capability helper; No secondary domain call` | `packages/client/src/daemon-client.ts:1955` |
| `get authFailureReason(): DaemonAuthFailureReason \| null` | — | `Local state/listener/capability helper; No secondary domain call` | `packages/client/src/daemon-client.ts:1959` |
| `getLastLivenessRttMs(): number \| null` | — | `Local state/listener/capability helper; No secondary domain call` | `packages/client/src/daemon-client.ts:1963` |
| `get supportsDrain(): boolean` | — | `Local state/listener/capability helper; No secondary domain call` | `packages/client/src/daemon-client.ts:1972` |
| `subscribe(handler: DaemonEventHandler): () => void` | — | `Local state/listener/capability helper; No secondary domain call` | `packages/client/src/daemon-client.ts:1976` |
| `subscribeRawMessages(handler: (message: SessionOutboundMessage) => void): () => void` | — | `Local state/listener/capability helper; No secondary domain call` | `packages/client/src/daemon-client.ts:1983` |
| `on( arg1: SessionOutboundMessage["type"] \| DaemonEventHandler, arg2?: (message: SessionOutboundMessage) => void, ): () => void` | — | `Local state/listener/capability helper; Delegates to subscribe` | `packages/client/src/daemon-client.ts:1995` |
| `measureLatency(params?: { timeoutMs?: number }): Promise<number>` | `outer ping`<br>`outer pong` | `Delegates to sendPingAwaitRtt` | `packages/client/src/daemon-client.ts:2542` |
| `supportsBackgroundWorkspaces(): boolean` | — | `Local state/listener/capability helper; No secondary domain call` | `packages/client/src/daemon-client.ts:3280` |
| `onTerminalStreamEvent(handler: (event: TerminalStreamEvent) => void): () => void` | `binary terminal frames (local listener)` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6738` |
| `async waitForTerminalStreamEvent( predicate: (event: TerminalStreamEvent) => boolean, timeout = 5000, ): Promise<TerminalStreamEvent>` | `binary terminal frames (local waiter)` | `Delegates to onTerminalStreamEvent` | `packages/client/src/daemon-client.ts:6742` |
| `getLastServerInfoMessage(): ServerInfoStatusPayload \| null` | — | `Local state/listener/capability helper; No secondary domain call` | `packages/client/src/daemon-client.ts:6771` |
| `setReconnectEnabled(enabled: boolean): void` | — | `Local state/listener/capability helper; No secondary domain call` | `packages/client/src/daemon-client.ts:7180` |

## Typed low-level client: Connected hosts and routing

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async listHosts(): Promise<HostDirectory>` | `hosts.list.request`<br>`hosts.list.response` | `Delegates to requireConnectedHosts` | `packages/client/src/daemon-client.ts:1828` |
| `observeHosts(): OwnedSubscription<HostDirectory>` | `hosts.list.request`<br>`hosts.list.response` | `Delegates to requireConnectedHosts` | `packages/client/src/daemon-client.ts:1837` |
| `async addHost(input: HostAdd)` | `hosts.add.request`<br>`hosts.add.response` | `Delegates to requireConnectedHosts` | `packages/client/src/daemon-client.ts:1850` |
| `async updateHost(registrationId: string, patch: HostUpdate)` | `hosts.update.request`<br>`hosts.update.response` | `Delegates to requireConnectedHosts` | `packages/client/src/daemon-client.ts:1859` |
| `async removeHost(registrationId: string): Promise<void>` | `hosts.remove.request`<br>`hosts.remove.response` | `Delegates to requireConnectedHosts` | `packages/client/src/daemon-client.ts:1868` |
| `async openHostRoute(serverId: string, scopeId: string): Promise<string>` | `hosts.route.open.request`<br>`hosts.route.open.response` | `Delegates to requireConnectedHosts` | `packages/client/src/daemon-client.ts:1875` |
| `async openHostLink(channelId: string): Promise<string>` | `hosts.link.open.request`<br>`hosts.link.open.response` | `Delegates to requireConnectedHosts, getLastServerInfoMessage` | `packages/client/src/daemon-client.ts:1890` |
| `sendHostFrame(routeId: string, data: string \| Uint8Array): Promise<void>` | `hosts.route.frame.request` | `Delegates to unavailableError` | `packages/client/src/daemon-client.ts:1903` |
| `cancelHostRoute(scopeId: string): void` | `hosts.route.close.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:1921` |
| `closeHostRoute(routeId: string): void` | `hosts.route.close.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:1925` |

## Typed low-level client: Agent timelines and search

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `onAgentAttentionRequired( handler: (notification: AgentAttentionRequiredNotification) => void, ): () => void` | `agent_stream`<br>`agent_attention_required` | `Delegates to on` | `packages/client/src/daemon-client.ts:2023` |
| `async fetchAgentTimeline( agentId: string, options: FetchAgentTimelineOptions = {}, ): Promise<FetchAgentTimelinePayload>` | `fetch_agent_timeline_request`<br>`fetch_agent_timeline_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3664` |
| `async appendAgentTimelineItem( agentId: string, item: Omit<import("@getpaseo/protocol/agent-types").PluginTimelineItem, "pluginId">, ): Promise<{ seq: number; epoch: string }>` | `agent.timeline.append.request`<br>`agent.timeline.append.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3709` |
| `async searchAgentTimeline({ agentId, query, cursor, }: AgentTimelineSearchOptions): Promise<AgentTimelineSearchPayload>` | `agent.timeline.search.request`<br>`agent.timeline.search.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3722` |
| `async listAgentTimelinePrompts( agentId: string, options: { requestId?: string; timeout?: number } = {}, ): Promise<AgentTimelinePromptIndexPayload>` | `agent.timeline.list_prompts.request`<br>`agent.timeline.list_prompts.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3737` |
| `observeTimeline( agentIds: string[], options?: { signal?: AbortSignal }, ): OwnedSubscription<CorrelatedResponsePayload<"agent.timeline.set_subscription.response">>` | `agent.timeline.set_subscription.response`<br>`agent.timeline.set_subscription.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3827` |
| `subscribeAgentTimeline( agentId: string, handler: (message: TimelineMessage) => void, ): TimelineSubscription` | `agent.timeline.set_subscription.response`<br>`agent.timeline.set_subscription.request` | `Delegates to observeTimeline` | `packages/client/src/daemon-client.ts:3838` |
| `async buildAgentForkContext( agentId: string, options: AgentForkContextOptions = {}, ): Promise<AgentForkContextPayload>` | `agent.fork_context.request`<br>`agent.fork_context.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3847` |
| `async pullRequestTimeline( input: { cwd: string; prNumber: number; repoOwner: string; repoName: string }, requestId?: string, ): Promise<PullRequestTimelinePayload>` | `pull_request_timeline_request`<br>`pull_request_timeline_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4873` |

## Typed low-level client: Presence, attention and notifications

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async clearAgentAttention(agentId: string \| string[]): Promise<void>` | `clear_agent_attention`<br>`clear_agent_attention_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2410` |
| `sendHeartbeat(params: { deviceType: "web" \| "mobile"; focusedAgentId: string \| null; focusedTerminalId?: string \| null; lastActivityAt: string; appVisible: boolean; appVisibilityChangedAt?: string; }): void` | `client_heartbeat` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2473` |
| `registerPushToken(token: string): void` | `register_push_token` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2492` |
| `async unregisterPushToken(token: string): Promise<void>` | `push.unregister.request`<br>`push.unregister.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2499` |

## Typed low-level client: Projects, workspaces, labels and recovery

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async clearWorkspaceAttention(workspaceId: string \| string[]): Promise<void>` | `workspace.clear_attention.request`<br>`workspace.clear_attention.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2433` |
| `async markWorkspaceUnread(workspaceId: string, requestId?: string): Promise<void>` | `workspace.mark_unread.response`<br>`workspace.mark_unread.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2459` |
| `observeWorkspaces( options: Omit<FetchWorkspacesOptions, "subscribe"> = {}, ): OwnedSubscription<FetchWorkspacesPayload>` | `fetch_workspaces_response`<br>`fetch_workspaces_request` | `Delegates to requireBackgroundWorkspaces` | `packages/client/src/daemon-client.ts:2740` |
| `async fetchWorkspaces( options?: FetchWorkspacesOptions, ): Promise< FetchWorkspacesPayload & { subscription?: OwnedSubscription<FetchWorkspacesPayload> } >` | `fetch_workspaces_request`<br>`fetch_workspaces_response` | `Delegates to requireBackgroundWorkspaces, observeWorkspaces` | `packages/client/src/daemon-client.ts:2847` |
| `listWorkspaceLabels( options: { sync?: { generation: string; afterSeq: number }; requestId?: string; } = {}, ): Promise<WorkspaceLabelListPayload>` | `workspace.label.list.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2882` |
| `observeWorkspaceLabels(options?: { signal?: AbortSignal; }): OwnedSubscription<WorkspaceLabelListPayload>` | `workspace.label.list.response`<br>`workspace.label.list.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2897` |
| `setWorkspaceLabel(options: { workspaceId: string; label: Extract< SessionInboundMessage, { type: "workspace.label.assignment.set.request" } >["label"]; assigned: boolean; requestId?: string; }): Promise<WorkspaceLabelAssignmentPayload>` | `workspace.label.assignment.set.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2907` |
| `updateWorkspaceLabel(options: { name: string; newName?: string; color?: Extract<SessionInboundMessage, { type: "workspace.label.update.request" }>["color"]; requestId?: string; }): Promise<WorkspaceLabelUpdatePayload>` | `workspace.label.update.request`<br>`workspace.label.update.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2927` |
| `deleteWorkspaceLabel(options: { name: string; requestId?: string; }): Promise<WorkspaceLabelDeletePayload>` | `workspace.label.delete.response`<br>`workspace.label.delete.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2944` |
| `inspectWorkspaceLabelDelete(options: { name: string; requestId?: string; }): Promise<WorkspaceLabelDeleteInspectPayload>` | `workspace.label.delete.inspect.response`<br>`workspace.label.delete.inspect.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2954` |
| `async listProjects(options?: string \| ProjectListOptions): Promise<ProjectListPayload>` | `project.list.request`<br>`project.list.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2967` |
| `listProjectSuggestions(options?: ProjectSuggestionsOptions): Promise<ProjectSuggestionsPayload>` | `project.suggestions.response`<br>`project.suggestions.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2988` |
| `async openProject(cwd: string, requestId?: string): Promise<OpenProjectPayload>` | `open_project_request`<br>`open_project_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3000` |
| `async addProject(cwd: string, requestId?: string): Promise<ProjectAddPayload>` | `project.add.request`<br>`project.add.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3011` |
| `async createProjectDirectory( input: { parentPath: string; name: string }, requestId?: string, ): Promise<ProjectCreateDirectoryPayload>` | `project.create_directory.response`<br>`project.create_directory.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3022` |
| `async searchGithubRepositories( input: { query: string; limit?: number }, requestId?: string, ): Promise<WorkspaceGithubSearchRepositoriesPayload>` | `workspace.github.search_repositories.response`<br>`workspace.github.search_repositories.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3036` |
| `async cloneGithubProject( input: { repo: string; targetDirectory: string; cloneProtocol?: ProjectGithubCloneProtocol }, requestId?: string, ): Promise<ProjectGithubClonePayload>` | `project.github.clone.request`<br>`project.github.clone.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3052` |
| `async archiveWorkspace( workspaceId: string, requestId?: string, ): Promise<ArchiveWorkspacePayload>` | `archive_workspace_request`<br>`archive_workspace_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3128` |
| `async renameProject( projectId: string, customName: string \| null, requestId?: string, ): Promise<{ customName: string \| null }>` | `project.rename.request`<br>`project.rename.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3448` |
| `async setProjectIcon( projectId: string, source: ProjectIconSource, requestId?: string, ): Promise<void>` | `project.icon.set.response`<br>`project.icon.set.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3468` |
| `async removeProject( projectId: string, requestId?: string, ): Promise<{ removedWorkspaceIds: string[] }>` | `project.remove.response`<br>`project.remove.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3480` |
| `async setWorkspaceTitle( workspaceId: string, title: string \| null, requestId?: string, ): Promise<{ title: string \| null }>` | `workspace.title.set.request`<br>`workspace.title.set.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3497` |
| `async setWorkspacePinned( workspaceId: string, pinned: boolean, requestId?: string, ): Promise<{ pinnedAt: string \| null }>` | `workspace.pin.set.request`<br>`workspace.pin.set.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3517` |
| `async inspectWorkspaceRecovery( workspaceId: string, requestId?: string, ): Promise<WorkspaceRecoveryState>` | `workspace.recovery.inspect.response`<br>`workspace.recovery.inspect.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3537` |
| `async restoreWorkspace(workspaceId: string, requestId?: string): Promise<void>` | `workspace.recovery.restore.response`<br>`workspace.recovery.restore.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3552` |
| `async getProjectIcon( projectId: string, requestId?: string, ): Promise<ProjectIconGetResponse["payload"]>` | `project.icon.get.response`<br>`project.icon.get.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5470` |

## Typed low-level client: Connection, daemon lifecycle and diagnostics

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async ping(params?: { requestId?: string; timeoutMs?: number }): Promise<{ requestId: string; clientSentAt: number; serverReceivedAt: number; serverSentAt: number; rttMs: number; }>` | `ping`<br>`pong` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2509` |
| `observeEvents( events: SessionEventSubscription[], options?: { signal?: AbortSignal; notifications?: boolean }, ): OwnedSubscription<CorrelatedResponsePayload<"session.events.set_subscription.response">>` | `session.events.set_subscription.response`<br>`session.events.set_subscription.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2712` |
| `async createAgent(options: CreateAgentRequestOptions): Promise<AgentSnapshotPayload>` | `agent.create.request`<br>`agent.create.response`<br>`creation.subscribe.request`<br>`creation.subscribe.response`<br>`agent.create.update`<br>`create_agent_request`<br>`status` | `Delegates to requireBackgroundWorkspaces` | `packages/client/src/daemon-client.ts:3284` |
| `async resumeAgent( handle: AgentPersistenceHandle, overrides?: Partial<AgentSessionConfig>, ): Promise<AgentSnapshotPayload>` | `resume_agent_request`<br>`status` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3567` |
| `async importAgent(input: ImportAgentInput): Promise<AgentSnapshotPayload>` | `import_agent_request`<br>`status` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3598` |
| `async refreshAgent(agentId: string, requestId?: string): Promise<AgentRefreshedStatusPayload>` | `refresh_agent_request`<br>`status` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3640` |
| `async restartServer( reason?: string, requestId?: string, options?: { timeout?: number; killTerminals?: boolean }, ): Promise<RestartRequestedStatusPayload>` | `restart_server_request`<br>`status` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4158` |
| `async shutdownServer(options?: ShutdownServerOptions): Promise<ShutdownRequestedStatusPayload>` | `shutdown_server_request`<br>`status` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4191` |
| `async updateDaemon(requestId?: string): Promise<DaemonUpdateResponse["payload"]>` | `daemon.update.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4219` |
| `async getDaemonConfig( requestId?: string, ): Promise<{ requestId: string; config: MutableDaemonConfig }>` | `get_daemon_config_request`<br>`get_daemon_config_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5570` |
| `async getDaemonStatus(options?: DaemonStatusOptions): Promise<DaemonStatusPayload>` | `daemon.get_status.request`<br>`daemon.get_status.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5582` |
| `async reloadDaemonConfig(requestId?: string): Promise<DaemonConfigReloadResponse["payload"]>` | `daemon.config.reload.request` | `Delegates to requireDaemonConfigReloadSupport` | `packages/client/src/daemon-client.ts:5597` |
| `async getDaemonPairingOffer( options?: DaemonPairingOfferOptions, ): Promise<DaemonPairingOfferPayload>` | `daemon.get_pairing_offer.request`<br>`daemon.get_pairing_offer.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5653` |
| `async collectDiagnostics(requestId?: string): Promise<DiagnosticsPayload>` | `diagnostics.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5666` |
| `async patchDaemonConfig( config: MutableDaemonConfigPatch, requestId?: string, ): Promise<{ requestId: string; config: MutableDaemonConfig }>` | `set_daemon_config_request`<br>`set_daemon_config_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5675` |
| `async listUsageReports( options?: { agentId?: string; requestId?: string; forceRefresh?: boolean; reportIds?: string[]; }, onReport?: (report: UsageReportEntry) => void, ): Promise<UsageListReportsPayload>` | `usage.list_reports.response`<br>`rpc_error`<br>`usage.list_reports.update`<br>`usage.list_reports.request`<br>`provider.usage.list.request` | `Delegates to getLastServerInfoMessage, listProviderUsage, subscribeRawMessages` | `packages/client/src/daemon-client.ts:5786` |

## Typed low-level client: Browser host and automation

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `registerBrowserHost( registration: Omit< Extract<SessionInboundMessage, { type: "browser.host.register.request" }>, "type" \| "requestId" >, options?: { signal?: AbortSignal }, ): OwnedSubscription<CorrelatedResponsePayload<"browser.host.register.response">>` | `browser.host.register.request`<br>`browser.host.register.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2698` |
| `sendBrowserAutomationExecuteResponse(response: BrowserAutomationExecuteResponse): void` | `browser.automation.execute.response` | `Caller-supplied typed response; reverse RPC` | `packages/client/src/daemon-client.ts:5717` |

## Typed low-level client: Agents and runtime configuration

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `observeAgents( options: Omit<FetchAgentsOptions, "subscribe"> = {}, ): OwnedSubscription<FetchAgentsPayload>` | `fetch_agents_response`<br>`fetch_agents_request` | `Delegates to requireBackgroundWorkspaces` | `packages/client/src/daemon-client.ts:2727` |
| `async fetchAgents( options?: FetchAgentsOptions, ): Promise<FetchAgentsPayload & { subscription?: OwnedSubscription<FetchAgentsPayload> }>` | `fetch_agents_request`<br>`fetch_agents_response` | `Delegates to requireBackgroundWorkspaces, observeAgents` | `packages/client/src/daemon-client.ts:2757` |
| `async fetchAgentHistory(options?: FetchAgentHistoryOptions): Promise<FetchAgentHistoryPayload>` | `fetch_agent_history_request`<br>`fetch_agent_history_response` | `Delegates to requireBackgroundWorkspaces` | `packages/client/src/daemon-client.ts:2787` |
| `async fetchAgent( input: FetchAgentOptions \| string, legacyOptions?: LegacyFetchAgentOptions \| string, ): Promise<FetchAgentResult \| null>` | `fetch_agent_request`<br>`fetch_agent_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3193` |
| `async deleteAgent(agentId: string): Promise<void>` | `delete_agent_request`<br>`agent_deleted` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3356` |
| `async archiveAgent(agentId: string): Promise<{ archivedAt: string }>` | `archive_agent_request`<br>`agent_archived` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3379` |
| `async detachAgent(agentId: string): Promise<void>` | `agent.detach.response`<br>`agent.detach.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3403` |
| `async updateAgent( agentId: string, updates: { name?: string; labels?: Record<string, string> }, ): Promise<void>` | `update_agent_request`<br>`update_agent_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3415` |
| `async sendAgentMessage( agentId: string, text: string, options: SendMessageOptions = {}, ): Promise<{ deliveryStatus?: "queued" }>` | `send_agent_message_request`<br>`send_agent_message_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3887` |
| `async sendMessage( agentId: string, text: string, options?: SendMessageOptions, ): Promise<{ deliveryStatus?: "queued" }>` | `send_agent_message_request`<br>`send_agent_message_response` | `Delegates to sendAgentMessage` | `packages/client/src/daemon-client.ts:3939` |
| `async rewindAgent( agentId: string, messageId: string, mode: "conversation" \| "files" \| "both", ): Promise<AgentRewindResponseMessage["payload"]>` | `agent.rewind.request`<br>`agent.rewind.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3947` |
| `async cancelAgent(agentId: string): Promise<void>` | `cancel_agent_request`<br>`cancel_agent_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3980` |
| `async setAgentMode(agentId: string, modeId: string): Promise<AgentProviderNotice \| null>` | `set_agent_mode_request`<br>`set_agent_mode_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4006` |
| `async setAgentModel(agentId: string, modelId: string \| null): Promise<void>` | `set_agent_model_request`<br>`set_agent_model_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4034` |
| `async setAgentFeature(agentId: string, featureId: string, value: unknown): Promise<void>` | `set_agent_feature_request`<br>`set_agent_feature_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4061` |
| `async setAgentThinkingOption( agentId: string, thinkingOptionId: string \| null, ): Promise<AgentProviderNotice \| null>` | `set_agent_thinking_request`<br>`set_agent_thinking_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4089` |
| `async applyAgentConfig( agentId: string, config: AgentConfigApply, ): Promise<AgentProviderNotice \| null>` | `agent.config.apply.request`<br>`agent.config.apply.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4127` |
| `async createFileEntry(input: { cwd: string; parentPath: string; name: string; kind: "file" \| "directory"; }): Promise<CorrelatedResponsePayload<"fs.entry.create.response">>` | `fs.entry.create.response`<br>`fs.entry.create.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5304` |
| `async renameFileEntry(input: { cwd: string; path: string; name: string; }): Promise<CorrelatedResponsePayload<"fs.entry.rename.response">>` | `fs.entry.rename.response`<br>`fs.entry.rename.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5315` |
| `async duplicateFileEntry(input: { cwd: string; path: string; }): Promise<CorrelatedResponsePayload<"fs.entry.duplicate.response">>` | `fs.entry.duplicate.response`<br>`fs.entry.duplicate.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5325` |
| `async deleteFileEntry(input: { cwd: string; path: string; }): Promise<CorrelatedResponsePayload<"fs.entry.delete.response">>` | `fs.entry.delete.response`<br>`fs.entry.delete.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5334` |
| `async listCommands( input: ListCommandsOptions \| string, legacyOptions?: LegacyListCommandsOptions \| string, ): Promise<ListCommandsPayload>` | `list_commands_request`<br>`list_commands_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5893` |
| `async waitForAgentUpsert( agentId: string, predicate: (snapshot: AgentSnapshotPayload) => boolean, timeout = 60000, ): Promise<AgentSnapshotPayload>` | `agent_update`<br>`fetch_agent_request`<br>`fetch_agent_response` | `Delegates to fetchAgent, on` | `packages/client/src/daemon-client.ts:6273` |
| `async waitForFinish(agentId: string, timeout = 60000): Promise<WaitForFinishResult>` | `wait_for_finish_request`<br>`wait_for_finish_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6378` |
| `async closeItems( input: { agentIds?: string[]; terminalIds?: string[] }, requestId?: string, ): Promise<CloseItemsPayload>` | `close_items_request`<br>`close_items_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6587` |

## Typed low-level client: Providers, commands and usage

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async fetchRecentProviderSessions( options?: FetchRecentProviderSessionsOptions, ): Promise<FetchRecentProviderSessionsPayload>` | `fetch_recent_provider_sessions_request`<br>`fetch_recent_provider_sessions_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:2814` |
| `async listProviderModels( provider: AgentProvider, options?: { cwd?: string; requestId?: string }, ): Promise<ListProviderModelsPayload>` | `list_provider_models_request`<br>`list_provider_models_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5484` |
| `async listProviderModes( provider: AgentProvider, options?: { cwd?: string; requestId?: string }, ): Promise<ListProviderModesPayload>` | `list_provider_modes_request`<br>`list_provider_modes_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5502` |
| `async listProviderFeatures( draftConfig: ListCommandsDraftConfig, options?: { requestId?: string }, ): Promise<ListProviderFeaturesPayload>` | `list_provider_features_request`<br>`list_provider_features_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5518` |
| `async listAvailableProviders(options?: { requestId?: string; }): Promise<ListAvailableProvidersPayload>` | `list_available_providers_request`<br>`list_available_providers_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5533` |
| `async getProvidersSnapshot(options?: { cwd?: string; ifNoneMatch?: string; requestId?: string; }): Promise<GetProvidersSnapshotPayload>` | `get_providers_snapshot_request`<br>`get_providers_snapshot_response` | `Delegates to requestProvidersSnapshot` | `packages/client/src/daemon-client.ts:5545` |
| `async refreshProvidersSnapshot(options?: { cwd?: string; providers?: AgentProvider[]; requestId?: string; }): Promise<RefreshProvidersSnapshotPayload>` | `refresh_providers_snapshot_request`<br>`refresh_providers_snapshot_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5745` |
| `async getProviderDiagnostic( provider: AgentProvider, options?: { requestId?: string }, ): Promise<ProviderDiagnosticPayload>` | `provider_diagnostic_request`<br>`provider_diagnostic_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5762` |
| `async listProviderUsage(options?: { requestId?: string }): Promise<ProviderUsageListPayload>` | `provider.usage.list.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5777` |

## Typed low-level client: Workspace scripts

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async startWorkspaceScript( workspaceId: string, scriptName: string, requestId?: string, ): Promise< Extract<SessionOutboundMessage, { type: "start_workspace_script_response" }>["payload"] >` | `start_workspace_script_response`<br>`start_workspace_script_request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3069` |
| `async listWorkspaceScripts( workspaceId: string, requestId?: string, ): Promise< Extract<SessionOutboundMessage, { type: "workspace.script.list.response" }>["payload"] >` | `workspace.script.list.response`<br>`workspace.script.list.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3087` |
| `async startWorkspaceScriptWithStatus( workspaceId: string, scriptName: string, requestId?: string, ): Promise< Extract<SessionOutboundMessage, { type: "workspace.script.start.response" }>["payload"] >` | `workspace.script.start.response`<br>`workspace.script.start.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3100` |
| `async stopWorkspaceScript( workspaceId: string, scriptName: string, requestId?: string, ): Promise< Extract<SessionOutboundMessage, { type: "workspace.script.stop.response" }>["payload"] >` | `workspace.script.stop.response`<br>`workspace.script.stop.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3114` |

## Typed low-level client: Workspace setup

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `observeWorkspaceSetup( workspaceId: string, options?: { signal?: AbortSignal }, ): OwnedSubscription<CorrelatedResponsePayload<"workspace.setup.subscribe.response">>` | `workspace.setup.subscribe.response`<br>`workspace.setup.subscribe.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3142` |
| `async fetchWorkspaceSetupStatus( workspaceId: string, requestId?: string, ): Promise<WorkspaceSetupStatusPayload>` | `workspace_setup_status_request`<br>`workspace_setup_status_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3163` |
| `async runWorkspaceSetup( workspaceId: string, requestId?: string, ): Promise<Extract<SessionOutboundMessage, { type: "workspace.setup.run.response" }>["payload"]>` | `workspace.setup.run.response`<br>`workspace.setup.run.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3177` |

## Typed low-level client: Provider subagents

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async listProviderSubagents( parentAgentId: string, options: { requestId?: string; timeout?: number } = {}, ): Promise<ProviderSubagentListPayload>` | `agent.provider_subagents.list.request`<br>`agent.provider_subagents.list.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3764` |
| `async fetchProviderSubagentTimeline( parentAgentId: string, subagentId: string, options: FetchProviderSubagentTimelineOptions = {}, ): Promise<ProviderSubagentTimelinePayload>` | `agent.provider_subagents.timeline.get.request`<br>`agent.provider_subagents.timeline.get.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:3791` |

## Typed low-level client: Voice, dictation and local speech models

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async setVoiceMode(enabled: boolean, agentId?: string): Promise<SetVoiceModePayload>` | `set_voice_mode`<br>`set_voice_mode_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4247` |
| `async sendVoiceAudioChunk(audio: string, format: string, isLast = false): Promise<void>` | `voice_audio_chunk` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4278` |
| `async startDictationStream(dictationId: string, format: string): Promise<void>` | `dictation_stream_ack`<br>`dictation_stream_error`<br>`dictation_stream_start` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4282` |
| `sendDictationStreamChunk(dictationId: string, seq: number, audio: string, format: string): void` | `dictation_stream_chunk` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4335` |
| `async finishDictationStream( dictationId: string, finalSeq: number, ): Promise<{ dictationId: string; text: string }>` | `dictation_stream_final`<br>`dictation_stream_error`<br>`dictation_stream_finish_accepted`<br>`dictation_stream_finish` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4345` |
| `cancelDictationStream(dictationId: string): void` | `dictation_stream_cancel` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4497` |
| `async abortRequest(): Promise<void>` | `abort_request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4501` |
| `async audioPlayed(id: string): Promise<void>` | `audio_played` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4505` |
| `async listLocalSpeechModels(requestId?: string): Promise<LocalSpeechModel[]>` | `speech.local_models.list.request`<br>`speech.local_models.list.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5689` |
| `async downloadLocalSpeechModel(modelId: string, requestId?: string): Promise<void>` | `speech.local_models.download.request`<br>`speech.local_models.download.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5699` |
| `async deleteLocalSpeechModel(modelId: string, requestId?: string): Promise<void>` | `speech.local_models.delete.request`<br>`speech.local_models.delete.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5708` |

## Typed low-level client: Checkout, Git and forge

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async getCheckoutStatus( cwd: string, options?: { requestId?: string }, ): Promise<CheckoutStatusPayload>` | `checkout_status_request`<br>`checkout_status_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4513` |
| `async getCheckoutDiff( cwd: string, compare: { mode: "uncommitted" \| "base" \| "all"; baseRef?: string; ignoreWhitespace?: boolean }, requestId?: string, ): Promise<CheckoutDiffPayload>` | `checkout.diff.get.request`<br>`checkout.diff.get.response` | `Delegates to normalizeCheckoutDiffCompare` | `packages/client/src/daemon-client.ts:4583` |
| `observeCheckoutDiff( cwd: string, compare: { mode: "uncommitted" \| "base" \| "all"; baseRef?: string; ignoreWhitespace?: boolean }, options?: { requestId?: string; signal?: AbortSignal }, ): OwnedSubscription<SubscribeCheckoutDiffPayload>` | `subscribe_checkout_diff_response`<br>`subscribe_checkout_diff_request` | `Delegates to normalizeCheckoutDiffCompare` | `packages/client/src/daemon-client.ts:4599` |
| `async checkoutCommit( cwd: string, input: { message?: string; addAll?: boolean }, requestId?: string, ): Promise<CheckoutCommitPayload>` | `checkout_commit_request`<br>`checkout_commit_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4615` |
| `async checkoutMerge( cwd: string, input: { baseRef?: string; strategy?: "merge" \| "squash"; requireCleanTarget?: boolean }, requestId?: string, ): Promise<CheckoutMergePayload>` | `checkout_merge_request`<br>`checkout_merge_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4632` |
| `async checkoutMergeFromBase( cwd: string, input: { baseRef?: string; requireCleanTarget?: boolean }, requestId?: string, ): Promise<CheckoutMergeFromBasePayload>` | `checkout_merge_from_base_request`<br>`checkout_merge_from_base_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4650` |
| `async checkoutPull(cwd: string, requestId?: string): Promise<CheckoutPullPayload>` | `checkout_pull_request`<br>`checkout_pull_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4667` |
| `async checkoutPush(cwd: string, requestId?: string): Promise<CheckoutPushPayload>` | `checkout_push_request`<br>`checkout_push_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4678` |
| `async checkoutRefresh(cwd: string, requestId?: string): Promise<CheckoutRefreshPayload>` | `checkout.refresh.request`<br>`checkout.refresh.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4689` |
| `async listCheckoutCommits( cwd: string, requestId?: string, ): Promise<{ baseRef: string \| null; commits: CheckoutCommit[] }>` | `checkout.commits.list.response`<br>`checkout.commits.list.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4700` |
| `async getCommitFileDiff( cwd: string, sha: string, path: string, requestId?: string, ): Promise<{ file: ParsedDiffFile \| null }>` | `checkout.commits.file_diff.response`<br>`checkout.commits.file_diff.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4719` |
| `async checkoutPrCreate( cwd: string, input: { title?: string; body?: string; baseRef?: string }, requestId?: string, ): Promise<CheckoutPrCreatePayload>` | `checkout_pr_create_request`<br>`checkout_pr_create_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4742` |
| `async checkoutPrMerge( cwd: string, input: { method: CheckoutPrMergeMethod }, requestId?: string, ): Promise<CheckoutPrMergePayload>` | `checkout_pr_merge_request`<br>`checkout_pr_merge_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4760` |
| `async checkoutForgeSetAutoMerge( cwd: string, input: { enabled: true; method: CheckoutPrMergeMethod } \| { enabled: false }, requestId?: string, ): Promise<CheckoutForgeSetAutoMergePayload>` | `checkout.forge.set_auto_merge.response`<br>`checkout.forge.set_auto_merge.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4776` |
| `async checkoutGithubSetAutoMerge( cwd: string, input: { enabled: true; method: CheckoutPrMergeMethod } \| { enabled: false }, requestId?: string, ): Promise<CheckoutGithubSetAutoMergePayload>` | `checkout.github.set_auto_merge.response`<br>`checkout.github.set_auto_merge.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4793` |
| `async checkoutForgeGetCheckDetails( input: { cwd: string; repoOwner?: string; repoName?: string; checkRunId?: number; workflowRunId?: number; changeRequestNumber?: number; }, requestId?: string, ): Promise<CheckoutForgeGetCheckDetailsPayload>` | `checkout.forge.get_check_details.response`<br>`checkout.forge.get_check_details.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4809` |
| `async checkoutGithubGetCheckDetails( input: { cwd: string; repoOwner?: string; repoName?: string; checkRunId?: number; workflowRunId?: number; }, requestId?: string, ): Promise<CheckoutGithubGetCheckDetailsPayload>` | `checkout.github.get_check_details.response`<br>`checkout.github.get_check_details.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4837` |
| `async checkoutPrStatus(cwd: string, requestId?: string): Promise<CheckoutPrStatusPayload>` | `checkout_pr_status_request`<br>`checkout_pr_status_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4862` |
| `async checkoutSwitchBranch( cwd: string, branch: string, requestId?: string, ): Promise<CheckoutSwitchBranchPayload>` | `checkout_switch_branch_request`<br>`checkout_switch_branch_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4890` |
| `async renameBranch(input: RenameBranchInput): Promise<RenameBranchResult>` | `checkout.rename_branch.request`<br>`checkout.rename_branch.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4906` |
| `async stashSave( cwd: string, options?: { branch?: string }, requestId?: string, ): Promise<StashSavePayload>` | `stash_save_request`<br>`stash_save_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4918` |
| `async stashPop(cwd: string, stashIndex: number, requestId?: string): Promise<StashPopPayload>` | `stash_pop_request`<br>`stash_pop_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4934` |
| `async stashList( cwd: string, options?: { paseoOnly?: boolean }, requestId?: string, ): Promise<StashListPayload>` | `stash_list_request`<br>`stash_list_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4946` |
| `async validateBranch( options: { cwd: string; branchName: string }, requestId?: string, ): Promise<ValidateBranchPayload>` | `validate_branch_request`<br>`validate_branch_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5073` |
| `async getBranchSuggestions( options: { cwd: string; query?: string; limit?: number }, requestId?: string, ): Promise<BranchSuggestionsPayload>` | `branch_suggestions_request`<br>`branch_suggestions_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5088` |
| `async searchForge( options: { cwd: string; query: string; limit?: number; kinds?: ForgeSearchRequest["kinds"] }, requestId?: string, ): Promise<ForgeSearchPayload>` | `forge.search.request`<br>`forge.search.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5104` |
| `async searchGitHub( options: { cwd: string; query: string; limit?: number; kinds?: GitHubSearchRequest["kinds"] }, requestId?: string, ): Promise<GitHubSearchPayload>` | `github_search_request`<br>`github_search_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5122` |
| `async checkoutDiscardChanges( cwd: string, input: { paths: string[] }, ): Promise<CorrelatedResponsePayload<"checkout.discard_changes.response">>` | `checkout.discard_changes.response`<br>`checkout.discard_changes.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5343` |

## Typed low-level client: Legacy managed worktrees and directories

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async getPaseoWorktreeList( input: { cwd?: string; repoRoot?: string }, requestId?: string, ): Promise<PaseoWorktreeListPayload>` | `paseo_worktree_list_request`<br>`paseo_worktree_list_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4962` |
| `async archivePaseoWorktree( input: { worktreePath?: string; repoRoot?: string; branchName?: string; workspaceId?: string; scope?: "workspace" \| "worktree"; }, requestId?: string, ): Promise<PaseoWorktreeArchivePayload>` | `paseo_worktree_archive_request`<br>`paseo_worktree_archive_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:4977` |
| `async createPaseoWorktree( input: CreatePaseoWorktreeInput, requestId?: string, ): Promise<CreatePaseoWorktreePayload>` | `create_paseo_worktree_request`<br>`create_paseo_worktree_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5001` |
| `async getDirectorySuggestions( options: { query: string; limit?: number; cwd?: string; includeFiles?: boolean; includeDirectories?: boolean; matchMode?: "fuzzy" \| "suffix"; }, requestId?: string, ): Promise<DirectorySuggestionsPayload>` | `directory_suggestions_request`<br>`directory_suggestions_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5139` |

## Typed low-level client: Creation lifecycle and receipts

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async createWorkspace( input: CreateWorkspaceRequestOptions, requestId?: string, ): Promise<WorkspaceCreatePayload>` | `workspace.create.request`<br>`workspace.create.response`<br>`creation.subscribe.request`<br>`creation.subscribe.response`<br>`workspace.create.update` | `Delegates to requireBackgroundWorkspaces` | `packages/client/src/daemon-client.ts:5024` |

## Typed low-level client: Files and binary transfer

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async listDirectory( cwd: string, path: string, requestId?: string, ): Promise<FileExplorerDirectoryPayload>` | `file_explorer_request`<br>`file_explorer_response` | `Delegates to requestFileExplorer` | `packages/client/src/daemon-client.ts:5195` |
| `async readFile( cwd: string, path: string, requestId?: string, maxBytes?: number, timeout?: number, ): Promise<FileReadResult>` | `file_explorer_request`<br>`file_explorer_response` | `Delegates to requestFileExplorer` | `packages/client/src/daemon-client.ts:5215` |
| `observeFile(input: { cwd: string; path: string; signal?: AbortSignal; }): OwnedSubscription<CorrelatedResponsePayload<"fs.file.subscribe.response">>` | `fs.file.subscribe.response`<br>`fs.file.subscribe.request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5252` |
| `async subscribeFile( input: { cwd: string; path: string; signal?: AbortSignal }, onUpdate: (version: FileVersion) => void, ): Promise<{ initial: FileVersion; subscription: OwnedSubscription<CorrelatedResponsePayload<"fs.file.subscribe.response">>; unsubscribe: () => Promise<void>; }>` | `fs.file.subscribe.response`<br>`fs.file.update`<br>`fs.file.subscribe.request` | `Delegates to observeFile` | `packages/client/src/daemon-client.ts:5264` |
| `async writeFile(input: { cwd: string; path: string; content: string; contentEncoding?: "utf-8" \| "base64"; expectedModifiedAt: string; expectedRevision?: string; }): Promise<FileWriteResult>` | `fs.file.write.request`<br>`fs.file.write.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5288` |
| `async uploadFile(input: FileUploadInput): Promise<FileUploadResult>` | `file.upload.request`<br>`file.upload.response` | `Delegates to whenConnected, sendBinaryFrame, rejectWaitersForRequestId` | `packages/client/src/daemon-client.ts:5352` |
| `async requestDownloadToken( cwd: string, path: string, requestId?: string, ): Promise<FileDownloadTokenPayload>` | `file_download_token_request`<br>`file_download_token_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5440` |
| `async requestProjectIcon( cwd: string, requestId?: string, ): Promise<ProjectIconResponse["payload"]>` | `project_icon_request`<br>`project_icon_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5456` |

## Typed low-level client: Hub management and execution

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async connectHub( hubUrl: string, token: string, permissions: readonly string[] = [], requestId?: string, )` | `hub.management.daemon.connect.request`<br>`hub.management.daemon.connect.response` | `Delegates to requireHubRelationshipSupport` | `packages/client/src/daemon-client.ts:5605` |
| `async updateHubPermissions( input: { grant?: readonly string[]; revoke?: readonly string[] }, requestId?: string, )` | `hub.management.daemon.permissions.update.request`<br>`hub.management.daemon.permissions.update.response` | `Delegates to requireHubRelationshipSupport` | `packages/client/src/daemon-client.ts:5619` |
| `async getHubStatus(requestId?: string)` | `hub.management.daemon.get_status.request`<br>`hub.management.daemon.get_status.response` | `Delegates to requireHubRelationshipSupport` | `packages/client/src/daemon-client.ts:5635` |
| `async disconnectHub(force = false, requestId?: string)` | `hub.management.daemon.disconnect.request`<br>`hub.management.daemon.disconnect.response` | `Delegates to requireHubRelationshipSupport` | `packages/client/src/daemon-client.ts:5644` |

## Typed low-level client: Project configuration

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async readProjectConfig(repoRoot: string, requestId?: string): Promise<ReadProjectConfigPayload>` | `read_project_config_request`<br>`read_project_config_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5721` |
| `async writeProjectConfig(input: WriteProjectConfigInput): Promise<WriteProjectConfigPayload>` | `write_project_config_request`<br>`write_project_config_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5732` |

## Typed low-level client: Agent permissions

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async respondToPermission( agentId: string, requestId: string, response: AgentPermissionResponse, ): Promise<void>` | `agent_permission_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:5913` |
| `async respondToPermissionAndWait( agentId: string, requestId: string, response: AgentPermissionResponse, timeout = 15000, ): Promise<AgentPermissionResolvedPayload>` | `agent_permission_response`<br>`agent_permission_resolved` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6237` |

## Typed low-level client: Plugins

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async getPluginCatalog(): Promise<PluginCatalogEntry[]>` | `plugin.catalog.get.request`<br>`plugin.catalog.get.response` | `Delegates to getLastServerInfoMessage, persistPluginClientBundles` | `packages/client/src/daemon-client.ts:5926` |
| `async listPlugins(): Promise<PluginListItem[]>` | `plugin.list.request`<br>`plugin.list.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6004` |
| `async getPluginLogs(pluginId: string): Promise<PluginLogEntry[]>` | `plugin.logs.get.request`<br>`plugin.logs.get.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6014` |
| `async installDirectoryPlugin(path: string, id?: string): Promise<PluginListItem>` | `plugin.directory.install.request`<br>`plugin.directory.install.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6079` |
| `async installPluginSource(input: { source: string; id?: string; ref?: string; }): Promise<PluginListItem>` | `plugin.source.install.request`<br>`plugin.source.install.response` | `Delegates to getLastServerInfoMessage` | `packages/client/src/daemon-client.ts:6089` |
| `async getPluginSourceStatus(pluginId?: string): Promise<PluginSourceStatusItem[]>` | `plugin.source.status.request`<br>`plugin.source.status.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6114` |
| `async previewPluginUpdates( input: { pluginId?: string; target?: PluginUpdateSelection } = {}, ): Promise<PluginUpdatePreview[]>` | `plugin.source.update.preview.request`<br>`plugin.source.update.preview.response` | `Delegates to requirePluginUpdates` | `packages/client/src/daemon-client.ts:6134` |
| `async applyPluginUpdates(proposals: PluginUpdateProposal[]): Promise<PluginUpdateResult[]>` | `plugin.source.update.apply.request`<br>`plugin.source.update.apply.response` | `Delegates to requirePluginUpdates` | `packages/client/src/daemon-client.ts:6148` |
| `async updatePluginSources(pluginId?: string): Promise<PluginSourceUpdateItem[]>` | `plugin.source.update.request`<br>`plugin.source.update.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6160` |
| `async inspectDirectoryPlugin(path: string): Promise<{ id: string }>` | `plugin.directory.inspect.request`<br>`plugin.directory.inspect.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6174` |
| `async reloadPlugin(pluginId: string): Promise<PluginListItem>` | `plugin.reload.request`<br>`plugin.reload.response` | `Delegates to managePlugin` | `packages/client/src/daemon-client.ts:6183` |
| `async enablePlugin(pluginId: string): Promise<PluginListItem>` | `plugin.enable.request`<br>`plugin.enable.response` | `Delegates to managePlugin` | `packages/client/src/daemon-client.ts:6187` |
| `async disablePlugin(pluginId: string): Promise<PluginListItem>` | `plugin.disable.request`<br>`plugin.disable.response` | `Delegates to managePlugin` | `packages/client/src/daemon-client.ts:6191` |
| `async removePlugin(pluginId: string): Promise<void>` | `plugin.remove.request`<br>`plugin.remove.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6195` |
| `async invokePluginRpc(pluginId: string, method: string, input: unknown): Promise<unknown>` | `plugin.rpc.invoke.request`<br>`plugin.rpc.invoke.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6221` |

## Typed low-level client: Orchestration skills

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async getAgentSkillsStatus(): Promise<AgentSkillsStatus>` | `agent.skills.get_status.request`<br>`agent.skills.get_status.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6024` |
| `async reconcileAgentSkills(): Promise<AgentSkillsStatus>` | `agent.skills.reconcile.request`<br>`agent.skills.reconcile.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6032` |
| `async uninstallAgentSkills(): Promise<AgentSkillsStatus>` | `agent.skills.uninstall.request`<br>`agent.skills.uninstall.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6040` |
| `async saveAgentSkillsSelection( selection: AgentSkillSelection, confirmedRemovals?: readonly string[], ): Promise<AgentSkillsSaveResult>` | `agent.skills.save_selection.request`<br>`agent.skills.save_selection.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6048` |
| `async importLegacyAgentSkillsSelection(selection: AgentSkillSelection): Promise<{ imported: boolean; selection: AgentSkillSelection; }>` | `agent.skills.import_legacy_selection.request`<br>`agent.skills.import_legacy_selection.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6064` |

## Typed low-level client: Terminals

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `observeTerminals(input: { cwd: string; workspaceId?: string; signal?: AbortSignal; }): OwnedSubscription<CorrelatedResponsePayload<"terminals_changed">>` | `terminals_changed`<br>`subscribe_terminals_request` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6406` |
| `async listTerminals( cwd?: string, requestId?: string, options?: { workspaceId?: string }, ): Promise<ListTerminalsPayload>` | `list_terminals_request`<br>`list_terminals_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6419` |
| `async createTerminal( cwd: string, name?: string, requestId?: string, options?: { agentId?: string; command?: string; args?: string[]; workspaceId?: string; size?: { rows: number; cols: number }; }, ): Promise<CreateTerminalPayload>` | `create_terminal_request`<br>`create_terminal_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6439` |
| `async renameTerminal(input: RenameTerminalInput): Promise<RenameTerminalResult>` | `terminal.rename.request`<br>`terminal.rename.response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6471` |
| `observeTerminal( terminalId: string, receive: (event: TerminalStreamEvent) => void, options?: { restore?: SubscribeTerminalRequest["restore"]; signal?: AbortSignal; requestId?: string; }, ): OwnedSubscription<SubscribeTerminalPayload>` | `subscribe_terminal_response`<br>`subscribe_terminal_request`<br>`terminal_stream_exit` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6489` |
| `async subscribeTerminal( terminalId: string, options?: { restore?: SubscribeTerminalRequest["restore"]; signal?: AbortSignal; requestId?: string; }, ): Promise< SubscribeTerminalPayload & { subscription: OwnedSubscription<SubscribeTerminalPayload> } >` | `subscribe_terminal_response`<br>`subscribe_terminal_request`<br>`terminal_stream_exit` | `Delegates to observeTerminal` | `packages/client/src/daemon-client.ts:6554` |
| `sendTerminalInput(terminalId: string, message: TerminalInput["message"]): void` | `terminal_input` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6568` |
| `async killTerminal(terminalId: string, requestId?: string): Promise<KillTerminalPayload>` | `kill_terminal_request`<br>`kill_terminal_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6572` |
| `async captureTerminal( terminalId: string, options?: { start?: number; end?: number; stripAnsi?: boolean }, requestId?: string, ): Promise<CaptureTerminalPayload>` | `capture_terminal_request`<br>`capture_terminal_response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6606` |

## Typed low-level client: Schedules and heartbeats

| Public member / signature | Protocol messages (including selected compatibility paths) | Delegation / local behavior | Source |
|---|---|---|---|
| `async scheduleCreate(options: CreateScheduleOptions): Promise<ScheduleCreatePayload>` | `schedule/create`<br>`schedule/create/response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6628` |
| `async scheduleList(requestId?: string): Promise<ScheduleListPayload>` | `schedule/list`<br>`schedule/list/response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6645` |
| `async scheduleInspect(options: InspectScheduleOptions): Promise<ScheduleInspectPayload>` | `schedule/inspect`<br>`schedule/inspect/response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6655` |
| `async scheduleLogs(options: InspectScheduleOptions): Promise<ScheduleLogsPayload>` | `schedule/logs`<br>`schedule/logs/response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6666` |
| `async schedulePause(options: InspectScheduleOptions): Promise<SchedulePausePayload>` | `schedule/pause`<br>`schedule/pause/response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6677` |
| `async scheduleResume(options: InspectScheduleOptions): Promise<ScheduleResumePayload>` | `schedule/resume`<br>`schedule/resume/response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6688` |
| `async scheduleDelete(options: InspectScheduleOptions): Promise<ScheduleDeletePayload>` | `schedule/delete`<br>`schedule/delete/response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6699` |
| `async scheduleRunOnce(options: InspectScheduleOptions): Promise<ScheduleRunOncePayload>` | `schedule/run-once`<br>`schedule/run-once/response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6710` |
| `async scheduleUpdate(options: UpdateScheduleOptions): Promise<ScheduleUpdatePayload>` | `schedule/update`<br>`schedule/update/response` | `No secondary domain call` | `packages/client/src/daemon-client.ts:6721` |

## Node connector and exported client utilities

| Export | Wire / role | Source |
|---|---|---|
| createHostClient(connection, options) | Creates DaemonClient with parsed direct/socket/SSH/relay transport; caller still connects. | packages/client/src/node/index.ts:205 |
| createHostTransportFactory(connection, budget?) | Per-attempt transport; Node ws, socket/SSH tunnel, relay transport selection. | packages/client/src/node/index.ts:28 |
| createHostChannelTransport | Inner WebSocket-equivalent channel over hosts routes; JSON frames and binary 0x30. | packages/client/src/hosts/index.ts:19 |
| parseHostTarget | Parses connection URI only; no network. | packages/protocol/src/host-target.ts |
| createSshTunnel / resolveSshFailureDetail | System ssh -W transport and diagnostic classification. | packages/client/src/node/ssh-tunnel.ts |
| HostTransportBudget | Shared buffered-byte resource management for host transport. | packages/client/src/node/buffering.ts |

No dedicated public SDK chat, loop, or Hub-execution command methods were found in `packages/client/src/index.ts` or DaemonClient. They remain wire contracts; no current server handlers were found for chat/loop, while Hub execution has session dispatch. Do not infer implementation or a CLI entry point from protocol membership. Browser-host execution responses are deliberately supplied by the client host rather than a normal client-to-daemon request.

## serverInfo.features: complete advertisement and gating inventory

All schema fields below are optional booleans. **Missing is unknown/unsupported unless the calling compatibility path explicitly provides a fallback**; optional does not mean true. `ServerInfoStatusPayloadSchema` accepts additive keys with passthrough. Server advertisements come from buildServerInfoStatusPayload, and some are conditional on a constructed service/runtime setting. Schema-only keys can outlive advertisements.

| Feature key | Wire type/default | This server advertisement | Purpose / compatibility note from source |
|---|---|---|---|
| `agentRequestReceipts` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2011` | COMPAT(agentRequestReceipts): added in v0.8.0; remove gate after 2027-03-05. |
| `workspaceRequestReceipts` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2012` | COMPAT(workspaceRequestReceipts): added in v0.8.0; remove gate after 2027-03-07. |
| `creationLifecycle` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2013` |  |
| `hubAgentRpc` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2014` | COMPAT(hubAgentRpc): added in v0.8.0; remove gate after 2027-03-05. |
| `internalAgents` | `z.boolean().optional()`; absent unspecified | not advertised by buildServerInfoStatusPayload | COMPAT(internalAgents): added in v0.9.0; remove gate after 2027-03-17. |
| `internalWorkspaces` | `z.boolean().optional()`; absent unspecified | not advertised by buildServerInfoStatusPayload | COMPAT(internalWorkspaces): added in v0.9.0; remove gate after 2027-03-17. |
| `backgroundWorkspaces` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2015` |  |
| `providersSnapshot` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2025` |  |
| `connectedHosts` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2003` |  |
| `hostMultiplexing` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2004` |  |
| `usageSetting` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2007` | COMPAT(usageSetting): added in v0.14, remove after 2027-05-01. |
| `usageSources` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2008` |  |
| `providersSnapshotCwd` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2027` | COMPAT(providersSnapshotCwd): added in v0.3.2, remove gate after 2027-02-10. |
| `directorySync` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2017` | COMPAT(directorySync): added in v0.3.x, remove gate after 2027-02-12. |
| `workspaceLabels` | `z.boolean().optional()`; absent unspecified | `when this.workspaceLabelService: true` `packages/server/src/server/websocket-server.ts:2019` | COMPAT(workspaceLabels): added in v0.5.0, remove after 2027-08-14. |
| `workspaceSetupRun` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2021` | COMPAT(workspaceSetupRun): added in v0.8.0, remove gate after 2027-09-02. |
| `workspaceTerminals` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2082` | COMPAT(workspaceTerminals): added in v0.8.0, remove gate after 2027-09-05. |
| `checkoutForgeSetAutoMerge` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2031` | COMPAT(checkoutForgeSetAutoMerge): added in v0.2.0-beta.1. Remove the feature gate and checkoutGithubSetAutoMerge fallback after 2027-01-17 once the supported daemon floor is >= v0.2.0. |
| `checkoutGithubSetAutoMerge` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2035` | COMPAT(checkoutGithubSetAutoMerge): added in v0.1.75 and retained as the fallback for checkoutForgeSetAutoMerge. Stop advertising and consuming it after 2027-01-17 once supported floors are >= v0.2.0. |
| `githubCheckDetails` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2039` | COMPAT(githubCheckDetails): added in v0.1.92 and retained as the fallback for forgeCheckDetails. Stop advertising and consuming it after 2027-01-17 once supported floors are >= v0.2.0. |
| `forgeCheckDetails` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2043` | COMPAT(forgeCheckDetails): added in v0.2.0-beta.1. Remove the feature gate and githubCheckDetails fallback after 2027-01-17 once the supported daemon floor is >= v0.2.0. |
| `forgeSearch` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2047` | COMPAT(forgeSearch): added in v0.2.0-beta.1. Remove the feature gate and github_search fallback after 2027-01-17 once the supported daemon floor is >= v0.2.0. |
| `daemonStatusRpc` | `z.boolean().optional()`; absent unspecified | `when this.advertiseDaemonStatusRpc: true` `packages/server/src/server/websocket-server.ts:2049` | COMPAT(daemonStatusRpc): added in v0.1.76, remove gate after 2026-11-18. |
| `daemonConfigReload` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2051` | COMPAT(daemonConfigReload): added in v0.4.0, remove gate after 2027-02-14. |
| `speechSettings` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2053` | COMPAT(speechSettings): added in v0.12.0, remove gate after 2027-04-18. |
| `relayConfig` | `z.boolean().optional()`; absent unspecified | `when this.advertiseRelayConfig: true` `packages/server/src/server/websocket-server.ts:2055` | COMPAT(relayConfig): added in v0.2.6, remove gate after 2027-01-31. |
| `pushTokenRevocation` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2057` | COMPAT(pushTokenRevocation): added in v0.3.2, remove gate after 2027-02-10. |
| `plugins` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2059` | COMPAT(plugins): added in v0.3.0, remove gate after 2027-08-07. |
| `pluginManagement` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2060` | COMPAT(pluginManagement): added in v0.4.0, remove gate after 2027-08-14. |
| `pluginLogs` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2064` | COMPAT(pluginLogs): added in v0.4.0, remove gate after 2027-08-16. |
| `pluginGitManagement` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2061` | COMPAT(pluginGitManagement): added in v0.7.0, remove gate after 2027-08-26. |
| `pluginSourceInstallation` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2062` | COMPAT(pluginSourceInstallation): added in v0.8.0; remove gate after 2027-03-16 once daemon floor supports source identifiers. |
| `pluginSourceUpdates` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2063` | COMPAT(pluginSourceUpdates): added in v0.8.0; remove gate after 2027-03-16 once daemon floor supports reviewed updates. |
| `pluginThemes` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2066` | COMPAT(pluginThemes): added in v0.5.0, remove gate after 2027-08-20. A daemon that predates this flag keeps `addTheme` in the server bundle it compiles, so a theme plugin cannot start there at all. |
| `pluginSettings` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2067` |  |
| `pluginTimelineItems` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2068` |  |
| `skillManagement` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2070` | COMPAT(skillManagement): added in v0.4.0, remove gate after 2027-08-16. |
| `idleRuntimeCleanupSetting` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2072` | COMPAT(idleRuntimeCleanupSetting): added in v0.13.0, remove gate after 2027-04-26. |
| `resumeOnStartupSetting` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2073` | COMPAT(restartRecovery): added in v0.14.0, remove gate after 2027-04-28. |
| `terminal-restore-modes` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2075` | COMPAT(terminalRestoreModes): added in v0.1.81, remove gate after 2026-11-23. |
| `terminal-input-mode-replay` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2077` | COMPAT(terminalInputModeReplay): added in v0.2.6, remove gate after 2027-02-02. |
| `terminal-size-ownership` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2079` | COMPAT(terminalSizeOwnership): added in v0.2.6, remove gate after 2027-02-02. |
| `terminal-grapheme-widths` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2081` | COMPAT(terminalGraphemeWidths): added in v0.15.0, remove gate after 2027-05-01. |
| `rewind` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2084` | COMPAT(rewind): added in v0.1.X, drop the gate when floor >= v0.1.X. |
| `agentTimelinePromptIndex` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2086` | COMPAT(agentTimelinePromptIndex): added in v0.2.X, drop the gate when floor >= v0.2.X. |
| `agentHistorySearch` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2088` | COMPAT(agentHistorySearch): added in v0.3.0, remove gate after 2027-02-07. |
| `checkoutRefresh` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2090` | COMPAT(checkoutRefresh): added in v0.1.86, remove gate after 2026-11-29. |
| `workspaceMultiplicity` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2092` | COMPAT(workspaceMultiplicity): added in v0.1.97, drop the gate when floor >= v0.1.97 |
| `projectRemove` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2094` | COMPAT(projectRemove): added in v0.1.97, drop the gate when floor >= v0.1.97. |
| `projectAdd` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2096` | COMPAT(projectAdd): added in v0.1.97, drop the gate when floor >= v0.1.97. |
| `worktreeRestore` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2102` | COMPAT(worktreeRestore): added in v0.1.97, drop the gate when floor >= v0.1.97 |
| `workspaceRecovery` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2104` | COMPAT(workspaceRecovery): added in v0.1.105, remove after 2027-01-11 once daemon floor >= v0.1.105. |
| `workspaceFileEditing` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2106` | COMPAT(workspaceFileEditing): added in v0.2.0, remove after 2027-01-18 once daemon floor >= v0.2.0. |
| `fileByteWrites` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2108` | COMPAT(fileByteWrites): added in v0.13.0, remove gate after 2027-04-20. |
| `providerUsageList` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2110` | COMPAT(providerUsageList): added in v0.1.98, drop the gate when daemon floor >= v0.1.98. |
| `agentDetach` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2112` | COMPAT(agentDetach): added in v0.1.98, remove gate after 2026-12-19 once daemon floor >= v0.1.98. |
| `agentThinkingUpdate` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2114` | COMPAT(agentThinkingUpdate): added in v0.2.4, remove gate after 2027-01-28. |
| `daemonDiagnostics` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2116` | COMPAT(daemonDiagnostics): added in v0.1.100, remove gate after 2026-12-25 once daemon floor >= v0.1.100. |
| `daemonSelfUpdate` | `z.boolean().optional()`; absent unspecified | `this.daemonRuntimeConfig?.desktopManaged !== true` `packages/server/src/server/websocket-server.ts:2118` | COMPAT(daemonSelfUpdate): added in v0.1.93, remove gate after 2026-12-13. |
| `terminalsSurviveRestart` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2120` | COMPAT(terminalsSurviveRestart): added in v0.12.0, remove gate after 2027-04-21. |
| `agentForkContext` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2122` | COMPAT(agentForkContext): added in v0.1.102, remove gate after 2026-12-28. |
| `agentForkContextCursor` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2124` | COMPAT(agentForkContextCursor): added in v0.1.108, remove gate after 2027-01-14. |
| `providerSubagents` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2126` | COMPAT(providerSubagents): added in v0.1.107, remove gate after 2027-01-12. |
| `projectedSubagentTimeline` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2128` | COMPAT(projectedSubagentTimeline): added after v0.8.0, remove gates after 2027-03-14; retain wire field. |
| `wholeTurnTimeline` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2129` |  |
| `providerSubagentNesting` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2131` | COMPAT(providerSubagentNesting): added in v0.7, remove gate after 2027-03-04. |
| `workspacePinning` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2133` | COMPAT(workspacePinning): added in v0.1.107, remove gate after 2027-01-12. |
| `workspaceMarkUnread` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2135` | COMPAT(workspaceMarkUnread): added in v0.5.0, remove after 2027-08-20. |
| `hubRelationship` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2137` | COMPAT(hubRelationship): added in v0.1.X, drop the gate when floor >= v0.1.X. |
| `relayProtectedSecurity` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2006` | COMPAT(relayProtectedSecurity): added in v0.13.0. A capable daemon always advertises it; relay clients that negotiated legacy security reject the connection on seeing it. |
| `projectGithubClone` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2139` | COMPAT(projectGithubClone): added in v0.1.108, remove gate after 2027-01-15. |
| `workspaceGithubRepositorySearch` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2141` | COMPAT(workspaceGithubRepositorySearch): added in v0.1.108, remove gate after 2027-01-15. |
| `projectCreateDirectory` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2143` | COMPAT(projectCreateDirectory): added in v0.1.108, remove gate after 2027-01-15. |
| `projectList` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2098` | COMPAT(projectList): added in v0.2.4, drop the gate when floor >= v0.2.4. |
| `projectSuggestions` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2100` | COMPAT(projectSuggestions): added in v0.14.1, remove gate after 2027-05-01. |
| `commitsList` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2145` | COMPAT(commitsList): added in v0.1.110, remove gate after 2027-01-16. |
| `commitBaseClassification` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2147` | COMPAT(commitBaseClassification): added in v0.2.0, remove gate after 2027-01-23. |
| `providerRemoval` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2149` | COMPAT(providerRemoval): added in v0.1.105, drop the gate when floor >= v0.1.105. |
| `hopliteProvider` | `z.boolean().optional()`; absent unspecified | `this.providerSnapshotManager.supportsConfigRequiredProvider("hoplite")` `packages/server/src/server/websocket-server.ts:2150` | COMPAT(hopliteProvider): added after v0.11.2, remove after 2027-04-15 once host floor includes the bundled adapter. |
| `importSessionWorkspaceTarget` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2152` | COMPAT(importSessionWorkspaceTarget): added in v0.1.110, remove gate after 2027-01-16. |
| `importSessionSearch` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2154` | COMPAT(importSessionSearch): added in v0.8.0, remove gate after 2027-03-02. |
| `forgeProviders` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2157` | COMPAT(forgeProviders): added in v0.2.0-beta.1. Drop the gate after 2027-01-17 once the supported daemon floor is >= v0.2.0. Daemon advertises pluggable non-GitHub forge support (the forge registry); the client gates non-GitHub setup UI on it. |
| `selectiveAgentTimeline` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2159` | COMPAT(selectiveAgentTimeline): added in v0.1.106, remove after 2027-01-12. |
| `explicitEventSubscriptions` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2160` |  |
| `ownedSubscriptions` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2009` |  |
| `binaryChunkingV1` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2010` |  |
| `workspaceSetupObservation` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2022` |  |
| `workspaceSortTimes` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2023` |  |
| `canonicalSubmittedPrompts` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2162` | COMPAT(canonicalSubmittedPrompts): added in v0.2.6, remove gate after 2027-01-30. |
| `agentMessageProvenance` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2163` |  |
| `agentMessageBatching` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2164` |  |
| `agentTurnIdentity` | `z.boolean().optional()`; absent unspecified | not advertised by buildServerInfoStatusPayload | COMPAT(agentTurnIdentity): accept peers that observed pre-release v0.2.6 through 2027-01-31. |
| `stableProjectIdentity` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2166` | COMPAT(stableProjectIdentity): added in v0.1.109, remove gate after 2027-01-15. |
| `workspaceScriptManagement` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2168` | COMPAT(workspaceScriptManagement): added in v0.1.105, remove gate after 2027-01-10. |
| `projectCustomIcon` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2170` | COMPAT(projectCustomIcon): added in v0.2.0, remove after 2027-01-20. |
| `fsEntryOps` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2172` | COMPAT(fsEntryOps): added in v0.3.0, remove gate after 2027-02-08. |
| `fsEntryDuplicate` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2174` | COMPAT(fsEntryDuplicate): added in v0.3.0, remove gate after 2027-02-09. |
| `checkoutDiscardChanges` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2176` | COMPAT(checkoutDiscardChanges): added in v0.3.0, remove gate after 2027-02-08. |
| `checkoutDiffAll` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2177` |  |
| `agentProfiles` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2179` | COMPAT(agentProfiles): added in v0.3.2, remove gate after 2027-02-11. An older daemon parses its persisted config strictly, so writing agentProfiles to one is silently dropped. The client hides the feature rather than letting a save appear to succeed. |
| `agentConfigApply` | `z.boolean().optional()`; absent unspecified | `true` `packages/server/src/server/websocket-server.ts:2181` | COMPAT(agentConfigApply): added in v0.3.2, remove gate after 2027-02-11. |

## Client capabilities and compatibility projections

Capabilities are client declarations in hello, distinct from serverInfo.features. Unknown capability keys survive the hello parser, while server helpers retain only recognized CLIENT_CAPS. Default SDK capabilities are in `packages/client/src/connection/index.ts:127`; per-client config overrides them. The server uses appVersion as a historical fallback for all-provider visibility, but explicit capability declarations work for clients outside the Paseo app release series (`session.ts:299`, `session.ts:2162`).

| Capability constant | Wire spelling | Source intent / compatibility rule |
|---|---|---|
| `terminalChunkedReplay` | `"terminal_chunked_replay"` |  |
| `binaryChunkingV1` | `"binary_chunking_v1"` |  |
| `helloRejection` | `"hello_rejection"` |  |
| `ownedSubscriptions` | `"owned_subscriptions"` | COMPAT(ownedSubscriptions): added in v0.8.0, remove legacy ownership after 2027-03-09. |
| `explicitEventSubscriptions` | `"explicit_event_subscriptions"` | Clients outside the Paseo app version series can explicitly accept provider IDs. COMPAT(explicitEventSubscriptions): added in v0.8.0, remove legacy broadcasts after 2027-03-08. |
| `allProviders` | `"all_providers"` |  |
| `selectiveAgentTimeline` | `"selective_agent_timeline"` | COMPAT(selectiveAgentTimeline): added in v0.1.106. Capable clients receive agent streams only for their explicit viewed set. Remove after 2027-01-12 once the supported client floor is >= v0.1.106. |
| `reasoningMergeEnum` | `"reasoning_merge_enum"` |  |
| `customModeIcons` | `"custom_mode_icons"` | COMPAT(customModeIcons): added in v0.1.84. Old clients pin AgentModeIcon to a closed enum and crash rendering unknown values; daemon downgrades icons outside the legacy set to "ShieldCheck" when this cap is absent. Drop the gate when floor >= v0.1.84. |
| `terminalReflowableSnapshot` | `"terminal_reflowable_snapshot"` | COMPAT(terminalReflowableSnapshot): added in v0.1.88. The daemon attaches per-row soft-wrap flags (gridWrapped/scrollbackWrapped) to terminal snapshots only when the client advertises this, so restored content can reflow on resize. Old clients use a strict TerminalState schema and would reject the extra fields. Drop the gate (always send the flags) when floor >= v0.1.88. |
| `providerSubagents` | `"provider_subagents"` | COMPAT(providerSubagents): added in v0.1.107. The daemon emits provider-owned child descriptors and timelines only to clients that understand the new messages. |
| `projectedSubagentTimeline` | `"projected_subagent_timeline"` | COMPAT(projectedSubagentTimeline): added after v0.8.0, remove gates after 2027-03-14; retain wire capability. |
| `projectUpdates` | `"project_updates"` | COMPAT(projectUpdates): added in v0.1.109, remove gate after 2027-01-15. |
| `compactProviderSnapshots` | `"compact_provider_snapshots"` | COMPAT(compactProviderSnapshots): added in v0.2.X. Capable clients receive provider catalogs with shared thinking sets and may revalidate by content hash. Remove the legacy snapshot encoding after 2027-02-04. |
| `providerSnapshotReferences` | `"provider_snapshot_references"` |  |
| `timelineReplacementInvalidation` | `"timeline_replacement_invalidation"` | COMPAT(timelineReplacementInvalidation): added in v0.5.0, remove legacy reconstructed timeline replay after 2027-02-21 once the client floor supports invalidation. |
| `timelineNotifications` | `"timeline_notifications"` | COMPAT(timelineNotifications): added in v0.7.2. The daemon omits notification timeline items for older clients whose strict timeline union rejects them. Remove after 2027-03-03 once the supported client floor is >= v0.7.2. |
| `pluginTimelineItems` | `"plugin_timeline_items"` | COMPAT(pluginTimelineItems): added in v0.8.0, remove after 2027-03-07 once client floor >= v0.8.0. |
| `workspaceSetupObservation` | `"workspace_setup_observation"` |  |
| `workspaceSetupBlocked` | `"workspace_setup_blocked"` | COMPAT(workspaceSetupBlocked): added in v0.8.0, remove after 2027-03-07 once client floor >= v0.8.0. |
| `browserHost` | `"browser_host"` |  |

Exact default capability set: `{ [CLIENT_CAPS.terminalChunkedReplay]: true, [CLIENT_CAPS.binaryChunkingV1]: true, [CLIENT_CAPS.helloRejection]: true, [CLIENT_CAPS.ownedSubscriptions]: true, [CLIENT_CAPS.allProviders]: true, [CLIENT_CAPS.selectiveAgentTimeline]: true, [CLIENT_CAPS.reasoningMergeEnum]: true, [CLIENT_CAPS.customModeIcons]: true, [CLIENT_CAPS.terminalReflowableSnapshot]: true, [CLIENT_CAPS.providerSubagents]: true, [CLIENT_CAPS.projectedSubagentTimeline]: true, [CLIENT_CAPS.projectUpdates]: true, [CLIENT_CAPS.compactProviderSnapshots]: true, [CLIENT_CAPS.providerSnapshotReferences]: true, [CLIENT_CAPS.timelineReplacementInvalidation]: true, [CLIENT_CAPS.timelineNotifications]: true, [CLIENT_CAPS.pluginTimelineItems]: true, [CLIENT_CAPS.workspaceSetupBlocked]: true, [CLIENT_CAPS.workspaceSetupObservation]: true, [CLIENT_CAPS.explicitEventSubscriptions]: true, } satisfies Record<Exclude<ClientCapability, typeof CLIENT_CAPS.browserHost>, true>`.

## Protocol shared constants referenced by validation

| Symbol | Source | Value |
|---|---|---|
| `AGENT_LIFECYCLE_STATUSES` | `packages/protocol/src/agent-lifecycle.ts:1` | `[ "initializing", "idle", "running", "error", "closed", ] as const` |
| `BROWSER_AUTOMATION_BROWSER_ID_MESSAGE` | `packages/protocol/src/browser-automation/rpc-schemas.ts:18` | `"browserId must be a real id returned by browser_new_tab or browser_list_tabs"` |
| `BROWSER_AUTOMATION_BROWSER_ID_PATTERN` | `packages/protocol/src/browser-automation/rpc-schemas.ts:16` | `/^(?:[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\|\d{13,}-[0-9a-f]+)$/i` |
| `BROWSER_AUTOMATION_COMMAND_NAMES` | `packages/protocol/src/browser-automation/rpc-schemas.ts:23` | `[ "list_tabs", "new_tab", "snapshot", "click", "fill", "wait", "type", "keypress", "navigate", "back", "forward", "reload", "screenshot", "upload", "select", "hover", "drag", "logs", "evaluate", "scroll", "resize", "close_tab", ] as const` |
| `BROWSER_AUTOMATION_WAIT_CONDITION_MESSAGE` | `packages/protocol/src/browser-automation/rpc-schemas.ts:20` | `"browser_wait requires exactly one of text or url"` |
| `BUILTIN_PROVIDER_IDS` | `packages/protocol/src/provider-config.ts:68` | `["claude", "codex", "copilot", "opencode", "pi", "omp"] as const` |
| `CLIENT_CAPS` | `packages/protocol/src/client-capabilities.ts:1` | `{ terminalChunkedReplay: "terminal_chunked_replay", binaryChunkingV1: "binary_chunking_v1", helloRejection: "hello_rejection", ownedSubscriptions: "owned_subscriptions", explicitEventSubscriptions: "explicit_event_subscriptions", allProviders: "all_providers", selectiveAgentTimeline: "selective_agent_timeline", reasoningMergeEnum: "reasoning_merge_enum", customModeIcons: "custom_mode_icons", terminalReflowableSnapshot: "terminal_reflowable_snapshot", providerSubagents: "provider_subagents", projectedSubagentTimeline: "projected_subagent_timeline", projectUpdates: "project_updates", compactProviderSnapshots: "compact_provider_snapshots", providerSnapshotReferences: "provider_snapshot_references", timelineReplacementInvalidation: "timeline_replacement_invalidation", timelineNotifications: "timeline_notifications", pluginTimelineItems: "plugin_timeline_items", workspaceSetupObservation: "workspace_setup_observation", workspaceSetupBlocked: "workspace_setup_blocked", browserHost: "browser_host", } as const` |
| `DAEMON_PERMISSIONS` | `packages/protocol/src/messages.ts:147` | `[ "daemon.read", "daemon.manage", "tunnel.manage", "access.manage", "workspace.read", "workspace.write", "workspace.manage", "automation.manage", "hub.execute", ] as const` |
| `MAX_EXPLICIT_AGENT_TITLE_CHARS` | `packages/protocol/src/agent-title-limits.ts:1` | `200` |
| `MIN_REPOSITORY_PATH_LENGTH` | `packages/protocol/src/messages.ts:2667` | `3` |
| `PROVIDER_ID_PATTERN` | `packages/protocol/src/provider-config.ts:69` | `/^[a-z][a-z0-9-]*$/` |
| `TCP_PORT_RANGE_PATTERN` | `packages/protocol/src/paseo-config-schema.ts:3` | `/^(\d{1,5})-(\d{1,5})$/` |
| `TERMINAL_ACTIVITY_ATTENTION_REASONS` | `packages/protocol/src/terminal-activity.ts:4` | `["finished", "needs_input"] as const` |
| `TERMINAL_ACTIVITY_STATES` | `packages/protocol/src/terminal-activity.ts:3` | `["idle", "working", "attention"] as const` |
| `TOOL_CALL_ICON_NAMES` | `packages/protocol/src/agent-types.ts:201` | `[ "wrench", "square_terminal", "eye", "pencil", "search", "bot", "sparkles", "brain", "mic_vocal", ] as const` |
| `WORKSPACE_LABEL_COLORS` | `packages/protocol/src/workspace-labels.ts:1` | `[ "violet", "sky", "emerald", "orange", "pink", "indigo", "teal", "red", "amber", "blue", ] as const` |

## Connection envelopes, authentication and versioning

Outer envelopes live at `packages/protocol/src/messages.ts:7733`. They are separate from session discriminators even when both use the name ping/pong. JSON session messages are wrapped as `{type:"session",message:<session object>}` by `wrapSessionMessage` (`messages.ts:7840`). There is no outer requestId; correlation lives inside the session message.

| Envelope | Direction | Fields/types | Handler and behavior |
|---|---|---|---|
| hello | C→S | type:"hello"; clientId:string.min(1); clientType:"mobile"\|"browser"\|"cli"\|"mcp"\|"hub"; protocolVersion:int; sessionPermissions?:DaemonPermission[]; routed?:boolean; producer?:{serverId:string[1..256],sourceId:string[1..1024]}; auth?:{kind:"password",password:string}\|{kind:"localCredential",token:string}; appVersion?:string; capabilities?:open object of client capability values | `packages/server/src/server/websocket-server.ts:1740` handleHello; local/remote admission and optional session restrictions; sends status/server_info after acceptance. |
| hello.rejected | S→C | reason:"password_required"\|"incorrect_password"\|"incompatible_protocol"; accepts:["password"] | `websocket-server.ts:1894` rejectHello; sent when auth exists or hello_rejection capability is true, then closes. |
| ping | C→S | type:"ping" only | transport liveness; matched by outer pong, not requestId RPC. `websocket-server.ts` handleRawMessage/handlePendingConnectionMessage; client `daemon-client.ts:2569` sendPingAwaitRtt. |
| pong | S→C | type:"pong" only | `packages/client/src/daemon-client.ts:6960` resolves the outstanding liveness probe and records RTT. |
| recording_state | C→S | type:"recording_state"; isRecording:boolean | Accepted compatibility envelope, ignored by the current server (`websocket-server.ts:2599`); no response. Schema `messages.ts:7781`. |
| session (inbound) | C→S | message:SessionInboundMessage | `packages/server/src/server/websocket-server.ts` validates WSInboundMessageSchema, then `session.ts:2211` handleMessage → request ownership → dispatch. |
| session (outbound) | S→C | message:SessionOutboundMessage | `packages/client/src/daemon-client.ts:6910` handleJsonPayload validates, normalizes and distributes to raw handlers, typed handlers, event listeners, RPC waiters, owned subscriptions. |

Server `WS_PROTOCOL_VERSION=1` (`websocket-server.ts:515`); client sends 1 (`daemon-client.ts:6814`). The server rejects **versions below 1**, rather than enforcing equality (`websocket-server.ts:1747`). HELLO_TIMEOUT_MS=15,000; no hello closes 4001, invalid/pre-hello session closes 4002, incompatible version closes 4003, shutdown uses 1001. Password rejection closes with WS_CLOSE_DAEMON_AUTH_FAILED=4401 (`websocket-server.ts:135`); legacy clients infer auth failure from close reason. Server acceptance is the status/server_info message, not a separate hello.accepted envelope.

The protocol package explicitly says it is **not a stable public API** (`packages/protocol/README.md:5`); all published packages in this checkout are version 0.15.4. Features and capabilities supply incremental compatibility; package semver alone does not negotiate each feature. Pairing `ConnectionOfferV2Schema` uses independent `v:2`, stable home-scoped serverId, daemonPublicKeyB64, relay endpoint/useTls?, and optional minimumSecurityMode:"protected" (`connection-offer.ts:9`). Relay encryption handshakes are implemented in packages/relay, outside packages/protocol, and are not counted as session messages.

## Request correlation, errors and outcome uncertainty

Normal reply names follow request suffix conventions: `.request` → `.response`, `_request` → `_response`, slash RPC → `/response`, otherwise `_response`. Both requestId equality **and originating request type** matter. Server `session/owned-subscriptions/replies.ts:26` declares all nonconforming correlated outputs: pong←ping, agent_archived←archive_agent_request, agent_deleted←delete_agent_request, agent_permission_resolved←agent_permission_response, terminals_changed←subscribe_terminals_request, usage.list_reports.update←usage.list_reports.request, daemon.update.progress←daemon.update.request.

`status` is a multiplexed channel. Its correlated status names are agent_created/agent_create_failed for create, agent_resumed/agent_create_failed for import, agent_resumed for resume, agent_refreshed for refresh, restart_requested for restart, shutdown_requested for shutdown (`replies.ts:40`). Other status payloads can be broadcasts, config/catalog notifications or errors. `transcription_result` belongs to a retained voice operation; checkout_status_update's legacy requestId is a producer marker, not ordinary RPC correlation. Preserve the nested status union from the shared-type dictionary.

| Failure surface | Shape / behavior | Source |
|---|---|---|
| Generic correlated failure | `{type:"rpc_error",payload:{requestId:string,requestType?:string,error:string,code?:string}}` | `packages/protocol/src/messages.ts:3909` |
| Invalid/unknown request schema | rpc_error with invalid_message or unknown_schema when requestId can be recovered; otherwise status `{status:"error",message}` | `packages/server/src/server/websocket-server.ts:2448`; `session/owned-subscriptions/index.ts:213` protocolFailure |
| Authorization | rpc_error code permission_denied for hosts.*; otherwise access_denied; unauthorized uncorrelated input can be ignored | `packages/server/src/server/session.ts:2230` |
| Handler throw | rpc_error code handler_error, error `Request failed: …`, plus activity_log error | `session.ts:2247` |
| Domain failure | Many response payloads carry `error:string\|null`, accepted/success booleans, null result, or structured error codes rather than throwing | Each exact response schema above; client methods differ in whether they throw or return it. |
| Client RPC error | DaemonRpcError: requestId, requestType?, code?, serverId?, userMessage; Error.message also includes correlation metadata | `packages/client/src/daemon-client.ts:1070` |
| Invalid correlated response | DaemonProtocolError with code invalid_response, requestId, responseType?; corresponding waiter rejected | `daemon-client.ts:1096`, `daemon-client.ts:6930` |
| Connection/timeout | DaemonConnectionError, code DAEMON_CONNECTION_LOST or DAEMON_REQUEST_TIMEOUT; serverId? and outcome?:"unknown" | `daemon-client.ts:1053`; submitted requests that lose transport are not proof of server failure/rollback (`daemon-client.ts:7441`). |
| Authentication | DaemonAuthError reason password_required/incorrect_password; incompatible protocol is connection error, not password failure | `daemon-client.ts:249`; outer hello.rejected handler |
| Size/transport | Oversize logical messages fail before send or close 1009 on daemon; malformed chunk framing throws ChunkedTransportProtocolError; invalid host route closes 1008 | `packages/protocol/src/websocket-message-limit.ts:2`, `chunked-transport.ts:127`, `server/websocket-server.ts:2468` |

Incoming JSON parse failures are discarded client-side. Invalid outbound schema is logged; an identifiable correlated response rejects its waiter, so the caller does not wait for the full timeout. No universal transaction, replay or retry guarantee follows from requestId. Creation operations add idempotency explicitly; ordinary mutation callers must account for unknown outcomes.

## Subscription ownership, directory sync and reconnection

Server `SessionDelivery` (`packages/server/src/server/session/owned-subscriptions/index.ts:65`) tracks each physical source/socket, not just a clientId. Modern ownership requires both serverInfo.features.ownedSubscriptions and hello capability owned_subscriptions. RPC replies inherit request provenance; subscriptions/retained operations own future emissions. `subscription.release.request` releases a source-owned registration; source detach aborts its cancellation signal and releases all subscriptions/operations (`index.ts:176`). A reply or event must not leak to another socket that shares a session.

Client `OwnedSubscription<T>` (`packages/client/src/connection/owned.ts:13`) has current subscriptionId|null, ready:Promise<T & {subscriptionId}>, subscribe(observer) and release():Promise<void>. Observers receive authoritative snapshot first, then update messages, and optional error. Reconnect changes daemon IDs while keeping the local handle stable. Client generation counters discard stale completions from the old connection. Release is idempotent, removes routes/listeners, waits for pending establishment and releases any late-acquired daemon ID. API dispose/AbortSignal releases owned handles.

MAX_BOOTSTRAP_UPDATES=128 (`owned.ts:35`). Updates can be buffered for later local observer attachment; overflow invalidates the cached snapshot and releases/restarts the observation before a new observer relies on it. Establishment failure rejects ready and releases the handle. A transport failure with unknown bootstrap outcome can reset the source instead of accidentally reissuing an ambiguous request (`daemon-client.ts:2688`). `ConnectionSubscriptions` (`connection/index.ts:17`) adapts old hosts: local legacy IDs are not proof of daemon membership, and readiness on broadcast-era event streams means only that the local listener is attached.

| Subscription family | Bootstrap / delta | Lifetime / compatibility |
|---|---|---|
| Agents | fetch_agents_request with subscribe; fetch_agents_response snapshot; agent_update deltas. Filter/scope/sort/page/sync live in the exact schema. | observeAgents / agents.list({subscribe:{}}); mutation membership and directory sync are server-owned. |
| Workspaces | fetch_workspaces_request with subscribe; fetch_workspaces_response; workspace_update. | observeWorkspaces; background visibility is explicitly gated. |
| Workspace labels | workspace.label.list.request with subscribe; label list response and workspace.label.update. | observeWorkspaceLabels. |
| Session events | session.events.set_subscription.request events[] + notifications?; response subscriptionId; listed events only, no directory bootstrap. | Multiple owned observers can coexist; legacy explicit event selection is adapted. |
| Timeline | agent.timeline.set_subscription.request agentIds[]; response membership; agent_stream and agent.timeline.replacement. | subscribeAgentTimeline/observeTimeline; release membership; selective legacy timeline state is adapted. |
| Workspace setup | workspace.setup.subscribe.request workspaceId; response current snapshot; workspace_setup_progress. | Requires workspaceSetupObservation and owned subscriptions. |
| Files | fs.file.subscribe.request cwd/path; version snapshot and fs.file.update. | Owned observeFile; legacy explicit fs.file.unsubscribe supported. Content reads/writes remain separate. |
| Checkout diff | subscribe_checkout_diff_request cwd/compare per schema; snapshot + checkout_diff_update. | Owned observation; legacy unsubscribe_checkout_diff_request retained. |
| Terminal directory | subscribe_terminals_request; terminals_changed is both initial correlated snapshot and later delta/list state. | observeTerminals; legacy unsubscribe_terminals_request. |
| Terminal stream | subscribe_terminal_request; response assigns slot/subscriptionId; subsequent binary replay/output; terminal_stream_exit. | Slot is bound to source; owned release detaches stream; legacy unsubscribe_terminal_request. |
| Browser hosting | browser.host.register.request hostKind/supportedCommands; registration subscriptionId. | Reverse automation RPC delivered to that owning host; browser-host lease is revoked on permission loss. |
| Creation | create requests with subscribe:true; workspace.create.update/agent.create.update snapshots; creation.subscribe for reconnect recovery. | idempotencyKey + revision; independent of UI callback lifetime. |
| Hosts | hosts.list.request observe:true; hosts.changed. | observeHosts; route/link channels have their own scope/lifetime. |

The event names accepted by session.events.set_subscription are the exact SessionEventSubscriptionSchema values in the dictionary (project/provider/attention/permission/checkout/script/setup/subagent/terminal/status/activity/Hub events). An SDK local event callback alone is not daemon demand. Notification interest is separately declared by notifications.

Timeline live subscriptions **do not fetch missed history**. On re-establishment, the client emits local agent.timeline.subscription_restored before subsequent updates; use fetchAgentTimeline with cursor/projection to close gaps. A replacement carries a new epoch and invalidates previous timeline identity. The local agent.timeline.error stops/releases observation. These two local lifecycle message names are not additional wire members (`packages/client/src/timeline-subscription/index.ts:9`). Preserve source sequence ranges, collapse reasons and turn identity fields when porting projections.

CreationClient (`packages/client/src/creation/index.ts:56`) fingerprints intent by kind/idempotencyKey excluding requestId, rejects conflicting reuse, observes monotonically revised snapshots, and on reconnect subscribes to authoritative operation state before resubmitting a missing operation. Old agent creation uses create_agent_request/status; legacy workspaces can be created then followed by agent creation. Caller-selected IDs and newer background semantics require supported hosts. UI onEvent callbacks do not advance or cancel server creation.

## Binary terminal, file and host streams

The binary demultiplexer is `packages/protocol/src/binary-frames/demux.ts:21`. These are **12 application opcodes plus 1 chunk-framing opcode**, outside the 457 JSON session messages. Integers below are bytes except stated uint16 lengths (network/big-endian, DataView default). Text is UTF-8. Invalid decoders return null; consumers decide whether to ignore, reject or close.

| Opcode/name | Direction | Frame layout / typed payload | Feature / server and client handler |
|---|---|---|---|
| 0x01 Output | S→C | opcode:u8, slot:u8, remaining bytes:ANSI/UTF-8 terminal output | terminal-session-controller.ts:938 → client terminal-stream-router.ts handleFrame |
| 0x02 Input | C→S | opcode:u8, slot:u8, UTF-8 input bytes | terminal-session-controller.ts:244; accepted only for an active slot owned by that source |
| 0x03 Resize | C→S | opcode:u8, slot:u8, JSON `{rows:positive int,cols:positive int,intent?:"claim"\|"update"}` | binary-frames/terminal.ts:11; terminal-session-controller.ts:256 applies size ownership |
| 0x04 Snapshot | S→C | opcode:u8, slot:u8, JSON TerminalStateSchema | retained grid/scrollback compatibility snapshot; client decoder validates full schema |
| 0x05 Restore | S→C | opcode:u8, slot:u8, ANSI restore bytes | terminal restore delivery; router restores before live tail |
| 0x06 ReplayBegin | S→C | opcode:u8, slot:u8, JSON `{rows:positive int,cols:positive int,retainedHistoryLines:nonnegative int}` | chunked terminal replay metadata |
| 0x07 ReplayData | S→C | opcode:u8, slot:u8, raw ANSI replay bytes | replay data delivered in bounded chunks |
| 0x08 ReplayEnd | S→C | opcode:u8, slot:u8, empty payload | replay completed; apply buffered live tail |
| 0x10 FileBegin | Both codec; S→C reads/C→S upload path | opcode:u8, requestIdLength:u8 (1..255 UTF-8 bytes), requestId bytes, metadataLength:u16, JSON metadata | mime:nonempty string,size:nonnegative int,encoding:"utf-8"\|"binary",modifiedAt:string,revision?:string,fileName?:string; binary-frames/file-transfer.ts:13; session/files/workspace-files-session.ts |
| 0x11 FileChunk | Both | opcode:u8, requestIdLength:u8, requestId bytes, remaining raw bytes | ordered chunks correlated by requestId; file reads stream server→client, uploads handled server-side |
| 0x12 FileEnd | Both | opcode:u8, requestIdLength:u8, requestId bytes; no body | client verifies total received byte count equals declared size before resolving read |
| 0x30 HostRoute | Both | opcode:u8, routeIdLength:u16, UTF-8 routeId, enclosed unmodified binary frame | routeId 1..256 bytes; routes binary without base64/JSON re-encoding; server connected-host session |
| 0x31 ChunkedTransport | Both, negotiated | opcode=0x31, version=1, kind=0 text/1 binary, flags bit0=start/bit1=end, payload | Ordered connection-scoped logical framing below serialization/routing and above encryption (`chunked-transport.ts:102`). |

Terminal slots are 0..255, max MAX_TERMINAL_STREAM_SLOTS=256 (`packages/server/src/terminal/terminal-session-controller.ts:44`). A stream maps to terminalId + owner source; a slot alone is not globally addressable. Replay captures a revision cut, buffers live output and discards live revisions already covered by the cut. Chunk replay defaults to TERMINAL_REPLAY_CHUNK_BYTES=64 KiB (`terminal-replay.ts:24`), gated by terminal_chunked_replay. Terminal restore choices/scrollback/input modes/size ownership remain in subscribe schema and capability flags; do not equate snapshot restoration with a new shell process.

For file reads, file_explorer_request(mode:"file",acceptBinary:true,maxBytes?) triggers FileBegin/Chunk/End instead of embedding large content. The client tracks pending/active/completed transfers and synthesizes a file_explorer_response to resolve its ordinary RPC waiter (`daemon-client.ts:7063`). Incomplete size rejects; an old host ignoring maxBytes is bounded by the client discarding over-budget transfer accumulation (`daemon-client.ts:7079`). File writes carry expectedModifiedAt/expectedRevision for conflict detection and optional contentEncoding base64; file upload also has JSON admission/control messages. Paths are interpreted on the daemon host.

HostRoute imposes MAX_HOST_ROUTE_ID_BYTES=256, MAX_HOST_ROUTE_HEADER_BYTES=259, MAX_ROUTED_PAYLOAD_BYTES=100 MiB−259, and MAX_HOST_CHANNEL_FRAME_BYTES=floor((32 MiB−259)/6) (`binary-frames/host-route.ts:4`). JSON text routes use hosts.route.frame messages; binary takes opcode 0x30. A routed host client sends its own hello/session envelopes through that channel and owns a separate logical connection. Host channel close and parent transport disconnect release the route (`packages/client/src/hosts/index.ts:39`).

Chunk framing is negotiated by binary_chunking_v1/server binaryChunkingV1. MAX_WEBSOCKET_MESSAGE_BYTES=100 MiB; relay encrypted frame max is 32 MiB (`websocket-message-limit.ts:2`). The client transport default physical frame budget is 32 MiB; host channels use their smaller expansion-safe budget. ChunkedTransport permits one ordered assembly; interleaving a normal frame, changed kind, repeated start/missing start, unsupported version/flags, or excess logical size fails the connection. Logical output queues share the 100 MiB byte limit; drain-capable sends serialize chunks. Connection close discards partial assembly.

## Client defaults and limits to preserve

| Constant / scope | Value / effect | Source |
|---|---|---|
| DEFAULT_SESSION_RPC_TIMEOUT_MS | 60,000 ms unless method override | packages/client/src/daemon-client.ts:1125 |
| DEFAULT_CONNECT_TIMEOUT_MS | 15,000 ms | daemon-client.ts:1127 |
| DEFAULT_RECONNECT_BASE_DELAY_MS / MAX_DELAY_MS | 1,500 / 30,000 ms with reconnect scheduling; CLI explicitly disables reconnect | daemon-client.ts:1123; packages/cli/src/utils/client.ts:203 |
| DEFAULT_LIVENESS_TIMEOUT_MS | 5,000 ms for measurement default | daemon-client.ts:1128 |
| LIVENESS_HEARTBEAT_INTERVAL_MS / TIMEOUT_MS | 10,000 / 15,000 ms; separate from app client_heartbeat presence | daemon-client.ts:1129 |
| PROJECT_GITHUB_CLONE_TIMEOUT_MS | 5 min | daemon-client.ts:212 |
| PUSH_TOKEN_REVOCATION_TIMEOUT_MS | 2,000 ms | daemon-client.ts:1126 |
| Low-level waitForFinish | default 60,000 ms; 0 means no limit | daemon-client.ts:6378 |
| Public SDK handle wait | 10 min (DEFAULT_WAIT_FOR_FINISH_MS) | packages/client/src/index.ts:69 |
| CLI run/send/wait | default no wait limit; --no-wait returns after dispatch/create; explicit duration parsed by shared utility | packages/cli/src/commands/agent/run.ts:309, send.ts, wait.ts:62 |
| Dictation finish | acceptance timeout 60 s; old-host fallback final timeout 5 min; accepted max-processing budget + 5 s grace | daemon-client.ts:1135, finishDictationStream |
| MAX_BOOTSTRAP_UPDATES | 128 local buffered updates before rebootstrap | packages/client/src/connection/owned.ts:35 |
| EXTERNAL_SESSION_DISCONNECT_GRACE_MS | 90,000 ms before external disconnected session cleanup | packages/server/src/server/websocket-server.ts:509 |
| CLI hook input/post deadlines | 100 ms per stdin next; HTTP POST abort after 500 ms; errors ignored | packages/cli/src/commands/hooks.ts:63, hooks.ts:99 |
| CLI daemon deadlines | start/restart 600 s; stop graceful 15 s and forced 3 s | packages/cli/src/commands/daemon/local-daemon.ts:72; stop.ts:15 |

## Reference platform and storage boundaries

The JSON protocol is shared across web, desktop, mobile, CLI, MCP and Hub clients; hello.clientType selects identity/context, while capabilities select safe payload projections. Node-only CLI connectors can use system ssh, local daemon home files and process/supervisor APIs; browser/mobile cannot assume these APIs. Native desktop project/agent open is an OS deep-link flow, not a session RPC. `paseo agent open` returns an existing agent link and can use --server without a local lookup (`packages/cli/src/commands/agent/open.ts`). Windows process/signal and path handling are implemented in daemon-control/connector modules; this static survey does not claim those OS paths were executed.

| Shared entity / durable boundary | Transport connection |
|---|---|
| Daemon identity/config | `<PASEO_HOME>/config.json`, instance/readiness metadata, local credential, CLI client ID; config/get/patch/reload and local daemon config are distinct surfaces. |
| Projects/workspaces | Registry files `<PASEO_HOME>/projects/projects.json` and `projects/workspaces.json` (`packages/server/src/server/bootstrap.ts:942`, `:946`); workspace-registry.ts supplies mutations; directory subscriptions project derived snapshots. |
| Agent/session/timeline | Agent manager/provider persistence/storage and timeline sequence/epoch projections are server-owned. Client refs/cache/listeners are not durable storage. The protocol carries IDs, persistence metadata, turn state and attention/permission state. |
| Terminal | Terminal worker owns PTY/process/replay state and may outlive a daemon restart; JSON lifecycle/owned subscriptions attach viewers to it. Retaining a terminal requires explicit advertised capability. |
| Checkout/files | Real daemon-host filesystem and Git/forge state; diff/status/file subscriptions watch external changes, while mutations emit updates. RPC acknowledgement does not imply other clients have consumed those updates. |
| Schedule/heartbeat/chat/loop | Schedules/heartbeats have server-managed timers/runs/targets; their exact persistence backend was not traced here. Chat/loop have declared message schemas and permissions but no dispatcher/storage implementation was found in this checkout. |
| Plugin | Daemon plugin runtime/installations, source revisions, client bundle hash cache, plugin settings and skill reconciliation; catalog updates notify clients to load compatible bundles. |
| Hub/connected host | CLI credential store, daemon Hub authority relationship and per-source host registrations/routes have separate ownership. Destination serverId must stay attached to agent/workspace identity. |

## CLI runtime defaults and validation details

`run --provider` is syntactically optional in Commander but **required at runtime**; there is no fallback provider in this command (`packages/cli/src/commands/agent/run.ts:602`, `utils/provider-model.ts:14`). Provider/model shorthand splits on the first slash; supplying a conflicting --model throws CONFLICTING_MODEL_OPTIONS. Schedule new-agent creation also requires a provider at runtime. Import has an explicit required provider option.

`run` uses --title before hidden --name; omitted cwd is the CLI current directory. Omitted workspace uses the calling agent's workspace when agent-scoped, otherwise the creation placement path; --new-workspace explicitly isolates. Worktree placement defaults to branch-off; source-specific options are validated and incompatible combinations rejected. Omitted --wait-timeout means 0/no limit. --output-schema accepts inline JSON or a schema file, forces structured JSON output and retries validation up to maxRetries=2; --no-wait is incompatible with needing a final schema response (`agent/run.ts:239`). Ordinary timeout/permission/error result statuses are not process errors; structured-output failures are.

`send` accepts one prompt source: positional prompt, --prompt or --prompt-file; image attachments may accompany it. Behavior defaults to steer when PASEO_AGENT_ID identifies a caller and interrupt otherwise (`agent/send.ts:207`); --behavior overrides it. --no-wait reports queued/sent receipt without waiting for an idle result. There is no --wait-timeout flag on send. `wait --timeout` uses the shared duration parser despite its <seconds> help metavar, and defaults to unlimited. Its optional activity preview is last 5 entries with a 2 s fetch timeout (`agent/wait.ts:39`).

`logs` non-follow mode fetches history through cursors and prints a human transcript; --tail=0 prints nothing, invalid negative/non-numeric tails fail. Follow defaults to last 10 entries; --filter values tools/text/errors/permissions and --since affect history/live timeline output; Ctrl+C detaches. `attach` prints history, then stream events, with replacement/reconnect notices. Neither implements JSON/YAML output even though global flags parse. An initial history-read error can warn and still attach to live events.

`ls` defaults to the current directory scope and excludes archived/background agents; --all removes the directory restriction and includes archived/background; --global changes directory scope; --background only includes background. Label filters are repeatable key=value and --thinking filters effective thinking option. These are listing filters, not provider launch options (`agent/ls.ts:137`).

Workspace create defaults --mode branch-off for worktree isolation, inherits background visibility unless explicitly overridden, and may infer the source from --project. Local project/workspace path convenience is interpreted by CLI versus daemon according to target; schedule/project creation rejects missing remote cwd/path. `workspace rename` and `project rename` require a supplied title/name or --reset. Terminal create opens the cwd as a workspace if no --workspace is supplied; terminal list defaults to current directory unless --all or --workspace. Terminal capture --scrollback sets start=0 and overrides --start; --ansi toggles stripAnsi=false (`terminal/capture.ts:68`).

Schedule creation requires exactly one cadence (--every or --cron), timezone defaults UTC, runOnCreate defaults false, and maxRuns must be positive integer. --expires-in becomes an absolute ISO expiresAt. Heartbeat update only changes cadence/timezone; it requires --cron and validates target ownership. Hub origin resolution is explicit --hub/origin → PASEO_HUB_URL → active stored origin → https://hub.paseo.sh (`commands/hub/authority.ts:16`); credentials resolve separately and are not printed in this survey.

Terminal backpressure constants (`packages/server/src/terminal/terminal-restore.ts:10`): MAX_TERMINAL_OUTPUT_FRAME_BYTES=256 KiB, MAX_CLIENT_BUFFERED_BYTES=4 MiB, DEFAULT_VISIBLE_RESTORE_SCROLLBACK_LINES=200, MAX_VISIBLE_RESTORE_SCROLLBACK_LINES=500. Overflow catches a slow viewer up through a visible snapshot rather than blocking every observer. These are application-layer limits in addition to wire frame budgets.

## Declared protocol messages excluded from active unions

These two named message schemas remain exported but are **not members** of SessionInboundMessageSchema or SessionOutboundMessageSchema. They are not aliases accepted by the current connection validator and must not be implemented as active wire commands merely because a declaration exists. Only their definitions were found by searching packages/protocol/src, packages/client/src and packages/server/src non-test sources.

| Message | Intended direction | Fields/types | Registration / server handler |
|---|---|---|---|
| send_agent_message | Historical C→S intent | `{type:"send_agent_message",agentId:string,text:string,messageId?:string,activeTurnBehavior?:"interrupt"\|"steer",images?:ImageAttachment[],attachments:AgentAttachmentsSchema}` | `packages/protocol/src/messages.ts:1257` SendAgentMessageSchema; no active-union entry or server handler found. Current path is send_agent_message_request/response. |
| agent_list | Historical S→C snapshot | `{type:"agent_list",payload:{agents:AgentSnapshotPayload[]}}` | `packages/protocol/src/messages.ts:4271` AgentListMessageSchema; no active-union entry or server emitter found. Current directory path is fetch_agents_response + agent_update. |

`assistant_chunk`, `agent_status` and `artifact`, unlike the above two, **are active outbound-union members**, but no current server emission was found in packages/server/src non-test sources. Their exact schemas remain catalogued. This is a source finding, not proof that a plugin or older peer can never produce them.

## Static coverage audit and limits

- Source commit rechecked: `1c00b365c9e0b74c1580eba33d905d693125ae21`; `git status --short` in the reference checkout was empty after the survey. Only this notes artifact and temporary extraction/audit files outside the source checkout were written.
- CLI static AST audit: 105 action sites (104 chained registrations + usage's variable-bound action); expanding the three dynamic loops adds 6 paths, root daemon factory reuse adds 5, and plugin install's add spelling adds 1: **117 executable invocation paths**. This includes hidden compatibility paths and aliases. There are **18 named group nodes**, including executable usage; 17 are discovery-only and are not included in the executable-path count. Root `paseo`, automatic help/version, and directory-open invocation syntax are separate parser forms.
- Protocol membership audit: **219 inbound + 238 outbound = 457 distinct active session discriminators**, each with a field contract and server dispatch/emission reference or explicit not-found statement. Two additional message schemas are declared outside those unions. Seven directional outer-envelope schemas (six unique envelope names) and 13 binary/framing opcodes are documented separately; these categories are not silently added to the session-message total.
- Client coverage: **231 distinct public DaemonClient member names** (overload signatures deduplicated; includes getters), **67 public SDK method/handle entries**, plus Node connector exports. **101 server feature keys**, **21 client capability keys**, **331 reachable shared payload schemas**, and 16 validation constants are indexed.
- Read-only evidence only: no builds, test suites, daemon/CLI actions, network sessions, runtime mutation, permission changes, or inter-agent messages. Cross-platform/runtime behavior is source-derived. Exact schedule storage and other areas' UI state transitions were not traced here; chat/loop handlers/storage were not found.
- Declared-but-unemitted active outputs: assistant_chunk, agent_status, artifact. Chat has 7 request/7 response variants and loop has 5 request/5 response variants in the active unions but no dispatch or response producer was found; server occurrences are permission-table entries only. Declared-but-unregistered message schemas: send_agent_message, agent_list. No unsupported wire behavior is inferred from those declarations.

## Cross-area connections

| Other area / shared entity | Connection from CLI, protocol and client | Source entry point |
|---|---|---|
| Host connection / authentication | CLI selectors and Node transport resolve home/socket/TCP/SSH/relay offers; hello admits client identity, restrictions and capability projection. Inner routed clients keep destination identity. | packages/cli/src/utils/client.ts:181; packages/client/src/node/index.ts:205; packages/server/src/server/websocket-server.ts:1740 |
| Sidebar / directory state | Agent/workspace filtered directory observations bootstrap and maintain sidebar membership, background visibility and sync cursors; project events feed project registry state. | packages/app/src/runtime/directory-sync/index.ts:313, :620, :724 |
| Agent creation / workspace placement | CLI run/workspace create and SDK creations share idempotency receipts, setup progress and background intent; profiles/providers supply runtime config. | packages/client/src/creation/index.ts:56; packages/cli/src/commands/agent/run.ts:592 |
| Agent chat / timeline | Send/wait/logs/attach map to receipt, turn, projection, history and live events; reconnect resumes observation but consumer repairs gaps; epoch replacement invalidates cached history. | packages/client/src/timeline-subscription/index.ts:23; packages/cli/src/commands/agent/logs.ts:328 |
| Permissions / questions / attention | Permission requests/resolutions are separate from normal turn output; permit allow/deny changes pending permissions; wait returns permission state; app notification routing uses focus and event demand. | packages/server/src/server/session.ts:2930; packages/app/src/data/push-router.ts:284 |
| Terminal UI / supervised processes | CLI terminal uses SDK handles; app terminal controller uses owned binary observations; terminal worker lifetime can survive daemon replacement; scripts expose associated terminal IDs. | packages/app/src/terminal/runtime/terminal-stream-controller.ts:80; packages/server/src/terminal/terminal-session-controller.ts:232 |
| Workspace scripts / service routes | Script list/start/stop status includes health, ports and proxy URLs; script_status_update feeds directory/runtime routes. | packages/app/src/runtime/directory-sync/index.ts:327; packages/cli/src/commands/script/index.ts:18 |
| Files / attachments / editor | File subscriptions/version checks/binary read/upload/download tokens connect filesystem UI and message attachments; OS open/deep-link is separate from remote file RPC. | packages/server/src/server/session/files/workspace-files-session.ts:377; packages/cli/src/commands/agent/open.ts:27 |
| Checkout / forge / change requests | Git status/diff watches, commit/branch/stash/pull/push/merge/PR/check-detail RPCs share cwd/project placement; feature flags choose forge-aware versus legacy GitHub fallback. | packages/server/src/server/session.ts:2999 dispatchCheckoutMessage |
| Provider settings / usage | Provider snapshot compact/hash references, feature discovery, modes/models, runtime config and account usage flow through distinct typed methods; quota reports differ from agent token usage. | packages/client/src/provider-snapshots/index.ts; packages/cli/src/commands/usage/index.ts:23 |
| Plugins / themes / skills | Install/review/update/runtime RPC, catalog bundle hashes, status notifications and skills reconciliation share plugin provenance; app subscribes to catalog changes. | packages/app/src/plugins/catalog-sync-model.ts:51; packages/app/src/screens/settings/plugins-page.tsx:324 |
| Speech / dictation | Voice audio, dictation stream states and local speech models share config patches; CLI speech is empty despite these protocol/client operations. | packages/server/src/server/session/voice/voice-session.ts; packages/protocol/src/speech/rpc-schemas.ts |
| Browser automation | Browser hosts register supported commands under owned leases; daemon sends reverse RPC execution to the host, which responds through the client. | packages/server/src/server/session.ts:2351; packages/client/src/daemon-client.ts:2698 |
| Scheduling / orchestration | Schedule/heartbeat implementations use agent targets/run lifecycle; chat/loop schemas describe related orchestration intent but lack current server dispatch. Hidden schedule --target still supports self/existing agent. | packages/cli/src/commands/schedule/shared.ts:102; packages/protocol/src/chat/rpc-schemas.ts; packages/protocol/src/loop/rpc-schemas.ts |
| Hub / remote execution | CLI login/export/deploy uses Hub HTTP; daemon relationship/connect/permissions uses session RPC; Hub execution requests/events manage remote agent work independently. | packages/cli/src/commands/hub/index.ts:56; packages/server/src/server/hub |
| Desktop / web / mobile shells | Shared protocol and typed client are transport-neutral; Node process/home/SSH and native desktop deep-link entry points need platform adapters. | packages/cli/src/classify.ts:51; packages/cli/src/commands/open.ts; packages/protocol/src/messages.ts:7741 |
| Daemon settings / lifecycle / diagnostics | Local config edits differ from remote mutable config RPC; reload and restart expose distinct guarantees; diagnostics include connection/subscription/runtime state. | packages/cli/src/commands/daemon/config.ts:75; packages/server/src/server/session/daemon/daemon-session.ts |

## Inventory

Counts: 117 executable CLI paths; 18 named command groups; 457 active session messages; 2 unregistered declared messages; 7 directional outer envelopes; 13 binary/framing opcodes; 67 SDK entries; 231 low-level public members. The list is flat; payload schemas are supporting types rather than extra messages.

- CLI: `paseo ls`
- CLI: `paseo run`
- CLI: `paseo import`
- CLI: `paseo clone`
- CLI: `paseo attach`
- CLI: `paseo logs`
- CLI: `paseo stop`
- CLI: `paseo delete`
- CLI: `paseo send`
- CLI: `paseo inspect`
- CLI: `paseo wait`
- CLI: `paseo archive`
- CLI: `paseo agent ls`
- CLI: `paseo agent run`
- CLI: `paseo agent import`
- CLI: `paseo agent attach`
- CLI: `paseo agent logs`
- CLI: `paseo agent open`
- CLI: `paseo agent stop`
- CLI: `paseo agent delete`
- CLI: `paseo agent send`
- CLI: `paseo agent inspect`
- CLI: `paseo agent wait`
- CLI: `paseo agent mode`
- CLI: `paseo agent archive`
- CLI: `paseo agent reload`
- CLI: `paseo agent detach`
- CLI: `paseo agent update`
- CLI: `paseo daemon config get [path]`
- CLI: `paseo daemon config set <path> <value>`
- CLI: `paseo daemon config unset <path>`
- CLI: `paseo daemon set-password`
- CLI: `paseo daemon pair`
- CLI: `paseo daemon reload`
- CLI: `paseo daemon restart`
- CLI: `paseo daemon start`
- CLI: `paseo daemon run`
- CLI: `paseo daemon status`
- CLI: `paseo daemon stop`
- CLI: `paseo heartbeat create`
- CLI: `paseo heartbeat update`
- CLI: `paseo heartbeat delete`
- CLI: `paseo hooks`
- CLI: `paseo host ls`
- CLI: `paseo host add`
- CLI: `paseo host update`
- CLI: `paseo host inspect`
- CLI: `paseo host enable`
- CLI: `paseo host disable`
- CLI: `paseo host rm`
- CLI: `paseo hub connect`
- CLI: `paseo hub deploy`
- CLI: `paseo hub disconnect`
- CLI: `paseo hub export`
- CLI: `paseo hub status`
- CLI: `paseo hub init`
- CLI: `paseo hub login`
- CLI: `paseo hub logout`
- CLI: `paseo hub permissions list`
- CLI: `paseo hub permissions grant`
- CLI: `paseo hub permissions revoke`
- CLI: `paseo hub projects`
- CLI: `paseo onboard`
- CLI: `paseo permit ls`
- CLI: `paseo permit allow`
- CLI: `paseo permit deny`
- CLI: `paseo plugin init`
- CLI: `paseo plugin ls`
- CLI: `paseo plugin status`
- CLI: `paseo plugin logs`
- CLI: `paseo plugin install`
- CLI: `paseo plugin update`
- CLI: `paseo plugin reload`
- CLI: `paseo plugin enable`
- CLI: `paseo plugin disable`
- CLI: `paseo plugin remove`
- CLI: `paseo project create`
- CLI: `paseo project ls`
- CLI: `paseo project rename`
- CLI: `paseo project delete`
- CLI: `paseo provider ls`
- CLI: `paseo provider models`
- CLI: `paseo provider diagnostic`
- CLI: `paseo schedule create`
- CLI: `paseo schedule ls`
- CLI: `paseo schedule inspect`
- CLI: `paseo schedule logs`
- CLI: `paseo schedule pause`
- CLI: `paseo schedule resume`
- CLI: `paseo schedule delete`
- CLI: `paseo schedule run-once`
- CLI: `paseo schedule update`
- CLI: `paseo script ls`
- CLI: `paseo script start`
- CLI: `paseo script stop`
- CLI: `paseo terminal ls`
- CLI: `paseo terminal create`
- CLI: `paseo terminal kill`
- CLI: `paseo terminal capture`
- CLI: `paseo terminal send-keys`
- CLI: `paseo usage ls`
- CLI: `paseo usage inspect`
- CLI: `paseo workspace setup`
- CLI: `paseo workspace create`
- CLI: `paseo workspace ls`
- CLI: `paseo workspace rename`
- CLI: `paseo workspace archive`
- CLI: `paseo worktree ls`
- CLI: `paseo worktree create`
- CLI: `paseo worktree archive`
- CLI: `paseo pair`
- CLI: `paseo reload`
- CLI: `paseo restart`
- CLI: `paseo start`
- CLI: `paseo status`
- CLI: `paseo usage`
- CLI: `paseo plugin add`
- Command group: `agent`
- Command group: `daemon`
- Command group: `daemon config`
- Command group: `host`
- Command group: `hub`
- Command group: `hub permissions`
- Command group: `terminal`
- Command group: `script`
- Command group: `schedule`
- Command group: `heartbeat`
- Command group: `permit`
- Command group: `provider`
- Command group: `usage`
- Command group: `plugin`
- Command group: `speech`
- Command group: `project`
- Command group: `workspace`
- Command group: `worktree`
- Session C→S: `hosts.list.request`
- Session C→S: `hosts.add.request`
- Session C→S: `hosts.update.request`
- Session C→S: `hosts.remove.request`
- Session C→S: `hosts.link.open.request`
- Session C→S: `hosts.route.open.request`
- Session C→S: `hosts.route.frame.request`
- Session C→S: `hosts.route.close.request`
- Session C→S: `browser.host.register.request`
- Session C→S: `subscription.release.request`
- Session C→S: `session.events.set_subscription.request`
- Session C→S: `hub.execution.agent.create.request`
- Session C→S: `hub.execution.agent.validate.request`
- Session C→S: `hub.execution.control.request`
- Session C→S: `browser.automation.execute.response`
- Session C→S: `voice_audio_chunk`
- Session C→S: `abort_request`
- Session C→S: `audio_played`
- Session C→S: `fetch_agents_request`
- Session C→S: `fetch_agent_history_request`
- Session C→S: `fetch_recent_provider_sessions_request`
- Session C→S: `fetch_workspaces_request`
- Session C→S: `project.list.request`
- Session C→S: `project.suggestions.request`
- Session C→S: `fetch_agent_request`
- Session C→S: `delete_agent_request`
- Session C→S: `archive_agent_request`
- Session C→S: `close_items_request`
- Session C→S: `update_agent_request`
- Session C→S: `project.rename.request`
- Session C→S: `project.icon.set.request`
- Session C→S: `project.remove.request`
- Session C→S: `workspace.title.set.request`
- Session C→S: `workspace.pin.set.request`
- Session C→S: `workspace.label.list.request`
- Session C→S: `workspace.label.assignment.set.request`
- Session C→S: `workspace.label.update.request`
- Session C→S: `workspace.label.delete.request`
- Session C→S: `workspace.label.delete.inspect.request`
- Session C→S: `workspace.recovery.inspect.request`
- Session C→S: `workspace.recovery.restore.request`
- Session C→S: `set_voice_mode`
- Session C→S: `send_agent_message_request`
- Session C→S: `wait_for_finish_request`
- Session C→S: `daemon.get_status.request`
- Session C→S: `daemon.get_pairing_offer.request`
- Session C→S: `daemon.config.reload.request`
- Session C→S: `hub.management.daemon.connect.request`
- Session C→S: `hub.management.daemon.get_status.request`
- Session C→S: `hub.management.daemon.disconnect.request`
- Session C→S: `hub.management.daemon.permissions.update.request`
- Session C→S: `diagnostics.request`
- Session C→S: `plugin.catalog.get.request`
- Session C→S: `plugin.list.request`
- Session C→S: `plugin.logs.get.request`
- Session C→S: `plugin.directory.install.request`
- Session C→S: `plugin.directory.inspect.request`
- Session C→S: `plugin.source.install.request`
- Session C→S: `plugin.source.status.request`
- Session C→S: `plugin.source.update.request`
- Session C→S: `plugin.source.update.preview.request`
- Session C→S: `plugin.source.update.apply.request`
- Session C→S: `plugin.reload.request`
- Session C→S: `plugin.enable.request`
- Session C→S: `plugin.disable.request`
- Session C→S: `plugin.remove.request`
- Session C→S: `plugin.rpc.invoke.request`
- Session C→S: `agent.timeline.append.request`
- Session C→S: `agent.skills.get_status.request`
- Session C→S: `agent.skills.reconcile.request`
- Session C→S: `agent.skills.uninstall.request`
- Session C→S: `agent.skills.save_selection.request`
- Session C→S: `agent.skills.import_legacy_selection.request`
- Session C→S: `get_daemon_config_request`
- Session C→S: `set_daemon_config_request`
- Session C→S: `read_project_config_request`
- Session C→S: `write_project_config_request`
- Session C→S: `dictation_stream_start`
- Session C→S: `dictation_stream_chunk`
- Session C→S: `dictation_stream_finish`
- Session C→S: `dictation_stream_cancel`
- Session C→S: `create_agent_request`
- Session C→S: `list_provider_models_request`
- Session C→S: `list_provider_modes_request`
- Session C→S: `list_provider_features_request`
- Session C→S: `list_available_providers_request`
- Session C→S: `get_providers_snapshot_request`
- Session C→S: `refresh_providers_snapshot_request`
- Session C→S: `provider_diagnostic_request`
- Session C→S: `provider.usage.list.request`
- Session C→S: `usage.list_reports.request`
- Session C→S: `resume_agent_request`
- Session C→S: `import_agent_request`
- Session C→S: `refresh_agent_request`
- Session C→S: `cancel_agent_request`
- Session C→S: `shutdown_server_request`
- Session C→S: `restart_server_request`
- Session C→S: `daemon.update.request`
- Session C→S: `fetch_agent_timeline_request`
- Session C→S: `agent.timeline.search.request`
- Session C→S: `agent.timeline.list_prompts.request`
- Session C→S: `agent.provider_subagents.list.request`
- Session C→S: `agent.provider_subagents.timeline.get.request`
- Session C→S: `agent.timeline.set_subscription.request`
- Session C→S: `agent.fork_context.request`
- Session C→S: `set_agent_mode_request`
- Session C→S: `set_agent_model_request`
- Session C→S: `set_agent_thinking_request`
- Session C→S: `set_agent_feature_request`
- Session C→S: `agent.config.apply.request`
- Session C→S: `agent.detach.request`
- Session C→S: `agent.rewind.request`
- Session C→S: `agent_permission_response`
- Session C→S: `checkout_status_request`
- Session C→S: `checkout.diff.get.request`
- Session C→S: `subscribe_checkout_diff_request`
- Session C→S: `unsubscribe_checkout_diff_request`
- Session C→S: `checkout_commit_request`
- Session C→S: `checkout_merge_request`
- Session C→S: `checkout_merge_from_base_request`
- Session C→S: `checkout_pull_request`
- Session C→S: `checkout_push_request`
- Session C→S: `checkout.refresh.request`
- Session C→S: `checkout.discard_changes.request`
- Session C→S: `checkout_pr_create_request`
- Session C→S: `checkout_pr_merge_request`
- Session C→S: `checkout.forge.set_auto_merge.request`
- Session C→S: `checkout.github.set_auto_merge.request`
- Session C→S: `checkout.commits.list.request`
- Session C→S: `checkout.commits.file_diff.request`
- Session C→S: `checkout.forge.get_check_details.request`
- Session C→S: `checkout.github.get_check_details.request`
- Session C→S: `checkout_pr_status_request`
- Session C→S: `pull_request_timeline_request`
- Session C→S: `checkout_switch_branch_request`
- Session C→S: `checkout.rename_branch.request`
- Session C→S: `stash_save_request`
- Session C→S: `stash_pop_request`
- Session C→S: `stash_list_request`
- Session C→S: `validate_branch_request`
- Session C→S: `branch_suggestions_request`
- Session C→S: `forge.search.request`
- Session C→S: `github_search_request`
- Session C→S: `directory_suggestions_request`
- Session C→S: `paseo_worktree_list_request`
- Session C→S: `paseo_worktree_archive_request`
- Session C→S: `create_paseo_worktree_request`
- Session C→S: `workspace_setup_status_request`
- Session C→S: `workspace.setup.subscribe.request`
- Session C→S: `workspace.setup.run.request`
- Session C→S: `list_available_editors_request`
- Session C→S: `open_in_editor_request`
- Session C→S: `open_project_request`
- Session C→S: `project.add.request`
- Session C→S: `project.create_directory.request`
- Session C→S: `workspace.github.search_repositories.request`
- Session C→S: `project.github.clone.request`
- Session C→S: `archive_workspace_request`
- Session C→S: `workspace.create.request`
- Session C→S: `agent.create.request`
- Session C→S: `creation.subscribe.request`
- Session C→S: `workspace.clear_attention.request`
- Session C→S: `workspace.mark_unread.request`
- Session C→S: `file_explorer_request`
- Session C→S: `fs.file.subscribe.request`
- Session C→S: `fs.file.unsubscribe.request`
- Session C→S: `fs.file.write.request`
- Session C→S: `fs.entry.create.request`
- Session C→S: `fs.entry.rename.request`
- Session C→S: `fs.entry.duplicate.request`
- Session C→S: `fs.entry.delete.request`
- Session C→S: `project_icon_request`
- Session C→S: `project.icon.get.request`
- Session C→S: `file_download_token_request`
- Session C→S: `file.upload.request`
- Session C→S: `clear_agent_attention`
- Session C→S: `client_heartbeat`
- Session C→S: `ping`
- Session C→S: `list_commands_request`
- Session C→S: `register_push_token`
- Session C→S: `push.unregister.request`
- Session C→S: `list_terminals_request`
- Session C→S: `subscribe_terminals_request`
- Session C→S: `unsubscribe_terminals_request`
- Session C→S: `create_terminal_request`
- Session C→S: `terminal.rename.request`
- Session C→S: `start_workspace_script_request`
- Session C→S: `workspace.script.list.request`
- Session C→S: `workspace.script.start.request`
- Session C→S: `workspace.script.stop.request`
- Session C→S: `subscribe_terminal_request`
- Session C→S: `unsubscribe_terminal_request`
- Session C→S: `terminal_input`
- Session C→S: `kill_terminal_request`
- Session C→S: `capture_terminal_request`
- Session C→S: `chat/create`
- Session C→S: `chat/list`
- Session C→S: `chat/inspect`
- Session C→S: `chat/delete`
- Session C→S: `chat/post`
- Session C→S: `chat/read`
- Session C→S: `chat/wait`
- Session C→S: `schedule/create`
- Session C→S: `schedule/list`
- Session C→S: `speech.local_models.list.request`
- Session C→S: `speech.local_models.download.request`
- Session C→S: `speech.local_models.delete.request`
- Session C→S: `schedule/inspect`
- Session C→S: `schedule/logs`
- Session C→S: `schedule/pause`
- Session C→S: `schedule/resume`
- Session C→S: `schedule/delete`
- Session C→S: `schedule/run-once`
- Session C→S: `schedule/update`
- Session C→S: `loop/run`
- Session C→S: `loop/list`
- Session C→S: `loop/inspect`
- Session C→S: `loop/logs`
- Session C→S: `loop/stop`
- Session S→C: `hosts.list.response`
- Session S→C: `hosts.changed`
- Session S→C: `hosts.add.response`
- Session S→C: `hosts.update.response`
- Session S→C: `hosts.remove.response`
- Session S→C: `hosts.link.open.response`
- Session S→C: `hosts.route.open.response`
- Session S→C: `hosts.route.frame`
- Session S→C: `hosts.route.closed`
- Session S→C: `browser.host.register.response`
- Session S→C: `subscription.release.response`
- Session S→C: `session.events.set_subscription.response`
- Session S→C: `hub.execution.agent.create.response`
- Session S→C: `hub.execution.agent.validate.response`
- Session S→C: `hub.execution.control.response`
- Session S→C: `hub.execution.agent.update`
- Session S→C: `hub.execution.agent.stream`
- Session S→C: `browser.automation.execute.request`
- Session S→C: `plugin.catalog.get.response`
- Session S→C: `plugin.list.response`
- Session S→C: `plugin.logs.get.response`
- Session S→C: `plugin.directory.install.response`
- Session S→C: `plugin.directory.inspect.response`
- Session S→C: `plugin.source.install.response`
- Session S→C: `plugin.source.status.response`
- Session S→C: `plugin.source.update.response`
- Session S→C: `plugin.source.update.preview.response`
- Session S→C: `plugin.source.update.apply.response`
- Session S→C: `plugin.reload.response`
- Session S→C: `plugin.enable.response`
- Session S→C: `plugin.disable.response`
- Session S→C: `plugin.remove.response`
- Session S→C: `plugin.rpc.invoke.response`
- Session S→C: `agent.timeline.append.response`
- Session S→C: `agent.skills.get_status.response`
- Session S→C: `agent.skills.reconcile.response`
- Session S→C: `agent.skills.uninstall.response`
- Session S→C: `agent.skills.save_selection.response`
- Session S→C: `agent.skills.import_legacy_selection.response`
- Session S→C: `activity_log`
- Session S→C: `assistant_chunk`
- Session S→C: `audio_output`
- Session S→C: `transcription_result`
- Session S→C: `voice_input_state`
- Session S→C: `dictation_stream_ack`
- Session S→C: `dictation_stream_finish_accepted`
- Session S→C: `dictation_stream_partial`
- Session S→C: `dictation_stream_final`
- Session S→C: `dictation_stream_error`
- Session S→C: `status`
- Session S→C: `pong`
- Session S→C: `push.unregister.response`
- Session S→C: `rpc_error`
- Session S→C: `artifact`
- Session S→C: `agent_update`
- Session S→C: `workspace_update`
- Session S→C: `workspace.label.list.response`
- Session S→C: `workspace.label.update`
- Session S→C: `workspace.label.assignment.set.response`
- Session S→C: `workspace.label.update.response`
- Session S→C: `workspace.label.delete.response`
- Session S→C: `workspace.label.delete.inspect.response`
- Session S→C: `project.update`
- Session S→C: `project.list.response`
- Session S→C: `project.suggestions.response`
- Session S→C: `script_status_update`
- Session S→C: `workspace_setup_progress`
- Session S→C: `workspace_setup_status_response`
- Session S→C: `workspace.setup.subscribe.response`
- Session S→C: `workspace.setup.run.response`
- Session S→C: `agent_stream`
- Session S→C: `agent_status`
- Session S→C: `fetch_agents_response`
- Session S→C: `fetch_agent_history_response`
- Session S→C: `fetch_recent_provider_sessions_response`
- Session S→C: `fetch_workspaces_response`
- Session S→C: `project.add.response`
- Session S→C: `project.create_directory.response`
- Session S→C: `open_project_response`
- Session S→C: `workspace.github.search_repositories.response`
- Session S→C: `project.github.clone.response`
- Session S→C: `start_workspace_script_response`
- Session S→C: `workspace.script.list.response`
- Session S→C: `workspace.script.start.response`
- Session S→C: `workspace.script.stop.response`
- Session S→C: `list_available_editors_response`
- Session S→C: `open_in_editor_response`
- Session S→C: `archive_workspace_response`
- Session S→C: `fetch_agent_response`
- Session S→C: `fetch_agent_timeline_response`
- Session S→C: `agent.timeline.replacement`
- Session S→C: `agent.timeline.search.response`
- Session S→C: `agent.timeline.list_prompts.response`
- Session S→C: `agent.provider_subagents.list.response`
- Session S→C: `agent.provider_subagents.timeline.get.response`
- Session S→C: `agent.provider_subagents.update`
- Session S→C: `agent.timeline.set_subscription.response`
- Session S→C: `agent_attention_required`
- Session S→C: `agent.fork_context.response`
- Session S→C: `cancel_agent_response`
- Session S→C: `clear_agent_attention_response`
- Session S→C: `workspace.create.response`
- Session S→C: `agent.create.response`
- Session S→C: `workspace.create.update`
- Session S→C: `agent.create.update`
- Session S→C: `creation.subscribe.response`
- Session S→C: `workspace.clear_attention.response`
- Session S→C: `workspace.mark_unread.response`
- Session S→C: `send_agent_message_response`
- Session S→C: `set_voice_mode_response`
- Session S→C: `daemon.get_status.response`
- Session S→C: `daemon.get_pairing_offer.response`
- Session S→C: `daemon.config.reload.response`
- Session S→C: `hub.management.daemon.connect.response`
- Session S→C: `hub.management.daemon.get_status.response`
- Session S→C: `hub.management.daemon.disconnect.response`
- Session S→C: `hub.management.daemon.permissions.update.response`
- Session S→C: `diagnostics.response`
- Session S→C: `get_daemon_config_response`
- Session S→C: `set_daemon_config_response`
- Session S→C: `read_project_config_response`
- Session S→C: `write_project_config_response`
- Session S→C: `set_agent_mode_response`
- Session S→C: `set_agent_model_response`
- Session S→C: `set_agent_thinking_response`
- Session S→C: `set_agent_feature_response`
- Session S→C: `agent.config.apply.response`
- Session S→C: `agent.detach.response`
- Session S→C: `agent.rewind.response`
- Session S→C: `update_agent_response`
- Session S→C: `project.rename.response`
- Session S→C: `project.icon.set.response`
- Session S→C: `project.remove.response`
- Session S→C: `workspace.title.set.response`
- Session S→C: `workspace.pin.set.response`
- Session S→C: `workspace.recovery.inspect.response`
- Session S→C: `workspace.recovery.restore.response`
- Session S→C: `wait_for_finish_response`
- Session S→C: `agent_permission_request`
- Session S→C: `agent_permission_resolved`
- Session S→C: `agent_deleted`
- Session S→C: `agent_archived`
- Session S→C: `close_items_response`
- Session S→C: `checkout_status_response`
- Session S→C: `checkout_status_update`
- Session S→C: `checkout.diff.get.response`
- Session S→C: `subscribe_checkout_diff_response`
- Session S→C: `checkout_diff_update`
- Session S→C: `checkout_commit_response`
- Session S→C: `checkout_merge_response`
- Session S→C: `checkout_merge_from_base_response`
- Session S→C: `checkout_pull_response`
- Session S→C: `checkout_push_response`
- Session S→C: `checkout.refresh.response`
- Session S→C: `checkout.discard_changes.response`
- Session S→C: `checkout_pr_create_response`
- Session S→C: `checkout_pr_merge_response`
- Session S→C: `checkout.forge.set_auto_merge.response`
- Session S→C: `checkout.github.set_auto_merge.response`
- Session S→C: `checkout.commits.list.response`
- Session S→C: `checkout.commits.file_diff.response`
- Session S→C: `checkout.forge.get_check_details.response`
- Session S→C: `checkout.github.get_check_details.response`
- Session S→C: `checkout_pr_status_response`
- Session S→C: `pull_request_timeline_response`
- Session S→C: `checkout_switch_branch_response`
- Session S→C: `checkout.rename_branch.response`
- Session S→C: `stash_save_response`
- Session S→C: `stash_pop_response`
- Session S→C: `stash_list_response`
- Session S→C: `validate_branch_response`
- Session S→C: `branch_suggestions_response`
- Session S→C: `forge.search.response`
- Session S→C: `github_search_response`
- Session S→C: `directory_suggestions_response`
- Session S→C: `paseo_worktree_list_response`
- Session S→C: `paseo_worktree_archive_response`
- Session S→C: `create_paseo_worktree_response`
- Session S→C: `file_explorer_response`
- Session S→C: `fs.file.subscribe.response`
- Session S→C: `fs.file.unsubscribe.response`
- Session S→C: `fs.file.write.response`
- Session S→C: `fs.entry.create.response`
- Session S→C: `fs.entry.rename.response`
- Session S→C: `fs.entry.duplicate.response`
- Session S→C: `fs.entry.delete.response`
- Session S→C: `fs.file.update`
- Session S→C: `project_icon_response`
- Session S→C: `project.icon.get.response`
- Session S→C: `file_download_token_response`
- Session S→C: `file.upload.response`
- Session S→C: `list_provider_models_response`
- Session S→C: `list_provider_modes_response`
- Session S→C: `list_provider_features_response`
- Session S→C: `list_available_providers_response`
- Session S→C: `get_providers_snapshot_response`
- Session S→C: `providers_snapshot_update`
- Session S→C: `refresh_providers_snapshot_response`
- Session S→C: `provider_diagnostic_response`
- Session S→C: `provider.usage.list.response`
- Session S→C: `usage.list_reports.update`
- Session S→C: `usage.list_reports.response`
- Session S→C: `list_commands_response`
- Session S→C: `list_terminals_response`
- Session S→C: `terminals_changed`
- Session S→C: `create_terminal_response`
- Session S→C: `terminal.rename.response`
- Session S→C: `subscribe_terminal_response`
- Session S→C: `kill_terminal_response`
- Session S→C: `capture_terminal_response`
- Session S→C: `terminal_stream_exit`
- Session S→C: `terminal_attention_required`
- Session S→C: `chat/create/response`
- Session S→C: `chat/list/response`
- Session S→C: `chat/inspect/response`
- Session S→C: `chat/delete/response`
- Session S→C: `chat/post/response`
- Session S→C: `chat/read/response`
- Session S→C: `chat/wait/response`
- Session S→C: `schedule/create/response`
- Session S→C: `schedule/list/response`
- Session S→C: `speech.local_models.list.response`
- Session S→C: `speech.local_models.download.response`
- Session S→C: `speech.local_models.delete.response`
- Session S→C: `schedule/inspect/response`
- Session S→C: `schedule/logs/response`
- Session S→C: `schedule/pause/response`
- Session S→C: `schedule/resume/response`
- Session S→C: `schedule/delete/response`
- Session S→C: `schedule/run-once/response`
- Session S→C: `schedule/update/response`
- Session S→C: `loop/run/response`
- Session S→C: `loop/list/response`
- Session S→C: `loop/inspect/response`
- Session S→C: `loop/logs/response`
- Session S→C: `loop/stop/response`
- Session S→C: `daemon.update.progress`
- Session S→C: `daemon.update.response`
- Declared/unregistered message: `send_agent_message`
- Declared/unregistered message: `agent_list`
- Outer envelope: `hello C→S`
- Outer envelope: `hello.rejected S→C`
- Outer envelope: `ping C→S`
- Outer envelope: `pong S→C`
- Outer envelope: `recording_state C→S`
- Outer envelope: `session C→S`
- Outer envelope: `session S→C`
- Binary/framing: `0x01 Output`
- Binary/framing: `0x02 Input`
- Binary/framing: `0x03 Resize`
- Binary/framing: `0x04 Snapshot`
- Binary/framing: `0x05 Restore`
- Binary/framing: `0x06 ReplayBegin`
- Binary/framing: `0x07 ReplayData`
- Binary/framing: `0x08 ReplayEnd`
- Binary/framing: `0x10 FileBegin`
- Binary/framing: `0x11 FileChunk`
- Binary/framing: `0x12 FileEnd`
- Binary/framing: `0x30 HostRoute`
- Binary/framing: `0x31 ChunkedTransport`
- SDK: `PaseoClient.connect`
- SDK: `PaseoClient.close`
- SDK: `PaseoClient.ensureConnected`
- SDK: `PaseoClient.getConnectionState`
- SDK: `PaseoApi.dispose`
- SDK: `PaseoApi.observeEvents`
- SDK: `hosts.list`
- SDK: `hosts.observe`
- SDK: `hosts.add`
- SDK: `hosts.update`
- SDK: `hosts.remove`
- SDK: `hosts.client`
- SDK: `projects.list`
- SDK: `projects.subscribe`
- SDK: `workspaces.list`
- SDK: `workspaces.ref`
- SDK: `workspaces.open`
- SDK: `workspaces.create`
- SDK: `workspaces.archive`
- SDK: `workspaces.subscribe`
- SDK: `workspace handle.current`
- SDK: `workspace handle.refresh`
- SDK: `workspace handle.setTitle`
- SDK: `workspace handle.archive`
- SDK: `workspace handle.subscribe`
- SDK: `workspace handle.agents.create`
- SDK: `workspace handle.terminals.create`
- SDK: `workspace handle.terminals.list`
- SDK: `agents.list`
- SDK: `agents.ref`
- SDK: `agents.create`
- SDK: `agents.subscribe`
- SDK: `agent handle.current`
- SDK: `agent handle.refresh`
- SDK: `agent handle.reload`
- SDK: `agent handle.send`
- SDK: `agent handle.respondToPermission`
- SDK: `agent handle.run`
- SDK: `agent handle.waitForFinish`
- SDK: `agent handle.commands`
- SDK: `agent handle.archive`
- SDK: `agent handle.detach`
- SDK: `agent handle.subscribe`
- SDK: `agent.timeline.append`
- SDK: `agent.timeline.refetch`
- SDK: `agent.timeline.subscribe`
- SDK: `providers.listModels`
- SDK: `providers.listModes`
- SDK: `providers.listFeatures`
- SDK: `providers.listAvailable`
- SDK: `providers.snapshot`
- SDK: `providers.waitForReady`
- SDK: `providers.refresh`
- SDK: `providers.diagnostic`
- SDK: `providers.listUsage`
- SDK: `providers.subscribe`
- SDK: `config.get`
- SDK: `config.patch`
- SDK: `terminals.create`
- SDK: `terminals.list`
- SDK: `terminals.ref`
- SDK: `terminal handle.current`
- SDK: `terminal handle.refresh`
- SDK: `terminal handle.write`
- SDK: `terminal handle.sendKeys`
- SDK: `terminal handle.capture`
- SDK: `terminal handle.kill`
- DaemonClient: `connect`
- DaemonClient: `close`
- DaemonClient: `ensureConnected`
- DaemonClient: `requireConnectedHosts`
- DaemonClient: `listHosts`
- DaemonClient: `observeHosts`
- DaemonClient: `addHost`
- DaemonClient: `updateHost`
- DaemonClient: `removeHost`
- DaemonClient: `openHostRoute`
- DaemonClient: `openHostLink`
- DaemonClient: `sendHostFrame`
- DaemonClient: `subscribeHostFrames`
- DaemonClient: `cancelHostRoute`
- DaemonClient: `closeHostRoute`
- DaemonClient: `getConnectionState`
- DaemonClient: `subscribeConnectionStatus`
- DaemonClient: `isConnected`
- DaemonClient: `isConnecting`
- DaemonClient: `relaySecurityFloor`
- DaemonClient: `lastError`
- DaemonClient: `authFailureReason`
- DaemonClient: `getLastLivenessRttMs`
- DaemonClient: `supportsDrain`
- DaemonClient: `subscribe`
- DaemonClient: `subscribeRawMessages`
- DaemonClient: `on`
- DaemonClient: `onAgentAttentionRequired`
- DaemonClient: `clearAgentAttention`
- DaemonClient: `clearWorkspaceAttention`
- DaemonClient: `markWorkspaceUnread`
- DaemonClient: `sendHeartbeat`
- DaemonClient: `registerPushToken`
- DaemonClient: `unregisterPushToken`
- DaemonClient: `ping`
- DaemonClient: `measureLatency`
- DaemonClient: `registerBrowserHost`
- DaemonClient: `observeEvents`
- DaemonClient: `observeAgents`
- DaemonClient: `observeWorkspaces`
- DaemonClient: `fetchAgents`
- DaemonClient: `fetchAgentHistory`
- DaemonClient: `fetchRecentProviderSessions`
- DaemonClient: `fetchWorkspaces`
- DaemonClient: `listWorkspaceLabels`
- DaemonClient: `observeWorkspaceLabels`
- DaemonClient: `setWorkspaceLabel`
- DaemonClient: `updateWorkspaceLabel`
- DaemonClient: `deleteWorkspaceLabel`
- DaemonClient: `inspectWorkspaceLabelDelete`
- DaemonClient: `listProjects`
- DaemonClient: `listProjectSuggestions`
- DaemonClient: `openProject`
- DaemonClient: `addProject`
- DaemonClient: `createProjectDirectory`
- DaemonClient: `searchGithubRepositories`
- DaemonClient: `cloneGithubProject`
- DaemonClient: `startWorkspaceScript`
- DaemonClient: `listWorkspaceScripts`
- DaemonClient: `startWorkspaceScriptWithStatus`
- DaemonClient: `stopWorkspaceScript`
- DaemonClient: `archiveWorkspace`
- DaemonClient: `observeWorkspaceSetup`
- DaemonClient: `fetchWorkspaceSetupStatus`
- DaemonClient: `runWorkspaceSetup`
- DaemonClient: `fetchAgent`
- DaemonClient: `supportsBackgroundWorkspaces`
- DaemonClient: `createAgent`
- DaemonClient: `deleteAgent`
- DaemonClient: `archiveAgent`
- DaemonClient: `detachAgent`
- DaemonClient: `updateAgent`
- DaemonClient: `renameProject`
- DaemonClient: `setProjectIcon`
- DaemonClient: `removeProject`
- DaemonClient: `setWorkspaceTitle`
- DaemonClient: `setWorkspacePinned`
- DaemonClient: `inspectWorkspaceRecovery`
- DaemonClient: `restoreWorkspace`
- DaemonClient: `resumeAgent`
- DaemonClient: `importAgent`
- DaemonClient: `refreshAgent`
- DaemonClient: `fetchAgentTimeline`
- DaemonClient: `appendAgentTimelineItem`
- DaemonClient: `searchAgentTimeline`
- DaemonClient: `listAgentTimelinePrompts`
- DaemonClient: `listProviderSubagents`
- DaemonClient: `fetchProviderSubagentTimeline`
- DaemonClient: `observeTimeline`
- DaemonClient: `subscribeAgentTimeline`
- DaemonClient: `buildAgentForkContext`
- DaemonClient: `sendAgentMessage`
- DaemonClient: `sendMessage`
- DaemonClient: `rewindAgent`
- DaemonClient: `cancelAgent`
- DaemonClient: `setAgentMode`
- DaemonClient: `setAgentModel`
- DaemonClient: `setAgentFeature`
- DaemonClient: `setAgentThinkingOption`
- DaemonClient: `applyAgentConfig`
- DaemonClient: `restartServer`
- DaemonClient: `shutdownServer`
- DaemonClient: `updateDaemon`
- DaemonClient: `setVoiceMode`
- DaemonClient: `sendVoiceAudioChunk`
- DaemonClient: `startDictationStream`
- DaemonClient: `sendDictationStreamChunk`
- DaemonClient: `finishDictationStream`
- DaemonClient: `cancelDictationStream`
- DaemonClient: `abortRequest`
- DaemonClient: `audioPlayed`
- DaemonClient: `getCheckoutStatus`
- DaemonClient: `getCheckoutDiff`
- DaemonClient: `observeCheckoutDiff`
- DaemonClient: `checkoutCommit`
- DaemonClient: `checkoutMerge`
- DaemonClient: `checkoutMergeFromBase`
- DaemonClient: `checkoutPull`
- DaemonClient: `checkoutPush`
- DaemonClient: `checkoutRefresh`
- DaemonClient: `listCheckoutCommits`
- DaemonClient: `getCommitFileDiff`
- DaemonClient: `checkoutPrCreate`
- DaemonClient: `checkoutPrMerge`
- DaemonClient: `checkoutForgeSetAutoMerge`
- DaemonClient: `checkoutGithubSetAutoMerge`
- DaemonClient: `checkoutForgeGetCheckDetails`
- DaemonClient: `checkoutGithubGetCheckDetails`
- DaemonClient: `checkoutPrStatus`
- DaemonClient: `pullRequestTimeline`
- DaemonClient: `checkoutSwitchBranch`
- DaemonClient: `renameBranch`
- DaemonClient: `stashSave`
- DaemonClient: `stashPop`
- DaemonClient: `stashList`
- DaemonClient: `getPaseoWorktreeList`
- DaemonClient: `archivePaseoWorktree`
- DaemonClient: `createPaseoWorktree`
- DaemonClient: `createWorkspace`
- DaemonClient: `validateBranch`
- DaemonClient: `getBranchSuggestions`
- DaemonClient: `searchForge`
- DaemonClient: `searchGitHub`
- DaemonClient: `getDirectorySuggestions`
- DaemonClient: `listDirectory`
- DaemonClient: `readFile`
- DaemonClient: `observeFile`
- DaemonClient: `subscribeFile`
- DaemonClient: `writeFile`
- DaemonClient: `createFileEntry`
- DaemonClient: `renameFileEntry`
- DaemonClient: `duplicateFileEntry`
- DaemonClient: `deleteFileEntry`
- DaemonClient: `checkoutDiscardChanges`
- DaemonClient: `uploadFile`
- DaemonClient: `requestDownloadToken`
- DaemonClient: `requestProjectIcon`
- DaemonClient: `getProjectIcon`
- DaemonClient: `listProviderModels`
- DaemonClient: `listProviderModes`
- DaemonClient: `listProviderFeatures`
- DaemonClient: `listAvailableProviders`
- DaemonClient: `getProvidersSnapshot`
- DaemonClient: `getDaemonConfig`
- DaemonClient: `getDaemonStatus`
- DaemonClient: `reloadDaemonConfig`
- DaemonClient: `connectHub`
- DaemonClient: `updateHubPermissions`
- DaemonClient: `getHubStatus`
- DaemonClient: `disconnectHub`
- DaemonClient: `getDaemonPairingOffer`
- DaemonClient: `collectDiagnostics`
- DaemonClient: `patchDaemonConfig`
- DaemonClient: `listLocalSpeechModels`
- DaemonClient: `downloadLocalSpeechModel`
- DaemonClient: `deleteLocalSpeechModel`
- DaemonClient: `sendBrowserAutomationExecuteResponse`
- DaemonClient: `readProjectConfig`
- DaemonClient: `writeProjectConfig`
- DaemonClient: `refreshProvidersSnapshot`
- DaemonClient: `getProviderDiagnostic`
- DaemonClient: `listProviderUsage`
- DaemonClient: `listUsageReports`
- DaemonClient: `listCommands`
- DaemonClient: `respondToPermission`
- DaemonClient: `getPluginCatalog`
- DaemonClient: `listPlugins`
- DaemonClient: `getPluginLogs`
- DaemonClient: `getAgentSkillsStatus`
- DaemonClient: `reconcileAgentSkills`
- DaemonClient: `uninstallAgentSkills`
- DaemonClient: `saveAgentSkillsSelection`
- DaemonClient: `importLegacyAgentSkillsSelection`
- DaemonClient: `installDirectoryPlugin`
- DaemonClient: `installPluginSource`
- DaemonClient: `getPluginSourceStatus`
- DaemonClient: `previewPluginUpdates`
- DaemonClient: `applyPluginUpdates`
- DaemonClient: `updatePluginSources`
- DaemonClient: `inspectDirectoryPlugin`
- DaemonClient: `reloadPlugin`
- DaemonClient: `enablePlugin`
- DaemonClient: `disablePlugin`
- DaemonClient: `removePlugin`
- DaemonClient: `invokePluginRpc`
- DaemonClient: `respondToPermissionAndWait`
- DaemonClient: `waitForAgentUpsert`
- DaemonClient: `waitForFinish`
- DaemonClient: `observeTerminals`
- DaemonClient: `listTerminals`
- DaemonClient: `createTerminal`
- DaemonClient: `renameTerminal`
- DaemonClient: `observeTerminal`
- DaemonClient: `subscribeTerminal`
- DaemonClient: `sendTerminalInput`
- DaemonClient: `killTerminal`
- DaemonClient: `closeItems`
- DaemonClient: `captureTerminal`
- DaemonClient: `scheduleCreate`
- DaemonClient: `scheduleList`
- DaemonClient: `scheduleInspect`
- DaemonClient: `scheduleLogs`
- DaemonClient: `schedulePause`
- DaemonClient: `scheduleResume`
- DaemonClient: `scheduleDelete`
- DaemonClient: `scheduleRunOnce`
- DaemonClient: `scheduleUpdate`
- DaemonClient: `onTerminalStreamEvent`
- DaemonClient: `waitForTerminalStreamEvent`
- DaemonClient: `getLastServerInfoMessage`
- DaemonClient: `setReconnectEnabled`
- Contract: Connection selectors
- Contract: Command output serializers
- Contract: Exit status vs agent turn status
- Contract: Request correlation and exceptional replies
- Contract: Source-owned subscriptions
- Contract: Legacy subscription adapters
- Contract: Directory sync and filtered membership
- Contract: Timeline gaps and replacement epochs
- Contract: Creation idempotency and recovery
- Contract: Provider snapshot hash/compact reference resolution
- Contract: Hello authentication and protocol version
- Contract: Pairing offer v2 and relay security floor
- Contract: Server feature advertisements (101 keys)
- Contract: Client capability projections (21 keys)
- Contract: Shared payload validation (331 schemas)
- Contract: Validation constants (16)
- Contract: Error envelopes and unknown mutation outcomes
- Contract: Terminal restore, replay and backpressure
- Contract: File byte budgets, uploads and optimistic write conflicts
- Contract: Routed-host channel lifetime
- Contract: Ordered binary chunk assembly
- Contract: Reconnect/liveness/RPC deadlines
- Contract: Platform adapters and durable storage boundaries
