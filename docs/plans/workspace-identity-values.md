# Workspace identity values

This dependency contains identifiers only. It does not register calls, change
storage, infer ownership, or complete the project/workspace feature package.

[Decision Log]
- Purpose and intent: one workspace identity across creation, panes, plugins,
  labels, scripts and archive, independent of session, worktree and cwd identity.
- Existing implementation and constraints: vessel-proto already owns a UUID id
  macro with private fields, random minting, text parsing and serde validation.
  SessionId and WorktreeId cannot represent an empty or shared-cwd workspace.
- Alternatives considered: alias an existing id, use a path string, add another
  UUID wrapper implementation or an external dependency.
- Chosen approach: reuse the existing macro for WorkspaceId and
  WorkspaceOperationId, exporting both from vessel-proto's root.
- Reason: distinct Rust types catch accidental id substitution; the same wire
  shape follows existing callers without a second representation or dependency.
- Advantages, costs and effects: ten declaration lines and two root exports;
  no field/call is exposed to older peers, so no protocol/schema number is needed
  until registry/create calls are separately registered. Operation identity is
  for creation receipts; it is not an automation approval or credential.

Reference: Paseo 0.15.4
`packages/server/src/server/session/workspace-provisioning/workspace-provisioning-service.ts:238-269`
allocates an independent record even for the same cwd;
`packages/server/src/server/creation/index.ts:64-100` persists request/resource
identity before work and refuses a changed intent with the same request key.
Vessel deliberately uses its existing UUID wire shape rather than Paseo's
prefixed workspace id string. All consumers must use the shared typed value.

Validation: exact text/serde round trips, invalid decode rejection and distinct
Rust type identities for workspace, operation, session and worktree ids. Narrow
checks and source review are pending; no runtime/gate/feature completion claim.
