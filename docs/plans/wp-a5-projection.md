# A5 shared attention projection

Status: bounded checked source adoption passed V4 row234 for corrected246f1ba28;
exact view strict rc0 and tests3/0/0. Old911438d2 remains HOLD history. The identical
producer is included in announced green package base6ebdf8e8 (1473/0/2).
This is a pure view policy producer, not the Inbox, node contract, registry
runtime, resource lifecycle, terminal resident or native host badge API.

[Decision Log]
- Purpose: W11 needs one reusable bucket/workspace projection without importing
  the whole A5 Inbox or pending node types.
- Existing assets: A5's pure Bucket, LatestRoot and badge functions already own
  the reference ordering and unknown handling.
- Alternatives: a workspace-local second classifier; adopting the whole Inbox
  branch; an external library. Each adds coupling or diverging policy.
- Choice: modify/reuse the existing policy as view::attention, with no I/O or
  proto dependency. Inbox reexports the same values; no second implementation.
- Why: the canonical workspace owner supplies membership/role/latest-root facts;
  A5 combines only those facts. Standard Rust suffices.
- Cost: a small shared public view module, with consumer/gate proof still owed.

Paseo v0.15.4 references:
- app utils/sidebar-agent-state.ts:20-29 aggregate urgency; agent-state-bucket
  preserves request > failure > working > review > done.
- app utils/workspace-agent-activity.ts:22-48 chooses one latest eligible root
  for the DISPLAY activity index, not the workspace aggregate. W11 owns it.
- server workspace-directory.ts:270-289,365-407 aggregates ALL current eligible
  workspace-relative roots and running same-workspace descendants. The owner
  supplies complete AgentContribution facts; timestamps do not choose urgency.
- app hooks/use-favicon-status.ts:27-39 checks raw running first, then independent
  requiresAttention OR pending permissions. IconFacts preserves both axes; a
  sidebar Bucket cannot recover them. Failure alone does not imply attention.
- server session.ts:5714,5811 initializes workspace Done;
  workspace-directory.ts:424-429 ignores null/no-bucket terminal activity;
  :478-493 explicitly defines empty workspace Done. Its test :525-530 covers
  absent agent plus uninstrumented terminal => workspace Done.

TerminalState::Untracked is neutral ONLY for workspace projection, never a
terminal Done/exit/archive proof. Known root Working/NeedsInput survives it.
Missing/unavailable/future terminal facts and unknown contributor/membership remain
Unknown. The source adapter must prove known plain-shell/uninstrumented facts;
it must not map a missing response to Untracked or an empty iterator.

WorkspaceCount counts eligible canonical workspaces once; it does not count
requests/tabs. Host APIs, current authority and known coverage remain consumer
responsibilities. GPUI dock badge API is an approved later desktop-host item.
Tests cover source-backed empty behavior, distinct icon priority, unknown facts,
counting and Untracked interactions. All Rust checks run via the build VM;
no production permission/service/configuration or infrastructure changes.

Bucket ordering/ALL is aggregate urgency. Sidebar display/shortcut groups are
separate: Paseo utils/sidebar-agent-state.ts:36-42 places review before working.
The producer does not prescribe display order or maintain shortcut state.

[Decision Log: bounded review correction]
- Purpose: prevent the display index from discarding an older urgent root and
  prevent bucket priority from discarding a running agent's raw icon activity.
- Existing constraint: 911's one LatestRoot aggregate and icon(Bucket) lose
  independent reference facts, even though its strict/tests passed.
- Choice: keep LatestRoot only as a DISPLAY carrier; workspace accepts all
  owner-resolved AgentContribution values. IconFacts uses typed activity and
  attention axes; no inferred conversion from Bucket is provided.
- Why: workspace-directory and use-favicon-status have different source inputs.
  Duplicating role/membership resolution here would violate canonical ownership.
- Impact: intentionally changes the unused shared API before adoption; no live
  consumer exists. Complete coverage/ownership remain receiving-owner duties.
  Unknown remains explicit; no timestamp is needed for a complete aggregate.

Regression cases: older NeedsInput plus newest Done remains NeedsInput; Done
root plus running descendant becomes Working; known neutral descendants and
Untracked terminals remain neutral; absent-only known Untracked is workspace
Done; any unknown contributor remains Unknown. Icon tests cover a SINGLE
running+pending agent, failed without attention, pending-only, either cross-agent
order and Unknown. These tests do not claim canonical registry/role admission.

Prior evidence retained: exact911 all-target View Clippy rc0/62s, executable
ed950 tests3/0/0. That evidence does not verify the corrected source. Entire
A5 runtime, host APIs, captures, whole gate and consumer integration remain owed.
