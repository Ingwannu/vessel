# ADR 0065: MCP parity — the tool catalogue, per-kind policy, and HTTP

Status: implementation in progress, 2026-11-15; not accepted or consumer-ready.
The original 2026-11-14 proposal below is retained as historical design context.
Its HTTP and permit CLI proposals are not retained implementations. The
"Package implementation amendment" and "Implementation of host skill
administration" sections describe the current source; `docs/plans/wp-mcp.md`
is the current correspondence, decisions, verification ledger and gap list.

This extends ADR 0013 without widening agent grants. The Quality B method
table is present and MCP_PARITY exists as None. The announced node base is
protocol 25 (workspace metadata); this package assigns no protocol number. The
integrator assigns its feature number at the wiring merge.
Skills selection uses settings-style JSON persistence, with no SQLite schema
number. No passing whole gate, accepted ADR or callable handover is implied.

## Original proposal context (2026-11-14)

## Purpose and intent

Paseo serves its own tools over HTTP and decides, per provider, which
of them an agent is given. Vessel attaches `vessel mcp` on stdio to
every session and advertises every tool. The intent is the same
catalogue and the same moment of decision (session start), and an
HTTP listener a person can turn on. No agent is given that listener
in this milestone, so injection stays on stdio. The listener is not a
second authority: it cannot do more than the stdio server already can
with that session's credential.

## Existing implementation and constraints

**What Paseo does** (0.15.4; paths are inside that tree):

- The catalogue is one registry, `packages/server/src/server/agent/tools/paseo-tools.ts`.
  MCP is an adapter over it (`agent/mcp-server.ts`, `registerTool`
  loop). Browser tools are registered beside it
  (`server/browser-tools/tools.ts`) and are out of scope here; they
  are named in the table so a later change cannot treat silence as
  inclusion.
- Policy is not a mode named all, selected or none.
  `ProviderPaseoToolsPolicy` is `{ enabled?: boolean, disabledTools?:
  string[] }` (`packages/protocol/src/provider-config.ts`, lines 31
  to 34), stored on `agents.providers.<exact provider id>.paseoTools`.
  A custom provider does not inherit the id it `extends`. Absent
  policy, or an absent field, means the catalogue is on and every
  tool is offered (`docs/data-model.md`, the "Agent provider Paseo
  tools" section). `enabled: false` offers nothing.
  `disabledTools` is a denylist of exact tool names, not an
  allowlist. `speak` stays even when the catalogue is off
  (`agent/paseo-tool-policy.ts`, lines 14 to 29). Vessel has no
  voice tool, so that exception is not copied.
- Two daemon switches sit above the per-provider policy.
  `daemon.mcp.enabled` defaults to true and is the HTTP route
  (`packages/server/src/server/config.ts`, line 552).
  `daemon.mcp.injectIntoAgents` defaults to false (line 554; the
  `?? true` in `bootstrap.ts` `createInitialMutableDaemonConfig`
  does not apply once that resolver has filled the boolean). When
  inject is false, no provider is given the tools
  (`docs/data-model.md`; `agent-manager.ts` `prepareSessionConfig`,
  lines 5538 to 5548). A child is filtered by the child's provider
  id, not the caller's (`docs/providers.md`).
- The filter is applied when a session is created, resumed, imported
  or reloaded. A running session keeps the list it started with.
- HTTP is `POST /mcp/agents` on the daemon's own listen address,
  stateless (no MCP session id). The injected URL is
  `http://127.0.0.1:<port>/mcp/agents?callerAgentId=<id>` with one
  bearer shared by every agent, random per daemon run, in the
  `Authorization` header (`agent/runtime-mcp-config.ts`, lines 33 to
  64; `bootstrap.ts` around the `randomUUID` that mints it). The
  query parameter is the caller: nothing checks it. The route skips
  the daemon-password middleware. With no password it is open; with
  a password it accepts that shared bearer or the daemon password
  (`server/auth.ts`, `isAgentMcpRequestAuthorized`). DNS-rebinding
  protection on the MCP transport is off (`enableDnsRebindingProtection:
  false`); a separate host allowlist can allow every host. CORS
  reflects a configured origin, including `*`.
- Both MCP switches and `agents.providers` are reloadable.
  `RELOADABLE_PATHS` lists `daemon.mcp.enabled` and
  `daemon.mcp.injectIntoAgents` (`daemon-config-store.ts`, lines 184
  and 185) and `agents.providers` (line 202). `reload` is line 484.
  The diff loop inside `reloadPersisted` was not re-read for this
  record; a path in that list being applied on reload is ADR 0064's
  reading of the same file. `prepareSessionConfig` (lines 5538 to
  5550) builds the launch config and passes a null MCP URL when the
  tools are off, so a session already launched is not rewritten here.
- `paseo permit allow` with no request id allows every pending
  request of that agent. `allow.ts` line 84 says that if no reqId is
  provided, allow all by default, and lines 130 to 132 treat a
  missing id and `--all` as the same. The comment calls `--all` an
  explicit alias for clarity. `paseo permit deny` does not follow
  that default: `deny.ts` lines 29 to 35 throw when both the id and
  `--all` are absent, with the message "Request ID is required
  unless --all is specified". `--all` then denies every pending
  request of that agent (lines 67 to 68).
- The settings row is the host page's Orchestration card
  (`host-page.tsx` line 294, `PaseoToolsCard`). The English title is
  "Enable Paseo tools" and the hint is "Agents will be able to manage
  worktrees, agents and schedules" (`en.ts` lines 2918 to 2921). The
  switch writes `{ mcp: { injectIntoAgents } }`
  (`paseo-tools-card.tsx` line 15).
- ADR 0013 already refused that shape: a claimed id, one token for
  every agent, and tools with no ownership check. Those refusals
  stand. This ADR does not reopen them.

**What Vessel does today:**

- `vessel mcp` is a stdio process per session (`vessel-mcp`), one
  tool call to one protocol call on the session's agent principal
  (ADR 0013). The credential is `AgentAuth` on the local socket, never
  the node token. Scopes are the creator's, at most `read` and
  `operate`, `owned_only` always. An agent does not own its own
  session. Reads of sessions are open. The server holds no authority
  of its own.
- There is no HTTP listener for these tools, no per-kind filter, and
  no `vessel permit`. `answer_permission` answers one request the
  caller may operate. `node.settings` / `node.set_settings` (protocol
  12, drivers in 18) are the setting object a later field extends.
  `NodeSettings` and `SetNodeSettings` are `Copy` today.
- `rmcp` 3.9.0 (Apache-2.0) is already the MCP library, with features
  `server` and `transport-io` only. `vessel-node` does not depend on
  `vessel-mcp` except in tests; the node spawns the `vessel` binary
  (`Config::vessel_mcp`).
- Agents are refused workspace calls except the orchestration tools
  ADR 0027 names, refused scripts (ADR 0031), and refused
  `schedule.run` (ADR 0033: an agent must not pump its own cadence).
  A delegated caller cannot create or update a schedule.

## Alternatives considered

1. **A mode `all | selected | none`.** It is not what Paseo stores.
   `selected` would be an allowlist. Paseo's `disabledTools` is a
   denylist: a tool added later is offered until someone names it.
   An allowlist would hide a new tool from every agent until each
   kind was edited, which is the opposite default.
2. **Paseo's HTTP admission.** A claimed caller id, one bearer for
   every agent, a daemon password that is the owner's authority, and
   an open port when no password is set. ADR 0013 refused this. It
   would let one session act as another, or as the owner.
3. **Serve HTTP inside `vesseld` by depending on `vessel-mcp`.** One
   process, but a production edge `vessel-node -> vessel-mcp` (and
   through it the client). The tool table would sit in the process
   that also holds the node token. A mistake in that process can
   call the service without the agent preamble. The test-only
   dependency that already exists is not that edge.
4. **Put the secret or the session id in the URL**, as Paseo puts
   `callerAgentId` in the query. URLs are copied into logs, traces
   and referrers. ADR 0005 already keeps the node token out of logs;
   the agent secret is the same kind of value.
5. **Keep attaching every tool to every session and add HTTP as an
   anonymous loopback helper.** Loopback is not a boundary (ADR
   0005). An open helper is the owner's authority.

## The choice

### The catalogue

One catalogue, the tools `vessel mcp` already advertises, filtered
at session start. A tool that is filtered out is absent from
`list_tools` and a call to it is a refusal that names the tool, not
a success and not a silent skip. Hiding it only in the list is not
enough: a client may call a name it was not shown.

Browser tools and `speak` are out of scope. They are not added, not
stubbed, and not accepted in a denylist as if they existed. The
twenty-two browser tools Paseo registers are: `browser_back`,
`browser_click`, `browser_close_tab`, `browser_drag`,
`browser_evaluate`, `browser_fill`, `browser_forward`,
`browser_hover`, `browser_keypress`, `browser_list_tabs`,
`browser_logs`, `browser_navigate`, `browser_new_tab`,
`browser_reload`, `browser_resize`, `browser_screenshot`,
`browser_scroll`, `browser_select`, `browser_snapshot`,
`browser_type`, `browser_upload`, `browser_wait`. (The file also
mentions `browser_denied`, `browser_disabled`, `browser_no_host`,
`browser_stale_ref`, `browser_timeout` and `browser_unsupported` as
results, not as tools.)

Paseo's name on the left, Vessel's on the right. "Refused" means a
later slice must not add it: the guard already says no, and adding
the tool would widen what a session can do.

| Paseo | Vessel | Relation | Why it is not the same |
| --- | --- | --- | --- |
| `list_hosts` | `list_nodes` | differs | Fleet nodes by id or unique name. No second host directory, no `serverId` on each call. |
| `create_agent` | `create_session` | differs | Paseo requires `provider/model`, a title (at most 60), an initial prompt, and a workspace placement; a child notifies by default. Vessel takes a harness, an optional model, `cwd` or a project or worktree, an optional prompt and `notify`. Depth and the eight live sessions stay (ADR 0013). |
| `send_agent_prompt` | `send_input` | differs | Paseo: `steer` or `interrupt`, `notifyOnFinish` defaulting to true for an agent caller. Vessel: text, the queue or a steer, `notify` only when asked. Only a session the caller owns. |
| `get_agent_status` | — | missing | `session.get` is already `read`. A tool may wrap it. It must not add fields the get does not return. |
| `list_agents` | `list_sessions` | differs | Vessel lists id, harness, mode, status, profile, creator. It does not filter by background, archive, cwd, age or a status set. |
| `cancel_agent` | — | missing | `session.send` of a cancel is already `operate`. Only a session the caller owns, never its own (it does not own itself). |
| `archive_agent` | `archive_session` | differs | Vessel archives a session the caller started, and stops it first. |
| `kill_agent` | `kill_session` | differs | Same act, Vessel's ownership. Paseo's any-id kill stays refused. |
| `update_agent` | — | missing | `session.rename` and `session.set_option` are `operate` on a session the caller owns. No labels. Not the caller's own session. |
| `get_agent_activity` | `read_conversation`, `read_events` | differs | Vessel returns the conversation or the log, not a curated digest. No second summary that can drop a line. |
| `set_agent_mode` | — | missing | `session.set_option` on a session the caller owns. Not its own session: that would be a new grant. |
| `list_pending_permissions` | — | missing | No call lists them. A new read call may, gated on `MCP_PARITY` (the number is given at merge), and only what `session.read` already returns to that principal. An agent is not given the owner's inbox. |
| `respond_to_permission` | `answer_permission` | differs | Vessel takes the option id, or a refusal. The caller must own the session. Paseo's free-form `updatedInput` is not added. |
| `list_terminals` | `list_terminals` | differs | Vessel: terminals that caller opened, and only where the profile gate allows (ADR 0031). |
| `create_terminal` | `create_terminal` | differs | Beside the caller's session or one it started. Not an arbitrary cwd. |
| `kill_terminal` | `close_terminal` | differs | The name. Only one that caller opened. |
| `capture_terminal` | `read_terminal` | differs | A cursor and at most 32 KiB, escapes removed. Not a scrollback range. |
| `send_terminal_keys` | `write_terminal` | differs | Vessel's names: enter, tab, escape, backspace, the arrows, ctrl-c, ctrl-d, ctrl-z, plus text, at most 16 KiB. Paseo's function keys, paging, home and end are not added in order to match the list. |
| `list_workspaces` | `list_projects`, `list_worktrees` | differs | Those two lists. No background flag. |
| `create_workspace` | `create_worktree` | differs | A project's branch, setup run or skipped. Adopting any directory, checkout of a pull request, and a hidden workspace are not agent tools. |
| `archive_workspace` | — | refused | ADR 0027: an agent does not archive a workspace. |
| `rename_workspace` | — | refused | The same. |
| `list_workspace_scripts`, `start_workspace_script`, `stop_workspace_script` | — | refused | ADR 0031: scripts are not offered to agents. |
| `create_schedule` | `create_schedule` | differs | Vessel also takes `every` (seconds) or cron with a zone, a new session or one the caller owns, `keep` or archive. Eight per session. A delegated caller cannot create one. |
| `create_heartbeat` | `create_heartbeat` | differs | Both prompt the caller's own session through its queue. Vessel's cadence is the schedule cadence, not cron-only. |
| `delete_heartbeat` | `delete_schedule` | differs | No second name. Deleting one the caller owns covers a heartbeat. |
| `list_schedules` | `list_schedules` | differs | Paseo lists only schedules that start a new agent. Vessel lists the schedules that principal may see, heartbeats included. |
| `inspect_schedule` | — | missing | `schedule.get` is `read`. The tool returns that record, not a wider one. |
| `pause_schedule`, `resume_schedule`, `delete_schedule` | the same names | differs | Only a schedule that caller owns. |
| `update_schedule` | — | missing | `schedule.update` is `operate`, and only a schedule that caller owns. A delegated caller stays refused (ADR 0033). |
| `schedule_logs` | `schedule_runs` | differs | The name, and at most the newest 50. |
| `run_schedule_once` | — | refused | ADR 0033. |
| `list_providers` | — | missing | `harness.list` is `read`. The tool is that list, not a second catalogue. |
| `list_models` | — | differs, not a wrapper of `model.list` | `model.list` is the routing table, not a provider's model catalogue. Wrapping it under Paseo's name would lie. A tool waits until it can return the models the harness options already report. |
| `list_profiles` | — | missing | `profile.list` is `read`. Names and labels, never a secret the profile holds. |
| `inspect_provider` | — | missing | `harness.options` is the compact view. Draft feature settings that the options call does not compute are not invented. |

Vessel tools with no Paseo counterpart stay: `wait_turn`,
`read_events`, `read_conversation`, `change_queue`,
`answer_question`, `resume_session`, `search_sessions`,
`fork_session`, `rewind_session`, `get_pull_request`,
`open_pull_request`, `list_saved_prompts`, `list_ports`,
`vessel_lsp_definition`, `vessel_lsp_references`, `vessel_lsp_hover`,
`vessel_lsp_implementations`, `vessel_lsp_type_definition`,
`vessel_lsp_incoming_calls`, `vessel_lsp_outgoing_calls`,
`vessel_lsp_workspace_symbols`, `vessel_lsp_document_symbols`,
`vessel_lsp_diagnostics` (`crates/vessel-mcp/src/tools/lsp.rs`, the
list in `tools`). They are part of the catalogue the policy filters.
Nothing here removes one.

A missing row is added only by calling a method the principal can
already call, with that method's scope and ownership. A refused row
is not added. A new method, if `list_pending_permissions` needs one,
is read-scoped and additive, and it is gated on `MCP_PARITY`; it is
not registered in this commit. The number is given at merge.

### Policy, per agent kind

Paseo's shape, not a three-way mode. One top-level key on
`node.settings`, named `mcp`. ADR 0064 makes every field of
`SetNodeSettings` one variant of its key enum, and the class is
decided by the ADR that adds the field. This field is that one
variant. Its class is **reloadable**: not start-time, and not pinned
by a launch flag, so a reload does not put it in `restart_required`
or `override_controlled`. It is not three variants. This ADR does not
edit ADR 0064, and it does not name a path under `vessel node config`.
The command that sets the field is ADR 0064's, over `mcp`.

A reload that finds the saved `mcp` object changed lists that one key
as applied once each sub-field's effect below has been done. The
effects are not the same moment:

| Sub-field | Paseo path | What "applied" means |
| --- | --- | --- |
| `inject` | `daemon.mcp.injectIntoAgents` (`RELOADABLE_PATHS` line 185) | The next session start: create, resume, and a profile move that restarts the harness. A session already running keeps the server it was given. |
| `serve` | `daemon.mcp.enabled` (line 184) | At once. The supervised child starts, or it is stopped and nothing is left listening. |
| `policy` | `agents.providers` (line 202), the `paseoTools` field only | The next session start of that harness. A running session keeps the list it started with. |

`agents.providers` is not this key. The denylist is `mcp.policy`.

- `mcp.inject` (Paseo's `daemon.mcp.injectIntoAgents`). A directory
  this build creates stores **false** before it creates `node.id`.
  That is Paseo's fresh install (`config.ts` line 554, `?? false`).
  A directory that already has `node.id` is not a fresh install: see
  the migration below. A session already running keeps its server.
- `mcp.serve` (Paseo's `daemon.mcp.enabled`). Default **false**,
  including when the field is absent inside a `mcp` object that is
  present. Stated difference from Paseo's `?? true` (`config.ts` line
  552). Paseo's true is the injection transport, on the listener the
  daemon already has. Vessel's listener is a separate child, and no
  agent is given it in this milestone, so an absent choice does not
  open a port.
- `mcp.policy` is a list of `{ harness, enabled, disabled }`, not a
  JSON object keyed by harness. `HarnessRef` for an added agent is
  itself an object (`{"added":"<name>"}`, `agents/name.rs` lines 101
  to 103), so it cannot be an object key and still keep that wire
  form. A known harness stays its existing string (`"claude-code"`,
  `"codex"`, `"gemini"`, `"open-code"`). An added agent is its own
  entry, not the kind it extends. `enabled` defaults to true.
  `disabled` is a denylist of tool ids. A harness with no entry is
  every catalogue tool, which is Paseo's absent `paseoTools`.
  `enabled: false` attaches no server. There is no allowlist.
- Inject off wins over a per-kind policy, as Paseo's global switch
  does. The per-kind policy is still stored.
- Resolved at start, at resume, and when a profile move restarts the
  harness. Not pushed into a server that is already up.
- A child session is filtered by its own harness, not its parent's.
- `speak` is not special-cased.

Tool ids are a closed enum of this build's catalogue, not strings
matched later (T1, T3). A client that names a tool this build does
not have is refused, and nothing is stored: a denylist entry the
node cannot enforce would look like a disable that is not one.
A name only a newer build knows is the enum's `Unknown` on read, is
written back unchanged, and disables nothing here (the driver
pattern, ADR 0057). It is not `#[non_exhaustive]`.

### What false inject takes off a new session

The node keeps running schedules and heartbeats it already stored
(ADR 0033: the scheduler is the node's, not the tool's). A person
keeps every call. A session that is already running keeps the server
it was given. A **new** session with inject false is not given
`vessel mcp`, so its agent loses every tool that server advertises:

- Sessions: `list_nodes`, `list_sessions`, `send_input`,
  `answer_permission`, `read_events`, `wait_turn`, `kill_session`,
  `create_session`.
- Conversation: `read_conversation`, `change_queue`, `answer_question`,
  `resume_session`, `archive_session`. Search: `search_sessions`.
  Fork: `fork_session`, `rewind_session`.
- Workspace, which is the agent's path into ADR 0027: `list_projects`,
  `list_worktrees`, `create_worktree`. Forge: `get_pull_request`,
  `open_pull_request`.
- Terminals: `create_terminal`, `list_terminals`, `read_terminal`,
  `write_terminal`, `close_terminal`, `list_ports`.
- Automation: `create_schedule`, `create_heartbeat`, `list_schedules`,
  `schedule_runs`, `pause_schedule`, `resume_schedule`,
  `delete_schedule`, `list_saved_prompts`. Without `create_heartbeat`
  a new session cannot start a heartbeat (ADR 0033's one exception
  that lets an agent prompt its own session). Without `send_input` it
  cannot ask to be told about a turn it started (ADR 0033, decision
  6). Notices the node has already queued are not cancelled.
- The ten LSP tools named above.

An upgraded directory does not lose these until a person turns inject
off. A fresh directory starts without them, which is Paseo's fresh
install.

### Where a person turns inject on

`node.set_settings` with `mcp.inject` set to the JSON boolean `true`,
admin, once the field exists. The same object carries `serve` and
`policy` when those change. The CLI that writes the field is ADR
0064's `vessel node config set` over `mcp`. This ADR does not choose
how that command spells a sub-field.

The app row is the one Paseo puts on the host page's Orchestration
card. Paseo's English title is "Enable Paseo tools" (`en.ts` line
2919) and the hint is "Agents will be able to manage worktrees,
agents and schedules" (line 2920). Vessel's title is "Enable Vessel
tools". Stated difference: the tools belong to Vessel, and the title
would otherwise name the other product. The hint is Paseo's sentence.
The switch is bound to `mcp.inject`. This track does not draw the
screen. This branch's native tree has no such row; the settings track
draws it from this name.

### Migration

`settings.json` has no protocol number (ADR 0038). ADR 0064 does not
accept `version`. Absence of `mcp` cannot mean both "fresh" and
"upgraded". The applying slice looks at whether `node.id` is already
in the data directory (`data_dir.rs` lines 6 to 7) **before** this
start creates it. The check is the file's presence. It does not open
`node.key` or `token`.

- `node.id` is absent. This start is creating the directory. Before
  it creates `node.id`, it writes `mcp` with `inject: false` and
  `serve: false`, and no policy entries. `park_after` and `drivers`
  stay absent, so they keep following their flags (ADR 0038). A
  failure to write refuses the start with that write error, and
  `node.id` is not created, so the next start is still a creation
  and not an upgrade.
- `node.id` is present and the saved settings have no `mcp` key. This
  directory already ran Vessel, which attaches `vessel mcp` to every
  session (ADR 0013). Before any session is launched, the node writes
  `inject: true` and `serve: false` and leaves every other saved
  field. A failure to write refuses the start. Sessions are not
  launched on an unstored decision.
- `mcp` is present. Missing `inject` or `serve` inside it is false.
  That is not a second migration.

Setting `mcp.inject` or `mcp.serve` to false stores that false inside
the object. It does not remove `mcp`. `mcp.inject` has no flag, and a
missing `mcp` on a directory that already has `node.id` is the upgrade
signal, so clearing the whole key would turn inject back on at the
next start. A person who wants inject off sends `mcp.inject: false`.
Setting `policy` to an empty list is the absent policy, every harness
included. Deleting `settings.json` and restarting a directory that
still has `node.id` takes the upgrade path again, so inject comes
back on. A person who wanted it off sets it off again.

`node.set_settings` carries the change field by field, admin, as it
does for parking and drivers. A client that sets `mcp` speaks
`MCP_PARITY` (`Call::since`). The number is given at merge. An older
node would ignore the field and the client would think it had landed,
so the client does not send it below that constant. An older client
omits the fields; the node leaves them. A reply to a peer that
negotiated less than `MCP_PARITY` omits them (`for_protocol`), so the
old peer neither sees nor clears them. Absent on the wire means the
defaults of a present object (inject false, serve false, no policy
entries), not "all tools injected". The upgrade write is a start-up
write of an explicit `true`, not an absent field that old peers must
read.

`NodeSettings` and `SetNodeSettings` drop `Copy` in the slice that
adds the fields: a denylist is owned data. Call sites that copied
them clone, in that same slice. No `serde_json::Value`.

### HTTP

A second front of the same server, not a second authority.

- No agent receives this endpoint in this milestone. The consumer is
  none yet. Injection stays `vessel mcp` on stdio. `mcp.serve`
  therefore defaults off: Paseo's `daemon.mcp.enabled` defaults true
  because HTTP is how it injects, and that costs no extra listener
  (`config.ts` line 552). Vessel's listener is a child nobody is
  given, so the default does not open it.
- While `mcp.serve` is on, the node supervises one `vessel mcp` child
  and stops it when the setting goes off or the node stops. Off means
  nothing is listening, not a route that answers 404. The child is a
  `vessel-proc` child: stopping it signals the process group on Unix
  and terminates that child's job on Windows (`child.rs` lines 113 to
  137; `lib.rs` lines 28 to 31). It is not a systemd unit and not a
  launchd agent. A failure to start it is that crate's spawn error,
  `serve` is not applied, and nothing is left listening. The
  error names no secret: the child holds none in its environment, on
  its command line, or in a file the node writes for it. Each request
  brings its own.
- Bind `127.0.0.1` only, port chosen by the kernel, never the node's
  websocket port and never a non-loopback address. The bound port is
  not a secret; it may be told to a client that already holds `read`.
  It is not a URL with a credential in it. Paseo serves MCP on
  whatever address the daemon listens on, which can be public: Vessel
  does not.
- `POST /mcp/agents` only. Any other method is refused. A query
  string is refused, so a token cannot arrive in a URL. No
  `callerAgentId`. The body is one JSON-RPC message, at most the
  frame size the node already accepts. Stateless: the response does
  not set an MCP session id.
- Admission is `AgentAuth`: the session id and the session secret,
  the same pair the stdio server sends as the socket's first frame.
  The secret travels in `Authorization: Bearer`. The session id
  travels in the header `Vessel-Session`, one value, the session id's
  `Display` text (a UUID). A missing header, an empty value, a value
  that does not parse, or more than one value is the same refusal as
  a bad bearer. The name is not `Mcp-Session-Id`. The response does
  not set a transport session id, and the header that carries the
  Vessel session has to be one a test can tell apart from that.
  rmcp's own header constant was not read for this commit. The header
  is never in the
  URL and never in a trailer the access log prints. The node token
  is not accepted. A daemon
  password is not accepted. There is no anonymous mode. The compare
  is the socket's compare, constant time, and unknown, wrong and
  ended are the same refusal. The connection that then runs is that
  session's principal: its scopes, its ownership, its limits. The
  HTTP process does not keep a client opened as the owner.
- `Host` must be `127.0.0.1:<port>` or `localhost:<port>`. Anything
  else is refused (DNS rebinding). `Origin`, when present, must be
  `http://127.0.0.1:<port>` or `http://localhost:<port>`; any other
  origin is refused and is not reflected. A missing `Origin` is
  allowed, because an agent is not a browser. No `Access-Control-Allow-Origin: *`.
  The MCP transport's own rebinding check, if the feature has one,
  stays on.
- The secret is never logged, never put in an error, never copied
  into a test failure, and never written into the data directory's
  `mcp/` files. Those files are terminal MCP configuration (ADR 0012,
  `data_dir.rs` lines 20 to 21), not this credential. A debug flag
  that prints `Authorization` is not added.
- This milestone does not hand the URL, the header or the secret to
  any agent. A later milestone that names a consumer passes three
  things in that agent's MCP server config, in memory only, the way
  Paseo passes `headers.Authorization` (`runtime-mcp-config.ts` lines
  49 to 53) and not the way it puts the caller id in the query: the
  URL `http://127.0.0.1:<port>/mcp/agents` with no query and no
  userinfo, `Authorization: Bearer` holding the session secret, and
  `Vessel-Session` holding the session id. Nothing in this milestone
  writes that config. Until a consumer is named, turning serve on
  opens a port that no agent has been told about. The setting exists
  so that later change is the default and the handoff, not a second
  design.
- When the slice turns HTTP on, it enables `rmcp`'s existing feature
  `transport-streamable-http-server` (rmcp 3.9.0, Apache-2.0, already
  in the workspace) and no other feature of it. `auth` stays off.
  That slice records what the feature pulls in. It does not add a
  crate. If the feature cannot serve a stateless `POST` under the
  rules above, the slice stops and this ADR is extended; a second
  HTTP stack is not introduced quietly.

Stdio does not go away. A session that is given tools still gets
`vessel mcp` on stdio, with the credential in that server's
environment as ADR 0013 placed it (`VESSEL_AGENT_SECRET` in that
child's environment, not on a command line). No agent is switched
from stdio to HTTP here. Inject off attaches neither. What the
listener can reach, when it is on, is the calls that session's
principal can already make. What refuses a request is a non-loopback
bind (it is not offered), a query string, a `Host` or `Origin` that
fails the checks above, a missing or bad `Vessel-Session`, a bad
bearer, the node token, a daemon password, or no credential.

### `vessel permit`

Not in this commit. When it is built: `vessel permit ls`,
`vessel permit allow`, `vessel permit deny`, for the person invoking
the CLI (the owner principal), over the calls the owner can already
make. `ls` is the pending list above. The session argument is one
session, never every session on the node.

`allow` with no request id answers every pending request of that
session. `--all` is the same act. That is Paseo's allow (`allow.ts`
lines 84 and 130 to 132): a missing id allows all, and `--all` is an
alias. `deny` with no request id and no `--all` is a usage error
before any call, and the message is Paseo's: "Request ID is required
unless --all is specified" (`deny.ts` lines 29 to 35). `--all` denies
every pending request of that session (lines 67 to 68). One request
id answers that request only. The two commands are not given the same
default. Paseo already split them, and a missing argument that denied
every pending request would be the accident `deny.ts` already
refuses.

Deny takes an optional reason. Nothing in the arguments is a secret.
An agent is not given this command's reach; the agent keeps
`answer_permission`.

## Why this and not the others

- The denylist is Paseo's, so a new tool is offered until it is
  named, and a kind that was never configured does not need a row.
- The three-way mode was the wrong reading of that schema. Coding it
  would be a setting Paseo does not have, with the opposite default
  for a tool added later.
- HTTP uses the credential and the scopes the stdio server already
  uses, so serving another transport cannot promote an agent to the
  owner. Paseo's shared bearer and open-when-unconfigured route are
  the holes ADR 0013 closed.
- A supervised child keeps `vessel-mcp` out of the node's production
  dependencies. The process that holds the node token is not the
  process that parses tool arguments.
- Loopback, a refused query string, and an origin check are there
  because an HTTP port is reachable by a browser on the same machine
  and a URL is not a safe place for a secret.
- Serve defaults off because this milestone names no agent that would
  receive the endpoint. Inject stays on for a directory that already
  has `node.id`, because taking the tools off every upgraded node
  would stop the agent paths ADR 0033 and ADR 0027 already ship.

## Advantages, costs and effects

- Advantages. One catalogue, one principal, one admission compare.
  Policy is data on `node.settings`, so an older peer keeps working.
  A person can turn the tools off per kind without a code change.
- Costs. A fresh directory stores inject false, so its new sessions
  have no `vessel` tools until a person turns `mcp.inject` on. That
  is Paseo's fresh install, and it is a behaviour change from ADR
  0013 for a directory this build creates. An upgraded directory is
  written `inject: true` once, so the change does not land on it by
  absence. Running sessions are not rewritten. `NodeSettings` stops
  being `Copy`. HTTP is a process to supervise, and it is off until
  a person turns it on. `rmcp`'s HTTP feature pulls its own HTTP
  crates (measured in that slice). Pending permissions need a read
  call the protocol does not have yet.
- Effects. No crate edge, so `docs/ARCHITECTURE.md` and ADR 0001 are
  unchanged. No method is registered here. Browser tools, scripts,
  workspace archive and rename, `schedule.run`, and a self-mode
  grant stay out. Ownership is not relaxed to match Paseo's any-id
  kill, send or archive.

## What this commit does not verify

The table was read from Paseo's registry and from `vessel-mcp`'s
tool list. It was not executed against a daemon. HTTP was not
bound. `reloadPersisted`'s diff loop was not re-read. This commit
changes documents only, so its check is `cargo fmt --all --check`,
the size script and the boundary script. Clippy and the tests are
not part of that check.


## Package implementation amendment

The tool identifier table lives in `vessel-proto::mcp`, shared by the node
and MCP adapter. This avoids a second denylist vocabulary. Moving the
existing parser is behavior-preserving; the MCP adapter retains its unknown
LSP-prefix refusal. No protocol method or settings field is registered by
this move. There is no new dependency. The original proposed HTTP listener
is deferred: no consumer requires it, so no transport or port is added.

Skills management uses the same typed node boundary as other host settings;
its explicit homes and filesystem transaction rules are recorded in the
package plan before implementation. The skill service is Admin-only and
never an MCP tool: agents cannot install or remove their own instructions.


The shared skill name is the existing fleet bundle-path invariant moved to
`vessel-proto::skills`. Fleet keeps its YAML-frontmatter reader and error
mapping; only the directory value moves down. Selection is all/custom and
normalizes names before use. Malformed selections are typed refusals instead
of Paseo's fallback to all, to avoid accidentally broadening installation.
Skill names and operation lists are bounded by `MAX_SKILLS`; this is a stated
resource limit beyond Paseo's unbounded arrays. No new dependency is added.


Skill selection is stored in `skills.json`, separate from synchronized fleet
content. Its optional selection distinguishes no choice from an explicit all
choice for legacy import. The file uses store's existing atomic private write
and set-aside machinery: malformed input is preserved as unreadable; future
fields are copied aside before replacement. Whole-file reads are bounded.
Links and special files are refused, including on write, so preserving a
malformed file cannot change the permissions of a linked outside target.


The implemented settings field is `mcp`, with partial `inject`/`policy` edits.
It has no HTTP sub-field in this package: the unused listener remains deferred.
Both direct settings replies and nested fleet replies hide the field while
`MCP_PARITY` is None and from older peers after assignment; edits require that
feature at every hop. Fresh and upgrade decisions are persisted before
identity creation under the data lock. Agent fixtures opt in explicitly, with
behavior assertions unchanged, while independent tests pin both defaults.

A bounded tracked task owns each admitted settings write through apply, even
after request cancellation. Shutdown prevents new jobs and drains admitted
ones. Each new process takes one policy snapshot before credential creation;
profile-move failure restores the previous credential, including absence.


## Implementation of host skill administration

The node owns both the typed pending calls and their single writer. This
extends the contract above without adding an HTTP listener or changing agent
grants. Five Admin-only unary `skills.*` calls are non-forwardable and refused
to agents. Fleet supplies safe filesystem/transaction mechanisms; store owns
`skills.json`; node captures explicit homes, serializes blocking operations,
tracks admitted work through cancellation/shutdown, and persists the selection
last. `MCP_PARITY` now exists as `None`; the integrator assigns its version.
The earlier "no code in this commit" describes the original ADR proposal;
current implementation details and remaining gaps are in `plans/wp-mcp.md`.


## Negotiated structured-result compatibility (implementation in progress)

The pinned rmcp 3.9 API represents the 2026-07-28 protocol, where structured
content may be any JSON value. Vessel also serves initialize-based 2025 clients;
those permit structured objects only. Earlier revisions do not define that
field. The SDK strips the newer resultType for legacy peers, but its structured
constructor does not project arrays/scalars to the older structured schema.

Reuse RequestContext::protocol_version for the actual request and one private
result-serialization adapter. Keep the existing model-visible JSON text byte
shape. Object-capable legacy peers retain matching structured objects; a legacy
array/scalar result omits optional structuredContent instead of wrapping/changing
its old text API. Modern peers retain matching arbitrary structured JSON,
including explicit null. Missing negotiated metadata is conservative text-only.
This is representation compatibility, never a grant or a tool-policy fallback.

The alternative of wrapping all results would change existing text consumers;
always returning the latest SDK shape rejects a valid legacy client's result.
Paseo's object-shaped tool results naturally retain structured content. Vessel's
legacy array results deliberately omit it where the negotiated protocol cannot
represent that shape. No method, transport, output data or authority is added.

Reference: the official
[2025-11-25 tools specification](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2025-11-25/server/tools.mdx)
and [2026-07-28 tools specification](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/server/tools.mdx).
A revision/shape matrix checks unchanged text plus permitted structured data;
the actual pipe fixture asserts its negotiated 2025 version and decodes the
structured slot as a strict object, replacing its invalid any-JSON assumption.
Exact changed-source strict and real pipe execution remain owed.
