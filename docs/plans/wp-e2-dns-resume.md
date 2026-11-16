# G4 tracked native DNS resume

Status at the quota warning: coherent shared primitive checkpoint; NOT a
consumer-ready or full-consent implementation. No owning whole gate is queued:
it cannot finish within the remaining warning window. Do not start another
slice, control, merge or long build in this stop period.

## Committed source and evidence

- Base bc9cacc67dba6cc8d3fb4f71fa10c31e5de4565a (announced node gate57).
- Contract f2779ab901e102d029c89c1ef5c9aefc41504450: V3 contract-source PASS.
- Initial c6dc48a2b/880c8edf4: shared primitive only. Exact880 first strict
  failed14 mechanical lint points; its runtime was never submitted. Keep red.
- Corrected compiled source244224f5e31878ff9d5cc68a8eedfe0539492cbf: Relay
  all-target strict0 and actual resolution tests9/0/0. Real changed Relay
  checking/compilation appears in logs. Old assertions/deadlines were kept;
  true single-blocking-slot queued work is a separate ninth test.
- Exact strict run devices-dns-narrow-20261116T032327Z-244224f5.
- Exact runtime run devices-dns-narrow-20261116T032529Z-244224f5; all four
  planned stages0 (web preparation, nested fake/node builds, target tests).
- Subsequent690ba8b50/38c33eada/8c7685d41 change documentation ONLY. Rust,
  manifests and lock remain identical to compiled244. They record W5 as the
  third consumer and the coordinator's injection/client-lifetime rulings.

Current API is vessel-relay::resolution: stable ResolutionBudget,
ResolutionQuota/HostName/ResolutionPort, non-Clone ResolutionScope<Hold>, weak
ResolutionHandle, bounded Addresses, safe Failure/FailureKind, ScopeState and
opaque NormalDrain. Native work owns the original permits-only hold INSIDE
spawn_blocking; one retained completion owns each actual blocking join.
Scope close/admission/native start share an order. Drain can be cancelled and
awaited again; worker failure never manufactures a normal-stop receipt.

## Pending review and receiving work

V3 SAME row305 is reviewing the implementation and corrected target, including
the latest W5 contract supplement. No implementation SOURCE PASS is claimed.
Keep the exact prior snapshots and receipts; apply any returned findings as one
owned correction batch, then check the changed source. No unchanged rerun.

The primitive has NO NodeLink/socket, node lifecycle-root or HTTP consumer
wiring yet. Do not hand it off as ready or assign/activate any method. After
review, receiving NodeLink work must retain physical resolution through control
retry/stop, split existing resolution/TCP/TLS/WS stages without re-resolution,
and preserve final callback ACK/established-carrier independence. The old370
settings preparation stays unreviewed and separate; accepted row231 proves
only its prior normal async registration/callback drain.

Dictation's per-stream scope must retain its original permits-only recording
lease, then drain before Registry.finished/removal and replacement settings
preparation. GKS owns that receiving work; endpoints stay SINCE=None.

Usage is deliberately DIFFERENT: one Node-owned scope for the persistent
LimitContext OnceCell client shared by saved/additional reads. Its canonical
physical quota uses the existing bound/root budget. Per-read two-slot HTTP
admission is separate, not DNS proof. Do not pin the first initializer permit
or use mutable/task-local request association. Harness keeps proto/proc only;
the node injects Arc<reqwest Resolve> through the real builder hook and retains
scope ownership. Close/drain after source jobs before client replacement/node
stop. No injection means new methods stay None/fail closed, never default Gai.
W5 holds code until this contract and the shared producer are reviewed.

Numeric standard-native plumbing and controlled blocking holds are scoped
evidence, not an observed hung external DNS server or whole-host termination.
No external DNS/provider/relay/account, service, permission or other-OS test
was performed. No fake fleet/display belongs to this DNS item. Both VM runs
above finished; there is no queued DNS run to replay.

Next session first reads this file and the track's retained source-review/audit
packet, checks VM status and reviewer disposition, then continues only the
pending shared/receiving slice. All Cargo/rustc/metadata remain VM-only.
