# Session lifecycle resume boundary

Work is paused. Resume only after explicit continuation, from the announced node
head rather than this branch's unreviewed attention-mark implementation.

## Completed producer

The clock implementation e0c7f08d passed source review and landed in 13b01a92.
Its owning integrated gate passed all eighteen stages: Rust 2862/0/12, web unit
465, browser e2e 80/80, forge fixture 4 and fresh generated declarations. Schema
22 is MUTATION_TIME_SCHEMA. SESSION_MUTATIONS is still unassigned; no positive
wire availability or complete lifecycle feature is claimed.

## Separate attention correction

The canonical correction is owned by the separate integration carrier:
3f4d8d615 contains its source and tests, a8456deff its documentation. Those
candidates are not consumed by this branch. Workspace selection must wait for
the carrier's qualified, announced landing.

Local 7d7a50fda records the contract, 3d855c94f the test-first regression,
c95932861 an unreviewed implementation and 7a9f843a1 stricter fixture setup.
They are preserved as drafts; do not merge or present them as the canonical fix.
The test-first loaded clock equality failed by one millisecond. The c959 Node
and Store library run passed 486 tests, but its strict run failed on three
unused fixture row counts. The later fixture-only change has not been compiled.
There is no complete gate or source approval for this local correction.

## Deferred work

No quiescence implementation has started. ADR 0103 is a contract only; complete
Linux descendant settlement, typed agent wait/drain and the ended-run proof are
unimplemented. Windows and real-agent behavior remain unverified. Current group
cleanup receipts cannot supply that stronger proof.

Retain the designated ownership of stop and invalidation joins. Do not duplicate
the shared observer or adopt an unannounced producer while resuming.
