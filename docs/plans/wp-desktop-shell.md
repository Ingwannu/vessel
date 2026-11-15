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

This package owns windows, menus, OS links, CLI installation, single instance,
login environment inheritance, shared updater ownership and release history.
Onboarding, managed-node lifecycle, quit/update finalization and document guards
belong to their feature owners. Session navigation and project registration use
their authenticated owner adapters; desktop never substitutes a book alias.

## Current status

Implementation is source under integration, not a completed parity claim. The
latest application base is package line dd13ce37 (base 6, connection preflight,
the safe Windows environment broadcast, reviewed G2 menu profiles and corrected
Cargo.lock). No node head was merged directly into this application tree.
There is no final whole native gate, inspected capture set or V4 package verdict.
Shared producer review and narrow checks do not substitute for those proofs.

## Existing assets and decisions

**Modify existing assets.** GPUI provides native windows, URL/reopen callbacks,
menus and quit observers. The kit supplies the one menu/dialog/snap renderer and
Fluent owns user-facing text. Keep the release owner's signed updater unchanged.
The assigned areas/update.rs path does not exist; app/src/update.rs is canonical.

[Decision Log]
- Purpose: preserve reachable windows and credential-free navigation across native launches.
- Constraints: view stays pure, link owns local I/O, the app owns UI callbacks, and existing signed-update trust must remain intact.
- Alternatives: another desktop framework, a TCP launch server, independent per-window stores/updaters, or native unsafe calls.
- Choice: extend existing controls and OS/process primitives with typed host adapters.
- Reason: one renderer, one persistence path and one process updater avoid divergent behavior and races.
- Consequences: native OS behavior needs platform evidence; missing platform APIs remain explicit. No tray is added and no unowned daemon is stopped.

## Module responsibilities and data flow

| Module | Responsibility |
| --- | --- |
| view::desktop | Strict navigation, finite geometry, zoom rules, CLI states and structural Markdown/reveal rules. |
| link::desktop::instance | Private kernel locks, bounded local IPC and per-window latest intents. |
| link::desktop::store/window_writer | Versioned bounded geometry file and serialized debounce/final writes. |
| link::desktop::environment | Bounded shell capture into child-only CommandSpec overrides. |
| link::desktop::cli/bundle | Explicit companion locations, exact install ownership and user PATH policy. |
| link::desktop::preferences | One canonical actor, typed structural deltas, revision-aware optimistic rebase and exact write receipts. |
| app::desktop | Process globals, window registry, startup, callback admission and native preference broadcasts. |
| app::areas::desktop | Shared popup/native menu vocabulary, keyboard adapter, help routing and CLI controls. |
| app::areas::about::changelog | Bundled/signed-note history through the existing configured dialog. |
| app::update | App Global Rc owner of the existing updater; windows follow independent watch receivers. |

A GUI argv or OS URL is parsed before renderer startup. Claiming a primary binds
local IPC before releasing the short bootstrap lock; secondaries forward a
validated bounded intent and wait for acknowledgment. Each window keeps one
latest session and one latest directory independently. Book hydration plus an
installed owner handler permits consumption; a missing project handler cannot
erase or delay an independently received session link.

Settings and nonsettings mutations still use the existing SavePrefs command.
Desktop intercepts it before the per-window link actor, derives a delta against
that client's acknowledged/optimistic baseline, applies it to the latest canonical
snapshot, and uses the existing Prefs::save_at atomic private-file helper. Every
window observes the resulting canonical revision with pending local edits rebased.

W12's checked cache hook from the 9e18cf0d..30a4396f range is now called by the
canonical observer. Capture the previous Client canonical snapshot before observe,
then pass old/new canonical layout records to the owner before replacing projected
Shell preferences. The hook retains locally modified, dirty, compact-sheet and
owner-held trees; independent additions/removals update the cache. Only the two
agreed cfg(test) staging attributes were removed with this real caller. A product
apply-path regression checks that another window's new workspace survives the
next local layout save; its combined run is still pending.

## Windows and startup

The primary restores validated geometry (1200×800 default,400×300 minimum),
clamped to visible work areas. Additional windows start independently. Geometry
writes debounce 400 ms, serialize, and flush the latest state at exit; malformed or
newer documents are preserved. macOS reopen uses the latest in-memory geometry.
Capture-only size overrides do not relax normal product minimums.

The active live window is preferred, then a visible window, then the first live
window. Closed handles and expired Shell entities cannot be routing destinations.
Visibility is read from GPUI, without a second cached window registry.

Single instance uses private local IPC, never TCP. Packets are bounded 4096 bytes,
queue 32, acknowledgment deadline 3 s. A kernel bootstrap gate prevents a secondary
connecting before IPC exists; the long login-shell/GUI initialization follows
that gate. Windows publishes its next pipe instance before acknowledging the
previous connection. Updater relaunch waits up to 15 s for the old process lock and
never forwards back into the quitting process. Timeout opens a clear error-only
window. No early primary-lock release is needed.

The updater's app-only relaunch marker is removed from owned child environments
and CLI passthrough using its public constant, without global environment mutation.
Initialization retains a failure exit code until the kit, canonical desktop state
and first window start; a failed first window exits Explicit mode rather than
holding an invisible primary lock.

## Menus, shortcuts and close admission

Native menus and the popup share typed rows. Copy/Paste/SelectAll dispatch the
kit's existing actions to the selection owner. EditContext admits Copy and Paste
independently, so read-only script/service terminals can disable Paste. Desktop
contains no second terminal parser, clipboard engine or input policy.

G2's isolated8158be6d producer was adopted via103a042ac, retaining newer dialog
configuration. MenuState is the sole weak return-focus owner. A chosen row
restores before dispatch; a replaced app-menu cycle refuses old row actions.
The separate strong saved opener was removed. Real-field action tests remain
part of the pending combined evidence.

Source audit found a distinct stale-dismissal gap: handlers rendered for cycle A
can close or restore the newer cycle B before the app's chosen-action guard runs.
G2 confirmed the shared producer has no per-open render lease. The coordinator
assigned S2, sole menu writer, an opaque per-open token checked before pointer,
key, highlight, outside, close and focus mutations, with retained-render A-to-B
regressions. V1 reviews and the lead lands that shared fix before consumption.
The app action guard is retained but is not claimed to protect dismissal; this
package does not edit a second menu renderer or work around it by reopening.

Shell::run_desktop_shortcut is shared by menu and keyboard consumers. Pure
half-level zoom rules remain modeled, but production Zoom is unsupported and
hidden from menus under the coordinator ruling: set_scale_factor is test-only,
and set_rem_size cannot scale the kit’s px geometry. The lead owns a future
per-window viewport zoom seam covering layout, paint, hit testing and native DPI.
W22 uses desktop_shortcut_supported to avoid advertising these actions and owns the sole
matcher/default table; CloseTab falls back to CloseWindow only after layout is
Unhandled, and opening/refusing a close confirmation counts as handled.

On non-macOS, last-window removal goes through the quit owner before removal.
Additional/macOS removal has an optional document-owner callback. It defers until
the initiating Shell borrow ends, coalesces Idle/Checking, retains a refused
window and rechecks last-window status before actual removal. It owns no second
save/discard or quit policy. W15 owns registration and finalization integration.

## Environment and CLI

Shell capture uses vessel-proc's existing groups/jobs and cancellation behavior,
with one 30 s overall deadline, interactive then noninteractive attempts and 1 MiB
output bounds. Values stay redacted and apply only to explicit child commands;
XDG_RUNTIME_DIR is not inherited from a stale login profile. Windows/failure uses
the inherited environment. Hermetic process tests use a checked-in /bin/sh fixture,
cleared environment and nonexistent HOME, never a user's rc files or real agent.

Companions are executable-parent/bin/vessel and bin/vesseld (with .exe on Windows).
Missing files are normal and do not create a download/install offer. macOS has
no published signed bundle layout and returns UnsupportedBundle instead of
assuming executable code under Resources. AppImage has no companion assumption.

POSIX install owns only its exact symlink and never overwrites a foreign tool.
Known shell rc edits are bounded/idempotent, preserve existing bytes/mode, create
new files privately and otherwise expose manual PATH guidance. Onboarding and
Settings share snapshot/set_installed; only explicit Continue applies a choice.

[Decision Log: Windows user PATH]
- Purpose: support Korean and other Unicode install paths.
- Constraint: UTF-8 .cmd text is interpreted through an OEM code page and can corrupt the path; native crates forbid unsafe Win32 calls.
- Alternatives: OEM batch conversion or script/PInvoke bypass.
- Choice: Windows-only winreg 0.56 (already locked), raw UTF-16 HKCU Environment Path and transactional updates.
- Reason: one exact directory entry can be added/removed without expanding or replacing unrelated entries. Transaction conflict/failure cannot fall back to an unsafe plain overwrite.
- Consequences: REG_EXPAND_SZ is retained, malformed/interior-NUL/oversized values are refused, optional/repeated trailing NULs are accepted and written back once. A REG_SZ containing percent expressions and a new literal-percent directory are refused to preserve their meaning.

Entry comparison folds single Unicode uppercase scalars without expanding
mappings; unpaired UTF-16 units survive. Extended drive/UNC prefixes use ordinary
installer spelling; 8.3 alias equivalence is not resolved. A trailing separator is
reused. Pure tests never touch the real registry. Disabled transactions have a
specific translated refusal. The release installer owns corresponding cleanup.

The safe GPUI Windows WM_SETTINGCHANGE producer is now on base 6. Only the
Windows binary entry names its platform crate. Installation and rc setup return
Changed/Unchanged, so only a real environment write schedules notification.
The completed Snapshot returns immediately; a detached background task runs the
host callback without holding the panel or onboarding continuation. Its
named5s timeout is per nonresponding receiver. Failure preserves the completed
installation and Configured fresh-terminal/login guidance rather than pretending
the registry write rolled back. No native unsafe or script bypass is added.
The release owner's739db8f2 follow-up clears stale last-error and represents a
zero-return/success-code result as Unconfirmed, mapped honestly to Other at the
binary bridge. Real Windows execution remains unverified; Wine packaging checks
are separate.

## Canonical preferences and feature caches

Exhaustive Prefs/Preferences/nested patterns force new fields to select a merge
policy. Per-key removal is real; independent machines, providers and default
records survive stale windows. Provider option bags remain atomic under their
existing bounds. Queue 64 refusal is visible, unreadable files remain launch-only,
and old failures cannot roll back newer accepted edits.

Each workspace layout merges by LayoutKey; its validated pane tree stays atomic
so allocator/reference invariants cannot be corrupted. W12 owns the UI-book cache
rebase, including Dirty, sheets, rename, menus, drag and resize preservation. The
canonical map alone does not prove this cache integration complete.

Client::canonical_preferences separates observed canonical context from pending
projection. The cache hook receives old canonical before observe and new canonical
from Snapshot, before shell.prefs is replaced. Shortcut, prepared-word and terminal
cache invalidation remain feature-owned callbacks on canonical/rebased changes.

Optional Receipt::completed retains the exact per-write Snapshot/Outcome even
when watch updates coalesce. prefer_with_receipt wraps the existing prefer once;
it neither duplicates validation nor emits another SavePrefs. Validation refusal
cannot return a stale receipt. Queue refusal remains typed. Feature UI generation
checks reject stale acknowledgments; shared notices own generic failures, while
Saved and LaunchOnly remain distinct. The existing atomic writer is unchanged.

## Updates and changelog

The app starts exactly one existing updater before its first window. Each window
uses a fresh watch receiver; closing the first window cannot stop updates. Focus
checks respect automatic-update preference. Keep the signed preview feed, key,
signature and no-feed build behavior. There is no invented stable/beta selector.
W15 owns canonical pre-update flush and owned-node/document finalization; its
checked consumer is required before claiming restart lifecycle completion.

Changelog uses the shared dialog and snap engine, not another renderer:
packages/app/src/changelog/internal/changelog-sheet.ts:80-90 specifies 620 width,
exact desktop 85%, and compact 85%/95% draggable stops. Legacy dialog 520/640 widths
and 85%/90% fitting caps remain unchanged. Captured open cycles protect close and
reveal callbacks; actual close alone acknowledges ReturnFocus and clears snap
state. Configured bounds and snap/scroll handoff have regression tests.

The complete available release-owner notes 0.4 through 0.20 are bundled newest first,
including historical known problems/unchecked flows and Korean-only early notes.
CHANGELOG.sources retains source hashes. Notes19/20 additionally match the supplied
signed-manifest hashes: 98e33a5ed2edc2031f48cbb44fbb84f220f20466f14513da6a5b126f731b24ae
and f5dae40d1c84944ef01e8da26894c84297e710c6987b1475f7de3b212af4ca9e.
Running signed-feed notes supplement that history. No published Vessel changelog
endpoint exists; never fetch Paseo releases. Reveal is 1 immediately, 5 after yield,
then 10 more; 17 real releases make the paging controls meaningful.

## Remaining integrations and deliberate differences

- W21's authenticated bounded session lookup is integrated provisionally. C2's retained Binding terminal cutover is still required; the old alias-only terminal path is not declared identity-safe.
- W11's managed-local project adapter is not consumer-ready. Its directory intent stays queued; no first-alias fallback, implicit trust or registration occurs. The callback now returns ProjectDelivery::Opened only after the exact editable draft opens, otherwise Pending retains the directory without overwriting a newer arrival. Pending is idempotent and does not emit repeated UI events.
- W15's managed-node, document/quit and update-restart consumer remains pending a checked range.
- W22's sole dispatcher, shortcut invalidation and receipt UI remain pending its checked range. No competing keyboard listener is installed.
- S2 owns the shared sidebar callout frame/dismissal band. Its checked source is required for update-provider adoption. No Rosetta/download fact is inferred from a remote machine or an absent macOS asset.
- Native renderer reload refreshes native/node state; there is no browser DevTools. No Electron compositor watchdog or Chromium sandbox helper is copied into GPUI.
- OS notification permission/delivery status is a separately assigned later platform API; GPUI's missing acknowledgment is never presented as granted/delivered success.

## Verification and completion

Old scoped evidence: nine early pure model tests passed. One later link desktop
run passed 16 and failed one fixture before IPC because the runner's TMPDIR exceeded
Unix socket address length; that fixture now uses a unique short path. These are
not current combined-source passes. Strict narrow runs reached typechecking and
reported actual lints; fixes added no blanket allowances. The latest unit attempt
stopped at a missing translator-context comment before tests, now fixed.

One queued run ended 143 during the approved build-VM resize before compilation;
its logs remain infrastructure interruption evidence. Only one replacement was
submitted. Immutable unit13 at be31d4d65 completed: kit166/0, link110/0,
view955/1. The one failure was About's dependency inventory missing winreg; the
inventory is now updated for winreg and the Windows host crate. All pure Windows
UTF-16 policy, login-shell process, primary bootstrap, canonical preference and
receipt tests in that source passed. Because --lib selects library targets,
the binary-only app tests were not run: no menu-field or window-guard GPUI result
is claimed. The next app check must select --bin vessel-app or --bins. The newer
base6 broadcast and project-delivery acknowledgment also need their union checks.
Static manifest parsing found a duplicated gpui-windows workspace key in the
queued f919 source: the host-bridge addition duplicated the base's existing key.
The current source keeps one entry, and Python tomllib parsed all five native
workspace/package manifests without invoking Cargo. The older queued job stays
unchanged under the coordinator rule and cannot establish compilation/tests of
this repaired union. The new control producer uses the clean dd13 base separately.
Current source-size checks passed705 files/14 existing exceptions; no allowance
increased. Direct rustfmt checked all108 Rust files changed from dd13ce37; this
is formatting evidence only, without a local Cargo/Rustc invocation.

All new cargo/rustc, clippy, tests, whole gates and capture builds run through the
prescribed remote gate tool. No local compilation or metadata-Cargo invocation is
started under the operator's VM-only rule. Direct rustfmt only formats source.

Before handoff: finish owner integration, run focused checks, freeze one final
commit/base, run the whole native gate with app-only export, then inspect EN/KO
and dark/light captures at 1280×900, 390×844 and 800×600. Submit one V4 review packet
with precise works/does-not/unverified evidence; only reviewed results advance
tracker rows or reach the release lead.

The lead approved read-only protocol 23 shared root binaries from 6036 on base 5
for E3 windows/menus/changelog/session navigation: protocol and 178 wire names are
unchanged. They omit base 5 service-port/allocation approval, strict decode-failure
handling, session.delete handover wait and pending-label admission. Exclude those
flows from captures and never claim a same-commit root build. Desktop does not
rebuild these shared binaries or access the live preview/real agents.
