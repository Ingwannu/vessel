# 0082: Explicit project metadata and independent workspaces

Status: proposed. On 2026-11-15 the six registry metadata methods
(`workspace.registry`, `.list`, `.get`, `.rename`, `.pin`,
`.release_automation`) merged on the node line and were assigned protocol 25
(`PROJECT_WORKSPACE_REGISTRY = Some(25)`); creation, provisioning, archive,
restore, config and editor work stay unassigned. Text below saying no
protocol number is claimed describes the review checkpoint.

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

The lifetime budget is explicit: MAX_WORKSPACES workspace/backing identities and
MAX_RESOURCES resource ownership claims, including archived, failed, interrupted
and ended records. Each workspace atomically reserves MAX_DESCRIPTOR_BYTES,
MAX_RECOVERY_BYTES, MAX_INTENT_BYTES and WORKSPACE_KEY_BYTES capacity before
backing effects. Resource claims reserve RESERVED_BYTES_PER_RESOURCE before
spawn. MAX_RETAINED_BYTES is computed from these named constants in the store
capacity module. Fixed slot reservations cover later growth without borrowing a
create receipt's independent budget. Oversized records fail without publishing
partial state. New identities return RegistryFull at capacity; exact existing
identities can still reconcile and settle, with no eviction or ownership/key
release. Physical SQLite indexes, page allocation and WAL remain subject to the
existing store management; this logical payload bound is not a disk quota.
Descriptor and membership caps bound a complete public snapshot below the
existing RPC MAX_FRAME ceiling.

Coalesced watches emit only after a successful writer commit. Consumers re-read
a complete descriptor/membership snapshot on wake; no polling interval, cwd
inference, resource payload or project config text is put into notifications.
The retained external-source gate and explicit release CAS described below are
included. The filesystem provisioner remains staged in the project continuation;
this dependency does not start automatic setup or claim provisioning is wired.

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


## Stable terminal runs and restart admission

A terminal may retain its TerminalId over several process runs. The initial
claim deliberately refuses an already Ready resource. Its supervisor retains
the opaque first lease, completes registration, and proves the previous process
has actually joined before requesting claim_registry_restart. A signal, revoked
lifetime token or an ambiguous wait error is not proof that a process stopped.

Restart compares exact workspace/resource/predecessor lease and requires its
stored stage still Ready plus an Active workspace at the expected revision. It
then allocates a new Pending lease in that same transaction. Concurrent restarts,
foreign owners, Archiving/Restoring, pending/uncertain predecessors and late
completions refuse. The resource id/owner and retained capacity slot never change;
restarting an existing row remains possible when fresh resource capacity is full.

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

## Amendment: archived records and the live cap (ADR 0106, 2026-11-16)

Coordinator ruling for runtime workspace registration. It replaces the lifetime
budget above where the two disagree.

[Decision Log]
- Purpose: registration (ADR 0106) creates a workspace each time a project root is
  first opened, and legacy removal archives it. Under a lifetime cap, enough
  open/remove cycles would refuse every later create for good, so a live create
  would fail because of history.
- Choice:
  - `MAX_WORKSPACES` counts live identities only: records that are not Archived,
    plus every reservation.
  - `MAX_RESOURCES` counts only the ownership rows of workspaces that are not
    Archived.
  - Archived records are kept, but they do not count toward either cap.
  - Archived `Local` records beyond `MAX_ARCHIVED_WORKSPACES` (equal to
    `MAX_WORKSPACES`) are pruned oldest first (archive time, then id), in the
    same savepoint as the archive that exceeded the bound. Their ownership rows
    and label assignments cascade. A record with a reservation is never pruned.
  - A live create is never refused because of archived records.
  - `MAX_RETAINED_BYTES` adds one `RESERVED_BYTES_PER_WORKSPACE` term for each
    retained archived slot.
- Why `Local` only: an archived `Managed` record may still describe a worktree
  that the registry must remove after its last reference. Only W21's archive path
  creates archived `Managed` records, and that path brings its own bound with it.
- Snapshot size:
  - live and archived descriptors are at most 512 × 8 KiB = 4 MiB;
  - explicit memberships are at most 4096 rows;
  - derived memberships (ADR 0106) add one small entry for each placed session
    that is not archived;
  - all of this stays below the 16 MiB `MAX_FRAME` until about 90,000 such
    sessions. A session count of that size needs paging, which is a follow-up and
    not decided here.
- Effects:
  - existing identities still reconcile at capacity;
  - archive is not an eviction of live state;
  - pruning removes archived history beyond the bound (a declared difference:
    Paseo keeps archived workspace records without a count limit).
