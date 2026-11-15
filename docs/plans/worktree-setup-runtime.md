# Runtime follow-up to ADR 0093: structured setup observation and cancellation

Status: proposed, D2. Protocol assignment pending; no new wire call is
available until the node integrator assigns the feature constant.

## Decision Log

- Purpose and intent: show the actual sequential setup commands, their
  output, result and elapsed duration; stop the exact setup generation
  when requested without removing the worktree.
- Existing implementation and constraints: `vessel-workspace::commands`
  owns spawning, bounded output and process-group cancellation. The node's
  `workspace::setup` owns background task lifetime and persists the legacy
  `SetupState`. `worktree.output` supplies one combined log, which cannot
  establish command boundaries or distinguish cancellation from signal exit.
- Alternatives considered: parse the printable `$ command` log markers;
  replace the runner; observe its actual structured execution boundaries.
  Parsing is ambiguous because commands can print those same markers;
  replacing the runner would duplicate quoting, credential removal and
  process-group handling.
- Selected approach: extend the existing runner with synchronous borrowed
  events through a generic callback. The runner emits command start, raw
  output and terminal result with a monotonic duration. The old `run`
  entry point discards events and keeps its return shape. An I/O failure
  produces an explicit observation failure, never a successful command.
  The node will own bounded snapshots, generation-specific cancellation
  and authoritative workspace/trust binding above this runner boundary.
- Why: the event source knows when a process is actually started and
  reaped. It does not know principals, registry identity, SQLite or RPC;
  those remain node/store/protocol responsibilities (ADR 0001).
- Advantages, drawbacks and impact: old callers and approvals retain their
  behavior. Observers never await or perform network/persistence work themselves.
  A slow synchronous callback still delays the runner and can backpressure the
  bounded output pumps, so this callback only updates bounded local memory and
  wakes readers. Legacy state persistence and network delivery stay outside the
  callback; structured observations remain volatile. Runner events alone do not
  make observation a supported public capability.

## Reference behavior and remaining integration

Paseo 0.15.4 `packages/server/src/server/worktree-bootstrap.ts:33-125`
keeps 64 KiB per command with middle truncation; `:240-265` projects
command status/duration/log. `workspace-setup-runtime.ts:27-36` aborts and
waits for a running setup. `worktree-session.ts:360` blocks automatic
automation for an untrusted source. The client setup panel
`packages/app/src/panels/setup-panel.tsx:265` renders waiting, command
results and blocked-source state. These facts must come from typed state,
not terminal text.

W11's registry owns stable WorkspaceId, memberships and durable source
trust. W8 consumes that reviewed boundary rather than inferring ownership
from cwd. The existing WorktreeId remains the runner resource identity;
one path may have multiple workspace owners. Block release must be an
explicit persisted transition, separate from rerunning a failed setup.

Automatic terminal bootstrap is W8's follow-up after approved setup
success. It reuses the credential-scrubbed interactive terminal shell and
first-output/readiness seam. A ProjectCommand is typed into that shell;
it is not the service-port helper's executable/argument primitive.
ADR 0066 requires file-order sequential bootstrap; Paseo's implementation
at `worktree-bootstrap.ts:319-343` currently uses `Promise.all`, so that
ordering is an explicit Vessel policy difference. Terminal-only plans
must check approval before creating any terminal.

## Validation

Focused runner tests exercise forged log markers, sequential command
results, cancellation before spawn and during execution, and stop after a
failed command. No real agents or external services are involved. The
node integration additionally needs bounded output, stale cancel identity,
restart/blocked-source persistence, authorization and lifecycle tests.

Runner slice validation on the assigned 29eebb8b base: focused
`cargo clippy -p vessel-workspace --all-targets -- -D warnings` passed;
`cargo test -p vessel-workspace --lib commands::` passed 5/0. Size and
crate-boundary checks passed. This is runner evidence, not a public RPC,
durable blocked-source or native UI claim.

## Additive node contract (pending)

The single method table registers `worktree.setup_progress` (Read) and
`worktree.cancel_setup` (Operate) with WORKTREE_SETUP_OBSERVATION=None.
Both ends refuse pending methods before dispatch. Agent callers are refused
by the workspace policy; forwarded calls retain destination scope/principal
checks, and delegated readers do not hold a long poll. Existing worktree
resources are node-global person-managed resources, as `worktree.setup`;
these methods do not grant a browser service Preview or infer workspace owner.

Progress uses an invocation UUID plus checked revision cursor. A stale cancel
returns Stale and never signals a newer setup or teardown. Requested is a
signal receipt, not completion; only the runner's terminal snapshot reports
Cancelled. Logs are separately bounded per command at 64 KiB with a typed
truncation flag and an inserted middle marker; the legacy combined 1 MiB log
remains unchanged. Observations are volatile like Paseo's runner snapshots;
a restarted node returns absence, never reconstructed command boundaries.
Durable blocked-source observation and trust release remain behind W11's
registry boundary and are not advertised by these worktree-only methods.

Consumed exactly W11's source-reviewed e45a5d89 identity-only commit in
merge 0d536c6b. Registry/store/trust ownership remains unconsumed. The
pending node integration is awaiting changed-crate Clippy/tests and wire
generation; it is not consumer-ready.

W11 has now proposed (source-only adb81d77, not yet adopted) canonical
registry BlockedSource::ChangeRequest with RepoRef/PullNumber and
WorkspaceAutomation Allowed/Blocked. The eventual workspace setup view must
render the current registry gate ahead of old backing-worktree history: a
blocked workspace must not appear ready because another invocation succeeded.
Release uses the exact workspace/source/revision CAS and starts nothing by
itself. D2 then checks Allowed and obtains an authoritative start lease before
automatic bootstrap; unknown or unavailable registry state cannot authorize it.
The generation-bound backing-worktree calls do not infer that authority.

## Registry adapter staging

Consumed green node head ba45a6e3 under A8 (merge dc5a4fab7): the head's
rpc.rs extraction and protocol-family tests stay unchanged. The two setup rows
are appended to rpc/table.rs and remain pending None; the integrator alone
assigns their version. W11's registry and automation gate are still unavailable
as a reviewed consumer contract.

A cfg(test)-only bootstrap module now exercises the future adapter boundary
with an in-memory backend. Its scope includes exact WorkspaceId and setup
invocation UUID; the backend must validate that the invocation is still current
and successful for that workspace. Each start obtains its own lifecycle/trust
lease and revalidates before interactive shell input. Dropping an unopened lease
releases a proved no-spawn reservation; uncertain spawns remain unsettled.
The adapter, not this policy module, must own credential scrubbing and shell caps.
There is no production registry adapter, wire call or automatic terminal spawn
enabled by these tests.

The policy preserves file order, waits for first output or 1500 ms, and sends
exact approved ProjectCommand bytes plus CR. A cancellation during readiness
sends no input and starts no later terminal. Terminal-only sets require approval
before any start claim. Independent terminal failures leave the worktree and
allow later specifications to be attempted. These are staged policy obligations,
not evidence that durable workspace trust or actor registration is implemented.

## Canonical values followup (V3 row194 source scope)

The ordinal is one-based, matching Paseo utils/worktree.ts657-665 and
server/worktree-bootstrap.ts240-255. Ordinal 1 identifies the first command in
the approved list; ordinal n maps to approved-list position n-1. Consumers must
render the supplied ordinal without adding one. A runtime with zero-based event
positions must use a checked position+1 conversion when constructing this value.
The first/last/order roundtrip regression checks the scalar mapping only; it
does not establish runner or UI equivalence.

`vessel_proto::workspace::MAX_PROJECT_COMMANDS` is the single lower-layer owner
of the unchanged command-list limit. The project parser retains its existing
`MAX_COMMANDS` name as a re-export, so errors, admission and observation index
conversion derive from the same value without a dependency back to the parser.

These are descriptive DTOs, not validated observation snapshots. The public row
vector and generic TerminalBytes do not enforce list length, unique contiguous
ordinals, ordering, individual output or aggregate limits. The later runtime
and decoder must enforce and test those boundaries before any wire renderer
may treat a snapshot as validated. No such enforcement is claimed here.

The node adapter constructs those ordinals by checked position+1 from its
zero-based CommandEvent position. CommandEvent remains an internal array cursor;
its direct row lookup is unchanged. A full-limit producer regression checks
every approved row and the actual-runner cancellation test checks observed
ordinals. The merged node44 head supplies ProcessLifetime but this setup
observation does not treat that producer as workspace/Preview authority.

Terminal-only worktree creation now checks exact command-set approval before
the empty-setup branch; explicit Skip records Skipped. This closes the planning
bypass without enabling automatic terminal creation. Approved terminal-only
plans still do not spawn anything until the reviewed registry backend and
post-setup hook are integrated. The plan regression tests only that boundary.

## Snapshot admission after the scalar carrier

[Decision Log]
- Purpose: keep structured setup responses bounded when decoded from a peer,
  rather than trusting the public DTO vector or generic terminal byte wrapper.
- Constraint: the reviewed scalar carrier deliberately did not validate complete
  snapshots; direct Rust construction remains possible for producer and fixtures.
- Alternatives: comments alone cannot reject malformed peer snapshots; a second
  renderer-side limit could drift from the actual truncation accumulator.
- Choice: the proto owner defines the per-command output budget once, derives
  the aggregate from the canonical project command count, and checks contiguous
  one-based ordinals, count and output budgets while collecting decoded rows.
  The runner imports that same budget. Local DTO users call `validate` explicitly.
- Tradeoff: row decoding still uses the existing generic terminal byte decoder,
  within the transport's independent frame limit; this is not a streaming byte
  parser. A bad row is rejected before insertion into the admitted row vector.
  It grants no workspace, process or Preview authority.

Boundary regressions admit the full command count with every output exactly at
its cap, reject one extra output byte, excess rows, duplicate ordinals and
reordering through both local validation and actual serde decoding. These are
later decoder/runtime changes, not retroactive evidence for the scalar carrier.

Cancellation of an unknown observation, or a contradictory running observation
whose token is absent, returns Unknown without signalling. NotRunning is reserved
for a structured terminal attempt state; neither uncertainty nor absence proves
that a process tree ended. A regression injects these states without spawning
any shell and checks that an unknown attempt's token remains untouched.

A terminal-only creation plan that selects Nothing still creates no structured
setup invocation today. Its lack of commands is not a successful attempt token.
The later authoritative bootstrap adapter must establish an approved current
invocation and observe its empty-command completion before using that run as
bootstrap scope. Neither old worktree setup history nor an absent observation
can supply it. No automatic terminal hook is enabled by the planning fix.

## Independent observation/cancel handoff

The coordinator approved ADR 0093 observation and cancellation as a separate
node handoff from D2/bootstrap. It builds on announced node `9c150c6e`; automatic
terminals remain literally test-only. Each implemented call operates solely on
the existing worktree resource and observed invocation, never on inferred
Workspace bootstrap authority. Unknown remains distinct from NotRunning.
The integrator alone assigns a capability/number after the final root gate and
V3 review. The app consumes only a later announced package base.

The reviewed head's reserved worktree-id/admission bookkeeping and every existing
head test are retained in the merge. Generated TypeScript starts from that head's
reviewed baseline and must be regenerated on the VM for the added runtime rows;
restoring the baseline is not a freshness pass. The scalar carrier's evidence
remains separate from decoder, runner, guard and runtime evidence.

This observer retains Vessel's pre-existing SETUP_LIMIT and TEARDOWN_LIMIT.
Paseo does not configure those command deadlines; retaining the existing bounded
runner policy is an explicit difference, not a newly inferred source timeout.
Per-worktree retained raw output derives from MAX_PROJECT_COMMANDS and the single
MAX_SETUP_COMMAND_OUTPUT_BYTES owner. The existing MAX_WORKTREES also bounds
retained observations; no smaller node-wide output-pool claim is made. Output
buffer clones share their immutable bytes, while replacement buffers can overlap
with responses still being serialized. The final review must assess these actual
resource bounds rather than treating the DTO's per-snapshot check as a global
memory budget.
