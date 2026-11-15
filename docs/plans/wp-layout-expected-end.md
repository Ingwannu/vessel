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

## Required counterfactual evidence

Use the actual framed producer to emit normal SessionEnded/EOF before a real
typed stop reply. The expected case keeps input and the exact pending ticket,
rejects later records/new operations and permits one matching acknowledged
completion. Also cover no expectation, wrong ticket/kind, duplicate reply,
same-node/ABA reattach, transport retirement and a failed replacement retaining
dirty/IME input. Physical retirement must win before or after queue acceptance.
Timeout/refusal cannot extend the budget, clear input, resend the stop or close
a successor. A normal-end marker alone is not this actual producer/owner proof.
