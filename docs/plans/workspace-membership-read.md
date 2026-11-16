# Connection-scoped workspace membership read

Status: source leaf prepared; changed-source checks and review are owed. This
extends the already reviewed registry metadata internals, not public RPC policy.

[Decision Log]
- Purpose: workspace attention actions need exact owned resource IDs from the
  same writer transaction as lifecycle, role/attention checks and mutations.
- Existing constraints: Store::workspace_registry opens a separate read
  transaction and therefore cannot supply that atomic mutation boundary.
- Alternatives: a client loop, cwd/Creator inference, another registry, duplicate
  SQL/decoders, or a test-only wrapper which the real consumer cannot call.
- Choice: expose the existing canonical decoder inside vessel-store, parameterized
  by an optional workspace. A WorkspaceId converts to an exact selection; whole
  snapshot hydration explicitly passes None on its held transaction.
- Reason: both real uses share one decoder, identity type, ordering and error
  boundary. No unused producer or test-only feature promotion is needed.
- Impact: callers own their connection/transaction and permission/lifecycle/
  role/attention checks. Association includes pending, failed and archived data;
  it is not process liveness, root eligibility, backing generation or exit proof.

The API is crate-internal registry::workspace_memberships(&Connection,
impl Into<Option<WorkspaceId>>) -> Result<Vec<WorkspaceMembership>, StoreError>.
The intended mutation call passes its exact WorkspaceId directly, after
require_active on the SAME connection. None is only complete snapshot hydration.
This introduces no public request, method, schema, migration or dependency.

Two authored regressions observe pending IDs inside an uncommitted writer
transaction, exact workspace exclusion and rollback; corrupted selected resource
kind fails closed while an unrelated selected workspace remains empty. Whole
snapshot production hydration uses the same function and its existing atomic
read transaction. These are source assertions until actual VM tests pass.
