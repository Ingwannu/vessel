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
