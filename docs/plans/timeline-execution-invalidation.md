# Execution invalidation producer, first checkpoint

The coordinator assigned W23 the common fence. V3 row251 source-passed the exact
Agents / owned metadata lease / synchronous actor admission proposal in
timeline-execution-fence.md at 99ab9cc02. This isolated checkpoint implements ONLY
incarnation values and invalidation, based on the announced node gate52 head.
It imports no rich receipt, notice store, actor queue or app ancestry.

[Decision Log]
- Purpose: give GKS lifecycle and W2 detach one real invalidation owner, without
  coupling their call-site joins to unfinished receipt/notice runtime.
- Existing assets: UUID identifier macro and the Agents entry/registry mutex.
- Alternatives: ProcessLifetime or is_live snapshots cannot distinguish a later
  execution with the same session identity; a second registry would grow apart.
- Choice: one fixed-size incarnation/latest-change watch per existing entry.
  Invalidation replaces it under the actual registry lock. Retirement invalidates
  naturally before removing the entry; forget/replacement closes the old watch.
- Why: old captures can observe a change or owner loss but cannot recapture a
  newer execution through an identical SessionId.
- Consequences: credentials, scopes, parentage and live-session counts are not
  elevated or changed by invalidation. The remaining capture, metadata lease and
  atomic queue capsule still must be implemented and reviewed before admission.

Exact owned API: Agents::invalidate_execution(session, ExecutionInvalidation).
The closed causes are PersonStop, Resume, Archive, Detach, DriverReplaced,
GrantChanged and natural Ended. Transition owners call after ordinary authorized
state validation and before first effect; they retain their actual call sites.
This checkpoint joins ONLY natural retirement. It does not claim the other joins.
Diagnostics run after releasing the synchronous registry lock.

Proto ParentExecutionStamp and FinishGrantCeiling are serializable correlation
values, not private runtime capabilities. No request/response/event/method accepts
them. The shared feature remains None, with no new constant or allocation. Only
ReadOperate can qualify future finish admission, intersected with actual current
grant at queue insertion. Admin is unrepresentable in that saved ceiling.

Authored tests use the real in-memory Agents registry, no process or provider:
all explicit causes change the target entry/watch, an unrelated entry/grant stays
unchanged, natural retirement closes it, and replacing the same SessionId cannot
reuse its incarnation/watch. Pure proto fixtures reject invalid UUID/admin while
preserving the exact stamp. These are not capture/actor atomicity or transition
call-site tests. Exact VM strict and runtime execution remain owed.
