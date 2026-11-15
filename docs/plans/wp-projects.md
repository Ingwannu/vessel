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

The continuation's staged provisioning service reuses worktree creation with a fixed,
durable backing id. Local provisioning creates only a registry record. Replays
return the same descriptor; an id with a different intent is refused. Approval
is returned before a worktree is made. Startup marks interrupted Creating
reservations Uncertain; no automatic filesystem replay occurs. Descriptor
publication and reservation Ready are one transaction. The node module remains on that continuation until its real caller wires it.
This dependency retains the unchanged legacy worktree.create implementation,
which chooses its own random id.

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


Earlier registry dependency checkpoint: local Store suite 96 passed/0 failed,
node registry provisioning 5/0, TypeScript generation 4/0. Strict all-target
Clippy then found explicit pin defaults, a borrowed error adapter, one oversized
existing worktree function after fixed-id admission, and an unused test result.
These are corrected without changing head test expectations. Stable terminal
restart now has a distinct exact-predecessor lease API and two new adversarial
cases plus at-capacity restart coverage; current checks and A12 remain owed.
Existing Ready admission is not process-exit proof. Its consumer must join the
old run before restarting; no signal/lifetime heuristic is substituted here.

## Dependency whole-gate findings and correction

The frozen registry dependency at 1958fd7b passed strict root/TS/forge Clippy,
wire generation and build checks. Its whole Rust run had 2304 passed, 8 failed,
12 ignored: the explicit forwardable-method expectation lacked the six new
registry methods; seven legacy migration fixtures left the registry append in
place before rewinding user_version. The registry admission/restart/watch tests
passed in that run. Web unit tests passed 449 and forge tests 4; browser e2e
passed 65 with one unresolved command-list Escape/filter failure at
conversation.spec.ts:258. This is a failed gate, not acceptance or parity proof.

The correction adds the six methods to the explicit expected set and removes
only the new registry tables in rollback fixtures. The title-origin fixture now
targets its preexisting schema checkpoint instead of newest-minus-one, so a
later append cannot change which migration it tests. Production migration SQL,
authorization, method scopes and assertions remain intact. Targeted reruns and a
new whole gate remain owed; the unrelated browser failure must also be checked.

## Dependency boundary: provisioning stays in the continuation

The registry/store dependency excludes the cfg(test) filesystem provisioner and
its node-only tests. Under A10, an unwired node service stays on its continuation
branch until its real caller removes staging in a wiring commit. Its source is
retained in the separate project-management continuation; it has no consumer-
ready or shipped provisioning API claim. This dependency contains the live
registry read/mutation handlers, durable metadata/reservations/membership and
start-admission seams. Creation receipts, actual backing provisioning and actor
launch remain separate work. No default workspace is synthesized.

The first fixture-correction narrow failed at compile time because devices.rs
uses its own inline fixture builders rather than support/fixtures.rs. The
rollback SQL now has one small test-only module used by both fixture families.
The failed run is retained; a corrected narrow and whole gate remain owed.


## Corrected dependency narrow evidence

At exact source 53c13163, proto/store all-target tests passed 465/0/1, including
the corrected legacy migration fixtures, admission/restart and registry watch
cases. Store benchmarks also completed. The subsequent Clippy command did not
execute because local Rust compilation was disabled centrally; this is not a
compiler failure. Strict proto/store/node Clippy is now submitted to the remote
build gate. No new Rust work will run on the development host.

The unrelated command-list failure was classified by its web owner as a base
caret timing defect: a deterministic regression failed 3/3 before its fix and
passed with the fix. Its combined node gate is still running. This dependency
will merge the announced complete node head when available, as instructed,
before retrying the whole gate. No web source was edited here. Source review
and dependency adoption remain pending.


## Announced caret-fix node merge

The complete announced gate44 node head ede20014 is merged under A8, rather than
taking its web fix alone. Its root gate passed Rust2377/0/12, web464, e2e68 and
forge4; that is base evidence, not this registry union's gate. The node head owns
rpc/contract.rs and rpc/table.rs; the registry's redundant request.rs extraction
is removed, and its six pending rows are appended at the method table's end.
The head's existing rows, tests, protocol and schema entries remain intact. The
registry append still follows the unchanged newest head migration. Architecture
and protocol documents preserve the head's new process/service-port facts and
append the independent registry section. Changed-head TS/narrow/full checks are
required next, all on the remote build machine.

The explicit forwardable expectation appends only this package's six names in
their new method-table order. The head's existing entries and order are unchanged;
this prevents the table move itself from causing an expected-order regression.


## Merged-head strict and generated-wire evidence

Exact c524681c6 on the announced node head passed strict whole-workspace Clippy
and proto/web TS-feature all-target Clippy remotely. The TS generator's four
selected tests passed; the named TS step correctly remained red because its
generated proto.ts differed in three method-union lines after table relocation.
The build VM's exact generated artifact and wire patch were recovered through
the supported gate artifact path, source-commit provenance and all SHA256SUMS
verified, then applied verbatim and compared byte-for-byte. No TypeScript was
hand-edited. The new final candidate also retains the expected-method-order
correction; its complete A12 and V3 registry review remain owed.

The first merged-head job was explicitly promoted under a coordinator exception
because the dependency blocks six core adapters: the safe queued-only endpoint
verified it held no build slot, terminal cancellation was confirmed, and the
same immutable c524 commit was submitted once with priority. Its original logs
remain retained. No running build or unrelated job was canceled.
