# ADR 0104: Tracked DNS resolution lifetime

Status: contract source accepted by V3 on 2026-11-16, exact f2779ab90.
Shared implementation and consumer source/runtime checks remain separate.
This is a separate prerequisite for relay consent (ADR0096), native
HTTP dictation endpoints (ADR0067), and W5's account-usage HTTP client.
It does not extend these packages'
existing source/runtime verdict or activate an unassigned method.

## Intent, existing assets and alternatives

[Decision Log]
- Purpose: a native lookup must keep its capacity and completion owner after
  the async caller disappears. A normal stop must prove the lookup ended.
- Constraint: started OS `getaddrinfo` cannot be cancelled; cancelling an
  HTTP/websocket future is not completion of that native work.
- Existing assets: Tokio's blocking pool and standard `ToSocketAddrs`, relay
  socket/Reach/TLS stages, node lifecycle ownership, and reqwest's resolver hook.
- Choice: one tracked native-resolution primitive in `vessel-relay::resolution`,
  with thin consumers for NodeLink and the dictation/usage reqwest clients.
  The primitive and the original two node-owned consumers need no new crate
  edge: the node already depends on vessel-relay. Usage has the separate
  injection/ownership boundary described below; no Harness edge is assumed.
- Rejected: a second DNS implementation/cache, an independent resolver actor,
  one pool per HTTP request, or a lease held only by a cancellable async wrapper.
- Why: the existing native dial crate is the lower shared boundary for these
  three consumers. Process helpers and pure proto/view types do not own DNS.
- Cost: stop may remain uncertain until the OS resolver returns. Explicit
  resource refusal replaces unbounded lookup admission. This is not a promise
  of cancellable DNS, a timeout on getaddrinfo, or bounded libc-internal memory.

### Source evidence

The pinned dependency sources are primary evidence, not a blocked-DNS runtime
experiment: Tokio1.54.1 `net/addr.rs`192–224 uses `spawn_blocking` around standard
`ToSocketAddrs` for a hostname, while numeric IPs return immediately. Its
`task/blocking.rs` documents that abort cannot stop an already running blocking
closure. Reqwest0.13.5 `dns/resolve.rs` defines Resolve and its port rules;
`async_impl/client.rs` selects default Gai unless `dns_resolver` is supplied.
Hyper-util0.1.21 `client/legacy/connect/dns.rs` starts the same native call;
its GaiFuture dropping/aborting the JoinHandle does not stop started work.

Current relay `socket::open_reaching` hides resolution inside tungstenite for
Reach::Any and awaits lookup_host for Reach::Public. NodeLink's STEP timeout,
outer cancellation and pending-child abort can drop those futures. Current
HTTP job abort/join similarly does not own the implicit Gai blocking closure.
Existing loopback and async-child tests establish neither native DNS drain nor
termination. The earlier isolated normal-registration-drain verdict stays intact
and explicitly limited; this ADR is the new item closing that separate gap.

## Shared ownership and API boundary

1. The proposed native module owns only validated hostname/port input, bounded
   socket-address results, resource leases, native work and drain receipts.
   It knows no registration, HTTP, credentials, request body, audio, book,
   session, provider, TLS policy or connection identity. It adds no DNS protocol,
   library, cache, process, runtime, periodic runner or independent resolver actor.
2. A native lifecycle root creates one shared `ResolutionBudget` and retains it
   across every client, recording and registration generation it owns. Consumers
   obtain a non-Clone `ResolutionScope<Hold>` and cloneable resolver handles
   for it. Hold is a statically typed opaque permits-only consumer hold
   (Send + Sync), never a ConnectionLife guard, registry, config, URL, key
   or audio value. The scope never inspects it. Every actual blocking work
   lease retains its original Arc<Hold> through native completion, including
   owner/caller drop; a clone is ownership, not a second counter.
   A handle cannot create/reset a pool, reopen a scope, mint a stop receipt or
   substitute another generation. Closing/dropping a scope closes admission.
   A reused endpoint/client must never silently construct a fresh pool.
3. The deliberately small API consists of bounded HostName/ResolutionPort and
   Addresses values, a budget, scope/handle, safe FailureKind, a state snapshot
   and an opaque NormalDrain receipt. Handle::resolve is fallible; scope close
   is synchronous; drain is cancel-safe and may be awaited again. Open,
   Draining and Drained are distinct. No public raw JoinHandle, source string,
   caller callback engine, credential or transport escapes this boundary.
4. Before any blocking spawn, atomically admit against the same scope and
   shared budget, publish the tracked completion, and move the actual work
   lease, including that original opaque consumer hold, into the native
   blocking closure. A short admission lock may protect
   bounded bookkeeping only; it is never held through DNS or an await.
   Close and admission must have one linearization order. Closing cannot miss
   an admitted-but-not-yet-started worker, even if it races spawn/publication.
5. Execute standard `std::net::ToSocketAddrs` on Tokio's existing
   `spawn_blocking` pool, carrying that lease until the call and bounded result
   preparation actually return. This is the same OS resolution used by Tokio,
   not a replacement DNS resolver. Opaque lookup_host alone cannot carry our
   lease into its internal blocking closure, so an outer cancellable wrapper
   is insufficient. No nested runtime/block_on or extra resolver thread is used.
6. Minimal tracked completion ownership retains/joins the blocking task even
   if its result receiver, dial timeout, HTTP future or caller is gone. A
   bounded per-work completion task on the existing Tokio runtime is allowed;
   it is not a second registration/resolution service. Normal close never
   aborts that join. The physical lease remains in the blocking closure even
   if an async completion observer is dropped. All admitted work is counted,
   including queued work which has not entered getaddrinfo yet.
7. A worker which has not crossed the native-start gate when close wins skips
   the lookup. A worker whose start won may run after the close request;
   normal drain still waits for it. A token check alone is not the proof.
   NormalDrain is minted only after admission is closed and every admitted
   blocking task is actually joined, with no remaining native lease. Dropping
   a drain waiter retains those joins and their capacity. An owner/runtime
   drop or forced abort is not a NormalDrain receipt and cannot produce a
   successful consumer stop, re-enable, released recording or new generation.
8. `getaddrinfo` cannot be cancelled. Its wait is bounded only by the OS
   resolver, not by STEP, an RPC/HTTP deadline or the scope's async waiter.
   A consumer deadline reports uncertain/draining and keeps its existing
   generation/reservation until actual completion. It refuses a replacement
   generation meanwhile. No lease or quota is released merely because a
   caller stopped waiting. A stuck OS resolver can therefore keep capacity
   unavailable; this intentional cost must remain visible, not be called a
   rollback or successful stop. Normal shutdown obeys the same drain rule.

## Finite admission and output

The proposed named limits are MAX_NATIVE_RESOLUTIONS=64 per lifecycle-root
budget, MAX_HOST_BYTES=255 and MAX_RESOLVED_ADDRESSES=64 per lookup. These are
Vessel resource limits, not values inferred from Paseo. Input validation and
both quota reservations precede copying/spawning native work. There is no
unbounded queue of waiting lookup callers: full/closed refuses immediately.

Each relay generation additionally admits one control lookup and at most the
existing16 data-open lookups; its17 slots remain charged through native work.
Repeated control retry cannot overlap another unresolved control lookup.
A dictation stream's scope follows its existing MAX_IN_FLIGHT_COMMITS=2;
the constructor receives that existing source constant rather than copying
the literal in the shared module. Its original permits-only recording lease
is retained inside actual native work while the scope drains.
The implementation and tests use these constants or the existing source
constants, not repeated literals. Closing a scope cannot be bypassed by
creating a replacement scope/client while its generation is still draining.

Results preserve resolver order, remove duplicate addresses within this one
result and retain at most64 raw records. If more records are returned, refuse
the whole lookup before dialing; never truncate into a partial successful
answer. That bounds application-retained results and iteration, not libc's
internal allocation before getaddrinfo returns. No addresses are cached across
lookups. Literal IPs may use the same admission/closed-scope gate without
starting native DNS; their result must not bypass the consumer's stop fence.

Native errors retain typed causes privately but public Display/Debug/source
chains expose fixed categories only. No URL, hostname, address, key, token,
body, audio, home path or panic payload is formatted by this primitive.
Capacity/closed/invalid-input/too-many-results/native-failure/worker-failure
remain distinguishable without introducing a new wire ErrorCode.

## Relay consumer (ADR0096)

The same NodeLink run owns its resolution scope, pending data opens and normal
stop. Its existing socket implementation is split into preparation/resolution,
TCP and TLS/WS stages, rather than duplicating a transport. Every TCP address
is the exact address already resolved and checked under the same Reach policy.
Try eligible unique addresses in resolver order at most once until the first
TCP success; use one TLS/WS handshake with the original authority/SNI, roots,
request path, ticket and existing nodelay policy. Do not resolve again inside
an implicit connect helper, add new retries or dial untried addresses after a
successful TCP selection. Preserve public-only filtering, including IPv4-mapped
IPv6 and the existing numeric-host policy. Too-large/empty results dial nothing.

Dial timeouts/cancellation may end network waiters, but the NodeLink still owns
and drains the resolution scope. Publish Stopped/run return only after both
pending final accept/discard callbacks and NormalDrain. The settings deadline
remains uncertain/disabling until then, and re-enable refuses instead of
starting a fresh scope. A drop/abort is not a normal ACK. Established encrypted
gateway carriers and independent fleet callback ownership stay independent.
This adds no endpoint/token/TLS input over the wire, method or version number.

## HTTP consumer (dictation)

The dictation owner supplies a thin reqwest Resolve adapter through
ClientBuilder::dns_resolver; it delegates only hostname resolution to the same
scope handle. The adapter contains no independent quota/cache/native fallback.
It returns port0 addresses; reqwest keeps the explicit URL port or supplies
the scheme's conventional port, as its documented hook requires. Its boxed
future/iterator/error are the external library's real open extension boundary;
the shared primitive remains typed and uses static dispatch internally.

Resolver work never receives the URL, Authorization, body/audio or provider
state. An endpoint Client clone does not grant a new scope. Existing HTTPS,
root selection, proxy/redirect disabling, retry and endpoint policies remain
owned by dictation. Dropping/aborting HTTP jobs is not normal DNS drain: its
existing generation owner retains its recording reservation and reports
uncertain/draining until NormalDrain, refusing replacement meanwhile. Wiring
the builder hook alone is insufficient: after Jobs::clear aborts/joins async
HTTP jobs, retirement must await the same scope before Registry entry.finished,
entry removal, or a replacement generation settings snapshot/preparation.
A retained permit with an early logical finished signal is not sufficient.
Preserve the existing first-stop reason and connection isolation, and keep
logical provider failure distinct from physical draining. The consumer
HTTP10s/60s and finish deadlines do not bound native DNS completion.
GKS owns the consumer wiring/tests; until shared and consumer checks plus
integration complete, its SINCE remains None and external calls fail closed.
No HTTP or dictation activation is part of this contract-only change.

## Verification and explicit limits

### Account-usage HTTP consumer (W5 activation hold)

The coordinator adds W5's `vessel-harness/src/limits/http.rs` `Http::new` and the
`AccountUsageProbe` Local body as the third consumer. It must use the SAME
tracked primitive through reqwest's `ClientBuilder::dns_resolver` hook, with
host-only/port0 results, the stable node-root budget and its own existing
resource bound/permits-only hold. No private resolver, cache, fork, credential
or configuration capture is introduced. HTTP cancellation/timeouts are not
physical DNS completion; the actual probe/lifecycle owner must close and drain
its scope before releasing its admitted operation or replacing its generation.
W5 records its known gap and keeps new account methods at SINCE=None until
this consumer wiring and receiving proof exist. Bounded backend preparation
may have a qualified pass with this explicit activation hold; neither that
pass nor the shared primitive claims active readiness or physical DNS drain.
This documentation addition does not request another W5 whole gate.

The Harness crate currently depends on proto/proc, not relay. Naming this third
consumer does not authorize a new Harness-to-relay dependency or relocating the
primitive. Before wiring, the owners/coordinator/V3 must settle the concrete
injection and retained-lifetime boundary at reqwest's real resolver extension
point. An outer lifecycle may supply that canonical resolver while retaining
the same scope and permits-only hold, but the exact receiving API and drain
ordering still require source review. No resolver interface of our own, erased
callback engine, parallel quota or fallback default Gai may fill the gap.

### Shared and receiving proofs

Before code, V3 reviews this shared contract and ADR0096's reference. Afterwards
prove admission-before-spawn, close-vs-start, queued-skip, physical lease through
waiter cancellation, retained joins after drain-waiter cancellation, full quota,
result limits/dedup/order, dropped receivers, owner drop and no false receipt.
A bounded real blocking-worker barrier must hold native work across caller
abort/timeout and normal stop. Use Tokio/standard localhost resolution only
for actual native plumbing only when it cannot leave loopback; numeric-literal
standard-resolution plumbing and a controlled native-worker barrier suffice
without any external DNS query. A controlled blocked worker is not a claim
to have hung an external OS DNS server. No public DNS, provider, real relay or customer
account is contacted. Test-only injection reuses the same work owner, not a
production fake resolver trait. Root tests use testkit outer safety plus named
finite phases; release/abort/reap test-owned barriers on failed waits/unwind.

Relay tests must hold both control/data resolution through stop and repeated
retry, prove no second generation before true drain, preserve final callback
ACK ordering/Reach/TLS/SNI and keep an established encrypted carrier alive.
HTTP tests use the real reqwest hook, hold a native worker after the HTTP future
is cancelled, and prove the original recording reservation stays charged until
normal drain; also prove entry.finished/removal and replacement settings
snapshot/preparation cannot precede drain. Preserve first-stop and connection
isolation; no replacement/fallback/default Gai bypass is permitted.
Canary host/error inputs must not escape safe Debug/Display/source chains.

Source review, narrow checks, the owning package gate and consumer integration
are separate evidence. No implementation, timing, DNS termination, supported
platform, resolver-internal memory bound, service-account behavior or full
consent claim follows from this ADR. Other unmigrated native DNS call sites
(such as push/client paths) are not silently covered by these three consumers.
