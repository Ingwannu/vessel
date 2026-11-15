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
