# Workspace files, editor and document panels

Status: implementation in progress; no package acceptance yet. App base `8d5513e4`
is merged; node companion includes Quality B `29eebb8b`.

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
