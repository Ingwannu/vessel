# Search, chat find and prompt outline

Status: message search and expanded find/outline implemented; narrow checks and
prompt-index integration remain. Extends ADR 0037; the prompt-index node slice
lives in docs/plans/wp-a3-node.md on the node track. No new dependency or crate edge.

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

Done: typed scope/cursor, palette wiring, immediate invalidation, prepared
destination, EN/KO labels and language refresh. Narrow palette tests: app 5
passed and view 31 passed. Compile warnings were corrected afterward; lint and
the full package gate remain pending.
Next: expanded find/outline and server prompt-index contract; narrow checks;
then one combined capture matrix, whole gate and review.


## Expanded search and outline contract

The package also audits Paseo's timeline search and prompt-index behaviour,
chat find and outline (timeline survey items 17, 18, 48, 49).
`packages/server/src/server/agent/timeline-prompt-index.ts:16-40` indexes all user
messages with collapsed whitespace and a bounded preview.
`agent-stream/chat-outline/hover-intent.ts:44-86` keeps its initial timer while
moving vertically and restarts it only on horizontal transit.

Existing assets: `timeline::Finder` incrementally scans readable text with a
retained revision cache; conversation `find` already yields to the UI
between slices, marks occurrences and reveals folded entries. `timeline::outline`
and the existing rail derive prompts from the subscribed log. Reuse these rather
than introduce another timeline. Differences needing work: find shortcut observer
must not bypass overlay/IME/rebinding policy; closing should restore focus; outline
must honour the Chat setting and its hover intent. The server prompt-index call
is absent from the actual node method table; that boundary awaits the central
node contract or a ruling on deriving the full index from the complete log.
No fictional call is registered.

Current state: global Messages implementation and regressions drafted; narrow
checks queued. Find/outline parity audit started; full-history prompt index
boundary reported. Final captures and whole gate cover the expanded package.


### Find/outline skeleton checkpoint

The existing preference store now owns `Appearance.chat_outline` as typed
`ChatOutline::{On,Off}`, default On with lenient field decoding. The existing
Chat card appends the outline switch; the rail reads that same value. The switch
uses the existing motion-aware kit control, with its own revision. The complete
native-default fixture gains only this serialized field; the page fixture stays
unchanged. E2 approved the seam and the localization owner approved the separate
`chat-outline.ftl` labels.

Find now uses the shared rebindable key listener alone; the old independent
observer is removed. Its action respects route/overlays/IME, and Escape restores
the opening surface's focus. New actual-input regressions cover this; they await
narrow checks. The earlier Messages-only tests passed; this checkpoint is not a
claim that the full expanded package has passed.

### Find and rail implementation checkpoint

Find now exposes typed Idle/Searching/Loading/Ready/Failed phases and typed
Connection/HistoryChanged/Reveal failures. Edits cancel the old identity before
the 120 ms debounce; close/reopen uses allocation identity so an old task cannot
accidentally match a reset counter. Leaving the Session route closes the widget
before the destination takes focus. Loading a search destination has a 5 s
identity-guarded deadline, and failure offers Retry. Rewind invalidates the result
set rather than quietly changing its count. Ordinary appends retain the selected
occurrence without scrolling the reader away.

Deliberate architecture difference from Paseo: native Follow reads the complete
retained session (A7 confirmed it preserves that contract), so local sliced search
reuses the existing Markdown/readable-text cache and does not issue another
server search for text it already owns. This also keeps the page's accent-folding
and answered-question matching, which extend Paseo's user/assistant scope. The
node's existing `session.search` remains the global Messages boundary. If Follow
switches to partial windows, this difference must be revisited before that change
ships. The new metadata-only prompt request is independent of text matching.

Outline hover is a view state machine with ticketed effects: vertical travel
preserves the initial 150 ms deadline, horizontal transit of at least 4 logical
pixels restarts it, active attention responds immediately, and leave/session
changes invalidate old timers. The rail is keyboard reachable, Up/Down select a
prompt and Enter/Space jump. Reduced motion suppresses magnification as Paseo
does. Pill dimensions use the shared kit QUICK curve/150 ms rather than a new
140 ms CSS-specific token; no recurring idle animation. Steers count as prompts.

L approved exact EN/KO `chat-find.ftl` hashes 64c0b300…/b5161722… and retained
`chat-outline.ftl` hashes 89cb7db4…/f073e348…; resource/wording approval only,
not runtime or gate approval. The narrow check on e023b85c compiled product code
and reported five test semicolon warnings, now corrected. Expanded regressions
and complete package validation are still pending. A7's `reveal_turn(entry)` must
precede find/outline row lookup when its reviewed area API reaches the base.

### Typed prompt-index consumer

The app branch now joins the A3 node candidate with Quality B so its consumer
uses actual `ListSessionPrompts`/`PromptIndexPage` types. This candidate is not
released: root gate, V3 review and the integrator's protocol assignment remain
required. The pending call is refused before wire I/O; compatibility keeps the
complete followed-log outline on older or unassigned peers.

`view::prompt_index` owns target/ticket/cursor/epoch checks, bounded accumulation,
coalesced refreshes and the borrowed rail projection. A partial page set never
becomes navigable. `link::prompt_index` owns two-call admission, a named deadline
and code-only errors; the standard command/event/unsent/demo dispatchers register
it. `conversation::prompts` requests metadata at open and latest-prompt changes,
invalidates after observed rewinds, and bounds pending jumps while the existing
Follow delivers bodies. Ready metadata shows every tick before those bodies
arrive. Pointer callbacks keep session, sequence and epoch instead of an ordinal
that could change under them. No second transcript, store, provider call or wire
shape is introduced. The app's metadata and jump tests are pending validation.
