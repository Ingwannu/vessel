# Project management after the registry dependency

Status: planned source work; no runtime/gate/review/parity claim.

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

Outstanding: selected-root registration, plain directory creation and rollback,
custom names/equivalence/icons, history suggestions, revision-aware vessel.toml,
editor targets and automatic archive. Root/native full gates and UI captures are
owed. No real provider, editor, live node or filesystem outside isolated tests
has been touched by this work.
