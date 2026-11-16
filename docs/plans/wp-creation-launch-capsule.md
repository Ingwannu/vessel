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
No fake caller, blanket dead-code expectation, RPC or protocol assignment
is supplied to manufacture standalone readiness. The separately staged canonical
Store dependency now permits actual Begin acknowledgements in constructor tests. Compile/strict/runtime
of this joint candidate and its real consumers remain owed.

Common-owner source review adds two fixed preconditions: Candidate.owner_matches
compares the retained Sessions owner's actual Agents pointer with the registry
being locked, rather than treating matching UUIDs as owner identity. Candidate
uses checked_add for its finite monotonic deadline before consuming count
correlation; range failure returns the same retained input. Placement Debug now
prints only correlation ids, never recursively traverses the retained Sessions
owner. These checks add no authority token or constructor and still require
actual common insertion and settlement qualification.


The constructor fixture reuses the actual Sessions fake harness runtime, Project
registration/admission, Store registry descriptor/start leases and canonical
receipt writer. Parent membership is completed only after actual fake actor
registration/ready; its principal comes from the actual credential admission.
Child placement reads the current canonical revision and obtains the sealed
Sessions-owned hold. Begin is the Store writer's committed ACK, not Receipt::advance
called in test code. This tests construction, not common parent-grant insertion
or physical backing continuity. Each async phase has a named 15-second bound
inside the 120-second outer safety net. Test assertion/phase panic retains the
SAME qualified fixture cleanup actor through actual runtime stop, task drain
and Store close, then resumes the failure. Observation deadlines do not cancel
that actor; sibling reuse is recorded in the fixture-owner plan. A deliberate assertion control verifies those effects.

The retained placement also captures the original runtime LaunchAdmission from
that same Sessions owner. Candidate.check_runtime forwards its synchronous close
predicate, retaining SessionError for the common refusal path. This observer is
additional refusal evidence only, not an execution-incarnation permission or a
current accepting/launch-spawn guarantee. Actual runtime internally performs its
own early-stop checks. No new token, registry or permission cache is introduced.
The constructor regression checks both open and actually shut-down runtime cases.


## Staged child lifecycle custody

Sessions::keep_bound_child accepts an actual runtime Created after the tracked
admitted driver has already owned spawn; it supplies no spawn/current-grant API.
After actual Store registration it MOVES the original CapturedParentExecution
into the child's existing tracked persistence/hand_over lifecycle. There is no
receiver clone, recapture, second child registry, separate detached monitor or
creation permit retained for the child's lifetime. Project/metadata/count/start
settlement remains the creation driver's separate obligation.

The child persistence owner checks original observation and selects that same
watch against actual durable drain. Pending-before-first-poll invalidation stays
observed, unavailable/grant-loss is not treated as success, and simultaneous
completion prefers the invalidation branch. Revocation calls normal handle.kill
and continues to own persistence and hand_over. Registration failure also kills
and retains actual cleanup through hand_over, not just a timed reply. Original
capture is continuously owned across the move and through final child custody.

RegisteredBoundChild returns the actual handle and historical parent observation.
It is not a public settlement or KnownNoSpawn result. The controller must still
own pending spawn, stop supervision, exact start/count/receipt settlement and
RevokedAfterAdmission persistence; lost caller/deadline/failed stop never release
physical work. Those joins and the three logical admission orders remain owed.
Legacy creation/persist is unchanged. This entire lower seam stays cfg(test).


Three lower-layer regressions use real fake child processes: invalidation already
pending before custody's first poll, invalidation after registered custody, and
normal child completion while the parent remains live. Each waits actual ended
and durable Store hand_over. Their helper starts the actual Created future in
Sessions' existing TaskTracker, so a cancelled observer cannot discard it. These
are lifecycle custody tests only: they do not exercise the unfinished creation
controller, physical backing, first prompt, public receipt classification or
current-grant launch admission. The common owner's six separate staged cases
exercise original-parent/count/Begin checks and remain independently attributed.
