# Session lifecycle in the native app

Status: implementation in progress.

## Reference and existing assets

Paseo 0.15.4: `packages/app/src/screens/sessions-screen.tsx:24-160`
keeps search separate from transcript search, filters hosts and reports partial-host failures.
`packages/app/src/subagents/select.ts:77-104` selects unarchived managed children
across hosts and sorts them oldest first. `subagents/close-tab-policy.ts:5-13`
archives roots on close but removes only layout for children.
`packages/protocol/src/agent-deep-link.ts:28-73` routes an explicit host and session;
credentials, query and fragment are rejected by the desktop link parser.
`packages/app/src/hooks/use-archive-agent.ts` owns archive cache rollback;
`subagents/use-detach-subagent.ts` performs explicit detach.

Existing assets: `view::board::Fleet::tree`, `Card::from_session`,
`link::session::Act`, board and conversation confirms, `Shell::open_session`,
mark/resume/delete calls and title folding are reused. The archive is a board dialog,
not a chronological sessions screen. The sidebar tree caller belongs to its existing
owner. Import/history discovery and transcript search retain their separate owners.

## Decision Log

- Purpose and intent: expose durable sessions, lifecycle operations and parent/child
  navigation consistently without coupling screens to network code.
- Existing constraints: app and node bases are converging; labels and reload are
  staged. A creator is immutable provenance, so detach must not rewrite its history.
- Alternatives: overload archive as history; introduce a second lifecycle command
  dispatcher; filter child identity by mutable title; external UI/data dependency.
- Choice: modify existing lifecycle actions and projections, add isolated `sessions`
  modules for chronological history, navigation and metadata. Reuse existing kit and
  standard collections; introduce no dependency. Workspace archive and restore build on the shared workspace identity contract.
  Managed child detach belongs to its separate subagent package.
- Why: existing confirmation, queue rejection and typed node errors remain the one
  path; pure selection rules can be checked without GPUI or real providers.
- Advantages, costs and effects: small registration seams; independent history UI
  and cross-host addresses; explicit compatibility gates while node features stage.

## Differences from Paseo

- Link scheme and entity names use Vessel node/session identifiers.
- Existing node lifecycle semantics (ADR 0063) govern archive/resume/reload; no app
  request bypasses an unsupported protocol.
- Provider-native subagent tracks require a separate node contract; this package
  concerns managed sessions with a durable creator relationship.

## Structure and data flow

`view/sessions` owns chronological filtering, paging and lifecycle policy.
`link/sessions` resolves node identities and fetches missing sessions with deadlines. A coalesced watch
snapshot retains one destination through reconnect under one deadline; newer
requests abort older metadata reads. An offline alias is refused because it
cannot safely identify a replacement host.
`app/areas/sessions` draws history and metadata using the kit and typed locale API.
The shared session menu delegates lifecycle actions and navigation into this area.
The existing fleet supplies current titles, marks and statuses. No provider process
is started merely to list or inspect history. Existing Tokio test utilities are
enabled only for deterministic virtual-time reconnect/timeout tests; no runtime
dependency is introduced.

## Verification

Narrow crate tests/clippy through the shared build slot per commit. One whole native
gate and fake-fleet capture matrix at package end; root gate for the node package.
No live preview, real agent invocation, push or production mutation is authorized.

## Current implementation limits

History searches title and branch metadata through the shared matcher with
opt-in per-token typo tolerance, then retains activity chronology and grows the
visible window fifty rows at a time. Default command/catalog matching is unchanged.
The canonical registry must still contribute workspace/project display names; host
labels, provider names, directory paths and transcripts are not surrogate fields.
Paseo performs this search on the host with server cursors. The current node lists
complete rows, so local paging is a compatibility path; bounded host history
querying remains unfinished and is not claimed as cursor parity.
The current host failures are explicit; offline hosts do not contribute invented rows.
Fresh conversation refuses before archiving when a separate new-chat draft already
exists or when a named profile cannot be carried by the current draft contract.
This preserves unsent work and launch identity until the shared tab/profile
contracts provide an exact destination. Archived sessions hide their composer
from the fleet snapshot before the session metadata request has completed.

## Resume point

Done: reference/asset survey; chronological history, host selector and date/page
models; correlated navigation and lifecycle shapes; initial narrow view check
passed 20 tests including six package regressions. Fast size/boundary/token checks
pass. Session identity, link stream forwarding and shell event dispatch were
mechanically extracted to avoid increasing shared file budgets.
In progress: native session confirmations and success-only close/fresh navigation;
framed fake-peer link tests and narrow clippy. The first link clippy exposed helper
test-cfg lint handling and a dispatcher function at its length boundary; repairs
retain the original results and do not weaken the lint configuration.
Ownership: final single-owner table assigns A2 here; subagents/detach, labels
and OS deep-link delivery remain their respective packages. The separate node
worktree stages ownership-fenced workspace archive/recovery behind a typed fake;
its reviewed registry/store dependency is not yet consumer-ready.
Next: narrow checks, current-parent relation and pane close integration, registry
adapter plus pending workspace calls, then package capture/gate evidence.


## History matcher decision

Existing `view::matcher` already implements the reference exact/subsequence tiers
and UTF-16 ranking with UTF-8 marks. Modify it with an additive opt-in constructor
and isolated bounded-edit module instead of copying the matcher or adding a
third-party fuzzy library. Budgets follow `protocol/src/search/text-match.ts`: no
typos through three UTF-16 units, adjacent swaps only at four, one edit through
seven, two above seven. ASCII words and leading slices are the typo candidates;
whole-word highlights identify a typo match. Existing picker defaults and tests
remain the compatibility contract.


## Authenticated passive access

The lookup result retains a `sessions::Binding` (validated host name plus exact
node/session address). After W10 reserved `conversation_opened` for this seam,
its metadata/conversation reads and the log follow use `SessionAccess` instead
of alias-only commands. The link actor validates the live `NodeClient` identity
and captures it synchronously before dispatch; alias replacement refuses without
contacting the replacement. Lifecycle requests retain the same optional expected
node, preventing an archive/delete on an alias that changed after lookup. The shared `Machine.node` welcome identity now hydrates ordinary saved-host
opens too; already-bound deep links retain their original expected identity.
Legacy records without a hydrated identity keep their existing passive path. Queue/web refusals settle the exact pending read/stream.

Evidence so far: the standalone staged node workflow's first 11 tests pass
(in-memory ports only), and native view's full 870 tests pass. Node concurrent
archive exclusion and native scoped-read/follow changes are newer and still
require their exact-head checks. No full gate, capture or runtime parity claim.


Composer `/exit` and `/clear` execute immediately, as
`client-slash-commands/index.ts:17-36` and `panels/agent-panel.tsx:1579-1610`
specify; they do not reuse the menu's Archive/Delete confirmation dialog.
Vessel deliberately retains the visible session until authoritative archive
acknowledgement instead of optimistically closing/retargeting before the RPC.
Busy/refused/uncertain outcomes keep the session available with an inline error.


Lifecycle acknowledgements recheck the target's expected NodeId against the
current shared Machine.node before updating fleet metadata or granting close.
A replaced/disconnected destination keeps the result uncertain in the UI and
cannot receive the old session projection. This complements the pre-send link
identity guard; the resident cutover still needs the shared connection lease and
FollowBinding so same-node transport retirement also fences stale replies.


## Snapshot publication ordering

History availability uses the same HostScope predicate as rows and per-host
errors. If all selected hosts fail, an empty projection says "Unable to load
sessions" rather than claiming the directory is empty or the search had no
matches. The existing Refresh action retries that scope. A successful selected
host keeps its rows and ordinary empty/search state with the other host's error
banner; a book with no hosts is still an empty directory, not a failed read.

Existing periodic fleet reads can overlap an asynchronous archive/delete result.
Reusing their arrival order alone would let a response built before archive undo
the acknowledged row. `link::sessions::Projection` therefore stores one checked
local revision, advanced after session mutations (including uncertain results).
A fleet read captures it before querying. The final check and synchronous bounded
channel enqueue share the revision write lock, after channel capacity has been
reserved; there is no await under the lock. This closes the check/enqueue race
without an unbounded snapshot queue. Superseded reads are retried by the existing
periodic tick. Passive reads never advance it; exhaustion permanently disables
publication until link restart instead of recycling a revision. The counter is
projection ordering, not a second session/connection identity registry.

On package base 6 the shared connection owner retains `reconcile_connections`,
`read_sessions` and authenticated node/connection-epoch projection. This package
wraps that unchanged refresh path with the projection capture/publication check;
it does not reinstate the older `read_machine` implementation or its connection
failure policy. Reload retires G4 admissions before notifying the existing
navigation readiness watch, and refresh notifies it after reconciliation and
session reads. Test actors use the shared `testing::state` and G4 admission
constructor. Deterministic tests cover a full UI channel during archive
acknowledgement so a queued stale snapshot cannot follow it.

This union does not make the legacy session carriers connection-epoch safe.
Queued Records/SessionLost/Acted still need the canonical resident FollowBinding
and selected connection lease at both boundaries. The sole resident owner's
single-slot contract amendment must be approved before that coordinated product
conversion; no second epoch registry or global fallback is introduced here.


Base4 terminal integration preserves C2's terminal_follow, focus and render
presentation hooks in the extracted session opener. Conversation hydration takes
&Open and does not ask a terminal for Conversation. Access::ReadTerminal carries
link::TerminalOpening (not a fabricated view type), verifies its machine/session
against the retained Binding, and preserves its opaque receipt on every refusal
or log result. C2 owns propagation of retained node/connection authority at its
terminal entry; a missing binding must not trigger alias fallback.


The interim Creator-based close heuristic was removed: creator is provenance,
not current attachment. Until reviewed W2 Parentage and W12 role/context adapters
are connected, legacy Close reports unavailable and sends no archive or local
hide. Explicit Archive and slash /exit retain their explicit user intent. This
is unfinished close parity, not a claim that an older node proves a root.

Lifecycle confirmation uses the shared kit's captured dismissal and weak return
focus. Its separate `Lifecycle` ticket sequence identifies UI decisions only;
network operations keep their existing independent tickets. Escape, scrim, close,
Cancel and Confirm retain the rendered confirmation ticket. A stale callback
cannot cancel or confirm a later decision, even on the same session. Actual
accepted dismissal calls `DialogState::dismissed`; navigation invalidates the
pending decision without recycling its ticket. The renderer remains the one kit
dialog. This addresses delayed UI events without a second modal or identity map.

The legacy selected-session mutation bridge requires a retained Open.binding.
An unbound target cannot borrow the latest Fleet NodeId to authorize archive,
delete, resume or a slash-command close. It shows a localized reopen requirement
before any confirmation/request is issued. W3's creation receipt and W12's
resident context will supply the original admitted address for new/resident
sessions; until cutover a legacy unbound open must be reopened to admit it.

W3 creation receipts enter through `Shell::started_at(card, SessionAddress, cx)`.
It verifies card/session agreement and validates the saved host name before
clearing draft sending state, then installs the receipt's original binding before
conversation hydration and follow. It does not infer the node from current Fleet.
The legacy `started` path retains its previous unbound behavior until W3 cutover;
its later lifecycle actions require reopening. The shared completion body moves
to the existing session opening sidecar, lowering session.rs's measured ratchet
to 641. This is the legacy receipt adapter, not a resident or connection manager.

Navigation cancellation defers the accepted dialog-close acknowledgement until
the next window-dependent settlement, or before another confirmation opens.
This clears the old weak opener cycle without moving focus away from a newer
field. A regression switches opener fields between cancelled and new decisions;
stale dismissal preserves the new dialog, and its own dismissal returns only to
the new opener. Deferred UI cleanup is distinct from a network mutation ticket.

History row reads and archive/unarchive callbacks retain the node identity from
the rendered Fleet snapshot. Mutation controls are unavailable without that
identity, and callbacks use the existing correlated lifecycle/confirmation path
rather than an alias-only board action. The confirmation's live warning reads
its exact target; a failure on the Sessions route remains visible in the window
notice. The selected session is not used to decide the history row's target.

Copy link uses the retained admitted address directly, via the shared desktop
NavigationTarget URL formatter, including while offline. It does not resolve a
saved alias again or supersede an in-flight session-open ticket. An unbound or
internally mismatched binding refuses without changing the clipboard; the
read-only address accessor checks binding host/session agreement. Legacy resolved
copy responses share the same URL/clipboard helper. A fake-shell regression
keeps the original node despite a replaced Fleet destination and preserves the
pending open and clipboard on a later invalid binding.

## Shared resident source adoption

W12's initial producer1720 was held before original-tree adoption. The isolated
held branch is retained only as work evidence. The successor adopted here is the
single native producer9480d4d0 with its explicitly reviewed value prerequisite
e46cd57b (including canonical WorkspaceId identity), on the current shared app
line. It is separate from plugin runtime or a node-head merge into the app.
`view::sessions::SessionAddress` re-exports canonical
`view::residents::SessionAddress`; navigation, receipt bindings and resident
contexts therefore share one type without changing existing import paths.
The producer includes a metadata admission/fence table and pane registry, with
no live per-session resource getters or follow cutover. A2 retains no second
resource manager and does not substitute selected-global state for scoped access.
The producer's reviewed/clippy evidence does not replace this consumer's union
checks or the final package/runtime gate.

A mutation with no readable node error reply is LostReply, carrying its original
ActFailed diagnostic. It is distinct from an explicit wire refusal and from the
overall deadline. The UI preserves the session, explains the uncertain outcome
in both languages and retains the detail, while refresh invalidates stale Fleet
snapshots. An in-memory framed peer drops the transport after receiving the
mutation; the regression verifies one call, no replay and projection invalidation.

History rows without a rendered authenticated node do not install an open
callback; they cannot ask a later Fleet snapshot to bless an originally unbound
alias. This is only the legacy address pin. Canonical resident/lease admission
must still precede resource switching in the coordinated product cutover.
# Canonical staged lifecycle foundation

[Decision Log]
- Purpose: keep one lifecycle owner type while the full A2 area joins W12's
  canonical resident bundle.
- Existing constraint: the isolated producer uses flat compatibility namespaces;
  full A2 already owns directory modules with the same names. Keeping both files
  makes Rust module resolution ambiguous and retaining the old owner duplicates
  its binding/readiness policy.
- Choice: preserve the full directory modules, alias their staged resident path
  to `areas::session_lifecycle`, and re-export the same local action/UI-ticket
  values from `view::session_lifecycle`. Remove the old concrete owner copy.
- Why: the reviewed producer remains the sole type; existing page and link
  imports keep their names without introducing another map or authority source.
- Effects: the legacy UI-only counter now uses the canonical ticket's checked
  FIRST/next API. It still cannot mint a resident operation ticket. The staged
  module remains `cfg(test)`, with no bundle insertion or product promotion.

The isolated foundation's bounded V4 source adoption is `7e710a189`; its complete
tree equals the exact `04287520` strict and five-GPUI-test checkpoint. The full
A2 union additionally merges announced native base `3e7820e3` and needs its own
changed-source checks. That earlier evidence is not this union's final gate.
The generic acknowledgement path must not service prearmed stops: the actual
stop adapter still requires W12's checked kernel, acknowledged-held owner state,
original receipt/guard/context and exact originating Window commit.
