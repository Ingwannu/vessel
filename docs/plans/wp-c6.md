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
