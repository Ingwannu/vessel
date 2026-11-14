# MCP parity

Branch `backend/m12-gk-mcp`. ADR 0065 is the contract. This file is what
a restart reads. Nothing here widens an agent principal past ADR 0013.

## Resume point

- Done: ADR 0065 and this plan. Docs only. No protocol number is
  registered; 21 is provisional until the slice that adds the fields.
- Half-done: nothing.
- Next: slice 2, the missing tools the ADR marks `missing`, each one
  a call the principal can already make. Do not start slice 3 until
  slice 2 is gated, or a review of slice 1 has been applied if one
  comes back first.

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
| `paseo permit ls\|allow\|deny` | `packages/cli/src/commands/permit/index.ts` |

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
   shows that principal the same requests; provisional protocol 21;
   an agent does not receive the owner's inbox. Refused rows stay
   refused (workspace archive and rename, scripts, `schedule.run`,
   browser, `speak`). `list_models` is not wrapped onto `model.list`.
   Each tool validates before the call and returns the node's typed
   refusal. Tests on the testkit's fakes, no real agent, no network.
3. **Policy at session start.** `mcp.inject` default false,
   `mcp.serve` default true, per-`HarnessRef` `{ enabled, disabled }`
   denylist. `NodeSettings` drops `Copy`. A running session is not
   rewritten. Unknown tool id from a client is refused. A stored
   name only a newer build knows is kept and disables nothing.
   Older peer: `Call::since` is 21 when the fields are set; a reply
   below 21 omits them. `docs/PROTOCOL.md` in this slice.
4. **HTTP.** Loopback child, `POST /mcp/agents`, no query, `AgentAuth`
   in the bearer and one session header, same principal as stdio.
   `Host` and `Origin` as the ADR says. `rmcp` feature
   `transport-streamable-http-server` only, size written into the
   ADR before it is enabled. No new crate. Off means no listener.
5. **`vessel permit ls|allow|deny`.** Owner principal only. `--all`
   is one session, not the node. Built if the CLI still lacks it
   after slice 2's list exists.

## Reuse decision

Modify `vessel-mcp` and the settings types. Do not add a crate, do
not adopt an HTTP library besides the `rmcp` feature the ADR names,
do not fork a second tool table. Paseo's TypeScript is the behaviour
to match, not code to move.

## Gate

From the repository root, in order: `cargo fmt --all --check`,
`node scripts/check-rust-size.mjs`, `node scripts/check-boundaries.mjs`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`. No `web/` step until a slice changes the
wire. A failure in a test this work did not touch is reported, not
waived.
