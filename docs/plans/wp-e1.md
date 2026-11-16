# E1: hosts and connections

## Scope and boundaries

Specification: the Paseo 0.15.4 area-5 inventory entries 11–36, excluding
15–20 (pairing, offers, QR, confirmation and device management belong to
G4). This is the app's machine registry, connection runtime, switching,
Overview and Connections, direct/SSH addition and routed discovery. The
generated tracker also associates earlier node lifecycle entries with
E1; that does not authorize replacing their existing node owners or
changing authentication policy. New node calls require their owner and
the shared protocol registration process.

The current rename package is a prerequisite, not completion of E1. Its
parent-release control and final consumer have bounded source/gate/
inspected-visual approval; selection remains a separate package. The field split and select-all follow the
lead's announced validated package base. An unannounced base under
validation is not consumed.

## Existing assets and first audit

| Responsibility | Existing owner/path | Gap against the survey |
|---|---|---|
| Durable machine registry | client `book::BookFile`, native link `machines` | Book is shared with CLI; one address per entry, not Paseo's independent host-profile store. Preserve private atomic edits and identity pinning. |
| Saved connection projection | view `connections::{Endpoint,Route}` | Safe endpoint labels exist; no per-route probe/latency/removal state. |
| Admission and physical client | link `nodes`, `Connected` | Reuse the client connector and G4's admitted identity/generation boundary; do not create a second transport or alias-only authority cache. |
| Machine selection | view `machines`, Settings `choices` and `nav` | Existing pickers and successful rename/forget propagation; audit every entry and stale selection against survey fallback order. |
| Overview | Settings `overview` and its `dialogs` | Rename/forget exist; color/badge, lifecycle and update need their own actual backing API. |
| Connections | Settings `connections` | Existing rows are read-only; latency and route removal deliberately absent until implemented. |
| SSH | client `NodeAddress::Ssh` | Native `nodes::connect` explicitly refuses it; reuse the existing remote/client transport owner, not shell command interpolation in an area. |

## Decision and data flow

[Decision Log]
- Purpose: complete the host flows without losing the CLI/app shared book,
  authenticated node identity or successful-edit propagation.
- Existing constraint: screens only hold safe projections; credentials and
  connectors belong to the client/link boundary. A label is not authority.
- Alternatives: another app-only host database/connector, or extending the
  existing book and client contracts through their owners.
- Choice: reuse and extend the existing contracts. Pure view state holds
  connection generations, request tickets and presentation states; link
  owns I/O and named deadlines; app draws and dispatches; FTL owns words.
- Why: a second transport/store would make pairing, CLI edits and routed
  connections disagree and could revive an obsolete authenticated client.
- Costs: route editing and SSH can require client/node changes before a
  working app action is exposed; unavailable operations are never fake
  successful rows.

Inputs enter the existing parser once. Only safe typed metadata crosses
to view/app. Results are accepted only for the current machine identity,
connection generation and request ticket. Successful rename moves local
state and retires old tickets; forget discards local state without remote
stop/delete; failed edits change neither selection nor clients. G4 owns
the shared admission retirement and its actor tests.

## Build order and evidence

1. Merge the announced validated package base; finish field move and
   select-all without changing composer hooks/registration.
2. Audit the inherited host frame against the survey and its actual
   components. Commit typed missing states and fixture coverage before
   wiring new calls. No fixture is presented as a remote success.
3. Complete direct/SSH inputs, saved routes/runtime and appearance in
   reviewable subfeature commits, preserving specialist-owned seams.
4. At package end: 1280×900/390×844 and 800×600 where layout matters,
   English/Korean, light/dark; errors, auth refusal, empty/many/long names,
   stale replies and successful/failed edits. Narrow checks per commit;
   one complete A13 native gate and one E1 review package. Security
   changes retain per-commit review. No push or live preview mutation.

Current state: audit/preparation; no new E1 skeleton or parity pass is
claimed by this document. Exact status remains in the package's commits
and the shared tracker, updated only for rows actually verified.

## First pure presentation rule

`view::hosts` adds typed host color choices and the automatic/explicit
badge policy, following the complete `hosts/appearance.ts` and its
`appearance.test.ts` local/remote/pending fixtures. Explicit Name, Icon
and Hidden choices override pending local detection. The automatic
choice waits, then hides a local machine or names a remote one. This
prevents painting a transient remote badge during local discovery.

This is an independent E1 model commit while field source remains frozen
for the lead's validated base. It does not add a store, reinterpret a
book alias as authenticated identity, paint new tokens or register an
inert control. Persistence validation, badge-list selection and the real
Settings consumer are still owed in the E1 skeleton/package. Existing
serde and standard-library assets suffice; no external dependency or
second connector is introduced.

The source's host rename edits the per-device display label, not a
connection/authority key (`HostRuntimeStore::renameHost`). The inherited
Vessel dialog instead renames the CLI book alias with its ASCII grammar;
that is a documented adaptation in the earlier machine-book work, not
proof of full host-label parity. E1 must keep a future display label
separate from the authenticated NodeId and book alias, including Unicode
labels, before claiming this complete. The new pure badge policy does
not silently change the already reviewed book-edit contract.

## Validated package base integration

Merged the announced `8d5513e4` package base (A8, no rebase). The only
manual conflicts were README prose, the shrinking size list and view
module registration. Both README responsibilities and both fork/host
modules are retained. The link root leaves the size list, and the
already extracted demo retains its lower 523-line ceiling rather than
raising it to the base's 589. Static size/boundary/token checks pass.

Registration now follows the base's extracted command/event/dispatch
modules and shell event child module. The pure host-policy commit's
local all-targets view Clippy passed; its tests travel as the immutable
`7ad95e37` through the new remote native narrow mode after the verified
waiting local test wrapper was cancelled under the coordinator exception.
That test run does not validate the combined base until its own checks
run; no combined feature or runtime pass is inferred.

## Pairing/registry ownership agreement

G4 confirmed it has no competing route-edit/upsert or identity-to-profile
store. E1 owns the one typed client-book upsert and connection-edit API;
G4 pairing consumes it after new/changed-host confirmation. Merge on the
authenticated NodeId and pinned key, preserve an existing internal book
alias, and keep display names out of identity matching. Do not add a
second registry or connector. G4 retains the existing Connected record's
authenticated identity/non-reused epoch, parsed BookEntry binding and
retirement/facts in the extracted refresh lifecycle.

The operation/result contract must distinguish ordinary book failure
(no admission retirement) from a post-pairing partial outcome. Redeeming
an invitation or writing credentials may already have happened before a
book write fails. That outcome must remain typed, retire only admission
that actually changed, and never automatically redeem the one-use
invitation again. Credential recovery and local book retry are separate
from remote pairing. The format/compatibility/security decision precedes
implementation; no route-upsert API is claimed by this agreement.

The next pure value is `hosts::Label`: nonblank Unicode display text,
separate from the ASCII book alias and NodeId. It trims the source's
JavaScript whitespace set at construction, including BOM and excluding
NEL, rather than silently substituting Rust's different trim set. A
missing legacy label can resolve to the typed NodeId for display. These
values introduce no registry, credential lookup or settings mutation.

Label narrow-check correction: the first all-targets Clippy stayed red
for a test using `NodeId::new` instead of the existing `NodeId::random`,
and a missing backtick around the type in its API documentation. The
follow-up changes only that fixture constructor and documentation; it
changes no label behavior or test assertion. New narrow checks are owed.

ADR 0097 proposes the one multi-route client-book format and mutation
contract, including explicit identity conflicts and partial credential
outcomes. Its status is proposed and V3 review is required before any
persisted-format implementation. No callable upsert API is claimed.

V3 returned the first ADR draft for two contract ambiguities: stronger
stored secure trust must not be matched by an incoming welcome-only id,
and redemption without a definitive reply must remain uncertain. The
amendment adds the two-sided evidence/target matrix, a G4-owned retained
RedemptionUncertain operation, concrete byte/count/worker/retention bounds,
nonblocking safe-file requirements and cancellation barriers. Status stays
proposed until the amendment is reviewed. The coordinator separately
approved the add-only shared `BookRouteId(NonZeroU32)` name/value boundary;
its code commit introduces no RPC, persisted-format or current consumer
change. Exact integer persistence tests are prepared, not yet passed.
