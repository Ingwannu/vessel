# ADR 0065: MCP parity — the tool catalogue, per-kind policy, and HTTP

Status: accepted, 2026-11-14. This records the contract. No code in
this commit. The plan is `docs/plans/mcp-parity.md`. It extends ADR
0013 (an agent principal, a credential per session, grants that only
shrink) and does not widen what that principal may call. Protocol 21
below is provisional: the node integrator assigns the final number
when the fields land. `docs/PROTOCOL.md` is updated in that slice,
not here. `docs/ARCHITECTURE.md` does not change: no new crate edge.

## Purpose and intent

Paseo serves its own tools over HTTP and decides, per provider, which
of them an agent is given. Vessel attaches `vessel mcp` on stdio to
every session and advertises every tool. The intent is the same
catalogue, the same moment of decision (session start), and an HTTP
listener a person can turn off, without a second authority that can
do more than the stdio server already can with that session's
credential.

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
| `list_pending_permissions` | — | missing | No call lists them. A new read call may, provisional protocol 21, and only what `session.read` already returns to that principal. An agent is not given the owner's inbox. |
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
`vessel_lsp_workspace_symbols`, `vessel_lsp_document_symbols`,
`vessel_lsp_diagnostics`. They are part of the catalogue the policy
filters. Nothing here removes one.

A missing row is added only by calling a method the principal can
already call, with that method's scope and ownership. A refused row
is not added. A new method, if `list_pending_permissions` needs one,
is read-scoped, additive, and provisional protocol 21; it is not
registered in this commit.

### Policy, per agent kind

Paseo's shape, not a three-way mode.

- `mcp.inject` (Paseo's `daemon.mcp.injectIntoAgents`). Default
  **false**, including when the field is absent, so a node that has
  never set it matches Paseo. Cost, accepted: a new session stops
  receiving `vessel mcp` until a person turns inject on. That is
  what Paseo does on a fresh install. Sessions already running keep
  the server they were given.
- `mcp.serve` (Paseo's `daemon.mcp.enabled`). Default **true** when
  absent. This is the HTTP listener below, not the stdio attachment.
- Per kind, keyed by `HarnessRef` (a custom agent is its own key,
  not the kind it extends): `enabled` (default true) and `disabled`,
  a denylist of tool ids. Absent means every catalogue tool, which
  is Paseo's absent `paseoTools`. `enabled: false` attaches no
  server, which is Paseo's none. There is no allowlist.
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

`node.set_settings` carries the change field by field, admin, as it
does for parking and drivers. A client that sets `mcp` or a kind
policy speaks provisional protocol 21 (`Call::since`). An older node
would ignore the field and the client would think it had landed, so
the client does not send it. An older client omits the fields; the
node leaves them. A reply to a peer that negotiated less than 21
omits them (`for_protocol`), so the old peer neither sees nor clears
them. Absent on the wire means the defaults above, not "all tools
injected".

`NodeSettings` and `SetNodeSettings` drop `Copy` in the slice that
adds the fields: a denylist is owned data. Call sites that copied
them clone, in that same slice. No `serde_json::Value`.

### HTTP

A second front of the same server, not a second authority.

- The node supervises one `vessel mcp` child while `mcp.serve` is
  on, and stops that process group when it is off or the node stops.
  Off means nothing is listening, not a route that answers 404.
  The child holds no credential in its environment, on its command
  line, or in a file the node writes for it. Each request brings its
  own.
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
  The secret travels in `Authorization: Bearer`, the session id in
  one header named for that id, never in the URL, never in a trailer
  the access log prints. The node token is not accepted. A daemon
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
  into a test failure. A debug flag that prints `Authorization` is
  not added.
- When the slice turns HTTP on, it enables `rmcp`'s existing feature
  `transport-streamable-http-server` (rmcp 3.9.0, Apache-2.0, already
  in the workspace) and no other feature of it. `auth` stays off.
  That slice records what the feature pulls in. It does not add a
  crate. If the feature cannot serve a stateless `POST` under the
  rules above, the slice stops and this ADR is extended; a second
  HTTP stack is not introduced quietly.

Stdio does not go away. A session that is given tools still gets
`vessel mcp` on stdio, with the credential in that server's
environment as ADR 0013 placed it. HTTP is how a client that speaks
HTTP reaches the same catalogue with the same credential. Inject
off attaches neither.

### `vessel permit`

Not in this commit. When it is built: `vessel permit ls`,
`vessel permit allow`, `vessel permit deny`, for the person invoking
the CLI (the owner principal), over the calls the owner can already
make. `ls` is the pending list above. `allow` and `deny` take one
session and one request, or `--all` for that session's pending
requests, not every session on the node. Deny takes an optional
reason. Nothing in the arguments is a secret. An agent is not given
this command's reach; the agent keeps `answer_permission`.

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

## Advantages, costs and effects

- Advantages. One catalogue, one principal, one admission compare.
  Policy is data on `node.settings`, so an older peer keeps working.
  A person can turn the tools off per kind without a code change.
- Costs. Inject defaults to false. After the slice that applies it,
  a new session has no `vessel` tools until `mcp.inject` is set.
  That is Paseo's default, and it is a behaviour change from ADR
  0013's "attached to every session". Running sessions are not
  rewritten. `NodeSettings` stops being `Copy`. HTTP is a process
  to supervise. `rmcp`'s HTTP feature pulls its own HTTP crates
  (measured in that slice). Pending permissions need a read call
  the protocol does not have yet.
- Effects. No crate edge, so `docs/ARCHITECTURE.md` and ADR 0001 are
  unchanged. No method is registered here. Browser tools, scripts,
  workspace archive and rename, `schedule.run`, and a self-mode
  grant stay out. Ownership is not relaxed to match Paseo's any-id
  kill, send or archive.

## What this commit does not verify

The table was read from Paseo's registry and from `vessel-mcp`'s
tool list. It was not executed against a daemon. HTTP was not
bound. The gate for this commit is the repository gate on a
docs-only tree.
