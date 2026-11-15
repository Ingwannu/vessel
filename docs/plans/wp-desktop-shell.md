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


## Additional decisions from integration

- Multiple windows revealed a stale whole-document preference overwrite. One
  process-wide coordinator now applies typed three-way deltas to the latest
  snapshot (including map-key removal and non-settings fields), writes through
  the existing private Prefs helper, and broadcasts canonical snapshots. Failed
  writes retain canonical state; bounded-queue refusals use the existing unsent
  UI path. New persisted fields demand an explicit merge policy.
- R's updater is owned by an App Global behind Rc, so opening/closing the first
  window cannot start another downloader or stop updates for remaining windows.
- The updater already marks relaunches in the child's environment. Such starts
  wait for the kernel file lock with a finite startup deadline instead of
  forwarding into the quitting app. Normal launches still forward. This avoids
  a restart race without changing the updater or temporarily releasing the old
  instance's lock. A timeout displays an error-only window with no node/writer.
- ADR 0094 supersedes the initial sibling CLI naming: both companions use the
  app executable directory plus `bin/vessel[.exe]` and `bin/vesseld[.exe]`. One
  typed bundle getter serves CLI passthrough, installation and managed-node
  startup. It only computes a location; each consumer checks presence. This
  avoids the Windows case-insensitive app/CLI name collision without guessing
  a macOS signing layout. macOS returns a typed unsupported-layout refusal
  until a bundle is published; AppImage has no companions. Installation stays
  visibly unavailable when a release lacks the CLI.
- The release owner's initial NSIS check found a command-macro quoting error;
  the two registry-command uses now preserve the macro as one quoted argument.
  Packaging runtime evidence remains separate from app behavior verification.

## Package-base 2 and narrow feedback

Merged 6036eeb8 without changing other owners' feature drafts. Link narrow
checks exposed strict lint failures in the new desktop modules; the fixes use
explicit returned-value handling, borrowed worker state and documented public
fields. The public error type is `DesktopError` to identify its boundary.
Bundle-path tests cover both tools on Linux/Windows, relative-path refusal and
unassigned macOS layout; no test executes a companion or assumes it exists.

## Window actions and process update owner

Zoom rules now live in `view::desktop::zoom`, preserving the half-level changes
from `packages/desktop/src/features/menu.ts:119-143`. The screen applies a
bounded multiplier to its own native scale; pure tests cover inverse steps,
reset and both saturation endpoints. Native startup installs the shared update
owner before opening any window; each window follows a fresh watch receiver.
Focus-triggered checks are silent and honor automatic-update preferences. The
existing signed updater and its timer stay unchanged. W15 still owns managed-node
finalization and the canonical preferences flush before actual restart.

A deterministic kernel-lock test waits for the failed-acquisition signal before
releasing the old primary lock; no scheduling sleep is used. An oversized-link
regression tests the named byte boundary before route/identifier parsing.
These additions await the current narrow compile/test runs.

## Authenticated navigation integration and context menus

Merged W21's narrow-green 963ca595 provisionally (whole package review pending),
keeping this package's bounded shared URL parser when resolving add/add history.
Desktop startup registers `Shell::open_session_address`; after book hydration it
passes the latest target once. W21 owns the single connection/reconnect deadline,
identity resolution and passive session reads. Desktop adds no second host wait.

The existing TextField input engine already supplies selection-aware editing
context menus. Terminal selection and clipboard actions remain C2's; a typed
`CopyState` and `areas::desktop::open_edit_context` adapt its admission to the
existing application-menu state and kit renderer. The popup restores the original
focus before dispatching toolkit actions. Its real-input test checks disabled
Copy navigation and copy/paste behavior; C2's actual terminal consumer is pending.
G4's reviewed return-focus primitive is awaited for dismissal restoration.

Link narrow tests at 88fc1df9: 16 passed, one fixture failed before IPC startup
because the remote runner's TMPDIR made its Unix socket address exceed SUN_LEN.
The lock-waiter test itself passed. The IPC fixture now uses a short unique Unix
/tmp directory, preserving its real forward/restart assertions and exact cleanup.
Product Unix IPC still reports the original OS error for an overlong configured
socket location; this is an OS pathname limit, not silent forwarding or fallback.

## Deferred CLI setup consumer

Onboarding and Settings share `areas::desktop::cli::snapshot` and
`set_installed(Operation, cx)`, both returning background tasks with typed
`view::desktop::cli::Snapshot` acknowledgements. The read-only query checks the
bundled file, exact shim ownership and current PATH/known shell rc; it never
creates or edits the rc. Only an explicit install calls `PathSetup::ensure`.
An installed CLI with an uneditable/unknown rc returns the manual PATH hint,
while a foreign destination remains a typed refusal. Removal preserves the rc
and reports its remaining configuration honestly. The existing ownership writer
is reused rather than adding onboarding-specific filesystem policy.

## Shared menu policy and close admission

The native OS menu bar and kit popup now use one row vocabulary. Active-window
locale/shortcut-recording changes rebuild native labels and zoom admission, so
capturing a shortcut cannot be consumed by a stale native zoom item. Terminal
context admission has independent typed Copy/Paste states; read-only script or
service terminals keep Copy/Select All and disable Paste. Help destinations use
`Shell::run_desktop_help(HelpAction, window, cx)` for About/version, the changelog
sheet and the public issue form. Sidebar consumers call the same handler.

The existing chrome renderer gains an optional close callback; without a host
callback its default behavior remains ordinary window removal. Desktop menus,
custom chrome and the OS should-close hook share the host lifecycle boundary.
On Linux/Windows the last window stays visible while W15 finalizes owned work;
additional windows and macOS windows can close normally. The later app-quit
observer remains a backstop. Signal observations belong to W15's shared reactor.

Local app narrow 1 and remote app narrow 5 reached only the two unbound
cancellation acknowledgements in link; fixed by standalone 8a203181. The second
local app check was still queued (its cwd/command and waiting child were checked)
and was cancelled in favor of the warm remote target. No app-wide green claim.

## Reopen memory and close regression

The remembered first-window state is updated in application memory alongside the
serialized write, so a macOS dock reopen during the same process restores the
latest geometry. A real GPUI-window regression exercises the installed chrome
close callback: a pending/refused finalizer retains the last non-macOS window,
while another window closes without requesting process quit. This test awaits
compilation/execution; OS-native close callbacks still require platform smoke.

Merged package base 3 (6c782cb6); only announced base changes touch root crates.
This package adds no node method and has no independently changed root sources.
