# Shared connection validity at queued delivery

Status: interface and proof plan, not source/runtime adoption approval. This
extends the existing selected connection lease; it does not add a registry.

## Decision Log

- Purpose: a result can wait in the bounded app queue while its actual connection
  retires. A copied ConnectionContext or a later Fleet notice cannot close that gap.
- Constraint: G4 owns the original client, context and irreversible validity watch.
  A Session FollowBinding and operation ticket fence different, additional lifetimes.
- Alternatives: cache an alias/node/boolean in App, export the client, or carry a
  narrow validity capability for the same lease. The first two are rejected.
- Choice: an actor-minted opaque `DeliveryGuard`, cheaply cloned into result
  envelopes and checked immediately before installation without an await.
- Why: streams and Unary replies can share exactly one queue validity predicate.
- Cost: one canonical shared lease per Connected record and one nonblocking
  validity check per installation; this does not make network or filesystem effects
  atomic and cannot undo a write already sent.

## Planned API and ownership

The deliberate public native API is `vessel_app_link::DeliveryGuard`:

```rust,ignore
impl DeliveryGuard {
    pub fn validate(&self, expected: &ConnectionContext) -> Result<(), Refusal>;
}
```

It has private fields, cheap Clone and redacted Debug. It has no public constructor,
Default, serde, raw-client accessor, alias resolver or epoch allocator. Only the
existing `SelectedConnection::delivery_guard()` creates it inside Link after actual
preflight. Connected anchors one Arc<SelectedLease>; SelectedConnection clones that
same Arc only while its request/follow runs. DeliveryGuard stores only Weak to that
lease, with no client/transport or cycle. Mint once for each selected request/follow
binding, then clone only Weak into events; do not clone aliases/clients per record.

The canonical lease retains only the validity receiver, never its Sender.
Validation requires exact original context, actual original client welcome node,
Active validity and an open original transport. A dropped validity owner refuses
Changed even if its last value was Active and a running request still holds the
original client. Guard validation transiently upgrades the weak lease and releases
that strong reference before returning; expired weak also refuses Changed. Queued
guards alone cannot delay original transport destruction. Unary request completion
does not invalidate an active Connected anchor; retired in-flight work may retain
its original transport until it ends, but its result must refuse installation.
A retired watch never becomes active again, including reconnect to the same node.
Managed local connections use this exact capability and predicate, with no exception.

The browser build cannot mint such a native capability. Its representation is
uninhabited/refused, not a demo guard that can report success. Preflight refusal
still settles the original correlated UI operation without inventing a guard;
an admitted wire result carries one. Feature adapters retain their own typed error
projection and must not expose raw peer Debug/Display/error-chain data.

## Nonblocking predicate

Use one internal selected-lease predicate for existing request/follow checks and
delivery validation. In addition to actual NodeId and watch value it checks sender
closure and original transport closure. No root SDK API is added.

`vessel-client::NodeClient::closed` delegates to `vessel-rpc::Connection::closed`,
which awaits the existing shared CancellationToken's `cancelled()` future. One
stack-pinned poll with a no-op waker, then immediate future drop, performs no
network I/O, retry, spawn or blocking wait and retains no cancellation waiter.
Source review must trace that implementation and its drop behavior; tests must
also verify validation issues no framed request or background reconnect.

## Queue and receiver rules

Link selects retirement against a bounded send under backpressure and checks
again after awaits. That check alone is insufficient: retirement can occur after
the send was accepted but before the app installs its result. The app validates
the capability against the stored original context immediately before its scoped
receive/settle mutation, with no intervening await. It separately validates the
full current FollowBinding and consumes operation tickets exactly once.

A current binding whose lease is retired loses its operation admission and moves
matching pending UI to Uncertain, preserving input even if a replacement slot was
refused. An older binding cannot retire a successor. Local field refresh of the
same resident/claim is not a network admission or delivery path.

No consumer enables this by merely copying a successful boolean or suppressing a
retirement notice. Devices Unary replies, session metadata/streams/actions and
other consumers reuse this capability; none creates another connection cache.

## Proof required before enabling product callers

- Exact context succeeds; wrong node, epoch or alias refuses before mutation.
- Retired watch and dropped owner with last Active value refuse while the original
  physical client is still open. Closed transport refuses without framed calls.
- An already buffered item, error or EOF is rejected after retirement.
- A full bounded channel does not keep a retired event live; accepted send followed
  by retirement before App installation still refuses (also test a Unary reply).
- Current pending UI becomes Uncertain without waiting for a newer Fleet/slot
  notice; a stale old binding never retires its successor or clears later input.
- Retiring/removing Connected and ending old work actually drops the peer transport
  while a queued guard is retained. Active Unary completion still permits delivery
  from its unchanged Connected anchor; retired in-flight Unary completion refuses.
- Guard clones share only Weak; final drop releases it without a cycle, task or
  retained waiter. Debug/error output contains no peer payload or secret.
- Staged source uses literal test-only gates until real caller wiring removes the
  exact attributes and checks product visibility plus the full consumer union.

These proofs are separate from final live stream, permissions, UI, OS and multi-pane
acceptance. The earlier bounded connection preflight review is not queue proof.
