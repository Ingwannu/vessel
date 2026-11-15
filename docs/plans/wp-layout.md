# Workspace layout

Status: implementation in progress.

## Decision Log

- Purpose and intent: one typed pane contract for files, terminals, browser
  and agents, with client-local layout and lifecycle-owned close.
- Existing assets and constraints: extend native view/link/app, reuse kit
  tabs, menus, sheets and resize edges. The current code panel is tied to
  the selected session and cannot retain a recursive pane tree. The durable workspace registry is supplied by its separate owner; this package
  consumes only its reviewed identity values and a typed lookup boundary.
- Alternatives: per-feature docking duplicates focus and persistence;
  adopting a docking dependency introduces another framework and cost.
- Choice: modify existing app. Pure state in `view::layout`, narrow area
  adapter in `app::areas::layout`, typed persistence through preferences.
  Standard collections and existing serde/thiserror need no new dependency.
- Why: feature owners retain buffers and resource lifecycles; layout owns
  target identity, placement, focus and order. Passive opening sends no RPC.
- Benefits, costs and effects: shared pane seam with deterministic tests;
  session routes need an explicit bridge until durable workspace identity
  reaches the app. Layout identity never uses cwd.

## Reference facts: Paseo 0.15.4

Paths are relative to Paseo `packages/app/src/`.

- `workspace-tabs/model.ts:34-70`, `identity.ts:139-198,239-280`: typed
  targets, unique launcher instances and host/workspace persistence keys.
- `workspace-tabs/launcher/internal/catalog.ts:1-28`: primary order agent,
  terminal, changes, diff, files, browser, PR; supporting starts changes.
- `panels/panel-manifest.ts:58-198`: host constraints, Explorer singletons,
  compact file/subagent sheets; unknown agents stay tabs
  (`workspace-tabs/compact-sheet.ts:20-35`).
- `stores/workspace-layout-store.ts:232` and
  `stores/workspace-layout-constants.ts:1`: depth bound and minimum ratio.
- `components/split-container.tsx:146-230`: recursive groups, pane focus,
  Explorer outside split rendering and maximize projection.
- `constants/layout.ts:37-56` and `workspace-tabs/layout-mode-publisher.tsx:5-18`:
  workspace compact mode is below 600 points; wide touch remains single-pane.
- `screens/workspace/workspace-deck-retention.ts:3-34,70-125`: warm web
  deck count/TTL; native keeps only active workspace mounted.
- `workspace/open-in-editor/planner.ts:59-104`: desktop targets require
  local execution and bridge; workspace-relative paths are validated.
- `screens/workspace/workspace-tab-menu.ts:181` and
  `use-workspace-tab-rename.tsx:32,129`: entity rename, bounded title and
  lifecycle-specific close.

## Structure and data flow

Palette actions capture a `LayoutActionContext` before presentation. The layout
owner compares the workspace, focused pane, selected tab/resource and top compact
sheet before dispatch through the existing action handler. This prevents a menu
opened in one workspace from acting on a later selection. Background reordering
does not invalidate it. Refusal stays handled, including Close, so desktop window
fallback cannot discard the guarded window. This is a UI-selection fence only;
node effects still require the admitted resident and connection lease. Reusing
the model projection avoids another focus manager or keyboard listener.
Its `target()` / `selected()` queries give the top compact sheet precedence over the underlying
pane. Composer destination capture must match a current explicit Draft/Agent
target and workspace; a hidden draft is never a global fallback.

Feature polling uses read-only `layout_tab_exists` and `layout_panel_visible`.
They never insert a layout, and visibility requires the displayed workspace,
selected tab and compact/maximized/top-sheet projection. The window argument is
required to apply the current breakpoint; OS window visibility remains with the
caller. Typed Explorer rendering is not installed yet and is not reported visible
through this adapter. Sidebar actions use idempotent `layout_exit_focus`, rather
than toggling into focus mode on an already ordinary layout.
Drawing and visibility queries share the effective pane projection. A stale
maximized pane id, including after closing an empty pane, falls back to the
surviving tree; it cannot suppress every resource renderer or poller.
Workspace renderers, placement and command availability use the same public
`areas::layout::mode(window)` and pure `LayoutMode::for_width` policy. This uses
Paseo's 600-point workspace boundary, rather than inheriting the legacy shell's
768-point sidebar breakpoint. Keeping that distinction explicit prevents the
command palette advertising a split which its owner refuses at 600–767 points.

The staged preferences hook rebases an initialized layout cache against old/new
canonical preferences by workspace key. It installs unrelated additions/removals
and keeps each pane tree atomic. Locally changed durable trees, dirty tabs,
compact sheets and active rename/menu/drag/resize interactions retain their
current tree; metadata refresh never discards their owner resources. It does not
save preferences. The desktop coordinator must remove its literal test-only
staging attributes with the real canonical-snapshot caller and check that union.
This avoids the former stale whole-book write removing another window's layout,
without introducing another preferences coordinator.

`view::layout` owns targets, manifests, recursive tree, placement,
reconciliation, close plans and compact sheets. The app area adapts live
selection, draws through feature renderers and routes lifecycle effects
to their owners. The pane model never owns editor text, terminal streams or browser
engines. The separately reviewed resident contract assigns the sole native
session-resource registry to this package; concrete timeline/composer resources
remain owner-defined behind scoped access. Persistence is typed and validated before installation; sheets
and commit diffs are ephemeral. Renderer/cursor are panel-owned state.

## Differences and dependencies

- GPUI desktop supports pane splits directly; Paseo React Native does not.
  Compact and single-pane capabilities remain distinct from desktop splits.
- Until the workspace registry reaches the app, session identity bridges
  the existing route. Sessions sharing cwd are never conflated.
- Targets without working renderers or launchers stay hidden. Contract
  support alone does not prove the associated feature works.
- Opening an editor/file manager belongs to the workspace owner; this layout
  package supplies its target/context and does not execute those requests.
- Plugin targets use the canonical protocol contribution address and context.
  Restored missing/stale entries retain their slot and modified state; opening,
  retargeting and dragging require the plugin owner's live placement resolver.
  Until that separate resolver is available those operations refuse explicitly.
- Persisted agent roles are reset to Unknown; a saved root flag cannot authorize
  archive. Lifecycle always obtains fresh authoritative parentage.
- Rename captures expected NodeId and checks the actual client before effects;
  retries allocate fresh tickets. Legacy alias-only rename stays hidden until its
  lifecycle owner supplies the original admitted binding, never latest Fleet data.
- Host forget drops clean legacy presentation; modified records keep their exact
  key/tab/target for recovery. Retained display metadata cannot authorize I/O and
  always requires the original node binding before any future operation.
- Device format one is a typed Rust format, not an importer for Paseo JSON.
  Split fractions reject NaN/infinity and are clamped to the shared minimum.
  Restored group IDs are unique and strictly below the shared split allocator,
  preventing ambiguous resize targets or future allocation collisions.

## Producer contract

`view::layout::Target` and `LayoutKey` are the only panel identities.
`LayoutKey::Workspace { node, workspace }` uses authenticated NodeId and the
canonical protocol WorkspaceId. Legacy Session keys are explicit alias/session
bridges and never infer ownership from filesystem paths.

`areas::layout::registry::PanelContext` carries `&LayoutKey`, PaneId, TabId,
`&Target` and focus. Feature renderers register in its one exhaustive dispatch;
resource-owned title/status/cursor/renderer are not identity. File identity is
WorkspaceRoot plus RelPath; commit identity is CommitId with file focus retained
by the git owner. DraftId is client-local and never converts into a node SessionId.

The app seam is `layout_open_target(&LayoutKey, Target, OpenIntent, Placement,
Window, Context) -> Result<TabId, LayoutError>`. Async callers capture the key,
never the currently selected route. `update_layout(&LayoutKey, change, Context)`
permits only the pure model's validated operations, including retargeting passive
clean drafts and publishing modified state. `layout_close_settled(&LayoutKey,
&ClosePlan, Result<(), LayoutError>, Context)` applies a lifecycle result to its
original layout and refuses changed/dirty targets. Unknown agent role requests
ResolveAgent; it never silently falls back to root archive.

`run_layout_action(LayoutAction, Window, Context) -> bool` is routed exclusively
through the shortcut owner's dispatcher. Layout installs no keyboard listener.

## Current evidence and remaining cutover

The pure pane model, native frame, typed persistence and correlated rename adapter
are implemented. Strict all-target view/link/app Clippy passed on the shared
producer source. The source review's split-group identity correction and the
modified-record host-forget regression are receiving targeted narrow tests.
This is not a whole-package gate or capture claim.

The resident contract in `wp-layout-residents.md` passed contract-only review.
Pure admission/focus/transport-fence types and tests are in `view::residents`;
SessionView/ConversationArea/composer globals have not yet been cut over. No
second live registry is enabled. The actual shared connection lease, concrete
resource snapshots, keyed renderers, follow handles and keyboard/composer adapters
must land together before multi-session panes can be claimed. Existing frame can
consume only the current legacy agent screen once; other unregistered targets
retain a localized unavailable placeholder and are omitted from launchers.

Remaining package evidence: narrow tests, feature-owner registrations, single-owner
resident cutover, compact picker/explorer integration, isolated fixture captures,
one final whole native gate and package source review. No live node or agent is
used by these checks.
