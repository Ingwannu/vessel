# Native desktop shell parity

## Reference and scope

Paseo 0.15.4 sources read before implementation:

- `packages/desktop/src/settings/window-state.ts:8-166` and
  `window/window-manager.ts:15-109`: first-window restoration, reachable work
  areas, default 1200×800, minimum 400×300, deferred and final saves.
- `features/menu.ts:80-249`: File/Edit/View/Window actions, shortcut-capture
  suppression of zoom, terminal Copy/Paste/Select All.
- `agent-navigation.ts:1-44`, `window/desktop-window-owner.ts:42-125`,
  `main.ts:850-910`: single instance, latest pending destination per window,
  foreground-window delivery and bootstrap ordering.
- `login-shell-env.ts:154-506`: bounded login-shell resolution before startup,
  interactive/noninteractive attempts, Windows retaining inherited environment.
- `integrations/cli-install/install.ts:40-107`, `shell-rc.ts:25-118`:
  user-local CLI install and removal of only the application's own shim.
- `packages/app/src/changelog/internal/parse-changelog.ts:1-180` and
  `use-revealed-releases.ts:16-48`: fenced Markdown, release/section structure,
  immediate latest release, first five and ten more.

This package owns windows, menus, OS links, single instance, CLI installation,
environment inheritance, update presentation and changelog. Local node,
onboarding and quit lifecycle have a separate owner. Session navigation uses
the session area's authenticated NodeId resolution, never a book alias.

## Existing asset analysis and decision

**Modify existing assets.** GPUI supplies bounds, URL callbacks, menus, actions
and quit observers. Reuse the kit and Fluent catalogs. `app/src/update.rs`
(the assigned `areas/update.rs` does not exist) and the signed updater remain.
Reuse `view::desktop::NavigationTarget`; standard-library APIs avoid another
desktop framework.

## Decision log

- Purpose: launches preserve window reachability and route credential-free
  links into the authenticated session flow.
- Existing constraints: UI and I/O live in distinct crates; signed updates
  already work; current releases bundle the app alone.
- Alternatives: Electron/Tauri, replacing the updater, a network launch server,
  per-window snapshots of process-wide state.
- Choice: extend GPUI with pure rules, private I/O and small entry modules.
- Why: one renderer/update trust boundary, with explicit process ownership.
- Effects: OS behavior requires separate platform evidence. No tray is added
  (Paseo has none), and no unowned daemon is stopped.

## Deliberate differences

- Links use `vessel://h/<NodeId>/session/<SessionId>` and validated protocol IDs.
- Native Rust APIs replace Electron preload IPC. Browser-pane reload belongs
  to the browser package; a native renderer has no browser DevTools.
- Keep the existing signed preview feed. No stable/beta selector until real
  signed channel feeds exist.
- No published changelog endpoint exists. Show bundled Vessel Markdown and
  signed-feed notes; never fetch Paseo releases.
- CLI installation refuses unrelated existing files. Missing bundled CLI is a
  typed refusal, with no invented download or global package installation.

## Resume point

Reference and asset audit complete; shared link contract imported. Next: pure
models, I/O, app registration, narrow checks, captures, full native gate, review.
No OS behavior or package-completion gate is claimed yet.


## Work in progress checkpoint

The native models passed nine focused desktop tests (no failures). A later view
clippy attempt correctly refused missing per-key translator comments; those
comments have been added. No complete package gate or capture evidence exists.

Implemented drafts: bounded session links, geometry restoration, Markdown release
parsing, private serialized window saves, OS-lock/local IPC single instance,
CLI ownership checks, login environment capture through the existing process
crate, typed centralized preference merge/writer, GPUI windows and menu surfaces.
These drafts are not yet consumer-ready. Native menu registration, preference
queue refusal/rollback, update-owner sharing, CLI UI/PATH handling and platform
integration still need completion and compilation. Session/project dispatch waits
for the owning areas' authenticated adapters; quit registration and pre-update
finalization are owned by the local-node package.

The new `vessel-proc` dependency reuses an existing internal process primitive,
including kill-on-drop groups/jobs. No new external library is selected. The
native dependency table records this edge.


## Package-base merge

Merged the announced package base. In `link/src/lib.rs` the base extraction and
registrations win byte for byte; the desktop module is registered afterward.
In `areas/about.rs` the base's Fluent migration is retained, including the old
release-note renderer in the merge. The desktop changelog renderer remains the
registered entry, and the superseded renderer will be removed in the following
implementation commit. No base test expectation was edited. Root changes in this
merge are inherited from the announced base, not desktop wire additions.
