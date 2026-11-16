# Canonical workspace role in the native layout

## Decision Log

- Purpose: accept the already shared root/child/unknown relation projection in
  native layout without a second classification policy or duplicate enum.
- Existing assets: `vessel_proto::subagents::Relation::workspace_role` uses the
  caller's exact node-scoped workspace owners. Its `WorkspaceRole` has the same
  three variants as the former native `layout::AgentRole`.
- Alternatives: an exhaustive adapter would work but retain two identical types;
  copying the classifier could drift when the canonical relationship changes.
- Choice: re-export the canonical `WorkspaceRole` as the existing `AgentRole`
  path. Layout's existing internal persisted close role still maps exhaustively;
  no serialized layout field, open algorithm or close plan changes.
- Why: canonical relation output now passes directly into `Layout::open`, while
  every current caller keeps the same path and variant names. No new dependency
  is needed: View already depends on Proto.
- Cost: the alias inherits Proto's value traits, including Default(Unknown) and
  serde. They are value semantics, not metadata freshness or effect permission.

## Source and data flow

The root values producer is already on the announced App base14.2. The sole
policy remains `crates/vessel-proto/src/subagents/relation.rs`; no root source is
changed. The provider supplies authenticated current relation and exact
`(NodeId, WorkspaceId)` child and full-`SessionRef` parent ownership observations.
Missing, stale or unavailable observations remain Unknown. Native layout accepts
the resulting role and keeps its existing compact-sheet and close-effect rules.

The runtime registry lookup/hydration owner is still unavailable here. Current
App calls that explicitly use Unknown keep doing so. This type cleanup installs
no query, implicit membership, live child admission, archive/close capability or
wire activation. A role is presentation metadata only; the concrete owner must
still acknowledge its existing lifecycle close plan.

The authored native View cases pass canonical classifier output directly into
the existing layout and inspect compact child versus cross-node root placement
and existing close effects. Missing owner metadata must still require resolution.
These are synthetic metadata/model cases, not a current registry or transport
proof. Static checks and actual receiving View strict/runtime remain owed.
