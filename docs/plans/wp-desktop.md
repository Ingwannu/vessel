# Desktop lifecycle and first-run setup

## Reference and scope

Paseo 0.15.4 is the behavioural reference, read before implementation:

- `packages/desktop/src/features/menu.ts:80-231`: File/New Window,
  Edit, View zoom/reload/fullscreen, Window; zoom withheld during shortcut capture.
- `packages/desktop/src/window/window-manager.ts` and
  `settings/window-state.ts`: first window restores geometry, later windows do not;
  default 1200×800, minimum 400×300, visible-work-area clamping, 400 ms save debounce.
- `packages/protocol/src/agent-deep-link.ts:1-80`: reject credentials, ports,
  queries and fragments; navigate by host identity, queue until the renderer is ready.
  `desktop/src/agent-navigation.ts` and `window/desktop-window-owner.ts` own delivery.
- `packages/app/src/onboarding/internal/model.ts:1-187`: decide once after connected
  setup host and hydrated directory; existing projects suppress automatic setup;
  appearance/providers/projects/orchestration/phone; monotonic visits reject stale advance.
- `onboarding/internal/orchestration-setup.ts:16-144`: all choices initially on,
  only supported effects offered, apply on Continue and advance only after every success.
- `changelog/internal/parse-changelog.ts:1-180`: release and section headings outside
  fences; retain Markdown; `use-revealed-releases.ts`: reveal 1, then 5, then +10.
- `desktop/updates/desktop-app-updater.ts` and
  `desktop/updates/resolve-update-callout.ts`: manual and focus checks,
  download/install/failure status, restart confirmation, What's new.
- The desktop survey found no system tray. None is added.

## Existing asset analysis and decision

**Modify existing assets.** Reuse About, updater and its signed channel feed,
Prefs' atomic owner-only persistence, settings pages, workspace project state,
provider discovery, device pairing and kit dialogs. GPUI already supplies windows,
menus, URL callbacks and window bounds; std and the existing serde/chrono crates
suffice for new pure models. Electron/Tauri would duplicate the established GPUI
host, so no new toolkit or dependency is introduced.

The pure rules belong to `view::desktop` and `view::onboarding`; desktop-specific
I/O belongs to `link::desktop` and the GPUI entry point; screen state and drawing
belong to `app::areas::desktop` and `app::areas::onboarding`. Shared shell/link files
receive registration only. Node service execution remains owned by the node
lifecycle implementation; the app must not invent commands or signal arbitrary PIDs.

## Decision log

- Purpose: desktop windows and startup should preserve identity and make setup
  repeatable without letting old asynchronous work advance a new visit.
- Constraints: a native host replaces Electron; an updater and private prefs store
  already exist; the node service control contract is separate.
- Alternatives: another desktop runtime; local shell commands for lifecycle; new
  settings storage; one wizard implementation per screen.
- Choice: extend the existing four layers and reuse operational screens and typed
  calls. Keep navigation targets parsed, bounded and credential-free.
- Why: this preserves one owner per state and the existing update trust boundary.
- Consequences: GPUI OS dispatch is verified separately from pure model tests;
  unsupported node operations stay hidden and are reported as blocked, never faked.

## Deliberate differences and dependencies

Vessel uses `vessel://h/<node>/session/<session>` rather than Paseo's scheme and
agent terminology; both identifiers are validated protocol IDs. Workspace links
require the workspace contract before registration. Update artifacts remain
Vessel's signed manifests, never Paseo's GitHub installer or unsigned Markdown.
The built-in feed channel remains usable until stable/beta artifacts are published.
The updater's latest signed manifest notes remain the release-text source;
the desktop-shell package owns retention and changelog presentation.

ADR 0071's earlier proposal is superseded where it inserts a Machine step or
opens without a setup host: pairing remains the no-host route; this wizard follows
the reference's five steps and connected/hydrated host gate. Phone is omitted at
compact width as Paseo's current model requests. Phone native builds do not open
setup automatically. Completion remains client-local; unreadable preferences
suppress automatic setup and are never overwritten.

## Validation and handover

Per-commit checks: format, size/boundary/token checks, changed-crate tests and
clippy through the build slot. Package end: one native remote gate, inspected
captures in both languages/themes at desktop/compact/minimum sizes, then review.
Results and exact unsupported operations are recorded here before handover.

## Package ownership refinement

Desktop shell lifecycle, menus, OS navigation delivery, CLI installation, update
presentation and changelog are implemented separately. This plan's implementation
now owns the walkthrough, managed-node controller and finalization before quit or
update. The shared typed navigation contract was split into its own commit so
session navigation and the OS host consume one generator/parser.

The historical proposed model in the earlier onboarding branch was inspected:
it inserts Machine and decides without a connected setup host. It cannot be
adopted unchanged because both contradict the current reference. Its completion
and visit invariants are retained in the model here, with the reference's exact
five-step order and compact Phone omission.

The node lifecycle library contract is in `docs/plans/desktop-local-node.md` on
the node line. The node integrator approved client/proto/store/proc edges, bounded
readiness in `vessel-proto::local_launch`, and a consuming observation-only Proc
Detached handle. The desktop package contains no service-file or arbitrary-PID
control path. Published builds currently provide only their configured preview
channel and no bundled daemon/CLI; managed startup remains unavailable when the
corresponding verified package binary is absent.

Validation so far: initial completion/step/visit/orchestration models, seven unit
tests passed. The first build rejected missing translation context comments;
those were added before the passing run. Native app/link clippy is queued.

### Managed local node integration (implementation checkpoint)

The root `vessel-local-node` crate is the only child owner. Native `link::local_node`
adds a bounded process-wide actor and read-only watch snapshots, without GPUI.
`app::local_node` owns that runtime until process exit; per-window areas register
its opaque authenticated connection in their own link. Registration uses a typed
`LocalTarget` with NodeId, validated ephemeral Name and OwnedChild versus
AttachedManagedHome provenance. A saved-name collision refuses registration; a
later collision invalidates it before normal book dialing. No credential is added
to the user's book. A stale registration response cannot publish a stopped target.

Onboarding prioritizes this acknowledged, hydrated local target, waits through
managed startup/registration, and otherwise chooses the first saved host. Controls
also compare the fleet's authenticated NodeId with the pinned visit target. All
controller preferences and completion records pass through the desktop canonical
three-way preference actor, so another window cannot reset them with a stale save.

ADR0094 fixes companion binaries at the running executable's parent/bin/vesseld
(or vesseld.exe). The shared W19 bundle getter owns platform layout; macOS is
explicitly unsupported pending signed helper layout. Missing binaries display
NotInstalled; no PATH search, shell command, downloaded helper or PID record is a
launch source. The explicit private home is config/vessel/managed-node. The root
Store facade dependency adds its existing SQLite dependency to the link graph,
but this controller only calls DataDir/StartLock and never opens a database.

Quit and update restart await owned-node finalization and canonical preferences
flush before app exit/restart. Protected stop refuses active sessions; explicit
keep-running or interruption choices are separate UI actions. A last-window
refusal opens a window again. The irreversible OS backstop requests observation-only release through the
runtime supervisor; its 200ms callback is not a protected-stop deadline or veto. Attached nodes
are never stopped. App updates retain the release track's canonical updater.

Verification: node-line all-target clippy passed at9d13f24f across proc/local-node/
store/proto/node/testkit. Current onboarding model tests8 passed. Process fixture
runs and native integration compilation are pending; no runtime or Windows claim.

The release owner confirmed that NodeInfo.version (crate version) and the app's
release label are different domains. They are never compared. Owned update
restart always stops and reaps the old child before invoking the canonical
updater, even if ordinary quit keeps the node running. An Attached node uses the
existing handshake protocol range as compatibility evidence and is never re-owned.
Interruption confirmations retain an opaque snapshot permit; an intervening
controller operation invalidates it instead of interrupting a replacement launch.

### Exact operation acknowledgements and interruption admission

Manual stop/restart and quit/update finalization retain the failed operation's
immutable Snapshot for their confirmation dialog. The modal never regenerates its
confirmation from a later watch value. Its opaque permit expires when the actor
publishes a replacement observation, even if NodeId remains unchanged; a stale
approval is refused before any interrupt RPC. Manual actions now receive a
bounded-queue exact acknowledgement, and duplicate actions in one window remain
disabled until it settles. Cancel preserves the process. Quit alone offers an
explicit keep-running alternative; update approval resumes update restart after
owned-node finalization and canonical preference flush. The standard kit modal
stays above the current screen rather than navigating away from setup.

These changes are source checkpoints. Their remote native type check and the
combined root checks remain queued; no current-head runtime or capture claim is
made from the older passing model/process suites.

The package line's shared dialog dismissal/focus seam is now merged. Actual
setup completion and local-node prompt/log dismissal acknowledge the shared
DialogState only after closing; step changes keep the same opening cycle.
Compact setup renders the fixed opening's current/total step counter and the
step descriptions in English and Korean. The wide modal still uses the shared
kit header/footer with an 880×640 bound and 200-point rail; this renderer reuse
is a deliberate native composition difference from Paseo's rail-only brand header.

Unsupported companion bundle layouts (including the unpublished macOS layout)
render the same ordinary Not installed state as an absent Linux/Windows daemon.
Initialization errors remain Failed. No guessed macOS helper path is spawned.

The Phone step reuses Vessel's existing authenticated device invitation and
credential reveal/expiry controls for its fixed setup host. Paseo's App Store
and Google Play entries install a different product and cannot connect to Vessel;
no Vessel mobile distribution or compatible store URL has been supplied. Those
store entries and Paseo-specific relay/Tailscale documentation are consequently
not copied. Device pairing remains usable through the existing native owner.
This is a product availability difference, not evidence of mobile app parity.

### Captured dismissal identity

Onboarding Skip, Back, rail selection and every modal dismissal capture Visit,
and check it before mutating the currently open walkthrough. A stale close after
reopen cannot close a new target or rewrite completion. Local-node confirmation
callbacks also capture an opaque per-modal Rc identity before acknowledging shared
focus restoration; stale cancel/approve handlers cannot dismiss a replacement
prompt. Process-interruption authority is still independently checked by the
actor's opaque Snapshot permit. Added GPUI stale-close/rail regression; execution
is pending the combined native gate.

The ba0518c3 combined narrow suite reached the kit and stopped on dialog's
101-line function limit after the shared base merge. Card-width selection is now
owned by DialogSize rather than the renderer; dimensions and behavior are
unchanged and no lint/size allowance was raised. This is a failed narrow result,
not app type-check or runtime evidence. The earlier 51949423 NodeHome type error
was already corrected before this suite.

### Document owner joins ordinary quit

`app::quit::set_document_guard(fn(&Shell)->DocumentReadiness,cx)` is C5's sole
read-only admission hook. C5 registers from CodePanel::new before any buffer can
become dirty. The guard must include every active/retained buffer and original
write ticket; read-failed empty viewers alone are not modifications. The desktop
registry supplies existing live Shell owners, with no second window registry.
Normal quit and app-update restart check admission before node finalization and
again immediately before actual process exit/spawn, with no await after the final
check. A later edit while the node is stopping therefore still prevents exit.
Dirty/Pending/Failed preserves windows and reports a localized refusal; W15 never
saves or discards document content. C5 may expose an explicit keyed Save All as
its own separate user action. The initial no-document guard is replaced by C5;
until its registration lands, this is a staged seam, not editor protection proof.

C5 confirms no recovery journal exists. Forced OS/process exit can lose unsaved
edits; on_app_quit's node/prefs backstop cannot veto it or provide document
recovery. This limitation remains explicit and is not covered by ordinary quit
admission or any process-lifecycle test result.

`app::quit::admit_shell(&mut Shell,&mut Context<Shell>)->bool` is the same
read-only guard/notice adapter for additional-window and macOS window closes.
The desktop-shell owner must call it before removing that particular window;
last non-macOS close still uses whole-app request_quit. The guard state test
covers Dirty/Pending/Failed refusal and Ready admission without preference edits;
it does not claim C5's buffer mapping or OS hard-exit recovery is implemented.

All-window admission runs in an App-level update after the initiating Shell
button/menu borrow has ended. Re-reading that same entity during its mutable
Context callback would violate GPUI's entity borrowing. Final update execution
also performs its guard outside any Shell borrow and does not await before
updater.restart; only a failed restart then updates the originating Shell notice.

The current narrow type check reached app and found the setup hint's borrowed
text escaping into a static element; hint/description elements now own their
strings, and the unused scope-preparation context was removed. The earlier
App::quit function-item mismatch is corrected by the App-level admission closure.
Desktop-menu trait imports/unused paths are coordinated with the shell owner.
Owned status names retained management rather than asserting the process is
running: a failed checked wait retains that lease and displays its typed failure,
without claiming Serving or a confirmed stop. Fleet hydration remains separate.

The refusal modal's Keep running and quit applies only to that quit attempt. It
does not change the separately persisted Keep running after quit preference.
Only the explicit settings switch changes future-launch policy.

The current pane base adds Prefs.layouts. The shell owner's source-only
86082834a per-LayoutKey delta and two-workspace/removal/concurrent-add regressions
are adopted provisionally, preserving this package's separate completion and
local-node policy field deltas. Each validated tree stays atomic. W12's owned
layout-cache rebase is a separate pending consumer; this preference merge alone
does not prove cross-window layout cache behavior. Native Cargo.lock now includes
only the incoming root serde_path_to_error and Store-to-Proc manifest edges;
the already queued local build generated those two missing lock dependency lines.

Finalization phase is now owned once by app::quit, independently of whether a
local-node controller exists. `is_finalizing(&App)` is C5's editability and
mutating-handler admission hook: keep editors read-only and refuse new edits or
writes while a quit/update is settling. `begin` coalesces requests and refreshes
all windows; refusal/failure `finish` restores editing. Registering a new C5 window
replaces only the callback and cannot reset an in-flight phase. This prevents a
new edit after Ready from leaving unsaved work behind a node already stopped;
the final guard remains necessary for other asynchronous owner changes. Local
node policy/actions also disable while this same phase is active.

The ca0f17957 native type check stopped at the pane base's new Ask::Layout arm.
It now uses the ordinary client path alongside session/worktree operations;
layout's existing handler checks its own captured NodeId. Onboarding's stricter
control guard remains on project mutations and reused settings controls, and the
generic pinned-alias replacement fence still applies to every client lookup.

The reusable control-only producer is now app/src/quit.rs plus the canonical
view::desktop::quit enum and quit.ftl words. It has no desktop-shell/controller
calls. All-window registry traversal is in local_node/documents.rs, the host
adapter. C5 can consume the control blobs without importing the provisional E3
branch; the adapter still uses the one existing desktop registry. The policy and
callback signatures are unchanged, including direct admit_shell for E3 closes.

The shell owner's source-only 9066c757 deferred window guard is adopted with its
real GPUI regression, adapted only to this tree's existing Shell::without_io
fixture. The one registration beside set_quit_handler passes quit::admit_shell.
Additional/macOS close waits until the initiating entity borrow ends, coalesces
requests, and rechecks last non-macOS status before removal. A whole-app quit in
flight refuses individual window removal so failure/prompt targets remain alive;
this uses the same app::quit phase, not a second manager. Runtime union proof is
still pending; the two source packages' prior scoped results do not cover this.

Disabled local-node controls are also guarded at their mutation entry points;
visual disabled styling alone cannot admit queued callbacks. Working/queued
lifecycle operations and the shared finalization phase refuse new node actions or
policy edits before any preference mutation/actor enqueue. Missing-bundle manage
activation is likewise refused. A GPUI regression captures a queued policy edit
after begin and requires the original preferences to remain unchanged.

### Canonical connection adoption (2026-11-15)

The base-6 merge supplies the single link admission allocator. Setup Target now
wraps that canonical ConnectionContext; Resolve captures it from the Fleet rather
than resolving a queued alias anew. Bind and reused-control guards compare the
whole context, including a same-node reconnect epoch. Failed binding still owns
its Visit fence. UI installation also checks the exact current Fleet context.
W12's opaque queued-delivery capability remains a required subsequent integration;
these checks do not claim to close the actor-to-UI queue retirement interval.
The managed-local registry port remains reserved third after W2 and W12 land;
its pre-base-6 adapter is not a compiling handoff. The exact generated-name helper
uses native View's existing machine_book::Name, not root client's Book Name; no
crate edge or second parser is added. E2 converts at its existing native edge.

### Immutable controller transport correlation

Local actor state now owns a non-clone Source beside its LocalNode. A successful
new start/restart mints its opaque transport lease; ordinary state/log/failed-stop
snapshots clone that lease. Original transport closure and Source drop publish
irreversible retirement, and snapshots retain receivers only. The per-window
observer preserves its registration ticket/target across same-source status
updates instead of generating artificial reconnects. This is controller-source
correlation, not a UI epoch allocator, a process receipt or signal permission.
The canonical registry's lease-retirement watch and exact source-removal join
remain part of the third-slot ManagedLocal port; this preparation does not claim
that the legacy adapter implements them. Tests cover receiver clones surviving
owner withdrawal, successor independence and actual original transport closure.

### Setup footer uses the shared source control geometry

Paseo onboarding-modal.tsx:86-105 uses Outline Skip, Secondary Back and Primary
Continue, with Skip separated from the trailing actions. The setup footer now
uses those kit variants and explicit ControlSizing from its existing compact
layout decision (32px pointer /44px touch). First/last omitted actions are not
invisible keyboard targets. No private control-height override or second button
renderer is introduced. The existing kit dialog still owns card/scroll geometry;
actual minimum-size and compact captures remain required before parity approval.

Update failure releases the app-wide finalization phase before attempting to
notify the initiating weak Shell. A window disappearing while I/O awaits must
not leave every surviving window permanently frozen. Notice delivery remains
best-effort; it does not own the phase or controller cleanup authority.

Managed-local admission now has a dedicated safe refusal vocabulary rather than
reusing book-write failures. Generated-name collision, a second local source,
identity change and withdrawn transport have distinct EN/KO notices. Their
canonical actor enforcement still belongs to the reserved third-slot source.
The local Overview contribution is read-only provenance plus navigation to
General; it no longer embeds stop/restart or policy controls under host context.
Canonical Machine provenance/Unknown and Book-only edit fences remain the E2/G4
integration contract; no alias-only locality inference is introduced.

### Irrevocable OS exit and GPUI's 200ms shutdown window

[Decision Log]
- Selection: modification of the existing controller, using its existing Tokio
  reactor and standard-library thread/channel ownership; no external API/crate.
- Rejected alternatives: blocking the non-vetoable callback beyond its platform
  budget, leaking a runtime without an explicit reaper, or duplicating node stop
  policy. None establishes the intended cleanup/observation lifetime.
- Tradeoff: one blocked supervisor thread is retained alongside the existing
  reactor worker; ordinary user-mediated finalization still owns all vetoes.

Verified local GPUI0.3.10 app.rs:79 and shutdown:1219-1268: on_app_quit cannot
veto and waits only SHUTDOWN_TIMEOUT=200ms, then cancels its futures. Waiting for
our 15s protected stop there could cancel the callback before its Keep fallback
and drop the runtime/owned child. Ordinary quit/update remain unchanged: they
check documents, flush canonical preferences and await protected finalization
BEFORE App::quit or updater.restart.

The backstop now synchronously sends Preserve to a bounded one-slot supervisor
created before any local child starts. That existing supervisor owns the reactor,
requests KeepRunning without an interruption confirmation, and retains runtime
and direct-child reaper while the launcher lives until observed exit. Controller
Drop's later Ordinary message cannot override the first handed-off policy. No
saved Keep preference is changed. Thread creation failure occurs before actor
launch, with the existing typed runtime error. There is still one process-wide
controller, not a second quit manager or a process signal API.

The observer does NOT promise a release acknowledgement within200ms. If the OS
ends the process before the queued release executes, releasable-process
launcher-death semantics apply (including the documented Windows crash-policy
cost); no PID is re-owned later. No document recovery journal or forced-OS-exit
save guarantee is claimed. Synthetic runtime tests cover real supervisor/thread
ownership, original UI sender drop while release acknowledgement is held, reaper
runtime survival, eventual observed exit and ordinary owner cleanup. Actual
GPUI/OS-exit/node integration and Windows/macOS remain unverified.

The reserved ManagedLocal actor transition mechanics and weak-observer lifetime
are recorded in [desktop-managed-admission-order.md](desktop-managed-admission-order.md).
This preparation does not lift the W2/W12 product-promotion source hold.

Quit refusal distinguishes state-persistence failure from the typed node-control
failure. Preferences/geometry errors do not claim the local node could not stop
and cannot offer interruption approval. Only an actual ActiveSessions snapshot
from the controller opens that fixed process confirmation. Update state-flush
failure uses the same localized save/retry notice after releasing finalization.

### Recovery after Keep succeeds but final state save refuses quit

Surrender cannot be undone. If a later persistence/final document fence keeps the
app open, General offers explicit Reconnect for Kept. The same public startup
path authenticates the still-running managed home as Attached; it never rebuilds
an Owned child from its identity. The detached direct-child reaper stays alive.
Only Owned stop is requested when disabling automatic management; Attached,
Kept and already stopped states persist the opt-out without a spurious NotOwned
control failure. The portable root join will extend its existing public-API Keep
fixture to verify reattachment and continued refusal of stop/restart authority.
No new process-ownership proof or force API is introduced.

### Destructive control intent retains its observed owned node

Protected Stop/Restart now capture a sealed ControlLease from the rendered owned
snapshot, containing its immutable observation identity and NodeId. The actor
rejects a stale lease before entering the control path, including a same-node
replacement. Interruption Confirmation wraps that same observation and retains
its separate explicit UI choice. Action::observed_node is only a document facts
filter; neither a NodeId nor a displayed status can construct an accepted lease.
This prepares C5's one-node retained-write guard without retargeting its checked
node across the async UI/actor boundary. Stale/replacement/forged-display tests
are prepared; no current checked handoff of these new app-side changes is claimed.
The shared root library and its frozen54 gate are unchanged.

### Released-child history during explicit reconnect

[Decision Log]
- Selection: modify the existing controller actor; standard Tokio JoinSet only.
- Purpose: after Keep and a later state-save veto, explicit Reconnect authenticates
  Attached reuse without losing the earlier direct-child reaping obligation.
- Rejected alternatives: infer the original child exit from the Attached socket,
  manufacture owned authority from its NodeId, or let current display status end
  the reactor while a historical released child is still being reaped.
- Cost: retain at most eight observation tasks in the existing actor. Start refuses
  when this bound is full, before launch/authentication; completed observations
  are pruned. No signal handle, connection source or second registry is retained.

An irreversible PreserveExit command closes actor admission before surrendering
its current owned child. Its acknowledgement follows the current released child
and every historical direct-child observation, independently of current Attached
status. Failed/aborted observations hold the runtime rather than claim reaping or
fall through into owned Drop cleanup. The actual OS may still terminate the
launcher; this is no hard-exit persistence guarantee. Synthetic fixtures prepare
historical-observer, bounded-admission and Attached-successor barriers; changed
source compilation and real node/OS integration remain owed.

### Manual stop/restart document admission

[Decision Log]
- Selection: extend the existing read-only document hook and finalization phase.
- Purpose: a manual protected or explicitly confirmed stop must not strand an
  accepted editor write on the observed owned node.
- Constraint: C5 owns canonical retained FileKeys and original pending/uncertain
  writes; a welcome NodeId alone is neither process authority nor admission.
- Choice: C5 may register one node-filtered callback over the same reducer.
  Until then its global guard is the conservative fallback. Dirty, Pending and
  Failed refuse without implicit Save/Discard; Start/Reconnect remains separate.
- Alternative rejected: another document map, alias-based ownership inference,
  or checking after Controller.request, which already enqueues synchronously.

Manual destructive intent captures an opaque ControlLease from the displayed
Owned snapshot before yielding. After the initiating Shell borrow ends, the one
app-wide phase freezes new mutation intent and the registered desktop shells are
queried for that exact node. Only then is the controller command enqueued; the
actor rechecks the original permit before any effect, refusing a replacement even
with the same NodeId. Original accepted edit mirrors/write acknowledgements still
settle. Global phase release does not depend on the origin window surviving.

The C5 node callback and barrier fixtures remain owner integration work; this
source alone does not claim dirty-buffer or runtime safety before that checked
consumer union. No forced-exit document journal exists.

Current source checkpoint after the base-8 merge remains unqualified as a product
union: the old native local-registration adapter does not implement canonical
Connected/ManagedLocal admission. Source changes in the shared connections and
selected lease remain explicitly held until the lead grants the third slot after
W2's real A4 and W12's actual DeliveryGuard promotion. No unreviewed fake BookEntry
or second connection epoch is used to make this checkpoint compile.

Lead ordering update: W2 A4 returned for correction and released its writer slot.
The prerequisites for ManagedLocal are now a real production DeliveryGuard export
with an actual consumer (whichever owner lands it) plus W12's actual follow
promotion, both green on the line, followed by an explicit third-slot grant.
Staged exports remain insufficient. This changes writer scheduling, not the
accepted source-lifetime, classification, reservation or barrier contract.

### Current-state persistence fence before exit

[Decision Log]
- Selection: consume the desktop-shell owner's bounded source packet 552b3f526;
  modify the existing normal quit/update consumer, without another writer.
- Purpose: a successful old write is not proof that the latest preferences and
  geometry can be abandoned. A→B→A bounds changes also invalidate old receipts.
- Constraint: accepted preference writes may precede canonical publication or a
  Shell's observation; launch-only storage cannot bypass unsettled local edits.
- Choice: one owner receipt checks canonical revision/publication identity, all
  live clients settled/no refusal, and latest geometry plus publication token.
  Then the existing document guard and actual quit/restart execute synchronously
  in that same App update, with no subsequent await.
- Tradeoff: each persistence phase allows four attempts and ten seconds total;
  changed receipts yield to reconcile UI observations, then take a fresh receipt.
  Exhaustion, write failure and deadline retain the app with a save/retry notice.
  LaunchOnly is admitted only after the owner's earlier settlement checks.

The first bounded phase settles policy/preferences before node effects. Owned
node finalization runs exactly once. A fresh post-node persistence phase handles
later changes; it never retries the node stop/release. Refusal restores the same
app-wide phase, and writer checkpoint workers remain alive after a veto or failed
updater restart. The short OS observer uses the same owner facade as best effort;
its 200ms window promises neither durable save nor veto.

The mirrored Link preference actor, tests and window writer and App receipt
fixtures are exact source552 blobs, with the existing merge/layout implementation
and local-node/onboarding fields retained. Desktop geometry wiring is surgical;
no moving session/project/menu policy is imported. Producer unit18 and this actual
consumer union are not yet checked; no scoped predecessor result is borrowed as
proof of the new fence, retry deadline or app-wide exit behavior.

Lead-approved unavailable skeleton: the obsolete branch-only registration that
constructed/cloned raw Connected values is removed. Until the canonical third
writer slot, registration of Some(connection) explicitly returns Unavailable,
withdrawal acknowledges None, and no local Machine is published. This deliberate
incomplete difference from Paseo hides the unusable local Fleet destination;
owned process controls remain separately scoped. No shared connections/selected/
delivery source is edited, fake Book or second epoch introduced. The skeleton
requires V1 review if it reaches the line and is not a parity-complete handoff.

Unavailable-skeleton cleanup also removes its branch-only refresh hooks rather
than retain empty async methods. Fleet publishes only the existing book path;
no local target is synthesized. The desktop-shell owner approved removing the
two uncalled session/project setter definitions in this older snapshot, preserving
its fields/drain/None behavior until their actual owner consumers arrive. This
is source hygiene, not session/project delivery or E3 completion.

The node-filter setter stays literal cfg(test) staging until C5 registers its
actual retained-buffer reducer. The production readiness query conservatively
falls back to the existing global guard; no absent owner is presented as a
successful node-filter capability. C5 promotion requires its real caller and
combined checks, not a synthetic registration for lint.

Owned daemon launch removes the updater's app-only RESTARTED_FROM marker from
its child environment, including inherited fallback environments. This consumes
the desktop-shell owner's small without builder rather than mutating process
globals or duplicating shell resolution. The existing immutable launch recipe
still validates program, arguments and private home; only child environment
changes. Its in-memory regression is prepared; new-source execution is owed.

The earlier unconsumed native name-grammar draft is removed while the canonical
lower ManagedLocalAlias value waits for an announced base. The unavailable local
registration has no generated-name path. Native wrappers will be restored only
as delegations to proto::book_values::ManagedLocalAlias; no duplicate prefix or
UUID grammar is retained in product or test code.
