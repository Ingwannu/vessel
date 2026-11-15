# A6: workspace label node contract and persistence

The native view/link/app implementation is on `backend/m12-wp-labels`, with
its own `docs/plans/wp-labels.md`. This plan is the node side on
`backend/m12-wp-labels-node`; separating the documents avoids merging two
independent progress records into the same file. ADR 0086 owns the decisions.
Session key/value label calls remain the lifecycle owner's ADR 0063 work.

## Reference and existing assets

Read Paseo 0.15.4 sources, relative to the reference source root:

- `packages/protocol/src/workspace-labels.ts:1–31`: ten closed colors,
  ECMAScript whitespace normalization and lowercase identity; no name cap.
- `packages/server/src/server/workspace-labels/internal/service.ts:85–229`:
  active workspace admission, existing host spelling/color wins, ordered
  assignments, atomic rename/recolor and idempotent removal.
- `packages/server/src/server/workspace-labels/internal/sequence.ts:1–90`:
  process generation, contiguous sequence, retained journal of 256 changes,
  and full snapshot when the cursor is unusable.
- `packages/app/src/workspace-labels/internal/host-replica.ts:1–56`:
  never apply a gap; refetch authoritative state.
- `packages/app/src/workspace-labels/internal/workflow-model.ts:24–320`:
  independent per-name pending assignment, retained draft on error,
  inspection before deletion confirmation.

[Decision Log]
- Purpose and intent: preserve host-local catalog/assignment atomicity and
  committed observation without guessing workspace ownership from a session
  or working directory.
- Existing implementation and constraints: one SQLite writer/read pool
  already supplies savepoints, durable commit and delayed replies. W11 owns
  the registry identity/lifecycle and its global revision/watch. Quality B
  owns the sole method table and pending-version refusal.
- Alternatives: reuse session metadata; duplicate descriptors; use a second
  database or compound JSON journal; publish from the RPC future.
- Choice: new feature modules reusing SQLite, the existing writer, std
  collections and Tokio channels. No dependency or crate edge is added.
- Why: one database can atomically rename catalog and assignments; an actor
  can finish publication after the requesting future is cancelled.
- Effects: integration depends on the reviewed registry helpers. Test-only
  staging exposes no fake product backend and claims no live RPC support.

## Modules and data flow

- `vessel-proto::workspace_labels`: one validated name/key/color grammar,
  checked counts, ordered assignment replies, snapshot/delta/cursor values,
  typed semantic refusals and request shapes. `WORKSPACE_LABELS` is None;
  five unary requests have pending method rows.
- `vessel-store::workspace_labels`: mutation planners and real SQL behind a
  typed `Registry` trait. Admission precedes writes in the same savepoint.
  Catalog keys cascade rename/delete into ordered assignments; names resolve
  the catalog's current spelling instead of persisting another descriptor.
  The result carries catalog delta and typed assignment invalidation.
- `vessel-node::workspace_labels`: bounded commit journal plus actor. The
  actor serializes durable reply -> publish -> caller reply; a cancelled
  caller cannot omit an accepted commit's publication. Failed/no-op writes
  do not publish. A close marker drains earlier work before store shutdown.
- The future handler owns scopes/agent policy and existing wait shares.
  Observe uses a bounded unary long poll, subscribes/marks seen before its
  snapshot, and directly releases its receiver on cancellation. No detached
  per-observer task or lease is introduced.
- Native replicas validate whole batches, keep cached offline colors, and
  use actual answered NodeId ownership. Unary mutation replies settle forms;
  the ordered observer is the catalog's only update path.

The store and actor now have real registry/writer/RPC callers, so their exact
cfg(test) staging attributes are removed. Explicit fakes remain test-only timing
and admission fixtures, never installed into a product build.

## Agreed registry integration

Reviewed identity-only `e45a5d89` is adopted. Announced node gate-40 head
`ba45a6e3` merged cleanly; protocol 23, schema 18. The reviewed registry producer is now adopted through announced node head
`9c150c6e` (gate 50c). Native adoption still waits for the lead's base 9.

The agreed producer seams are `registry::write::{require_active,bump_revision}`
inside the writer savepoint, typed Missing versus Inactive refusal, and
`effects.registry = RegistryChange::Changed` after successful assignment-name
changes. Recolor-only/idempotent changes do not wake workspace observers.
Incrementing the durable revision alone is insufficient to wake watchers.

Ordered assignment projection joins inside the registry snapshot's existing
read transaction. Labels stay out of persisted WorkspaceDescriptor JSON;
there is one assignment store. This owner adds its projection values and
WORKSPACE_LABELS omission gate after the reviewed producer is available.

## Verification

- `29c075fa8`: 26 label tests passed (node 10, proto 4, store 12), including
  real SQL with a fake registry and four scripted actor timing regressions.
  Run `wp-labels-node-narrow-20261115T084513Z-29c075fa`.
- `e764f8a66`: proto/store/node strict all-target clippy passed, including the
  new assignment invalidation fields/assertions and gate-40 base. Run
  `wp-labels-node-narrow-20261115T090757Z-e764f8a6`.
- `d6a38120f`: all 26 scoped tests passed again on that code/base, with no
  failed or ignored tests. Run
  `wp-labels-node-narrow-20261115T093008Z-d6a38120`.
- Earlier TypeScript regeneration/check passed three tests on the strict
  patch/structured-error source. The generated file merged with gate 40;
  the current full declaration gate remains owed.
- The new ordered-shutdown regression has not run. No full gate, RPC runtime,
  inspected native capture, review or production flow is claimed.

The validated name module also exposes the existing ECMAScript whitespace
predicate through one edge-trim helper for native query matching. Both sides
share that grammar: FEFF trims, 0085 remains, internal query spaces retain
their meaning. Boundary assertions were added to the existing proto test;
their new execution remains pending with the ordered-shutdown regression.

## Remaining work

Consume only the reviewed W11 producer; append one named schema migration at
that head's next number; add real writer/read APIs, commit watch effects and
snapshot assignment projection. Start/drain the actor in node lifecycle,
before closing the store. Register five pending rows at the end of
`rpc/table.rs`, retaining None until the integrator assigns the number.

Complete direct/nested/forwarded/older-peer and agent policy coverage, existing
wait-share downgrade for Observe, generated TypeScript and SDK boundaries.
Then run the node whole gate once and submit the complete node package to V3.
The native package waits for that reviewed consumer boundary for picker/create
entry, observation and final V4 gate/captures; it does not invent RPCs.

## Real adapter integration checkpoint

Merged announced node `9c150c6e` as `2a30c0385`. Additive conflict resolutions
retain both workspace-label and diagnostic ErrorDetail variants, both Store
error groups and generated declaration groups. Generated TS remains owed from
the remote generator; its standard trailing whitespace is not hand-reformatted.

The real adapter uses canonical require_active/bump_revision in the same writer
savepoint, with Missing/Inactive mapped to typed label reasons. Results and
deltas wait for the outer commit before the actor publishes. Registry labels
join the same held snapshot read transaction; descriptors stay byte-identical.
Assignment/name changes record the existing post-commit registry invalidation,
while recolor/no-op writes leave that watch untouched. The additive migration
follows REGISTRY_SCHEMA through one derived constant; no numeric version is
invented or released entry edited. The existing rollback helper drops only the
new label tables before its unchanged registry predecessor rollback.

Five pending method rows append after the announced table. Shared scope/agent
checks, typed targets, SDK refusal and nested version projection remain the sole
boundaries. Snapshot labels are omitted for unassigned/older peers, including
Forward/Relay/Deliver nesting. All label calls remain person-only and unavailable
until their version is assigned. Observe reuses the existing wait shares and
downgrades delegated/exhausted shares to immediate reads. Startup opens the actor
on the existing Store, and shutdown drains it after callers finish and before
Store::close. Push/limits initialization moved unchanged into the existing setup
child beside that new actor initialization; the shrinking node size record drops
by one line instead of raising its allowance.

Authored verification now includes three real Store tests, one real actor/Store
reopen test, typed direct/nested capability/projection tests and an in-memory SDK
no-request-frame test. These new tests and current strict compilation have NOT
executed yet. No A6 whole gate or reviewer acceptance is claimed.

A fourth real Store regression exhausts only the isolated fixture's registry
revision, forcing failure after catalog/assignment SQL has already run. It checks
savepoint rollback, unchanged snapshot and silent watch, then restores the fixture
revision and proves a later write still succeeds. This complements the planner
collision test and scripted actor failure; it remains unexecuted with the new
adapter checks.

Two additional isolated-node tests directly invoke the generated guarded handler
without advertising the pending methods on a transport. They test read-only
write refusal, person-only agent refusal, actual owner Store results and final
assignment/catalog removal. The second uses a paused clock to prove delegated
and exhausted-share Observe calls return without advancing through the long-poll
wait. These are source-authored handler-policy tests, not public RPC availability
or execution PASS. Lookup is empty and the user/data directories are isolated.
