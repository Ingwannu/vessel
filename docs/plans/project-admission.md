# Project admission and reserved backing identities

## Decision and reference

Modify existing modules. The node already owns project registration, ready-root
resolution, a tracked worktree creator and durable project/worktree records.
Extend those paths rather than add a registry, filesystem worker or dependency.
Tokio's existing owned mutex/semaphore guards provide cancellation-safe ownership;
no additional library or external API is needed.

Paseo's `packages/server/src/server/session/workspace-provisioning/
workspace-provisioning-service.ts:220–304` requires an active project, accepts a
caller-reserved workspace identity and records the placement before publishing it.
`packages/app/src/hooks/open-project.ts:80–114` treats project registration as an
independent operation. Vessel keeps its existing independent typed identities and
makes the project admission explicit across backing creation and descriptor commit.
This package is an internal prerequisite, not the complete provisioning flow.

## ADR 0082 supplement: retained project admission

[Decision Log]
- Purpose: prevent a registered project from disappearing between resolving its
  root, creating reserved backing and finalizing a workspace descriptor.
- Existing constraint: root resolution previously released the metadata lock
  before asynchronous backing work; the legacy creator minted its own id.
- Alternatives: recheck only after effects, hold a synchronous mutex across I/O,
  or create a second provisioning registry. Those leave an effects race, block
  the executor, or duplicate the authoritative registry.
- Choice: one owned asynchronous project metadata mutex, retained in an opaque
  `Arc<ProjectAdmission>`, plus a non-waiting semaphore admission budget.
- Reason: the provisioner and tracked backing task can each retain the same hold;
  cancellation of the RPC cannot release another holder's protection.
- Tradeoff: this coarse mutex serializes admissions across projects. The named
  budget of 64 includes the holder and admission waiters; exhaustion refuses
  before waiting or creating backing. Registration/removal also join the mutex.

`projects::admission` owns the hold, exact `Workspaces` allocation/project check,
and checked root. Registration deduplicates under the same lock and persists
before publishing, inside a tracked task so caller cancellation cannot interrupt
that sequence. `remove_project` waits before inspecting or changing its record.

`worktrees::create` reuses the existing creator with a supplied `WorktreeId`.
The admitted entry requires `SetupChoice::Skip` before any effects. The caller
retains its own admission through descriptor finalization; passing a clone to
backing creation does not transfer that responsibility. Existing and in-flight
ids refuse replay, including two calls sharing one admission. The existing
`Making` lifetime releases the in-flight id and creator capacity together.
The legacy entry retains `NotGit`/`NotReady` outcomes and may still run its
existing setup; it is not the managed workspace provisioner's entry.

## Authority limits and integration

The hold protects node project registration/removal and admitted metadata work.
A later typed project-config writer must join this same admission. No such
config I/O contract is added here. Generic file/git operations, external editors,
symlinks and filesystem replacement are not frozen by a metadata mutex. The root
is checked at admission; the hold does not attest continuing physical identity.

The workspace provisioner must separately reserve/reconcile its durable ids and
retain the hold until its descriptor transaction settles. Initial local placement
uses the checked project root without filesystem mutation; managed backing uses
the admitted Skip entry. Nested config-derived rerooting is separate work.
Fresh configuration/approval, blocked-source release, canonical backing generation,
physical-root exclusion, process start leases and actual stop/join proof remain
independent requirements. This package supplies none of those authorities.

The existing legacy project/worktree deletion routes still need registry-aware
fences after the registry dependency lands. A hold through descriptor finalization
does not assert that these later lifecycle operations are already integrated.
No wire call, protocol/schema number, migration or new crate edge is introduced.

## Validation and resume point

Tests exercise final-holder unregister exclusion, allocation/project mismatch,
admission capacity, bootstrap refusal before effects, fixed-id preservation and
replay refusal, in-flight duplicate refusal, legacy typed refusals and concurrent
registration durability. They use isolated temporary repositories and existing
fake/test facilities, not real providers or user projects.

Source and static checks are prepared; remote changed-crate strict checks and
functional tests remain owed. The package is not adoption-ready. After those
checks, run the prescribed whole root gate once and submit the immutable range
for source review. The real workspace provisioner consumes only the checked,
reviewed producer. No native UI or filesystem provisioning adapter is included.
