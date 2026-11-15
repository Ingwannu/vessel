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
