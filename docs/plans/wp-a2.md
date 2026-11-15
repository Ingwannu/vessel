# Native conversation history import

Status: implementation in progress. Implements ADR 0053 after its existing
node contract reaches the native base. No new wire method or dependency.

## Reference

Paseo 0.15.4, read before implementation:
- `packages/app/src/components/import-session-sheet.tsx:293-366`: title,
  activity, preview, directory and disabled importing rows.
- `import-session-sheet.tsx:394-476`: adaptive sheet and bounded long list.
- `import-session-sheet.tsx:478-572`: folder scope, provider filter and search.
- `import-session-sheet.tsx:678-713`: preserve cwd, close/navigate on success.
- `import-session-sheet-view-model.ts:5-20,76-100`: paging, deduplication,
  newest-first ordering.
- `import-session-list.web.tsx:10-154`: virtual list and page keys.
- `screens/new-workspace/import-session-button.tsx:8-55`: entry below composer.

## Decision Log

- Purpose and intent: bring conversations created outside the app into its
  timeline, preserving the account, directory and native conversation id.
- Existing implementation and constraints: reuse commands/events, session
  navigation, Fluent, dialog, combobox, fields and scrolling. The starting
  native base lacks agent.history; central integration must precede its wire
  adapter. ADR 0053 specifies own-history import through session.create with
  Structured.resume, cwd, profile and title; session.import copies across nodes.
- Alternatives considered: transcript readers, another wire call, loading
  every conversation for previews, new toolkit controls.
- Selected approach: modify/reuse existing layers and controls. Pure view state
  owns scope, tickets, sorting and selection; link maps existing calls; the area
  draws and navigates. An in-memory fake tests the boundary before integration.
- Reason: preserves node safety and replay rules without harness file readers
  or blocking the UI executor. Standard collections suffice; no new dependency.
- Costs and effects: one agent start per listing page; raw node failures cross
  as codes only. No new dependency edge or change to the accepted wire design.

## Deliberate differences

- No prompt previews: ADR 0053 lists titles and times without loading sessions.
- One selected agent/account instead of probing all enabled agents: listings
  start agents and the node admits only two probes concurrently. Explicit
  selection binds imports to the account whose history was listed.
- Opaque cursor paging follows the node contract instead of growing limits.
  Empty pages with a cursor remain pageable. Search filters loaded titles and
  paths and explicitly says it searches loaded conversations.
- Already imported rows offer Open rather than disappearing (ADR 0053).
- Listing pins the authenticated node id; a reused machine-book alias refuses
  an old selection instead of creating on a different node.
- Multi-selection extends Paseo's single import per the requested package.
  Failures remain visible; uncertain creates are never automatically repeated.
- The shared kit supplies adaptive geometry, focus and motion; no second modal.

## Structure and flow

view::history owns typed requests, results and state; link::history maps node
calls; app::areas::import composes controls. EN/KO history.ftl owns the words.

Input -> scoped request -> keyed page -> sorted/deduplicated rows -> selected
resume specs -> creation receipts -> open session. A scope change invalidates
old list answers. Creation receipts remain independent of subsequent selection.

## Validation and resume point

The native tree now merges the assigned8d5513e4 package base. agent.history is
wired to its existing typed request, not an Unsupported stub. Exact a53eb8ad
link all-target strict Clippy passed remotely. New framed paging/old-peer and
activation tests plus final app/view checks remain pending. Earlier12 state
tests passed before later identity/activation coverage was added; that is not
proof of the current complete package. No fake fleet/captures/whole gate yet.

## Existing-history activation

Existing rows use the same pinned batch receipt state as new imports. The link
fetches the original session and verifies full node identity, native transcript,
harness and account (including implicit default aliases) before resuming an ended
one. session.resume reopens that same row and clears its archive mark; a live
session opens directly. Lost resume receipts require refresh and never create a
replacement. Tests use the actual framed in-memory transport.

Paseo restores an archive flag after a failed activation. The existing Vessel
resume contract can reopen before its agent handshake fails; that session may
then remain unarchived. This adapter reports that refusal and preserves the
original id. It does not perform a racy compensating archive RPC, which could
kill a successful concurrent resume. Full transactional restore parity would
need a node-owned activation operation, not a client-side check/then/kill.

Expanded A4 scope and dependencies are recorded in wp-a4.md. Cross-machine move
is beyond the complete parity package, after its gates and review.
