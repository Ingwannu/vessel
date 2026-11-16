# Session children: node contract and execution plan

## Current disposition

The provider/history node package is built on announced ed4cd137, with
SUBAGENT_RELATIONS still None. No assigned wire path, live provider or full
A4/native acceptance is implied. Durable detach and canonical role values
already landed through their separate reviewed node integrations.

- Original whole run wp-transfer-drivers-20261115T172627Z-cc7374c1 passed
  Rust2592/0/12, web tests464, E2E80/0 and forge4/0. The one package failure
  was the web parent reducer's missing exhaustive ProviderChild case.
- Correction8b5b4637 adds the explicit parent isolation case and boundary
  regression. Actual fresh agent all-target strict Clippy passed in run
  wp-transfer-drivers-narrow-20261116T003440Z-8b5b4637. The two new history
  suites now have testkit safety nets and named phases without changing
  assertions or the original five-second exchange/product probe limits.
- Run wp-transfer-ts-20261116T003440Z-8b5b4637 executed proto declaration3/0
  and web declaration/alias2/0. Its only failure was the expected stale
  proto.ts snapshot; all six exported artifact hashes and full source SHA
  were verified before applying the generated patch unchanged.
- The concurrent history request stopped before Rust in web-ci because it
  still had the head's pre-generation snapshot. That failed prerequisite
  remains recorded; it proves no Rust test result. Test execution follows
  on the committed generated union, without rerunning the obsolete source.

The ed4 merge preserves schema21, the head migration sequence and listing
metadata, all existing method rows and family fixtures. Only the two pending
provider reads are appended; generated wire declarations come from the exact
receiving union. Historical scoped checks below remain predecessor evidence.

Before final node handoff: validate the corrected generated union, finish
its changed whole-root gate and obtain complete V3 package review. Earlier
scoped durable-detach/security and value approvals do not approve provider
execution. Native keyed child resources, archive-finished and draft seeding
have separate consumer integration obligations. No real provider, credentials,
running service, permissions or deployment changed in these checks.

The package decision is ADR 0090. Native import reuses the existing ADR 0053 contract; account matching needed the
correctness repair recorded below.

## Structure

- vessel-proto/subagents: complete identities, current-parentage and durable
  detach result; provider descriptor/event/page types; one pending protocol gate.
- vessel-store/subagents plus store facade and writer registration: atomic
  first/repeat receipt, safe read/projection, one appended metadata migration.
- vessel-node/sessions/subagents and service/subagents: cancellation-safe detach
  and authoritative admission/publication; existing guard/envelope registration
  only after Quality B. Ownership uses current parentage.
- vessel-agent native decoding edges: actual child lifecycle/timeline extraction;
  unrelated main-timeline frames retain their current behaviour.
- Provider projections reuse the stored parent event stream, bounded reads and
  typed page results. No second event transport or raw JSON in domain state.

## Asset/reuse analysis

Modify existing store writer, marks publication, authenticated principal/ownership
and event-page mechanisms. New modules isolate the new responsibilities; existing
APIs do not model current delegation or provider-owned child identities. Standard
collections, serde and existing RPC/store crates suffice. No dependency adoption.

## Delivery order and evidence

1. Types and tests; no method registration while Quality B is absent.
2. Detach persistence/ownership regressions and cancellation-safe service.
3. Provider extraction/projection, with edge fixtures and no real agents.
4. Central Quality B merge, pending table rows, wire compatibility and generated
   TypeScript; one V3 node review and root gate for the complete node package.
5. Native integration consumes only a reviewed, integrated head.

Resume: Quality B merged. Validated DTOs and the durable store boundary are
staged; no detach method is exposed before authoritative node admission and
publication exist. ADR0090 architecture-only PASS is review row118. Provider
bounds/driver extraction and the complete native track remain outstanding.

## Durable metadata decision

The store appends one nullable receipt column after title origin. Receipt decode
must agree with immutable Creator; corrupt rows fail closed. Writer savepoints
serialize first/repeat/never-child decisions and only answer after transaction
commit. A lost oneshot receiver does not undo the write. Appends, archive and
reopen preserve the receipt, and a repeat retains the original timestamp.
Existing SessionInfo carriers share one optional-field mask, including nested
fleet replies. Until SUBAGENT_RELATIONS is assigned, no peer receives the field.
This stage does not authorize detach or alter live processes; node ownership
admission and cancellation-safe publication are the next required boundary.
Tests cover concurrent first/repeat writes, abandoned commit receivers, reopen,
restart, never-child refusal and receipt/creator corruption. No provider or
service process is launched by these tests.

## Own-history account correctness

Inspection found that agent.history's existing in_vessel map keyed native id
across every profile. Copies in another account could therefore be shown as an
existing session in the account being imported. The map now filters by the
resolved account, treating its default's absent and explicit canonical name as
one account. This preserves ADR 0053's account boundary; unit fixtures use two
accounts sharing the same native id and retain newest-only within each account.
No wire addition or provider process is involved in this correction.

## Node admission and publication

The existing marks/resume mutex now serializes a complete ancestry admission
with detach commit and publication. Ownership reads the authoritative receipt
from the store even when the actor still has an older snapshot. Local ancestor
walks retain MAX_DEPTH and full node+session comparisons; cross-node ancestry
is never invented. Human-only Operate policy rejects both local and delegated
agents. The registry removes only the current parent link, preserving depth,
credentials, scope grants and open connections. Detach is registered behind the
single pending SUBAGENT_RELATIONS constant; each fleet envelope propagates it.

Publication runs in the node TaskTracker even after the RPC future is dropped.
The runtime's existing metadata projection copies the committed receipt without
restarting or sending input to the actor. Resume refreshes it under the same
mutex before opening its publication gate, so an older prepared snapshot cannot
reattach the child. Existing SessionInfo readers receive it through the shared
handle metadata projection; the node's workspace projection stays unchanged.
Fake-backed tests cover PID preservation, local/cross-node complete identities,
live/stored ancestor cuts, the commit/publication interval, cancelled callers,
stale resume metadata, and registry grant/depth/credential preservation. These
are staged source/tests pending narrow execution; no live-provider claim.

## Provider collection and event boundary

[Decision Log]
- Purpose: retain actual provider tasks without mixing their replies, requests or authority into the managed parent.
- Reuse: canonical AgentEvent and parent EventRecord are reused, with the existing required_protocol/for_protocol projection; no second timeline schema, epoch or transport.
- Alternatives: a free JSON payload or a fabricated managed session would lose validation or grant unsupported controls.
- Choice: a pending ProviderChild observation wraps a validated read-only ChildActivity. Ready, permission/question controls and recursive observations are refused at construction and decode.
- Effect: ordinary main timeline/status/search/inbox projections ignore these wrappers; a dedicated child projection unwraps canonical activity. Supported wrappers still project optional inner fields for older peers.

ChildDescriptors validates count, actual escaped JSON bytes, unique ids and nesting. ChildRecords validates canonical read-only content, strict sparse parent sequence order, count and serialized byte size. Wire sequence decoding checks before retaining each additional row, ignores untrusted size hints and counts escaping without allocating a serialized copy. Missing provider parents remain missing rather than being invented. Typed coverage accompanies empty responses. These are source/tests under construction; no actual provider extraction or projection claimed yet.

## Collection check correction

Exacta511 collection check failed at the test vector's ambiguous collect type and two strict map_err_ignore lints, before product/shared-consumer completion. The vector now explicitly collects Vec<_>; intentionally sanitized bound/encoding failures use named ignored errors. No bound or assertion is weakened. No test execution is claimed from that red run.

The3530 strict proto check then found a test-only unreachable macro, a single-match fallback and a wildcard compatibility arm. The all-variant fixture now explicitly tests the real pending ProviderChild variant as Option(None), retaining all old assigned fixtures; page fallback uses if-let; compatibility uses explicit family/envelope tests without an enum wildcard. None of the limits or behavioral assertions are weakened. This local completed red result is retained; all subsequent compilation follows the operator's build-VM-only rule.

## Native child drivers and development prerequisites

The Claude router310d5d50 and its corrected retained-map budget share the existing
canonical translator per child. Codex's router uses typed native collaboration
and activity envelopes; canonical message/tool/plan decoding is reused. Native
Claude remains unselectable until W22 supplies real transport/permissions proof.
Real provider execution, historical native replay and child final streamed suffix
repair are still unverified or incomplete; none is implied by pure fixtures.

W23 explicitly authorized staging its behavior-neutral93f4e6ed/594b345d/4e9e1eb9
extractions for combined development checks. They are cherry-picked here as
6a9d976ad/3fd8bbd89/5927fe8b6; the size conflict preserves the current head's
client723 entry and removes only the two divided Codex files. The child hook is
isolated in session/notifications.rs. These are staged prerequisites, not claimed
checked/reviewed consumer-ready. The source owner retains canonical mapper edits.

### Identity bridge merge correction

The first combined5718 strict run found the isolated identity bridge's module
registration had been appended in addition to the existing A4 module registration.
Only that duplicate registration is removed; the single canonical identity.rs
export and full subagents module remain. The exact isolated bridge0560 itself
passed strict proto Clippy. No runtime or wire behavior is changed by the repair.

## Combined fake-backed execution and TS classification correction

Exact b4ad1de3 combined agent/node library run
wp-transfer-combined-tests-20261115T124446Z-b4ad1de3 executed393 passed,
0 failed,2 ignored (agent120/node273). Actual Claude/Codex child routing,
provider projection and detach publication/authority fixtures ran. This is
predecessor source execution, not current merged-head or real-provider proof.
The current123121dd five-crate strict run returned one remaining semicolon
lint in the Codex collaboration arm. The earlier TS run returned the primitive
alias inventory before generation, identifying three unclassified values.

[Decision Log]
- Purpose: retain one protocol-to-TypeScript classification and real generated
  declarations, rather than hand-writing wire types in consumers.
- Existing policy: ts/aliases.rs brands provider-minted handles, while content
  and quantities remain plain; every generator primitive must be classified.
- Choice: ProviderChildId is a Handle, ProviderText Content, ChildLimit Quantity.
  Runtime parsing/bounds are unchanged. The method-arm semicolon and explicitly
  discarded deque/first-boundary return values only correct strict warnings.
- Impact: generated declarations still require the normal TS step and freshness
  evidence. No wire feature number or runtime/provider readiness is assigned.

The subsequent exactbdbd7581 five-crate all-target strict run returned at the
agent integration test's exhaustive diagnostic event-name match and two node
strict warnings. The diagnostic now labels ProviderChild explicitly without
flattening child content. The scan deadline uses checked_add, refusing with the
same bounded-read Unavailable outcome if the clock cannot represent the budget;
the existing test deadline uses the checked operation as well. The remaining
assignment block ends with a semicolon. An exhaustive consumer scan found the
CLI/session/store/conversation masks already handle ProviderChild; no wildcard
or lint allowance was added. Corrected-source strict remains owed.

The first combined pure proto/store run atbdbd7581 executed Proto331 passed/1
failed, stopping before Store. The one failure pins the forwardable method set:
its explicit expected list lacked the three new session methods already declared
forwardable by the canonical table. The list now adds session.detach,
session.provider_children and session.provider_timeline, preserving every old
entry and the exact assertion. Runtime forwarding/admission policy is unchanged;
unassigned protocol methods are still refused before transport polling.

## Generated wire snapshot and named base alias defect

Run wp-transfer-ts-20261115T142826Z-bdbd7581 completed both declaration
generators: proto3/0 and the web declaration test passed. The separately selected
web primitive-alias inventory failed on the baseline LimitPercent entry.
The integrator confirmed canonical correction9dfdf0a1: LimitPercent aliases
FiniteNumber, not a primitive; removing the unused metadata changes no generated
TS. The worker does not duplicate it and waits for the announced checked head.

All six exported artifact SHA256 checks passed, and only the generated proto.ts
patch was applied. Its SHA256 is
93a3734287407ea54bbde419ae3f7c507307c7b755a9cf6da5b361bb5880ec45.
The current protocol/generator source differs from that run only in the
non-generator forwardable-set test's expected strings. Generated declarations
were not hand-edited. Overall TS step remains RED with the named base defect;
exact current-head freshness and the final gate remain required after adoption.

The exactedb all-target strict run reached the external native-history process
test and returned one get_unwrap lint. A whole changed-file scan found the same
pattern twice in the typed history test and an unchecked fixture request-id add.
All use validated test indexing after the unchanged two-entry assertion, and a
checked request-id increment. No production code or test expectations change.
The generated snapshot intentionally retains its generator's trailing spaces;
Rust/docs whitespace checks are separate from that immutable generated artifact.

## Announced gate50c base merge

The worker merged announced9c150c6efbff583762d02e6a8576b3fa66cc7ae9 (all18
steps0, Rust2549/0/12), preserving its canonical protocol/store definitions.
The alias table merged cleanly: canonical9dfdf0a1 is present and the three own
provider primitive classifications remain once each. No independent alias fix
or protocol/schema assignment was made.

Registration conflicts retain the head's complete method/forwardable/web set,
then append the two provider read methods and their typed unsupported fake rows.
The existing detach row remains once; subagents module and ProviderChildRef have
one canonical registration. No ancestry/admission/store algorithm needed manual
resolution. The head's generated proto.ts is retained byte-for-byte rather than
hand-merging generated text; the new union requires its own generator snapshot
and narrow checks before the final whole root gate.

## Verified post-base generated union

Run wp-transfer-ts-20261115T162413Z-477ae375 completed the proto declaration
selection3/0 and web declaration/alias selection2/0. The prior LimitPercent base
defect is absent after canonical9dfdf0a1. Overall rc1 now reports only the stale
proto.ts snapshot (188 insertions/4 deletions), not a failed generator/test.
All six artifact hashes and exact source commit were verified. The patch applied
cleanly after the1c762ccf base merge, whose announced gate reports wire diff0;
its proto.ts bytes match the recovered snapshot SHA256
dbed8f1c7af0e02c58ba791f3d54383ece769a6d52c4b24711c676431eed4f73.
The generated whitespace is retained unchanged. Current-head freshness and all
shared consumers remain the final whole root gate's responsibility.


## Whole-gate correction and current node union

The first package whole gate, wp-transfer-drivers-20261115T172627Z-cc7374c1,
finished with Rust2592/0/12, web464, E2E80/0 and forge4/0. Its sole failure
was the web reducer's exhaustive switch missing ProviderChild. This is a
package defect, not the separately recorded base E2E intermittent.

[Decision Log]
- Purpose: preserve the managed parent's timeline while retaining separate
  provider activity in its durable log.
- Existing assets: the web reducer already advances the parent cursor before
  applying an AgentEvent; native projection and node status/search already
  isolate child observations.
- Alternatives: flatten child content into the parent (incorrect ownership), or
  add a second web timeline (outside this package's native child UI contract).
- Choice: explicitly ignore ProviderChild in the existing parent reducer.
  A reducer regression covers support/descriptor/activity/gap, unchanged
  parent conversation/plan/attention/turn, cursor advancement, replay and the
  parent's own subsequent turn completion. No new dependency is needed.
- Test safety: the two new Codex history suites reuse vessel_testkit::within
  after fake prebuild, plus named exchange/process phases. The original
  five-second exchange and product probe limits remain unchanged. A separate
  process phase includes both existing bounded cleanup waits; the outer
  safety net does not stand in for that phase or loosen a product limit.

The ed4cd137 merge retains the head's schema21, migration sequence module,
listing projection, event-page masking and existing method rows. Only the two
pending provider read rows are appended. The generated wire snapshot is taken
from the head until the receiving union's generator supplies its exact bytes;
it is never manually merged. SUBAGENT_RELATIONS stays unassigned.
