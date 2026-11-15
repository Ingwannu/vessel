# Vessel today (measured 2026-11-15 04:49 UTC), the side the plan builds on

## Two lines that have drifted apart

| Line | Branch / head | What it is |
|---|---|---|
| Node line | `backend/m12-node` @ `242076b5` (integrator) | 22 crates under `crates/` + `web/` (the page). Protocol 23, store schema 17, 172 RPC methods. |
| App line | `backend/m12-spike` @ `1d1f11df` (native lead; build 0.0.0.0.18) | `spikes/native/` (app, kit, link, view, web) on top of root crates at `56a7cbd3`: protocol 15, 156 methods. |

The app line's root is **246 commits and about 60,000 lines behind** the node line
(`git diff --stat 56a7cbd3 242076b5 -- crates web`: 519 files, +60654 / -2153). Methods the node has and
the app cannot call yet: `agent.history`, `device.self`, `fleet.published`, `inbox.get|list|mark`,
`push.key|presence|subscribe|unsubscribe`, `session.side_open|side_prompt|side_reset`,
`usage.limits|lookup|query`. Every app package that needs one of these is blocked until the node line is
merged into the app line. This is the first structural step of the plan (section "Step 0").

## Node crates (node line)

| Crate | Role |
|---|---|
| vessel-proto | wire contract: ids, sessions, events, the RPC method table (`src/rpc.rs`, `rpc_methods!`), protocol versions |
| vessel-rpc | frames over any transport: handshake, call correlation, subscriptions |
| vessel-secure | end-to-end channel (Noise NK) for relayed and direct links |
| vessel-relay | the relay server, a node's registration link |
| vessel-client | typed async client with gap-free session streams (used by CLI, page bridge, native link) |
| vessel-node | the daemon `vesseld`: sessions, fleet, devices, schedules, inbox, push, usage, settings |
| vessel-session | one actor per session, gap-free event numbering |
| vessel-store | data directory: identity, token, SQLite store and migrations |
| vessel-agent | ACP client: drives a harness or adapter over stdio |
| vessel-harness | everything harness-specific: discovery, version probes, terminal harnesses |
| vessel-pty | pseudo-terminal runner |
| vessel-proc | process primitives: command specs, process groups, group kill |
| vessel-workspace | projects, worktrees, git, files |
| vessel-forge | pull requests on a forge (GitHub first) |
| vessel-lsp | bounded read-only language-server context |
| vessel-llm | model APIs for the routing proxy (wire types, SSE) |
| vessel-mcp | `vessel mcp`: the MCP server giving a session's agent the Vessel tools |
| vessel-fleet | fleet bundles: manifests, materialiser |
| vessel-remote | reaching nodes over SSH: runner, platform detection, install |
| vessel-cli | the command-line client |
| vessel-web | the local web UI server and its embedded page |
| vessel-testkit | fake harnesses (fake-tui, fake-acp, fake-ssh, fake-tool) |

## Native app (app line, `spikes/native`)

Crates: `kit` (controls, theme, fonts), `view` (pure state, words, rules; no GPUI), `link` (talks to
nodes: one actor, Command / Event), `app` (GPUI screens: `app/src/areas/*`), `web` (page bits).

Areas present: about, archive, board, composer (attach, drafts, files, modes, offers, running, slash),
conversation (about 40 files: timeline, tool, permission, question, plan, queue, fork, rewind, find,
search, meter, ended, follow …), draft (new session: book, pickers, effort card, probe), git (menu,
branch, commit, stashes), pair, palette, pulls (card, facts, feedback, form, link, merge), session,
settings (agents, appearance, chat, connections, devices, diagnostics, general, host agents, language,
motion, overview, sync, …), shortcuts, sidebar (display, machines, row menu, statuses), toasts,
workspace (approval, code, detail, projects, start card …).

Shared files at their size limits: `app/src/shell.rs` (500), `link/src/lib.rs` (988 recorded).

## Review and build machinery (unchanged)

Reviewers V1 (strict app tracks), V3 (node takeover tracks), V4 (new packages). Whole gates on the build
machine `build-vm` (3 slots). Rules in `~/refs/notes/briefs/m12-grok-common.md` A1-A14.

## Scope rule from the operator (2026-11-15 04:5x UTC)

Vessel may do more than Paseo. Paseo parity is the floor, not the ceiling: extras such as a PDF viewer,
an SVG viewer and language-server features (Vessel already has `vessel-lsp`) are welcome. The plan
marks them "beyond Paseo"; they come after the parity packages in their area, never instead of them.
