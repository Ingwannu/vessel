# MCP parity

Branch `backend/m12-gk-mcp`. ADR 0065 is the contract. This file is what
a restart reads. Nothing here widens an agent principal past ADR 0013.

## Resume point

- Done: ADR 0065 and this plan. Docs only. The first reading left the
  consumer, the upgrade default, the reload class, `permit` with no
  request id, and the status line open; this commit answers those.
  No protocol number is registered. `PROTOCOL_VERSION` stays 20.
  `MCP_PARITY` is the constant the integrator sets, and it is not in
  code yet. Status of the ADR is proposed.
- Half-done, not in this commit: a start on slice 2 in the working
  tree (`crates/vessel-mcp/src/tools/driving.rs`,
  `crates/vessel-mcp/src/tools/offer.rs`, and the edits to
  `tools.rs`, `tools/automation.rs`, `tests/automation.rs`,
  `tests/mcp.rs`). Keep it. Do not revert it. It is not gated, and
  the new dispatch must not match tool names as strings or return
  `serde_json::Value` past the edge (the ADR: tool ids are a closed
  enum).
- Next: finish slice 2, the missing tools the ADR marks `missing`,
  each one a call the principal can already make. Do not start slice
  3 until slice 2 is gated.

## What Paseo does

Read first, not copied. Tree paths below are inside Paseo 0.15.4.

| Fact | Where |
| --- | --- |
| Catalogue and every non-browser tool | `packages/server/src/server/agent/tools/paseo-tools.ts` (`registerTool` from line 1367; `create_agent` input from the schemas above line 1257; schedules from line 2647) |
| `speak` always on; `enabled !== false`; denylist | `packages/server/src/server/agent/paseo-tool-policy.ts` lines 14 to 29 |
| Policy shape `{ enabled?, disabledTools? }` | `packages/protocol/src/provider-config.ts` lines 31 to 34 |
| Absent policy means all tools; inject false wins; applied at create, resume, import, reload; child uses the child's provider id | `docs/data-model.md` section "Agent provider Paseo tools"; `docs/providers.md` on native tools versus MCP |
| `daemon.mcp.enabled` default true, `injectIntoAgents` default false | `packages/server/src/server/config.ts` lines 552 to 554 |
| HTTP route, shared bearer, query caller id, rebinding check off | `packages/server/src/server/bootstrap.ts` (token near line 720, route from line 1610, `callerAgentId` at line 1692); `packages/server/src/server/agent/runtime-mcp-config.ts` lines 33 to 64 |
| Open when no password; else shared bearer or daemon password | `packages/server/src/server/auth.ts` `isAgentMcpRequestAuthorized` |
| Browser tools, not this track | `packages/server/src/server/browser-tools/tools.ts` `registerBrowserTools` (22 tools, listed in the ADR) |
| `paseo permit ls\|allow\|deny` | `packages/cli/src/commands/permit/index.ts` lines 15 to 34 |
| `allow` with no request id allows every pending request; `--all` is an alias | `packages/cli/src/commands/permit/allow.ts` lines 84 and 130 to 132 |
| `deny` with no request id and no `--all` is "Request ID is required unless --all is specified" | `packages/cli/src/commands/permit/deny.ts` lines 29 to 35 and 67 to 68 |
| Both MCP switches and `agents.providers` are reloadable | `packages/server/src/server/daemon-config-store.ts` lines 184, 185, 202; `reload` at line 484 |
| Host Orchestration row "Enable Paseo tools" | `packages/app/src/i18n/resources/en.ts` lines 2918 to 2921; `paseo-tools-card.tsx` line 15; `host-page.tsx` line 294 |

There is no `all | selected | none` mode. `agents.skills.selection`
is a different setting (`{ mode: "all" }` or custom skill names) and
is not this work.

## What Vessel already has

Reuse, do not replace:

- `vessel-mcp` tools and `VesselTools::call` (`crates/vessel-mcp/src/tools.rs`).
  A new tool is another arm and a schema beside the others, calling
  `vessel-client` the way the existing arms do.
- Admission: `AgentAuth` on the local socket (`crates/vessel-node/src/ipc.rs`,
  `crates/vessel-proto/src/profile.rs`). HTTP must call that check,
  not a second one.
- Settings: `node.settings` / `node.set_settings`, field by field,
  `Call::since`, `for_protocol` (`crates/vessel-proto/src/settings.rs`,
  `crates/vessel-proto/src/rpc/since.rs`, `rpc/older.rs`).
- The node already spawns `vessel mcp` (`Config::vessel_mcp`). HTTP
  is another mode of that binary, supervised the same way. No
  production dependency from `vessel-node` to `vessel-mcp`.

Not reusable as a Paseo model list: `model.list` is the routing
table (ADR 0015). The ADR says so.

## Slices

1. **ADR and this plan.** This commit.
2. **Missing tools that do not widen.** Only rows the ADR marks
   `missing`: status via `session.get`, cancel via `session.send`,
   rename and set-option on a session the caller owns (not its own),
   `schedule.get` and `schedule.update` on a schedule it owns,
   `harness.list`, `profile.list` (no secret), `harness.options`.
   Pending permissions: a read call only if `session.read` already
   shows that principal the same requests; gated on `MCP_PARITY`,
   whose number is given at merge;
   an agent does not receive the owner's inbox. Refused rows stay
   refused (workspace archive and rename, scripts, `schedule.run`,
   browser, `speak`). `list_models` is not wrapped onto `model.list`.
   Each tool validates before the call and returns the node's typed
   refusal. Tests on the testkit's fakes, no real agent, no network.
3. **Policy at session start.** One `node.settings` key, `mcp`.
   A fresh directory stores `inject` false and `serve` false before
   it creates `node.id`. A directory that already has `node.id` and
   no `mcp` key is written `inject` true and `serve` false before any
   session starts. Per harness, a list of `{ harness, enabled,
   disabled }`, not a string-keyed map. `NodeSettings` drops `Copy`.
   A running session is not rewritten. Unknown tool id from a client
   is refused. A stored name only a newer build knows is kept and
   disables nothing. The fields are gated on `MCP_PARITY`; the number
   is given at merge. A reply below that constant omits them.
   `docs/PROTOCOL.md` in this slice. Reload class is in the ADR:
   inject and policy at the next session start, serve at once.
4. **HTTP.** Loopback child, `POST /mcp/agents`, no query, `AgentAuth`
   in the bearer and the header `Vessel-Session`, same principal as
   stdio. No agent is given the endpoint in this milestone, so the
   listener stays off until `mcp.serve` is set. `Host` and `Origin`
   as the ADR says. `rmcp` feature
   `transport-streamable-http-server` only, size written into the
   ADR before it is enabled. No new crate. Off means no listener.
5. **`vessel permit ls|allow|deny`.** Owner principal only. One
   session, not the node. `allow` with no request id answers that
   session's pending requests, and `--all` is the same act. `deny`
   with no request id and no `--all` is the usage error the ADR
   quotes. Built if the CLI still lacks it after slice 2's list
   exists.

## Reuse decision

Modify `vessel-mcp` and the settings types. Do not add a crate, do
not adopt an HTTP library besides the `rmcp` feature the ADR names,
do not fork a second tool table. Paseo's TypeScript is the behaviour
to match, not code to move.

## Gate

A commit that changes documents only is checked with
`cargo fmt --all --check`, `node scripts/check-rust-size.mjs` and
`node scripts/check-boundaries.mjs`. Clippy and the tests are not
run for that commit. A commit that changes code, from the repository
root, in order: those three, then
`cargo clippy --workspace --all-targets -- -D warnings`, then
`cargo test --workspace`. No `web/` step until a slice changes the
wire. A failure in a test this work did not touch is reported, not
waived.
