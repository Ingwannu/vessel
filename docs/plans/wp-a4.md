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
