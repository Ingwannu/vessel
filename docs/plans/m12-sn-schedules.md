# Native Schedules rollout

[ADR 0072](../adr/0072-schedules-screen.md) records the node contract and
its differences from Paseo. P1 started from package base 14.105 and
incorporates 14.106 for the review fixes. Paseo source references below
are relative to the pinned 0.15.4 tree.

## P1 · Screen and rows

TRACKER 581–583, catalogue F-I4–I6 and F-I29–I31. The existing sidebar
slot and the Command Center open the same route. The GPUI area uses the
kit's header, segments, picker, settings rows, badge, menu and dialog.
The row menu trigger exposes its Button role and expansion for its own
row so assistive technology can tell which menu is open.
Fluent prepares English and Korean text. The view model owns host
aggregation, per-host state, filtering, ordering, and optimistic row
feedback. Offline hosts remain selectable. Aggregation skips them until they
have an admission, so a banner describes a failed read on a reachable host.
Link calls the existing schedule and target-metadata RPCs
through the selected physical admission, with bounded work and a
weak guard on queued replies. The node remains the source of record.

Reference lines:

- Route, navigation and palette in `packages/app/src/app/schedules.tsx:4`,
  `components/sidebar/sidebar-nav-rows.tsx:168` and
  `command-center/root-registration.tsx:231`.
- Filters, newest-first sorting and body states in
  `packages/app/src/screens/schedules-screen.tsx:144` and `:226`.
- Online host aggregation and reconnect invalidation in
  `packages/app/src/schedules/aggregated-schedules.ts:61` and
  `runtime/host-runtime.ts:2559`.
- Row badges, ordering and actions in
  `packages/app/src/components/schedules/schedule-row.tsx:78`, `:171`
  and `:265`, confirmation in `schedules-table.tsx:116`, and optimistic
  mutations in `packages/app/src/hooks/use-schedule-mutations.ts:145`.

### Decision Log

- Intent: ship a reachable list and row operations on the current native
  app, then add the form and heartbeat editing in their own packets.
- Constraints: the abandoned branch carries unrelated code and predates
  the current app; an asynchronous answer can arrive after its source
  connection has been replaced.
- Alternatives: merge the old branch, rewrite the list and its tests,
  or port schedule-specific decisions and rewire the current app.
- Choice: reuse the view's bounded list/cache, original-admission and
  row intent checks, pure descriptions and race cases. Use the current
  app's shell, link ownership and kit. A provisional status or delete
  is computed from the row's pending ticket over an untouched snapshot.
- Reason: a failed write can revert only the matching row. An ambiguous
  result invalidates the read and cannot replay the write. A route
  arrival, connection change, explicit retry or mutation can request
  a new list. The relative clock only updates cached display text.
- Effects: the node's Unknown and Creator gone states and list cap remain
  visible. Unavailable admissions or denied grants restrict actions.
  The client calls every existing-session target a Heartbeat and
  offers authorized deletion as its sole row menu action. The target
  drives that label regardless of who defined the schedule. The
  node's elapsed cadence stays exact. The form for new-session targets
  and the heartbeat detail arrive with P2 and P3 respectively.

## Follow-up packets

P2 covers 584–585 with the form and cadence editor. Device IANA comes
first, then the machine zone. Node preview and typed bounds govern the
save. The form keeps stored expiry and all fields it does not edit.
Person saves disclose the node's ownership transfer. Run history,
expiry controls and the active-empty docs link stay outside these
packets. Per-run worktree isolation remains a parity gap.

For 587, P3 adds the heartbeat detail and cadence-only edit. A node-line
packet must supply the revision-checked, owner-preserving operation
before the form can enable Save. On older nodes, existing heartbeats are
read-only with deletion. The agent's existing tools create heartbeats.

Run native fmt and clippy in the short lane, then the whole native and
wasm32 checks via the remote gate. Capture against the isolated base14.105
fleet for the initial pass and base14.106 for the review fix, via
`spikes/native/scripts/fleet.sh`.
