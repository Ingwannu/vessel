# ADR 0093: Structured worktree setup observations and cancellation

This commit is a values-only carrier. It registers no method row, adds no
client call, node handler, runner or bootstrap, and assigns no protocol number.
The complete runtime remains a separate W8 node package. The app may consume
this carrier only after V3 source pass and integrator adoption, as WORKER's
10:4x values-only exception requires.

[Decision Log]
- Purpose: keep command boundaries, results and elapsed units canonical while
  the native app builds against an in-memory backend.
- Existing assets and constraints: WorktreeId identifies the managed backing
  resource; DurationMs and the existing UUID/counter macros already validate
  the wire values. Legacy combined logs cannot establish command boundaries.
- Alternatives: infer commands from log markers; duplicate invocation ids and
  result enums in the app; merge an unreviewed node runtime into an app tree.
- Choice: export one bounded command index, invocation UUID/revision cursor,
  typed result/state and request/reply values, with feature version None.
- Why: a later reviewed runtime and the app must share the same schema without
  making source-only values executable or advertising a capability early.
- Costs and effects: there is no live observation/cancellation RPC in this
  carrier; a concrete backend remains unsupported until the runtime is landed.

SetupRunId identifies an invocation, not a canonical WorkspaceId, backing or
admission generation, process lifetime or Preview permission. Any workspace
controller must resolve ownership, current automation gate and restoration
admission through W11's reviewed contract; these values grant none of them.
Absent/unknown observations are not success. A Requested cancellation receipt
is a signal acknowledgement, not proof the process tree has ended.

Paseo references: server/worktree-bootstrap.ts33-143,240-265 bounds each command's
output and records actual results; app/panels/setup-panel.ts164-265 preserves
manual collapse and formats durations; workspace-setup-runtime.ts27-36 stops a
running attempt. The complete runtime must observe real runner events rather
than parse log text and keep durable blocked-source state separate from old
backing-worktree history. Automatic terminals remain the separate approved,
credential-scrubbed interactive-shell/bootstrap integration.

The focused values tests cover index decoding, unknown variants, distinct run
identities, nullable observations and explicit millisecond request units. The
carrier's proto check/tests run on the build VM, never through local Cargo.

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

## Later runtime package

The reviewed values carrier remains the baseline above. The separate, pending
[runtime follow-up](../plans/worktree-setup-runtime.md) adds the actual runner,
decoder resource bounds and cancellation behavior; its evidence is not inherited
from the scalar carrier or the integration gate. No assigned capability or
automatic-terminal authority is introduced by consuming the values.
