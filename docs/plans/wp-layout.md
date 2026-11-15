# Workspace layout

Status: implementation in progress.

## Decision Log

- Purpose and intent: one typed pane contract for files, terminals, browser
  and agents, with client-local layout and lifecycle-owned close.
- Existing assets and constraints: extend native view/link/app, reuse kit
  tabs, menus, sheets and resize edges. The current code panel is tied to
  the selected session and cannot retain a recursive pane tree. The app
  currently has no durable workspace registry.
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
- `screens/workspace/workspace-deck-retention.ts:3-34,70-125`: warm web
  deck count/TTL; native keeps only active workspace mounted.
- `workspace/open-in-editor/planner.ts:59-104`: desktop targets require
  local execution and bridge; workspace-relative paths are validated.
- `screens/workspace/workspace-tab-menu.ts:181` and
  `use-workspace-tab-rename.tsx:32,129`: entity rename, bounded title and
  lifecycle-specific close.

## Structure and data flow

`view::layout` owns targets, manifests, recursive tree, placement,
reconciliation, close plans and compact sheets. The app area adapts live
selection, draws through feature renderers and routes lifecycle effects
to their owners. It never owns editor text, terminal streams or browser
engines. Persistence is typed and validated before installation; sheets
and commit diffs are ephemeral. Renderer/cursor are panel-owned state.

## Differences and dependencies

- GPUI desktop supports pane splits directly; Paseo React Native does not.
  Compact and single-pane capabilities remain distinct from desktop splits.
- Until the workspace registry reaches the app, session identity bridges
  the existing route. Sessions sharing cwd are never conflated.
- Targets without working renderers or launchers stay hidden. Contract
  support alone does not prove the associated feature works.
- Desktop editor requests never execute on a remote node.

## Resume point

Reference and existing assets read; isolated worktree/target prepared.
Next: typed contract/tests, share compilable API with panel consumers,
area integration, captures, one whole native gate, one package review.
