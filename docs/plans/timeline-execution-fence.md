# Proposed execution-incarnation fence (review before shared registration)

This is the coordinator-assigned missing authority producer for ADR0084/ADR0033
row251. It is a proposal, not implemented or a private notice capability. The
existing ProcessLifetime observes a provider process; Agents::Entry currently
has only credentials, scopes, parent and a SessionHandle. Neither is this fence.

[Decision Log]
- Purpose: old asynchronous finish work must not cross a human transition or
  borrow a newer execution's grant when it finally reaches the parent queue.
- Existing constraints: Agents owns current grants under one synchronous mutex;
  Sessions::marking already linearizes authoritative stored parentage against
  detach/resume/archive, including ended children. Session owns queue mutation.
- Alternatives: liveness/scope snapshots race; a new authority database duplicates
  ownership; holding a synchronous Agents lock across an await is forbidden.
- Choice: exact typed incarnation plus the existing metadata lease, followed by
  synchronous current-grant validation and queue insertion under Agents' lock.
- Why: invalidation and actual queue admission have one linearization order,
  while all I/O and publication happen outside the synchronous lock.
- Consequences: the metadata lease may span a bounded actor admission wait. A
  stale/closed/timed-out candidate retains uncertainty or a proved no-effect
  refusal; no fresh capture or Owner fallback repairs its original identity.

## Values and owners

W23 supplies the common producer. Transition owners retain their call sites:
GKS lifecycle calls invalidation for explicit person stop, resume and archive;
W2 calls it for detach. Every other actual driver replacement or grant change
must use the same owner rather than silently preserving a captured incarnation.

- `ExecutionIncarnationId`: node-minted UUID, distinct from SessionId, turn
  EventSeq, provider ProcessLifetime and FinishIntentId. Its serialized spelling
  is correlation only and cannot construct the runtime fence.
- `ParentExecutionStamp { node, session, incarnation }`: bounded durable evidence
  in the finish intent. Public submit/observe requests cannot select it.
- `FinishGrantCeiling`: closed Read / Operate / ReadOperate value; no Admin.
  Finish registration requires ReadOperate. Saved ceilings never grow. Actual
  enqueue requires both still present in the current Agents entry too.
- `ExecutionInvalidation`: closed PersonStop / Resume / Archive / Detach /
  DriverReplaced / GrantChanged causes, for private diagnostics and tests.
- `CapturedParentExecution`: node-private, non-serializable capture containing
  the exact stamp, normalized saved ceiling and an incarnation-change receiver.
  Only the direct authenticated local Agent path captures one. Owner, Device,
  Delegated and foreign Agent inputs refuse; no serialized intent can mint it.

Agents::Entry gains one incarnation and one bounded latest-value watch sender.
The existing registry remains its sole owner: no per-intent authority map, weak
registry, lifetime database or second watcher. `capture_execution` subscribes and
reads under the same Agents mutex, so termination/invalidation cannot occur in
an unobserved subscribe-after-snapshot gap. Capture also checks the actual live
registered handle; liveness is an additional refusal predicate, never authority.

`invalidate_execution(session, cause)` runs after normal authorization/state
validation and before the transition's first effect. Under the same Agents mutex
it replaces the UUID and publishes the change. Retire/forget removes the entry,
closing its old watch. Existing credentials/grants/parentage are not elevated.
Fresh authenticated calls may capture the new stamp; an existing intent cannot
recapture, substitute that stamp, or reinterpret its retained refused key. A
failed transition conservatively leaves old captures invalid, never resurrected.

## Exact actor admission boundary

1. Existing-key receipt observation/replay remains available before fresh capacity
   or capability checks. It neither creates an authority object nor sends work.
2. New notice work uses its previously reserved immutable key/payload/fingerprint.
   After store awaits, acquire the existing Sessions metadata mutex as an owned
   lease. Its field becomes Arc<tokio::sync::Mutex<()>>; existing `.lock()` users
   retain the same mutex and order. No competing parentage lock is introduced.
3. Under that lease, re-read authoritative stored parentage using the existing
   MAX_DEPTH-bounded traversal. It must still reach the exact captured local
   parent; foreign links, detach, missing/corrupt metadata and changed ancestry
   refuse. An ended child can still be checked from Store without restarting it.
4. Transfer the owned metadata lease into a node-private, single-use boxed
   `InternalQueueFence`. The queued actor command owns that box after send, so a
   dropped RPC cannot release the ancestry lease while stale work is queued.
   No wire/serde constructor creates this object.
5. The session crate exposes one documented policy injection boundary:
   `InternalQueueFence::admit(self: Box<Self>, effect: &mut QueueAdmission<'_>)`.
   QueueAdmission is a concrete actor-owned capsule with private fields; its
   synchronous commit method can insert only the already validated bounded
   notice into that actor's existing queue and return its actual entry identity.
   The trait is an open embedding-policy boundary between session and node;
   domain states/results remain closed enums. It accepts no arbitrary future.
6. The production node implementation locks the actual Agents registry, verifies
   local node/session/incarnation equality, current handle state, and BOTH current
   Read/Operate intersected with the saved ceiling, then invokes that synchronous
   insertion before releasing the Agents lock. No await, provider operation,
   logging callback, Store I/O or canonical publication runs while it is held.
   Invalidation/retire/grant update uses this same lock: either insertion happened
   before invalidation or it refuses before mutation. A token returned by an
   earlier unlocked check is explicitly insufficient.
7. Drop the single-use box/metadata lease immediately after this decision. The
   actor then publishes canonical notice/queue records and the existing retained
   writer completes their same-key transaction/recovery. This fence alone does
   not implement the queue/event/receipt atomicity promised by row251.

Lock order is metadata lease, then Agents mutex; never the reverse. Agent callers
must not await the metadata mutex while holding the synchronous registry lock.
GKS/W2 invalidation is synchronous and can revoke an old capture while a candidate
waits; metadata mutation still uses the existing owned metadata order. The actor
never acquires that mutex again inside the transferred lease.

The existing named setup/final supervision deadlines remain the owners of held
work (120s setup / 5s final). Before a possibly queued command, expiry is a typed
no-effect refusal. Once send may have happened, timeout/lost acknowledgement is
OutcomeUnknown, with the actual command/lease retained until actor settlement;
it does not authorize retry. An expired command must refuse before insertion
even if it is dequeued later. The fence adds no unbounded waiter registry.

## Detach, recovery and limits

Detaching the captured parent rotates its execution. Detaching an observed child
or intermediate ancestor invalidates its parentage proof through the same metadata
lease/order, including a stored child with no Agents entry. Thus no retained
child-entry registry or history-derived current parent is needed. Work admitted
before an authorized transition remains an earlier admission; invalidation does
not fabricate a rollback of its accepted queue receipt.

Persisted stamps never revive authority at boot. Only a lifecycle-owned proof of
the SAME committed execution could restore its runtime incarnation. No such
restoration proof exists today: explicit resume/new reserve mints a fresh UUID,
and private cross-restart re-admission remains Unsupported/observation unavailable
until that producer is separately reviewed. Retained receipt observation still
works without re-enqueueing or waking a session. ProcessLifetime alone is not the
restoration proof. This is an explicit Vessel authority floor beyond Paseo.

Incarnation/watch state is one fixed-size object per existing live Agents entry.
Each captured handle belongs to an already bounded finish intent/watcher; the
intent retains its stamp/ceiling within the approved 128KiB envelope. The metadata
lease is the existing single owner, not a new retained counter. Dropping handles,
acknowledging or revoking authority never releases durable replay reservations;
only committed authorized whole-parent-session removal does.

## Required proof before positive notice admission

- Hold Store/setup/actor admission futures; stop/resume/archive/detach/grant
  shrink before release must refuse without queue/event/provider effects.
- Hold the actor after command send, lose the caller and expire the deadline:
  the lease stays owned, late work cannot insert, and no automatic resend occurs.
- Exercise both orders at the synchronous check/insertion boundary; a transition
  cannot interleave after validation but before the queue mutation.
- New execution with identical SessionId, same provider process spelling or
  widened fresh grant cannot upgrade the old stamp or retained refusal.
- Capture subscribes before state, and parent end/retire closes it. Unrelated
  child detach does not invalidate a different ancestry chain.
- Stored/ended child detach, ancestor detach and foreign-node parentage refuse;
  positive direct local parent insertion uses the existing exact queue once.
- No lock-order deadlock; no metadata lease leaks after rejection/owner loss;
  actual setup/final supervision owns blocking work through settlement.
- Combine with row251 crash/reservation/key-conflict/renderer-provenance tests.

This proposal changes no registration, provider, grant, service or process.
V3 must accept the exact lifecycle/actor/Agents shape before shared code is added.
