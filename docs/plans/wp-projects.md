# Native projects and workspace parity

## Purpose and references

The native client must preserve host/project/workspace identity across navigation,
async mutations, setup and recovery. Reference: Paseo 0.15.4, commit
`1c00b365c9e0b74c1580eba33d905d693125ae21`; implementation is independently written Rust.

Read source evidence (paths relative to Paseo `packages/app/src`):

- `add-project-flow/model.ts:29-85,135-146`: host-bound page stack, preferred
  or sole host, back navigation and per-location drafts.
- `hooks/open-project.ts:80-114`: registration calls addProject only; no
  workspace or agent is created by registration.
- `screens/projects-screen.tsx:24-80,109-116`: host-specific rows, including
  projects without workspaces, exact host/project navigation.
- `workspace/workspace-archive.ts:82-135`: optimistic hide with rollback and
  per-target bulk failures. It does not equate workspace with a Git worktree.
- `panels/setup-panel.tsx:168-204`: running/last log auto-expansion with manual
  collapse precedence, carriage-return normalization and selectable output.
- `screens/new-workspace-screen.tsx:153-165`: first-agent naming context is
  distinct from starting that agent.
- Survey `p1-projects-workspaces.md`, sections 1-17 and 32-33 supplies the
  remaining source paths. Survey is source evidence, not runtime verification.

## Existing assets and decision

[Decision Log]
- Purpose and intent: extend native projects without a second registry or a
  second filesystem authority.
- Existing implementation and constraints: `view/workspace/projects.rs` owns
  cached project lists; `app/areas/workspace/projects.rs` has registration and
  host settings rows; `place.rs` and `link/workspace/start.rs` start sessions
  in projects/worktrees. `setup_log.rs` already decodes paged setup output.
- Alternatives considered: replacing these areas, generic file writes for
  settings, deriving workspaces from sessions or paths, adding a new library.
- Choice: modify existing screens, add pure typed flow state beside projects,
  and add typed link adapters only for existing RPCs. No dependency additions.
- Why: the node remains the only project/placement authority. File writes
  cannot reproduce the missing config authorization/revision contract, and
  sessions/worktrees cannot substitute for independent workspace identity.
- Advantages, costs and effects: small registration seams, explicit stale
  response rejection and reusable views; full parity waits for node contracts.

## Ownership and data flow

`view/workspace/projects/` owns pure navigation and operation state;
`link/workspace/project_flow.rs` adapts existing project calls and deadlines;
`app/areas/workspace/project_form.rs` owns registration form interactions.
Existing host project cache remains authoritative for display. The app submits
one host-bound operation, link returns its ticket, and view accepts only its
matching result. Drawing uses kit controls and prepared EN/KO Fluent words.
No new crate dependency edge, local filesystem write, shell command, or node
configuration parser is introduced.

Scripts, service ports and command approval rendering belong to their existing
owners. `vessel.toml` parsing, approved digest and automation security remain
node-owned. Shared shell and link files receive registration only.

## Feature contract map and explicit differences

| Reference feature | Existing node contract | Implementation boundary |
| --- | --- | --- |
| Register existing directory; clone | project.add/clone/get/list | Extend host-bound form and preserve progress |
| New directory without Git | None; project.init creates a Git repository | Hidden pending real directory-create RPC; existing Git-init action retains explicit name |
| Suggested projects | None | Hidden; do not scan user provider homes |
| Rename and icon | None | Hidden; no client-only override pretending to persist |
| Remove project | project.remove | Confirm concrete host and project; node refusal remains visible |
| Project settings | project.commands/approve read approved commands only | Existing read-only vessel.toml command view; revision editing blocked |
| New workspace, local/branch-off/checkout-branch | session.create place + worktree.create | Existing new-chat flow extended; do not claim independent empty workspace support |
| Checkout PR | No worktree PR mode | Hidden pending node contract |
| Setup logs | worktree.output/setup | Reuse paged log and explicit approval; no trust-release equivalence |
| Workspace trust release | None | Hidden; project digest approval is a different operation |
| Archive/bulk archive/unarchive/recovery | No WorkspaceId or independent workspace API | Blocked; worktree.remove is never renamed Archive |
| Auto-archive after merge | No setting/call yet | Node track owns design; hidden until authoritative capability exists |
| Automatic title and branch names | Pending node feature | Display authoritative names only; no app-generated branch rename |
| Host projects home | project.list and existing settings host selector | Retain host-specific empty/loading/error/populated states |

Node contract absences were independently checked on the app base and current
node source. Missing contracts were sent to the coordinator and node owner;
no fabricated call or protocol number is introduced.

## Validation and security

Pure state tests cover duplicate submissions, host switches, stale replies,
removal cancellation and transport failure. Link tests use in-memory fakes;
never real agents, provider credentials or live nodes. Narrow checks per commit
and one complete native gate/capture matrix at package end. No push.
Runtime service ownership/permissions are unaffected: this package edits local
source only and does not deploy to a running service.

## Resume point

- Completed: reference and existing asset map; node contract gaps confirmed;
  ownership coordinated with scripts and node workspace tracks.
- In progress: typed flow skeleton and supported project interactions.
- Next: narrow checks, connect supported interactions, merge announced app base,
  captures and native gate when the package is reviewable.
- Blocked: full workspace lifecycle, project revision editor and metadata
  mutations await node contracts. No gate, capture or parity claim yet.

## Final ownership correction

The canonical plan assigns this package project UI plus registry/status/rename/pin
and ADR 0082 node identity. Create-flow UI/receipts and terminal profiles belong
to the create package; archive/restore UI belongs to lifecycle. They consume the
shared registry rather than re-declaring ids. Setup progress UI belongs to scripts.
Project settings and editor targets remain this package. The registry's first
staged contract is `vessel_proto::WorkspaceId` and `vessel_proto::registry`,
including an atomic `RegistrySnapshot { node, revision, workspaces, memberships }`.
Session/terminal ownership is explicit via WorkspaceResource; no old SessionInfo
field or cwd-based ownership is introduced. Protocol remains unassigned.

Narrow validation: local proto test cancelled while still queued (no compiler
ran) to move the dependency-critical contract check to the build machine.
The contract is staged until its narrow test and Clippy complete; no native or
root whole-gate/capture/consumer-ready claim yet.

## Registry persistence implementation

The SQLite append follows the base's newest migration through one local constant.
The public Store handle uses its existing writer savepoints/read pool. Descriptor
CAS, resource admission and global revision are committed together. Admission also
advances the descriptor revision, so an archive claim made from an older member
snapshot fails. Archiving blocks further admission. Creation uses fixed backing
reservations and a descriptor+reservation completion transaction; these are not
the create package's user operation/subscribe receipts.

Regression cases: shared cwd stays distinct, stale CAS rollback, unique ownership,
archive/member race, changed provision intent, corrupt rows and counter exhaustion.
Existing inbox rollback fixture gains only this migration's DROP statements before
its original rollback; no production migration runner or head assertion is changed.

Resume: staged contract 702277bd passed 5 proto registry tests; its narrow Clippy
is queued. Store persistence and Archiving/root adapter are implemented locally,
awaiting narrow store checks. No node RPC is registered, no app wire integration,
no full gate or captures yet. Shared registry types are not called reviewed.

The staged provisioning service reuses existing worktree creation with a fixed,
durable backing id. Local provisioning creates only a registry record. Replays
return the same descriptor; an id with a different intent is refused. Approval
is returned before a worktree is made. Startup marks interrupted Creating
reservations Uncertain; no automatic filesystem replay occurs. Descriptor
publication and reservation Ready are one transaction. The node module remains
test-only until pending RPC wiring; existing worktree.create still chooses a
random id before delegating to the same implementation.

Contract narrow Clippy on 702277bd returned three documentation-markdown lints
(no type/runtime error). They are corrected in the next commit and rechecked;
the failed log remains evidence. Tests on that commit were 5 passed/0 failed.

The first store narrow run (53e835a1) stopped while prebuilding the node because
SQLite does not decode u64 directly. The registry reader now decodes i64 and
checks conversion to the typed nonnegative revision; no default or truncation.
The original failed log is retained.

Store narrow run c620d7c2: 87 passed/0 failed. Local Clippy found unused SQL row
counts, doc markup and discarded signed-conversion causes. The next change binds
row counts and retains TryFromIntError as a typed source. Registry invalidation
uses a coalesced watch emitted only after a successful writer commit; consumers
re-read authoritative lifecycle/ownership, with no polling or cwd guess. New
watcher/owner regressions are added and await checks. This is not a runtime port
allocator or lifecycle drain implementation.

## Dependency split and pending registry calls

The identity-only dependency has its own value-types plan. Registry/store remains
separate and unreviewed until its whole root gate and source review pass. Quality B
pending rows now cover workspace.registry/list/get/rename/pin under the single
unassigned feature constant; no protocol number is selected here. A finite
long-poll uses the existing waiter shares and the store's committed watch. Empty
and archived records remain authoritative, with stable NodeId in the snapshot.
The root Request/UnexpectedReply definitions moved to rpc/request.rs without
changing their public re-export, to keep the single method table below its budget.

Actor admission uses an opaque pre-spawn lease, exact ownership and an archive
fence. Restart marks unfinished admissions uncertain; never replay automatically.
Named lifetime count and worst-case byte reservations are in registry/capacity.rs.
Tests cover lease/archive interleaving, stale completion after retry, capacity
reconciliation without eviction, and oversized record rollback. These tests and
new pending handlers still require current narrow and full-package validation.
Project metadata/config/settings/editor and trust-release runtime work remains
incomplete. No native integration/capture/parity completion is implied.

External-source automation now has a canonical typed source, persisted in the
provision intent and descriptor. The pending workspace.release_automation method
compares exact source+revision and requires Active/person authorization. It does
no execution. Blocked provisioning forces setup Skip; tests assert skipped state
and absence of the isolated marker. Wrong source, stale revision and an archive
claim all refuse release without changing state. These changes await current
checks. The first registered-call narrow build found three exhaustive mapping
omissions (agent withholding, old replies, forwarding); all new registry methods
now explicitly choose those mappings, with no wildcard or head-case rewrite.

The registry dependency migration contains only its live descriptor/reservation/
resource/revision tables. Project presentation storage is deferred to the later
project package's append; no unused project-metadata table is reserved here.

### Project admission retained through provisioning

[Decision Log]
- Purpose: W3 must not finalize a workspace for a project concurrently removed or rewritten through node project/config APIs.
- Existing constraint: resolve(Project) rechecks the ready root only for one call; it does not retain project registration. The existing project_registrations mutex already serializes registration/removal/config writes.
- Alternatives: a second per-project lock table, repeated existence checks after effects, or relying on cwd.
- Choice: reuse that mutex as an owned, opaque project admission retained by Arc through tracked backing I/O and descriptor finalization. The token binds to the exact Workspaces allocation and ProjectId; it cannot authorize another node/project.
- Reason: one serialization domain avoids duplicate lock registries and check/remove races. Existing fixed-id backing creation accepts this admission rather than resolving a caller's current selection.
- Impact: admitted project operations are serialized globally, matching the existing writer's conservative scope. The token does not prove a physical FD/root, config approval/digest, WorkspaceId lifecycle, or resource start permission. W9 root exclusion and registry start/generation contracts remain distinct. W3 must retain its admission until descriptor finalization and use workspace-aware W8/D2 bootstrap later.

This continuation is separate from the frozen1057336b registry delivery. Actual W3
filesystem driver files remain W3-owned. Any producer below is source-only until
its exact remote checks and review; old resolve alone is never that admission.

Project admission reserves one of MAX_PROJECT_ADMISSIONS64 permits before waiting
on the existing mutex. Both owner and waiters retain capacity through completion;
fresh overflow returns typed Busy without mutation. This preserves serialized
concurrent requests while bounding detached tracked creation/config writes. The
actual unregister regression polls removal while two admission holders remain and
checks the project still exists; dropping only one holder cannot release it.

The retained-admission fixed-id provisioning primitive explicitly rejects
SetupChoice::Run before any backing effect. Its caller must choose Skip, finalize
the independent descriptor, then ask the workspace-aware bootstrap owner later.
The existing legacy worktree.create Run behavior remains a distinct legacy path;
its helper is not a workspace bootstrap adapter. No W3-owned files are edited.
