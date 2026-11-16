# C3 comparison/history node producer candidate

Status: coordinator-authorized bounded producer; NOT reviewed/numbered/adoptable.
This straight candidate is based only on the announced node headbc9cacc6. It
contains the C3-owned runtime source from the already-merged development tree;
it does not bring an entire moving worker branch into the shared line.

## Concrete scope

- git.compare and git.compare_file implement distinct Uncommitted, Committed
  and All semantics, original-root revalidation, explicit pinned/default-base
  handling and lazy bounded typed file pages. Untracked All reads use a private
  index whose owner awaits cleanup after success, failure and cancellation.
- One64-payload comparison cache retains node-tracked active work independently
  of delivery, with root/project-aware before/after invalidation and no cache
  hit standing in for root authorization.
- git.history preserves classified workspace/base commits, publication against
  observed remote refs, complete commit metadata and typed refusal/size states.
- Git writes serialize under the existing authoritative project identity and
  revalidate the SAME snapshot after admission. Existing C5 file mutation and
  duplicate operations reuse ONE retained physical-work owner/permit, keeping
  cache invalidation through actual completion and partial errors.
- Typed registrations, forwarding and generated history declarations accompany
  those implementations. GIT_PANEL stays None; only the integrator may assign
  a version. No public successful RPC or native UI acceptance is claimed.

## Exclusions and prerequisite evidence

The later pure metadata decoder/observation design is deliberately excluded
from this candidate; it is not a watcher. No status/diff subscription, background
fetch/recovery, new merge-to/from-base, workspace metadata generation service,
checkpoint restore hook, physical RootLease/CAS, C3 native UI or C4 forge work is
included. ADR0087 records remaining purpose contracts, not implementation.

Earlier exact22ab checks were strict four-crate0, actual workspace88/0/0 and
TS6/0/0/wire difference0; these are PRIOR-source evidence. The development94e Node/Proto/Client strict and Workspace strict are0, with
identical relevant core Rust except its separate metadata-module export. Those
are NOT candidate-SHA execution/gates. Earlier results stay separately attributed.
No worker whole gate is run: the coordinator explicitly assigned that whole
to the integrator's receiving gate after V3 review.

[Decision Log]
- Purpose: make the implemented read runtime concrete enough for the coordinator
  to decide whether it may be a separate shared producer before the remaining
  full C3 package, so native consumers can eventually exercise real methods.
- Existing constraint: A14 permits one whole gate per feature package. The coordinator authorized
  this bounded producer with ordinary exact-candidate narrow checks and V3;
  the integrator's priority receiving gate is the whole. No worker whole.
- Choice: isolate only owned source on the announced node head, keep unassigned
  methods None and record all missing behavior. Do not import this whole node
  candidate into an App tree or present the full C3 package as passed.
- Next: ordinary exact-candidate all-target strict for all six changed crates,
  actual library runtime cases below, and generated-wire freshness. Then ONE V3
  runtime review, integrator receiving whole/number decision and B0/lead receiving
  after its announcement. GIT_PANEL remains None here; full C3 stays incomplete.

The generated proto.ts whitespace is retained byte-for-byte from its verified
generator artifact; do not hand-normalize it. Freshness against this new base
still needs the prescribed generation/gate. No release, push, credentials or
live repository operation occurs in preparing the candidate.

## Ownership and data flow

| Boundary | Responsibility | Lifetime |
| --- | --- | --- |
| Proto / Client / RPC / web bridge | Typed request, reply and forwarding; unassigned feature admission | GIT_PANEL None; no successful public method claim |
| service/git_comparison | Route the three typed requests through Workspaces and map one typed error boundary | Delivery only, not the read's cleanup owner |
| workspace/roots | Capture project/worktree/default/pinned facts from one registry snapshot and open the recorded root | Rechecked around awaited opens; not RootLease/CAS |
| workspace/comparison + operation | Normalize comparison identity, admit the pinned base, prepare the lower read | Original request snapshot retained by the operation |
| workspace/comparison/cache + jobs | Share active reads, bound completed payloads, cancel stale commands and await cleanup | Node TaskTracker owns active work after a receiver is lost |
| vessel-workspace git/diff | Compare real repositories and manage the private temporary index | Explicit awaited cleanup before operation completion |
| workspace/git_history + git/panel_history | Read complete workspace metadata and bounded base context | Same node-owned job budget; separate from cached comparisons |
| workspace/git_writes | Bound writers and serialize one recorded project, then revalidate its original root facts | Existing per-project lease; no external-writer exclusion |
| service/file_panels + workspace/files/mutations | Keep C5's one physical task/permit and before/after comparison invalidation | Guard remains inside actual retained disk work through errors or lost delivery |

This reuses Tokio tasks, oneshot/watch delivery and existing semaphore admission
rather than another transport, pool or root registry. Git remains the existing
subprocess boundary; the cache's short synchronous lock holds only coordination
state, never an awaited disk or subprocess operation. Completed payload eviction
does not discard an active task or its private-index cleanup.

## Exact checks registered; source-audit successor pending

The ce9667ac candidate has three ordinary/priority0 immutable runs registered:
strict six-crate all-target, six-crate library tests and TS generation freshness.
Their run ids and hashes are retained in the external review packet. No result
is inferred while a run is queued. The66 named cases are an execution checklist,
not a substitute for their individual test output.

A subsequent self-audit found a specific remaining registry race in this source:
prepare_comparison opens a captured snapshot, but a cache hit can bypass the
post-open registry check performed only by Operation::run on a new build. A
registry change during the awaited open could therefore return old cached data.
The successor reuses resolve_unchanged before the cache probe and adds
a real blocking-pool regression for summary/body hits and changed path/default.
It is now installed but uncompiled/unexecuted; this candidate is NOT ready
for V3 runtime submission even if the existing checks finish green. Preserve
those outcomes on their exact source and group the correction with any actual
returned findings before one meaningful successor check. No separate whole.

## Cache-probe correction: behavior change

The successor refuses a registry path/default change that occurs while its
initial cache-probe root open is awaited, including a cache hit. This is an
authority-fact revalidation change, not just lint or documentation. It uses the
existing resolve_unchanged wrapper before constructing the cache identity;
Operation::run still rechecks a newly executed job. No resolver, lock, pool,
protocol number or physical RootLease is added.

The new real Git fixture populates one cached summary or file body, occupies
the actual one-thread Tokio blocking pool, polls a second read until its root
open is queued, then changes path or project default and releases the pool.
All four cases must return TargetChanged while the completed payload remains
in storage. Before the production correction the hit skips Operation::run and
returns the old result; that baseline behavior is source-traced, not claimed
as an executed failing test. The existing ce9667ac remote jobs stay attached
to their old immutable source and are not execution of this successor.
