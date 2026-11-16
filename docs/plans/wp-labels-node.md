# A6: workspace label node contract and persistence

## Current status and dependency

The separate native branch is `backend/m12-wp-labels`, with its own
`docs/plans/wp-labels.md`. This node branch is `backend/m12-wp-labels-node`;
ADR 0086 records its decisions. Session key/value label calls, mutation clocks
and LabelsChanged remain the lifecycle owner's ADR 0063 work.

The real Store adapter, writer, actor, registry projection and five pending
handlers are implemented. Current strict, real integration tests, TS generation,
whole gate and V3 acceptance remain incomplete. `WORKSPACE_LABELS` stays None;
no public RPC capability or consumer-ready source is claimed.

Adopted reviewed W11 registry `0140ac0a` through announced node `9c150c6e`
(gate 50c), `1c762ccf` (gate 52, protocol 25, schema 20), then `93970acb`
(gate 53, protocol 25, schema 21). The current base includes the canonical
listing append. Six registry metadata methods are assigned; none of this
package's label methods is assigned. W11's later membership-read leaf remains source-only
and was not adopted. Native adoption uses only the lead's announced app bases.

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
- Effects: the real adapter now consumes the reviewed registry helpers. Explicit
  fakes remain test-only. Pending protocol rows still expose no public RPC support;
  this package's current execution and review are incomplete.

## Modules and data flow

- `vessel-proto::workspace_labels` owns the validated name/key/color grammar,
  checked counts, ordered assignment replies, cursor/delta/snapshot values,
  semantic refusals and five pending request shapes. No parallel method table.
- `vessel-store::workspace_labels` plans and applies SQL. Canonical
  `registry::write::{require_active,bump_revision}` run in the same savepoint;
  Missing versus Inactive maps to distinct typed reasons. The typed result
  retains catalog delta and assignment invalidation until the outer commit.
- `writer::apply::workspace_labels` uses the existing isolated savepoint and
  marks `effects.registry = Changed` only on success. The existing writer gates
  both result and effect on outer commit. Revision increments alone do not wake
  readers. Assignment/name changes wake; no-op/recolor writes do not.
- Catalog-key foreign keys cascade rename/delete through ordered assignments.
  Registry snapshots join current names in the same held read transaction as
  descriptors/memberships. Descriptor JSON and catalog colors are not duplicated.
- `vessel-node::workspace_labels` owns one bounded actor and commit journal.
  Accepted mutation -> durable Store reply -> journal publish -> caller reply.
  Cancelling a requester cannot skip or reorder its accepted publication. Failed
  and no-op writes publish nothing. Restart starts a new catalog generation.
- `service::workspace_labels` reuses generated scopes, the exhaustive person-only
  policy, and existing Waiter shares/wait_slot. No general agent policy widening.
  All five browser-forwardable classifications and fake Handler implementations
  are extended explicitly; existing fake methods still return Unsupported.
- Node setup opens the actor on the existing Store. Unchanged Push/limits setup
  moved to the existing setup child, preserving order and shrinking node.rs by
  one line. Teardown captures the label handle through the gateway that already
  owns the moved service; no second service or actor. After RPC/settings callers
  finish, its close marker drains earlier accepted commits before Store::close.

Exact test-only staging attributes were removed only from modules with real
callers. Registry/timing fakes remain test-only and never select a product backend.
No dependency or crate edge is added; runtime remains SQLite/Tokio/std.

## Observation, compatibility and storage

Observation is a bounded unary long poll, preserving the reference workflow
without extending the session-only event frame. Subscribe/mark the watch seen
before snapshot; wait only when the supplied cursor is current, at most thirty
seconds. A cancelled poll drops its receiver. Delegated and exhausted-share
Observe downgrades to Now. Clients must pause after immediate unchanged responses
instead of busy looping. No detached per-observer task or durable lease exists.

Catalog generation plus contiguous sequence scopes the retained journal of 256
changes. Wrong, expired or future cursor returns a full snapshot. Native replicas
validate complete batches before replacing cache; offline colors remain readable
but cannot authorize writes or contribute to online merged catalogs.

All five rows append at the end of the shared method table with None. Shared
SDK/RPC admission refuses unavailable methods before frames/dispatch, including
nested calls. RegistrySnapshot::for_protocol omits unavailable assignments at
both direct handler and nested Forward/Relay/Deliver reply hops. The existing
pinned forwarding set and exhaustive version/sample/agent classifiers are
extended; no unrelated assertion or policy is weakened.

One migration appends after BACKGROUND_SCHEMA through a derived constant; the
integrator assigns merge order. Released SQL is unchanged. The existing rollback
fixture drops only the two new tables before its unchanged registry predecessors.
There is no second database, compound JSON writer or descriptor copy.

The one ECMAScript whitespace predicate also serves native query edge trimming:
FEFF trims, 0085 remains, internal query spaces retain their meaning. Name caps
are not invented; the existing transport budget applies. Color decoder failures
keep their source internally and expose safe wire text, never stored bytes.
TS uses serde default/skip inference for the optional non-null Vec projection;
explicit ts(optional), which requires Option, is not attached to that Vec.

## Authored verification, not current execution proof

- Protocol: value normalization/patch decoding/structured refusals; five method
  scopes/capability requirements; envelope preservation; direct/nested registry
  omission; old snapshots with the optional field absent.
- SDK: in-memory MIN/CURRENT handshakes; unavailable methods send no frame.
- Four real Store cases: missing/all inactive states refuse atomically; no-op and
  recolor watch silence/collision rollback; forced revision exhaustion rolls back
  both tables and watch, then later writes recover; ordered rename/delete and
  snapshot persistence across reopen with descriptor bytes unchanged.
- Actor: real Store mutation/reopen/generation plus scripted held commit,
  cancellation, publication and close drain. Real SQL fake-registry tests remain
  distinct from canonical production admission.
- Guarded handlers: isolated node, empty harness lookup, explicit temporary
  user/data directories. Actual direct dispatch checks read-only write refusal,
  person-only agent refusal and owner Store results. Paused-clock service tests
  check delegated/exhausted-share Observe returns without the long-poll wait.
  Direct tests do not advertise the pending methods or prove wire availability.

These new real-wiring tests have not run. No real agent, browser or live user
configuration is used by them. Current scoped tests and final whole gate must
also cover rollback fixtures, pinned forwarding and request samples.

## Actual evidence and retained failures

- Staged core: `wp-labels-node-narrow-20261115T084513Z-29c075fa` passed 26
  tests (node 10, proto 4, store 12). The later
  `wp-labels-node-narrow-20261115T093008Z-d6a38120` repeated 26/0/0.
- Staged drain core: `wp-labels-node-narrow-20261115T100539Z-81a07c0f`
  passed 27 tests (node 11, proto 4, store 12), including ordered shutdown.
  `wp-labels-node-narrow-20261115T105532Z-3bd691bc` passed proto/store/node
  all-target strict on that earlier staged source. Neither proves real wiring.
- Earlier TS generation passed three tests before the present full projection;
  current generator feature compilation and declaration evidence remain owed.
- `wp-labels-node-narrow-20261115T164335Z-9224cca2`: actual strict RED at two
  Store lints (catch-all error mapping and doc ticks). Fixed with exhaustive
  source-preserving variants and doc ticks, no allowance or test change.
- `wp-labels-node-narrow-20261115T171412Z-8c1c31bc`: actual six-package strict
  RED at one proto-test identical-match-arm lint. Store checking was reached;
  full Node/test acceptance was not. The label-list pattern now shares the
  existing empty-sample arm with identical sample bytes.
- `wp-labels-node-narrow-20261115T173853Z-b832f8f7`: queued-only cancellation
  after source audit found service use after move. Verified process group and
  terminal 143 before replacement; no compiler ran, so no compiler diagnostic
  or PASS is claimed. Capture now goes through the existing gateway owner.
- `wp-labels-node-narrow-20261115T174453Z-5e29641e`: registered once on the
  corrected immutable source; currently queued. Six-package all-target strict
  is still owed. This document-only cleanup changes no Rust source in that run.

## Remaining work and acceptance

1. Complete current strict and execute the real scoped tests; preserve immutable
   heads, actual failures and successful result counts separately.
2. Generate TS remotely and adopt its artifact/patch; never hand-generate or
   strip standard generated whitespace. Complete the TS declaration checks.
3. Adopt any newly announced node base at a natural point before final freeze,
   preserving held-transaction label hydration and integrator schema ordering.
4. Run one final whole root gate on the frozen package, then submit one V3
   review packet and await actual review/number assignment.
5. The lead brings accepted runtime to the native app through an announced base.
   Real catalog transport/observation, canonical sidebar/header picker entry,
   native whole gate/captures and V4 acceptance remain separate work.

No physical workspace provision/archive/restore/generation authority is added.
The lifecycle owner's checked c88066f2 has no LabelsChanged/SetLabels or metadata
ordering guarantee. Its new session mutation clock is not assumed to be a label
revision. Native current-label apply/Ready stays held for an exact tested atomic
snapshot, typed watermark or causal-read contract; no speculative event/editor.


The strict run on5e296 completed RED after reaching Client and Node. Corrected
one Client-test manual-let-else lint, moved a test-only CatalogDelta import to
its timing fixture, and changed guarded tests to inspect the actual Outcome
rather than assuming Result. Scope/error expectations remain the same. Node
start was101/100 lines; the original guarded-server construction moved verbatim
into the existing bootstrap owner with the same config/watch/token. No limit or
allowance was raised. Current execution remains owed after these corrections.


The strict run on54f6 completed RED in the new Store integration fixture: its
root-level preparation helpers are outside Clippy's test-function exemptions.
They now return typed directory/Store/name errors to the actual test callers;
no unwrap allowance or cfg workaround was introduced. Replaced the redundant
name-access closure with the canonical method reference. Runtime expectations
and the four test scenarios are unchanged; no runtime test has executed yet.


After that run finished, merged announced `93970acb` (gate53, all18 steps0,
Rust2577/0/12). It supplies iterator/selection/listing and schema21, but no
LabelsChanged/SetLabels/clock/reload/watermark. The only merge conflict was the
migration list: preserve its canonical listing SQL, then append label SQL.
The label relative index now lives in the existing schema entry module because
the shared migration list is at500 lines; it derives from BACKGROUND_SCHEMA,
without an invented numeric version or raised size allowance. Preserve W11's
same-transaction assignment hydration and the new producer's separate metadata
visibility/refusal logic. Current merged runtime still owes its own checks.


Exact source `171d8036f` passed the six-package all-target strict run
`wp-labels-node-narrow-20261115T181311Z-171d8036` (proto/store/node/client/rpc/web,
rc0, 138 seconds). The log confirms actual checking of all six changed crates.
This establishes current default-feature compilation/lint acceptance only.
The same immutable source's real filtered runtime suite is registered as
`wp-labels-node-narrow-20261115T182754Z-171d8036` on the ordinary lane; its plan
includes web-ci and nested fixture binaries before Cargo. Runtime counts,
TS generation, final whole gate and V3 review are still pending. No parallel
or duplicate test request was submitted. This checkpoint changes documentation
only; the test's Rust source remains byte-identical.


## Resume point after the interrupted build window

The exact-source runtime run
`wp-labels-node-narrow-20261115T182754Z-171d8036` completed all selected steps
with rc0: web preparation, nested fixture binaries and the filtered Cargo suite.
Actual tests were38 passed/0 failed/0 ignored: Node14, Proto7, Store12 unit plus
4 real integration cases, Client1. The actual guarded handlers, Store reopen,
failed-revision rollback/watch silence and SDK no-frame refusal all executed.
Zero-test filtered binaries are not counted. This supplements the earlier
six-package all-target strict pass on that same immutable source.

Merged announced node `ed4cd137`, preserving all new file/skills method rows,
classifiers, typed ids and service slots alongside A6. Its canonical migration
array moved to `schema/migrations/sequence.rs`; append the label SQL there after
listing while retaining the relative index in the schema entry module. No old
SQL, size allowance or protocol number was changed. The accepted base has the
named open intermittent startSession routing e2e defect; this is not a completed
A6 whole gate or permission to edit/waive that unrelated test.

Next: remote TS generation on this merged source, inspect/adopt its artifact,
then final whole-root verification and V3 packet. The prior38-test success is
scoped to171d, not a claim that the new union has executed. Native adopted only
announced appbase13 through its own line; session LabelsChanged/coverage remains
a separate unready producer. No speculative session editor or readiness switch.


Remote generation `wp-labels-node-ts-20261116T002758Z-85ac4ecf` compiled the
current union and passed5 TS tests (Proto3/Web2,0 failures). The step returned1
because proto.ts differed, with an exported generator patch and byte manifest.
Verified every artifact hash and the generating commit, then adopted the exact
proto.ts bytes. It adds the five call/reply shapes, catalog/assignment values,
ids and optional non-null registry label projection; no declaration was hand
edited. A subsequent whole gate must confirm zero generator diff. Native
workspace row/header entry remains staged: both owners confirm no checked
runtime context/active/assignment projection is available yet.
