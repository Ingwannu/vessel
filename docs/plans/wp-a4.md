# A4: own history, children and fork/rewind parity

PLAN section7 assigns W2 the complete package. WP-A2 documents the import
source reads, reuse analysis, native module flow and deliberate differences.
ADR0090 in the separate node tree owns subagent parentage/provider projection.

## Responsibilities and data flow

- view::history owns scope/ticket/node identity, cursor pages, row selection,
  import and existing-session reactivation receipts, safe typed failures.
- link::history adapts existing agent.history/profile.list/session.create resume
  and session.get/session.resume; it retains its authenticated client across
  navigation and never retries a possibly committed mutation automatically.
- app::areas::import composes shared kit controls and local Fluent words.
- Planned view/link/app subagents modules consume the reviewed node contract;
  managed SessionRef identities remain separate from provider-native child ids.
- The node owns durable detach authorization, commit and publication. A local UI
  hide never grants or revokes authority. Provider projection reads actual
  observations from the canonical durable parent log, not a second transport.

## Fork and rewind audit

Paseo parts/A-lifecycle29-34 and37 and parts/C-timeline20/gaps Subagent track are
the specification. The existing native conversation menu supports ADR0052's
provider-native fork at completed turn points and conversation rewind. Its
origin projection displays inherited history from the original session.

Paseo's fork instead serializes recent chat under a100000-character summary cap,
wraps `<chat-history-summary>` and opens a configurable draft, allowing a
provider/model change. This remains an explicit gap until the common draft seed
path is wired. W3 owns view::creation::DraftSeed and Shell::seed_draft, W12 owns
workspace draft identity, and W6 owns composer::context::ChatHistory(Payload).
W2 owns transcript selection/truncation and wrapper production; it will reuse
those reviewed typed APIs rather than create another draft/context store.

Files/both rewind is W9's checkpoint contract; W2 retains the current
conversation rewind while checking its capability/menu registration against
that owner. Provider-child rows never acquire managed detach/archive/stop rights.

## Evidence boundaries

The skeleton exists for own-history import/activation; changed source has fast
format/size/boundary checks. Narrow checks and child integration are ongoing.
No full A4, provider extraction, UI capture or end-to-end parity claim is made.
One complete package native gate/capture review follows implementation, with
node security deltas separately reviewed under A14.

## Text-fork producer checkpoint

view::history::fork_summary prepares an ephemeral snapshot from canonical
projected timeline rows. Full SessionRef+entry key+revision identifies a selected
boundary, including inherited rows from different sources. It refuses missing
or changed checkpoints and unavailable history, and discloses retained-only
history separately from budget omissions. It owns no draft/context persistence;
conversion to W6 ChatHistory/ContextText and W3 DraftSeed is still required.

Source read: server/agent/activity-curator.ts:308-375 and tests520-550. The full
wrapper measures100000 UTF-16 units, metadata1000, tool summary200. Recent whole
messages win; a single oversized last message retains beginning/end with an
explicit middle omission. Rust preserves Unicode scalars at boundaries rather
than slicing half a surrogate. User/assistant/tool summaries are included;
thoughts/debug logs/raw external tool input are excluded, as in Paseo's fork.
The current native ToolView title supplies a tool's readable summary; detailed
W10/W23 tool presentation can enrich this once their common type is integrated.
Tests cover the full wrapper, Unicode, recent selection, near-budget last
message retention, source identity/revision, metadata bounds and excluded raw
input. Source/fixtures are staged pending narrow execution; no fork UI claim.

## Shared managed-child projection

Existing view/board/tree.rs was audited first: it keys only SessionId, derives
parentage from immutable Creator and sorts children newest-first. It cannot be
the current relationship source after detach and conflates equal session UUIDs
on different nodes. The new view::subagents::ManagedTree<W> prepares one index
for session/legacy-tree order, the child track and session command-center reveal. W preserves W11's
actual WorkspaceId at callers; it adds no workspace registry or cwd/title key.
Existing board Row/origin compatibility will delegate once node hydration is
integrated; no second lasting tree source is intended.

Snapshots require explicit Known(current parent) or Unknown hydration. Detached
and original roots both resolve Known(None); unsupported/old/conflicting data
remains Unknown and cannot authorize archive/hide. Full SessionRef deduplicates
book aliases. Conflicting snapshots fail; cycles become Unknown/Partial while
remaining visible. Active children are oldest-first and exclude archive intent.
Same-workspace children nest; cross-workspace children are normal rows in their
own workspace. rows and reveal use the same index, so session keyboard/tree jumps
expand the same ancestor chain. Labels/status remain canonical card projections.

Preparation caps16384 snapshot records and64 visible ancestry levels as named
UI resource bounds; over-limit input refuses rather than dropping rows, and
invalid/deeper relationships flatten with Unknown/Partial confidence. Source
completeness is explicit. This pure view seam is staged; authoritative link
hydration, root/provider actions and native track controls are still owed.

Fresh S2 source correction: sidebar-workspace-row.tsx and sidebar-shortcuts.ts
operate on visible workspace placements, with at most9 numeric workspace
targets. Managed child/session rows never enter that numeric projection. S2/W11
own that single workspace source; this index serves only A4/session-tree and
legacy session callers. No workspace-sidebar caller code was added here.

## Exact fork overlap boundary

W10's isolated Entry.through bridge20ae7edac (base4 parent, same code as consumer-tested9fbda65af) supplies the exclusive last contributing/invalidating source record watermark. The fork checkpoint captures it alongside source/key/revision and refuses any earlier same-source row whose current watermark crossed the selected checkpoint. This catches late tool updates even when the selected answer itself did not change. It never infers a historical body from revision counts or a contiguous seq range. Unanchored current snapshots remain allowed. Exact merged view/link checks remain owed.

Plugin history uses the same agent.history/session.create resume boundary. W16's plugin-provider registration is not executable/consumer-ready yet; plugin history is unsupported until W22's provider adapter exists, rather than a second intake/store.

## Subagent native skeleton and staged boundary

The app tree now merges its own node A4 pending source (af15194f) for combined package compilation, not as a reviewed consumer-ready release. This makes real typed inputs/outputs available without a duplicate UI id or imaginary RPC. Security core corrections and finite input budgets have scoped V3 passes; observations/collection enforcement/projector and final package checks remain pending. Native view::subagents::Track owns full-node managed rows, correlated provider reads, unknown coverage, bounded local provider hide and detach confirmations. Provider child navigation uses ChildPane with the existing canonical Timeline on sparse parent sequences. The pane contains no control/intake command. A2 owns the canonical finished eligibility predicate and conditional archive operation; the track receives that eligibility and never approximates it from a display phrase.

W11 owns background/workspace membership and W12 pane targets; no workspace numeric target is synthesized here. Managed and provider rows remain different identities. Old/unassigned relationship protocol is typed Unsupported rather than immutable Creator inference. The read-only provider wrapper is ignored by the main conversation/timeline projection; its child page alone unwraps canonical events.

## First native subagent frame

The track/header, adaptive kit dialogs, managed child navigation/current-parent lookup, explicit detach confirmation and uncertain-result refresh boundary, provider local hide and correlated canonical child paging are now wired to the pending typed methods. Current parentage is accepted only from a capable authenticated node; a reused book alias refuses before read/mutation. The main timeline ignores ProviderChild wrappers. ChildPane validates requested cursor windows, immutable overlaps and aggregate retained count/bytes before publishing pages. Renderer state is an embedded ConversationArea with shown/info unset, using the existing rows/Seat API without a fabricated managed SessionId.

Remaining before package acceptance: W12/W10 keyed read-only subview replaces the interim function-pointer Seat before final review; current generation checks protect asynchronous answers but old painted cosmetic toggle closures are not yet keyed. A2 canonical finished eligibility/conditional archive and W11 background/workspace projection remain dependencies; managed eligibility stays explicitly unknown, and the UI discloses partial hydration. The current bulk action hides terminal provider rows only until A2 is integrated. Nested provider navigation and the detailed status-count pill still require filling. No real provider observations have been emitted yet: driver extraction/projector execution and the full package gates/captures/review remain owed. This commit is a frame, not completed parity.

## Shared projection and context intake

ChildPane now transfers an accepted Timeline exactly once with take_projection,
retaining its bounded immutable source records for the next prepend. The app
installs this projection into its existing read-only ConversationArea rather than
folding the same records a second time. Preserving disclosure/bookmark UI state
across replacement still awaits W10's shared A7 projection installation and W12's
keyed Seat; no incompatible transitional renderer is introduced here.

W6's isolated checked composer-context values20ea88e2 are reused. ForkSummary
adapts into the canonical ChatHistory/Payload with validated ContextText, preserving
the complete bounded wrapper and no fetched URL. Caller-supplied validated id/title
stay typed; there is no second context store. W3's Busy-protected actual draft seed
and W12's existing-workspace DraftId integration remain required before wiring the
user action. A top-level workspace opener is never substituted for an existing
workspace draft target.

## Nested provider navigation

Root/provider child rows and breadcrumb paths now use the same bounded descriptor
projection. A known child can open its own canonical read-only pane and direct
children; Back follows its actual observed provider parent before returning to the
managed parent's track. An omitted ancestor stays an orphan root row with its real
parent metadata preserved. No managed SessionId is fabricated. Row and breadcrumb
callbacks capture the active full parent/child source before navigation/hide, so
painted callbacks from another nested view refuse instead of mutating that view.

## Navigation and projection generations

Track refresh and child page reads draw tickets from the SubagentArea owner's
navigation-lifetime counter, not a counter reset by each new Track/ChildPane.
This prevents an A→B→A or same-child reopen from admitting the first old reply.
An accepted rebuilt child projection exposes projection_revision (the accepted
read ticket), so the keyed renderer can invalidate old index callbacks even after
a same-query prepend. Query identity alone cannot prove an old numeric row index.
Tests cover delayed same-child reopen and revision change on earlier-page rebuild.

## Current parentage in fleet cards

CardDetail now carries advisory Relation, default Unknown for old/demo/cache
records. Only from_session_at_protocol on the authenticated answering client
projects current parentage behind SUBAGENT_RELATIONS. Old/pending peers never
become roots from missing detach receipt or immutable Creator. Creator remains
an audit field. Lifecycle consumers must still hydrate current authority before
mutation. A changed capable fleet relation clears stale parent navigation and
requests the same authoritative lookup rather than keeping a detached child's
Back link indefinitely. No workspace identity or visibility is inferred here.

## Mixed fan-out status pill

Paseo track-presentation.ts79–147 reports each active state separately, failed
before working, then falls back to the total once no active state remains.
Track::pill_counts uses the same visible root rows, explicit failed session/provider
facts and active managed turns/provider running reports. Unknown/idle/completed
states gain no invented active mark. Prepared localized segments pair their own
kit status dot and count; a mixed fan-out is never collapsed into an urgent color
that would imply all work stopped. Labels use whole Fluent messages in EN/KO.

## Shared physical connection admission

Subagent Target/Host now carry the existing ConnectionContext from Machine's
single authenticated snapshot, alongside full managed address. State validates
that context and uses the common SelectedConnection::call pre-poll and post-answer
retirement fence. No raw client, alias-by-node reconnect, extra allocator or
feature-specific connection registry is introduced. G4 delegated one tiny protocol
metadata getter; node identity remains Context.node. Metadata is never authority.
Same-node reconnect invalidates target equality and old drawn row actions. Retired
writes are conservatively uncertain because an already sent detach can complete;
there is no automatic replay. Node capability remains separately required.

A shared eight-call native bound (matching Devices) retains permits through bounded
UI publication. Rejected admission/full capacity uses actor backpressure before
spawning, so rapid navigation cannot create an unbounded refusal task queue.
The actor regression replaces the same-node admission then proves the old A4
request refuses before any framed RPC, without a real node/credential or browser.

## First executed summary/view regression

Remote exact71a7 view tests executed997 passed/1 failed: reserving the omission
notice could pop the only selected recent message. The correction preserves the
newest message by truncating its middle after dropping whole older messages, then
reserves the same complete marker/wrapper budget. The failing regression stays
unchanged; no broader test pass is claimed until the corrected immutable run.
Strict frame a78 separately returned unused-result/arithmetic findings, corrected
with explicit outcomes, bounded count iteration and panic-free access; no lint
allowance or weakened assertions were added.

## Workspace-relative root policy

Paseo workspace-root-policy.ts6–16 treats an attached child in another known
workspace as a root tab in that workspace. Relation::workspace_role joins current
parentage with W11 registry ownership and returns the existing W12 AgentRole,
rather than defining another role enum. No-current-parent is Root; attached with
both known equal workspaces is Child, known unequal is Root; missing ownership or
unknown relation remains Unknown. W11 supplies both memberships by full SessionRef
and owns newest-root activity selection. This advisory policy does not change the
node's global delegated boundary or authorize cached lifecycle mutations.

## Conditional archive-finished progress skeleton

The pure ArchiveFinished controller snapshots canonical A2-supplied finished
eligibility, hides terminal provider rows locally first, and emits one captured
managed FinishRead at a time. It rechecks the latest track before each dispatch,
rejects old-ticket/changed-admission replies, retains typed failure attribution,
and retries only failures that remain eligible; successful steps are not repeated.
Archived and conditional Skipped both finish a progress step. The app keeps A2's
canonical receipt; this view state defines no wire method or replacement outcome
DTO. Node parent/idle/archive recheck remains A2's atomic lock/actor boundary.

The controller and in-memory transition tests are staged independently while
W21's actual session.archive_child/predicate source awaits checks/V3 and the app
base route. It is not yet activated by the product Archive finished action; that
still performs provider-local hide only. Wire adapter/progress/retry UI remain
required before this row can be accepted as full parity.

Parent-resident union uses the existing SubagentArea once in SessionResources.subagents; readonly_for accessors reject mismatched canonical child identities and projection revisions before exposing the shared ConversationArea. Full SessionContext admission remains the resident owner’s prerequisite. The current function-pointer caller is transitional until the W12/A7 keyed Seat union; this accessor alone does not prove stale callback rejection.

## Original admission for history (2026-11-15)

[Decision Log]
- Purpose: a reconnect of the same node must not receive an import that was chosen
  under a retired authenticated admission.
- Existing assets: G4's ConnectionContext and SelectedConnection already own exact
  alias/epoch/node checks before polling and after completion. A separate raw-client
  history task bypassed those generation fences despite checking the node UUID.
- Choice: Scope carries that original shared context; actor admission and every
  list/profile/get/create/resume call reuse SelectedConnection. Eight permits bound
  history tasks before spawn; a full budget settles as unsent. Publication has a
  named five-second budget. Imports remain sequential, without automatic replay.
- Tradeoff: retirement after a write may have reached the node is uncertain;
  receipts never cause a replacement create, and opening a successful receipt also
  requires the exact still-current admission. Existing pure model and framed-peer
  regressions are retained, with explicit same-node replacement and budget tests.
- Validation: native strict and framed-peer execution are remote-only and pending
  for this changed head; the earlier raw-client adapter pass is not extended here.
