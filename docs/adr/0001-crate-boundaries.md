# ADR 0001: Crate boundaries and dependency direction

Status: accepted, 2026-11-05. The edges each later milestone added are
recorded in the decision below (milestones 2 to 4, 2026-11-06).

## Context

Phase 0 had three crates: `vessel-core` (types), `vessel-node` (everything
else, including process handling and HTTP) and `vessel-cli`. The backend is
about to grow a PTY runner, an ACP client, persistence, a typed RPC layer and
a client library, built by two agents in parallel. Without hard boundaries
this becomes Paseo's `session.ts` or Zed's `acp_thread.rs`.

## Options

1. Keep three crates, split by modules. Cheap, but module privacy is weak,
   every change recompiles the whole node, and two agents would edit the same
   crate.
2. Fine-grained crates by responsibility (Zed has ~250). Boundaries are
   enforced by the compiler, incremental builds stay small, and each agent
   owns whole crates.
3. A handful of mid-size crates (protocol, runtime, server, client).
   Middle ground, but "runtime" would mix PTY, ACP and supervision.

## Decision

Option 2, with this map and only these dependency edges:

```
vessel-proto    (wire contract; no I/O, no tokio)
vessel-rpc      -> proto
vessel-proc     (process spawning, process groups, kill tree)
vessel-harness  -> proto, proc
vessel-pty      -> proc
vessel-agent    -> proto, proc
vessel-lsp      -> proto, proc            (milestone 6)
vessel-workspace -> proto, proc           (milestone 11)
vessel-session  -> proto, harness, pty, agent, proc (milestone 11)
vessel-store    -> proto
vessel-node     -> proto, rpc, harness, session, store, secure, relay,
                   fleet, client (milestone 4), lsp (milestone 6),
                   llm (milestone 5), workspace, forge, proc (milestone 11)
vessel-client   -> proto, rpc, secure, relay
vessel-remote   -> proto, rpc, client      (milestone 2)
vessel-secure   -> proto                   (milestone 3)
vessel-relay    -> proto, secure           (milestone 3)
vessel-fleet    -> proto, harness          (milestone 4)
vessel-mcp      -> proto, client           (milestone 4)
vessel-llm      -> proto                   (milestone 5)
vessel-forge    -> proto, proc             (milestone 11)
vessel-web      -> proto, client           (milestone 7)
vessel-cli      -> proto, client, node, remote, mcp, harness, web
vessel-testkit  (fake harness binaries for tests; dev-dependency only)
```

Milestone 2 (2026-11-06) added `vessel-remote`, which runs the system
`ssh`, installs `vesseld` on remote machines and connects to them over SSH
stdio (ADRs 0007 and 0008), and the edge `vessel-cli -> remote`. It starts
`ssh` with `tokio::process` directly: one process, no process group, so no
`vessel-proc` edge.

Milestone 3 (2026-11-06, Track S) adds `vessel-secure`, the end-to-end
secure channel (ADR 0009). Its one edge, `-> proto`, is for the key types
that travel on the wire (`NodePublicKey`, `RoutingId`, the relay's
`HandshakeMessage`); everything computed from them with cryptography stays
in `vessel-secure`, so `vessel-proto` stays free of it. `vessel-relay`
(`-> proto, secure`) is the relay server, the node's link to it and the
client's dial, and the websocket carriers both ends use. Track P added
`vessel-node -> secure` (answering handshakes with the node's key pair) and
`vessel-client -> secure` (initiating them to a pinned key, and
fingerprints). `vessel-node` gains `-> relay` for `vesseld relay` and, with
Track P's integration, `/secure` and relay data connections. The client's
`-> relay` (Track P's integration) is for `connect`, `dial`, `WsCarrier`,
the close codes and `TlsConfig`: `SecureConnector`'s websocket dialer
(`WsDialer`).
`vessel-testkit` (dev-only) has a `relay` feature: a misbehaving relay
proxy built on `tokio-tungstenite`, with no edge to Vessel crates.

Milestone 4 (2026-11-06, Track F) adds `vessel-fleet`: fleet bundles
(ADR 0012), the materialiser that writes them into harness homes, and MCP
secret resolution. It moves no data between machines (the node does); its
edges are `-> proto`, for the manifest and MCP types, and `-> harness`,
for each harness's home layout (`vessel_harness::layout`: the instruction
file and skills directory names a target is built from).

Milestone 4 (2026-11-06, Track H) adds `vessel-mcp`: `vessel mcp`, the
MCP server a node attaches to every session so its agent can drive
sessions (ADR 0013). It is a client like any other, so its edges are
`-> client` (the local-socket connection with the agent's credential) and
`-> proto`; `vessel-cli -> mcp` runs it. The MCP protocol itself comes from
the `rmcp` SDK (plan decision 8). `vessel-cli -> harness` is for `vessel
init` (F5): which harnesses are installed, their install and login
commands, and other accounts' homes, all of which only `vessel-harness`
knows.

Milestone 4 (2026-11-06, Track F2) adds `vessel-node -> fleet` (the
home bundles, members materialise: `vessel-fleet`'s mechanisms under the
node's policy of when) and `vessel-node -> client`: a member calls its
fleet's home (`fleet.manifest`, `fleet.blob`, `fleet.watch`,
`fleet.report`) and joins it by pairing, as any device does, through
`vessel-client`'s `SecureConnector` and `pair_over`. `vessel-client` does
not depend on the node, so there is no cycle.

Milestone 5 (2026-11-07, Track M5) adds `vessel-llm`: model APIs for
the node's routing proxy (ADR 0015): the Anthropic Messages, `OpenAI`
Responses and Chat Completions wire types, Server-Sent Events, the
translation between them and the calls to upstream providers. It knows
nothing of sessions, nodes or harnesses, so its one edge is `-> proto`
(the opaque `RawJson` passthrough type and the usage types sessions
record). `vessel-node -> llm` is the proxy itself: who may call it, with
which route and key, and where usage is recorded. How a harness is
pointed at the proxy stays in `vessel-harness`.

Milestone 7 (2026-11-07, Track M7) adds `vessel-web`: `vessel ui`'s
loopback server, its embedded single-page app and the bridge from the
browser to the book's nodes (ADR 0017). The bridge is a client like any
other, so its edges are `-> client` and `-> proto`; `vessel-cli -> web`
runs it and gives it, through a seam, the CLI's knowledge of how a book
entry becomes a connection.

`vessel-core` is renamed `vessel-proto` because its one job is the wire
contract.

## Why

- The compiler enforces the boundaries; reviews do not have to.
- Each piece can be tested alone: the ACP mapping without processes, the
  store without a node, the RPC layer over an in-memory transport.
- Two agents can work in parallel on disjoint crates.
- `vessel-session` does not depend on `vessel-store`: the session runtime
  hands the node a bounded channel of records to persist. Persistence policy
  stays in the node, and the runtime stays testable without a disk.

## Consequences

- More `Cargo.toml` files and some re-exports.
- A new edge needs an update to this ADR and `docs/ARCHITECTURE.md`.
- Shared helpers have no grab-bag home; they go to the lowest crate whose job
  they are.

## Milestone 6 addition (2026-11-07)

`vessel-lsp -> proc` owns the language-server transport, saved-document
positions and later its process lifecycle (ADR 0016). Authentication stays
in the node. Agent-facing types and the node integration add their edges
when those steps land; the initial framing/position foundation needs no
wire dependency.

M6 runtime adds `vessel-lsp -> proto` for read-only context requests and
results, and `vessel-node -> lsp` for lifecycle assembly. The node owns grants
and configured roots; the LSP crate owns mechanisms. Test-only `vessel-mcp
-> lsp` sets an explicit fake registry; production MCP still uses only proto
and the client, never the server runtime.

## Milestone 11 addition (2026-11-11)

`vessel-workspace -> proto, proc` (track B, ADRs 0023 and 0027): git run
as a hardened program, worktrees, snapshots, checked paths and file
operations, the name index and the project's file. It knows no principal,
store or session; its tests need only git. `vessel-node -> workspace`: the
node decides who may call, keeps projects, worktrees and approvals in its
store (schema 9), runs clones and setups as its tasks, and ties worktrees
to sessions, as it does for `vessel-lsp`.

`vessel-forge -> proto, proc` (track B2, ADR 0029, 2026-11-12): pull
requests on a forge behind one trait (GitHub first), signing in with a
token the binary found or the machine's `gh` (run through `vessel-proc`,
no shell), conditional reads and rate limits, and an in-memory forge for
tests (feature `fake`). It knows no session, store or principal: which
sessions are watched, what is sent to them and who may call is the
node's.

`vessel-node -> forge`: the node decides which sessions are watched,
when, for whom, and what becomes of the answers (the board, feedback
through the queue, the calls); the provider makes the requests.

Wave 2, track C (2026-11-12, ADR 0031): `vessel-session -> proc`, since a
project script's run is spawned piped in its own process group (a
kill-on-close job object on Windows) by the same runtime that keeps its
log, and a shell's session is swept after it exits; `vessel-node ->
proc`, for the command a terminal runs (`CommandSpec`), the account's
shell and the ports its trees listen on (`vessel_proc::listening`). Both
are mechanism the crates already used through others; the node still
decides who may open a terminal and where.

## Native attachment diagnostics

The native link also names `vessel-rpc` directly for `CallError` when an
attachment read or resume fails. `vessel-client` already uses and returns
this type; preserving it avoids replacing the source with a string. The
edge adds no external crate, protocol operation, or runtime subsystem.
Its authoritative workspace table is `spikes/native/README.md`; ADR 0025
and `docs/plans/wp-a8.md` describe the preview contract.
