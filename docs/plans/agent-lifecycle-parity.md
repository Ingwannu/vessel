# Agent lifecycle parity

Labels, a background listing, a live reload, and selectors on `vessel log`.
The decision is ADR 0063. Provisional protocol 21; the integrator assigns
the final number. This track does not touch parking, rewind, fork, or
`vessel-agent`.

## What Paseo does

Read first, and reproduced except for the differences ADR 0063 lists.

### Labels

- `packages/protocol/src/agent-labels.ts:2-6` reserves
  `paseo.parent-agent-id`, `paseo.parent-server-id` and
  `paseo.open-agent-tab.<client>`. Those are parentage and open tabs, not
  a person's marks. Vessel already records the creator and a side
  question, so it does not copy the reserved keys. A key under `vessel.`
  is refused so the node can use that prefix later.
- The wire value is `Record<string, string>`
  (`packages/protocol/src/messages.ts:915` on a list item,
  `:989-996` on `update_agent_request`, and the import request's
  `labels` at `:1898`). No maximum length or count is declared.
- A patch merges. `null` deletes one key
  (`packages/server/src/server/agent/agent-manager.ts:631-644`,
  `writeLabels` at `:2296`). The CLI and the MCP tool only send
  strings, so a person cannot delete. An omitted key stays.
- CLI create and import: `key=value`, key trimmed and non-empty, value
  the remainder including empty and not trimmed
  (`packages/cli/src/commands/agent/update.ts:103-140`,
  `packages/cli/src/commands/agent/import.ts:54-82`). The same split is
  used by `run`. The last copy of a key wins.
- CLI update refuses an empty name, an empty `--thinking`, and combining
  `--thinking` with `--name` or `--label`
  (`update.ts:152-197`). Labels alone are a metadata update.
- List filter is logical and, and only when asked
  (`packages/cli/src/commands/agent/ls.ts:126-153` and `:228-241`).
  A filter without `=` is ignored by the CLI parser. Vessel refuses it
  instead of dropping it.
- MCP: `create_agent` takes `labels` (`paseo-tools.ts:1185`);
  `list_agents.includeBackground` defaults to false (`:2133`);
  `update_agent` takes `name`, `labels` and `settings`
  (`paseo-tools.ts:2266-2298`). Settings-only still calls the metadata
  command, which reports nothing to update, and the tool ignores that
  report. Vessel's `set_labels` has no settings, so an empty patch is a
  typed refusal.

### Background

- `docs/agent-lifecycle.md:186-203`. `workspace.background` is discovery
  only. It is chosen at creation and cannot be changed later. Omission
  inherits the caller workspace, or `false` when there is no caller.
  Default lists, including history, exclude background work.
  `filter.includeBackground` includes it. An exact id does not consult
  the filter. `ls -a` includes archived and background
  (`ls.ts:114-150`).
- MCP `create_workspace.background`
  (`paseo-tools.ts:1396-1401`) and `list_agents.includeBackground`
  (`:2133`, default false) and `list_workspaces`
  (`:1526`, default false).
- Vessel has no workspace visibility. The flag is `SessionListing` on
  the session (ADR 0063). Archived sessions stay listed as they are;
  `--background` does not mean `-a`.

### Reload

- CLI `packages/cli/src/commands/agent/reload.ts:27` ("restarts the
  underlying process"), `:66` `refreshAgent`, result `reloaded` plus
  `timelineSize` (`:75`). The app's tooltip is skills, MCPs or login.
- Wire `refresh_agent_request` (`messages.ts:1903-1907`) answers
  `agent_refreshed` with `timelineSize` (`:3949-3952`).
- Handler `packages/server/src/server/session.ts:4760-4838`: a live
  agent is interrupted, then `reloadAgentSession` with
  `rehydrateFromDisk: true`. A closed agent is loaded
  (`ensureAgentLoaded`) and its provider history hydrated. A missing
  record, an unavailable provider or a record with no persistence is an
  error. The worktree is not recreated.
- `agent-manager.ts:1625-1638` starts the reload. Labels, workspace,
  owner, launch env, created and updated times, last usage, last error
  and attention are kept. A running turn is cancelled first. If the
  cancel fails or times out, the turn stays and reload fails
  (`docs/agent-lifecycle.md:116`). The old process is closed before the
  new one starts; a close timeout keeps the old runtime
  (`agent-manager.ts:1764-1783`, "Timed out closing previous session
  during refresh"). After a successful close, a failed resume leaves
  the agent closed and retryable (`docs/agent-lifecycle.md:46-47`).
  `rehydrateFromDisk` deletes the in-memory timeline so provider
  history is read again (`agent-manager.ts:1627`, applied at `:1715`).
- Vessel's difference, and why: the log is the cursor
  (`crates/vessel-proto/src/event.rs` decision log). It is not wiped.
  Replay is dropped, as a move already drops it. `next_seq` is the
  count Paseo returns as `timelineSize`. A closed agent is not loaded:
  that is `session.resume`. A parked agent is not woken: that is
  `session.wake`. Both refusals name the call. There is no MCP reload
  tool in Paseo (`paseo-tools.ts` registers none); Vessel adds none,
  and refuses the RPC to agents.

### Logs

- `packages/cli/src/commands/agent/logs.ts:17-20`: `-f` / `--follow`,
  `--tail`, `--filter`, `--since`.
- `--since` is `Date.parse`; a failure is `INVALID_TIMESTAMP` and the
  example form (`:211-224`). `--tail` is an integer `>= 0`; anything
  else is an error (`:201-209`). `0` prints nothing.
- Filter (`:226-249`): `tools` is `tool_call`; `text` is a user message,
  an assistant message or reasoning; `errors` is type `error`;
  `permissions` is a type containing `permission`; any other string is
  a substring of the type. Vessel accepts only the four names and
  refuses the rest, so a typo is not an empty log.
- Follow prints the tail of what exists (default 10 when `--tail` is
  omitted, `:334-335`) and then streams. One-shot with no `--tail` prints
  the whole selection. Follow joins assistant and reasoning fragments
  (`createFollowTranscriptWriter`). Vessel already joins reply chunks
  in the CLI view; follow uses that, and does not reprint Paseo's
  banners for a replaced timeline. The log has no epoch.

## Existing assets

| Asset | Where | Decision |
| --- | --- | --- |
| Title, separate from the spec | `session.rename`, schema 6 `title` | Modify: labels follow the same split. Do not fold them into the title. |
| Marks as columns | schema 7 | Modify: schema 15 adds `labels` (JSON text, null is empty) and `listing` (`shown` default). |
| Unit `session.list` | `rpc/requests.rs` | Modify: a struct that still decodes `params: null`. |
| `Call::since`, `Reply::for_protocol`, `SessionInfo::for_protocol` | `rpc/since.rs`, `rpc/older.rs` | Modify: new fields need 21; older replies drop them; new events have a `HarnessLog` stand-in. |
| `Method::forwardable` and its pinned list | `forward.rs`, `rpc/tests.rs` | Modify: `session.labels` and `session.reload` are forwardable, as rename and wake are. |
| Agent refusal | `vessel-node/src/workspace/policy.rs` | Modify: `session.labels` allowed; `session.reload` refused with its own reason, not the file-and-git sentence. |
| Stop and `session/load` | `vessel-session/src/moving.rs`, parking's wake | Reuse from a new reload order. Do not change move, park, wake, rewind or fork. |
| `session.resume` | protocol 10 | Reuse only as the retry named when a replacement fails, and as the refusal for a session that is not live. |
| `vessel log`, the view, `session.read` | `vessel-cli` | Modify the command and the view's match for the two new events. No new read RPC. |
| MCP `create_session`, `list_sessions`, `read_events` | `vessel-mcp/src/tools/sessions.rs` | Modify the first two. Leave `read_events` as it is. |
| `ErrorCode` | `vessel-proto/src/error.rs` | Reuse. No new variant, so existing matches stay. |

No external crate. The standard library and the types above hold the
bounds, the map and the timestamps.

## Slices

1. **This commit.** ADR 0063 and this plan. No code, so `PROTOCOL_VERSION`
   stays 20 and schema stays 14.
2. **Labels.** `LabelKey`, `LabelValue`, `SessionLabels`. Create, import
   copy, `session.labels`, `labels_changed` and its stand-in, the list
   filter, schema 15's `labels` column, CLI `--label`, MCP `labels` and
   `set_labels`. Tests beside the parser, the patch, the filter and the
   old-peer rewrite.
3. **Background.** `SessionListing`, creation inheritance, schema 15's
   `listing` column (same migration as slice 2 if slice 2 has not yet
   shipped it; if it has, schema 16). `session.list`'s `scope`. CLI
   `--background` on `run` and on `sessions`. MCP `listing` and `scope`.
   Tests: default hides, `all` shows, an exact get still finds it, an
   older peer is not filtered and does not see the field, an agent
   inherits, a person does not.
4. **Reload.** `session.reload` and `reloaded`, implemented in
   `vessel-session` beside move, using the existing stop and
   `session/load`. Refusals in ADR 0063, each with a test that no
   process was spawned or, for the cancel timeout, that the original
   process is still the session's. A failed replacement leaves the
   session `failed` and the log readable. Agents get `forbidden`.
   If this cannot be done without editing `vessel-agent`, stop and
   report; do not edit it.
5. **`vessel log` selectors.** Parse before connect. Order is from, then
   since, then filter, then tail. Follow waits on the stream. Tests use
   the testkit's fakes and an event, never a sleep and never a real
   agent.

The commit that first changes a wire type raises protocol 21 (or the
integrator's number), updates `docs/PROTOCOL.md`, regenerates
`web/src/wire`, and extends the store sentence in `docs/ARCHITECTURE.md`.
No row of the crate table changes.

## Tests the contract requires

- A key of 64 scalars is kept; 65, empty, a control, `=`, and `vessel.`
  are `invalid_request` and write nothing.
- A value of 256 is kept; 257, or a newline, is refused. An empty value
  is kept.
- The 33rd label is refused and the previous 32 stay.
- A patch changes the keys it names and no other. `{}` is refused.
- `session.list` with two pairs returns only a session that has both.
  `params: null` still decodes.
- A background session is absent from a current default list, present
  with `scope: all`, and present in `session.get`. An older connection's
  list includes it and the encoded info has no listing field.
- `session.create` with `listing: background` sent toward a negotiated
  protocol of 20 is refused by the client before the socket write.
- Reload of a terminal, a parked session, an ended session, and a
  session with no harness session id does not spawn. A running turn
  that does not end within the grace does not spawn a replacement.
  A replacement that fails readiness leaves one `failed` session and
  the old records in place. Success keeps the harness session id and
  appends `reloaded` without duplicating the earlier conversation.
- `vessel log --tail 0` prints nothing. `--tail -1` and `--filter typo`
  and a bad `--since` fail before connect. Follow's first print is the
  last 10 matching records when `--tail` is omitted.

## Resume point

- Done: slice 1, the ADR and this plan. No code.
- Half-done: nothing.
- Next: slice 2, labels, starting at `vessel-proto` (`LabelKey` and the
  `session.labels` row). Do not raise the protocol until that commit's
  wire types exist. Do not start reload or the log flags before labels
  and the listing are in.
