# Node-internal service Preview API

Status: staged source implementation; execution pending.
The only real backend is Unavailable; no product runtime grant exists.
ADR 0066 defines the bounds and security obligations.

## Consumer shape

The browser's node-owned adapter consumes concrete types under
crate::terminals::preview; the browser crate has no node dependency.
Its generic connector can use PreviewConnection as its associated owned
AsyncRead + AsyncWrite + Unpin + Send stream with typed error projection.

- PreviewContext: canonical WorkspaceId, opening SessionId, ConnectionLife
  owner/close observers plus actual transport close. No lifetime guard,
  authority decoder, second connection identity or default id.
- PreviewOrigin: exact HTTP/HTTPS scheme and validated literal loopback
  socket address; public matcher metadata, never a capability.
- PreviewService::admit(context, origin): opaque PreviewLease or refusal.
- PreviewLease::check(context, origin): required before each command.
- PreviewLease::connect_verified(context, origin): owns dial and accepted-peer
  verification, returns the owned concrete stream; no independent dial.
- PreviewLease::revoked(): target/profile/ref shutdown signal.
- PreviewLease::revoke_and_drain(): close admission and wait for actual
  preparations/stream owners, producing a receipt for this lease only.

The initial constructor exposes only Unavailable. Test-only fixtures can
exercise the flow and real loopback transport, but cannot become product
backends or qualify private browser access. No DTO, RPC, protocol number,
node setting, public sharing consent or listener is added by this API.

## Reuse and missing sources

Reuse existing Node ConnectionLife, Proc run observers, Tokio sockets,
semaphores, cancellation and TaskTracker, and the one terminal scan PERIOD.
The actual canonical registry/start/incarnation, seeded process identity,
accepted-socket reader and bounded scheme evidence are still absent from
the assigned node head. The socket_peer policy contract is a prerequisite,
not an implemented backend. No parser, collector, connection identity or
workspace generation is duplicated to avoid this dependency.

Source evidence: Paseo server/local-service-connection.ts4–28 and
service-proxy.ts319–400; it lacks this per-run connected-peer contract.
Vessel node principal.rs/guard.rs keep authentication and call policy above
this API, and connection_life.rs provides the canonical close observer.

## Commits and validation

1. Contract to security review before the narrow producer.
2. Typed scope/origin/budget/error and unavailable backend under A10.
3. Lease registration/freshness/revocation and owned verified connection
   flow, with all positive evidence providers still cfg(test) only.
4. Narrow all-target node Clippy and relevant tests; report exact source
   hash and evidence to the browser adapter owner with the A10 limits.
5. Later wire the real canonical backend and remove staging only with
   its security review and enclosing package gate. No fixture-only PASS.

## Resume point

The A10 producer now includes the closed unavailable backend, renewable
model bindings, lease close/registration gate, owned connection wrapper,
and physical-worker admission/drain ownership. Positive authority and peer
checks are private cfg(test) fixtures only. Their controlled loopback tests
cannot attest kernel ownership or qualify a browser engine. No source
backend consumes legacy listener metadata or infers a workspace generation.

The matcher/context commit has its own immutable narrow suite queued.
The lease/transport commit will run strict node all-target Clippy and the
Preview tests after that suite; no local Cargo or Rust compiler is used.
The real registry/incarnation, seeded identity, socket-peer and bounded
scheme producers, browser adapter and enclosing package gate remain owed.

The control commit separately establishes unavailable admission and lease
freshness/cancellation. The following connection commit adds registration
under its existing gate and preserves those control tests. Its fake
inspection is a transport-ordering model, not an OS peer collector.
