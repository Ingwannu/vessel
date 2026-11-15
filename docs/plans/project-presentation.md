# Project presentation continuation

Status: source design only, not implemented, compiled, reviewed or consumer-ready.

## Existing assets and source evidence

Paseo app/src/components/project-edit-sheet.tsx:230-246 acquires the icon source,
then saves the name followed by the icon with step-specific failures. Server
session.ts:3631-3698 trims a blank override to reset, updates only the custom
name, and republishes project/workspace presentation. Workspace-registry.ts:
26-35,642-660 keeps derived displayName separate from the user's override.
Neither rename nor icon editing changes a directory or stable project identity.

Vessel already has typed ProjectMetadata, ReadProjectMetadata and RenameProject
values; ProjectName validates a portable display name. The existing store
workspace writer uses ON CONFLICT UPDATE, and projects retain ProjectInfo JSON
for legacy readers. The node's clone progress updates that snapshot separately.
No second registry or external service is needed for names.

## Decision

[Decision Log]
- Purpose: durable custom-name/reset behavior which cannot be reverted by a late
  clone-progress save, while retaining legacy ProjectInfo replies.
- Existing constraints: ProjectInfo.name currently conflates a derived label and
  an override; clone progress saves whole captured snapshots. A node-only name
  edit followed by put_project can overwrite newer progress, and a late clone
  save can overwrite the new name.
- Alternatives: infer reset from the current label forever; mutate the directory;
  serialize a second JSON registry; update only the node's current memory.
- Selected approach: extend the existing SQLite writer with one presentation
  append in the C1 package. Retain derived/custom names separately. A rename
  updates presentation and the legacy JSON name projection in one savepoint,
  reading the latest stored project rather than accepting a captured snapshot.
  Ordinary project-progress writes reapply authoritative presentation before
  persistence, and the node merges only name changes into its live snapshot.
- Reason: one durable writer owns both the custom-name authority and its legacy
  display projection. Restart cannot resurrect an older clone's name, while a
  rename cannot roll clone state backward.
- Costs and effects: an additive migration, typed writer result, and focused
  race/restart tests. The package constant remains pending and the integrator
  re-points the append at merge. Icon storage/schema is added only with its real
  bounded validation/storage implementation, not as a placeholder table here.

## Intended data flow and checks

ReadProjectMetadata resolves a registered ProjectId. A legacy project imports its
current non-derived label as an override without silently discarding it. Rename
accepts Option<ProjectName>; None resets to the recorded derived name. Node
mutation tracking retains the selected Name+NodeId and never renames files.

The existing project mutation lock serializes unregister/config/presentation
operations. The store writer preserves concurrently recorded clone state, and
late generic project writes cannot override a custom name. Node publication
merges presentation into the latest live project record rather than replacing
its progress with the reply's earlier snapshot.

Required checks: custom rename, reset, old-project import, id/path preservation,
concurrent/late clone-progress writes, restart, failed persistence with no visible
name mutation, removed-project refusal and step-specific UI error retention.
The separate physical-root writer lease belongs to the checkpoint producer;
name edits do not touch a filesystem root or acquire filesystem authority.

Outstanding: implementation, icon source validation and cache/persistence,
project equivalence projection, native editor/settings integration, narrow/full
gates and reviewer acceptance. This design is not a shipped feature claim.
