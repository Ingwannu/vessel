# Project management after the registry dependency

Status: source implementation in progress. Narrow checks below are scoped; no
whole-gate/reviewer/consumer-ready/parity claim.

## Existing assets and decision

[Decision Log]
- Purpose and intent: register the selected directory as a project, create a
  single empty directory, and persist project presentation/config independently
  of workspace creation.
- Existing implementation and constraints: the node workspace area's adding
  module already checks canonical roots and project capacity, but protocol 10
  add_project replaces a selected nested directory with Git's top level. The
  independent registry and opaque actor admission are separate dependencies.
- Alternatives considered: change legacy add_project silently; call Git init for
  empty directories; client-only metadata; a second node registry or database.
- Selected approach: extend existing root checks, project state and store writer;
  add typed project-management calls behind one pending package constant. Keep
  legacy wire behavior stable and introduce selected-root registration explicitly.
- Reason: exact root selection can represent subprojects without changing old
  clients. Project registration and directory creation never create an agent or
  workspace, execute setup, or release external-source automation.
- Advantages, costs and effects: one authority, new explicit capability and more
  typed outcomes. Managed worktree creation must preserve a selected subdirectory
  separately from its Git backing; that mapping remains incomplete.

## Source evidence

Paseo server/session/workspace-provisioning/workspace-provisioning-service.ts:
201-219 stores resolve(cwd) unchanged and probes Git metadata separately. Its
sibling test at656-667 retains a repository root and selected nested directory
as distinct projects. The existing Vessel legacy add helper instead calls
Git::toplevel before insertion; it cannot substitute for the new exact-root call.

Paseo server/project-directory-service.ts:47-111 validates one component and an
existing parent, performs nonrecursive mkdir, registers, then attempts only
nonrecursive rmdir if registration fails. Rollback failure retains its path.
App/components/add-project-flow.tsx:737-780 submits once, retains the draft on
error and opens a workspace draft from the returned project. No Git init runs.

## Intended structure and data flow

The proto project-management module owns typed requests/outcomes and its pending
feature constant. Node workspace/project management extends existing canonical
root policy and registration state; filesystem steps run on the blocking pool.
Store presentation writes use the existing writer and a genuine append migration,
without placeholder tables in the registry dependency. Native view/link/app use
these requests only once the node implementation exists; in-memory fakes model
unavailable/loading/stale/partial outcomes meanwhile.

Outstanding: verification of selected-root registration and plain directory creation/rollback,
custom names/equivalence/icons, history suggestions, revision-aware vessel.toml,
editor targets and automatic archive. Root/native full gates and UI captures are
owed. No real provider, editor, live node or filesystem outside isolated tests
has been touched by this work.

## First source checkpoint (unverified)

Selected-root registration now has a typed pending method with its own
PROJECT_MANAGEMENT feature gate, separate from the approved registry dependency.
The old add call keeps Git-top behavior. Both registration paths use serialized
insertion, recheck exact-root deduplication under that admission lock, save before
publishing, and keep critical insertion on the existing task tracker after a
caller disconnects. Tests compare nested-root behavior with the legacy call,
concurrent deduplication, and a closed store that must leave no visible ghost.
Current source has passed format/size/boundary checks only; compilation, tests,
TS regeneration and full C1 validation remain owed. No native caller was wired.

## Empty-directory source checkpoint (unverified)

The pending `project.create_directory` call uses the existing validated
`DirectoryName` and selected-root registration. It requires an existing absolute
parent, checks root/Git-internal policy before nonrecursive mkdir, and does not
initialize Git. The node task tracker retains creation through persistence and
compensation when the RPC waiter disappears. Registration failure returns a typed
`RegistrationFailed` outcome with `Removed` or `Retained { path, error }`;
compensation uses only remove_dir and preserves concurrently added contents. A
node process crash can still leave an unregistered directory; an existing target
is refused on a fresh request rather than adopted or deleted automatically.

Tests cover a new non-Git project, absent parents, pre-existing targets, closed
store rollback, and nonempty-directory preservation. No test has run at this
checkpoint. The existing framed test handler fakes now explicitly refuse both
C1 calls and the separate registry methods. New project-family capability test
entries await the integrator's announced method-table/family-test base.

## Registered project config source checkpoint (unverified)

Paseo's `session/project-config/project-config-session.ts:40-109,178-199`
authorizes active registry roots and separates read/write from setup/approval.
Its mtime/size revision and JSON projection are replaced by Vessel's existing
content-hash WriteExpect and raw TOML. The same 64 KiB bound and ProjectFile
schema used by execution apply; invalid TOML can be read for repair, while
binary/NUL/oversized sources refuse. Missing differs from a present empty file.

`vessel_workspace::project_config` reuses the bounded regular-file read and
files::write's checked path, exclusive adjacent temporary, sync and atomic
replacement with existing permissions retained. It preserves comments/unknown
keys verbatim and never formats parser source into a user error. External
processes can still race the optimistic content check and rename, as in the
existing file editor and Paseo's stat/rename implementation. Node config writes
are serialized with project registration/removal and task-tracked through IO,
so two C1 saves with the same revision cannot both win. The root remains a
registered ProjectId lookup; no arbitrary client path authorizes config access.

No approval digest is added, no setup/service is started, and automation is not
released. Tests cover missing/empty revisions, equal-length external changes,
repair, invalid saves, raw unknown fields, bounded/binary reads, external links,
retained file mode/uid and serialized saves/removed-project refusal. These
source changes are not compiled/tested yet. New method table rows stay pending
PROJECT_MANAGEMENT; no protocol/schema number assigned.

## Narrow evidence checkpoint

Selected-root registration and empty-directory source at cf171f035 passed strict
all-target Clippy for proto/node/client/RPC/web on the remote narrow run. This is
compile/lint evidence, not execution of the new directory tests; their node test
run remains queued. Later config edits are not covered by that earlier check.

At f2558c4ce, local proto TypeScript checks passed 3/0 with generation enabled.
The generated web/src/wire/proto.ts is committed with the canonical pending
registration/directory/config requests, responses and aliases. The config IO
functional tests and strict proto/workspace check remain queued; no live config
file, provider, editor or node was accessed for these checks.


The preexisting local config check finished with a strict lint failure after
TS generation 3/0 and config substrate tests 4/0 passed: the UTF-8 refusal
discarded its decoding cause. ConfigEditError now retains Utf8Error as a source,
while public text remains redacted; the node mapping and existing binary-input
regression match the payload variant. This changes no permission or validation
policy. Corrected strict workspace/proto/node checks move to the remote gate.

The corrected UTF-8/source check reached the node and found two identical error
mapping arms. They are combined without changing classification or error text.
The failed exact0f8097f3 log remains retained; a new immutable strict check is
required. The independent directory/config node test run at28950f795 passed77/0
with its web/fake/node prerequisites, still not a complete C1 gate or review.

The continuation now merges the complete announced gate44 node head ede20014.
Its existing process, service, plugin-value and protocol accessor changes stay
intact; the architecture merge appends the project sections after the head's new
sections. Current strict/config checks are owed on this union, and generated
wire freshness will be checked remotely before a whole C1 gate. No real actor,
editor, service or filesystem outside isolated tests was started.

A manual source audit also found invalid-save parsing discarded its typed cause
through is_err. ConfigEditError now keeps ProjectFileError as its source, while
a manual Debug delegates only to the already-redacted public Display message.
The invalid-save regression checks both retained typed cause and absence of the
rejected TOML in Display/Debug, while preserving the existing no-write assertion.
This requires a new current-head check; the previous source's 4/0 result does
not verify the new error surface. No config payload is added to wire errors.


## Coordinator-approved A1 provisioning ownership transfer

The actual empty-workspace filesystem provisioning and receipt/launch adapter
move to A1. This package keeps canonical identity/registry/start leases, project
and config admission, the priority native projection, nested project execution
reroot, PR mode, unregister and legacy destructive-route fencing. The shared
owner table section7 records this ruling. No unreviewed helper becomes shipping
merely because ownership changes.

A1 owns these node adapter files from this continuation as source reference:
- workspace/registry.rs (staged adapter result/error surface);
- workspace/registry/provision.rs (reservation and filesystem adapter);
- workspace/tests/registry.rs (its staged provisioning tests).
A1 may rename these under a provisioning module and add its own receipt driver.
Only the corresponding module/test registration lines are shared edits.

This package retains workspace/worktrees/create.rs, including the currently
staged fixed-id primitive create_worktree_with_id, and project/config modules.
It will provide a reviewed primitive/admission boundary rather than allowing
two writers in that file. Store registry modules and proto registry values
remain canonical here; A1 consumes them and does not duplicate persistence.

The reference is not production-ready: project/unregister admission, nested
execution reroot and PR mode are incomplete. In particular, its Allowed+Run path
still delegates legacy worktree setup before an independent workspace descriptor
is finalized. A1 must not ship that path as automatic workspace bootstrap.
Backing creation and workspace-aware setup/terminal execution must be ordered
so the latter uses the exact WorkspaceId, retained gate, command approval and
start-lease producer with the scripts/runtime owner. Blocked sources keep Skip;
no release or retry implicitly approves/runs anything. Existing InProgress and
Uncertain outcomes remain distinct, with no repeat filesystem effect on replay.

This is a scope/ownership update only. The three files remain staged and have
no current checked-adapter adoption claim. The frozen minimal registry delivery
excludes them and continues its independent final gate and source review.


## Admission producer extraction and current validation limits

Project admission and the reserved backing primitive are being reviewed as a
separate small dependency on the announced node base. The isolated producer
preserves legacy NotGit/NotReady outcomes and excludes duplicate in-flight fixed
ids even when callers share one admission. The older draft in this continuation
is source reference only and must be reconciled to the reviewed producer before
use; its older check is not evidence for the isolated dependency.

The original continuation strict check stopped in a canonical-budget doc comment
before compiling the project changes. The reviewed comment-only correction is
now present. The primitive alias table also stops classifying WorkspaceOperationId
until an actual wire declaration reaches it, matching the node integration fix;
this changes no generated declaration. Fresh strict and TypeScript checks remain
owed. No provisioning, config or membership adapter becomes checked by these
source corrections.


### Config writer cancellation before metadata admission

The writer acquires the landed bounded project admission before creating a
tracked task, then retains that same Arc through the file write acknowledgement.
A cancelled admission waiter therefore cannot leave a detached writer which
changes the file after the caller has gone. The regression polls an actual write
behind a retained project hold, checks that no tracked task was added, drops the
waiter, releases the hold and verifies the file remains missing; a subsequent
admitted write succeeds. This test is authored but not yet executed on the merged
source. It grants no physical-root, approval, generation or RPC activation proof.


New-directory tasks take the same finite, nonwaiting project-operation capacity
before detaching or mkdir. The retained outer task and its registration child
each count toward the existing budget; under saturation the child can refuse
and trigger nonrecursive cleanup, rather than wait on a second unbounded queue.
The authored saturation regression holds all admission slots, expects refusal
before any tracked task or directory exists, then releases and creates normally.
Its execution is still owed. Legacy init/clone pre-registration effects and
external filesystem races remain outside this cancellation guarantee.

## Read task admission lifetime (source only)

A config read now mirrors the existing tracked writer lifetime: obtain bounded
project admission before detaching, then move its own Arc into the tracked task
until the lower blocking read actually joins. Dropping an RPC waiter cannot free
project metadata admission while that task still reads. The prior post-read
membership check is redundant while this canonical admission is retained.
This does not freeze external edits or prove physical root continuity; root
exclusion and the independent public ConfigEditError source-chain privacy follow-up
remain incomplete. Existing config identity/CAS tests require changed-head VM
execution. No new runtime, wire activation or cancellation proof is claimed here.
