# ADR 0093: Structured worktree setup observation and cancellation

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
  behavior, and observers cannot await or backpressure child pipes. A slow
  callback still delays the runner, so callbacks only update bounded local
  memory and wake readers. Snapshot persistence and network delivery happen
  outside that callback. Runner events alone do not make observation a
  supported public capability.

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
