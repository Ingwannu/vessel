# 0082: Explicit project metadata and independent workspaces

Status: proposed

## Purpose and intent

A project is a registered directory. A workspace is an independent, durable
container within a project, with its own identity, title, isolation and lifecycle.
Two workspaces can share a cwd without owning each other's agents or terminals.
Registering a project creates neither a workspace nor an agent.

Reference behaviour: Paseo 0.15.4 `packages/app/src/hooks/open-project.ts:80-114`,
`add-project-flow/model.ts:29-85`, `workspace/workspace-archive.ts:82-135`, and
`packages/server/src/server/session/workspace-provisioning/`.

## Existing implementation and constraints

Projects/worktrees already have typed RPCs, a SQLite-backed node registry,
checked roots and digest-scoped command approvals. They are reused. There is
no independent workspace identity, project metadata mutation or guarded config
editor. A worktree id cannot replace a workspace id: deleting the last managed
backing and archiving one logical workspace have different effects.

## Alternatives considered

- Infer workspaces from cwd or session id: conflates independent owners and
  cannot represent an empty workspace.
- Client-local registry or metadata overrides: diverges across clients and
  cannot authorize filesystem mutation.
- Reuse generic file.write for vessel.toml settings: lacks project-specific
  authorization, full syntax checking and explicit stale revision semantics.

## Decision

Add typed project and workspace registry contracts to vessel-proto, with a
single pending feature constant. The protocol number is assigned at merge after
all consumers handle the new outcomes. No call is reachable while unassigned;
older peers are refused before dispatch, and existing replies remain unchanged.

Stage the node implementation independently until the shared method table can
register pending calls. Store schema numbers are assigned by the integrator.
Keep new SQL in a dedicated migration module rather than rewriting released SQL.
Reuse the store writer and read pool; do not create a second database/file registry.

Project metadata is a separate descriptor keyed by ProjectId; existing
ProjectInfo remains backward compatible. Rename changes only custom display
name; reset derives from the recorded directory. Plain directory creation uses
exclusive nonrecursive mkdir and reports registration/rollback separately.
Suggestions use registered harness history readers with explicit limits and do
not import conversations. Icons are bounded image bytes with content revisions;
URL fetching remains a client responsibility, never a node SSRF endpoint.

Configuration editing targets only active registered roots. Preserve raw TOML
text/unknown keys rather than projecting and rewriting a partial schema. A content
hash revision distinguishes absent file from empty file. Compare immediately
before exclusive-temp-file rename; serialize node writes per project. External
writers can still race stat/check/rename: this is optimistic concurrency, not an
OS-level compare-and-swap guarantee. Validate via the existing vessel.toml reader
before committing; saving never approves or executes commands. Preserve existing
file access modes and refuse symlink targets. New config files are private to the
service account; read/write permission failures keep the original source error.

Workspaces persist a project id plus explicit local or managed-worktree backing,
state, title, revision and resource ownership. Creation receipts use a caller
operation id to reconcile disconnects without duplicating a worktree. The registry
must serialize conflicting lifecycle mutations. Archive operates by workspace id,
archives owned agents and terminals, stops owned setup, and removes managed backing
only after its last active reference, with the existing safety checks. Local
folders remain. Partial cleanup failures are typed and visible, never success by
optimistic UI alone. Bulk archive returns every target's result. Recovery is
inspected by the node, re-inspected before mutation, and does not resume agents
or rerun setup implicitly.

## Why this choice

It extends the existing authority and typed boundaries, permits empty/shared-cwd
workspaces, preserves old clients, and avoids widening automation approval.
Existing worktree removal/snapshot semantics remain distinct from archive.

## Advantages, costs and effects

Additional typed calls, storage rows and lifecycle coordination cost more than a
path alias but make ownership reviewable. No new dependency is introduced.
Parsing, filesystem IO and image inspection run off async executor threads.
Windows uses the same checked filesystem contract; an unavailable atomic replace
or permission operation returns a typed refusal rather than claiming success.

## Security boundary

Only authenticated person callers with the required Read/Operate scopes may use
registry mutation/config/icon/history endpoints; agents are refused before any
filesystem or provider-history access. Delegated callers retain scope intersection.
Recheck stored root identity against canonical filesystem identity on every IO.
No shell command or repository hook is executed by project metadata/settings.
Existing digest approval and environment stripping continue to gate automation.
Debug and errors do not include config/icon/history payloads or credentials.

## Verification and staged state

Contract decoding validates private bounded fields and rejects forged values.
Tests cover absent-vs-empty revisions, stale writes, unknown TOML keys, symlink
and root replacement, directory rollback, concurrent lifecycle actions,
shared-backing last-reference rules and recovery reinspection. Fake harnesses
only. No protocol/schema number, runtime call, gate, capture or parity completion
is claimed by this decision document.

## Registry dependency and actor admission

The value-only identity dependency is intentionally independent of these runtime
contracts. Registry/store is a separate dependency package; project UI, config,
editor targets and automatic archive remain later feature work. Create receipts,
resource launch and lifecycle orchestration use this one store and are not
implemented by this dependency's getters/presentation mutations.

Before spawn, the owner claims a preallocated SessionId/TerminalId on an Active
workspace at its expected revision. The writer returns an opaque start lease.
Only after process registration may the owner complete it; a proven no-spawn
failure may mark it failed for an explicit retry. Claims remain permanently tied
to their original workspace. A retry gets a new lease revision, so stale callbacks
cannot settle it. Archive first claims Archiving with CAS, then drains exact
pending leases and resources; it cannot publish Archived while a pending or
uncertain start remains. Already-admitted starts may complete during Archiving.
Restart marks interrupted starts Uncertain rather than rerunning them. This
requires the lifecycle consumer to stop resources only after all starts settle.
A generic metadata CAS is not evidence that resource teardown happened.

The lifetime budget is explicit: at most 256 workspace/backing identities and
4096 resource ownership claims, including archived, failed, interrupted and ended
records. Each workspace atomically reserves 8 KiB descriptor, 256 KiB recovery,
16 KiB intent and 1 KiB bookkeeping capacity before backing effects. Resource
claims reserve 256 bytes each before spawn. The total logical payload allowance
is the named MAX_RETAINED_BYTES formula in the store capacity module. Fixed slot
reservations cover later growth without borrowing a create receipt's independent
budget. Oversized records fail without publishing partial state. New identities
return RegistryFull at capacity; exact existing identities can still reconcile
and settle, with no eviction or ownership/key release. Physical SQLite indexes,
page allocation and WAL remain subject to the existing store management; this
logical payload bound is not a disk quota. A complete public snapshot is bounded
by descriptor and membership limits and fits the existing 16 MiB frame ceiling.

Coalesced watches emit only after a successful writer commit. Consumers re-read
a complete descriptor/membership snapshot on wake; no polling interval, cwd
inference, resource payload or project config text is put into notifications.
The retained external-source gate and explicit release CAS described below are
included. The filesystem provisioner remains staged in this continuation; no
automatic setup or receipt driver is wired by this dependency.

## Retained external source and explicit release

`BlockedSource::ChangeRequest` retains a typed forge repository (kind/host/path)
and positive PullNumber. WorkspaceAutomation is explicitly Allowed or Blocked;
there is no serde default that turns an omitted gate into authorization. The
node-resolved provision intent includes the gate before backing effects. Blocked
worktrees are created with setup skipped even if the original choice requested
Run. Normal command-digest approval remains independent.

A person releases one workspace with both its expected descriptor revision and
exact blocked source. The same writer transaction checks Active, matches both,
advances the revision and persists Allowed. A stale source/revision or lifecycle
claim refuses without mutation. A release neither starts setup, retries services,
approves a command digest nor performs metadata generation. Script and setup
consumers must enforce the retained gate before automatic execution and offer a
separate explicit action after release. Historical create intent stays blocked,
so replay cannot reinterpret the original request as a newly authorized effect.


## Planned lifecycle incarnation fence (not implemented)

The minimal metadata dependency keeps project, path, backing and creation time
immutable under a workspace id. This logical tuple does not prove that a future
restore has the same backing incarnation. A coalesced watch can skip the whole
Archiving/Archived/Restoring interval, and restoring an existing directory can
retain its inode. Neither path equality, physical identity nor a generic title
revision is an adequate actor/route restart authority.

[Decision Log]
- Purpose: a restore/unarchive cycle must invalidate old launch, Ready-resource
  and service-route authority even when it reuses the same directory and ids.
- Existing implementation and constraints: registry revisions reject stale writes
  but also change for presentation; start leases identify one exact run. The
  lifecycle consumer owns actual stop/drain and backing validation. No restore
  adapter is currently wired in this continuation or the minimal dependency.
- Alternatives considered: infer replacement from a changed path or inode, force
  clients to observe every lifecycle transition, or overload descriptor revision.
- Selected approach: a producer-owned typed admission generation in descriptor
  and durable resource/start leases. Atomically advance it at exclusive
  Archived-to-Restoring admission, including existing-directory unarchive, with
  checked overflow refusal before I/O. Preserve the admitted generation through
  partial failure and restart/retry of that same restoration claim. Publish
  Active only for that claim after its consumer proves backing readiness.
- Reason: a complete snapshot can then distinguish a new lifecycle incarnation
  from rename/pin/release without replaying every intermediate event.
- Effects and tradeoffs: lease completion, restart and automatic effect admission
  must compare the canonical generation and lifecycle. Old Ready tokens cannot
  resume after the cycle. Existing starts may settle while Archiving; Archived
  still requires all pending/uncertain starts to be resolved. Actual prior-run
  wait/join proof remains necessary for same-terminal restarts. W9's PhysicalRoot
  and RootLease remain separate filesystem exclusion authorities.

This section reserves a contract boundary, not a published type, SQL field or
API. Its migration, serializers, exact transition checks and adversarial tests
remain to be implemented and reviewed before real restore or D2 cache/effect
wiring. The minimal dependency has no public backing replacement/restore route;
its immutable logical placement must not be described as physical continuity.

registry_resource_run exposes retained admission metadata for supervisor recovery.
Ready means the last run was registered; it attests neither current liveness nor
successful wait/join. The node supervisor must reconcile the actual resource and
run generation before using the opaque predecessor token. The store does not
infer process state from cwd, PID shape or an elapsed time. Failed initial starts
remain separate and retryable only after proof that no actor was spawned.

## Dependency boundary: provisioning stays in the continuation

The registry/store dependency excludes the cfg(test) filesystem provisioner and
its node-only tests. Under A10, an unwired node service stays on its continuation
branch until its real caller removes staging in a wiring commit. Its source is
retained in the separate project-management continuation; it has no consumer-
ready or shipped provisioning API claim. This dependency contains the live
registry read/mutation handlers, durable metadata/reservations/membership and
start-admission seams. Creation receipts, actual backing provisioning and actor
launch remain separate work. No default workspace is synthesized.

## One project-file byte budget (V3 row213 correction)

The proto MAX_CONFIG_BYTES is the canonical lower-layer64KiB budget for both
ConfigText and the authoritative vessel.toml reader/parser. The parser retains
its public project_file::MAX_BYTES name as a re-export, with the workspace owner's
agreement (message24bb3bd8). Existing parser/editor checks, errors and tests derive
from that one definition. No limit or MAX_COMMANDS ownership changes. This avoids
future editor/parser divergence without introducing a new dependency edge.
