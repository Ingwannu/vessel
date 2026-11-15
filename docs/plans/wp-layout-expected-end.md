# Expected stream end while a stop completion is pending

Status: contract amendment for review before source. No live carrier, stop call,
ticket or ingress rule changes are enabled by this document.

## Decision Log

- Purpose and intent: allow the original typed archive/exit/fresh completion to
  settle when stopping its actor produces normal SessionEnded/EOF first.
- Existing assets and constraints: the sole Residents table owns FollowBinding
  and single-use operation tickets. Full retirement currently invalidates both
  stream ingress and all operations. W21 owns actual app stop/archive calls and
  LifecycleResources. Connection retirement is independent from protected input.
- Alternatives: trust EOF as success; keep every old binding/ticket active;
  create a separate lifecycle ticket map; or seal one existing binding's ingress
  while retaining exactly its already-admitted stop completion.
- Choice: the last option, recorded in the same binding and operation ledger.
- Why: EOF order proves neither stop success nor node execution state. One
  bounded original completion can remain correlated without accepting old data,
  new operations or a reply after a successor attach.
- Benefits, costs and effects: no second table, allocator, lease or identity cache.
  Ingress, admission and settlement must now distinguish sealed from receiving.

## Eligibility and states

W12's atomic lifecycle admission may mark exactly one existing opaque Lifecycle
operation ticket as the expected stop completion. Eligibility comes from the
validated typed archive/exit/fresh intent before submitting its wire command;
neither EOF, a response nor generic Lifecycle kind can create that expectation.
The owner still validates original identity, current user intent and its close
plan. Arming is not evidence that an effect was sent or completed. A definite
pre-send rejection settles the original ticket without fabricating success.

On an actual normal stream end, W21 first validates the same original delivery
guard/context at App installation, then passes its exact FollowBinding to W12:

- Without an eligible current stop ticket, the existing full-retirement rule
  applies. The event cannot create a binding or operation.
- With that exact current ticket, the same binding becomes sealed. No further
  records, snapshot ingress or new operation admission is accepted from it.
  All other pending operations retire with typed owner notifications; the exact
  stop completion alone remains pending under its existing bounded deadline.
- Sealing grants no close, archive, fresh-target, draft-clear or retry authority.
  Resident resources, input, selection/IME and owner holds remain retained.
- The original typed stop completion or correlated refusal settles its ticket
  once, before owner mutation. Wrong kind/ticket/session/connection and duplicate
  replies refuse. Terminal failure/deadline retains input and uses the owner's
  existing NoSend/Uncertain distinction, without increasing a deadline budget.

Full actual lease retirement, unexpected follow failure, cancellation, eviction
or reattach still invalidates every old operation, including this ticket. A
same-node reconnect or alias ABA receives a new follow generation; an old stop
reply cannot close or clear its successor. A refused replacement does not defer
that independent retirement. A sealed ticket is not an exception to those rules.

Normal means a typed normal stream outcome under the original valid delivery
lease, not an arbitrary error, transport close or inferred state from a copied
snapshot. A SessionEnded record or status notice alone is not permission to
discard the stream's remaining records. The same ordered pump must install its
preceding record batches before its terminal normal-end envelope; a metadata
observer cannot synthesize that envelope from an ended status. Records arriving after sealing reject even if they were buffered
earlier. The preserved completion uses canonical ticket settlement, never the
record receive path or context refresh. The normal-end notice distinguishes
partial ingress sealing from full FollowRetirement so a generic owner callback
cannot accidentally retire the one preserved Lifecycle operation.

W23's node execution-incarnation fence invalidates execution on stop, resume,
archive and detach. This client policy does not infer or recreate that state
from EOF ordering. It neither admits a new effect nor overrides a node refusal.
Only the typed stop completion/refusal can settle the original lifecycle action;
even a newer metadata read is not an archive acknowledgement.

## Exact ownership handoff

- W21 prepares the validated original lifecycle intent, passes it through W12's
  admission and retains original Context, FollowBinding and opaque ticket in its
  actual call/envelope. It owns the bounded stop/archive wire call and outcome
  classification. It does not retire the ticket independently on normal EOF.
- W12 alone classifies normal end, seals or fully retires the same table, rejects
  new ingress/admission and consumes the original completion ticket. It invokes
  the concrete owner transitions synchronously after releasing table borrows.
- LifecycleResources stays Pending on the sealed normal end. It becomes
  Uncertain on full retirement of its still-sent ticket, not on unrelated or
  successor end events. A successful completion still holds resources until the
  exact original Window close/fresh commit; stale ClosePlan cannot affect a new
  target. EOF never substitutes for this acknowledgement/commit sequence.

Exact method and notice names belong to the reviewed source amendment. There is
no public bypass flag, second completion map, raw client or admission-from-reply.
W21 and W12 must consume the same policy in one checked carrier/call-site union.

## W21 call-site and carrier agreement

W21 confirms the direction above. This section identifies existing code and the
required coordinated replacement; it does not promote the legacy adapter or
approve implementation. W12 must confirm this handoff and V1 must close the
contract before either owner changes expected-end source.

### Existing source and the replacement boundary

The A2 prototype's `app/src/areas/sessions/lifecycle.rs` has these actual sites:
`prepare_session_lifecycle` and `confirm_session_lifecycle` validate a decision;
`composer_exit` and `composer_fresh` enter that same path; then
`send_session_lifecycle` sends `Command::SessionLifecycle`. The corresponding
`link/src/sessions/lifecycle.rs::State::session_lifecycle` translates exactly
`LifecycleAction::{Archive, Exit, Fresh}` to `Act::Mark(Mark::Archive)` and calls
`session::run`, whose Mark branch calls the existing typed node mark operation.
The answer reaches `sessions_lifecycle_answer` / `apply_lifecycle_result`, and
`settle_session_lifecycle` later receives the actual Window for navigation or
fresh-draft work. This is the concrete ordering problem, not a new stop RPC.

That prototype retains alias/node and a local UI ticket, not the complete
physical admission. None of those legacy checks authorizes the new path. The
coordinated call-site union replaces its global decision/send/answer/window
state with scoped lifecycle resources in W12's sole SessionResources bundle.
`LifecycleResources::prepare` and `sent` are the existing staged owner hooks;
the local LifecycleTicket identifies a UI decision only. W12's existing opaque
OperationTicket with `OperationKind::Lifecycle` is the network capability.

Immediately before the wire call, the validated original typed action and
current decision go through W12's atomic admission. Only Archive, Exit and
Fresh may arm the one existing stop ticket. Unarchive, Resume, Delete, generic
Lifecycle operations and history operations lacking that resident admission
do not acquire an expectation. Archive without closing still needs its typed
acknowledgement. Child hide remains local layout work and cannot arm a stop.
Any stricter node archive API, when assigned and adopted, must preserve this
typed classification rather than deriving it from response or status.

### Actual carrier and outcome classification

The staged carrier is `link/src/sessions/resident.rs`: Scope captures the
SelectedConnection's original DeliveryGuard, an immutable original
ConnectionContext and full FollowBinding. Envelope retains these exact facts;
its Completion retains the original Operation kind/ticket and typed RPC result.
Successful SessionInfo metadata and completion must name the binding's session;
the context node must name the resident's node. No latest-alias lookup supplies
missing provenance. The real call must use that same selected connection.

The current `Payload::Eof` is only a prototype marker. The coordinated source
must distinguish a typed ordered normal end from `Lost`, request/stream error,
physical retirement and cancellation. Only the same selected follow pump,
after installing preceding batches, may submit that normal-end envelope. A
SessionEnded record is ordinary ordered data, never that classification. W21
passes the checked outcome and exact binding to W12; only W12 decides whether
the original pre-armed ticket permits sealing. W21 does not call full
retire_follow for a seal that preserves that ticket. Other follow failures
retire only their exact binding; only actual shared-lease evidence authorizes
connection_retired. Neither can invalidate a successor by inference.

Envelope installation validates the original guard and identity before
exposing data. In the real union that same guard must also be validated
immediately before canonical receive or settle, with no intervening await or
owner mutation. Canonical settle consumes the exact ticket before invoking the
owner transition. A rejected/dropped envelope does not become a success path.
CorrelatedRefusal remains control-only and cannot fabricate a guard. Definite
actor pre-poll/pre-effect refusal maps to local Disposition::NoSend; an attempted
call with lost reply, late error, cancellation or timeout maps conservatively
to Uncertain. The prototype's broad peer-error mapping is not no-send proof.

### Pending, typed acknowledgement, then exact Window commit

1. Before sending, capture the originating Window's lifetime, authoritative
   LayoutKey, PanelClaim/tab and original resident/binding with the validated
   action. Close also retains the original immutable ClosePlan; Fresh retains
   its scoped launch choices and draft preconditions. This owner-held pending
   state is not another operation allocator, lookup cache or settlement map.
2. Ordered normal end leaves LifecycleResources Sending/Pending and its
   PendingEffect hold intact. It cannot call acknowledged, become Ready,
   release an owner hold, close a tab or clear input. Full retirement instead
   reports the exact sent operation as Uncertain and retains protected input.
3. A successful typed original reply must satisfy the requested action and
   session identity. After immediate original-guard validation, W12 consumes
   the exact Lifecycle ticket once; only then does W21 record acknowledged
   pending Window work. The current staged acknowledged-to-Ready transition
   is insufficient and must change in the reviewed union. The consumed ticket
   is never reissued, refreshed or copied into another settlement ledger.
4. Acknowledged Window work keeps the resource held until the originating
   Window can commit synchronously. Immediately before commit, validate the
   original guard again and recheck current binding/resource, key/claim/tab,
   immutable target, dirty/owner votes and action-specific draft preconditions.
   No await, latest selected resource or reused TabId may bridge this check.
   Full retirement/reattach invalidates pending Window work as well as pending
   wire work, including retirement after ACK and before Window processing.
5. Only that successful exact Window callback may approve the keyed layout
   close through layout_close_settled with the original key/plan, or perform
   the original scoped Fresh transition. Archive without local navigation
   still finishes its owner state in this exact callback. Failure, missing
   Window or stale plan leaves a typed held/refused state for reconciliation;
   it never mutates the replacement target, clears retained input, resends the
   operation or restarts its deadline. Readiness is released only after this
   final commit and all independent owner holds still apply.

The required union must exercise the real caller, selected framed stream,
Envelope, canonical ledger, LifecycleResources and Window callback together.
A pure EOF marker test or a manually manufactured acknowledgement does not
prove this handoff. In particular, test retirement both before canonical ACK
settlement and after settlement but before Window commit, as well as an actual
normal End preceding the typed Mark reply. No SDK error or transport EOF alone
is an archive acknowledgement or a node execution-incarnation fact.

## Required counterfactual evidence

Use the actual framed producer to emit normal SessionEnded/EOF before a real
typed stop reply. The expected case keeps input and the exact pending ticket,
rejects later records/new operations and permits one matching acknowledged
completion. Also cover no expectation, wrong ticket/kind, duplicate reply,
same-node/ABA reattach, transport retirement and a failed replacement retaining
dirty/IME input. Physical retirement must win before or after queue acceptance.
Timeout/refusal cannot extend the budget, clear input, resend the stop or close
a successor. A normal-end marker alone is not this actual producer/owner proof.
