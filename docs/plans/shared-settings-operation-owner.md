# One retained settings operation owner

Status: proposed contract only. This leaf adds no Rust/API implementation,
recovery policy, disk field, quota or activation. It extends ADR 0064's opaque
saved model and retained writer; the existing model package remains separate
until this shape is reviewed and the joint producer is named.

## Responsibility and alternatives

`[modify]`: one Store-owned private `SavedNodeSettings::Document`, one Node
settings owner and the existing atomic owner-only file replacement remain.
Recovery contributes its concrete `Policy` and Store policy/intent operations;
it does not retain another saved struct, compatibility writer or outer queue.
A callback trait, string/untyped mutation map, second file/writer or field-owned
settings quota is excluded. A cancellable caller waiting for the saved mutex is
also insufficient: queued acceptance must survive that caller's disappearance.

Paseo `packages/server/src/server/daemon-config-store.ts:393-406,484-485,655-659`
places supported patch and reload work on one mutation queue. This amendment
uses a finite node-owned queue and joins physical work. Explicit overflow refusal
and the Rust shutdown/ownership mechanism are stated implementation differences,
not claims that Paseo provides this resource bound or these teardown phases.

## Admission and ownership shape

One `settings::admission` owns a bounded Tokio mpsc channel and one consumer.
Its waiting capacity is `SettingsAdmission::MAX_WAITING`, defined once beside
the implementation type. The running consumer is structurally single: pending
capacity is additional to the active operation. Error messages and boundary
tests derive from the same named waiting-capacity owner.

- Protocol, scope and field authority, and pure typed input validation, precede
  admission. No new RPC method, scope or protocol number is introduced here.
- Submission uses non-waiting `try_send`; success is the admission linearization
  point. Full/closed return distinct typed non-admission errors. There is no
  unbounded semaphore/channel/mutex-waiter list in front of the lane.
- An operation owns its typed intent, reply sender and existing tracker token.
  The token is obtained before checking the owner's closed state. Successful
  enqueue transfers the entire operation to the node before awaiting its reply.
  Rejected submissions release their token and perform no I/O/publication.
- Only the independent consumer acquires the owned saved-document guard. It
  retains that guard and operation accounting through preparation, physical
  file I/O, publication, recovery SQL/cleanup and the reply attempt. There is
  no spawned mutation task per queued caller and no second settings owner.
- Accepted enqueue order is execution order. Caller cancellation drops only
  its reply waiter: it cannot remove/reorder a queued intent, release capacity
  for active physical work or let a later On pass an accepted Off.
- The existing tracker also owns worker completion; its worker state is separate
  from the outer sender so no Arc cycle keeps an abandoned owner alive. A closed
  worker never fabricates successful replies. Concrete task/receive failures
  use the reviewed safe diagnostics boundary, not raw panic text.
- Closing first seals admission, then closes the receiver and drains all accepted
  messages. A pre-close submission that races receiver closure is either owned
  and drained or receives typed non-admission. Token-before-closed accounting
  prevents a late producer from registering physical work after drain returned.
  Dropping a shutdown waiter does not abort the worker or lose its obligations.

The same lane covers generic patch, reset, explicit reload and approved dedicated
mutations. Existing MCP pre-lock settings admission is replaced at the joint
merge, not nested around this owner. This does not remove an independently
qualified reader/helper physical-capacity limit for another resource.

## Private record and projections

`Document` owns an optional typed recovery patch using the existing
`SetRestartRecovery`, together with today's generic fields. Preserve existing
canonical JSON bytes when absent, tolerant reads, future-field coverage,
set-aside behavior and atomic owner-only replacement. No public fields,
Serialize/Deserialize/TS or raw JSON escape is added to the opaque envelope.

Parking/drivers-only mutations preserve the saved recovery patch and every
future protected namespace. Omitted enabled/prompt fields preserve their owning
values; defaults and verbatim UTF-8 prompt validation reuse the existing recovery
value types. The wire patch is Clone rather than Copy when this field joins.
Readers borrow/project the opaque snapshot; candidate clones are explicit and
bounded. The blocking writer returns its same owned candidate after I/O instead
of cloning the complete document again for a background task.

Get/Set reply projection and the shared optional protocol accessor remain
canonical. An unassigned/hidden recovery feature is not confirmation of Off.
This leaf introduces no service/plugin schema, private metadata setter or
permission to reach those namespaces through generic reset/reload. A namespace
with mixed public/private parts has one typed disk owner plus explicit projection,
not duplicate flattened keys.

## Concrete RecoveryApply seam

Target module: `vessel-node/src/settings/recovery_apply.rs`, crate-private.
Nodectl owns assembly and every call site inside the common operation. Recovery's
owner supplies the concrete implementation over `Arc<recovery::policy::Policy>`
and the existing `vessel_store::Store` handle. It performs no settings file I/O,
launch, prompt, secret read, independent task/queue or alternate persistence.
There is no callback trait or dynamic extension mechanism.

Proposed signatures (contract notation, no facade exists yet):

```text
RecoveryApply::new(policy: Arc<Policy>, store: Store) -> RecoveryApply
RecoveryApply::seal_effect_admission(&self)
RecoveryApply::settle_before(&self) -> async Result<(), RecoveryApplyError>
RecoveryApply::begin_write(&self, next: &RestartRecovery)
RecoveryApply::write_failed(&self)
RecoveryApply::saved(&self, next: RestartRecovery)
RecoveryApply::settle_saved(&self, current: &RestartRecovery)
    -> async Result<(), RecoveryApplyError>
```

`RecoveryApplyError` keeps typed Apply/Cleanup Store causes. They map once at the
existing settings service boundary. Prompt contents never enter Display, Debug,
source traversal or the wire message. The concrete safe decoder cause remains
private; no string replacement or second sanitizer is introduced.

Inside the same guard, for every accepted operation:

1. `settle_before` pays pending Off cleanup with the actual Store policy
   operation. Failure leaves effective admission Off, blocks subsequent On and
   returns Cleanup before another mutation or unchanged-value reply.
2. Prepare and validate the complete candidate. If it differs, `begin_write`
   marks the pending-Off barrier immediately before preservation/replacement.
   It shares the existing short arbiter with actual recovery enqueue, so no
   startup launch/prompt can pass an uncommitted Off. A failed file operation
   calls `write_failed`, restoring the prior effective admission and returning
   the original file failure; it invents no successful save.
3. After real persistence, publish the saved snapshot and parking/drivers in
   their existing order. `saved` calls Policy::apply before any awaited cleanup:
   Off invalidates its admission generation, while On remains paused pending
   its actual Store apply.
4. `settle_saved` awaits the actual Store policy/intent operation and only then
   marks it cleaned. Failed Off cleanup keeps saved/effective Off plus debt;
   failed On apply keeps admission paused. A later identical explicit mutation
   retries missing apply without rewriting an unchanged file. Therefore the
   old next==saved fast return must call this settle path too.
5. Only after settlement does the owner attempt the reply and release its guard
   and operation accounting. Reply loss skips none of the physical obligations.

Already-admitted prompts retain their actual owner; policy Off does not
retroactively kill them. A person's emergency Stop uses its own committed
suppression/actual-run path and must not wait behind an unresolved prompt reply
or this settings lane. This amendment does not fix or approve that separate
recovery arbiter path.

## Boot snapshot and shutdown

`settings::Opening` is the one startup adapter, not another settings engine.
Its proposed `read(&Config) -> Opening` consumes the canonical Store file edge
before Store startup interruption/retention. An internal `recovery_policy(&self)`
projects that same validated opaque snapshot for candidate capture. Later
`Settings::from_opening(opening, runtime, recovery_apply)` consumes the original
snapshot, launch precedence and kept/future disposition; it does not re-read or
reconstruct a competing saved value.

Coverage must be computed from the same bytes at the canonical disk edge and
return a typed disposition, with no untyped tree outside that edge. The existing
move-aside/default/kept-file and uncertain-future-copy policies remain. Both full
and compatibility diagnostics use the reviewed SettingsDecodeFailure boundary
(category/masked bounded location, concrete cause private). Recovery adds its
startup/source-chain canaries at this adapter instead of another parser.

When node stop begins, seal settings admission and recovery effect admission
before waiting for the queue. The seal is under the same final enqueue arbiter;
a cancellation check alone is not an atomic start fence. It neither changes the
saved policy nor revokes an already-admitted prompt. Late On persistence cannot
reopen a stopping effect owner. Other field owners that can start a process need
their own qualified lifecycle fence and desired/effective outcome.

Then drain every accepted file/publication/SQL cleanup obligation while Store
and the session runtime remain available. Only after settings settlement does
teardown complete runtime/session shutdown and the existing log/forwarder/RPC
closure sequence; Store closes last. The current late park/driver-only proof is
not proof of this changed ordering or new effect owners. These lifecycle changes
and their real startup/stop join require the joint gate. A blocked filesystem
or Store operation can delay drain; no forced-cancellation deadline is promised.

## Owed evidence and delivery

- Hold real blocking replacement; cancel caller; verify accepted queued order,
  retained capacity/guard, final file/runtime value and shutdown joining.
- Hold saved-Off before cleanup, enqueue On and another namespace mutation;
  neither overtakes it. Force cleanup failure and same-value retry; force failed
  On apply and failed Off save, preserving the exact policy admission outcomes.
- Fill the named waiting capacity behind one active operation; overflow/closed
  refuse before work. Cancel queued reply waiters; their accepted intents still
  settle in order. Close/drop/shutdown races cannot leak a late admitted write.
- Boot uses the exact pre-Store snapshot, ordinary changes preserve recovery
  and protected data, unchanged data writes nothing, future bytes/set-aside and
  owner-only permissions remain. Complete diagnostic-chain/startup canaries.
- Stop closes final effect admission before held I/O settles; no late startup
  launch/prompt/helper and no false Running/apply receipt. Store/runtime remain
  valid through cleanup, and admitted work is joined before their close.

First review this docs-only shape. After acceptance the coordinator names the
exact joint base; recovery supplies its hook and field logic against that shape,
nodectl implements the common owner/record/boot join, and the union gets changed
strict/tests and one joint enclosing gate. A prior branch's green gate or a
source-only verdict does not qualify the new producer. Existing source-returned
recovery behavior stays out until its independent findings are corrected.

## Implementation resume point

The opaque model is now composed with the canonical private-cause decoder.
The initial operation lane is implemented for the existing generic patch: one
consumer prepares candidates after earlier publications, owns the saved guard
through the existing atomic replacement and applies before attempting a reply.
Its state does not own the sender. A closed tracker and receiver seal refuse
late work while draining accepted intents; dropping a shutdown waiter cannot
abort the consumer. A full waiting lane is a typed Conflict at the service edge.

Fixtures hold the real blocking writer to pin overflow, cancelled queued
replies, ordered preparation, shutdown-waiter cancellation and last-handle
drain without a sender cycle. The original cancellation regressions retain
their assertions and budget, with only the private state path adapted. A sealed
concrete task failure reuses the separately reviewed diagnostic policy. These
fixtures have not yet compiled or executed on this source.

This is not the completed joint producer: RecoveryApply, the typed recovery
field, same-value cleanup, one-read Opening and the changed effect/shutdown
order remain to be joined and checked. The initial lane does not authorize any
recovery, MCP or plugin activation and does not replace a consumer's temporary
quota until the qualified joint head is announced. Explicit reset/reload intents
join this same lane with their actual callers rather than unused variants here.
