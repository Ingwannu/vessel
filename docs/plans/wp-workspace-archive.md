# Workspace archive and recovery

Status: staged; identity types are reviewed, registry persistence is a separate dependency.

## Reference and existing assets

Paseo 0.15.4 `packages/server/src/server/workspace-archive-service.ts:136-205`
stops setup, marks archiving, archives exact workspace contents, then considers
backing deletion. Lines 515-564 select live/stored agents by workspace ID and
continue after individual teardown failures. Last-reference and ownership checks
precede disk removal; a failure can leave an archived directory behind.
`server/session/workspace-recovery/workspace-recovery-service.ts:95-185`
reinspects project/directory availability, unarchives an existing directory or
recreates a recorded worktree, then unarchives the workspace. It does not resume
its agents or rerun setup as part of recovery.

Existing assets: `WorkspaceId`/`WorkspaceOperationId` are the canonical UUID
identities; `RemoveWorktreeOutcome` carries retained/removed backing evidence.
The session archive and terminal shutdown paths already exist. They lack a
workspace-wide ownership/start-admission fence, so applying them by cwd would
stop unrelated resources and race provider startup.

## Decision Log

- Purpose and intent: archive only resources admitted into one durable workspace;
  recovery keeps that workspace identity and never silently starts its agents.
- Constraints: registry/store dependency is being reviewed independently; no
  duplicate registry DTO, UUID implementation or path-based owner lookup is added.
- Alternatives: infer ownership from cwd; archive metadata before draining leases;
  retry failed create/restore blindly; add an external lifecycle library.
- Choice: modify existing resource lifecycle through a small generic backend
  boundary. Associated claim/plan types keep registry revision/ownership opaque
  until its reviewed persistence adapter can be connected. Real workflow rules
  are exercised against an in-memory backend under A10 staging.
- Why: an exact ownership claim precedes shutdown, late actor registrations are
  settled before member enumeration, and backing cleanup is unreachable after
  an unproven shutdown. Existing Tokio and standard types are sufficient.
- Advantages, costs and effects: testable sequencing and retained typed causes;
  final product wiring and registry/physical-filesystem checks remain required.

## Staging and lifecycle requirements

The module is test-only until the registry adapter, request/response schema,
existing RPC guards and consumer wiring are complete. The pending method table
is available; protocol availability remains None until integration assigns it.
No new wire method is invented in the native app before the node implementation.

Archive claims persist Archiving before any stop. The backend waits through the
registry watcher until all earlier start leases settle; uncertain leases require
authoritative runtime reconciliation. Member admission is rejected while a claim
is active. Every member is drained even when another fails, but a partial drain
never permits backing removal or a success state. Final metadata CAS rereads the
revision after leases settle. Restore claims persist Restoring before any backing
creation; concurrent restores cannot create two trees. Final activation follows
backing and original-subdirectory validation, with sessions still archived.

Backing cleanup must retain local/unowned or still-referenced directories. The
existing worktree removal safety outcome remains the one evidence type. A timed
out backend operation must retain its durable claim while physical work is still
uncertain; cancellation is never proof that no side effect happened.

## Differences from Paseo

A failed drain retains Archiving instead of declaring a fully archived workspace
while a member may still run. This safety floor makes partial outcomes explicit.
The workflow has one finite safety deadline, shared by all its awaits; expiration
reports uncertainty and leaves the non-active durable claim for reconciliation.
Vessel uses its existing worktree retention/snapshot safety rules rather than
silently forcing removal of dirty work. A restore never reruns setup or resumes
archived sessions, matching the reference.

## Resume point

Done: exact reviewed identities merged after Quality B; reference and asset audit.
In progress: staged archive/recovery workflows and deterministic fake regressions.
Next: adopt the reviewed registry/store dependency, implement its backend adapter,
register pending typed RPCs, connect native recovery UI, then root/native gates and
real fake-fleet filesystem/process checks. No runtime workspace parity is claimed.


## Recovery wire foundation

`vessel_proto::workspace` reserves one `WORKSPACE_LIFECYCLE = None` threshold
and defines `InspectWorkspaceRecovery`, authoritative recoverable/unavailable
inspection, and an exact-identity restore report with stage/source failures or
uncertainty. Future action/state/reason/result values decode explicitly as Unknown;
clients must offer update/refetch rather than choosing a path-based fallback.
These are DTOs only: no Request implementation, method row, client call or app
command is advertised before the registry-backed node handler exists. Reuse the
registry producer's ArchiveWorkspace/ArchiveWorkspaces/RestoreWorkspace requests
when that reviewed dependency lands. No duplicate registry identity is introduced.

The workflow's internal RecoveryAction records a known operation that actually
settled; the wire action additionally admits a future Unknown value for older
clients. Conversion belongs at the node RPC edge, without widening the internal
executable action set. Pending registration and TypeScript reachability are part
of the later wiring commit, not evidence supplied by DTO roundtrips.

Narrow evidence: staged node workflow 12 passed/0 failed at afdcb766; node
all-target clippy passed at the same source. These do not cover a real registry,
filesystem/process shutdown or registered workspace RPC.
