# Shared native DNS lifetime: contract item

Base: announced node bc9cacc6 (gate57). Contract: ADR0104; relay consumer ADR0096.
No product source, manifest, lock, method, version, schema or service change.

Existing assets inspected: relay socket/NodeLink/DataOpens and public Reach,
Tokio1.54.1 standard native-resolution implementation, reqwest0.13.5 resolver
hook, hyper-util0.1.21 Gai lifetime, and dictation's endpoint/jobs/quota model.
Choice is a shared typed lifetime wrapper in the existing native dial crate,
with thin NodeLink/HTTP consumers. It does not introduce a DNS implementation,
cache, library, runtime or separate registration/resolution manager.

The crucial capacity lease lives inside the actual Tokio blocking closure
which calls standard ToSocketAddrs, not merely the async lookup waiter. The
same tracked generic scope moves the original permits-only opaque consumer
hold into that actual closure and retains/joins physical completion across
cancellation and
close. An OS-bound wait can remain uncertain indefinitely; callers may time
out but may not release admission or claim normal stop/re-enable/recording
completion. The node-root budget survives consumer generations.

Proposed bounds and typed failure/drain receipts are in ADR0104. V3 contract
review is the next step, before any source/API implementation. The existing
isolated normal-registration-drain source/gate stays accepted only within
its earlier bounds. Old370 remains unreviewed preparation, not a consumer API.
GKS keeps endpoint methods SINCE=None and implements no separate resolver.

After contract acceptance, implement the shared owner and deterministic native
worker barriers first; then isolated NodeLink stage/consumer tests; then actual
settings/HTTP consumer lifetime and integration. Every compile/test/gate runs
on the approved build VM, detached, once per meaningful frozen source. Final
package and consumer evidence never borrow an unchanged predecessor gate.

Current deliverable is documentation only. No DNS lookup, HTTP/provider,
real relay, account, permission, process or production action was performed.
If work pauses before V3 answers, resume at that pending contract review.

Consumer direction confirmed by GKS before code: per-stream original
Arc<dictation quota Lease> is permits-only, quota constructor uses the existing
MAX_IN_FLIGHT_COMMITS constant, and retirement must gate entry.finished/removal
and next-generation settings snapshot/preparation on actual scope drain.
First-stop reasons and connection isolation remain unchanged; a builder hook
or retained permit alone is not completion. GKS source/whole remains frozen.

## Shared primitive source checkpoint

V3 accepted the exact f2779ab90 contract before source installation. The first
implementation adds only vessel-relay::resolution and its registration, with
no manifest/dependency/lock/wire changes. A stable ResolutionBudget creates
non-Clone ResolutionScope<Hold> owners; weak ResolutionHandle values resolve
HostName/ResolutionPort under the root and constructor-supplied scope quotas.
Scope close, admission and actual native-start decisions share one small
bookkeeping lock; no lock is held through resolution or an await. Native work
and its original permits-only Hold live inside the actual blocking closure.
One bounded completion task retains each real blocking join after a result
waiter disappears. Drain closes admission, waits for every observed join and
never converts worker failure or runtime/owner drop into NormalDrain.

This source does not connect NodeLink sockets or an HTTP client. Neither uses
the primitive yet; their existing lifetime gaps stay open. No node lifecycle
root pool or dictation registry consumer is claimed ready merely because a
public source API exists. Normal stop can remain OS-bound and uncertain.

Authored tests hold the actual blocking closure before/after its native-start
gate, lose callers/drain waiters/scope owners, exhaust the shared root across
scopes, preserve original consumer holds and test failed-phase unwind. Numeric
standard-native plumbing, raw-address overflow/dedup/order/empty refusal and
all safe source-chain canaries are distinct checks. These are unexecuted until
the exact VM receipts exist; a controlled native-worker barrier is not an
observed hung external DNS server. All tests use named phase and outer bounds.
