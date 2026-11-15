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
| Auto-archive after merge | No setting/call yet | This package owns the missing node design; hidden until authoritative capability exists |
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

## Form module boundary

The existing registration form methods/rendering move to project_form.rs; the
host list and navigation remain in projects.rs. No request, validation, callback
or test expectation changes in this extraction. Moved pixel values reference
existing exact-valued spacing tokens (the existing ten-point gap is six+four),
so the new file introduces no style-ratchet entry. Command approval and script
files remain outside this change.

## Correlated existing project actions

The supported legacy project mutations now carry a validated host alias and a
checked monotonically unique form ticket. Cancelling/reopening does not claim a
remote operation was rolled back; stale completions may reconcile the host cache
but cannot close or fail the newer draft. The link bounds the mutation wait to
the reference SDK's default and exposes timeout as an unknown outcome with a
refresh instruction, never as proof of rollback. Queue refusal settles through
the same ticket. Existing project registration still creates no session.

Removal requires an explicit confirmation showing exact host/name/path. The
current legacy node's refusal while occupied is explained honestly. Deliberate
initial presentation difference: confirmation is inline in the settings content,
rather than a modal; the node's removal semantics are unchanged. This is not the
new workspace archive operation. Four pure transition regressions cover duplicate
submission, cancellation/reopen, host change and counter exhaustion. Real link and
rendered interaction checks remain pending.

Three link regressions now exercise real framed NodeClient RPC over duplex memory:
exact remove target/ticket, queue refusal correlation, and read-only node refusal.
No real host, agent or filesystem project mutation is involved. Checks are still
pending; source is not called passed. The package base merge follows the central
announcement rather than independently merging root node code into the app.

The first native narrow Clippy (d84449ca) stopped on one doc-markdown lint in the
new deadline comment; that comment is corrected. The package then merged the
announced app package base 8d5513e4 without conflicts. The projects size exception
remains removed; shared registration files follow the new base split. No capture
or runtime completion is inferred from this source merge.

Host-alias mutation correlation now rejects stale payload hydration. After a
cancelled form or successful book rename/forget, delayed project results cannot
write the cache or settle a newer form. If that host is still known, the app asks
for its current authoritative list instead. Renaming/forgetting closes the old
host's project mutation drafts without undoing an already submitted node effect.
A headless GPUI regression combines alias reuse, stale success and a newer pending
form with actual text input/caret retention. Current native checks remain owed.


## Remaining project feature work after registry dependency

The canonical ownership split places create-flow UI/receipts with create,
archive/recovery orchestration with lifecycle, setup progress with scripts, and
sidebar rendering with its navigation package. This package continues project
registration/plain directory creation, metadata/icons, suggestions, settings,
editor targets and automatic archive. These are incomplete; existing screens and
pending value declarations are not parity proof.

Fresh source decisions for the next stage:

- Project equivalence is presentation, never workspace identity or authority.
  Paseo server/project-key.ts:6-38 normalizes forge remotes, GitHub path casing,
  explicit ports and subdirectories; no-remote roots stay host-specific. The app
  projects/workspace-structure.ts:161-179 falls back to distinct placements when
  the same equivalent key appears twice on one host. Reuse existing forge remote
  parsing and explicit NodeId/ProjectId placements rather than a cwd join key.
- Editor targets use a local desktop bridge, not the legacy daemon editor RPC.
  app/workspace/open-in-editor/planner.ts:59-112 requires local execution and a
  bridge; desktop/editor-targets/registry.ts:31-56 defines target order and
  registry.ts:66-130 rechecks target availability and absolute existing paths.
  Require authenticated managed-local target provenance from the desktop node
  controller before resolving an OS path or launching. Remote projects keep
  eligible forge/clipboard actions. No editor was launched in this research.
- Settings preserve raw vessel.toml and unknown fields. The existing project
  reader has a fixed byte cap and typed schema validation; generic file.write
  cannot stand in for project registration/revision authority. Saving never
  releases an external-source gate or approves a command digest.

Package base 2 supplies Machine.node from the authenticated welcome. Project
mutation keys now capture that NodeId as well as the name and local serial;
link refuses a mismatching NodeClient before sending any mutation. Forms and
removal confirmations retain their original node. Rebinding an alias does not
retarget an existing path draft, and a response from the old node cannot hydrate
a different current node. A framed in-memory test verifies no request is sent
to the replacement peer. The earlier import cleanup accidentally removed part
of a trait name; it is corrected explicitly rather than by substring editing.

Native registration checkpoint: strict all-target Clippy for view/link/app and
font validation passed on the current source. Narrow project tests passed 27/0:
2 headless app input/correlation, 4 framed link tests and 21 view cases selected
by the filter (including existing projection/Git cases). These are scoped checks,
not screen captures, real fleet behavior, full C1 or V4 acceptance.

## Manual readiness producer (staged, unverified)

`Shell::setup_readiness(&Name, NodeId) -> ProjectSetupReadiness` reports
Ready/Pending/Failed for one authenticated online host. The onboarding owner
retains and validates its sole Target and Visit; this query does not invent or
acknowledge a suggested-project checked set. A missing/loading project list or
running clone is Pending; read/clone failure, missing directories and a rebound
or disconnected host are Failed. Form validation errors remain scoped to the
captured Name+NodeId.

[Decision Log]
- Purpose: onboarding cannot advance merely because a form was closed.
- Constraint: cancelling the visible form does not cancel an already sent RPC.
- Alternatives: a page busy boolean or treating detached responses as successes.
- Choice: separate bounded manual activity tracking from current form correlation.
- Reason: a late success/failure must settle activity while remaining unable to
  hydrate another node or close another form. Up to 16 retained reconciliations
  are allowed; at capacity new dispatch refuses, existing results still settle.
- Tradeoff: failed activity remains until explicit retry for that host; readiness
  is conservative and never proves checked suggestions were all added.

Pure tests cover cancellation, stale replies, host identity and capacity; a
headless shell test covers clone observation and host rebinding. New source has
not compiled or run yet; earlier 27/0 and strict Clippy evidence applies only to
the preceding checkpoint.
