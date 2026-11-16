# Explicit-stop execution invalidation join

[Decision Log]
- Purpose: an old bound creation intent must observe explicit parent stop before
  the stop's first effect, under the existing Agents incarnation owner.
- Existing boundary: authorized session.kill reaches Sessions::kill; find already
  decides live/stored/missing. The qualified Agents invalidation API owns its
  current UUID/watch under the same registry guard.
- Alternatives: observe liveness later or publish invalidation after stop. Both
  leave an older capture able to race the first stop effect.
- Choice: in the existing Live arm only, invalidate PersonStop immediately before
  the unchanged handle.kill call. No await/other effect is inserted between them.
- Impact: Stored stays a no-op and find errors/refusals propagate unchanged.
  The service's existing agent authorization is unchanged. PersonStop names an
  explicit-stop cause, not proof that the caller is a human principal.

This is the coordinator-assigned single-writer kill-slot leaf on the announced
node head. It adds no helper, observer, Origin/clock API, capture or new authority.
Archive/resume/driver joins remain the lifecycle owner's separate work. Existing
Agents invalidation and actual session lifecycle regressions supply narrow owner
coverage; no common creation capsule/race/current-grant or full lifecycle
qualification is inferred. Ordinary current Node all-target strict and affected
existing tests precede V3 and normal receiving integration.
