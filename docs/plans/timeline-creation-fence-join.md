# Fixed creation admission join (staged)

This isolated development tree composes the existing original-execution capture,
creation count reservation, concrete creation capsule and retained placement.
The common join is registered only under the existing literal test boundary.
It grants no product launch path and is not compiled or runtime-qualified yet.

[Decision Log]
- Purpose: close both count-reservation and post-Begin launch-admission races
  without an unlocked permission value or an arbitrary callback under Agents.
- Existing assets: one Agents registry guard, original non-serialized capture,
  `reserve_prepared`, exact count marker/Begin correlation, and the creation
  owner's concrete `Candidate`/`TrackedLaunch` box.
- Alternatives: checking a snapshot before an await loses the invalidation
  order; holding Agents across Store or spawn blocks synchronous retirement;
  a generic callback permits unrelated work under the authority lock.
- Choice: two fixed synchronous methods use the actual existing registry guard.
  Credential preparation happens outside it. The first checks the captured
  parent and reserves the child through the existing count owner. After the
  actual durable Begin acknowledgement, the second rechecks the same parent,
  exact placement registry, child count marker and deadline, then consumes the
  concrete capsule before releasing that same guard.
- Impact: failure returns the exact candidate, retaining its pending start,
  metadata and project holds for the owning driver's settlement. Success is
  logical admission only; physical spawn, revocation supervision and lifecycle
  handoff remain separate obligations of the concrete creation owner.

`reserve_captured_creation` accepts only an already captured direct local parent.
It normalizes the child grant to Read/Operate and calls `reserve_prepared` under
the same guard as the original incarnation/current grant check. The resulting
existing `AgentReservation` stays with the tracked driver. It is not a reply or
a transferable permission, and count reservation does not authorize launch.

`admit_captured_creation` checks the original registry pointer/node/session and
incarnation, the original watch, a current matching non-ended parent handle,
saved/current Read/Operate, the retained Sessions owner's actual Agents pointer,
exact child, unconsumed count marker/Begin and finite candidate deadline. Count
transition and `Candidate::insert` occur in that single guard interval. No
await, Store access, allocation, logging, foreign mutex or callback occurs there.

The insertion's concrete box retains the original capture/watch and placement;
it cannot be rebuilt from a serialized stamp or a later current-looking read.
A rejected candidate is returned intact, rather than dropping a pending child
start or misclassifying an uncertain operation as a fresh retry. The count
owner's exact pre-admission refusal protocol remains responsible for rollback.

Six authored cases use the actual Store-backed Begin/placement fixture for
current-grant/count admission, invalidation before insertion, original-watch
ownership after insertion, runtime closure, replaced count markers and the
existing final count slot. The fixture reuses the retained capture cleanup task,
including setup/drop cancellation of its actual workspace owner. Runtime closure
keeps its typed source. Child spawn/stop and all three full revocation orders
remain unimplemented and unqualified.
The existing `LaunchAdmission` observes early node/runtime closure only; it is
an additional refusal source, never current parent authority. Before insertion,
invalidation must refuse with no spawn. After insertion and before settlement,
the tracked owner must retain spawn through settlement, stop the resulting child
normally, and report `RevokedAfterAdmission`; stop failure remains uncertain.
After settlement, handoff to ordinary lifecycle must have no lost-watch gap.
Neither a deadline nor observer/caller loss proves physical release.

Physical root continuity/generation, the immutable historical membership relation,
Active start-CAS/drain and current parent authority retain their separate owners.
No cloned start lease is promoted into a live parent or launch guard. The held
placement uses the actual existing project-before-metadata-before-Agents order.

No production registration, RPC, protocol allocation, schema, Store writer,
provider or actual spawn/stop controller is introduced by the test-only
common source. The receiving fixture and all changed-source checks/review remain
owed before even test-only composition is qualified.
