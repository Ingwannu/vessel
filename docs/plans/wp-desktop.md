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
refusal opens a window again. OS shutdown uses protected stop, then observation-only
release on failure, preserving work without an implicit interrupt. Attached nodes
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
