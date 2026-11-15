# Workspace files, editor and document panels

Status: implementation in progress; no package acceptance yet. App base 4 and
the checked dialog seams (`e5e654b47`, containing `074ade16`) are merged. Node
companion includes Quality B, app base 3 and announced node head `ba45a6e3`.

Current evidence: exact `0df7abd9` strict app all-target Clippy passed and eight
focused real GPUI file/find/mutation tests passed. Later navigation, frontmatter
and preview work is pending a new narrow check; the first frontmatter test build
failed on an ambiguous test `Arc` constructor, now corrected. Node's five new
methods remain pending `None`: current wiring checks, unit tests, generated
TypeScript, whole root gate and V3 security review are outstanding. No whole
native gate, visual acceptance, PDF/SVG renderer or complete C5 claim exists.
The dated checkpoints below preserve their original scoped evidence.

## Reference and existing assets

Paseo 0.15.4 source is the behaviour reference (read before implementation):

- `packages/app/src/file-explorer/tree.ts:44-121,177-230`: flattened tree,
  directories first, name/modified/size ordering, restoration to depth five,
  and revealing dot paths by enabling hidden entries.
- `packages/app/src/file-explorer/visibility.ts:3-18`: hidden means a name
  beginning with a dot; default visibility is true (panel store).
- `packages/app/src/components/file-explorer-pane.tsx:478-526,716-764`:
  sorting/visibility and prefix rewriting after directory rename.
- `packages/app/src/file-panels/source/panel.tsx:63-78,158-176`: desktop
  editing at most one MiB, readonly on mobile, flush when disposed, one
  buffer shared by source and preview.
- `packages/app/src/file-panels/source/editor/model.ts`: guarded saves,
  autosave, changed/deleted conflicts and retained edits.
- `packages/app/src/file-panels/builtin.tsx:15-85`: source/Markdown/HTML
  panels, image preview and file-panel errors.
- Survey: `parts/p4-files.md`; catalogue and connections were inspected.

Existing assets: `view::workspace::{file_tree,editor,save}`, the kit's
`CodeEditor`, `link::workspace` typed request/answer routing, and
`app::areas::workspace::code` already implement lazy tree reads, bounded
source viewing, create/rename/delete, guarded editor saves and file search.
The node already implements read-only `lsp.*` saved-file queries.

Decision: **modify existing assets**. Keep rules in view, calls in link,
and rendering in the existing code area. Do not add another editor engine,
filesystem sandbox, parallel preferences file or RPC dispatcher.

## Structure and data flow

The tree retains validated relative paths and selection/expansion per
workspace. File buffers are identified by node, root and path, independent
of pane, renderer and cursor. Layout owns tab/pane identity and close
requests; files owns buffer lifetime and save confirmation. A file request
echoes its identity in its response so late replies cannot reach another
root or file. Pending saves must settle retained buffers after tab changes.

Native app -> link typed request -> authorized node workspace root ->
workspace file implementation -> typed answer -> retained buffer -> panel.
LSP queries use saved source positions only and do not enable repository
execution or change the node's existing trust policy.

## Differences and dependencies

- Vessel's content-hash save guard is retained instead of Paseo's inode/
  timestamp revision: it already protects files on coarse filesystems.
- The initial node has no binary reader, duplicate operation, single-file
  subscription or directory suggestions. ADR 0083 in the node companion
  owns those additions; no invented call is sent by the app.
- Modified ordering requires node metadata; it must not be fabricated from
  entry order or a missing timestamp.
- Existing node path restrictions (including `.git`) remain the security
  floor; symbolic links and special files are never blindly followed by
  binary reads or directory duplication.
- Layout contract is supplied by the layout area: root+path file target,
  stable tab and pane ids, explicit close plan and open placement.

## ADR: pure Rust document renderers

[Decision Log]
- Purpose and intent: view SVG and PDF inside file tabs without external
  programs, browser script execution or native library installation.
- Existing implementation and constraints: kit already uses `resvg 0.46`
  for static artwork; file reads currently omit binary bytes. Standard Rust
  does not parse or rasterize either document format.
- Alternatives considered: PDFium/MuPDF require native libraries; `lopdf`
  parses/edits PDF but does not provide a complete rasterizer; a browser
  creates a separate process and content/trust surface.
- Choice: reuse `resvg` for SVG; select `hayro` for PDF after binary file
  parity. Both are maintained pure Rust projects. The upstream READMEs and
  manifests were checked on 2026-11-15: `resvg` current main 0.50 and hayro
  current main 0.9 use `Apache-2.0 OR MIT`. The local locked `resvg 0.46.0`
  registry manifest was checked separately: it also declares
  `Apache-2.0 OR MIT`. Hayro release locking and transitive licence review
  remain pending until its dependency is added.
- Why: resvg is already present and has a large static SVG regression
  suite. Hayro separates parsing, interpretation and bitmap rendering and
  avoids a C library deployment dependency.
- Advantages, costs and effects: static preview cannot run document scripts;
  it needs explicit source-byte, page, raster-dimension and memory bounds,
  disabled external SVG resources and bounded background work. Hayro warns
  of malformed-input resource exhaustion; loading a document is not proof
  of a hard execution-time sandbox. Incremental binary sizes remain
  unmeasured until a build; upstream resvg reports a CLI below five MB,
  its text feature about 400 KiB, and hayro's CMaps about 250 KiB. These
  are upstream estimates, not measured app growth.

Primary sources: <https://github.com/linebender/resvg>,
<https://github.com/LaurenzV/hayro> and their versioned manifests/licences.

## Validation and resume point

Per-commit narrow checks use the assigned build slot and private target.
At package end: one native gate, fake-node UI capture matrix, each image
inspected, then one app review. The node companion has its own root gate
and node review. No live preview or real harness is used.

Done: reference/source and existing-asset survey; own isolated app and node
trees; missing node contracts reported and assigned to this package.
In progress: correlated editor read/write tickets, serialized saves after
typing during an in-flight write, root/path-owned retained buffers with a
dirty-buffer eviction refusal; name/size sorting and persisted hidden-file
choices; root create controls and portable one-segment name validation.
The layout adapter exports `ExplorerKind`, `show_layout_explorer` and
`render_explorer`, used by the existing panel as well. Native fast checks
(format, size, boundaries and tokens) pass. Narrow compile is still pending.

The companion node core has bounded binary pages, exclusive sibling
duplication, parent-directory observation, directory suggestions and rich
directory metadata; its narrow workspace clippy passed. It is not yet a
registered API or a consumer-ready commit. Next: narrow tests and app
compile; finish persisted expansions, modified ordering, retained I/O
routing and pane rendering; integrate the common node base, then LSP,
document rendering, captures and the final gates. Nothing is claimed
complete yet.

## Correlation regression checks (2026-11-15)

Retained `FileKey` uses the existing validated machine-book `Name`, authorized
root and relative path. Readonly loads now echo the same monotonic ticket
family as editor reads; a refresh invalidates the previous response without
discarding its currently displayed content. Shared picker searches retain a
stable wire `SearchKey` for node cancellation and a distinct local ticket
for every acquisition. Comparing only query text cannot reject the first
response in `A → B → A`, so tracked replies never fall through to another
consumer's legacy response path. No new node method is needed for correlation.

Headless shell tests use the existing isolated `without_io` constructor: no
node connection, updater, real agent or filesystem is started. They exercise
same-path readonly refresh, retained editor responses, wrong node/ticket
refusal, repeated query generations, cancellation and registration budgets.
These tests are written but not yet executed. Remote narrow clippy on
`18aded91` failed at app UI imports, extracted method visibility and one
LSP row signature; this source corrects those errors. Earlier red runs
remain failures; no whole gate or screenshot approval exists yet.

## Mutation ownership and shared base 2

App base `6036eeb8` is merged (typed machine welcome node id and the shared
field events/editability seams). Explorer refresh now reuses the existing
bounded folder/file refresh controller. File rename/delete has a single
tracked mutation registration; acknowledgements check node, root, operation
and ticket before any resource or inline-form update. Dirty or pending
source/target resources refuse a mutation. Clean source controls are held
readonly until it settles, preventing new edits from racing the operation.
A successful directory rename rewrites retained keys, viewer paths, expanded
folders and cached listing keys by complete path components. Source controls
are rebuilt because their callbacks capture the original key; the clean
model and caret line survive. Deletes release only the affected clean
resources. An old root's acknowledgement settles its retained resources but
cannot close the current root's file. Regression tests cover those cases.

Difference: renaming/deleting an unsaved file currently asks the user to
save and retry through an explicit refusal instead of opening a save/discard
modal. This preserves edits while the W12 close/lifecycle seam is pending.
The destination must have no resident buffer to avoid rekeying over another
buffer's lifetime. No acknowledgement is interpreted as proof of a mutation
without its matching local ticket. Node-side filesystem confinement remains
the authority; the UI checks protect buffer lifetime, not filesystem access.

`1df27749` narrow clippy reached the app with no library type errors and
failed on test fixture ID constructors plus strict app style lints. Those
failures are corrected in this source; the newer GPUI find/replace/Escape
and mutation tests remain unexecuted until the next narrow check completes.

## Source byte conventions and manual save

The editor model retains `TextFormat` (first observed LF/CRLF/CR separator
and UTF-8 BOM) independently from normalized editor text. Reads, guarded
writes and queued disk reads preserve these conventions; semantic comparison
uses normalized content so a separator-only disk change does not create a
false conflict. Save steps encode the wire text but track normalized
persisted content, preserving the in-flight serialization invariant. Tests
cover CR, CRLF, BOM, trailing newlines and a queued format change during a
write. The actual editor's primary-S handler invokes the same guarded save
model; it does not send a parallel unguarded write.

Narrow clippy on `48e876ed` failed only at an omitted `Div` import and an
inferred key-event type; both are corrected here. This is compile evidence
only, not an executed test result or package gate.

The source editor now consumes the shared kit `Editability` seam from base
2. During a mutation hold the engine rejects edits/replacements and find
keeps navigation while hiding replacement controls. The integration test
explicitly attempts a readonly replacement before releasing the hold and
replacing normally. Narrow clippy on `336f69e5` reached all app targets;
remaining failures were test helper/line-count lints and one lazy Option
closure. They are fixed here; no executed test result is claimed yet.

## Panel resolver and source navigation

The pure `view::workspace::file_panels` model follows Paseo's catalogue and
resolver. It filters by real file kind and literal selectors, suppresses the
Source floor beside a built-in renderer, chooses explicit tab selection
before a valid association before catalogue order, and asks only when a
plugin is among multiple candidates. Exact filename claims use a separate
association namespace; extensions fold Unicode case. Plugin identities are
generic and will use W16's canonical `ContributionAddress`; selectors borrow
projections of its validated declarations. No plugin registry/runtime or
renderer is introduced here. PDF/SVG identities cannot appear unless their
owner registers real candidates; the resolver does not promise an available
renderer merely from a filename. Catalogue work is bounded at 128 entries.

`FileLocation` validates one-based scalar ranges using existing protocol LSP
positions. W12 confirms that locations remain an owner payload separate
from its root/path `Target`. The kit adapts them once to the rope engine's
zero-based scalar coordinates, clamps stale lines/columns to the loaded
source and preserves UTF-8 boundaries. Definition navigation now includes
its actual range, and rename preserves both caret line and column. A GPUI
fixture selects an emoji between Korean characters, replaces it through
actual input and checks the model, then reveals an out-of-range line.
These latest resolver/navigation additions are source-only until their
narrow checks run.

Validation: exact app `0df7abd9` strict all-target clippy passed in remote
run `wp-files-narrow-20261115T075904Z-0df7abd9` (native-fonts and native-cargo
rc0). Focused app code tests for that same commit are queued separately in
`wp-files-narrow-20261115T080536Z-0df7abd9`; a clippy pass is not a test pass.
No whole package gate or screenshot approval exists yet.


## ADR: Markdown front matter parser

[Decision Log]
- Purpose and intent: port Paseo `markdown-preview/document.ts`'s YAML core
  mapping table, including ordered keys, scalar lists and nested values.
- Existing assets and constraints: the kit already renders Markdown. Rust's
  standard library cannot parse YAML; a hand-written scalar subset would
  silently disagree on quotes, aliases, lists and nested maps.
- Alternatives: deprecated serde-yaml; an unrestricted generic YAML tree;
  dropping front matter; a second Markdown renderer. None provides the
  reference semantics with explicit parser work bounds and existing kit use.
- Choice: the already locked `serde-saphyr 1.3.0`, only serialize/deserialize features, in
  view's private document-format parser. crates.io and the downloaded release
  manifest/licence files confirm MIT OR Apache-2.0, Rust 1.89 minimum, and a
  non-yanked release. `granit-parser 1.3.0` is its existing pure Rust parser.
- Why: YAML 1.2 core scalars with explicit budgets before AST allocation,
  while retaining the shared Markdown renderer and no local resource I/O.
- Advantages, costs and effects: source <=10 MiB; front matter <=64 KiB,
  depth16, aliases100, events/nodes4096, and bounded retained/replayed alias
  work. Parse/format errors, non-maps, duplicate keys or exceeded budgets
  preserve the complete original source. Complex mapping keys are rejected
  rather than inventing JavaScript object-key coercion. These additional
  resource limits are stated differences; binary-size growth is unmeasured.

Primary release sources: <https://crates.io/crates/serde-saphyr/1.3.0>,
<https://github.com/bourumir-wyngs/serde-saphyr>. The exact locked 1.3.0 registry source and its parser budgets were inspected;
this reuses both existing packages rather than introducing a second version. Parser tests cover ordered CRLF/BOM metadata,
scalar/list/nested display, invalid/non-map/duplicate/deep headers, alias
amplification, literal property syntax and the source budget. UI wiring and
this parser's narrow compile/test result are still pending.


## Shared Markdown interaction producer

Only the relevant link parser and kit Markdown/action files from W12b's
independently checked `8403be97` are adopted (kit/view strict all-target
Clippy, composer-shell23 tests). The composer-shell module registration
exposes only links here; unrelated producer models are not copied. Base4's
kit Palette/Editability/terminal exports are preserved. The old renderer
entry point remains source-compatible; file preview will use the new
Options/Actions path with host-owned link and all-image resolution. No
relative/file URI is handed to an OS opener or a default filesystem image
loader. C5 still authorizes file roots and controls actual acquisitions;
the parser grants no authority. Producer's later staged extension is not
consumed until checked.

### Markdown preview owner checkpoint

The app now retains a Source/Preview choice with each Markdown viewer (Preview
by default; explicit line navigation selects Source). Preview metadata parsing
runs on the GPUI background executor only after a complete source arrives and
uses a unique file ticket. Refresh/edit invalidates that ticket, so an earlier
parse cannot replace newer source, including while the buffer is retained.
`code/markdown` owns this lifecycle; `markdown/draw` uses the shared kit renderer,
selection and code-aware copy; `markdown/links` authorizes paths relative to the
captured file directory. Lexical parent traversal cannot leave its authorized
root; node path resolution remains the second wall.

This is source implementation, not a passed UI/gate claim. Inline images are
currently explicit unavailable resources through the host resolver, never a
fallback to the workstation filesystem. Absolute/home file links await the
shared admitted workspace resolver; W12 tabs and G4 connection leases are still
pending. The existing Name-keyed buffer rename limitation remains outstanding.
`file_workspace_scope` exposes only the active session's resolved root snapshot,
not proof of a live lease, and makes no extra node request.

### Reload decision lifetime

Reload/Overwrite actions capture the original retained FileKey. A destructive
reload prompt additionally carries its own FileTicket, including Cancel,
Escape and scrim dismissal through the shared captured-dialog API. A queued
answer from an older opening cannot consume a newer prompt at the same path;
a tab switch cannot apply its decision to another file's dirty content. The
new isolated shell regression pins both cases. This prepares owner-scoped
pane rendering without a second dialog renderer or active-file alias lookup.

W12's1720 producer was adopted with its reviewed canonical value dependencies,
then placed on HOLD by its owner pending a self-contained shared-line successor.
The source remains retained, no consumer-ready claim is made, and no File target
is offered by its launcher. G4 live connection admission and the actual resident
resource cutover remain outstanding; neither is inferred from Machine.node.

### Entry action opening lifetime

A menu, inline create/rename form, and Delete decision now retain an
EntryOpening (validated book Name, authorized root snapshot and unique
FileTicket). Enter, menu selection, confirm and cancellation check that exact
opening; mutation construction uses its captured root. Changing the followed
session invalidates the old action UI. An old callback cannot borrow the
currently selected root or consume a replacement form at the same path.
`code/entry_forms` owns submission/cancellation and `actions` owns menu/form
state, retaining the same mutation controller and kit controls. Isolated GPUI
regressions cover stale menu/Delete/Enter/Cancel and a valid current opening.
These tests are source-only until the next combined narrow suite. Book Name
still needs G4's canonical live epoch admission; this patch does not claim to
solve alias reuse or the pending B1 composer-destination integration.

### Shared submit capture

The first real form regression build found that TextField submit accepted only
a function pointer; a request-scoped closure could not compile. The existing
one-line/multiline/composer constructors now accept an immutable captured Fn,
retained in the same field event subscription. Existing function items still
fit; edit/IME/Enter ordering and the input engine stay unchanged. This extends
the existing kit control instead of adding a second form controller. The real
input regression retains the original captured identity after owner state changes.

The first size check caught the callback signatures adding two product lines
above field.rs's existing 635-line record. The original one-line/multiline
construction moved to field/construct.rs, leaving the public constructors in
place and using the same event subscription. The listed budget is not raised;
this separation keeps construction and input event delivery in their own modules.


## Original-connection cutover checkpoint

[Decision Log]
- Purpose: root metadata must belong to the original authenticated node and admission, including when the same book name is reused.
- Existing assets: G4 `ConnectionContext` and `State::selected_connection`, the one workspace file/LSP runner, W12 file targets. These are extended rather than adding a connection registry or filesystem client.
- Choice: a closed `workspace::scoped::Request` carries an exact metadata ticket or validated `FileScope` plus existing `FileAsk`. Admission happens on the shared actor; ordinary reads have a finite30-second deadline and every call uses the G4 before/after fence; the original direct write now deliberately has no local deadline (terminal-evidence section below). Sixteen operation slots bound detached tasks. File requests reject a root mismatch before dispatch. Safe error projection retains only the closed node code.
- Why: cloning a client by a mutable alias cannot preserve old intent. The shared file runner still performs exactly one write and at most one conflict readback. Old workspace consumers retain a private legacy adapter while C5 migrates.
- Cost/remaining: the root metadata path is migrated first. Canonical NodeId/root/path buffer keys, original-ticket settlement after retirement, all file request callsites, Finder cache, and the pane renderer remain unfinished; this checkpoint is not an approved consumer opener or complete live resource cutover. No dirty/uncertain preservation claim is added.
- Integration: merged the official shared app line `2e95b7d3`, preserving C5 Intelligence registrations. It supplies the existing `service_ports` approval adapter that the earlier app/root union lacked. The prior exact `267d036c` Clippy run stopped on that missing field before C5 compile validation; no tests or full gate are claimed.

## Sticky original write obligations

[Decision Log]
- Purpose: undo-to-baseline must not make an ambiguously completed remote write disposable or permit a second write.
- Constraint: G4 fences before/after each call but retirement, deadline, or disconnect can hide a result after dispatch. A tracked node filesystem operation may continue after the local future ends. A node error may follow publication while result-version acquisition fails.
- Alternatives: reread after reconnect, treat local cancellation as completion, or add durable write receipts/journaling. A fresh read can finish before the old blocked write publishes; it is not termination evidence. A new protocol/journal is separate scope.
- Choice: the existing model retains one original `WriteAttempt` with sequence, submitted text, original guard and `Pending`/`Uncertain` effect state. Its existing owner ticket remains held. Local edits/undo remain available, but reads, save/flush/overwrite and destructive reload never clear uncertainty. A matching definitive original result may settle; duplicate/wrong-sequence results cannot. No read observed during uncertainty is replayed after completion.
- Projection: no-guard, queue rejection, preflight refusal and unavailable protocol are known not sent; unclassified failures from an invoked write are conservatively uncertain. Generic remote errors are not assumed to be pre-effect refusals. A failed conflict readback means changed with disk state unavailable; it is not evidence of deletion.
- Tradeoff: if the only definitive original result is lost, the owner may remain held indefinitely. This is an explicit availability cost; ordinary quit/eviction must refuse it. No forced-exit journal or crash recovery is implemented. The legacy node text-read split-hash and check-then-rename concurrency limitations are not fixed by this owner migration; no coherent-snapshot/atomic-CAS claim is made.
- Checks added: pure model undo/read/reload/retry/wrong-ticket/definitive-completion scenario, known non-dispatch versus ambiguous save projection, and actual isolated GPUI retained-owner undo test. Source is not yet checked; focused tests remain owed.

## Canonical file ownership cutover

The file key is now `NodeId + WorkspaceRoot + RelPath`; route names and connection epochs are not buffer identity. Each retained viewer carries its current original `FileScope`. A write ticket separately retains the scope under which it was attempted, so rebinding a viewer cannot rewrite outstanding intent. The generic alias-based `Ask::File` path and generic workspace file answers are removed: all file/LSP effects go through the shared selected-connection adapter and closed `FileAnswer` family.

Root receipts for mentions are keyed by actual node/session and require exact original context plus acquisition ticket. They are bounded metadata facts, not a second connection registry. Old-context reads settle only their original local acquisition and cannot install text; original save results settle their own owner. A definitive save from a retired context enters changed/conflict rather than installing an old baseline or scheduling a new-context write. Reads observed during retirement are invalidated. No cwd or alias-by-node fallback exists.

Explorer menu/form openings capture the full original scope. Rename/delete holds are keyed by canonical node/root/path; a possibly dispatched failure keeps its hold and original ticket instead of releasing clean buffers. A conflict is a terminal rejection; other invoked mutation failures are conservatively uncertain. Forget/rename invalidate transient entry UI but retain active and parked dirty/uncertain owners. Retention still refuses admission rather than evicting protected owners.

Regression source now covers a reused alias serving a different actual node with the same root/path, retained drafts after forget, and an actual framed write retired after dispatch with one original request and no replay. Existing isolated GPUI helper APIs that accept an alias are test-only compatibility adapters; product receipts never reconstruct file identity from a name. New-source checks are pending, not a checked consumer handoff. File tabs, typed Explorer placement, lifecycle registration and the remaining preview/transfer surfaces are still incomplete.

## Initial empty-query files and folder refresh correlation

`FileQuery::All` is an explicit owner acquisition; `Matching(FindQuery)` retains the existing nonempty protocol query. All reuses exactly one scoped `file.list` for the requested root, with no invented blank `file.find` and no second walker/index. Only valid, nonignored regular-file children become initial results, bounded by the caller's existing limit. `complete` stays false because one parent listing does not cover the workspace. This is the coordinator-approved requested-parent difference until ADR0083 suggestions can be admitted, rather than claimed recursive Paseo parity.

Every local folder-list ask now echoes a `FileTicket` without changing the wire call. The Explorer stores its newest ticket by directory; a repeated same-scope refresh cannot install an older listing. Initial picker listings use the same global SearchKey allocator/controller and their own ticket, so cancellation/reopen cannot consume another field's result. Successful rename invalidates affected pending listing tickets and removes their loading placeholders, permitting the new path to be read afresh. New same-scope refresh and initial-list generation regressions are source-only until the next narrow tests.

## Focused source Find ownership

The existing global `Action::Find` dispatch first asks C5 `file_find_at_focus`. It requires the actual CodeEditor engine focus or that exact owner's find/replacement field focus, canonical current buffer identity, and the mounted legacy Files body. A displayed file/selected session alone never intercepts Find. It reuses the existing engine's `open_find`; no second keyboard listener, composition registry or text matcher is introduced. The kit adds only an actual-focus getter in its existing find module. The isolated GPUI regression focuses the source, then the composer, then an unmounted source to pin refusal boundaries. The typed pane/Explorer mount will replace the legacy visibility guard when W12 supplies its real context; no multi-pane focus claim is made here.

## Keyed rendering callbacks before pane registration

Deferred editor creation now captures the original canonical file key and locates that retained owner, rather than creating a control for whatever file is active when the callback runs. Read-only virtual rows, line selection, source save/conflict/reload chrome, Markdown mode changes and LSP queries/navigation carry the originating key. Back refuses another active file; selected-line toolbar callbacks cannot act on another buffer. The actual GPUI regression asks for A's deferred control after B is active and verifies B remains untouched. This removes global-file lookups at these callback boundaries; per-tab presentation entities/cursors and typed W12 renderer registration are still separate unfinished work, not established by this checkpoint.

## Saved-file observation adapter (pending protocol admission)

[Decision Log]
- Purpose: retained dirty buffers must meet external edits without a second host watcher, while a clean visible source can refresh.
- Existing assets: ADR0083's finite `ObserveFile`, node notify parent watcher, G4 unary admission, original C5 file tickets and serialized editor lane. No new wire method or connection manager is added.
- Choice: one observation per retained owner, separate 64-slot read-observation admission from the existing 16 ordinary file operations, and a 65-second client ceiling above the node's 30-second heartbeat/60-second task guard. Replies carry original scope/ticket; retired replies pause without installing content. Only active or protected owners continue. Clean parked owners stop renewing after the current finite observation.
- Why: long waits must not consume save/list admission. A node without a reserved principal wait slot may return an unchanged snapshot immediately; the owner enforces a one-second minimum request interval, preventing a busy loop. Actual read failures pause visibly and explicit reopen/refresh may retry under the same original admission; there is no retarget or write retry.
- Write boundary: observation anchors are separate from the persisted editor baseline. A change marks reconciliation needed, but no reconcile read is sent while the original write/read lane is pending or uncertain. A changed/missing observation never settles that write. Original terminal completion can then allow a fresh read; stale observations are not completion evidence.
- Admission/status: `FILE_PANELS_PROTOCOL` remains None until the integrator assigns it. The native adapter is therefore disabled on the current pending protocol and is not claimed as product-ready/live parity. It will use the assigned feature check plus exact live shared context, never an invented method number. The GPUI synthetic owner regression and interval-floor test are not real-node/live-watch proof.

A failed initial read is retryable on explicit reopen/refresh with a new read sequence; an older failed acquisition cannot install data into that retry. This also permits a paused observer to be retried by explicit reopen without implicitly retrying an original write.


Exact a3d7b3a1 strict four-crate all-target narrow15 finished rc1: `file_scope.rs` rustdoc backticks and captured-submit regression trailing semicolon only; both corrected. Kit library/type checking reached the captured Fn callback seam, but the all-target run is not green and Link/App checking remains owed. Latest canonical ownership/observation code was not part of that older immutable run.

Deep directory rename rewrites now join each already-validated suffix component individually. Passing an entire multi-component suffix to `RelPath::child` was incorrect because that API deliberately accepts one basename; clean deep descendants could otherwise disappear from the retained map. Pure nested-path and actual retained-owner rename regressions pin preservation and component boundaries.


## Existing-file source overwrite

[Decision Log]
- Purpose: editor overwrite must preserve a missing file as deleted and must not turn a failed conflict read into permission to force a write.
- Reference: Paseo source editor reports deleted files without an overwrite action (survey p4-files, source editor conflict lifecycle); its existing-file handle refuses recreation.
- Existing assets: the generic `save_text` accepts explicit `Any`, the node already supports a content-hash guard, and the admitted C5 runner owns its one write and one read. No additional protocol, actor, dependency or authority is introduced.
- Choice: the source runner uses `save_existing_text`. Normal saves reuse the existing guarded save. Explicit overwrite first reads at the editable one-MiB ceiling, confirms whole editable text and its hash, then sends one content-guarded write. Missing/unreadable/binary/partial/oversize snapshots send no write. A new conflict after this read is never retried.
- Why: reusing `Any` could recreate a deleted file. An unavailable snapshot is now a separate unverified conflict with Reload but no Overwrite; only an actual NotFound readback reports deletion. The generic helper remains source-compatible for other explicit unguarded consumers.
- Tradeoff: this strengthens intent under the existing wire contract but is not an immutable read snapshot or atomic filesystem compare-and-swap. The existing node hash/read and guard-check/rename gaps remain documented limitations. Invoked unknown write failures retain their original sticky obligation; pre-read failures are known not sent.
- Validation: added ready-boundary tests for fresh hash, missing/no-write, pre-read refusal, binary/partial/oversize refusal, guarded race/no-retry and unavailable conflict readback; model regression preserves the unverified draft and disables overwrite. These new tests are source-only until the next meaningful narrow run. Fast size, crate boundaries, token checks pass; no budget is raised.


## Modification-time tree sorting

[Decision Log]
- Purpose: preserve Paseo's Name → Modified → Size tree cycle without inventing modification dates or statting each row.
- Existing assets: pending ADR0083 `InspectDirectory`, typed optional `FileModifiedMillis`, the scoped file runner/tickets, existing `Listing` indices and pure tree flattening. Reuse those; no new protocol or directory watcher is added. Paseo discovers external tree changes on explicit refresh, so a root watch is unnecessary here.
- Choice: when the assigned feature protocol is admitted on the exact captured connection, each requested directory uses one inspection instead of an additional list. The same reply projects existing entry data and optional timestamps. Modified sorts newest known values first, unknown values last, directories first and deterministic name ties; rows preserve original snapshot indices.
- Correlation: metadata installs only with the matching original directory ticket and scope. Legacy refreshes clear older metadata for that directory. Root switches/forget clear it; acknowledged directory rename rewrites component-safe metadata keys and delete removes them with existing owner state.
- Difference/admission: the constant remains None, so live inspection and the Modified cycle are not offered yet. Older nodes retain existing Name/Size listings. Persisted Modified with unavailable metadata uses deterministic name ties rather than fabricated epoch values. No live parity or assigned-number claim is made.
- Validation: source regressions cover optional/epoch timestamps, directory ordering/ties, immutable row indices and clipped rows, projection scope/ignored/clipping facts, stale same-scope metadata and a subsequent legacy refresh. Fast size/boundaries/tokens pass; strict/native tests for this new source remain owed.

## Complete read-only native source

[Decision Log]
- Purpose: read-only source and Markdown must not silently stop at the first file.read page; native highlighted/plain/unsupported selection follows the reference 1MiB/10MiB budgets (survey p4-files, TextFilePanel source presentation).
- Existing assets: the ordinary typed paged ReadFile, G4 selected original connection, one retained file ticket, shared view lexer and existing virtual rows. Reuse them without another text engine, highlighter, protocol or filesystem path authority.
- Choice: the C5 readonly runner acquires complete text through at most64 one-MiB requests under one30-second overall deadline and the existing operation cap. Each continuation retains original root/path, compares the first returned FileVersion, and validates exact byte progress/EOF before appending. Failure returns no partial assembled source. Nontext/over10MiB stops after initial classification. Source over1MiB has no prepared syntax states and uses the existing plain-language lexer at draw; over10MiB shows the existing localized too-large fallback.
- Editor fallback: a first editor read that discovers larger paged text retires editing and reacquires the complete readonly source with a new original owner ticket. Binary/oversize needs no second acquisition. An older reply cannot install into this acquisition. Complete Markdown uses the same existing bounded preview renderer and parser.
- Limits/tradeoff: first-page classification may be read twice when an editor becomes readonly. The existing32-owner ceiling is now paired with32MiB of logical readonly source payload. A completed read may evict only disposable parked sources to fit; it never evicts the visible source, modified draft or original pending/uncertain operation. If space cannot be admitted the new readonly acquisition shows the existing buffer-limit refusal. This does not bound total process heap, lexer/row copies, parsed preview documents or editable undo history; resource/capture validation remains owed. Legacy file.read hashes/version and content are acquired separately on the node; matching page versions do not prove an immutable snapshot or whole-file digest agreement. The stronger ADR0083 binary-page transfer/sink contract is separate and not claimed by this helper.
- Validation: source tests pin UTF-8 byte boundaries, original scope, changed version/refused continuation, malformed EOF/zero progress, finite page count, binary/oversize/empty behavior. GPUI source regressions pin complete readonly fallback and explicit oversize with no extra acquisition. Fast size/boundaries/tokens pass. New-source strict/tests/real UI observations remain owed.

## Document readiness producer before desktop finalization

The canonical14-line `view::desktop::quit::DocumentReadiness` is copied unchanged
from W15's immutable853893652 control packet; its producer's strict run remains
pending. No provisional E3/controller/App quit implementation is adopted.
C5 now uses one per-viewer readiness predicate for cache eviction and path
mutation admission. It checks both original owner write tickets and model write
obligations before text dirtiness: uncertain is Failed, any original pending
attempt is Pending, modified error/conflict is Failed, other modifications are
Dirty. Readonly/read-only acquisition without writes grants no save authority.

The all-retained aggregation adapter `lifecycle::check_documents` remains literal
cfg(test) staging until W15's checked control and real finalizer registration.
It includes active/retained resources and path mutations without another owner
map; Failed takes precedence over Pending and Dirty. Its GPUI regression keeps
an uncertain write protected after undo and after a clean different active file.
This is not ordinary-quit integration: registration, finalizing-phase control
read-only and queued mutator/new-write refusal still need the canonical live
consumer. Forced exit has no journal or recovery guarantee. New tests are not
executed yet; fast checks are bounded source checks only.


The readonly payload reservation regression uses small actual retained sources
and a budget-sized reservation without allocating a large fixture. It pins
parked-only eviction and refusal to evict a visible owner for a late response.
No editable draft is truncated or discarded to meet the readonly cache cap.

### Generic save compatibility and proven non-dispatch

The generic save helper retains its original missing-conflict representation;
only the source-specific helper reports confirmed deletion. Both share one
write/conflict-read core. The two original head cases for missing disk and a
Forbidden refusal are restored unchanged, with separate source-deletion and
post-publication Internal-error regressions added. A narrowly classified
Forbidden/Unauthorized response is a terminal pre-handler policy/auth rejection:
node service/files delegates to workspace/files, whose filesystem error mapper
workspace/error emits Internal for I/O, including after publication, rather
than these authentication codes. Such proven policy rejection and local request
id exhaustion settle known not sent. A G4 call refusal, disconnect, unexpected
reply or timeout remains sticky uncertain. A direct handler error has the
method-specific terminal recovery described below; an ordinary read or
reconnection never releases a no-terminal-evidence effect.

### Readonly line storage

Complete source no longer allocates a String per line. The same existing
virtual rows and lexer read `source::SourceText`: one shared Arc source and
one validated u32 byte start per line, bounded by the same native source limit.
Visible rows borrow their line; only drawn styled text and explicit selected
composer context are copied. LF/CRLF trimming and final-newline behavior are
compared to standard str::lines, including Unicode and standalone CR. This is
an index for the existing viewer, not another editor or renderer. Markdown
shares the original Arc and the readonly cache continues to count its logical
source bytes. Offsets/preview/control allocations remain outside that payload
cap; no total-heap or frame-time proof is claimed. New tests are source-only.

Selected readonly rows use the same compact index directly; even a selection
starting far into a document does not walk preceding rows, and end/reversed
ranges clamp to existing rows. Source fixtures pin overshooting/reversed input.

The framed write-retirement regression uses a valid normal content guard, so
its dispatch barrier still names the original write itself after explicit
overwrite gained a pre-read. It additionally pins that exact transmitted guard;
the original single-write/no-replay and retained original scope assertions stay.

### Narrow17 corrections and initial Markdown loading

Exacte49f40387 strict four-crate all-target run17 reached compilation and
reported five missing observation type paths, one unnecessary event qualification
and one test-helper item placement lint. C5 now imports its canonical public
panel_files/directory_metadata modules rather than assuming root reexports;
metadata/source types added since that older run follow the same canonical paths.
The event uses its existing workspace import, and the ready-future test helper
lives before test statements. No protocol export or allowance is invented.

Initial Markdown rendering waits for a completed read instead of parsing the
loading model's empty placeholder; an actually loaded empty document remains
valid. A GPUI source regression distinguishes those two states. Name/timestamp
ties currently use deterministic Unicode lowercase ordering rather than JS's
locale-aware localeCompare: the existing Rust ordering avoids a new independent
collation dependency; non-ASCII collation may differ and is not exact parity.
Fast source checks pass; latest-source compiler/tests remain pending.

## Visible redacted failures

C5's admitted transport deliberately strips raw remote diagnostics, which can
include filesystem paths or credentials. The legacy workspace word tables
previously rendered an empty Node.message as an empty sentence. Their existing
problem rendering is now shared in text/workspace/problems: nonempty legacy
node diagnostics and the original transport/older/unsent sentences stay the
same, while empty diagnostics select a complete localized sentence by closed
ErrorCode. No fallback tells the person to retry an uncertain effect. The
actual editor/owner decides recovery and retains the original obligation.
Two regressions cover every closed code, EN/KO nonempty output, hidden transport
details and preserved legacy nonempty copy. The existing En/Ko size records
shrink681→669 and659→648; no allowance is raised. This source is newer than
strict18's immutable0eb404312 head and has no compiler/test receipt yet.

## Package base7 / shared field identity merge

Merged the official d18bb794 line before the next native submission, retaining
its FieldId, Callbacks::Plain/Identified, identity constructors/tests, G2 assets
and unrelated owners. Four conflicts are resolved explicitly: README keeps both
independent sections; field/events.rs takes the head identity dispatch then
reapplies only captured Plain submit; field.rs takes the head constructor/API
shape and delegates its two existing factories to C5's construct child; the
size list combines the head additions with the smaller owned limits.

Plain submit stays one boxed host callback on the same event subscription.
Identified submit/change signatures and engine FieldId checks are unchanged.
The head's source-aware constructor and identity tests remain byte-identical;
plain new/with_change plus composer/form factories accept captures as before.
The old captured-submit regression remains registered, with no duplicate public
constructors, text engine or event listener. Private construction bodies alone
remain in construct.rs. field.rs shrinks its existing owned557 record to556
(the incoming head record was615), without raising any size allowance. W22's
composition, K's Windowed events and B2's submit-press additions remain separate
unready producer work and are not copied here. No merged-head compilation or
shared-control consumer approval is claimed; exact strict18 is older-source.

## One shared IME registry

Consumed only W22b's checked control source a67511993: the exact composition
module and actual CodeEditor marked-text regression match its SHA manifest.
That producer has four-crate all-target strict Clippy rc0 and same-source
App112/0/2 plus Kit168/0/0, including the real Hangul/focus/commit case. These
are producer receipts, not the current C5/FieldId union's checks or V4 approval.

The old field weak registry is moved once to field/composition. C5's actual
central line/multiline factories register there, including W4 identity callers;
CodeEditor::new adds one editor registration. TextField::focused_composing
still supplies the sole global-shortcut guard. No callback, subscription,
composer_hooks, windowed notification or text engine is duplicated. Current
boxed Plain submit and unchanged Identified dispatch remain intact. field.rs
shrinks556→508; no size allowance rises. The older whole field root/ratchet
patch was not copied. The exact new module/test blobs are preserved while
constructor callsites are adapted to this worktree's already split factory.

### Path mutation non-dispatch settlement

Selecting the actor's connection is not evidence that a request reached the
wire: local request-id exhaustion, unassigned protocol or an authentication
refusal can still precede filesystem work. Path mutation settlement now reuses
the save owner's same narrowly defined known-not-written predicate, releasing
those holds while retaining invoked connection failures as uncertain. It does
not discharge any other original obligation. A GPUI source regression pins
Unsent/Older/Forbidden release versus post-poll Gone retention, with the original
source key/text unchanged. This avoids both phantom holds and blind retries.

## Original write termination and explicit reconciliation

[Decision Log]
- Purpose and intent: preserve drafts without falsely reporting Ready while an
  original blocking write might still publish, and recover when that exact
  operation is known to have ended.
- Existing constraint: the direct node file.write handler awaits the workspace
  write, which awaits its blocking closure. A successful rename can precede a
  failing final version read. A node error is therefore not proof of no effect.
  Transport retirement can discard even a completed successful response.
- Alternatives: clear uncertainty after any read (unsafe: a blocked old write
  can publish afterward); block every error forever (safe but unnecessarily
  loses availability when the original direct handler has actually answered).
- Choice: keep one original attempt in EditorModel with Pending, Ended, or
  Uncertain. Its submitted text, original baseline, sequence and owner scope/
  ticket survive local edit/undo. Only the direct original file.write node
  response establishes Ended. This is not inferred for forwarded mutations,
  arbitrary remote tasks, local deadlines or connection refusals.
- Why: terminal response ordering comes from the existing direct handler, not
  from read/hash equality. Only after that answer does a separately ticketed
  fresh read reconcile the effect. A read started earlier cannot confirm it.
- Costs and effects: the original direct write has no client-side timeout, so
  a slow live call remains Saving and retains its admission slot. Disconnect,
  unexpected reply, and any G4 call refusal retain Uncertain; reconnect, equal
  content, elapsed time and undo cannot release it. Lost terminal evidence can
  block saves, close, eviction and ordinary quit for the rest of the app run.
  There is no forced-exit recovery journal or restart preservation claim.

Ended confirmation retains the original cause on failed acquisition and offers
an explicit Check saved file action. A whole text read matching the attempt
establishes the new baseline; later local edits remain Dirty without automatic
replay. A read matching the old baseline retains Error and does not autosave a
persistent rejection. Leaving the file cannot implicitly flush either recovered
case until a new edit or explicit Save; pre-error autosave timers are retired
before the confirmation read. Third content, confirmed deletion, or noneditable data
preserves the draft as the corresponding conflict. The original ticket is
released only when that exact subsequent read resolves the Ended state.
Unknown writes never enter this recovery path. Existing disk observers cannot
settle either state independently of those original tickets.

Source regressions cover stale reads, all three text outcomes, failed retry,
later edits, deletion/nontext, open-write read immunity, keyed owner correlation,
and a real framed direct Internal reply versus post-dispatch retirement. These
new cases have not yet executed. The prior strict18 run failed on two malformed
HashMap type arguments in the earlier metadata tree source; those signatures
now use the canonical FileModifiedMillis value, preserving its existing tests.
No strict or whole-package pass is claimed for this changed source.

### Pending C3 comparison-cache connection

The git owner plans one before/after mutation invalidation hook around the
existing Workspaces file writers, including partial failures. C5 adds no second
comparison cache or git watcher. Its finite single-file observation concerns
saved-file invalidation only. C3's RAII cache writer is not yet an admitted
producer and is a pending integration connection, not implemented parity here.

### Per-node document readiness for owned stop/restart

The existing lifecycle reducer now accepts an optional actual NodeId facts
filter: all-document quit and node-scoped readiness share that one traversal
and the same Viewer predicate. It includes parked buffers and original write
obligations independent of text dirtiness; pending path mutations participate
by their original scope. No node/connection ownership registry or alias lookup
is introduced. Shell::file_documents_for_node returns the existing canonical
DocumentReadiness, not authority to stop or claim a controller.

Both entry points remain explicitly test-staged until their actual desktop
callers are wired. Stop/restart must conservatively respect Pending/Failed;
a Dirty/Keep choice preserves local text and cannot release an unknown effect.
Readiness for one node must not be blocked by unrelated remote drafts, while
ordinary quit still aggregates every node. A synthetic GPUI owner regression
reuses one alias/path/root for two actual nodes and pins parked Dirty/Pending/
Uncertain isolation. It is source-only and does not prove service lifecycle,
controller admission or forced-exit recovery.
