# Search, chat find and prompt outline

Status: source for message search, expanded find/outline and the typed prompt-index
consumer is present; the complete feature is not ready for acceptance. The checked
shared delivery fixture and original-connection destination/follow carrier remain
prerequisites, alongside regression execution, final gates, captures and review.
The new RPC is pending protocol assignment; the complete followed-log outline is
the compatibility path. Extends ADR 0037; the node slice lives in
docs/plans/wp-a3-node.md on the node track. No new dependency or crate edge.

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

Per meaningful commit: local native fmt, static size/tokens and diff checks;
remote-only dependency-boundary and changed-crate checks. All Rust compilation
runs through the shared remote gate; one own queued/running build at a time.
At package end: one complete native gate on the exact commit and fake-fleet
captures in both themes/languages at desktop and compact sizes. Review includes
the evidence and all unverified surfaces. No production node or real agent.

The initial Messages slice passed five app and 31 view tests before the expanded
find/outline work. That result is scoped evidence, not the complete package gate.


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
must honour the Chat setting and its hover intent. The new metadata-only node
call is now registered through Quality B with an unassigned protocol constant,
so it is refused before I/O until integration. No fictional wire version is used.
Final captures and whole gates cover the expanded package.


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

Find retains a single closed-widget query and moves it back into the next live
bar on reopening the same session, matching Paseo `ChatFindModel.close/open`.
Changing sessions clears this memory. Escape restores the opening focus;
panel deactivation only blurs the disappearing find field and never focuses a
hidden composer. Editing a pending query cancels only the reveal address owned
by that find target. The link also refuses an unknown welcome identity until
the shared fleet projection supplies one, so a reused book name alone is never
read authority for metadata.

Native focus difference: Paseo's web close focuses its stream viewport. The
current kit VirtualList has no focus target, so native retains and restores the
opening control's FocusHandle, consistent with its shared modal controls. This
keeps keyboard ownership explicit without adding a second viewport controller.

### Resident integration

The existing `Searching` object now belongs to `PaletteArea.messages_search`,
once per window, instead of the resident conversation area. All existing Shell
search methods and the single Messages renderer remain the boundary. Replacing
a conversation resident therefore cannot clear or multiply the global search.
Destination selection still goes through the existing `open_session` and
message reveal/find path, so W12 owns its registry cutover once. A regression
replaces the complete conversation area between two machine replies and checks
that the same window query and rows remain live. Per-resident hold excludes this
window-global controller and preserves only local find/index/jump/hover work.

Actual-shell prompt navigation tests now cover stale painted node/epoch/address
rejection, an unloaded destination deadline, a newer jump surviving the older
deadline, and the actual rail disappearing/reappearing through the existing
typed preference handler. These tests are source-complete and await execution.

Prompt refresh follows the last user-content record, including history chunks,
rather than only the prompt's first sequence. The first address remains stable
when another chunk extends its preview; using that address as a revision could
otherwise leave a completed metadata set with the first chunk's preview. The
existing view coalesces these edits while a read is in flight. A shell regression
extends one prompt twice and requires a fresh read after each completed set.

Closing or failing find cancels its outstanding reveal address as well as the
bar task, matching Paseo `ChatFindModel.cancel` and its reveal abort signal.
Clearing prompt navigation similarly cancels only its owned pending address.
An unrelated inbox destination is preserved. Without this boundary a body that
arrived after cancellation could still scroll the reader. Shell regressions
cover close, failure, unrelated addresses and disabling the outline mid-jump.

Compatibility limitation on the current timeline base: its local projection
merges user chunks only into the immediately preceding prompt. A harness-log
entry between replay chunks can therefore produce two local fallback ticks.
The authoritative metadata index coalesces such internal records. A7 owns the
canonical timeline projection and has been given the exact fold/log boundary;
A3 does not duplicate or change that renderer's folding rules.

Prompt targets carry the existing shared `ConnectionContext`, captured from the
fleet snapshot, rather than issuing another per-area epoch or resolving by alias
alone. A readmission of the same durable NodeId starts a new request identity,
so an Unsupported set can recover after an upgrade. The actor selects that exact
immutable admitted connection once; its common `SelectedConnection` guard fences
retirement before first poll and after the reply. Unknown or stale admissions and
a machine/context mismatch send no frame. The real-shell regression readmits the
same node and requires a fresh read. This preserves the known rewind floor across
readmission; an old admission cannot publish an obsolete metadata page.

Pane selection calls `Shell::conversation_deactivated(window,cx)` against the old
resident before swapping active resource getters, even when both routes are
Session. This closes find, preserves its remembered query, cancels local prompt/
hover work and invalidates its old tasks. Route departure calls the same boundary.
Without it, a task whose Shell getter changed could abandon the hidden resident's
search in progress. A GPUI regression deactivates without changing the route.

The standalone global-controller prerequisite `f9d7f460` is now ancestry of A3.
At union its extraction overlapped the existing owner implementation: keep the
head immediate-ticket invalidation and single palette field; add its actual GPUI
residency regression, with only the new shared Machine epoch fixture field.
Keep the existing identical detached-command helper once. Standalone narrow
proof does not approve this wider A3 union; it remains pending its own gate.

Global Messages pages now retain the same shared admitted connection for every
page of a query. A fleet admission change restarts the query immediately, before
debounce, so a cursor or late page from an older connection cannot join its
replacement. Choosing a painted hit also checks its source against the current
fleet. The actor uses the common immutable SelectedConnection, with a named
15-second deadline; only safe structured error codes survive projection, and
timeout/failure copy comes from the approved message-search domain. The session
selection itself still uses the existing canonical open_session boundary; W12
owns the context-bound pane/session cutover, not another A3 navigation registry.

A3's staged W12 owner adapters are resource-local: `navigation_held` votes Hold
for an open find field, active metadata read/jump, or pending hover intent;
`navigation_retired` fails find without changing its input/field identity and
invalidates only that resource's prompt/hover work. Both hooks remain test-only
until W12's single registry cutover consumes them. The real current find-failure
path already uses the same local primitive. A GPUI fixture retires a hidden area
after replacing the active one and requires retained input/identity and no active
area mutation. A7 moves the original area in its one TimelineSnapshot; A3 creates
no second snapshot, map, authority or follow manager. The registry must reject
retired binding/operation tickets before dispatching any metadata answer.

Search failures are retained as the pure `SearchFailure` enum rather than peer
text or a translated string captured on arrival. Rendering resolves its current
prepared message-search getter, so a language change preserves query/tickets and
rewords the same failure fact. Existing generic view refusal coverage is adapted
to the typed fact; its ordering/no-match behavior is unchanged.

Transport retirement discards metadata addresses but preserves the known rewind
floor. A temporary missing fleet admission also preserves the last observed node
for history comparison only; it never permits an RPC without ConnectionContext.
A known different node or session clears that guard. This prevents disconnect /
readmission from accepting metadata older than a rewind already observed on
Follow. The pure regression includes the unavailable and unknown-admission gap,
not only an immediate old-context → new-context swap.

The observed-rewind guard also accounts for retention passing that marker. A
fresh epoch whose retained start is after the known rewind no longer contains
the marker; rejecting it forever would strand the metadata index until a full
reset. A boundary at or before the marker still requires the known rewind. The
pure regression tests both sides of this boundary without weakening page-set
epoch consistency or accepting older pre-rewind data. Runtime check is pending.

The staged navigation vote and retirement hook recurse through the existing
`Origin::Shown` area accessor. The whole TimelineSnapshot includes those areas,
so an origin metadata read/jump/hover or find field must also Hold installation
and transfer. A7 still owns non-navigation loading/overlay/effect votes. The
regression places a read two origin levels below an idle main area, requires
Hold, then retires that exact tree and verifies no pending read or ticket reset.
No new snapshot, registry or global controller is introduced. Execution pending.

Base8b integration preserves the published standalone ownership move while
retaining A3's later search behavior. Its incoming `f9d7f460` search body predates
immediate query invalidation and prepared destinations; resolving that overlap
keeps the newer owner implementation, one PaletteArea controller and one
Commands/Messages renderer. The original residency regression remains with one
Machine epoch field; the later language-fact regression remains alongside it.
The incoming staged resident/minter modules and boxed preference command remain
unchanged. All architecture notes from both lines are retained. This source
resolution is not a changed-union compile or runtime receipt.

The canonical session owner confirmed that alias-only Open/Command::Open cannot
authorize an original-connection search destination. The eventual owner entry
resolves a SessionOpening against the current workspace lookup, admits the real
claim/context and FollowBinding before hydration, then selects the exact original
connection at actor dispatch. That carrier is not a checked live API yet. A3
does not add an opener or registry to bypass it; current navigation remains
unclaimed pending the sole core cutover. Queued search preparation and test
requirements are recorded in `wp-a3-delivery-fixture.md`.

The full A3 owner now preserves the portable base hook's conservative Hold for
any unresolved shared reveal and pending/visible hover. Its one public staged
query/retirement entry lives beside find; prompt metadata/jump contributes an
actual owner helper, and recursion still covers every shown origin. Retirement
uses the full controller's Rc identities and typed failure state, clears stale
match indices/refresh work, and retains input and other owners' reveals. This is
a new source integration, not approval inherited from portable producer 5ab's
V1/narrow pass. Full A3 execution and scoped callback/authority acceptance remain
pending its own consumer union and gates.

A7 has identified a displayed-body policy for complete spoken-input wrappers
and an exact voice-instruction prefix. Its borrowed helper is not yet a checked
consumer dependency here; body/find range alignment waits for that union.
Direct reference inspection keeps metadata preview separate: Paseo's
`timeline-prompt-index.ts:16-36` only collapses raw row text, and
`chat-outline/rail.web.tsx:95,178-179` displays that preview unchanged. The bounded
metadata preview therefore stays raw as in the reference, without duplicating
A7's body projection or importing a native-view dependency into the node. Global
raw-source search may still hit instruction terms hidden by body presentation;
that remains an explicit search/display integration difference.

Base9b integration takes the announced app line's root handlers, registry and
diagnostic policies unchanged, appending only A3's pending prompt row after all
head rows and adding its entry after the head's complete forwardable fixture.
Both architecture texts remain. The four owned standalone prompt-index test
files are aligned with the gated node producer f7c8ba9f8 (canonical constructors,
explicit limit defaults and awaited durable close); no foreign test is replaced.
The conflicted generated TypeScript takes the announced head's bytes without
manual edits. Its A3 additions still require the canonical node-generated
artifact at the next integration; this app source is not a root TS gate result.

The first full A3 three-crate strict check on c5666bfa7 stopped at three owned
link issues: two unnecessary type qualifications and a test reading the now
private machine-edit module. The test now uses State::saved_entries, the same
book-read boundary as tick/retry, without widening module visibility. The type
spelling fixes do not alter behavior. The original red receipt is retained; a
changed-source narrow must pass before claiming this wider A3 integration checked.

The second wider strict check (5607ab44d) reached the link test target and found
an overlong combined handshake/admission fixture and three unwraps in helpers
whose module used a combined `cfg(all(test, ...))` condition. The handshake now
has its own fixture function. The search test module uses explicit `cfg(test)`
and a separate native-target condition, matching the existing prompt-index test
registration and allowing Clippy to recognize these helpers as test code. No lint
expectation, production behavior or assertion is weakened. Both red receipts
remain evidence; the corrected source still requires its changed-source check.

The third wider strict check (75ea9d765) reached App and exposed an A3 editing
error: removing peer message text from search failures had also removed the
unrelated ActFailed queue-refusal message. That field is restored byte-for-byte
from the announced base. Only search failures use A3's safe typed facts; ordinary
session act failures keep their existing contract. This is an owned correction,
not a base defect, and the red receipt is retained.

The fourth wider strict check (167ae6af6) compiled the App test target and found
only three owned test-style diagnostics: two unchecked Duration divisions and
one missing unit-return semicolon. The deadline test uses checked division with
the same half-deadline intervals, and the callback now has its required
semicolon. Test semantics and deadlines are unchanged. Its receipt remains red;
changed-source strict/tests are still required, after the approved node review
correction qualification.

The announced base10 (3e7820e3) is merged before the next native check. Its
staged A7 resident owner/seat/tests arrive byte-identical to the base, while A3
keeps its existing recursive navigation vote and one window-owned search value.
Only the Architecture append conflicted; both complete texts are preserved.
No staging attribute or shared delivery/fixture exposure is changed. All 57
native Rust delta files pass formatting, and native size/tokens/diff checks
pass. This is source integration evidence; no union compilation/runtime or
navigation authority is inherited from the base's green gate.

## Outline timer resource identity

[Decision Log]
- Purpose: a timer scheduled by a retired/replaced area must never activate the
  current area's hover candidate, even before the resident cutover lands.
- Existing assets and constraint: hover tickets are local counters, starting at
  one in each new OutlineHover. The delayed Shell callback currently reads the
  selected area; two independently created Pointers can therefore share a ticket.
  FindBar already uses local Rc allocation identity for the same ABA boundary.
- Alternatives: a second registry/allocator, a global hover counter, or requiring
  the future resident network/admission API to fix a local timer. Each enlarges
  authority/state beyond the existing Pointer and is unnecessary.
- Choice: reuse the existing local Rc identity pattern, allocated once per
  Pointer and preserved by clear/leave/snapshot moves. A timer checks that same
  identity before touching the selected pointer, then uses its existing ticket.
- Why: the identity is local timer ownership, never connection/follow authority;
  it retains neither the old area nor its input or transport. Shared resident
  callbacks still need the canonical scoped consumer at product cutover.
- Tradeoff and validation: one tiny allocation per Pointer. An actual GPUI test
  schedules two equal-ticket timers on replaced resources at different times;
  the earlier timer must leave the replacement waiting, and only its own later
  timer may activate it. Existing rail resize assertions remain unchanged. This
  source will require execution with the next native checks, after row267 work.

The interrupted base11 merge is completed, followed by announced base13
(2ebeedf9). Registration conflicts retain every incoming History arm/hub and
A3's PromptIndex registration. The complete incoming RPC table and forwardable
fixture remain byte-exact after removing only A3's appended row/item; the own
pending row is last after skills.import_selection. Incoming deny/withholding
cases and both Architecture texts are retained. The generated TypeScript is
exact head output, including inherited generator whitespace; it is not edited
by hand and still needs canonical A3 regeneration at node integration.

Base13's selected-connection and shared actor files are byte-identical to the
announced line. Their exposure arrives from that accepted base; this package
does not independently promote staging attributes or install G4's unlanded
Scenario. The 55 native Rust delta files pass formatting, with native size,
tokens and diff checks passing (719 files/13 existing size entries). Full A3
strict/runtime, scoped delivery/destination proof, captures and final review
remain outstanding; base13's successful gate is not borrowed as their result.

## Resident consumer inventory (still open)

The following are concrete current call sites for the sole W12 cutover. This
inventory describes remaining work, not an implemented authority boundary. Local
controller identities survive moves; network replies additionally need the
original connection, binding and opaque operation ticket. Callbacks must never
fall back to whichever conversation later becomes selected.

| Existing owner path | Current local guard | Required consumer boundary |
| --- | --- | --- |
| find::start_search / find_step | Original FindBar Rc generation | Borrow the original resident for each debounce/slice; stop when that resource is retired or input closes. |
| find/status::find_target | Original target Rc and record address | Fail only that resource's pending reveal; retain its field/query and unrelated owner reveals. |
| prompts::prompt_jump | Original jump Rc and painted target/epoch/address | Reveal the owning turn before row lookup once A7's real API arrives; its timeout belongs to that same resident. |
| outline::outline_effect | Original Pointer Rc plus local hover ticket | The new local guard rejects a replacement Pointer; the scoped consumer still belongs to the canonical resident owner. |
| prompts::prompt_index_answered | Typed target, page/set ticket and epoch | Validate the original weak delivery guard, then settle the canonical Timeline operation under its exact binding before installing metadata. |
| search::message_searched / open_search_hit | One window search, original source/query ticket | Keep search outside residents; validate weak delivery and use the original-context admitted opener before hydration/follow. |
| find/status::find_retry | Retained shown session and input | Convert the existing alias-only Open through the same original-context carrier, with no late alias re-resolution. |
| conversation_deactivated / conversation_outline_changed | Resource-local cancellation primitives | Invoke on the old resource before a pane getter swap; global preference changes must reach the sole owner's retained resources. |

A7's exact entry_at_source membership/user-stream projection and real
reveal_turn implementation are pending checked prerequisites on the announced
base. Entry.through is only a conservative span, not proof that an arbitrary
record contributed to it. The planned borrowed body-presentation helper applies
to displayed/local-find/copy/restore text; metadata and outline previews remain
raw per the reference. No empty helper or second transcript index fills these
missing consumer contracts. Read-only projection replacement remains Hold while
navigation is protected and must not advance its installed revision on refusal.


## Announced base 13.1 and fixture consumption boundary

The app merges the announced `ad14f5f6` base after base13. The incoming staged
window enumeration and segmented-control producer remain on their original
paths and visibility. Shared selected-connection and actor source remains
byte-identical to that base. The merge is source integration only; the wider
A3 union still requires its own strict/runtime/native gate and capture review.

G4's frozen Scenario handoff `51f64421c` has a bounded source/narrow disposition,
but it is not yet an announced receiving line. A3 must consume it through the
lead's line before any positive actor fixture or extension work. Its synchronous
facade borrows the original Link outside a Tokio-entered runtime. Normal close
acknowledges the scenario peer and Devices hub only; A3 must explicitly settle
its own Search and prompt-metadata workers before close, using actual correlated
events and bounded phase waits. No generic worker-drain claim is borrowed from
that fixture. The later two-peer extension remains a separate lead-owned writer
slot and must preserve one State and canonical admissions.


The prepared metadata adapter drops its selected connection before awaiting a
local Busy notice when its own two-scan admission is full. That refusal starts
no request, so retaining the connection while the app queue is full would make
an unrelated notice extend the old transport lifetime. Successful/in-flight
work retains the existing guarded-send policy. This is prepared source only;
actual saturation/retirement and close/drain tests still qualify its adoption.


Navigation/rail component fixtures seed the existing PromptIndex view model
through its typed answered transition, asserting Ready before exercising UI.
They do not fabricate a successful unguarded receive event or a delivery guard.
The separate actual receive rejection fixture still sends a guardless success
and asserts no address installation. Genuine positive metadata receive remains
unverified until canonical protocol assignment reaches the announced app line;
component fixtures are not evidence for that wire or admission boundary. All
existing navigation/rail assertions remain in place.


## Native adapter test lifetime audit

The owned Search and prompt-index adapter tests reuse the Link's existing
memory-only `testing::within` and named `phase` safety helpers. No shared
fixture, dependency or deadline is introduced: the current five-second phase
and 30-second outer bounds remain. A missing local refusal, welcome frame,
no-frame EOF or task join must fail at its named wait instead of hanging the
runner. Existing no-frame, original-target/ticket and retirement assertions are
preserved. This closes the native counterpart found while auditing the root
row267 test correction; native strict/runtime qualification is still pending.
