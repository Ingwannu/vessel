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
  Workspace label values have one owner in `vessel-proto::workspace_labels`;
  the view reexports those types rather than retaining a second grammar.
  The additive value contract is shared from this package's own node branch,
  not a merge of the old app base with an unannounced node head.
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
workspace assignment storage and metadata mutation still require the node owners. The lifecycle owner
confirmed planned SetLabels merge-only semantics (no clear/delete), which is
a documented difference from nullable manager-level Paseo patches. This was reported before implementation. Do not add root RPC
or store code in this package without a routed contract.

## Shared identity paint

The theme owner approved the additive token-only `kit::theme::identity`
module; no new control or renderer was needed. `Hue` is generic, with
`foreground(Hue, Scheme)` and `tint(Hue)`; no protocol dependency or change
to the named palette builder. Read reference
`packages/app/src/styles/identity-colors.ts:16–117`,
`workspace-labels/swatch.tsx:13–180` and `chip.tsx:1–135`.
The foreground uses the source's light/dark contrast bands; tint retains
exact alpha 0x1a. App label colors map to this generic hue at the drawing
edge. Narrow kit clippy first refused screen-local float arithmetic in tint alpha
conversion; the token now composes exact RGBA bytes and delegates conversion
to the existing color library. Tests also guard RGB24 rows. Narrow kit checks
and the package review remain pending.

## Differences and limits

- Product refuses unavailable label operations explicitly until the node
  publishes their contracts. No user-created catalog is fabricated locally.
- The pure model uses deterministic Unicode lowercase ordering rather than
  platform locale collation. Identity follows the reference whitespace and
  lowercase rules; rendering order is stable across native platforms.
- Session reserved metadata remains owned by lifecycle/subagent code. It is
  not editable as workspace labels and is not translated into colors.

## Validation and resume point

Done: reference/asset analysis; manager, picker and create state models;
EN/KO Fluent resources; adaptive manager skeleton and typed link command/event
seam. The sidebar registers the manager; online hosts are explicitly shown
as unsupported until the contract is integrated. No mutation can claim success.
The package-base merge removed the shared-file pressure. Registration now
uses the base's direct link module declaration and one shell overlay line;
the temporary registration wrappers were removed without changing public paths. No kit renderer or dependency was added.

Fast checks: native size, boundaries and token ratchets pass. The first local
view test could not compile because the new Fluent keys lacked per-key
translator context comments; comments were added. Remote narrow view test run on `befc083a` passed 10 tests, 0 failed
(run `wp-labels-narrow-20261115T062329Z-befc083a`). The manager surface is
not yet compile-verified. Local app clippy found 9 warnings promoted to
errors in the new view code (clone assignment and intentionally superseded
collection entries); those were corrected without suppressing lints.
The follow-up preserves behavior while typing private catalog identities
and splitting list drawing out of the manager phase renderer.

Proto name/color tests (2) and proto clippy passed on the node value commit
`f767bc91`. Its additive value contract is now used by the native model;
name/color/assignment/count definitions are no longer duplicated in view.
The new root values make the root gate required at package close as well.

Client replica now stages atomic snapshot/catch-up validation. A gap, wrong
generation/base/head, or duplicate snapshot identity requests a full resync
without partially changing the cached catalog. Disconnect/load failure keeps
colors cached and revokes write authority. Four new regression tests are
pending; no subscription or reconnect RPC is claimed wired.

Additional reference edge: Save/inspection resolve the draft target against
the current host catalog, not its opening snapshot. A concurrent recolor
updates dirty-state calculation; an externally removed/renamed target cannot
be saved. A regression covers this without optimistic mutation.

Follow-up behavior: create failure retains its safe error and exact draft;
a later successful toggle of a different label cannot erase an earlier
concurrent failure; hiding/reopening a manager preserves in-flight work.
Regression assertions cover both error-retention cases. Checks are pending.
Local narrow clippy subsequently required one explicit unit-return semicolon
in the replica and two link documentation/discard fixes; no lint was
suppressed. Local font assets were absent, so the base's licensed font files
were prepared in the ignored font directory before repeating the check.

Merged app package base `8d5513e4` into `058d380b` without conflicts.
The node branch merged Quality B `29eebb8b` into `a6ff863c`, also without
conflicts. Existing upstream entries and tests were unchanged.

Next: obtain narrow compile/test results and consume
and reviewed registry/session-label contracts. ADR 0086 owns the node catalog
work in a separate node branch; that branch currently stages values, mutation
plans and a bounded observer journal, with no registered RPC or persistence. Full native gate and inspected capture
matrix occur once at package completion. No whole gate, node RPC test,
service-account/customer flow or UI capture is claimed at this point.
