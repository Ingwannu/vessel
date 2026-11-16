# Canonical workspace-relative session role

[Decision Log]
- Purpose: native workspace projection and the same-writer workspace unread
  selector must use one current-parent policy.
- Existing assets: ADR0090 Parentage validates immutable Creator plus the durable
  detach receipt. The native prototype's Relation/workspace_role already encodes
  Paseo workspace-root-policy.ts6-16; registry snapshots provide NodeId and the
  existing WorkspaceId, with ownership resolved by full SessionRef.
- Constraint: a store transaction cannot depend on native view/layout. A local
  A5 classifier would duplicate the policy. There is no root WorkspaceRef type.
- Choice: move the same pure Relation policy into vessel-proto::subagents and
  return WorkspaceRole Root/Unknown/Child. Generic W receives the caller's
  canonical node-scoped key, normally (NodeId,WorkspaceId), with no new ID alias.
- Why: known no-parent includes durable detach; attached cross-workspace agents
  are roots in their own workspace; missing parent ownership stays Unknown.
  Capability None/old-peer/corrupt metadata also stays Unknown.
- Impact: no RPC, event, schema, feature assignment, dependency or mutation is
  added. These metadata values are not a grant, membership/freshness proof or
  a substitute for same-writer admission and current ownership. Archived/history
  membership, activity aggregation and unread selection remain caller concerns.

The root store consumer computes inside its existing writer transaction using
validated stored metadata; native consumers join exact authenticated snapshots.
Parent lookup uses full SessionRef, never Creator/cwd inference. W12's existing
layout::AgentRole stays unchanged: a later native adapter exhaustively maps the
three root variants without copying the policy. The prototype's native Relation
definition must be replaced by the canonical re-export in that receiving union.

This isolated values-only producer follows V3, node integrator and B0 routing.
No consumer readiness, live role projection or workspace mutation is claimed
before its exact checks/review and announced receiving head. Tests cover the
policy table including equal workspace UUIDs on different nodes, unknown owner
facts, capability-conditioned validated parentage and full-reference roundtrip.
