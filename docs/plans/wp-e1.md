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


ADR0097 amendment001d9673 received V3 CONTRACT PASS after independently
checked original fast outputs at9a173ca0. This closes the docs-contract
review only. No format, credential writer, runtime upsert or prepared
BookRouteId implementation is approved. The new direct rustix dependency
must be authorized/registered before its safe-file code is introduced.
Package base3 6c782cb6 and base4 3cd0d3fc were merged without rebase;
base4 brings the reviewed eight-theme and L Diagnostics/Chat adapters.
Own route/appearance models and the input slice remain separate work.


## Private file foundation preparation

The separate E1 worktree starts from the immutable field/capture source
970563a2b; it does not alter the tree whose whole native gate/export is
queued. The approved cfg(unix) workspace rustix/fs dependency prepares
one client-owned file boundary, with matching root/native lock references
and architecture/client ownership docs. No local Cargo runs under the
09:2x core-capacity rule.

The first implementation is deliberately private Unix test staging:
finite input before UTF-8 decoding, directory-relative NOFOLLOW/NONBLOCK,
opened regular-file/anchor-owner checks, fixed safe error chains and
single-component names. Tests prepare exact/one-over byte boundaries,
Unicode byte counts, invalid UTF-8 canaries through every error source,
finite reader requests, directory replacement, symlink/directory/traversal,
missing files, metadata limits and a FIFO with a named safety-net deadline.
It is NOT wired to the legacy readers, writer, lock or worker/retention
owner. No Windows/new upsert or production file-safety API is claimed.
Actual narrow checks and V3 code review are still owed before integration;
source preparation and direct rustfmt are not a compile/test pass.

Private staging also distinguishes an existing read-only shared book from
an exposed credential: refuse group/other-writable books and anchors;
credentials require owner-read with no group/other permissions. Checks use
the opened handle and anchor owner, following the existing store pattern;
no current-process identity lookup or chmod of existing paths is implied.
The eighth prepared test checks actual temporary modes and refusal without
changing them. This remains uncompiled test staging, not production safety.

## Shared local appearance value move

Coordinator approval moves the existing pure Label/EmptyLabel, Color and
BadgePreference to proto::book_values. The view keeps its old re-exports
and all seven existing host/label tests byte-identical; locality-dependent
resolution remains in the view. Root producer tests cover the full source
trim character set, Unicode/serde/fallback and existing color/badge order.
The old label implementation is moved unchanged except for its NodeId
import, without a new dependency, RPC/version/TS or persistence operation.
The ADR0097 4096-byte UTF-8 cap remains a future file-edge visible refusal,
never a hidden label truncation. Compile/tests and V3 quick source review
are owed before the integrator/B0 can land this shared producer.

The private reader keeps its one-byte EOF probe on the stack rather than
reserving cap+1 encoded bytes. This prevents two maximum preimage/output
pairs from exceeding ADR0097's exact4MiB encoded-document budget by two
bytes, and avoids read_to_end growing a full vector to probe EOF. It pays
one bounded zero-fill per rare file read. The ninth prepared regression
checks empty/exact/one-over reads and retained capacity. Actual tests,
worker aggregate admission, writer/retention and production wiring remain
owed; this is not a total-heap or cancellation-runtime claim.

V3 independently SOURCE-passed only the ccedd4860 appearance-value move.
Its3 root producer tests and7 unchanged native consumers still require
execution before integration; private reader staging is outside that pass.


## Resume: shared values and fixed-lock consumer source

The Unix private-file code remains private cfg(test) staging. It has not
been compiled or used by a production book or credential operation. The
bounded EOF probe is outside the document buffer; worker admission,
transaction locking, safe replacement, partial-credential retention and
Windows refusal are still unimplemented. No all-writer safety claim is made.

The shared public values are isolated on the app base18eedad4 as the exact
range ending7becce7f. V3 accepted source through e3764115 (route identity,
unchanged appearance values and the precise ManagedLocalAlias grammar).
The root all-targets Clippy run122609Z completed with rc0: actual
Checking vessel-proto and Finished in106s were independently inspected
by V3. Nine new root tests and seven native compatibility assertions
still need execution; this is not a full consumer-ready handoff. The
coordinator corrected the initial routing instruction: only the five
root/docs commits are cherry-picked onto the node line, never app ancestry.
The app receives the checked values through an announced base merge.

This private development branch merged the fixed-lock selection source
0610958f as744a383e. It includes the announced dd13ce37 base and the
owner's isolated deterministic find-work correction. The lock merge was
automatic and preserves the approved client rustix edge. No check on this
combined E1 source has run. The native input package remains in its own
tree and must receive its gate/export/captures independently. Preserve
those receipts rather than attributing a later source pass to this branch.

Next: finish the active shared-value check, execute its root tests, then
check the exact native re-export consumer and private client staging.
Only after the actual security checks/review should the new book parser,
authority-binding projection and transaction path replace existing APIs.
There is no callable multi-route/upsert, secure evidence receipt, direct
or SSH addition flow yet. No production permissions or services changed.

## Checked route collection preparation

Reference reread: the complete Paseo `types/host-connection.ts`, including
the host/connection variants, equality and pinned-key propagation, the
upsert at lines297–373, and stored-profile normalization. Paseo preserves
a usable preference while combining matching routes. ADR0097 deliberately
refuses endpoint-only identity transfer and invalid saved references instead
of silently repairing/dropping entries. Those differences remain unchanged.

[Decision Log]
- Purpose: prepare the route-list invariants before any new file decoder or
  mutation can publish a multi-route profile.
- Existing assets: `BookRouteId`, `NodeUrl`, `SshTarget`, `secure::Route`,
  and existing token/credential file references. No external dependency is
  necessary and no new address parser is introduced.
- Choice: an additive immutable `book::Connections` value checks nonempty
  input, the64-route cap, duplicate ids/specifications, the preferred
  reference, retained allocation counter and shared secure key/credential
  binding. It preserves file order and retains a saturated counter.
- Why: one pure boundary keeps later parse/edit paths from repairing or
  truncating a profile differently. Configuration is never secure-channel
  proof, live connection authority or permission to fail over to a weaker
  transport. The future parser still must bound allocation before decoding
  its input vector and validate the host's NodeId/global book limits.
- Cost and scope: no file decoder/serializer, allocation/mutation routine,
  legacy expansion, endpoint probe, upsert or UI is wired by this value.
  Mixed configured transport classes still require the ADR's explicit
  selection/evidence/confirmation rules at their owning boundaries.

Four prepared regressions cover exact/one-over route budgets and order,
duplicate ids/specifications, invalid preferences/counters, retained
exhaustion, secure key/file disagreements and complete safe diagnostic
chains. They have NOT been compiled or executed. Direct rustfmt, size and
whitespace checks are source checks only. The current separate native
selection gate stays frozen; a later client narrow suite must execute
these four tests and the nine private-file staging tests before review.

The next pure adapter is `Connections::try_from(&BookEntry)`. Websocket
token references and the complete existing SSH target survive unchanged.
Secure expansion assigns deterministic1-based ids in direct-then-relay
order and checks the same64-route cap before allocating another vector.
It refuses inconsistent fields, missing credentials, empty routes and
duplicate specifications without changing the old entry. Node identity
remains the host's separate configured field; conversion proves no peer.
Four additional prepared tests cover stable expansion, exact/one-over
counts, complete websocket/SSH bindings and intact failure outcomes.
No legacy reader or file writer invokes this conversion, so no read-time
migration or stored-format compatibility pass is implied. All eight
route-model/adapter tests still need actual compilation and execution.

Pairing receipt boundary confirmed with the owner: the current generic
`secure::pair` issues a credential before its later hello can fail. The
new receipt path must use the actual encrypted `pair_over` preamble result
without adding a hello that discards that issued credential. G4's retained
operation owns IssuedInMemory before any later authenticated probe; probe
failure retains the credential, and local retry never redeems again.
No SDK receipt, changed preamble or secure-wrapper code is implemented yet.


### Shared producer integration receipt still pending

The strict run is settings2-narrow-20261115T122609Z-7becce7f, exact source
7becce7f4c16bb9f620d432b858a2707ea1b57b8. Its cargo log SHA begins
d646dcac293a0df0; spec, steps and original output are preserved. Clippy
compiled all targets but did not execute unit tests. V3 row214 closed
only this strict-check debt, retaining source acceptance and test debt.

The node integrator picked the five source commits, in order, as
 a58e08bc, c1717a02, d9bbb8c0,390fc201,c75d6391. It reports all code,
manifest hunks and producer plan byte-identical, a single toml_edit
reference on its own lock, and both sets of ownership documentation
retained. Its whole A12 gate47b on c75d6391 is running; actual nine-test
results, whole-gate result and announced app base remain pending.
No source from that unannounced batch is merged into this development
branch or the frozen selection gate. ManagedLocal consumers remain
preparation-only. The native re-export compatibility assertions are still
separate; node tests cannot establish native consumer/runtime approval.

### Remaining inherited dialog differences

Reread the complete `components/rename-modal.tsx` and its actual
`HostRenameButton` caller in `screens/settings/host-appearance-section.tsx`.
The host caller passes no custom validator: unchanged/nonblank/pending
button gating matches that call, while Vessel's existing ASCII book-name
validation is a documented adaptation until labels are separate.
Two existing drawing differences still belong to the later Overview
redraw: Paseo changes the submit label to its saving sentence while
pending, whereas the current native dialog retains the rename label;
Paseo's error text uses its base size, whereas the inherited native error
uses Type::Meta. The selection/profile subfeature does not fix these or
claim complete host-dialog parity. Preserve them in the E1 remaining-work
list rather than interpreting pending-input evidence as a full UI pass.
