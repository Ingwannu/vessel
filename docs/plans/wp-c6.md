# Usage settings and conversation limit notice

Status: implementation in progress. Contract: ADR 0042 (node baseline).

## Reference and existing assets

Read Paseo 0.15.4 before implementation:

- `packages/app/src/usage/usage-section.tsx:43-91`: off, unavailable,
  loading, error/retry, empty and report-list states are distinct.
- `packages/app/src/usage/queries.ts:36-38,59-83`: cached reads do not
  force vendor I/O; explicit refresh does. Reports arrive independently.
- `packages/app/src/usage/model.ts:139-168,208-238`: a failed refresh and
  the last successful values/freshness are separate facts.
- `packages/app/src/usage/list.tsx:11-43` and `card.tsx:78-183`: a card
  per source/account, independent windows, status, plan, freshness.
- `packages/app/src/usage/window-bar.tsx:55-80` and
  `reset-line.tsx:4-39`: independent percentages and reported reset time;
  narrow layouts preserve the reset countdown before exact time.
- `packages/app/src/usage/options.tsx:38-109`: explicit check-usage
  policy and used/remaining presentation choice.

Vessel assets to extend: Settings' existing `Section::Usage` registry,
`kit::settings`, `kit::segmented`, `kit::alert`, the shared machine picker,
Fluent catalogs, typed link command/event seam and `devices::Tickets`.
The node owns reported usage and vendor limits; the app must not read
credentials, calculate prices or infer consumption from context occupancy.
The page's `web/src/data/usage/{reader,facts,calls}.ts` is the behavior
reference for rolling ledger intervals and coverage. Node ADR 0042 owns
the distinction between ledger totals and account limits.

## Decision Log

- Purpose and intent: expose reported tokens, cost and each configured
  account's independent limits in the existing Settings frame.
- Existing implementation and constraints: Usage is an unregistered
  section. The initial app baseline lacks the newer node contracts;
  centralized baseline integration has supplied the protocol 23 calls.
- Alternatives considered: duplicate the wire types locally; reconstruct
  totals from conversation history; fetch vendor APIs in the app; reuse
  existing typed node replies and the shared settings kit.
- Choice: modify existing assets. Add pure usage state/rules under
  `view/src/usage`, a bounded typed call boundary in `link/src/usage`,
  and `app/src/areas/usage` drawing shared controls.
- Why: the node already validates, scopes and retains the source data;
  only it knows whether usage was reported and costs are visible.
- Advantages, costs and effects: no new dependencies or crate edges.
  One active request plus the latest queued selection bounds navigation
  work. Request identities prevent A-to-B-to-A and closed-page replies
  from publishing. Last successful data may remain only with explicit
  refresh/failure status; a new scope clears previous values.

## Deliberate differences

- Vendor lookup defaults off, per ADR 0042, rather than Paseo's default
  on. A policy change discloses saved-login use and stays explicit.
- Ledger totals are additional data, separate from account allowances.
  Day/week mean rolling 24 hours/seven days. Missing values are not zero;
  costs stay per unit, and withheld values and coverage gaps stay visible.
- Limits are the configured saved account's facts, not proof of the
  credential an already running process remembers. No automatic account
  fallback or automatic paid continuation is implied.
- Runtime support and old-node refusals must come from actual typed calls;
  this client introduces no protocol number or invented method.

## Current implementation boundary

The shared area now owns Settings rendering and typed legacy ledger/account calls.
The separate generic source/report family remains pending on the node line; no
caller sends an invented method. Cards use the approved shared kit meter, and
policy mutation closes the old reader before the acknowledgement is read back.
Cached reads do not enable lookup or refresh vendors. The shared modal and session quota/notice are implemented and await narrow
checks. Host selection persists NodeId; display mode persists separately from
sidebar visibility. Shared connection-ready invalidates old reader publication
even on a same-node reconnect. Sidebar pins and additional sources remain in progress.

## Verification and delivery

The account inventory uses `profile.list` (node's `profiles::list` calls
`Profiles::all`, including implicit defaults); the client does not append
a second guessed default per agent. Labels never identify an account.

Tests cover stale responses, refresh failure with retained provenance,
rapid scope changes and closing/reopening while a request is pending.
Further coverage is added beside formatting and quota-notice rules.
Only in-memory fakes or the isolated fake fleet are permitted.
One whole native gate, inspected package captures and one review cover
this complete package. Results are recorded after they run.

The usage preference section is boxed within the shared preference document.
Its JSON stays unchanged; the indirection keeps `Command::SavePrefs` below the
shared enum's size threshold as the preference document gains feature sections.

### Cached clock, bounds and reset presentation

[Decision Log]
- Purpose: visible account/session/summary facts can change while another client
  explicitly refreshes them; reset labels must keep advancing without claiming
  renewed quota.
- Reuse: the existing typed Reader and limits Cached mode already provide bounded
  admission and retain original successful timestamps; no new transport or timer
  framework is needed. One shell clock uses the existing GPUI task pattern.
- Choice: every sixty seconds only selected visible surfaces reread Cached; a
  pending policy mutation or active read defers that surface. No passive tick
  selects Refresh. The same minute tick repaints floored two-unit countdowns,
  beside the independently reported absolute reset time.
- Tradeoff: this explicit native cache tick replaces React Query's stale-query
  lifecycle. It performs no credential or vendor access, even on cache misses.
  Offlines close session publishing rights; session success timestamps remain
  separate from current attempt state.
- Bounds: the broader profile API is validated before per-account calls. More
  than 256 profiles or invalid whole account labels refuse the entire usage
  inventory; no configured account is silently omitted or relabelled.

### Explicit whole-inventory refresh

InventoryRefresh distinguishes Cached, one canonical Account and All. A shared
Settings/modal action captures the selected machine and explicitly requests every
account in that machine's validated inventory. Stale callbacks cannot refresh a
new host. Passive clocks/sidebar reads always select Cached; queued requests retain
their exact refresh choice. A fake framed peer checks both whole-inventory Refresh
and subsequent cache-only reads, independently from UI rendering.

### Generic report transport and shared source pins

[Decision Log]
- Reuse: consume the node's usage_reports DTO directly; view only projects status,
  facts, source disclosure and sidebar candidates. There is no native provider
  parser or quota engine. Older peers retain their real protocol-23 saved limits.
- Boundary: method-table since() selects generic calls only after integrator
  assignment and peer support. A pending None never becomes an invented wire call.
  Explicit report refresh resolves its exact opaque id, then rereads Cached; a
  stale legacy action cannot turn into a different generic account refresh.
- Identity: Inventory is Saved or Sources, never both. Pins now use typed source ids
  shared by the registry; legacy saved-source preference keys migrate through one
  canonical source registration. Null defaults and explicit empty remain distinct.
  Equal human labels never merge accounts. Only actual Available data feeds the
  summary; retained failures and disabled sources do not.
- Consent: source actions capture host/id/disclosure, keep non-reused tickets and
  only reread Cached after acknowledgment or uncertainty. They never fetch.
  Unsupported sources have no working-looking toggle or refresh. Every new source
  describes known local candidates and fixed recipients in EN/KO without private
  paths, credentials or arbitrary endpoints.
- Rendering: the shared kit meter handles independent used/remaining percentages,
  typed source tone, reset and optional reported forecasts. Node balance units are
  preserved; Withheld hides amounts even if a malformed peer populates them.
  Original success time stays separate from current attempt/cooldown/retained state.
  Additional-source Session and F6 are still explicitly unsupported upstream;
  native UI, fixture captures, assigned-wire checks and final gates remain owed.

### Shared connection preflight (base 6)
[Decision Log]
- Purpose: queued reads, source consent and refresh must stay on the connection selected by the displayed report, even if its saved alias is rebound or the same node reconnects.
- Existing constraint: a machine book key alone cannot distinguish those admissions. The shared G4 producer now supplies `ConnectionContext` and `SelectedConnection`.
- Alternatives: another local counter or repeated alias lookup would duplicate identity ownership and could retarget intent.
- Choice: all usage reader scopes and mutation callbacks carry the full shared context; the link captures the exact selected client and uses its before-first-poll, retirement and post-await fences for every call. Contextless legacy/demo snapshots cannot authorize requests. UI action checks also compare the current Fleet context.
- Impact: a reconnect invalidates readings and pending actions, including same-node reconnects. No automatic replay. Older peers fall back to saved-login inventory only when the local protocol guard reports the new method unavailable; a remote refusal never changes destinations.
- Regression: a selected usage policy future is created, its admission retired before first polling, then a new epoch for the same node is allocated. The future refuses and the framed peer records zero mutation frames. Remote compile/test evidence remains owed for this change.

### Balance bars and risk timing
Paseo `balance-bar.tsx:24–43` derives a drawing percentage only when a positive reported allowance and used amount (or allowance minus remaining) exist. Native uses that same presentation ratio without changing independent source facts; monetary Withheld also suppresses the derived bar. A zero/missing allowance never yields a bar. Source balance tone and reset are displayed through the same kit meter/reset formatter as percentage windows.

Paseo `window-bar.tsx:50–66` requires both source forecast and shortfall for risk. A retained forecast whose time has passed falls back to reset information; the risk row deliberately excludes the reset's exact time. Pure tests cover incomplete forecasts, current versus retained-past risk, positive/missing/zero balances, explicit-used precedence and populated-but-withheld values. No quota or forecast is inferred from ledger tokens.

Pending-success presentation follows the shared node DTO correction `aa9f9e88eb15200d61d32037792f13e0a96ab83b`: a pending refresh whose immediately preceding attempt was Available keeps that observation summary-eligible and `retained=false`; pending after failure/unknown stays card-only and `retained=true`. The native tree applies only the producer's proto value delta (no node-head/runtime merge) while source/narrow review is pending. Available-origin success keeps its original timestamp. Cache rereads also keep the last successful summary visible while the transport read is pending; actual failed/unavailable source attempts still remove their pinned windows. No extra usage identity or registry is introduced.

Global lookup Off continues to leave agent-reported token accounting, cost visibility and context occupancy in their separate readers. Only saved-login/source quota lookup is disabled. The conversation limit banner remains factual typed node notice data, never a diagnosis from percentages, a forecast or a stopped-turn string, and never automatic resume/account switching.

Card source values are rendered by a separate observed-content composition, and both legacy and generic percentage rows use the same used/remaining label formatter. This keeps rendering functions bounded without duplicating the shared meter or moving any policy into GPUI.

Remote exact 78d73771 native-sources-clippy-1 is RED: view/workspace/command_changes.rs did not cover the root producer's new service_ports field. This is the pre-base-6 snapshot; the required base6/dd13 merge already supplies the shared adapter. It is not a usage compile/runtime pass. The superseding native-sources-clippy-2 snapshot includes that base and the shared connection/forecast/origin changes. A separate scoped usage view/link test run is queued. Cancelled queued native-context-clippy-1 was explicitly superseded by this changed-head suite; its rc143 is retained and not relabelled compiler evidence.

K's 2026-11-15 ruling keeps Cursor/Grok/Kimi/MiniMax/Zai on the approved Generic/Bot artwork pending the operator's decision; no new vendor bytes/API may ship. Usage cards and sidebar share the exhaustive typed built-in mapping and existing app artwork adapter/kit mark renderer. OpenCodeGo is explicitly mapped to its OpenCode publisher identity; no source-id spelling or human label selects a brand. Claude/Codex/Copilot use their existing reviewed artwork. This is an explicit approved asset boundary, not nine new artwork implementations.

Coordinator assigned the remaining arbitrary-body adaptive UsageOptions popover to W19b. Consumer reuses the existing global-policy/display/sidebar controls at width 340; no row workaround or area popup is introduced. W19b owns validated geometry, per-open lease and admitted dismissal before focus restore, desktop anchoring and the one existing compact sheet engine. Inline controls remain accessible until that V1-reviewed producer is landed; exact cog/popover geometry is still owed.

### Session-list refusal is separate from a live authenticated usage connection
[Decision Log]
- Purpose: a semantic session-list refusal must not turn an authenticated host into a false usage Offline result.
- Existing constraint: G4's shared Machine snapshot retains NodeId/epoch for semantic refusals; Reach describes the session read, not connection validity or usage authority. Contextless legacy/demo Online snapshots cannot authorize calls.
- Choice: one pure `UsageHost::of_machine` projection uses the shared ConnectionContext for availability and the actual method table for support. Settings, modal, summary, session reads and stale-action checks all use that same admission fact; the actual usage RPC still decides scopes.
- Impact: usage can display its own typed refusal/success independently of the session list. Dropping the admitted epoch makes the host disconnected even if an old Online display flag remains. No alias substitution or second connection registry is added.
- Regression: a semantic-refusal Machine with a real fixture admission remains connected for usage; removing that epoch, including with an Online flag, becomes disconnected.

### Preference compatibility uses the existing decoder
[Decision Log]
- Purpose: match Paseo preferences.ts:25–44: read legacy `pinned` only when `pins` is absent; old empty means defaults, while new explicit empty means no pins.
- Existing asset: typed PinSelection plus the shared per-field lenient serde decoder already preserve valid host/display fields independently. Serde is already a dependency; no external API is needed.
- Choice: modify that decoder with a private missing/present pin state, preserve new null/malformed/empty precedence, and accept sourceId/windowId plus displayAs/serverId aliases at the storage edge. Serialize only canonical native fields so compatibility is read once through the normal writer.
- Impact: no second preference, pin registry or transport DTO. Default/explicit-empty distinction remains unchanged and the first edit still snapshots source defaults.

### Pin edits belong to the displayed observation
A connection can stay unchanged while a cache response replaces its windows or the
usage surface closes/reopens. Pin callbacks now capture the reader's existing
successful observation ticket as well as ConnectionContext. A replacement/close
revokes that edit before it can snapshot newer defaults under an older rendered
row. A pending/failed transport read that still draws the same immutable success
keeps its ticket. This reuses the correlation owner rather than allocating a second
UI generation or capturing a separate defaults vector for every window. The pure
reader regression covers replacement, failed refresh retention and same-scope reopen.

### Shared base and remaining producer ownership
Package base 7 (`d18bb794`) is merged before the next native check. Its README
additions and this area's usage contract notes are both retained; its shared
field identity and control changes do not create a second usage policy or renderer.
The coordinator assigned the executable F6 usage adapter to W16 and the additional
seven sources' actual-launch provider/environment/sign-in proof to W22. Until their
reviewed executable producers arrive, those cases remain Unsupported. Configured
metadata and data-only launch values cannot replace that proof. Current native
and node checks are still narrow pending results, with no full package acceptance.

The legacy account observation builder and the summary test's successful fixture
are extracted before the strict consumer check to keep functions within the same
size budget. The produced element tree, source values, test sequence and assertions
are unchanged. No additional control, preference or transport call is introduced.

Package base 8 (`91407173`, including the node's `82ae7c91`) is merged before the
next native check; no node head is merged directly into the app. Narrow run
`wp-usage-native-clock-20261115T131247Z-c9ff415f` reached the view crate and returned
three diagnostics: two local style lints (identical false arms and a constant
after statements) are corrected without changing behavior. Generated
`PlainUsage::resolve` also grew to 132 lines. The shared localization owner is
handling a bounded generator structure that preserves the typed getter API;
neither generated-line packing nor a larger allowance is a valid correction.
App rendering and later crates were not compiled by that failed run, so it is
not consumer or package proof. The next changed-source check is still owed.

Run `wp-usage-native-context-20261115T125652Z-ca10b2d6` compiled view/link and
passed all six usage link tests; view passed 41 and failed two quota text
assertions. The implementation preserved Fluent's intentional FSI/PDI isolation,
while these new test literals omitted it. Only expectations are corrected to
assert the full isolated output, including nested countdown components; the other
duration check is strengthened from substring to the complete scoped label.
Product formatting and Fluent's bidi protection remain unchanged. This test
correction still needs its changed-source run; the earlier run remains red.

### Reviewed usage wording corrections
L reviewed the exact EN/KO Usage catalogs as words/data, not as package or runtime
acceptance. Apply its machine versus client-device distinction, consistent Korean
refresh/vendor wording, exact fixed-service description and the existing PARTICLE
function for source disclosure. Per-key comments state that login/recipient values
are translated store/service descriptions, never credential contents. The whole
turn-coverage label becomes `Turns reported: reported / turns`, retaining the same
typed two-argument API and avoiding the singular “1 turns” error. New whole-sentence
goldens include Fluent isolation, Korean object particles and machine/device terms.
No number format, credential permission, fetch or wire behavior changes. These
goldens and the combined catalog still require the package's compiler/runtime gate.

### Admit the usage modal's actual close before focus acknowledgment
The modal previously hid its body without acknowledging G4's shared dialog focus
cycle. A later reopen could retain its old opener, and its uncaptured close callback
could affect a reopened overlay. `ModalCycle` uses the area's existing `Tickets`
allocator, with no additional counter or control engine. Repeated show is idempotent;
close/reopen reserves a new identity. Every rendered close captures it, refuses a
stale receipt before any mutation, then invokes `DialogState::dismissed` only for
an actual close. A pure lifetime regression checks repeated show and retained old
close versus reopen. The shared footer `open_usage` signature is unchanged. Native
compilation and actual focus/capture proof remain owed for this correction.

The corrected quota expectations passed with all scoped view/link usage tests:
`wp-usage-native-context-20261115T142558Z-32393b0c` reports 49 passed, zero
failed or ignored. This snapshot predates the wording and modal lifetime deltas.
The ordinary compiler run `wp-usage-native-clock-20261115T140611Z-88084ca7`
reached the app and found three missing InteractiveElement imports and one stale
Navigation.resolve call. The consumer now uses Entries.resolve and imports the
existing element trait; no new preference or rendering mechanism is added.
Its changed-source check and the later modal/wording tests remain pending.

The early compiled sidebar-navigation dependency also carried the unlanded
`d2060f754` header-row Kit proposal. The lead confirmed it is absent from package
base 8 and requested its removal. Only that unused module, its tests and its Kit
registration are removed; the approved navigation accessor/persistence seam and
this package's shared usage meter remain. The earlier normal compiler warning
from that unlanded file is not a defect attributed to the announced app line.
Future shared control adoption comes through an announced green package base.
The same dependency brought unused route-width additions in `view::panels`.
No application consumer references them in this tree; that file is returned to
package base 8 as part of the lead's dependency cleanup. This removes no usage
API or approved sidebar-navigation preference. The original S2 commits remain
in Git history for its owner to land through the shared app line.

W19 confirms its reviewed adaptive-popover range does not add caller actions to
DialogContent's header. Paseo UsageModalFrame puts HostFilter, Refresh all and
Options beside the title. The native first version retains accessible controls in
the body at the correct shared Wide width (640), but this title-row geometry is
still a shared-producer gap. An additive header-actions seam is requested from
one assigned owner; the area must not redraw or override the shared header.

### Host picker receipts keep rendered identity
The host filter previously recomputed an index against the latest fleet. A reorder
between render and click could persist another machine. Capture each rendered
NodeId and the existing ModalCycle ticket instead. The pure admission checks that
the exact opening is current and that this node remains connected before any
preference or picker mutation. A table-like regression reorders equal-labelled
hosts, disconnects the captured node, closes/reopens the modal and verifies that
old receipts cannot select the replacement. This changes no shared picker engine
or connection authority; the selected node's real context still admits usage RPCs.

### Landed shared options popover
The announced green base `2b806402` supplies W19's one adaptive popover. Usage now
opens its cog with the shared typed icon button, a measured anchor, end alignment
and width 340. Compact presentation delegates to that control's existing sheet.
Global lookup, sidebar visibility and used/remaining controls reuse their existing
writers; they are composed once inside options rather than duplicated inline.
Global Off hides legacy external cards while the separate reported ledger/context
remains visible. Source consent still has its own exact disclosure boundary.

OptionsContext reuses ConnectionContext and the enclosing ModalCycle ticket. Each
body action checks both, separately from the Kit lease; same-node reconnect,
host change and parent close/reopen revoke old actions. Stale domain dismissal
returns KeepOpen before any shared focus acknowledgment. Automatic retirement
first removes the owner pair/body, then defer_in closes the captured lease, whose
control check preserves a newer opening. An actual parent close retires its child
before the shared dialog acknowledgment. Pure regressions cover domain changes;
W19's control tests remain producer evidence only, with consumer compile/captures
still owed. The header-actions and shared picker producers remain independent
pending line handoffs; no area-owned header or overlay is introduced.

Exact `1026f12e9` passed the batched app/view/link usage suite in
`wp-usage-native-context-20261115T151531Z-1026f12e`: app 2, link 6, view 48,
zero failures/ignored. This includes the modal/host-pick and whole-sentence wording
regressions plus real compact-row geometry. It predates this base/popover delta;
it is not strict Clippy, a whole native gate, captures or F3 acceptance.

The announced green `6ebdf8e8` supplies G2's shared picker lease and actual-close
focus API. Usage retains its host NodeId/modal-ticket admission, then uses
close_and_restore before the parent dialog acknowledgment. The new Kit lease
owns stale pointer/key/default-focus admission; it does not authorize backend work.

The conversation notice's model action and quota refresh now capture the same
SessionSelection as their data. Before dispatch they compare both the current
reader/binding and the actually open agent's full ConnectionContext and SessionId.
A route/agent switch before redraw cannot retarget an old notice to the new agent.
Pure tests cover changed agent, same-node reconnect, binding replacement, closed
reader and absent active conversation. The existing node notice remains factual
saved-account data; it still does not diagnose a stopped turn or resume prompts.
