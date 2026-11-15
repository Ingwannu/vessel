# Managed local node, onboarding and quit lifecycle

This is the current W15 implementation/acceptance plan. Desktop windows, menus,
OS navigation, CLI installation, update presentation and changelog belong to the
separate desktop-shell package. Historical onboarding proposals are superseded by
the desktop amendment in ADR 0071 and the contracts below.

## Reference and existing assets

Paseo 0.15.4 is the behavioral reference:

- `packages/app/src/onboarding/internal/model.ts:1-187`: connected/hydrated host,
  one decision per launch, five steps and stale-visit rejection.
- `onboarding/internal/use-setup-host.ts`: prefer the managed desktop host, then
  the first saved host; retain the selected host for this flow.
- `onboarding/internal/orchestration-setup.ts:16-144`: choices initially on, offer
  supported effects only, apply on Continue, advance after every offered success.
- `onboarding-modal.tsx:78-84,160-199`: close/Skip complete, Back/rail navigation,
  project checked-add acknowledgement and deferred orchestration effects.
- `project-suggestions/internal/suggestions.ts:5-6,52-56` and
  `use-project-suggestions.ts`: history preselection and attempted-set settlement.
- `packages/desktop/src/daemon/daemon-manager.ts`: owned launch, authenticated
  readiness, thirty-second start, ownership-aware stop/restart and version drift.
- `daemon/quit-lifecycle.ts`: transport shutdown, owned-daemon policy, quitting
  presentation, signal routing and quit-time app update installation.
- `desktop/updates/desktop-app-updater.ts`: the actual update/restart boundary.

[Decision Log]
- Selection: modify existing assets; new pure orchestration models only where an
  existing settings screen has no client-local setup/visit semantics.
- Reuse: Settings theme/provider/project/device owners, canonical preferences,
  kit dialogs/buttons, existing signed updater, NodeClient typed RPCs and Proc's
  owned process primitives. No second theme, book, installer or document store.
- Alternatives rejected: Electron/Tauri, a wizard dependency, shell commands for
  lifecycle, arbitrary PID signals and independent per-window controllers.
- Why: preserve one authority for each state and the established GPUI layers.
- Cost: owner seams must land together before product readiness is claimed.

## Layers and authority

| Layer | Responsibility |
| --- | --- |
| `view::onboarding` | completion/visibility, fixed layout, monotonic Visit, deferred effects |
| `view::local_node` | preferences, evidence-backed status and safe refusal values |
| `vessel-local-node` | exact packaged launch, authentication, Owned/Attached distinction, stop/restart/release |
| `link::local_node` | one bounded actor/runtime, immutable snapshots, source correlation, exact acknowledgements |
| `app::areas::onboarding` | reuse settings owners under fixed Context + Visit |
| `app::quit` | one read-only C5 admission hook and one app-wide finalization phase |
| `app::local_node` | process-wide ownership, normal quit/update consumer, OS backstop |
| `desktop::persistence` | desktop-shell owner's canonical preferences/geometry receipt and current-state fence |

The root library's graph and Proc release policy are specified in
[desktop-local-node.md](desktop-local-node.md) and ADR 0001. It depends on
client/proto/store/proc, with Store facade access only in node_files and Proc only
in launch. No node/session/runtime dependency or new wire call exists. Node's dev
edge is only for real-binary integration tests through public controller API;
there is no production reverse edge or controller test hook.

Only the exact owned Child grants process authority. Authenticated reuse is
Attached; neither token, NodeId, PID, socket closure nor a readiness line grants
ownership. Keep consumes that authority into direct-child observation. The
checked exit receipt proves leader reap and the original accepted group/job
cleanup request/already-absent result, not observation of every descendant dying.
Windows releasable jobs can survive abrupt launcher death even before explicit
release; ordinary Proc spawn retains its previous crash policy.

## Onboarding contract

The five steps are Appearance, Providers, Projects, Orchestration and Phone.
Wide/Compact layout is fixed at opening; Compact omits Phone. Native mobile does
not open this walkthrough. No extra Machine step exists: no-host pairing remains
its separate route. Completion is client-local `Prefs.onboarding`; an invalid
completion record alone does not discard other preferences. Unreadable prefs
suppress automatic setup and are never overwritten.

Automatic setup waits for the selected authenticated host and authoritative,
settled project inventory. Existing projects suppress it once per launch. An
opened flow stays open if a project appears later. Skip, close and Finish persist
completion; unfinished setup begins again at Appearance after relaunch. General's
Show setup again reopens explicitly. Origin route/section/host is restored when
the walkthrough closes.

Target carries the canonical ConnectionContext, including physical admission,
plus a monotonic Visit for each stay. Alias/NodeId equality cannot validate a
same-node reconnect. Each effect also retains its original operation ticket.
Resolve/Bind success must eventually carry the canonical weak DeliveryGuard;
queued installation and every subsequent Ready query validate that exact guard,
Context and Visit with no await before mutation. The current context-only adapter
is preparatory until the real shared export/consumer union lands.

C1 owns authoritative Existing/Empty/readiness and controlled project forms. Its
withdrawn values carrier is not an API or runtime handoff; no alternate identity,
wire row or legacy Settings fallback substitutes for its checked producer. A raw
ProjectsView::Ready cache is not authoritative across physical retirement.

Suggestion settlement is separate from manual-form readiness. Preselection is at
least three sessions in thirty days with one fixed flow time. Continue captures
an attempted root set and succeeds only when all attempted additions succeeded
and the current checked roots contain no root outside that attempted set.
Successful additions legitimately remove rows; exact revision equality would
incorrectly reject those removals. Late provider counts can introduce a newly
checked root, which must prevent advance. Target + Visit + live original guard
remain required. No checked-suggestion UI/acknowledgement is implemented yet.

Orchestration changes apply only on Continue and advance after exact owner
acknowledgements. CLI uses the desktop-shell installation API, never a copied
PATH/service policy. An authorized CLI mutation invalidates earlier status-query
tickets; reopening/refresh does not start a query while that mutation is running.
A delayed pre-mutation snapshot cannot replace its exact operation acknowledgement. MCP/skills remain unavailable until their numbered/reviewed
owner APIs exist. Unsupported effects are not claimed installed. A foreign CLI destination offers
neither Install nor Remove: onboarding uses the owner State::allows admission and
omits that unsupported choice instead of trapping Continue on a refused overwrite. Shared footer
controls use Outline Skip, Secondary Back and Primary Continue/Finish with kit
pointer/touch metrics. Setup retains its 880×640 desktop bounds/viewport cap and
compact behavior; it is not silently replaced by a percentage-only dialog size.

## Managed node adapter

ADR 0094 resolves the binary through the single desktop bundle getter:
parent/bin/vesseld (vesseld.exe on Windows). No PATH search, shell/helper fallback
or downloaded daemon is a launch source. Missing companions display NotInstalled;
macOS helper layout is unsupported until the release owner supplies signing
layout. The private home is config/vessel/managed-node. Resolved shell overrides
are child-only, and the updater's RESTARTED_FROM marker is explicitly removed.
No process-global environment mutation occurs.

Snapshots expose authenticated provenance, status and bounded explicit log reads.
Manual destructive intent captures a sealed ControlLease before yielding. The
actor rechecks its exact permit and Owned node before effect, including when a
replacement keeps the same NodeId. Interruption approval captures the failed
operation's immutable Snapshot/actual live-session list; it cannot be regenerated
from a later observation. Cancel preserves the process. Generic failures never
offer implicit interruption. Pause stops only Owned; Attached/Kept/Stopped are
not sent an unauthorized stop.

The obsolete branch-only RawConnected registration is removed with lead approval.
Until the canonical third writer slot, Some(connection) explicitly returns
Unavailable, withdrawal acknowledges None, and Fleet publishes no local machine.
This incomplete difference from Paseo is visible/hidden as unavailable, never a
fake BookEntry or a second epoch. If the skeleton lands on the line it requires
V1 review; it is not parity completion.

The future canonical contract is in
[desktop-managed-connections.md](desktop-managed-connections.md) and
[desktop-managed-admission-order.md](desktop-managed-admission-order.md). Shared
connections/selected/delivery files remain held until a real production
DeliveryGuard export with an actual consumer and W12's actual follow promotion
are green on the line, followed by an explicit lead grant. Staged exports do not
unlock it. The earlier duplicate native alias grammar is removed; future wrappers
only delegate to proto::book_values::ManagedLocalAlias on an announced base.

## Ordinary quit, update and manual controls

One global Idle/Finalizing phase coalesces lifecycle work. C5's read-only guard
reports Ready/Dirty/Pending/Failed over all active/retained buffers and original
write tickets. Dirty, Pending and Failed refuse; no implicit Save All or Discard
is introduced. New edit/write intent is refused before effect while finalizing,
but already accepted edit mirrors and original write replies still settle. The
same owner guards additional/macOS window close after the initiating Shell borrow
ends. Finalization is projected temporarily into the existing frame foot without
replacing retained notices; onboarding hides/disables new host-effect admission
while this phase is active, and original operation acknowledgements still settle. No second window/document registry or recovery journal exists.

Manual Stop/Restart/confirmed interrupts enter this same phase, query every live
Shell for the sealed intent's node, then enqueue the controller request. NodeId
is only a facts filter; ControlLease separately grants authority. C5's node-filter
setter stays literal cfg(test) until its actual reducer registration promotes it;
production conservatively falls back to the global guard. Start/Reconnect is not
made destructive or used to clear an old uncertain write. Global phase release
does not depend on the origin window surviving.

[Decision Log]
- Choice: consume the desktop-shell owner's StateReceipt/current-state fence,
  not another saver. It checks accepted-publication identity, canonical revision,
  every live Client settled/no refusal, and geometry value/publication identity.
- Why: old successful writes and A→B→A bounds equality cannot authorize exit while
  another publication remains in flight. LaunchOnly cannot bypass pending edits.
- Bound: each persistence phase has four attempts and ten seconds total, using
  the same GPUI executor clock/timer. Changed yields briefly before a fresh
  receipt; exhaustion/write failure/deadline asks save/retry and keeps the app.
- Ordering: first settle state/policy/documents; finalize Owned exactly once;
  use a fresh post-node persistence budget. Retrying Changed never repeats node
  stop/release. The final current-state fence, document admission and actual
  quit/updater.restart run in one App update with no subsequent await.
- Cost: a later veto/restart failure leaves the writer alive and restores editing.

The root library API and Proc release join must first land through their reviewed
node package and announced app base. Preparatory root ancestry in this worktree
is not a substitute; app-only checks cannot approve that root join.

Paseo installs downloaded updates on quit within five seconds. Vessel's existing
signed updater verifies/replaces its artifact before Ready; restart selects that
installed set. No second quit-time installer is invented. Ordinary quit honors
explicit Keep; Restart to update always finalizes Owned first. Attached nodes
stay untouched. App release labels are never compared with node crate versions;
handshake protocol range remains Attached compatibility evidence.

## Irreversible OS exit and reaping

GPUI 0.3.10's non-vetoable on_app_quit cancels observers after 200ms. A fifteen-
second protected stop cannot live inside that callback. Before any node launch,
the controller creates a bounded synchronous supervisor channel/thread owning its
existing reactor. The OS callback synchronously hands off Preserve; later
Controller Drop's Ordinary message cannot override it. No saved policy changes.

PreserveExit closes actor command admission, releases current Owned without an
interrupt confirmation, and acknowledges only after current and historical
released direct children are observed/reaped. Explicit Kept→Attached reauth never
recovers ownership or loses the earlier observation: the actor retains at most
eight observation tasks, prunes completed ones and refuses further reconnect at
capacity before launch. Those tasks retain no source/signal authority. Failed or
aborted observations and lost release acknowledgements retain the reactor rather
than claim exit or fall into implicit Owned Drop cleanup.

This promises neither release within 200ms nor protection against actual OS
process death. Releasable-launch hard-crash semantics still apply. Preferences
and geometry use the same owner facade only as best effort in the OS observer;
unsaved editor text has no forced-exit journal. Windows/macOS actual behavior
remains unverified.

## Evidence and remaining acceptance

| Exact source / run | Scoped result |
| --- | --- |
| root54b9abcf / wp-desktop-node-20261115T115618Z-54b9abcf | A12 all18 steps0; Rust2404/0/12, web464, e2e68, forge4; predecessor ancestry, not portable landing |
| root54 / focused115154 | 70/0: local-node3, Proc51, launcher-exit4, spawn/carrier/detach12 |
| nativeViewde695b17 / view-tests115422 | 967/0; context/visit model, included later-removed draft grammar |
| shutdownaaeabae1 / shutdown123843 | synthetic supervisor/reactor2/0; not actual OS/node integration |
| controlaf4545c2 / control-tests130429 | actual App79/0/2, including generic quit and deferred-window guards |
| control48eb2bcc / current-narrow134404 | strict RED: two uncalled old E3 setters and unnecessary CLI Option; no blanket suppression; owner-approved later cleanup |
| currentd2f477b2 / link-current145243 | strict RED at Link: obsolete test State field, needless context borrow, let-and-return; App not reached; corrected source queued separately |

Root docs-only095f38c23 records the approved integration-test dev edge; prepared
3d2128ae5 adds real Keep→Attached authority regression. Those additions do not
borrow runtime proof from root54. The root must be rebuilt on an announced head
containing portable b37 checked-wait, not land the earlier 622 ancestry.

The desktop-shell source552 receipt packet, later b0cef test-only portable
waiter and source15ffa serial geometry checkpoint fix are provisionally consumed.
The writer fix retains newer accepted geometry when an older captured flush is
enqueued late, including after debounce; explicit Final still forces its supplied
snapshot and closes before acknowledgement. The immediate App currentness fence
remains mandatory. Its deterministic late-enqueue regression is not yet executed. Existing merge/layout behavior and this
package's own fields remain intact. The wait fixture does not pump GPUI foreground
observers, preserving its receipt-before-observe race. Producer and actual consumer
union execution, retry/deadline tests, C5 registration and retained-buffer barriers
are still owed.

All Rust builds/tests and boundary/quality (including Cargo metadata) run on the
build VM. Only verified Cargo-free formatting/static size/token checks remain
local. No final whole native gate, inspected desktop/compact bilingual/theme
captures, V4 product approval or Windows/macOS execution is claimed. The package
tracker remains skeleton until those actual owner/consumer boundaries are done.
