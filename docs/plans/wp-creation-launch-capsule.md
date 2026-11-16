# Concrete creation launch capsule (staged)

[Decision Log]
- Purpose: the bound creation owner must own the original capture, exact prepared
  command, count correlation and retained placement across admission/settlement.
- Existing constraint: count insertion or an unlocked stamp is not permission to
  spawn; the same metadata mutex and Agents guard must remain authoritative.
- Alternatives: an arbitrary callback, a token returned without the command, a
  second registry, or one concrete single-use owned command capsule.
- Choice and reason: cfg(test) agents::creation_launch::Candidate owns boxed actual
  SessionSpec/LaunchOptions, ORIGINAL CapturedParentExecution and canonical count
  correlation. Preparation/allocation occurs before the Agents guard. Only an
  Agents descendant can inspect the fixed capture/count/deadline predicates and
  consume insert() into TrackedLaunch. Insertion moves the existing allocation
  without await, recapture, callback, I/O, spawn or allocation.
- Impact: this is explicitly staged API source for the joint owner implementation,
  not a qualified current grant or a production create API. The common fence must
  perform all checks plus count-state change and consume insertion under ONE
  actual registry guard. No free-standing Authorized result is introduced.

Sessions alone constructs CreationPlacement from its actual owned metadata lock.
Project admission must be acquired BEFORE that lock, matching teardown order.
The factory checks the authenticated parent id's explicit Store membership,
Active canonical descriptor and expected revision, exact project-admission owner,
canonical project/worktree root and immutable descriptor path, then claims the
child's same-Store start lease with Active/revision CAS. It retains the exact
Sessions owner, project Arc, resolved root and start metadata through settlement.
None of this proves current parent permission, physical continuity or generation;
the captured execution and common guarded admission remain independent. A request
cannot choose the parent used by the actual controller; candidate binding checks
it against the original capture. Global64 project serialization through actor
settlement increases unrelated-project latency beyond the earlier through-
descriptor hold (Tracker765); it is not per-project concurrency. Existing legacy
removal fencing is separate and does not make a pathname a physical lease.

Candidate preparation requires the exact Resource Begin operation/revision and
child/workspace, original parent identity, resolved cwd, structured mode, no legacy
place/delegated via, and the exact local Agent creator. Its finite monotonic five-
second preparation age is checked again by the common owner. Refusal retains the
concrete input, and cannot silently fail or release an interrupted pending start.
The tracked driver retains AgentReservation independently of its reply/capsule.
Original capture observation is read-only outside the Agents guard; invalidated()
waits on that SAME watch. It is never used to authorize insertion.

Actual spawn/normal-stop/RevokedAfterAdmission settlement is NOT implemented by
this value cut. The owner must keep pending spawn work through invalidation and
stop any resulting child, preserving uncertainty on caller loss, deadline or
failed stop. The three revocation orders, lost-watch-gap handoff, exact parent/
count checks, positive constructor, caller-loss and phase/failure-owner tests are
still owed. Capture itself remains staged pending its separate A7 fixture closure.
No fake caller, blanket dead-code expectation, RPC, Store/schema or protocol
assignment is supplied to manufacture standalone readiness. Compile/strict/runtime
of this joint candidate and its real consumers remain owed.
