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
latest application base is package line `2ebeedf9` (base13, native1614/0/2),
merged at `e1cbf82d`. Earlier immutable runs retain their base9/base10 unions. The base13 body split
retains both lifecycle render children and the private Open type; the session
size record shrank660 to608 without an allowance increase.
It contains the reviewed shared popover,
title actions, pane/query seams, attention
values and picker admission. The link's existing SavePrefs fallback was extracted
by its owner; desktop native interception still owns its sole canonical writer.
No node head was merged directly into this application tree. Existing
immutable narrow/Windows runs retain their selected older commits.
Base11 registration conflicts retain both history/import and the provisional
session command/event arms, with History forwarded through native preferences
without modification. Session stream/run keep native-only guards; OS session
handler registration remains absent. The new base's staged guards and owner
resources are not promoted by this mechanical merge.
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
| app::desktop | Desktop persistence/startup, callback admission and native preference broadcasts. |
| app::host_windows | The sole process window records, shared with host callbacks and lifecycle readers. |
| app::areas::desktop | Shared popup/native menu vocabulary, keyboard adapter, help routing and CLI controls. |
| app::areas::about::changelog | Bundled/signed-note history through the existing configured dialog. |
| app::update | App Global Rc owner of the existing updater; windows follow independent watch receivers. |

A GUI argv or OS URL is parsed before renderer startup. Claiming a primary binds
local IPC before releasing the short bootstrap lock; secondaries forward a
validated bounded intent and wait for acknowledgment. Each window keeps one
latest session and one latest directory independently. Book hydration plus an
installed owner handler permits consumption; a missing project handler cannot
erase or delay an independently received session link.

Automatic OS launch routing recognizes vessel scheme spelling case-insensitively
before generic CLI passthrough; the shared destination parser still accepts only
the canonical credential-free route. Noncanonical/malformed OS links are refused,
and extra arguments beside a valid link cannot become CLI commands. Explicit
local CLI forcing remains a separate trusted startup mode.

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
next local layout save. It passed in unit19 and the later unit21/unit25 unions.

## Windows and startup

The primary restores validated geometry (1200×800 default,400×300 minimum),
clamped to visible work areas. Additional windows start independently. Geometry
writes debounce 400 ms, serialize, and flush the latest state at exit; malformed or
newer documents are preserved. macOS reopen uses the latest in-memory geometry.
Capture-only size overrides do not relax normal product minimums.

GPUI0.3.10 app.rs:79,1220-1258 cancels on_app_quit observer futures after200ms, so that observer is
only a best-effort backstop. desktop::flush_state captures canonical preference
flush plus the latest geometry checkpoint for the lifecycle owner to await before
ordinary quit/updater restart. WindowWriter::flush acknowledges a serial save
without stopping the worker; a later document veto or failed restart leaves it
usable. Final receipts close the queue before waking their waiter. Checkpoint
success/failure/retry tests passed in the selected unit21 union.
W15 must consume this facade; no second quit manager or owned-node policy is added.

The completed flush now returns StateReceipt rather than unit. A completed save
can still be obsolete: the preference Coordinator captures an opaque publication
identity before waiting, and the receipt records acknowledged geometry. Immediately
before exit, flush_current checks that identity and canonical revision, latest
geometry, plus every live window Client has reconciled all accepted writes and has
no queue refusal. It returns Changed / Saved / LaunchOnly / Failed. LaunchOnly is
checked only after all currentness/pending checks, never as a refusal bypass.
The lifecycle owner performs one bounded retry phase after Changed and no await
between final document admission, this App-level fence and quit/restart. Node
finalization is not repeated by desktop. New queue/checkpoint and real App late
resize/preference tests passed in the selected unit21 union.
Geometry also retains an opaque publication identity rotated on every remembered
input, including equal bounds. A -> B -> A can otherwise equal an old receipt
while the writer still has B in flight. The same App-level fence now requires
the captured identity as well as value equality; no second writer or lifecycle
manager is introduced. An actual App regression covers that equal-value cycle.
The writer independently captures accepted geometry publication at flush() call
time, before its blocking sender runs. If Changed(B) enters before an older
checkpoint Write(A), the actor persists its newer state rather than clearing B
and writing A. It retains that latest value after debounce too. Finalize keeps
its explicit final-snapshot semantics and closes before acknowledgment. A
deterministic capture-future/change/await disk regression passed in unit21. This is the same worker/file/API, not another quit phase.

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
S2 reports exactd0954f4b V1 source/Kit narrow PASS (173/0, strict Clippy0), including
direct and nested retained-render paths. Its lead-landed source is now consumed
through package base8; no moving S2 branch was merged. The historical native/Linux
focused-field F5 proof remains open.
The app action guard is retained but is not claimed to protect dismissal; this
package does not edit a second menu renderer or work around it by reopening.

Shell::run_desktop_shortcut is shared by menu and keyboard consumers. Base9 now
contains the lead's production Window::set_zoom/zoom API for layout, paint and
hit testing on top of native DPI. The app uses its existing half-level model;
the fork clamps the extreme steps to its0.25 minimum and5 maximum. No rem
or test-only scale substitute is used. Native/window menu zoom entries now use
the same adapter, with per-window state; additional windows start at actual size.
The first window remembers its validated integer step in window-state version2;
version1 loads at actual size and migrates on the existing atomic write path.
Unreadable, invalid zoom and newer files are preserved under launch-only policy.

The production bounds observers synchronously update Shell. Therefore the zoom
operation defers until the invoking mutable Shell borrow ends, verifies the same
root via W7's minimal993 helper, rechecks the one recorder predicate, adjusts the
model, ends that borrow and then calls set_zoom. This avoids observer reentry and
refuses an old queued action after root replacement. The root helper is staged
provisionally in this owner tree; no W7 moving branch or runtime approval is used.
The new pure/store and actual adapter/independent-window regressions are installed
but not executed yet. IME candidate position and native window menus at zoom !=1
remain unverified in a real Linux session, as the lead records.

W22 keeps zoom extension keys withheld until its announced-base/checked-adapter
union. It uses desktop_shortcut_supported and owns the sole
matcher/default table; CloseTab falls back to CloseWindow only after layout is
Unhandled, and opening/refusing a close confirmation counts as handled.

On non-macOS, last-window removal goes through the quit owner before removal.
Additional/macOS removal has an optional document-owner callback. It defers until
the initiating Shell borrow ends, coalesces Idle/Checking, retains a refused
window and rechecks last-window status before actual removal. It owns no second
save/discard or quit policy. W15 owns registration and finalization integration.

Deferred close also retains the original weak Shell identity. Before deciding
that the window is last, and again before document admission, it verifies both
the sole registry record and the current direct/wrapped root. Clearing a pending
check matches the same handle and Shell identity, so old callback A cannot clear
replacement B's queued check. Two retained-root replacement regressions cover
that coalescing boundary and A becoming stale before B becomes the last window.
These root/check-identity cases passed in exact9fa9 App29 (140/0/2). They
do not qualify the later missing-registry refusal or the base13 receiving union.

The pending close now also owns an opaque check identity. A registry A-to-B-to-A
cycle can return to the same handle and Shell while retiring its old operation;
identity comparisons alone would revive that operation. Both deferred admission
and resetting Checking require this exact check token. A separate actual registry
turnover regression keeps the original root visible, registers B then A, and
requires the old close to perform no document/quit action. This receiving test is
not a claim of an observed native OS root-replacement lifecycle.

An installed per-window document guard also refuses a missing registry or missing
record. Unknown windows cannot bypass that guard, and close/reset operations
never initialize the registry: only the host owner installs its cleanup observer.
A negative case covers both absent and initialized-but-empty registries without
inventing document facts. This later source correction needs its next execution;
earlier App29 totals do not qualify it.

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
One process-wide try-lock admits the entire install-plus-rc mutation. A concurrent
Settings window or onboarding attempt receives CliOperationBusy before any file
write, so idempotent reads cannot race into duplicate rc appends. It never waits
on the rendering thread. A later attempt rechecks ownership and current contents.

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
The release owner confirmed that plain windows-gnu native check is not provisioned
on the build VM (missing C compiler and fxc environment). T installed the agreed
locked cargo-xwin/MSVC --native-windows-check selection with the release SDK/fxc
environment. Exact36d11faaa is queued under wp-desktop-shell-win-check as a Windows
all-target typecheck only, not a runtime or root cross-check. No local fallback runs.

## Canonical preferences and feature caches

Exhaustive Prefs/Preferences/nested patterns force new fields to select a merge
policy. Per-key removal is real; independent machines, providers and default
records survive stale windows. Provider option bags remain atomic under their
existing bounds. Queue 64 refusal is visible, unreadable files remain launch-only,
and old failures cannot roll back newer accepted edits.
After a queue refusal the Client also refuses later full snapshots until canonical
observation reconciles that window. Otherwise a second, unrelated edit could
implicitly include the first refused field while its UI rollback is still queued.
This admission latch covers settings and nonsettings without another writer or
changing Shell::prefer's unit-return API. A deterministic filled-pending-queue
test checks refusal after the physical writer drains and successful recovery only
from the reconciled canonical projection. It passed in exactec90 unit21; later
base and owner consumer unions remain separate.
Exact receipt admission is separate from canonical application: after its owner
checks the current UI attempt, acknowledge reports an unseen failure even when a
newer successful snapshot already won. It never reapplies the old preferences.
A bounded64-entry per-window failure history deduplicates recent receipt/watch
delivery without treating a different newer failure as this receipt's outcome.
An actual Shell regression exercises failed write, newer unrelated success,
out-of-order distinct failures and duplicate receipt. It passed in exactec90
unit21 and later App24/App29; none of those is App strict or an OS verdict.

Each workspace layout merges by LayoutKey; its validated pane tree stays atomic
so allocator/reference invariants cannot be corrupted. W12 owns the UI-book cache
rebase, including Dirty, sheets, rename, menus, drag and resize preservation. The
canonical map alone does not prove this cache integration complete.

Client::canonical_preferences separates observed canonical context from pending
projection. The cache hook receives old canonical before observe and new canonical
from Snapshot, before shell.prefs is replaced. Shortcut, prepared-word and terminal
cache invalidation remain feature-owned callbacks on canonical/rebased changes.

At initial subscription the window can predate its new Client: a real writer may
have committed during window construction. Read and mark the watch's initial
snapshot synchronously, then run the same apply path before any local edit. The
layout hook's first baseline is this window's actual old layout records; later
deliveries use the Client's prior canonical records as usual. Otherwise an unseen
initial snapshot could leave both the UI and a retained removed workspace stale,
and a local edit could send those old values against the new Client baseline.
The regression commits a different theme/workspace before subscription, then
asserts immediate installation, cached-key removal and preservation in the next
real local publication. It passed in unit21; no extra observer/writer or feature
resource manager is introduced.

The actual two-window receipt fixture uses the production prefer_with_receipt
wrapper in both GPUI windows and one real atomic-file writer. It stages stale
independent edits, same-key last-accepted edits, then a physical save failure
followed by another window's successful unrelated edit before watch observation.
Both window projections, canonical memory and disk must converge; exact failure
acknowledgment must neither replay old preferences nor duplicate the notice.
This fixture passed in App24 as described below; it does not replace feature-owned
cache, complete Settings, native OS or accessibility validation.
Its first App-only run23 stopped before tests: the App did not declare serde_json
for decoding the actual file. Add the existing workspace dependency only under
dev-dependencies and align its one lockfile edge, with no new package or runtime
dependency. Do not substitute manual JSON string parsing or a second reader/store.
Corrected App-only run24 on exact7d70dc15 passed118/0/2, including this actual
two-window normal receipt path and physical failure/success sequence.13 pending
owner API warnings remain, so no App strict/full Settings/native OS verdict is
inferred. The later base9/zoom/source-root-helper union is not covered by run24.

Future sidebar bands are not yet adopted from S2's unchecked source. Header and
Footer navigation edits must merge independently. Callout dismissal edits are set
deltas by logical key, preserving concurrent unrelated additions and removals.
S2 conditionally authorizes the dedicated Refused::SidebarCalloutDismissals for a
merged set over128 once its producer is checked; never silently clip or reuse the
shortcut/machine refusal. Coordinate its exhaustive prepared/translated expression
with Settings before that union. The reserved owner hook is
areas::sidebar::callouts::preferences_changed(shell,cx), called once after the
projected Prefs replacement including rollback. It reconciles the existing store
from current validated preferences, without a watcher, writer or provider facts.

Optional Receipt::completed retains the exact per-write Snapshot/Outcome even
when watch updates coalesce. prefer_with_receipt wraps the existing prefer once;
it neither duplicates validation nor emits another SavePrefs. Validation refusal
cannot return a stale receipt. Queue refusal remains typed. Feature UI generation
checks reject stale acknowledgments; shared notices own generic failures, while
Saved and LaunchOnly remain distinct. The existing atomic writer is unchanged.

Terminal scrollback is an irreversible effect, unlike theme/shortcut projection:
shrinking a local history loses rows that a failed-save rollback cannot restore.
C2's pending validated ScrollbackLines leaf and terminal owner callback must join
a committed-only canonical consumer before the Settings row is available. The
current watch/receipt API has no ordered effect-delivery caller yet. In particular,
coalesced successful limits 0 then8 must retain the minimum trim before installing
the final capacity; a later failed outcome must not hide an earlier success.
New terminal openings must use the last committed applied limit, with no replay
of predecessor effects into a replacement owner. The implementation belongs to
this same canonical actor/App consumer, without another writer or receipt/cache.
E2 confirms Saved-only: LaunchOnly and failed writes do not trim history or
advance a new opening's limit; the row shows the last applied value and refuses
that irreversible edit while launch-only storage is in effect. A previous Failed
outcome is not a permanent disable: a later successful retry may apply normally.
The checked producer and exact effect-delivery implementation remain owed; no
optimistic live trim or lost-history recovery is claimed.

## Updates and changelog

Paseo desktop/settings/desktop-settings.ts:9-40 groups release channel, sound,
managed-daemon and keep-running choices in a separate desktop JSON file. Native
keeps the existing single private Prefs writer instead: daemon management / after
quit are W15's local-node policy, sound is W7's notification policy, and channel is
the signed updater's compile-time preview feed. No unhonored stable/beta choice
or second settings file is added. W15's Enabled/Stop defaults match the reference;
the actual node consumer remains a separate pending owner union.

W7 represents independent sound and desktop delivery compositionally through
Alerts::{Off,System,Sound,SystemAndSound}. Its Sound toggle preserves desktop,
and Desktop preserves sound, both through the existing PreferenceChange::Alerts
and canonical writer. W7 owns the approved missing/new SystemAndSound default;
explicit stored modes are retained. That unannounced receiving source is not
copied into E3 here. Native visual notifications suppress their own sound and
use Window::play_system_bell for the separate sound channel; W7's private D-Bus
probe confirms the hint, not actual audibility or OS permission/delivery success.

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

The complete available release-owner notes 0.4 through 0.22 are bundled newest first,
including historical known problems/unchecked flows and Korean-only early notes.
CHANGELOG.sources retains source hashes. Notes19/20 additionally match the supplied
signed-manifest hashes: 98e33a5ed2edc2031f48cbb44fbb84f220f20466f14513da6a5b126f731b24ae
and f5dae40d1c84944ef01e8da26894c84297e710c6987b1475f7de3b212af4ca9e.
Release21 was published on2026-11-15 at14:02:47Z. Its exact2356-byte owner text
was checked against signed-manifest hash
3e610b0d0dd856bc970a10ae6bfdd2d74124d8ddb96c1aa6523aa8ecca5fb00c
and added without changing any older note bytes or dropping its known-problem /
not-checked lines. Published release22 (2026-11-15T19:35:42Z) adds the exact4341-byte
text at SHA2561d145931fa8af863e94feaa032c1b3307026280b838dc9c748d3207a0d1729ef,
including its Known limits line; all earlier note bytes remain unchanged. Running signed-feed notes supplement that history. No published Vessel changelog
endpoint exists; never fetch Paseo releases. Reveal is 1 immediately, 5 after yield,
then 10 more; 19 real releases make the paging controls meaningful.

## Remaining integrations and deliberate differences

- W21's older bounded session lookup is present provisionally. Its retained connection snapshot carries Name/NodeClient and rechecks NodeId, but does not yet retain current G4 context/validity or the W12 carrier. The owner explicitly requires the OS handler to remain unregistered until its current checked portable producer arrives; the existing latest typed session target therefore stays queued. C2's retained Binding terminal cutover is also still required. Neither path is declared current-context safe.
- W11's managed-local project adapter is not consumer-ready. Its directory intent stays queued; no first-alias fallback, implicit trust or registration occurs. The callback now returns ProjectDelivery::Opened only after the exact editable draft opens, otherwise Pending retains the directory without overwriting a newer arrival. Pending is idempotent and does not emit repeated UI events.
- W15's managed-node, document/quit and update-restart consumer remains pending a checked range.
- W22's sole dispatcher, shortcut invalidation and receipt UI remain pending its checked range. No competing keyboard listener is installed.
- S2 owns the shared sidebar callout frame/dismissal band. Its checked source is required for update-provider adoption. No Rosetta/download fact is inferred from a remote machine or an absent macOS asset.
- Native renderer reload refreshes native/node state; there is no browser DevTools. No Electron compositor watchdog or Chromium sandbox helper is copied into GPUI.
- OS notification permission/delivery status is a separately assigned later platform API; GPUI's missing acknowledgment is never presented as granted/delivered success.

## Verification and completion

Selected checks qualify their exact source, not future changes or a completed
feature. All new Cargo/Rustc work runs through the prescribed remote gate; direct
rustfmt and the size/token scripts are the only local source checks.

| Evidence | Exact source and result | Limit |
| --- | --- | --- |
| Shared adaptive popover/configuration, run143258Z | 7730d570; selected fonts/fmt/quality/strict/test all0, raw1444/0/2, Kit193 including13 control cases; V1 row233 and announced line2b806402 | No E3 policy, OS, consumer or Linux focused-field F5 approval. |
| Shared header actions, run153102Z | dd7ef7ec; all selected steps0, raw1447/0/2, Kit196 including3 header cases; V1 row249 and announced linea7aab9d6 | Caller owns stale-action authority; headless Dark fixtures do not prove both-theme OS visuals. |
| Unit21, run160356Z | ec90c53f;1420/0/2: App106/0/2, Kit197, Link142, View975 | Delayed writer, state fence, initial preference/cache and edit-recipient tests passed; later two-window and zoom changes excluded. |
| Library strict26, run173910Z | b5cf516a; all-target Kit/Link/View Clippy0, actual changed Kit/Link checking | App and later enum/base13 source excluded. |
| Library strict22, run160743Z | ec90c53f; all-target Kit/Link/View Clippy0 | App excluded. |
| App24, run170102Z | 7d70dc15;118/0/2 including real two-window prefer_with_receipt, physical save failure followed by another window's success before observation |13 pending owner API warnings; no App strict or later base9/zoom proof. |
| Unit25, run173910Z | b5cf516a;1476/0/2: App127/0/2, Kit215, Link143, View991 | Actual production zoom/DPI/bounds, both limits, extra window, queued root replacement and legacy store migration passed; later close-admission hardening excluded. |
| Windows4, run151309Z | b0cef8c4; fonts0 and locked MSVC all-target App check0 | Normal/test warnings20/14. Typecheck only; no Windows runtime/root cross-check or later writer/base/zoom proof. |
| Windows5, run173954Z | b5cf516a; fonts0 and locked MSVC all-target App check0, including actual changed Kit/Link/App Checking | Normal/test warnings22/13. This compiles the production zoom and store migration, not later close/host enumeration/base10 changes; no Windows execution, IME or OS menu claim. |
| Close-admission App27 | a006a921; stopped before tests with E0502 in registered_probe | Fixture borrowed the registry before capturing its Shell. The receiving repair uses the real shared register operation, without changing assertions. |

App29 on exact9fa9dc4f passed140/0/2, including all three deferred close/root/check
identity tests, complete enumeration and the real current-root/missing-Client
persistence fence. Its13 warnings include an unused WeakEntity import removed
afterward and unconsumed owner APIs; it is not App strict or a C5 domain verdict.
The obsolete App28/c197 and Host3/dc059 queued runs were cancelled143 before
execution, with records retained. App29 predates the base11/base13 receiving union. No App
strict result, final whole native gate, native-build/export, inspected capture
set or V4 package verdict exists for this E3 tree. Pending owner consumers are
not hidden by lint allowances or fabricated registrations.

Earlier red runs are retained: unit16 stopped on stale owner fixtures; unit18 on
missing imports/old fixture fields; unit19 executed1396/1/2 with a single GPUI
foreground-wake scheduler refusal in a real-I/O test, repaired only by that test's
supported allow_parking mode and closed by unit21. App23 stopped before tests on
missing dev-only serde_json, closed by App24. Windows1 reached winreg's Cow bytes
mismatch; Windows3 also found a Linux-only test waiter. Existing Into conversion
and a bounded std waiter compiled in Windows4. No earlier red is relabelled green.

Native menus and popup/shortcut adapters use one production
Shell::desktop_commands_admitted predicate. Recording start retires the existing
menu before capture/blur. The actual retained popup-pointer/clipboard/focus test
passed in unit21 and later unit25; W22's real newer SequenceCapture union remains
pending. Native edit entries use desktop actions routed through admission before
forwarding to kit editing. These wrappers compiled in App24/unit25; no actual OS
native-menu/recorder or physical keyboard result is inferred.

[Decision Log]
- Purpose: a retained edit popup must not redirect Copy/Paste into a newer focused field.
- Constraint: shared MenuState correctly preserves newer focus on close; native edit commands intentionally use current focus.
- Alternatives: force focus back, duplicate return-focus ownership, or retain a weak original input recipient.
- Choice: require the original weak input recipient to remain focused after the shared close path before dispatching popup editing.
- Why: the same menu owns focus restoration; changed/dead recipients refuse without moving or retaining focus.
- Cost: another input needs a fresh popup. Actual retained-key Copy/Paste tests with a focus-changing interceptor passed in unit21/unit25. This is UI input identity, not host/session authority.

The sole host registry has a separate isolated producer on announced82ca07a7.
Its exact d8b73397 selected suite passed1490/0/2, including five host-window cases
and the root helper; V1 row273 accepted bounded source and the selected checks.
This E3 tree merged the selected owner, preserving its existing close-coalescing
field. Desktop startup installs the real window factory; all actual creations
register through the owner, and menus/launches use its root-checked dispatch.
The former E3 selector and duplicate registry cleanup were removed. No second
list or notice/context authority is introduced. App29 exercised this receiving
source, while the later base13 union and final lead gate remain unqualified.

Global lifecycle traversal needs a stronger boundary than choosing one valid
window. host_windows::shells now returns LiveShells::Unavailable if any live handle
is missing, duplicated, expired or has replaced its registered root. desktop::shells
delegates to it and requires App-level mutable access after all window/Shell
borrows end. The preference currentness fence uses that same enumeration and
also refuses a current Shell without its canonical Client. A partial weak list
can no longer grant global admission. The direct/wrapped/unknown/root-replacement and real preference-fence regressions
passed in exact9fa9 App29. The tiny staged host producer e262 separately passed
all selected steps/raw1491/0/2 and V1 row284; only actual receiving callers remove
its two staging guards. Neither proof grants C5 document/node authority. W15's
document guard must refuse this result, not omit the successor; its actual
consumer and the receiving source/runtime checks remain required.

Before handoff: finish owner integrations, run focused checks, freeze one final
commit/base, run the whole native gate with app-only export, then inspect EN/KO
and dark/light captures at1280×900,390×844 and800×600. Submit one V4 review packet
with precise works/does-not/unverified evidence. Only reviewed outcomes advance
tracker rows or go to the release lead.

The old read-only protocol23 root binaries were reapproved only through dd13/base6
for windows/menus/changelog/credential-free navigation, with no same-commit claim.
They omit service-port/allocation approval, strict decode failure, session.delete
handover and pending-label admission. The current base13 uses protocol25: obtain
fresh lead approval before any fleet capture. Desktop does not rebuild shared
root binaries, launch real agents, use live preview ports or open real browsers.

Availability uses LiveShells::{Available,Unavailable}, not an ad hoc Result error
or another App dependency. The complete-list test preserves typed refusal; real
callers must match Unavailable to refusal, never convert it into an empty list.
This source correction does not inherit the earlier focused-host review.
