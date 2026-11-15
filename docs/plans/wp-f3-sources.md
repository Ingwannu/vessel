# F3 account reports and usage sources

Status: contract proposal. Extends ADR 0042; no assigned report protocol.

## Existing assets and decision

Modify the existing usage/limits infrastructure, preserving accounting and
immutable launch binding. Add a distinct report family because profile-bound
quota facts cannot express source-account discovery, report ids, arbitrary
balance units, source tone, summary flags or plugin sources. Reuse the existing
`LimitKey`, `LimitName`, `FiniteNumber`, `LimitRefresh`, `CostVisibility`,
authorization checks, cancellation/permit discipline and saved-login policy.
No new dependency is required for the contract or actor skeleton.

## Reference specification, read before implementation

Paseo 0.15.4, repository-relative paths:

- `packages/app/src/usage/types.ts:1-34`, `model.ts:107-168,208-238`:
  host/query outcomes, current versus retained observation and freshness.
- `packages/app/src/usage/preferences.ts:8-32,50-96`: source/window pins,
  null defaults versus explicit empty, first-edit default snapshot.
- `packages/app/src/usage/pinned.ts:1-99`: adaptive sidebar
  layout, available reports only and source-grouped account order.
- `packages/app/src/usage/tone.ts:4-11`, `meter.tsx:46-74`: source tone
  first; fallback danger strictly above 90, warning from 70; quiet fill.
- `packages/app/src/usage/usage-modal.tsx:23-94`, `hosts.ts:1-59`:
  connected-host filter, persisted host choice and compact sheet.
- `packages/protocol/src/messages.ts:6411-6527`: report, attempt,
  independent window/balance/detail data and streamed response contract.
- `packages/server/src/server/plugins/usage-sources/index.ts:53-110,
  222-241,299-360`: discovery, five-minute cache, per-account coalescing,
  independent fetch deadlines and successful-history retention.
- `packages/plugin/src/server/usage.ts:67-180`: source registration,
  session/global scope, safe account keys, period labels and source tone.
- `packages/cli/src/commands/usage/index.ts:75-93`: list/inspect,
  session-versus-report selector exclusivity and unsupported host.

## Structure and ownership

- `vessel-proto::usage_reports`: identifiers, bounded collections,
  report states, scope, list and inspect value types. The protocol
  constant remains `None`; no call before method-table integration.
- `vessel-harness`: extend source adapters; fixed known origins, local
  read-only sign-in handling and fixture-only schemas stay here.
- `vessel-node`: one source registry/cache actor, reusing reviewed limits
  where applicable. Plugin adapter consumes ADR 0079's source helper API;
  no second plugin invocation protocol. Native/page/CLI share the same DTOs.
- Native view/link/app: request lifetimes, cards and bar tone,
  source/window pin selection, host-filtered modal, shared sidebar slot,
  account data within the existing context meter and factual limit notice.
- CLI glue uses the same report-id discovery and inspect resolver.

## Deliberate differences and hard invariants

External lookup defaults off. Read never starts login discovery or fetch;
explicit Refresh requires Operate. No arbitrary endpoint or credential path is
accepted from a client. Monetary fields stay withheld without Operate. A quota
is not token accounting, an observed saved login is not proof of a running
process's remembered credential, and a full quota is not proof why a turn
stopped. Account labels never identify reports. Missing remains missing.
The package cannot claim source or plugin parity before its real adapter exists.

## Verification

Identifiers: boundaries, Unicode/control rejection, canonical opaque report-id
round trips and no label-based resolution. Collections: first excess rejected
without decoding its body, duplicate identities and global response size.
Actor: coalescing, cache TTL boundary, failed discovery retains prior accounts,
failed refresh preserves only same-binding success, off/disable/exit fences late
publication, cancellation retains permits until actual work finishes.
Sources: sanitized fixtures only, every parse/status/rate-limit branch, no real
credentials or vendor endpoints. Consumers: old-peer typed refusal, report-not-
found and mutually exclusive selectors. Whole native/root gates and review
remain owed until the complete package is wired.

### Executable skeleton registration

Quality B base `29eebb8b` permits the two report selectors to register in the
single method table with `(pending ACCOUNT_REPORTS_PROTOCOL)`, which remains
`None`. Client and server admission therefore refuse these methods rather than
assigning an invented protocol version. The node handlers also refuse with typed
`Unsupported` until the actual source registry is connected. This is a skeleton,
not a source implementation or a successful empty-report claim.
