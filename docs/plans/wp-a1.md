# Message search from the command palette

Status: contract; implementation follows. Extends ADR 0037; no new wire calls,
protocol fields, crate edges or dependencies.

## Reference behaviour

Paseo 0.15.4 `packages/app/src/agent-stream/chat-find/model.ts:109-174`
cancels an earlier query immediately, debounces the next, and searches on the
host; `:229-284` loads and reveals the located message and reports failures.
`chat-find/index.web.tsx:76-92` binds search, loading and reveal.
`command-center/interaction.ts:69-116` resets selection for a new scope or query,
keeps identity for passive results and reveals only on a person's command.
`command-center/results.ts:115-119` lists workspace, agent, file and contribution
results; a global Messages mode is an intentional extension, as ADR 0037 already
records that Paseo searches titles rather than every transcript.

For the cross-session extension, GOAT
`apps/web/src/components/CommandPaletteThreadItems.tsx:69-89` searches connected
environments; `:193-254` supplies message time, snippets, stable shown order and
opens find on the matching message. `ThreadSearchMatch.tsx:32-49` supplies role
and semibold matched text. These are behavioural references, not copied code.

## Decision log

- Purpose and intent: make the existing cross-session search reachable through
  the normal palette, with keyboard, pointer and compact-sheet interaction.
- Existing assets and constraints: `view::search::MessageSearch` already merges
  and pages node replies; `link::search` sends the existing `SearchMessages`
  call; `conversation::search` draws rows and reveals a matching timeline
  position. The palette has no caller for them. Shared shell/link files are at
  their budgets and require no feature logic for this change.
- Alternatives: a separate search screen duplicates the palette; local transcript
  reads duplicate the node's index and miss unloaded history; a new search
  library is unnecessary. The standard library plus the existing kit suffice.
- Choice: modify/reuse these assets. Add a typed Commands/Messages scope and
  stable message cursor in view, an area module composing the existing command
  frame/segmented controls, and EN/KO Fluent labels. The existing link boundary
  remains the sole node adapter and is exercised with in-memory replies.
- Why: preserves the node's validated query, ordering, paging and older-peer
  refusal, while making this one missing user-visible path complete.
- Advantages, costs and effects: no backend change or new dependency; results
  remain unavailable on old nodes and offline machines cannot be searched.
  Incoming query edits invalidate old rows and replies before debounce.

## Contract and stated differences

Commands remains the default scope on each opening. The Messages segment keeps
the current input, starts a search of every online machine, and lists the
session, machine, message time, role and marked snippet. Up/Down wrap through
results and More; Enter and pointer selection use the same action. More pages
without closing the palette. Empty, invalid, searching, no-match, older-node and
failed-node states remain explicit. A missing session at selection shows a
notice in the palette instead of silently navigating nowhere.

Closing or returning to Commands invalidates pending search work. A selected
hit retains the validated query before clearing, then opens the session, reveals
its record and opens find on the same words. Stale pointer callbacks cannot act
on results from an earlier query or a closed scope.

Differences: the existing node index's newest-first ranking, bounded pages,
Unicode limitations and index catch-up remain ADR 0037's. The existing cross-node
view shows ten more rows per More and preserves rows already shown. Debounce
retains its existing duration to respect the node's concurrent search cap rather
than changing to Paseo's per-session duration. The row names its machine rather
than GOAT's project. Messages is explicit so ordinary command searches do not
fan out to every node. Motion reuses the existing command frame and kit fills;
reduced motion remains immediate. No environment-only example overlay ships.

## Structure and data flow

`view::palette::messages`: scope and message-selection rules, no UI or I/O.
`app::areas::palette::messages`: scope controls, message list and activation.
`app::areas::conversation::search`: debounce, existing link asks/replies, result
presentation and prepared destination. `link::search` -> node `session.search`
-> `Event::Searched` -> view merge -> palette rows -> session reveal/find.
Tests cover actual shell callbacks as well as cursor and stale-result rules.

## Validation

Per commit: native fmt, size/boundaries/tokens; narrow changed-crate checks.
At package end: one complete native gate on the exact commit and fake-fleet
captures in both themes/languages at desktop and compact sizes. Review includes
the evidence and all unverified surfaces. No production node or real agent.

## Resume point

Done: references and existing interfaces inspected; contract recorded.
Next: typed scope/cursor; palette wiring; immediate invalidation and destination
retention; regressions; narrow checks; whole gate, captures and review.
