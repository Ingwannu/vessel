# Workspace panels and split layout

## Reference and asset analysis

Paseo 0.15.4, app sources: `workspace-tabs/model.ts:34`,
`workspace-tabs/identity.ts:1`, `panels/panel-manifest.ts:1`,
`components/split-container.tsx:146`, `components/split-drop-zone.tsx:76`,
`stores/workspace-layout-store.ts:230,997,2292`,
`screens/workspace/workspace-tab-menu.ts:181`,
`workspace-tabs/launcher/internal/catalog.ts:1`, `mobile-panels/model.ts:1`.
Survey: p1 sections 26–30, area 4 sections 21–26 (inventory numbering).
Layout is device-local, keyed by host and stable workspace identity, never cwd.

[Decision Log]
- Purpose and intent: one typed frame for every workspace panel and lifecycle.
- Existing implementation and constraints: kit tabs, menu, popup and resize are reusable;
  the session and code panel currently have a single global selection. No split-tree model exists.
- Alternatives considered: a third-party docking engine, extending the code-panel switch,
  or a pure Rust tree with existing kit controls. No new dependency is required.
- Choice: modify existing controls and add `view::layout` pure state, `link::layout`
  persistence, and `app::areas::panes` rendering/registry.
- Why: pane rules remain independently testable and panel implementations stay with their owners.
- Benefits and costs: no duplicated panel managers or node RPCs; existing panel renderers need adapters.

## Panel API, integration contract (published first)

`vessel_app_view::layout` is the shared API. `WorkspaceKey` contains a `NodeId`
and stable workspace identity (the node's `WorkspaceId` after app-base merge).
`TabId`, `PaneId`, `GroupId` are local newtypes. `Target` is an exhaustive enum;
consumers use `Target::Terminal(TerminalId)`, `Target::Browser(BrowserId)`,
`Target::Files`, `Target::File { path: RelPath, location: Option<FileLocation> }`,
`Target::Changes`, `Target::WorkingDiff`, `Target::CommitDiff(CommitId)`,
`Target::PullRequest`; the workspace key supplies host/workspace context.
BrowserId is the browser owner's node type; until available it is a validated
local `PanelResourceId`, converted once by the browser adapter.

A `PanelManifest` defines kind, supported `Host` (Main/Explorer), singleton,
close button, compact-sheet policy and persistence. `PanelDescriptor` contains
label/subtitle/tooltip/title loading/status. Consumers must not write layout trees.

App registration lives in `areas::panes::registry`: one typed renderer function
per `PanelKind`, called with `PanelContext { workspace, pane, tab, target, focus }`,
window, tokens and Shell context. Renderer returns `AnyElement` and may dispatch
`PanelAction` through `Shell::panel_action`. Add registration lines only;
feature state, subscriptions and node calls stay in feature modules.

Open is `OpenRequest { target, intent: New/Reveal/Background, placement:
Focused/Pane/Prefer/Main/Side, parent: Option<TabId> }`; renderer actions include
open, close, retarget and state updates. New entity creation is an owner action,
not a layout RPC. Tabs deduplicate by normalized target, launchers have unique ids.

Close contract: owner publishes `CloseGuard::Ready` or `Modified` for each tab;
layout returns `ClosePlan` (passive, archive root agent, hide child agent,
kill terminal, confirm modified). A modified panel remains present until the
owner returns `CloseResolution::Discard` or `Saved`; `Cancel` retains it.
A terminal or root-agent confirmation never runs file discard implicitly.
Batch and pane close use the same plan and only remove acknowledged tabs.
File save/discard UI and asynchronous save failure remain owned by Files.

## Behaviour and bounds

Four user split levels; tree depth five including Explorer bookkeeping.
Split fraction minimum 0.1; desktop drag activation eight points; middle drop
zone 40%, edge 15%. Explorer persists while hidden. Last ordinary pane cannot
close. Files and Changes singleton in Explorer; setup/browser main only.
Compact files and child sessions use transient sheets; root sessions, terminals
and browsers stay tabs. Promotion retains identity/state. Commit diff and sheets
are excluded from persistence. Hydration gates entity pruning.

## Resume point

Done: reference/asset analysis, worktree, panel integration contract.
Next: compile skeleton, model transitions/tests, persistence and frame; merge app base
when announced. No gate, capture or review has run yet. No parity claim.
