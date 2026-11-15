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
