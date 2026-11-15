# Workspace attention mutations: contract preparation

Status: design only. No new RPC, handler, SDK, migration, timestamp producer,
scope grant or native control is installed by this document. Existing A5 has
five pending methods and ATTENTION_NOTIFICATIONS_PROTOCOL remains None.

## Source behavior and existing assets

Pinned Paseo 0.15.4:

- `packages/app/src/hooks/use-workspace-read-state.ts:23-43`: clear is offered
  for attention/failed; unread is offered only for a supported Done workspace.
- `packages/server/src/server/session.ts:7635-7768`: clear enumerates exact
  owning workspace identity, excludes archived agents, requires current
  attention and preserves pending permissions/permission attention. It does
  not filter to roots or clear terminal attention.
- The same file `:7770-7837`: unread chooses one non-archived workspace-relative
  root, Idle/Closed, no attention/pending permission, descending updatedAt and
  ascending id. It rejects when there is no eligible finished root.
- `packages/server/src/server/authorization/operation-permissions.ts:208-209`:
  both workspace operations require workspace.write. They do not justify
  widening Vessel's existing general session.mark scope.
- `packages/app/src/workspace/mark-unread.ts:11-28`: exit pane viewing before
  requesting unread and restore that focus token only if the mutation fails.
- `packages/server/src/server/agent/agent-manager.ts:1017-1031`: updatedAt is a
  monotonic mutation clock. It is not the last retained transcript timestamp.

Reusable Vessel boundaries are W11's WorkspaceId/registry membership and active
workspace check on the caller's writer connection; W2's capability-gated
Parentage and workspace-relative role contract; existing marks transaction,
live pending veto/marking lock and committed inbox rollup; and W12's keyed
layout focus exit. No client-side session enumeration or second read engine
is needed.

## Proposed implementation boundary

[Decision Log]
- Purpose: a workspace mutation must operate on the current owned set, preserve
  requests and deliberate unread state, and never retarget across a connection.
- Existing constraints: membership alone includes historical/pending resources;
  it proves neither root role nor runtime activity. The current SessionAttention
  DTO has counts/explicit_unread but no authoritative updatedAt ordering fact.
- Alternatives: a client loop over sixteen Inbox entries; cwd/Creator inference;
  sorting by legacy activity, ended_at or the last retained log event. These
  silently omit resources, misclassify roots or substitute another clock.
- Choice: prepare two person-only Operate mutations against canonical
  WorkspaceId. Within one existing writer transaction: require_active, read
  exact memberships, join current authoritative session visibility/status and
  attention, apply existing read/unread marking semantics, then commit/wake.
  Clear joins all eligible owned sessions, not only roots. Unread requires W2
  role coverage and a real mutation ordering clock before selecting one root.
- Why: the node owns the set and selection at effect time. A lost reply is not
  an offline replay instruction. Original app ConnectionContext and workspace
  ticket must survive submission and settlement.
- Impact: the missing membership/role/clock/status producers remain explicit
  prerequisites. Unknown coverage refuses a mutation rather than becoming an
  empty success or picking an arbitrary session. No staging value grants a
  scope, ownership or liveness fact.

The member helper is currently source-only W11 f9894b9a on announced node1c762ccf;
it is not adopted before its own checks/V3. W2 SUBAGENT_RELATIONS remains None,
so a missing receipt cannot establish a root. A separate read transaction via
Store::workspace_registry is unsuitable for this mutation.

## Timestamp producer to resolve before wiring

The node needs a durable monotonic session mutation timestamp, updated by the
canonical append/mark/metadata mutation sources and read in the same transaction
as attention. It must not use read time, file mtime, a client timestamp or a
pruned head-page maximum. A nullable legacy value preserves Unknown rather than
inventing historical updatedAt. The lifecycle owner keeps its own append after the announced head; protocol
projections must gate any optional timestamp with that owner's eventual
canonical feature constant.

Coordinator ruling: the session-record lifecycle owner supplies ONE durable
clock, the canonical append/mark/metadata producer updates and its own appended
migration in merge order. A5 consumes that checked/reviewed field and adds no
clock or migration for it. Exact consumer requirements were sent to lifecycle.
This is a proposed consumer data contract,
not a claim that such a clock or a compatible native workspace adapter exists.

## Required behavioral verification

- Clear includes eligible same-workspace descendants, skips another workspace
  sharing cwd, preserves pending requests and archived/internal visibility.
- Unread ignores a newer same-workspace child, includes a canonical cross-
  workspace root, uses updatedAt/id ties and refuses absent/unknown coverage.
- Membership/attention changes in the same writer transaction, rollback,
  deletion/archive and delayed pending append cannot produce partial marks.
- Exact-entry Seen/resolution never clears deliberate whole-session unread.
- A stale app context/workspace ticket cannot apply a reply or restore another
  pane's focus; failed mutation restores only its captured current focus token.

These are future test obligations. No test or runtime result is inferred from
this design or the existing Inbox/claim tests.
