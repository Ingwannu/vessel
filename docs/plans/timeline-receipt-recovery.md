# Receipt recovery contract and staged values

This is a contract checkpoint for ADR0084 and ADR0033 row618. It is not a new
method registration, producer capability or permission to consume moving runtime.

## Existing guarantees and gaps

SubmitPrompt binds original content, delivery, authenticated Creator and one
ClientMessageId. The receipt owner commits Pending before dispatch and never
evicts replay protection. Queued acceptance retains the actual QueueEntryId.
Started/Steered currently do not retain an actual turn sequence, and
InterruptedAndStarted retains only the interrupted turn. A canonical echo can
reconcile a displayed row, but a history scan cannot recover a pruned binding.
The existing actor TurnTicket is the authoritative observation seam: it handles
fast completion, steering into the held turn and queue advancement. Never derive
the next turn from counters, content or a later unrelated event.

Calling SubmitPrompt again is read-or-admit. It is not a read-only observation
when no retained key is found. A browser with transport uncertainty and a durable
finish watcher must not disguise that possibility as a receipt query. The node
still requires the existing finite Pending/no-eviction floor; a separate read-only
observation also avoids effects when a query sees no row yet.

## Proposed owned amendments

1. Retain the actual actor-observed start EventSeq for direct/joined acceptance,
   alongside the existing exact queue entry for queued acceptance. Reuse the
   existing TurnTicket observation, including its real queue-entry transition;
   never introduce a guessed turn or make CommandAccepted start one.
2. Add a narrowly typed read-only receipt observation through the same owner,
   using the original request/fingerprint and authenticated Creator. The sole
   TIMELINE_PARITY remains None; any new method/number needs integrator/coordinator
   disposition before registration. A missing row is Unobserved, not proof of
   no effects and not permission for automatic replay/fresh-id fallback. An
   observation creates no reservation, queue work or provider work. Accepted,
   retained no-effect refusal, pending uncertainty and fingerprint conflict stay
   distinct. No second receipt store is proposed.
3. Internal finish notices require a trusted namespace distinct from public user
   submissions. Creator in the existing fingerprint prevents reinterpretation
   but does not provide that namespace. Public requests cannot choose an internal
   namespace or supply internal authority. W18 first specifies its bounded
   NoticeKey, captured parent lifetime and grant in ADR0033. W23 then joins reserve,
   queue admission, canonical notice EventSeq and read-only recovery in the same
   retained owner/budgets. No Creator::Owner fallback or user-key collision.
4. A notice body remains TimelineNotification {level,message}; EventRecord.seq
   remains presentation identity. Producer idempotency must replay the same
   canonical sequence and queue identity across restart. Internal provenance must
   prevent an extra ordinary-user bubble. Atomic queue/event/receipt recovery is
   new work, not a claim about the current async watermark implementation.

The root web preflight's separate ruling permits no new node RPC. This proposal
therefore asks for an explicit disposition of the recovery need instead of
silently extending that preflight. W18 and W23 must agree the shared contract and
V3 must review it before implementation. Existing typed-grant checks, ADR0033
self-queue policy and the one node queue remain binding.

Coordinator disposition: approved. The no-new-RPC ruling applies only to local
web preflight. The recovery method is one staged row under TIMELINE_PARITY None
with a deny policy line and merge-assigned number. The concrete observation and
separate binding unions are now specified in ADR0084 for V3 contract review
before implementation. The exact W18 NoticeKey/grant packet is still separate.

Joint review source is now aligned with W18's final docs-only ADR0033 amendment
0c8124d2a..69a682be1. ADR0084 records the exact private key, finite reservation
ceilings, terminal-or-unavailable slot rule, whole-parent-only cleanup, captured
current-lifetime/grant fence and same queue/event acceptance. Existing Paseo
receipt and actor-ticket references remain the comparison; the durable recovery
lookup/private namespace is an explicit Vessel addition, not copied Paseo wire.
No product code or method is registered by this contract checkpoint.

## Values checkpoint after row251

Row251 source-passed the joint contract. The first implementation checkpoint adds
only timeline/recovery.rs, timeline/notice.rs and admission constants, reusing the
existing UUID declaration macro for FinishIntentId. It registers no method and
changes no Store, actor, watcher or provider behavior. Recovery values depend on
the complete original SubmitPrompt contract; this is not a portable body-only
producer or an app activation packet.

[Decision Log]
- Purpose: give the receipt and finish-intent owners one exact shared vocabulary.
- Existing constraint: accepted receipts lack durable actor target correlation;
  public client identities cannot safely namespace internal notices.
- Alternatives: reuse public ClientMessageId or create a second receipt store.
- Choice: closed private NoticeKey/checkpoint and read-only recovery DTOs, sharing
  the existing UUID implementation and retained admission budgets.
- Why: identity cannot imply execution authority, and missing observation cannot
  accidentally admit work. The existing atomic parent/grant fence is still owed.
- Consequences: values can be compiled independently of positive admission, but
  do not prove storage accounting, lifecycle recovery, exactly-once effects or
  public API support. The watcher adopts the shared needs-you ceiling when wired.

Authored pure fixtures cover every checkpoint's encoded key bound and UUID
validation, distinct request namespaces, the unmodified original observation
request, binding wire identity and missing versus uncertain outcomes. Current
changed-source compilation and execution are still owed.

The observation row is now explicitly staged under TIMELINE_PARITY None, with
the agent policy deny arm and an SDK wrapper. Its generated default handler
remains Unsupported; it is not a node read implementation or an assigned method.
SubmitPrompt's SDK documentation now explicitly calls it read-or-admit, never a
recovery query. The later owner-wiring commit must supply ordinary session and
Creator checks and change the row to pending together with the real handler.

The Store now has a read-only observation method using the existing retained
lookup/fingerprint owner. It can return the exact original queue entry or command
echo already in acceptance; direct/joined turn targets remain explicitly
Unobserved until actual ticket metadata is wired. Query-only SQLite regressions
are authored to catch accidental fresh admission and original-content conflicts.
The RPC remains staged/Unsupported and no node handler or actor binding is claimed.

The same migration's receipt row now has optional binding metadata, covered by its
existing worst-case receipt reservation. A fresh ReceiptToken can mint a private
binding token before acceptance consumes it; neither lookup nor replay can mint
one. The single writer only binds accepted rows, validates receipt/target pairing,
and permits unobserved-to-turn or exact queued-to-turn/withdrawn transitions.
Settled duplicate targets are idempotent; different settled targets refuse.
Acceptance state is unchanged. Startup checks target decoding, pairing and the
combined envelope instead of clearing bad metadata. No second table/store/schema
migration is introduced. Transaction rollback/reopen, stale targets, Creator
conflict, wrong queue identity and pending-row refusal fixtures are authored.
The actual ticket producer and node handler remain separate, unimplemented work.

The next source checkpoint wires the original Tickets owner for identified
submissions too: a receipt ticket shares the existing started/awaiting/queued
association, while legacy ticket receivers keep EventSeq/closed semantics. Only
explicit queue pruning emits Withdrawn, after QueueChanged publication, with its
exclusive watermark. Dropping the actor/ticket is uncertainty. Started/steered/
replacement acceptance now awaits its actual ticket and persists through that
exact start, rather than sampling a possibly earlier info watermark. Queued
acceptance and native command echo do not wait for a model turn.

After immutable receipt settlement, the existing TaskTracker supervises the
binding wait and canonical persistence barrier before the same writer commits
metadata. A crash/lost ticket/failed binding commit leaves explicit Unobserved or
the original Queued binding, never a guessed target or redispatch. The handler
now performs ordinary target/Creator checks without wake or attachment reads, and
the row changes from staged to pending with that owner implementation. Its sole
feature stays None and its agent policy deny stays in place. This source is
uncompiled/unexecuted until the next exact VM checks; it is not consumer-ready.

Two further private-fake regressions now exercise missing observation with a
nonexistent attachment before/after process handover, and an accepted fast turn's
exact binding after another real fake turn. They read the original keyed
TurnStarted record, never infer a next turn. All owned node timeline runtime cases
now have one named 120-second testkit safety owner covering setup and cleanup;
fake prebuild precedes that clock. These new cases and the safety-wrapper change
are authored only until their exact changed-source VM run, not inherited from
the older receipt/store or header results.

Current owner checks at 4fb8cc99 were RED before runtime: the new observer was
missing explicit Reply-agent masking, Method forwarding and Call field-version
classification, plus one inherited Forwarded test constructor's omitted default.
Those exhaustive arms/default are now completed without a new authority/version.
The actor barrier's test module also moves last and gains a named local testkit
safety bound. Old run results remain failed; no generated wire artifact or
runtime pass is inferred from them. Corrected execution is still owed.
