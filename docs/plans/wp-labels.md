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
  one host; connected hosts' cached catalogs enter cross-host projection.
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

## Editability and color choices

Manager name/color editing is a pure `EditAccess` policy: only an idle editor
on a connected capable host accepts input. A regression traverses every
async phase. Package base `6036eeb8` supplied the shared `Editability` and
`TextField::set_editability` API; it is wired once per policy transition,
including immediate changes after opening an editor. No local field renderer
was introduced and no render-notify loop is used. The merge was conflict-free.

Color choices reuse kit buttons and dots with generic identity ink. Selection
has a one-shot QUICK fade and uses the kit motion hold policy. Deliberate
layout difference: labeled buttons retain nonoverlapping 44-point touch
areas and wrap on narrow windows, rather than relying on overlapping hit slop
around reference 20-point swatches. Choosing a color changes the draft only.

## Differences and limits

- Product refuses unavailable label operations explicitly until the node
  publishes their contracts. No user-created catalog is fabricated locally.
- The pure model uses deterministic Unicode lowercase ordering rather than
  platform locale collation. Identity follows the reference whitespace and
  lowercase rules; rendering order is stable across native platforms.
- Session reserved metadata remains owned by lifecycle/subagent code. It is
  not editable as workspace labels and is not translated into colors.

## Validation and integration state

Native skeleton `72a16a1c` passed strict all-target app clippy through the
remote narrow runner (`wp-labels-narrow-20261115T082314Z-72a16a1c`, rc 0).
Earlier `befc083a` passed ten view label tests. Expanded replica/binding/policy
and action tests still await execution; these results are not a whole gate.

Package base 4 `3cd0d3fc` is merged. Additive merge resolutions keep both
identity and palette modules, both label settling and terminal presentation,
and both README additions. The retained label fields participate in the
shared language refresh registry without losing typed text or selection.

Manager actions now go through the existing bounded shell command queue and
correlated link events. Full/closed queue refusal settles the same operation;
an inspection is never mistaken for a deletion. Save, advisory inspection,
confirmation cancellation and deletion keep their host/ticket ownership.
Back preserves search and refuses abandoning pending work. Pending phases
retain the same readonly field/color controls so selection/copy remains
available. The affected count is a whole Fluent sentence with a numeric count.
Disabled controls attach no click handler, and intent preparation still
rechecks availability/current catalog at dispatch time. The destructive paint
is used only on the final confirmation. Form drawing is a child module so the
manager phase renderer remains within the strict function budget.
Native and browser transport currently answer Unsupported, with no node I/O
or simulated success, until the pending node contract is available.

Catalog ownership uses the actual answered `Machine.node`, with validated
book names only for routing/display. Never-answered aliases stay unresolved;
alias rename keeps colors, alias reuse by another node cannot inherit them.
Duplicate routes share the node's catalog. A removed host cannot retarget a
pending write. Replica batches validate atomically; gaps revoke authority and
request a snapshot while retaining cached offline colors.

Session-label reference audit: `use-open-agent-tab-labels.ts` is the app's
metadata update consumer; workspace tab rename sends name only. The reference
has no generic session key/value editor. Existing reserved-tab, create/import
and filter consumers must use lifecycle's reviewed contract; no extra editor
is invented for this package. Workspace labels remain a separate domain.

The node branch owns ADR 0086 and real catalog/assignment SQL behind a typed
registry trait. Eight SQL tests pass with an explicit fake registry, not the
production registry. Its store adapter, writer/actor and pending RPC rows still
await the reviewed registry dependency. Native picker entry/create UI,
background observation, registry label projection and complete peer gates
remain owed. No full gate, capture, review or production flow is claimed.

Entry coordination: S2 owns the single sidebar workspace row/menu caller;
W12 owns the workspace header and takes its canonical
`LayoutKey::Workspace { node, workspace }` / `PanelContext.workspace`. Neither
has announced a checked authoritative row/header adapter yet. Both will pass
registry identity/lifecycle plus this owner's ordered assignments; layout does
not store labels or introduce a label pane. No second frame is introduced.

Reference correction (`workspace-labels/index.ts:60–70,145–195,244–260`): a
connected host's failed catalog request does not revoke mutation capability
or remove its cached definitions from the merged choices. The initial model
incorrectly treated this as an unavailable host. Connected failed hosts now
remain writable, while Failed still means the cache is not deletion evidence
for destructive filter reconciliation. Valid contiguous updates recover that
load error without losing the cursor; gaps still require a snapshot. Added
regressions distinguish connection, load authority and mutation permission.

Mutation results settle their correlated form only. Catalog data must come
from the ordered observation, as the reference controller does; a unary reply
has no catalog cursor and patching it into the cache could race an earlier
list response. This keeps one replica owner and avoids a second local update
path before the background observer is connected.

Fleet rebinding now retains a loaded catalog's authority only for a connected
peer advertising the assigned feature version; pending None still refuses
every peer, and reconnect begins Loading. An unresolved alias cannot duplicate
or demote an answered route to the same node, regardless of route order. Added
regressions cover that mixed known/unknown identity case and capability gating.

Merged the current announced line `e5e654b47` (dialog seam `074ade16`) after
reading WORKER BASES. The label area consumes shared `DialogState::dismissed`
only after actual close. A nominal opening identity and captured inspection
ticket bind Escape/scrim/close to the displayed surface and decision; stale
callbacks do nothing. Dismissing confirmation returns to the retained editor,
while hiding an accepted write leaves it owned until its result arrives.
Two pure regressions cover old openings, replaced inspections, confirmation
cancellation and hidden in-flight work. Focus restoration remains the shared
kit's weak-opener policy, not a feature-local focus engine. Checks are pending.

Delete confirmation names the original host label explicitly, even when the
editor contains an unsaved replacement. The pending/read-only field must not
make that replacement look like the deletion target. The existing inspection
regression now retains an unsaved rename through inspection and verifies that
only the original label is submitted for deletion. Edit/delete headers use
whole Fluent messages in both languages.

Action callbacks retain the displayed opening, host and normalized catalog
target. Delete additionally retains the inspection ticket. Save/inspection,
swatch input, Back and final deletion cannot be retargeted by a newer editor
or confirmation before the old callback runs; stale actions are ignored before
ticket allocation or queue submission. Cancel uses the same captured-dismiss
policy as Escape/scrim/close. Added pure regressions for changed openings,
host/target switches, async phase changes and replaced deletion inspections.

Scoped verification: `c266d9b6` passed 38 tests matching labels across app,
link and view, including 24 tests in the owned view label module and both
queue/refusal tests. This predates the final binding and captured-surface/action
regressions. Node `e764f8a6` strict alltarget proto/store/node clippy passed.
The newest native strict check and the added regressions are still pending;
no whole gate, review or inspected capture is claimed.

Search whitespace also follows the reference: trim FEFF, retain 0085, and do
not collapse internal query spaces. The protocol's existing whitespace
predicate now has one edge-trim helper reused by the view's private Query;
picker and manager no longer use independent Rust trim/search expressions.
The added regression distinguishes empty search from no matches at those
Unicode edges. No name grammar, dependency, protocol number or wire field
is duplicated or changed.

[Decision Log] Exact connection ownership after the shared G4 seam
- Purpose: preserve the person's selected alias, credentials and admitted epoch
  through queue delay, reconnect and asynchronous settlement.
- Existing constraint: NodeId owns catalog colors, but it cannot authorize an RPC
  or distinguish two authenticated aliases. The published line 2e95b7d31 supplies
  the canonical ConnectionContext and selected_connection preflight.
- Alternatives: keep the first alias by NodeId, introduce another resolver, or
  carry the shared immutable admission. Only the last preserves caller intent.
- Choice: render explicit book routes, reuse cached colors by NodeId, and retain
  the selected route independently of the pure catalog manager. A missing epoch
  cannot authorize operations. Transport commands and results carry the original
  ConnectionContext; no actor lookup by NodeId, fallback alias or write replay.
- Captured callbacks compare their original context before allocating tickets.
  A confirmation remains pinned to the admission that supplied its inspection.
  Pending results settle by original context plus ticket even after retirement,
  so a refused old request cannot leave the retained form permanently busy.
- Tradeoff: multiple aliases to one node are separate choices; colors remain
  shared node metadata. Reconnect invalidates loaded authority and requires a new
  authoritative load. The staged link still refuses Unsupported for a valid
  selected admission until reviewed registry/RPC production wiring lands.
- Verification: added same-node reconnect/other-alias callback regression and
  missing-epoch authority regression; remote execution and strict lint pending.

Workspace picker callers now have a typed WorkspaceTarget constructor, using
only the canonical LayoutKey::Workspace and original ConnectionContext. It
refuses a mismatched node or legacy session layout and can fence callbacks
against a changed workspace/admission. This is a values seam, not a second
workspace registry, connection resolver or label store. The picker renderer
and caller registration remain unfinished; the new boundary test is pending.
