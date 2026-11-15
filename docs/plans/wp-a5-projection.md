# A5 shared attention projection

Status: source extraction awaiting exact narrow checks and bounded review.
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
  by attentionTimestamp/updatedAt; W11 owns that selection, not this module.
- server session.ts:5714,5811 initializes workspace Done;
  workspace-directory.ts:424-429 ignores null/no-bucket terminal activity;
  :478-493 explicitly defines empty workspace Done. Its test :525-530 covers
  absent agent plus uninstrumented terminal => workspace Done.

TerminalState::Untracked is neutral ONLY for workspace projection, never a
terminal Done/exit/archive proof. Known root Working/NeedsInput survives it.
Missing/unavailable/future terminal facts and unknown root/membership remain
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
