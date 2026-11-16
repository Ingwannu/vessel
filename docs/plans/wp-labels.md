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

Session metadata and colored workspace labels have separate owners and contracts.
The lifecycle owner reports reviewed source `c88066f2` with V3 final PASS and
all 18 whole-gate steps rc 0 (Rust 2404/0/12). It includes borrowed
ExactSizeIterator `SessionLabels::iter()` returning `(&LabelKey, &LabelValue)`;
`iter().len()` supplies the count. It is now integrated on announced node
`93970acb` (gate 53, all 18 steps rc 0, Rust 2577/0/12, listing schema 21).
The separate A6 node tree adopted it after its narrow run finished. This native
app still waits for the lead's announced app base bringing that node head;
it does not merge a whole node head or parse summaries/JSON as a substitute.
`session.labels` / SetLabels, LabelsChanged and the separate LabelPatch draft are
not included. The earlier planned merge-only shape is not an adopted request DTO.
A6 adds no generic session metadata editor or speculative mutation call.

The reviewed source's ListSessions selection preserves null as the legacy
full list; an explicit selection has required Shown/All scope and AND labels.
Use `ListSessions::selected` with `SessionSelection::shown()` or
`SessionSelection::all()`, then `with_labels(...)`, only after the announced head. Its unassigned SINCE gate refuses explicit
selections. These are producer status
facts, not permission to adopt unreviewed query or iterator code. Session labels
already stored at schema 17 do not establish RPC availability. The owner will
supply an exact reviewed, whole-gated consumer head.

A6 itself owns workspace catalog/colours/ordered assignments and their RPC gap
under ADR 0086. Its separate node worktree now wires the reviewed W11 transaction
helpers and existing post-commit watch into the real writer, actor and handlers.
Those changes still owe their own execution, whole gate and V3 review, so this
app does not adopt them or claim node-backed behavior. The app base contains
canonical registry metadata, but no A6 catalog transport. No dummy protocol
number or successful persistence response stands in for that dependency.

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

Fleet rebinding retains a loaded catalog's authority only for the same admitted
connection advertising the assigned feature version; pending None refuses every
peer, and reconnect begins Loading. Distinct book aliases remain distinct choices
and share cached colors by NodeId. An unresolved alias has no connection context
and cannot borrow authority from an answered route to the same node. Regressions
cover both route orders, missing epochs and capability gating.

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

A semantic session-list refusal is independent of the label connection: the
shared Machine snapshot deliberately retains its authenticated epoch/protocol
when that read is denied. Label availability now uses admission plus its own
feature capability, never Reach (a session-read result). The capability
regression distinguishes denied session reads from a missing/retired epoch.

Two GPUI regressions now exercise the real Shell dispatch with its existing
detached bounded link and isolated `without_io` factory. They verify that a
same-node reconnect rejects an old Save before queue submission; a wrong-context
or wrong-kind answer cannot settle the fresh operation; the original refusal
still settles after transport retirement while retaining exact draft text; and
an inspected deletion cannot be sent on a newer admission. The tests do not
create connections, touch credentials/configuration or prove node persistence.
Their execution is pending the next current-head remote native batch.

W12 follow-up adoption: verified that the exact range base 9e18cf0d is already
an ancestor of this tree and canonical plugin values e46cd57b are present. Merged
only the isolated checked tip 30a4396f (whole eight-commit 9e18..30a range), not
the moving pane/resources producer. It supplies shared captured layout actions
and visibility queries; A6 does not invent an alternative selection fence.
The preferences hook remains literal cfg(test); no canonical preferences or
workspace registry caller is enabled by this merge. W12's scoped source/clippy
and 38-test evidence is producer evidence, not an A6 current-head whole gate.

Merged announced package base 6 (18eedad4) without conflicts after the bounded
W12 follow-up adoption. This brings the line's root d432ef0f through B0 rather
than merging a node head directly into the app tree. The lead reported its
native baseline as 1404/0/2; A6 has no current-head gate result yet. The native
lockfile on this base is announced stale, so no --locked invocation or local
lockfile regeneration is used while awaiting the lead's corrected lock.

### Staged native picker adapter (not a shipped entry)

The independent picker/create renderer is now literal cfg(test) in
app/src/areas/labels/picker.rs. It consumes the reviewed WorkspaceTarget, ordered
assignment names and EditAccess; it never synthesizes a workspace from cwd,
session or an arbitrary alias. Enabling it requires the real S2/W12 registry
caller plus reviewed node mutation/observation plumbing. The global manager
continues to render through its existing production registration.

The picker reuses kit ComboboxState/ComboRow (checked rows, dot leading, pending
rows disabled, multi-select stays open), DialogState and TextField. Manager and
create pages now call the same private swatch painter, preserving the existing
buttons/dots/animation rather than adding another renderer. Create defaults to
violet, reserves one atomic Assign intent, retains text/color on failure and
returns to the still-open list after success through one captured deferred
focus transition. Read-only policy changes only on transitions. No local
assignment or catalog is optimistically updated: registry/catalog observation
remains authoritative, including after unary success.

The link's semantic Assign/Assigned variants retain workspace, original name,
exact context and ticket; valid admissions still receive Unsupported pending
real node RPCs. These are UI intents, not new registered wire methods. Existing
queue-refusal settlement therefore also covers assignments without a second
transport path. Three added GPUI tests use only the detached link/isolated Shell:
per-name duplicate suppression and observer-owned assignments, atomic create
with retained failure draft, and successful return to the list. They construct
the actual kit elements; execution, full layout captures and V4 review remain
owed. Nothing here claims a live canonical workspace menu entry or persistence.

Scoped evidence recovered at 11:31 UTC: native 05dd210b passed exactly 36 owned
filter-selected tests (view 28, app 6, link 1, kit identity 1). This predates
WorkspaceTarget, the Reach correction, actual Shell dispatch and staged picker
regressions, plus package base 6. Node 3bd691bc (gate42 merge, shutdown core)
passed strict alltarget clippy for proto/store/node. Node then merged announced
gate44 ede20014 cleanly, without a whole gate or registry adoption.

The published app line's lock correction dd13ce378 and checked G2 menu updates
are merged. Latest app source 5584eb427 now has a fresh remote alltarget clippy
registered; it is queued, not green. Inactive/read-only workspace hints explain
that target policy rather than telling the person to reconnect to unlock it.
The native create form uses the existing adaptive Dialog because typed combobox
rows cannot embed the persistent name field; Back and async success return to
the original picker. This deliberate presentation difference avoids a second
menu/input renderer while retaining the reference workflow.

Latest 5584 native alltarget clippy reached link and stopped only at the shared
handle function's 105/100-line bound, before checking app. A6's labels arm is
now a single typed Request envelope; its target/ticket/ask stay in labels.rs,
and the shared dispatcher has one registration line. Request::refused centralizes
native/browser/queue local refusal into the original Event shape. No allowance
or shared behavior was changed. The initial failure log is retained; current
app compilation and added GPUI execution are still owed.

Merged announced package base 8 (91407173) cleanly. Its shared FieldId API now
fences both manager and staged create input callbacks by actual input-engine
identity. A new manager edit target gets a new field; the same field survives
pending operations and language changes. Replaced fields' delayed events cannot
change their successors. Two GPUI regressions queue the actual feature callback
with an old engine id, replace the field, then deliver it; the manager test also
proves a current id still edits. This intentionally does not use set_text as an
event injector, because the component's programmatic setter does not emit Change.
The kit producer separately verifies real keyboard events report the engine id.
These new regressions are not executed yet; existing queued 480534 checks its
older immutable source and will be recovered before another submission.

Registry adapter preparation now exists outside the active trees in an isolated
preparation folder with an unapplied patch and SHA256SUMS.
Its textual base is the reviewed registry/Store contract. The types-only
carrier 65180ea8 was subsequently withdrawn as c3429952 because its method rows
require actual node handlers/guard targets; 295 stays unused and a corrected
carrier is forbidden. No carrier was merged here. The adapter now waits for
announced base9 (gate49 batch11+12), containing the full canonical implementation. The draft reuses the writer/savepoint/read
pool and same-transaction labels projection; no physical lifecycle authority.

The 480534 strict run recovered mechanically red at shared dispatch 101/100,
again before app compilation. The typed Labels registration is already one line;
the announced line's existing handle uses the entire 100-line function budget.
Moved only its SavePrefs body verbatim into a private async helper in the same
file. The actor still awaits the identical spawn_blocking save and logs exactly
the same failures before handling another command. No size allowance, dispatch
order, preference ownership or persistence semantics changed. Base8 is present
for the next meaningful remote check, including the FieldId integration.

The shared dispatch extraction is now a separate lead-requested slice. A patch
prepared against c3429952 contains only the existing SavePrefs body moved to
private async save_prefs(Box<Prefs>), with unchanged await/logging/order and no
Labels/Projects arms. The normalized body matches its source exactly. Commit
and remote strict check await that head's green announcement; it is not an A6
handoff or approval to activate registry code.

The 898508 remote alltarget check reached App: shared dispatch and link passed
that compilation stage. It then found FieldId imported from kit's root instead
of kit::field and redundant qualified paths in owned callback tests. Corrected
those paths and passed the prepared Request directly to global dispatch. The
red log remains retained; this is not current app clippy or GPUI execution PASS.
The next run is deferred until the extraction-only shared slice can use the
announced c3429952 base, so the one owned warm lane is not duplicated.

The staged create page now allocates a fresh opening on each Create/Back visit,
not only when the whole picker opens. A delayed old Create/Close/Back callback
cannot target a new blank form in the same workspace; the new form also gets a
new input engine FieldId. A closed picker refuses deferred return/focus effects,
and input callbacks recheck current visibility and write policy. A counterfactual
GPUI test traverses create -> back -> create, then invokes old controls before
proving the current visit still submits exactly its own draft. This additional
regression remains unexecuted pending the next owned narrow batch.

### Current scoped checkpoint

Run `wp-labels-narrow-20261115T145328Z-835e2dda` passed 45 selected tests:
App 14, View 29, Link 1 and Kit identity 1. This executes the staged picker,
input identity, create-visit and actual Shell callback regressions above. Eight
unused-result warnings in View tests are corrected by asserting their expected
replacement label or inspection/deletion name; no lint allowance was added.
The current four-crate strict check remains pending. The result does not prove
live registry/RPC behavior, captures or full package acceptance.

The announced green base `c3429952` is now merged through isolated shared
extraction `2b905dfe2`. Its only changed file moves SavePrefs into the identical
awaited private Box<Prefs> helper. The lead merged it as `92a0ecfc`, with
byte-identical dispatch source, and queued the whole native gate. At the lead's
request, the still-queued standalone strict run
`wp-labels-dispatch-narrow-20261115T152311Z-2b905dfe` was cancelled before taking
a slot (terminal 143); the lead's whole gate supersedes that check. Cancellation
is neither a compiler failure nor a PASS. A6 retains only its one Labels arm
on that shared dispatch. Registry activation still awaits announced base 9.

Merged announced base 8c `2b806402` cleanly; the previous immutable four-crate
strict run remains queued and proves only its own source when it finishes.
Base 9 now follows node gate 50. The shared extraction's lead gate was replaced
by `spike-20261115T153236Z-6ebdf8e8` (same extraction plus G2 picker admission);
the earlier whole run on `92a0ecfc` was cancelled, not passed.

The manager search field now uses the same kit FieldId callback as name editing.
Each fresh opening replaces only that input engine; hidden or non-listing input
cannot change the query. Reopening an accepted pending write retains its query,
while an idle reopen still resets it. A new actual-Shell regression queues the
old callback, closes, attempts hidden input and reopens before delivery, then
checks the fresh field still edits normally. The shared field renderer and
language refresh stay unchanged. This new regression awaits remote execution.

A separate unapplied native edge draft prepares safe error projection for the
future RPC consumer. It requires both broad code and typed reason for precise
label failures and never parses or retains peer diagnostics. Its three draft
tests remain uncompiled; no production transport or shared delivery hook is
enabled by that preparation.

The four-crate strict run `wp-labels-narrow-20261115T153246Z-3365de18` stopped
at two pre-existing View-test lints (manual equality assertion and Vec get/unwrap),
before establishing App strict acceptance. Replaced them with assert_eq and a
typed host-identity lookup. The failure log is retained. The replacement narrow
check includes base 8c and the new search callback fence; no lint suppression or
test weakening was introduced.

The lead announced `6ebdf8e8` green and it is now merged. Raw run
`spike-20261115T153236Z-6ebdf8e8` confirms fonts, fmt, quality, workspace Clippy,
build and tests all rc 0, with 1473 passed, 0 failed and 2 ignored. Shared
dispatch is byte-identical to isolated extraction `2b905dfe2`; the A6 tree adds
exactly its one Labels registration arm. This completes the shared mechanical
extraction handoff, not A6's whole gate or visual acceptance.

The separately registered four-crate strict run on `392f2fcc0` remains queued;
it predates this base merge. No duplicate replacement was submitted. Current
union tests must include the new search-opening regression. Registry wiring
still waits for announced base 9, and both external adapter drafts stay unapplied.

The four-crate strict run `wp-labels-narrow-20261115T155619Z-392f2fcc` reached
App and stopped on five owned lints: two missing documentation backticks and
three change handlers taking an unconsumed String. Callback closures now accept
the kit's owned string and borrow it into internal handlers. No kit signature,
behavior, allowance or test assertion changed. This log is retained as RED;
current execution and strict acceptance remain owed.


## Current session metadata owner and base checkpoint

Merged announced app base 9 `db42d9c5`, dialog base 9b `a7aab9d6`, then
announced base 10 `3e7820e3` (native baseline 1535/0/2). This does not adopt the
unreviewed A6 node runtime. Base 11 candidates and the lifecycle event producer
remain outside this app until their announcements.
The existing manager search retains identified TextField callbacks: the new
shared SearchField currently exposes only an unidentified change callback.
Changing it would discard the queued-old-editor fence. Reuse the same identified
input engine until the shared control offers an equivalent source-aware API;
no area-owned search renderer is added.

[Decision Log]
- Purpose: existing session-label views must read current metadata after a
  lifecycle label change, without changing immutable creation provenance.
- Existing assets and constraints: Card::from_session projects list snapshots;
  ConversationArea.info already retains SessionInfo, including for terminals.
  W12's staged SessionResources retains that same conversation owner. The
  legacy session-id-only record intake is not an admitted metadata carrier.
- Alternatives: a second session-label map, creation-spec fallback, applying
  every historical record, or resolving the selected alias after a reply.
- Choice: prepare an installer into the existing resident-owned current info,
  with passive list projection derived from SessionInfo.labels. No generic
  editor, new admission service or independently authoritative map.
- Why: historical replay and late snapshots must not replace newer labels.
  The node producer must first establish authoritative metadata revision or
  snapshot/watermark semantics; EventSeq alone is not an inferred guarantee.
- Impact: no source installer or event arm is enabled before that exact contract
  and its announced app base. W10 owns the two timeline/conversation metadata
  compile arms; C2 owns the terminal frame arm. Assemble one isolated native
  leaf with their portable deltas, rather than their moving feature branches.

Actual carriers must validate the original ConnectionContext and DeliveryGuard
immediately before W12's receive/settle callback. Info responses additionally
retain the original opaque Timeline/Lifecycle operation ticket and session-id
join. The staged callback does not itself establish live product authority.
The lifecycle owner was asked for exact snapshot/event ordering semantics; no
local counter or speculative SetLabels DTO fills that unresolved contract.

G2 now owns the coordinator-assigned ordered preference-save outcome helper.
The completed shared SavePrefs extraction is preserved; A6 adds no second
helper and its metadata work does not change preference-save behavior.


The lifecycle owner subsequently confirmed that reviewed `c88066f2` contains no
LabelsChanged/SetLabels implementation or ordering guarantee, and `22a85a91`
only prepares values and the existing rename queue. No new event/DTO shape is
frozen. Its newly assigned durable session mutation clock is an ADR-first shared
producer; neither updatedAt nor arbitrary EventSeq is assumed to be a label-map
revision. Current-label apply/Ready remains held until tested source proves the
atomic labels/cursor snapshot, including initial creation's live-to-store edge,
or establishes a typed watermark/causal-read contract. W10/W12/C2 share this
hold, and no native compile leaf has been installed.


## Current resume point after base 14.6

Merged only announced app base 14.6 `be93a803`, retaining history and label
registrations and the shared preference-save implementation. Local source size,
token and diff checks pass; current native compilation/runtime/captures remain
owed after the older scoped 45-test result. No whole node head entered this app.

The separate label node source includes announced ed4 and exact generated TS.
Its first frozen whole gate passed format/size/boundaries, then failed workspace
Clippy because our new label ErrorDetail was absent from the CLI diagnostic
fallback. The explicit arm and a safe-display regression are corrected on the
same base; actual corrected whole verification and V3 acceptance remain owed.
Prior171d six-package all-target strict and 38 actual runtime tests are scoped
evidence only. Label methods remain pending None.

Both sidebar and layout owners reaffirm no checked authoritative native
workspace row/header runtime producer for original context, active fact and
ordered assignments. Keep the picker staged and unregistered; presentation keys
are not authority. The current-session label watermark/event producer is also
separate and unready, so no current-map apply/Ready transition is installed.

The separately assigned additive SearchField leaf reuses engine FieldId
callbacks and id/focus/set_text without changing plain new/clear/render. Original
focused runtime passed the forced old-editor rejection and both existing tests,
but its new Enter event-count assertion failed. The test now follows the existing
single-line normalization path, with product behavior unchanged. Corrected leaf
`790a5f84` on base 13.1 passed Kit all-target strict and actual focused4/0/0,
243 filtered. The docs-only final `b7c199edf` passed V1 row304; the lead merged
it into green base14.1 (native1755/0/2). That producer is now adopted here.
The separate manager consumer below retains its opening/source callback
regression; its own narrow execution, V1 disposition and A6 gates remain owed.


## Shared manager search consumer

[Decision Log]
- Purpose: the label-manager filter needs Paseo's standalone Search/X surface
  while retaining the real editor identity used to reject stale callbacks.
- Existing assets and constraints: SearchField delegates to the same TextField
  engine; identified methods passed V1 row304 and are on green package base14.1
  `b54abbdc`. The existing manager opening/Listing/FieldId fence must remain.
- Alternatives: keep ordinary form chrome, draw a private search wrapper, or
  adopt another input library. Each duplicates existing shared behavior or loses
  the source search surface; no external API or dependency is needed.
- Choice: change only the manager search constructor/type and shared element
  call. Use the engine FieldId to namespace its element. Keep the name editor,
  localized placeholder, focus, silent query restoration/reset and callback
  admission unchanged. The shared control owns its clear target.
- Impact: one shared Search/X renderer with the source maximum width and clear
  behavior. Picker/workspace authority and current-session metadata remain
  staged/held. This consumer change is uncompiled until its remote narrow checks
  run; producer4-test/V1 acceptance is separate from manager or A6 acceptance.

Merged only announced app base14.1, retaining new execution incarnation IDs and
own label IDs in the proto root export union. This app merge does not advance
the separately frozen node package, whose corrected whole gate remains queued.


Consumer regression uses a fake Shell with a visible manager and fixture catalog,
types through the real shared engine, then clicks its actual X hit target and
requires both the manager query and engine text to clear. The earlier deferred
old-opening/source rejection test remains unchanged. These are UI fixtures only,
not evidence of an authoritative workspace projection or a connected node.


The separately frozen node whole2 ran all18 steps:17 passed, while Rust returned
2738/2/12. Both failures were the inbox predecessor-replay fixture retaining the
new A6 catalog tables; no product SQL/assertion was changed. The corrected
fixture focused suite passed5/0/0 on the same frozen node base, and whole3 is
pending. CLI regression, TSdiff0, web465/E2E80/forge4 were green on whole2; this
is scoped evidence, not node acceptance. The app independently adopted announced
base14.5 before its still-owed remote strict and label consumer tests.


The final frozen node whole3 passed all18 steps on `dbecc82d3`: Rust2740/0/12,
TS5/diff0, web465/E2E80/forge4. Its result-only documentation does not change
executed source. Node source review/number assignment/receiving acceptance remain
owed. The app independently adopted announced base14.6; its manager consumer is
still uncompiled until the following remote native narrow. Shared producer
V1 row304 is complete and does not substitute for this consumer's verification.


Native consumer strict `wp-labels-20261116T044136Z-14bbb9b9` reached App checking
and failed only the new SearchField comment's missing Markdown backticks.
Correct that comment only; no product expression, fixture or assertion changes.
The actual consumer runtime has not run. Keep the red result; corrected strict
and A6 App tests are still owed before a bounded consumer review request.


## Actual manager consumer narrow result

Exact `a8533798f` passed App all-target strict at
`wp-labels-20261116T044357Z-a8533798` (fonts0, actual Kit/App checking, cargo0).
Actual App binary tests at `wp-labels-20261116T044452Z-a8533798` compiled the
consumer and passed15/0/0,196 filtered. The new visible-manager real shared-X
case, old search close/reopen source rejection, replaced-name editor and existing
staged picker/dispatch cases explicitly pass. Step hashes are fec78c20a9077dc8
and abf431d671eb41dc. This result-only follow-up changes no Rust source.

Bounded V1 consumer review is owed. The three changed app files depend on the
broader A6 source, so their delta is not an independently existing manager on
the lead line. Pending catalog transport/observation/canonical workspace caller,
current-session metadata ordering, full native gate/captures and V4 remain owed.
