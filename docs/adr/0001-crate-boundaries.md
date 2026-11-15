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
vessel-agent    -> proto, harness, proc   (harness: milestone 12)
vessel-lsp      -> proto, proc            (milestone 6)
vessel-workspace -> proto, proc           (milestone 11)
vessel-session  -> proto, harness, pty, agent, proc (milestone 11)
vessel-store    -> proto
vessel-node     -> proto, rpc, harness, session, store, secure, relay,
                   fleet, client (milestone 4), lsp (milestone 6),
                   llm (milestone 5), workspace, forge, proc (milestone 11),
                   agent (milestone 12)
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
vessel-testkit  -> proto (feature `authority-fixtures`, milestone 12)
                (fake harness binaries for tests; dev-dependency only)
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

Milestone 12 (recorded 2026-11-14, when `scripts/check-boundaries.mjs`
began comparing `docs/ARCHITECTURE.md`'s table with the manifests; the
first two edges had been in the manifests without this record):

- `vessel-agent -> harness` (ADR 0057): a direct driver's private launch
  file is written by `vessel-harness` (`claude_mcp_file`, `McpFile`), the
  one owner of that format and of the file's checks.
- `vessel-node -> agent` (2026-11-12, protocol 11's `agent.status`, ADR
  0032): what a probe's handshake said of an agent reaches the node as the
  agent crate's typed outcomes (`ProbeError`, `Disconnect`, `Refusal`)
  instead of a string; the history, forking and side-question calls name
  its inputs (`HistoryFrom`, `ForkFrom`, `PromptContext`). The node still
  speaks no ACP: the session runtime runs the agent.
- `vessel-testkit -> proto` (protocol 20): typed caller-authority fixtures,
  optional behind the `authority-fixtures` feature, so the fake binaries
  keep no wire dependency.

From here the check holds the map: a manifest edge missing from the table,
or a table edge missing from the manifests, fails the gate.


## Node-control process mechanisms

Status: approved contract. The query move is implemented below; the later
CLI edge remains proposed until its real caller. Each edge changes its
manifest and active crate-map row in its own implementation commit.

[Decision Log]
- Purpose and intent: keep selected-file kernel observations and later process
  verification beside the existing process mechanisms, with one decoder and
  no signal authority inferred from a kernel PID alone.
- Existing implementation and constraints: `vessel-store::NodeLock` owns the
  data-directory lock and owner record. Its private Linux module observes
  `/proc/locks`. `vessel-proc` already owns process groups, process inspection
  and the workspace's existing Nix signal calls. The CLI currently consumes
  the lock observation through the node/store facade; it sends no signal.
- Alternatives considered:
  - Keep kernel lookup in store and hand proc only identifiers. This leaves
    the relationship between the selected file, the kernel row and the live
    descriptor split between crates, and encourages exporting PID getters
    before there is a verified process. Passing a bare PID is not the proof.
    FDinfo uses the same `lock_get_status` line decoder as `/proc/locks`,
    so retaining it in store would require a duplicate decoder in proc or
    the rejected reverse dependency.
  - Add another small crate. That adds a process-mechanism owner and more
    edges beside an existing owner; it would also split the existing Nix
    ownership or move it without a separate need.
  - Make proc depend on store. That gives a general OS primitive knowledge
    of node data layouts and persistence, reversing the current layering.
    Adding the forward edge later would also make a cycle.
- Selected approach: first move the generic selected-file kernel query into
  proc without changing behavior, adding `vessel-store -> vessel-proc`.
  `NodeLock::kernel_holder` and the existing re-export paths remain the
  store/node-facing API. Add `vessel-cli -> vessel-proc` only with the real
  force/restart caller, not with a placeholder or an unused wrapper.
- Why this approach: proc owns OS mechanisms without owning the meaning of
  a node's directory. Store supplies the selected lock path and maps the
  original I/O cause as before. The CLI owns authenticated target binding,
  live-session consent, deadlines and the choice of lifecycle operation.
- Advantages, costs and effects: two deliberate edges at separate phases,
  more focused tests, and a single process-inspection owner. There is no new
  crate or external dependency. The moved module remains read-only and uses
  the invoking account's permissions. No extra credential, environment or
  arbitrary command from an owner record is introduced.

What must stay in store: `DataDir`, the lock filename and owner-file format,
`NodeOwner`, record reads/writes, lock acquisition and SQLite. Proc receives
an OS path or file, never a node identity, principal, database or settings.
Node/client authentication and live-session policy must not migrate there.

A `/proc/locks` match remains an observation. Linux can retain a dead
creator's PID while an inherited open file description still holds a flock.
A later capture of the reused PID's start generation cannot establish the
missing initial ownership. Neither a matching owner record nor an executable
name alone may turn this observation into a signal target.

The subsequent verification must relate the candidate's current descriptor
of the selected locked file description to its process generation and PID
namespace, recheck before use, and refuse ambiguity. Linux
[`fs/proc/fd.c`](https://github.com/torvalds/linux/blob/master/fs/proc/fd.c)
looks up the task's actual descriptor before `show_fd_locks`;
[`fs/locks.c`](https://github.com/torvalds/linux/blob/master/fs/locks.c)
compares the open file description and owner before printing that FD's lock.
An open proc directory does not reserve a numeric PID
([kernel proc documentation](https://docs.kernel.org/filesystems/proc.html)).
A remaining final-check-to-numeric-signal race must stay explicit unless an
implemented primitive removes it; this contract does not claim to do so.

Implementation and verification sequence:

1. Behavior-preserving move, its own commit and whole gate. Keep the
   `NodeLock` API, typed I/O sources, bounds, selected-file-first decoding,
   duplicate-writer refusal and `NotVisible` fallback unchanged. Move private
   decoder tests with their rules. Keep store's data-directory and preserved
   owner-byte assertions there; relocate the private PID assertion beside the
   OS fixture in proc without removing or weakening it. Do not expose a getter
   solely for a test, change head cases, or mix in parser cleanup.
2. Descriptor/generation/namespace proof, a separate commit and whole gate.
   Owe a real held-file case, an unrelated live candidate that lacks that
   locked description, stale/disappeared and replaced generations, namespace
   disagreement, unreadable/oversized observations, and typed refusal without
   record mutation. An absent row or missing process is never a free lock.
   Unavailable platform fixtures are reported as unverified, not inferred
   from Linux decoding tests. No signal is added in this phase.
3. Actual CLI force caller, another commit and whole gate. Owe consent and
   remote/drain refusals before signals, reserved/self-PID protection,
   record/executable disagreement, rechecks before each single-process signal,
   command/kill deadlines and real OS-lock completion. Signal tests use an
   external isolated child, never the in-process test node or a process name.
   Windows and unconfirmed macOS paths retain ADR 0064's explicit refusals.

Paseo `packages/server/src/server/daemon-instance.ts` 197–218 refuses
reserved/self PIDs and checks instance replacement. Its Unix graceful path
sends SIGTERM to the PID (149); its escalation uses the imported `tree-kill`
wrapper (15–24, 215–218). Vessel targets only the verified node process,
not a name or arbitrary process tree; this narrower target avoids harming
unverified descendants. Its descriptor and namespace proof is an additional
safety floor, not a claim that Paseo performs it.
This amendment is reviewed before implementation; the active map changes
only in the commit that adds each manifest edge.


### Binding implementation conditions

The contract and first edge have approval. These conditions bind the move
and the later caller:

- The move adds only `proc` to store's active dependency cell; proc's cell
  remains `-`. The manifest, table and the following name restriction land
  in the same behavior-preserving commit.
- Add a `Names only some files may use` row restricting `vessel_proc` in
  `vessel-store` to its lock module (`src/lock/`). That module may delegate
  only the lock query, not spawn a child or reach a signal/group/session
  cleanup operation. Store retains its data layout and owner-record policy.
- When the real CLI caller lands, add a name rule restricting proc's signal
  entry point in `vessel-cli` to the force module. The graph edge does not
  grant other CLI modules a second signalling path.
- The move must remain readable as a move with `git diff -M`: no parser
  cleanup, API change, changed bound or error source, weakened assertion,
  different duplicate-writer handling, or changed `NotVisible` fallback.
  A kernel PID alone continues to grant no signal authority.


### First implementation: the generic lock query moves to proc

The integration decision approved the first edge under the binding
conditions above. The behavior-preserving move adds only proc to store's
manifest and active dependency cell; proc's cell stays empty. Store's
`lock/query` alone may name `vessel_proc` and delegates only the query.
The decoder moves as a file (`git diff -M`), keeping the existing bound,
selected-file-first parsing, duplicate-writer refusal, typed I/O sources
and fallback. Its previous tracing target is retained explicitly so an
existing log filter does not change as a side effect of the move.

The opaque snapshot types move with their decoder, retaining the public
store/node re-export paths. Private PID assertions move beside an actual
held-file fixture in proc; store keeps its data-directory, owner-byte and
damaged-record assertions. That fixture uses standard-library exclusive
creation, adding no external dependency. No signal, descriptor/generation
verification or CLI edge is introduced. Those remain separate phases.

## Checked owned process cleanup (W16 shared prerequisite)

The additive proc-owned Child::wait_checked retains the first group/job sweep
result while legacy wait keeps its leader-status behavior. It returns a private
OwnedGroupExit receipt or concrete typed wait/first signal cause; no delayed
numeric group-id retry is introduced. SignalError privately shares its original
io::Error so the existing signal API and a retained failure share one concrete
cause without string conversion. The receipt is leader reap plus accepted
cleanup request, not observed death of every descendant or account process.
ProcessLifetime revocation and durable PID identity do not mint it. No crate
edge, dependency, wire field or management caller is added by this mechanism.
The source/validation record is docs/plans/checked-process-cleanup.md.

## Durable process identity and checked helper signals (proposed)

[Decision Log]
- Purpose and intent: a provider helper left after a node restart must never
  be confused with a different process that received its old PID. One generic
  primitive owns durable kernel identity; the node owns the ledger and policy.
- Existing implementation and constraints: proc already owns process creation,
  selected-file observations and a bounded start-time decoder. A runner's
  in-memory lifetime observation identifies one issuance and can be revoked;
  it is not a durable boot/generation record. An owner record or bare PID is
  no signal authority. Libraries read no configuration or user home here.
- Alternatives considered: another node-local proc parser, a persisted group
  number, numeric PID checks before kill, unsafe syscalls, or a new OS wrapper.
  A second parser drifts; group IDs and numeric PIDs can be reused; unsafe is
  forbidden. Existing workspace rustix already provides safe pidfd operations.
- Selected approach: `vessel_proc::identity` captures an identity from a real,
  still-owned Child, and verifies it as Same, Gone or Reused. The serializable
  value has private validated PID, kernel start ticks, boot identity, namespace
  context and executable identity where available. Reuse the canonical crate
  root ProcessId; never add a second public PID type. Share the existing stat
  decoder, preserving its opaque-command and bounded-read rules.
- Why this approach: capture borrows the actual Child, refusing already reaped
  or released ownership. Repeated kernel observations detect a changed record;
  boot and namespace context prevent coincident identifiers in another context
  from being treated as the original helper. Read/permission/format failures
  remain typed errors, never Same. Other operating systems are Unsupported.
- Advantages, costs and effects: proc gains no ledger, restart loop, provider
  knowledge, data directory, principal or wire dependency. The node decides
  whether and when to reap, records its own helpers, and retains uncertain
  entries. Only a root process is in scope; no group or descendant sweep is
  inferred from a persisted PID. In-memory lifetime revocation remains distinct
  from direct-child exit observation and durable identity verification.

The later single-process `signal_verified` boundary opens a Linux pidfd first,
verifies the stored identity while retaining that descriptor, then sends the
signal to that descriptor. It never falls back to numeric kill. Gone or Reused
sends nothing; a missing pidfd facility is Unsupported. This closes the
verify-to-signal PID-reuse race; it does not claim that executable metadata
cannot change through exec after its observation. Node reap policy stays in
its provider-helper module and uses this one boundary. The CLI force module
may only consume it after its separate local-authentication and consent proof.
Name restrictions for actual signal consumers land with their callers.

Cancellation must not leave an effectful detached observer: potentially
blocking preparation only reads and retains handles. Cancelling it cannot
send a later signal. Sending uses the retained pidfd only after successful
preparation returns to the still-polled caller. Capture keeps a real Child
borrow across its observation and must not race another wait/detach of that
handle. The issuer hook accepts only Running; Reaped/Released refuse. Capture
also relies on the runner's exclusive child-reaping discipline; no caller may
reap the same child through an independent wait or ignore its exit status.

External reuse, before manifest changes:

- Existing workspace serde/serde_core (MIT OR Apache-2.0) supplies the typed
  ledger encoding. Standard Rust has no interchangeable serde implementation;
  a second ad-hoc ledger grammar would not compose with the existing node file
  helpers. Inspected source trees for serde 1.0.229 and serde_core 1.0.229 occupy
  668 KiB and 480 KiB respectively. These are local source-disk measurements,
  not linked-binary costs. They already occur in the workspace lockfile and
  node dependency graph; no new external package or version is introduced.
- Existing workspace rustix 1.1.6 adds its process feature on Linux. Its
  manifest offers Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT; its
  inspected source tree occupies 3412 KiB (again source disk, not binary size).
  The standard library's pidfd interface is unavailable on the pinned stable
  toolchain, and enabled nix has no safe pidfd open/send interface. Rustix is
  already locked; no FFI, unsafe block or additional OS wrapper is introduced.

The active internal graph remains unchanged: proc still has no Vessel
normal dependency. The implementation commit updates its module/job description
and manifests together; the node keeps its existing proc edge.

Owed tests: owned fake capture and serde round trip; exact Same, actual child
exit, altered generation/boot/namespace/executable and malformed identities;
reaped/released capture refusal; safe errors without record contents; pidfd
signal delivered only to an isolated owned fake; Gone/Reused/Unsupported send
nothing; a retained dead pidfd cannot redirect a signal to a new helper;
cancellation cannot trigger a late signal; non-Linux explicit refusal. Actual
same-numeric-PID reuse or namespace mutation is not inferred from synthetic
fixtures. The kernel API's stable-fd guarantee and each untested OS/context
must be separated from runtime evidence.

Primary references: Linux [pidfd_open](https://man7.org/linux/man-pages/man2/pidfd_open.2.html),
[pidfd_send_signal](https://man7.org/linux/man-pages/man2/pidfd_send_signal.2.html)
and [proc documentation](https://docs.kernel.org/filesystems/proc.html).
Paseo daemon-instance.ts 149 and 197-218 uses an instance PID, replacement
checks and tree cleanup. Vessel deliberately uses a verified root process
and pidfd, with explicit unsupported outcomes instead of a name/group guess.


### Supported initial identity issuer

Initial identity capture is opt-in. `identity::spawn` establishes Tokio's safe
SIGCHLD registration before creating the actual Child, retains the registration,
obtains a pidfd and initial evidence, and retains that seed before exposing the
child. Ordinary
spawn, a missing seed and already reaped/released ownership cannot be upgraded
by reading a delayed numeric PID. `capture` accepts only a seeded Running child.
Ordinary spawn/drop/detach behavior remains unchanged.

This uses Vessel's explicitly owned single SIGCHLD/reaping environment. Exclusive
signal and reaping ownership is an integration precondition, not inferred from
Running. Tokio caches registration; its first installation replaces inherited
SIG_IGN/SA_NOCLDWAIT behavior. An embedding that changes dispositions after that
installation, or independently reaps this child, is outside the support scope.
Creating another Tokio signal stream cannot repair a mutated cached registration.
No post-spawn pidfd_open is described as atomic process creation.

Registration, seed or evidence failure returns a typed error while retaining
ordinary owned-child cleanup. No failed or cancelled preparation can publish a
seed, leak an owned child, or signal later. Read-only preparation may briefly
retain bounded handles after cancellation; the live caller owns any later effect.
The implementation reuses the existing spawn/group/job engine through an internal
option, rather than creating a second spawn implementation. Its issuer and detach
hooks must preserve the other process owners' lifetime revocation and cleanup.
The initial issuer is Bound only; no current caller requires identity capture of
a releasable child. Ordinary releasable spawn remains unseeded and Unsupported.

Owed tests additionally cover registration/seed/evidence failure cleanup, a cached
registration within the supported environment, short-lived children, unseeded/
auto-reaped boundary refusal, release/reap refusal and cancellation after held
read preparation. The generic embedding case remains explicitly unsupported.
A separate atomic pidfd-spawn OS wrapper is not introduced; stable standard
library support may be considered when it exists.


### Shared start-time decoding preparation

[Decision Log]
- Purpose and intent: durable identity must reuse the lock observation's stat
  layout instead of growing a second field-position and opaque-command parser.
- Existing implementation and constraints: the bounded Record reader and
  generation owner already decode PID before comm/start ticks. Their typed
  failures and existing assertions must not change during extraction.
- Alternatives considered: duplicate the parser, move lock policy into identity,
  or make an eagerly PID-validating generic record. The first drifts, the second
  couples unrelated ownership, and the third changes the old refusal order.
- Selected approach: a private Linux proc_stat decoding edge splits the borrowed
  prefix and decodes start ticks. The current generation caller still validates
  its typed PID first, then transfers each original format cause into its same
  error variant. The existing combined generation tests remain byte-identical
  and exercise the extracted decoder through that actual caller.
- Why this approach: one owner retains the kernel format positions, while PID
  admission and namespace proof stay with the observation. No raw record is
  exposed publicly or retained in domain state.
- Advantages, costs and effects: no dependency, syscall, cap, public API, Child
  hook, identity issuer or signal is added. This behavior-preserving preparation
  still owes changed-crate execution and the enclosing identity package gate.
