# Orchestration settings (W18, ADR 0065)

The coordinator assigned both sides of F2, including tracker 594, to W18.
W15 owns onboarding and consumes this owner's typed tasks/events; its wizard
choices remain deferred until Continue. App base: 9e18cf0db, with the owned
pending node implementation merged for compilation only. No readiness is
inferred from that merge; MCP_PARITY is None and runtime calls must refuse.

[Decision Log]
- Purpose: port the host Skills install/update/uninstall/picker and tools switch.
- Existing assets: kit settings cards/switches/dialogs, settings Overview route,
  prepared Fluent words, typed node MCP/skills contracts and bounded link tasks.
- Alternative: duplicate configuration in app prefs or reuse onboarding's draft
  as persisted state. Rejected because actual state belongs to the chosen node.
- Choice: pure view state, typed owner tasks/events in link, existing kit drawing.
  The target includes book key plus returned NodeId; consumer, Visit and attempt
  correlation reject stale A-to-B-to-A answers. All calls have deadlines and a
  fixed concurrency bound. Raw node diagnostics never become screen text.
- Consequences: unsupported/disconnected machines cannot mutate. Timeout is an
  unconfirmed result, not proof of rollback. Focus reads actual state again.

Reference: Paseo packages/app/src/agent-skills/{index,use-agent-skills,
selection-sheet,legacy-migration}.tsx and screens/settings/paseo-tools-card.tsx.
The survey .area6-parts/A.md is an index; implementation files are the source.

The view skeleton reuses proto SkillName/Selection/Snapshot and McpSettings.
Picker drafts seed only on open; leaving all selects all currently available
names. First Save carries no stale removal approval. The exact fresh names
are confirmed and retried once; a second changed plan is an inline failure.
Uninstall confirms whole folders including user additions. Maintenance confirms
only non-delete ops, sorted add then update/name. Controls are disabled while
working; no selected skills disables install but leaves the gear available.

The source includes in-memory fake owner tests, the bounded link adapter,
Fluent EN/KO, host rows/dialogs and focus/identity/rejection fixtures. Actual
controller execution, shared preflight/queued-delivery integration, one whole
native gate and private captures remain pending. The root gate is separate.
No cargo/rustc executes locally under the coordinator's VM-only rule.

The link now owns bounded tasks (eight, including answer sends), one named
15-second deadline and exact welcome NodeId preflight. The Node trait fake
proves unsupported/changed targets cause zero effects. The real adapter uses
the pending MCP_PARITY and existing client methods; it never invents a wire
call or changes provider policy while toggling inject. Error text is reduced
to typed codes at the edge. Tokio test-util is enabled only in link's existing
dev dependency for paused-time deadline tests; no new dependency is added.

Existing idle-release fixtures explicitly use mcp=None after merging the
additive settings field. Their parking values and assertions are unchanged.
The controller keeps its previous node-settings call; the new orchestration
owner patches only mcp.inject, so neither owner erases the other's setting.

The host Overview now renders Tools and Skills through kit cards/switches,
with EN/KO Fluent messages. Disconnected tools are hidden; skills explain
unavailability. Unsupported capabilities are never invoked. Each drawn
button/dismiss handler captures its original Scope before dispatch, rejecting
a stale opener as well as stale replies. One adaptive dialog is reused for
picker/removal confirmation, with captured return-focus dismissal; a pending
save cannot close it. Maintenance excludes deletes; confirmation names remain
data rows rather than formatted peer diagnostics. Controls select the shared
Pointer/Touch sizing at the app edge. Native accessibility retains the kit's
known lack of a focusable/labelled button tree; no new renderer is introduced.

The shared owner Event is registered in shell dispatch. W15 must attach its
onboarding consumer to the unclaimed Consumer::Onboarding answer; no active
onboarding caller is imported or claimed wired in this tree. Existing idle
settings stay on their prior path; no copied settings storage is added.

Controller regression fixtures use Shell::without_io, the actual adaptive
selection dialog and real Escape dispatch. They exercise cancellation while
save acknowledgement is outstanding, original opener focus restoration after
accepted dismissal, stale visit dismissal/acknowledgement, and retained custom
selection across real language dispatch and input caret changes. A typed fake
answer is used without any node, home directory, or browser opener. These are
new source fixtures awaiting their exact remote test result; the earlier 9/0
view/link result does not cover the UI controller.

Base 6 (18eedad4) is merged, retaining the shared connection preflight and
new common actor fixture. Target now reexports the canonical ConnectionContext,
including its non-reused admission epoch, instead of duplicating Machine/Node
identity. The link captures SelectedConnection and every call uses its guarded
original client; retirement is checked before polling and after results. The
view rejects same-node reconnect answers as well as alias/node changes. A
retirement event drops that page and fences older queued snapshots. Window
reactivation refreshes current state without reseeding an open draft. These
changed union paths still need their exact narrow/native results.

An unconfirmed timeout or invalid acknowledgement now blocks another write
until a successful status read. Error recovery offers Refresh in the host
header and within the selection dialog, including when a previous confirmed
value is still shown. Refresh preserves the user's draft. This avoids a
misleading retry based on stale installed/injection state; no automatic retry
or rollback claim is made. A pure transition regression covers this boundary.

Availability uses the shared authenticated connection snapshot, not the result
of a session-list read. A semantic session-list refusal does not revoke an
otherwise valid Admin settings connection; the actual request still checks
its own scopes and capability. A closed transport has no admitted context.

The exact 9ad30682a UI strict batch stopped at the Fluent build contract:
individual orchestration messages lacked translator context comments (group
comments cover only the next message). Each EN/KO message now has its own
screen/action/recovery context. No compiler or runtime success is inferred
from that failed batch; it never reached the app controller check. Logs are
retained in /tmp/work/wp-mcp/app-strict-9ad3.

Dialog callbacks also capture a non-reused ticket within a visit. Closing and
reopening a picker or receiving a fresh removal plan invalidates older modal
callbacks; a host Visit alone would not distinguish those decisions. The one
retained Tickets source supplies both identities without wrapping. Modal
content changes retain the original focus-return cycle; actual dismissal
alone restores it. Host-row callbacks cannot act behind an open dialog. A
controller regression rejects a prior dialog's close and save in one visit.

Coordinator ruling for tracker598: no automatic native legacy file migration.
Vessel has never written that file, and it must never read Paseo's real Electron
profile or invent a prior Vessel path. An explicitly supplied legacy selection
can use the node owner's typed import-only-if-unset acknowledgement/AlreadySet
contract after it is reviewed and numbered. Its restart/import tests belong to
the node package; this app does not claim an automatic migration consumer.

The b819 strict run stopped before compilation because the remote helper uses
--locked and package base6's native lock lacked vessel-rpc's serde_path_to_error
entry (the coordinator had flagged that stale base). The line's committed
lock fix dd13ce378 and shared menu seam were merged; no dependency version was
selected locally and no Cargo ran on dev-host. This failure is retained and is
not an app compiler/runtime outcome. The current dialog-ticket union still
requires its actual strict and controller tests.

The exact c0c6 strict batch passed resource generation/lock resolution and
stopped in the view crate on a missing semicolon in a unit-returning block.
That is corrected, and the access-error pattern uses the exact Forbidden
variant rather than a redundant equality guard. The run did not reach the
app controller check; no UI/runtime success is inferred from it.

## Shared delivery integration hold

The current Scope/Visit/Task checks and selected.call fence do not yet prove
validity after bounded event backpressure or a queued UI result. Before a
consumer-ready handoff, the real reply must carry W12's opaque weak
DeliveryGuard and validate the exact original context immediately before
application, alongside the existing outstanding action/ticket checks. A
preflight refusal carries only a typed error and never a fabricated guard.
An admitted write whose outcome cannot be confirmed retains input and requires
a fresh read; an older binding cannot alter or retire its successor.

The link also needs the canonical zero-frame SelectedConnection::check_call
before even the older node.settings read can run against a peer missing the
shared MCP feature. That is protocol support, never an authorization verdict.
The lead currently retains the shared selected.rs/delivery-export slot after
return of the first A4 candidate. A checked straight consumer packet on the
announced line is required before slot adoption. W18's actual Skills consumer
first needs the approved portable Skills carrier through V3, the node batch and
the next app base. That carrier has no NodeSettings.mcp field: Tools read/patch
must remain Unsupported in that straight packet. No staging attribute is
removed here and no raw client escapes the link. The only actor fixture allowed
for the later delivery regression is the shared import actor's unchanged blob;
this package must not introduce a second fixture actor.
Existing UI/model tests do not claim this still-pending transport queue proof.

The exact c0c6 test batch reached the app test module and stopped on an
IntoElement import used by the Probe's return type: importing it as underscore
made the trait name unavailable. It now uses the named trait import. No UI
runtime test had started in that failed batch; the earlier pure9/0 remains
the scoped older evidence, not a controller or delivery-guard proof.


The exact d7e4 strict batch reached link: adding the owner registration crossed
State::handle's function budget (101/100), and a test helper lacked explicit
cfg(test), plus one test used get().unwrap(). Dispatch now delegates its
unchanged preference persistence body to a private module function, keeping
ordering/error handling and all limits unchanged. The helper is test-only and
the fixture uses an explicit missing-connection failure. The picker footer is
also a separate pure builder so owned drawing functions stay below the same
function budget. No size record or lint allowance is raised.


## Retired connection input retention correction

Inspection found that retiring an admission removed the whole picker and lost
its user draft. The controller now retains one volatile typed selection plus
its original machine/node identity when a connection is lost. This is input
only: no acknowledgement, pending task, visit, ticket, deletion confirmation or
installed snapshot crosses the retirement. A new admission gets a fresh visit
and must accept its own successful Skills read before reopening that input.
Another host/node or explicit page navigation discards it. Old completion and
close callbacks retain their old correlation and cannot apply to the new page.
This uses the existing Page/Picker model and no configuration store. The queued
9430 checks predate this correction; exact changed-source validation is owed.


The exact9430 strict run reached the app and returned E0700 in the two click
factories: the toolkit listener's opaque type captures its Context parameter
lifetime despite a static callback. The factories now capture the existing
weak entity directly and update it on invocation, matching the toolkit's own
listener mechanism. They retain only owned scope/intent values and the weak
entity, with no borrowed Context, lifetime extension or new allocation. Existing
scope/dialog fences and weak-owner dismissal behavior are unchanged. The failed
log is preserved; the corrected callbacks and retained-input change still need
exact-source strict and actual UI execution.


The exact9d24 strict run compiled the callback correction and returned only
four style issues plus the long Escape/ack regression. Collapsible conditions
and the unit callback now use the required syntax. Two read-only test assertion
helpers preserve every picker/focus assertion while keeping the existing test
below its function budget. No lint allowance or test expectation changed. The
skill documentation action now targets the coordinator's published repository
and the existing Agent tools heading. No opener is invoked by this correction.

## Skills-only portable-carrier preparation

The announced base 8c (`2b806402`) is merged without editing the shared selected
connection or delivery exports. The canonical portable carrier deliberately
contains no `NodeSettings.mcp` field. This private consumer now removes both
legacy settings RPC imports and the injection-patch constructor; Tools read and
both switch choices return typed Unsupported without touching the connection.
The real Skills calls remain typed. This does not enable the pending feature.

The production selected-call regression now exercises an actual Skills read
constructed before retirement and first polled afterwards. A separate framed
peer test exercises all three unavailable Tools actions and proves none is
substituted with a legacy settings call. The native-only partial-patch helper
and its test no longer describe a consumer; policy-preserving partial patch
behavior remains with the separate node settings owner and its regressions.
The safe peer-error-category assertion remains unchanged. The fixture reuses
existing `testing::state`/peer, with no second actor or shared fixture edits.

Prior `adac5123` all-target strict passed and `9d24cb2c` actual tests passed
16/0/0 (App five, Link four, View seven). These are prior-source evidence; they
do not attest this base/Skills-only change or queued DeliveryGuard behavior.
Current exact checks, canonical carrier/line adoption and the real guarded
result envelope are still required. The private branch contains root work in
progress and must never be merged wholesale into the app line: delivery is a
native-only straight packet on the checked announced base.


## Base 8e and canonical preference-save adapter

Run `wp-mcp-app-narrow-20261115T160318Z-23756a3e` failed at the link with
E0308: the merged command carries `Box<Prefs>`, while this branch's earlier
private size-preserving extraction still accepted Prefs by value. This was a
real compilation failure, before the new Tools/Skills regressions executed.
The still-queued ordinary sibling was cancelled to terminal143 under the
standing supersession rule; its assertions were not changed.

The announced base8e `82ca07a7` is now merged. Its canonical W10b
`save_prefs(Box<Prefs>)` helper replaces the obsolete local extraction, retaining
the same awaited persistence and error handling and the orchestration arm.
No second saver, lint exception, test relaxation or size record is added. The
newer base9 candidate remains unadopted until announced green. Exact replacement
strict and actual tests are required; earlier16/0 and strict0 remain scoped to
their older source. Shared selected/delivery exports remain untouched.


The exact base8e strict run reached the new shared test helper and refused its
unwraps because it lacked the explicit cfg(test) marker used by the existing
fixture helpers. The marker is now added to that helper alone, preserving its
body and all assertions with no lint suppression. No product source changes.
The already queued db89 actual-test run remains useful: its helper/body and
all test behavior are unchanged, and the missing marker is a Clippy test-context
classification issue, not a failed runtime test. It is not cancelled or
duplicated. A corrected strict receipt remains required.
