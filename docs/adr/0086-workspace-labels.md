# ADR 0086: Host-local workspace label catalogs

Status: proposed.

## Purpose and intent

Provide Paseo 0.15.4 workspace labels: ten colors, create-and-assign, ordered
workspace assignment names, atomic rename/recolor, inspected deletion, and
catalog snapshot/delta observation. This is independent of session key/value
metadata (ADR 0063). Workspace identity and lifecycle belong to ADR 0082.

## Existing implementation and constraints

The node stores session labels but has no colored workspace catalog. The
registry owner is adding one durable SQLite registry and a stable WorkspaceId.
No path or session is treated as a workspace identity. The existing store
writer owns all SQLite writes, isolates operations in savepoints, commits,
and only then answers callers. Use that writer and its read pool.

Read reference sources, relative to the Paseo 0.15.4 source root:

- `packages/protocol/src/workspace-labels.ts:1–31`: colors and normalization.
- `packages/server/src/server/workspace-labels/internal/service.ts:85–229`:
  assignment requires an active workspace, unknown unassignment creates no
  definition, existing host spelling/color wins, update is atomic and
  case-only rename is permitted, deletion removes every assignment.
- `packages/server/src/server/workspace-labels/internal/sequence.ts:1–90`:
  process generation, sequential changes, bounded journal, fallback snapshot.
- `packages/app/src/workspace-labels/internal/host-replica.ts:1–56`:
  generation/next-sequence check, never apply a gap; refetch on mismatch.
- `packages/app/src/workspace-labels/internal/workflow-model.ts:24–320`:
  per-name pending assignment, retained draft on error, inspection before
  confirmation, host switching disabled while pending.

## Alternatives considered

- Extend session metadata: loses independent workspaces and catalog colors.
- Separate catalog/assignment JSON files: needs a compound recovery journal
  which SQLite already supplies, with another writer and durability boundary.
- Duplicate label names/colors in registry descriptors: rename would require
  two stores to agree and recolor could leave stale copies.
- Global cross-host catalog: same label can intentionally differ by host.

## Decision

Add `vessel_proto::workspace_labels` values, typed requests/responses and one
`WORKSPACE_LABELS: Option<ProtocolVersion>`. The five unary rows were reviewed
as pending (None) in the shared method table; the integrator assigned protocol
28 at the node-line merge (2026-11-16, see "Activation" below).

Use catalog and ordered assignment tables in the existing SQLite database.
Assignments reference ADR 0082's registry primary key and the catalog key.
The catalog retains normalized display spelling and color; identity is
whitespace-normalized Unicode lowercase, as the reference. Referencing the
catalog avoids copying colors; a joined assignment projects current names.
A rename updates the catalog identity and all matching assignments in the
same savepoint; a case-only rename changes spelling; recolor changes only
the catalog. Collision rejects both requested fields. Deletion is idempotent;
its inspection count is advisory and is recomputed during deletion.

Assignment checks `registry::require_active(connection,id)` in the same
writer transaction. Archive claims must precede teardown; archived/missing
workspaces refuse assignment. Mutations affecting assignments also invoke
`registry::bump_revision(connection)` in that transaction. Registry snapshots
join assignments from this owner, never maintain a second descriptor store.
The SQL mutation result also carries the existing typed assignment-change
fact. The real writer must map it to the registry's after-commit watch effect;
incrementing a durable revision alone would leave existing observers asleep.
Recolor-only and idempotent mutations do not invalidate workspace assignments.

Method rows (protocol 28):

| Call | Scope | Result |
| --- | --- | --- |
| `workspace.label.list` | read | snapshot or retained changes; optional bounded observation |
| `workspace.label.assign` | operate | host definition and committed assignment names |
| `workspace.label.update` | operate | atomic definition and affected count |
| `workspace.label.inspect_delete` | read | advisory affected count |
| `workspace.label.delete` | operate | committed affected count |

All calls use the existing paired connection and its scope intersection;
all label calls refuse agent-origin requests through the existing exhaustive
person-only policy. They are host-local and forward only through the existing
forwarding scope checks. No credentials, process, external service or helper
is introduced; platform behavior is identical on Windows and Unix.

Catalog observation uses a process generation and contiguous sequence. The
retained journal bound has one named constant matching the reference's
256 entries. Only committed mutations publish. A different/expired/future
cursor returns a snapshot; no consumer applies a gap. Disconnect releases
the owned subscription; reconnect refreshes authority before writes. Cached
offline colors remain available to that host's rows but do not contribute to
merged online catalogs or prove that a failed load deleted a definition.

Use a bounded unary long poll for observation, as existing terminal/port reads
do, instead of adding unrelated catalog payloads to the session-only event
frame. Subscribe to a watch cursor before taking the catalog snapshot, wait
only when the supplied cursor is current, and drop that watch receiver when
the RPC is cancelled. A changed cursor re-enters the actor for an atomic
snapshot/catch-up. One actor owns mutation -> durable commit -> journal publish -> caller reply,
so request cancellation cannot reorder or skip publication. This transport
difference preserves the reference's user-visible observation semantics.

The eventual service handler must reuse `NodeService::wait_slot` and the
existing `Waiter` shares before selecting Observe. Delegated calls answer
immediately, as terminal/port reads do, and an exhausted connection/origin
share downgrades Observe to Now. This leaves capacity for other calls on the
same fleet connection. The client pauses after unchanged immediate polls;
it must not turn share exhaustion into a busy loop. The staged actor has no
principal/connection policy of its own; that remains the handler boundary.

Normal node shutdown queues the actor's close marker before closing the store.
The actor drains earlier accepted changes through durable reply and publication,
acknowledges that marker, and refuses work after it. Dropping an RPC does not
make its accepted mutation disappear from that drain. A scripted held-commit
regression checks the ordering and post-marker refusal; production cleanup
must wire this acknowledgement before `Store::close`.

## Activation (protocol 28) and a corrupt stored row

[Decision Log]
- Purpose: make the reviewed calls public, and decide what a corrupt stored
  row does now that a client can reach the catalog (A6 review note b).
- Reference: Paseo's `bootstrap.ts` 1070 awaits the label service's
  `initialize()` without a catch, and `catalog-store.ts` 171-178 parses the
  whole catalog; one bad entry stops the daemon. A missing file is an empty
  catalog.
- Choice (coordinator ruling, 2026-11-16): keep Paseo's behaviour. `Labels::open`
  reads the catalog at node start, so one corrupt row refuses start with
  `NodeError::WorkspaceLabels`. Its cause names the table and the row's place
  (`vessel_store::LabelRow`: the catalog row in key order, or a workspace's
  assignment in its order, from 1) and never the row's name, key or color:
  both tables are `WITHOUT ROWID` and keyed by the label name itself, and the
  color decoder's error, which can repeat the stored value, is dropped.
- Rejected: skipping or quarantining the row (silent data loss, not the
  reference); refusing only label calls (a different failure surface from
  the reference, and a client could then write over state it cannot read).
- Effect: the wire still shows a client only the fixed `internal` notice.
  Tests assign over the real server and SDK at 28, refuse older peers before
  a frame, and pin the start error's text against a canary row.

## Differences and version behavior

- SQLite transactions replace the reference's compound JSON commit journal.
  Durable catalog and assignment state cannot diverge within one database.
- Stable lowercase ordering replaces locale-dependent sort for deterministic
  cross-platform presentation. Host-local display spelling remains intact.
- No arbitrary label name/count cap is added. The existing transport frame
  budget applies. Typed names reject empty normalized text at decoding.
- Calls are refused before dispatch/sending to a peer older than 28; no local
  fake success. Old clients see no new descriptor fields/events; new clients
  against an old node show Unsupported. Forwarded replies obey the same gate.
- Session reserved metadata remains ADR 0063/subagent policy, not this catalog.

## Advantages, costs and effects

No dependency or crate edge is added. One database/writer supplies crash
atomicity. The registry and this feature coordinate revision updates and
snapshot joins; adoption must wait for the registry's reviewed contract.
Protocol tables, older-peer paths, generated TS, store rollback fixtures and
fake-node tests are updated when wiring lands, never weakened to stage types.

## Validation and current state

The staged core previously passed 27 scoped tests (node 11, proto 4, store 12),
including scripted accepted-write cancellation/drain and real SQL with a fake
registry. It also passed earlier proto/store/node strict checks.

The reviewed metadata registry `0140ac0a` is now adopted through announced node
`9c150c6e` (gate 50c: all 18 steps pass, Rust 2549/0/12). This wiring reuses its
real Store helpers, watch and read transaction. Test-only staging is removed only
where those real callers now exist. Added real Store/actor tests, five pending
method rows, SDK admission and direct/nested projection coverage still await
current execution, TS regeneration, whole A6 gate and V3 review. The constant
remains None. Native takes only the lead's announced base and has no real label
consumer-ready adoption from this unreviewed adapter. Session metadata remains
the lifecycle owner's separate contract. No capture or production parity is
claimed.


Current verification: the real-wiring source171d passed six-package all-target
strict and38 scoped runtime tests (Node14, Proto7, Store unit12/integration4,
Client1). It does not establish whole-gate or native acceptance. The newer
announced node base moves only the canonical migration-array registration path;
A6 follows that path and retains the same Store/actor/registry boundaries.
