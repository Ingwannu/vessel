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


## Driver replacement invalidation checkpoint

The existing Agents::begin_move owner now rotates the execution under the same
registry guard before publishing its replacement credential, also when MCP is
absent. An already-running move or failed credential mint does not publish another
transition. Restoring old credentials after a failed replacement does not restore
the old incarnation. Synchronous watch fixtures cover both credential modes and
the failed-move path without an await. This is another invalidation join only;
it does not qualify capture, metadata lease, current-grant queue admission, or
private notice production. The existing grant is immutable after reserve today;
future grant mutation must invalidate under that same registry guard. Exact
changed-source checks and review remain owed for this checkpoint.


The existing Sessions metadata mutex is now held in Arc, retaining all current
lock sites and order. This is the mechanical ownership prerequisite for moving
an owned lease into the later actor command; it neither adds a second mutex nor
captures a parent, verifies a grant, or enables private admission. The actual
capsule and held-command/caller-loss tests are still owed. No public API or
unused future entry point is added by this conversion.


## Staged capture checkpoint (A10)

The next source module is test-only until the finish-admission actor capsule is
implemented and wired. It captures only a direct local Agent whose actual Agents
entry has the matching registered, non-ended SessionHandle and both current
Read/Operate scopes intersected with the authenticated principal. Subscription
precedes UUID reading under that same registry guard. The non-serialized object
retains the actual registry, original stamp/ceiling and watch; a read-only
observation can cancel a watcher but never authorize a later queue effect.

Private fake SessionHandles cover unregistered handles, Owner/Device/Delegated
and foreign-agent refusal, both principal and registry grant ceilings, rotation,
fresh versus retained capture, and retirement. One named 120-second safety owner
covers setup and runtime shutdown/task settlement/store cleanup. These fixtures
are authored only. No capture API is exposed in product yet; no metadata lease,
actor capsule, actual atomic check/insertion, transition qualification or private
notice capability is inferred from this staged capture. Exact all-target and
fake runtime evidence plus source review remain owed.


## Bound child creation: exact launch admission

The coordinator's bound-creation ruling extends this common owner beyond private
notice queues. This is a contract checkpoint, not an exposed API or qualified
creation controller. ADR0091 owns the creation state machine and concrete launch
capsule; the shared Agents owner supplies current incarnation/grant admission.

[Decision Log]
- Purpose: close the revocation gap between a child count reservation, the durable
  resource Begin acknowledgement, and actual launch admission.
- Existing constraint: count admission cannot hold the synchronous Agents mutex
  across Store acknowledgement or process startup. The Sessions metadata lease
  does not exclude synchronous incarnation invalidation or grant retirement.
- Alternatives: treat a count reservation as permanent launch authority; or add
  a global authority lease to every parent transition owner. The former races;
  the latter extends all transition contracts beyond this fence unnecessarily.
- Choice: after committed Begin, validate again and insert exactly one concrete
  creation-owner launch command under the same Agents guard. Retain its original
  invalidation observation through spawn settlement, stopping any child revoked
  after admission through the normal lifecycle path.
- Impact: admission is the linearization point, not a promise that a process
  cannot be created after revocation. The tracked owner prevents such a process
  from becoming an unobserved child and reports the admitted revocation distinctly.

### Count, placement and the second check

The bound path accepts only the authenticated direct local Agent and its exact
captured execution. Owner, Device, delegated or foreign principals cannot obtain
this new path by supplying a serialized stamp. Legacy creation remains unchanged.

1. The existing Agents count owner reserves the preallocated child under its one
   mutex, including duplicate-before-credential-mint, current parent grant,
   depth and last-slot checks. This reservation is not the later launch authority.
2. The creation owner retains its canonical project/root admission and exact
   child start lease. It waits for the actual durable resource Begin ACK. Lost
   acknowledgement is reconciled by that retained owner; caller cancellation is
   not proof of no effect or permission to create another child.
3. Only after that ACK, the common owner checks the original registry identity,
   local parent/session/incarnation, actual registered non-ended handle, and
   saved/authenticated/current ReadOperate intersection under the SAME Agents
   mutex. The concrete capsule must name the exact reserved child and count
   marker, owning operation/Begin revision, parent and bounded deadline. No
   arbitrary closure or unlocked authorized token is returned.
4. Still under that guard, it consumes the single-use capsule into the concrete
   creation owner's tracked launch command. Insertion is synchronous and cannot
   await Store, spawn, logging or another mutex. A stale incarnation, expired
   command, stopped owner or changed count marker refuses before insertion.
5. The admitted command retains the ORIGINAL capture/watch and the exact root,
   start and count ownership through spawn and settlement. It cannot recapture a
   replacement incarnation or enlarge the saved ceiling. Spawn/I/O occurs outside
   the synchronous lock. The existing bounded tracked owner retains the command
   even if its RPC caller disappears; no new unbounded task/waiter registry.

Current qualified Store placement has narrower meaning than execution authority.
In the announced registry implementation, Session resource membership is retained
and conflicting owners are refused; there is no production removal/reassignment
path. Descriptor project/path/backing/created_at are immutable. This establishes
historical same-workspace placement, not a live parent or current permission.
The child start claim checks Active/revision atomically and creates Pending;
archive must drain Pending/Uncertain before removing backing. A cloned
WorkspaceStartLease is exact settlement metadata, not an authority lease.
The actual retained physical/backing/generation and start-drain joins remain
receiving-owner obligations. Sessions::marking does not serialize independent
registry writes; the implementation must not assert that it does.

### Revocation and settlement

Before capsule insertion, invalidation wins and no launch may occur. After
insertion but before spawn settlement, the original watch makes revocation
visible to the tracked owner. That owner keeps the actual spawn future and its
holds until it settles, then stops a created child through the normal stop path
and reports RevokedAfterAdmission. It does not discard the future, roll back
count as though no child existed, return a no-effect Refused outcome, retry, or
leave an orphan. A failed/uncertain stop remains owned and reported uncertain;
physical release is never inferred from the caller's deadline.

Once spawn has settled, ordinary lifecycle supervision owns the child and normal
invalidation/stop behavior. The handoff from pending launch supervision to that
lifecycle must have no lost-watch gap: a revocation before that handoff is handled
by the pending owner, one after it by the lifecycle owner. A read/check followed
by an unobserved transfer is insufficient. W3 owns the concrete capsule, this
handoff and retained creation result; W23 owns the common incarnation/grant
check. Neither may use a fresh capture to repair an old command.

Required tests hold each actual boundary: invalidate before insertion (no spawn),
invalidate after insertion while spawn is held (settle, stop, reported admitted
revocation), and invalidate after lifecycle handoff (normal stop). Include caller
loss, deadline while spawn is held, failed stop, exact child/count marker mismatch,
concurrent last slot, duplicate child id and parent replacement with identical
SessionId. Existing setup/final supervision remains bounded and retains work
through settlement. No positive bound creation is enabled by this document.
