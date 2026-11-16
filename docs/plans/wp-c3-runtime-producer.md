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
