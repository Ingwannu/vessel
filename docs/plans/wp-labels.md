# Session and workspace labels

## Scope and reference

Port Paseo 0.15.4 lifecycle feature 39 and project/workspace sections 18–19.
Reference sources read before implementation (relative to the reference source root):

- `packages/protocol/src/agent-labels.ts:1–64`: parent agent/host metadata,
  open-tab metadata, same-host fallback. These are lifecycle metadata, not
  workspace label colors.
- `packages/protocol/src/workspace-labels.ts:1–31`: ten colors, whitespace
  normalization, case-insensitive identity. No maximum name length.
- `packages/app/src/workspace-labels/internal/picker-model.ts:1–33`:
  assignment lookup by normalized identity.
- `packages/app/src/workspace-labels/internal/merge.ts:1–35`: target host
  wins duplicate names, otherwise server-id order; sorted merged projection.
- `packages/app/src/workspace-labels/internal/workflow-model.ts:24–106`:
  picker keeps its surface open, pending state per normalized name, failure
  retains the assignment read from the host.
- `packages/app/src/workspace-labels/internal/workflow-model.ts:108–320`:
  host selection, search, persistent edit draft, one atomic update, inspect
  before delete confirmation, no host changes during an operation.
- `packages/app/src/workspace-labels/picker.tsx:72–255`: picker/create
  pages, create-and-assign, offline/unsupported hints, default violet color.
- `packages/app/src/workspace-labels/manager-modal.tsx:52–177`: adaptive
  manager, host selection, list search, draft reset on close, save/delete.

## Existing assets and decision

[Decision Log]
- Purpose and intent: carry the reference label workflows into the shared
  native view/link/app layers without simulating successful persistence.
- Existing implementation and constraints: native kit already provides fields,
  buttons, choice lists, dialogs, alerts and motion; Fluent generates typed
  translation accessors. There is no workspace label catalog model or screen.
  The reviewed node head has session label values/storage (ADR 0063), but no
  label mutation RPC. A workspace color catalog is a different domain from
  a session's key/value metadata.
- Alternatives considered: overload session key/value labels for workspace
  colors; invent client RPC names; add a local catalog; wait without building
  the independent UI model.
- Choice: new feature modules reusing existing kit/Fluent and std collections.
  New UI value types express workspace labels only. The future node decoder
  must convert its validated wire types at the boundary, once.
- Why: overloading metadata loses host-local color ownership and atomic rename;
  a local catalog would claim persistence the node cannot provide. Independent
  state machines can be tested before RPC contracts land.
- Advantages, costs and effects: no new dependency or crate edge. The skeleton
  can render every state, but cannot claim node-backed parity until calls,
  reconnect synchronization and fake-node contract tests exist.

## Structure and data flow

- `view/src/labels/`: validated display names, color and assignment enums;
  host-local catalogs and merge projection; picker/manager transitions and
  correlated operation tickets. No I/O or GPUI.
- `view/locales/{en,ko}/labels.ftl`: every displayed sentence.
- `link/src/labels.rs`: typed commands/events for the UI seam. Only existing
  node calls may be invoked. An absent contract is an explicit unsupported
  result, never a mock success or a newly invented RPC.
- `app/src/areas/labels/`: existing kit controls draw view state. Shared
  registries only register this module and screen.
- Intended flow: UI action -> validated view intent -> link command -> node
  client -> correlated event -> view transition -> draw. Catalogs belong to
  one host; only online authoritative catalogs enter cross-host projection.
  Cached offline catalogs remain available for that host's own row colors.

## Node contract dependency

At node head `d954fa3a`, `crates/vessel-proto/src/labels.rs` exports
`LabelKey`, `LabelValue`, `SessionLabels`; the method declarations contain no
`session.labels` or workspace label calls. `lifecycle_version::SINCE` remains
unassigned. Schema 17 does not establish RPC availability. Assignment/list,
atomic rename/recolor, deletion inspection/deletion, catalog subscriptions,
workspace assignment storage and metadata patch deletion still require the
node owners. This was reported before implementation. Do not add root RPC
or store code in this package without a routed contract.

## Differences and limits

- Product refuses unavailable label operations explicitly until the node
  publishes their contracts. No user-created catalog is fabricated locally.
- The pure model uses deterministic Unicode lowercase ordering rather than
  platform locale collation. Identity follows the reference whitespace and
  lowercase rules; rendering order is stable across native platforms.
- Session reserved metadata remains owned by lifecycle/subagent code. It is
  not editable as workspace labels and is not translated into colors.

## Validation and resume point

Done: ADR 0086 proposed; typed workspace label names/keys/colors, generation
and sequence, snapshot/change payloads, request/response values. No method
registration or store migration is part of the value contract.

Narrow validation: `cargo test -p vessel-proto workspace_labels` passed
2 tests, 0 failed. This proves name/color decoding only, not RPC or durable
storage. Remote proto clippy on `f767bc91` also passed with warnings denied
(run `wp-labels-node-narrow-20261115T062639Z-f767bc91`).

Observer journal and mutation planners are now staged under cfg(test): the
journal checks contiguous cursors, retained-history boundaries, generation
changes and sequence exhaustion; assignment/update planners cover host-owned
colors/spelling, unknown unassignment, stable order and atomic collision
rejection. Eight journal/planner tests passed on `2758645a`. These modules have no product caller until
the registry/store/method wiring lands, per the staging rule.

Merged reviewed identity-only commit `e45a5d89` after Quality B. The sole
conflict was the id reexport list: kept every head export and appended the
two catalog generation/sequence types. Registry/store code was not adopted;
ownership lookup remains behind a typed staged boundary/fake pending review.

Staged SQL now uses the existing SQLite transaction model: catalog keys and
ordered assignments, cascade rename/delete, typed row decoding and refusal,
atomic affected counts, and revision invalidation in the same transaction.
The registry trait has an explicit test fake with only the agreed parent id
key; it does not copy unreviewed registry lifecycle SQL. Eight real-SQL tests
cover admission, collision, archived assignments, advisory deletion, rollback,
ordinal exhaustion and corrupt rows. All eight passed on `82f5974e`
(`wp-labels-node-narrow-20261115T070712Z-82f5974e`). This uses a fake registry,
so it does not establish production registry integration. The SQL is
not appended to the production migration list and no production registry
adapter, writer command or RPC is registered yet.

Patch decoding now distinguishes omission (preserve the current field) from
explicit null (refuse invalid input). Semantic refusals carry a typed reason
through the wire boundary; corrupt stored color diagnostics retain their
source internally but cannot echo stored row text to a client. Regression
tests for those boundaries are added and still await execution. The generated
TypeScript declaration check passed all three tests on this source with
`VESSEL_WRITE_TS=1`; it is scoped declaration evidence, not an RPC or whole gate.

Next: reviewed ADR 0082 registry adoption and single-writer
persistence. Node journal/planner tests, protocol wiring/old-peer/forwarded
coverage remain owed. The native skeleton lives on
the separate app branch and must merge the announced app base first. No full
gate, UI capture or production flow has been verified.

The staged actor core now owns commit-to-publication ordering even when the
request is cancelled after acceptance. It uses the existing bounded Tokio
channels; no new framework or runtime is introduced. Observer watches are
subscribed and marked seen before the actor snapshot, preventing lost wakes
and repeated immediate polls from a previously unseen watch version. Polls
time out after a named limit and cancellation drops the receiver directly.
Four timing regressions use an explicit scripted backend and await execution.
The real backend remains blocked on the reviewed registry/store producer;
this core is cfg(test), not a fake installed into product dispatch.
