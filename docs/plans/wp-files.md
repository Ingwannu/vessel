# Workspace files, editor and document panels

Status: proposed. Base: `1d1f11df`; node companion starts at `d954fa3a`.

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
  current main 0.9 use `Apache-2.0 OR MIT`. The locked resvg 0.46 licence
  must separately be checked before reusing it (licence changes are version
  specific). Do not claim current-main licensing for an older release.
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
