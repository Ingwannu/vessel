# Session children: node contract and execution plan

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
