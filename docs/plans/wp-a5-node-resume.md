# A5 node resume: admission corrections

Status at the quota warning, 2026-11-16: coherent work committed; changed-source
checks and runtime acceptance are incomplete. Start by reading WORKER's current
build rules and checking the existing VM run IDs below. Never resubmit a run
that is queued, running or already complete. No new slice or merge during the
quota stop.

## Committed source and boundaries

- Announced receiving head: ee8e345013d6756dfc55b1ed008a352db32786c7, merged by
  ea3bd962ed33befd29fb8b3a337895b1ff7d710a. Gate60 is counted17/18 with named
  TRACKER774, not wholly green. Protocol25, MIN2, canonical schema21.
- The head includes qualified GKS marking lease ac503bbe/a43e1367 and W2
  Parentage SQL scalar7202982b, without the unreviewed clock/SessionCommit
  branch. Same-connection membership and canonical relation values are present.
- Actual executable correction: ac1d2bb72b3895c8cb086be868563e41ea383594.
  read_attention obtains the SAME marking_lease before retaining a task, then
  moves the owned guard through existing pending veto, Store commit and live
  publication. No new lock, queue, authority or inherited mark_live repair.
- Earlier correction1e7c9eaa4 puts notification_current's existing canonical
  authority/source check in one borrowed-connection read transaction; no async
  wait or duplicated policy. Two auto-commit queries could mix incompatible
  device authority and Seen/Unread states. V3 confirmed this second P2.
- Shared SESSION SQL now calls vessel_has_current_parent inside all three
  existing candidate/source/final-lease consumers. Archive/side-question
  exclusions and before-LIMIT/statement joins remain. Current global parentage
  is distinct from immutable Creator audit and workspace-relative display roots.
- The early batch-count check avoids allocating the distinct-id set for an
  oversized attention request. Existing bounds/errors/scopes remain.
- Own five attention methods remain pending under None. Own claim migration
  remains22 after canonical21. No protocol assignment, workspace mutation or
  duplicate clock is added. Existing Inbox Web Push remains active; None gates
  the new local claim and terminal push paths.

The receiving conflict retained canonical runtime assembly/early-stop test
last in node/setup.rs and the existing single A5 push helper. Measured node.rs
record is lowered651 to646, never raised. Source formatting, size and diff
checks pass; this is not Rust execution evidence.

## Exact pending VM run

`wp-inbox-node-20261116T033059Z-ac1d2bb7`: ordinary lane, executableac1d2bb72,
Proto/Store/Session/Node/Client all-target Clippy with warnings denied. It was
queued at place43 at03:36, so completion within the warning window is unlikely.
Recover its immutable result first; do not cancel or duplicate the VM run.

After this check passes and work is resumed, run the actual Node+Store library
functional suite on the same source (ordinary VM, fake binaries supplied by the
runner). No new functional submission has been made after the quota warning.
The existing native test run is recorded in the app resume document.

## Authored but UNEXECUTED regressions

- Node sessions::tests::attention_admission: hold the actual common mutex and
  poll/drop sixteen read futures, asserting no retained tasks/effects; then hold
  a real SQLite writer/FIFO Pin ACK, drop an admitted caller and retain exactly
  one lease/task through real durable and live drain. No fake Store/ACK.
- Store store::inbox::tests: same exact source excluded while Attached, eligible
  after actual durable detach with Creator unchanged; actual candidates, local
  current/claim and final push lease. Seen/Unread keeps claims once-only, and
  real device revocation still vetoes its authority/lease.
- The old root-after-attached-page case is moved beside the private page
  constant, preserving the Root case and adding Detached without repeated page
  bound literals or a new public testing API.
- Store store::inbox::tests::snapshots: actual public final recheck with one
  private pooled-reader temporary identity view. A bounded observer pauses the
  real authority SELECT, the real writer commits revoke then Unread, and the
  original snapshot stays false. Writer-first, fresh Device/Owner and initial
  positive controls are included. Only the existing blocking reader thread
  waits; no production pause hook or authorization-policy copy is added.

These six test names/cases have no executed result yet. The local transaction
source correction and acquire-before-task correction are NOT V3-closed from
producer acceptance, source inspection or earlier passing tests.

## Review and remaining work

V3 owns original row162 full later-runtime audit and both new P2 corrections.
Its immutable full source pin remains1c762ccf..b97df3d4; later bounded correction
source/checks must be reported separately. Original A7 bound return was closed
only for its named slice. Pure carrier/source approvals do not approve runtime.
The current changed-source strict result, actual six regressions and complete
remaining security review are owed before runtime adoption/assignment.

After actual evidence: send the bounded correction packet to V3; coordinate
protocol assignment only with the node integrator. W17's configured CLI hook
producer, assigned positive wire tests, whole root/native gates, final captures
and full A5 review remain owed. Do not call this package complete.

Workspace marks remain design-only until lifecycle's durable updatedAt and
SessionCommit are qualified. Keep require_active, canonical membership/roles,
current attention/clock and mutation on the SAME writer connection. W4's live
response/card port, W21's safe opener and C2's actual TerminalId focus/resident
producer remain owner dependencies; do not copy their registries or invent
targets. The app receives runtime helpers only through an announced lead base.

No fake fleet, virtual display, browser, provider or local service is active for
this slice. Detached VM runs may continue; local log observers can be stopped
by their individually verified PIDs without cancelling those VM runs. Resume
by attach, not resubmission.
