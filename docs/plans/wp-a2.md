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

Done: reference/asset analysis, typed state and import boundary, EN/KO words,
area registration and controls, profile.list and create-resume adapters.
The initial pure-state run passed 12 tests before the final node-identity guard;
its updated tests and the changed crates' Clippy are pending. Fast size,
boundary and token checks passed. This is a skeleton, not a working or gated
history-import claim: the current agent.history adapter explicitly returns
Unsupported until the central native base supplies its existing contract.
No fake fleet or capture has been started.

Next: finish narrow checks, merge the centrally supplied base (never merge the
node line independently), map agent.history, test the real framed boundary,
inspect fake-fleet captures, run one native gate and submit one package review.
Cross-machine move is the following package.
