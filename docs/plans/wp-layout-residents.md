# Session residents behind workspace panes

Status: contract-only review passed. Registry implementation and runtime evidence
remain separate work; this approval does not establish two-pane behavior.
It extends the workspace layout contract; it does not add another panel manager.

The staged concrete owner is `app::areas::layout::residents::SessionResidents`,
behind a literal `cfg(test)` module while legacy globals remain the only product
path. Its private map holds each `SessionView` and `ConversationArea` exactly once;
the existing pure `Residents` table is its admission component, not another live
manager. Claim validates capacity before a scoped owner factory, view borrows check
the full current context, hidden stream ingress checks `FollowBinding`, and effect
settlement consumes the owner ticket before mutation. Clean retirement retires
metadata before returning resources to snapshot/cancel/drop. This foundation does
not authorize a transport or implement the production cutover. W1's window-global
search extraction and scoped composer constructors are prerequisites; the legacy
selected-session composer callback must never become a resident input callback.
The claim key itself must match the expected node (workspace form) or session
(older-session bridge) before allocation. This consistency check is additional
to authoritative workspace membership and the live connection lease; the staged
resource map alone grants neither. The final product admission caller must check
the current layout's agent target and authoritative registry membership before
claiming, and must release a claim on guarded retarget/close.
The staged owner retires admission in two phases: `retire_current` first invalidates
the binding/tickets and returns bounded `FollowRetirement` data; a required owner
callback updates matching pending UI to uncertain before replacement follow begins.
The callback precedes generation-allocation failure as well, so exhaustion cannot
leave an obsolete Sending indicator. Notices cannot mint or readmit operations.

## Decision Log

- Purpose and intent: two agent panes must own independent conversation, scroll,
  editor selection, prompt text, requests and async operations. Moving a tab must
  not reset them, and a hidden pane must not claim focus or mutate another agent.
- Existing implementation and constraints: `areas/session.rs:70-110` has one
  `SessionView` and composer field; `areas/conversation/area.rs:71-175` has one
  main `ConversationArea`. `conversation/place.rs:19-53` uses fixed shell accessor
  functions in `Seat`. `link/lib.rs:319-339,361-386` replaces one session follow.
  The composer also has shared catalog data and session-keyed draft/tray maps
  (`areas/composer/mod.rs:40-85`); those are reused, not copied wholesale.
- Alternatives: clone the whole shell per pane (duplicates node/lifecycle state),
  temporarily swap the globally selected session while drawing (callbacks can
  later act on a different session), or one keyed resident registry with explicit
  owner access. The registry is selected; temporary global swapping is rejected.
- Choice: layout owns the sole registry and resource admission/focus/retention.
  Timeline, composer, lifecycle and attachment owners adapt their existing state
  behind the same typed context. No new dependency is required.
- Why: resource identity is independent of presentation; a callback captures its
  resource generation and expected authenticated node instead of consulting the
  selected route when it eventually executes.
- Benefits, costs and effects: independent panes and safe late replies; existing
  direct global accesses need a coordinated cutover, with no second live manager.

## Reference behaviour

Paseo 0.15.4 paths are relative to `packages/app/src/`:

- `screens/workspace/workspace-panel-host.tsx:34-71,102-151` passes explicit tab,
  pane and host identity to retained panel implementations. Only visible/focused
  content is interactive. Stable target/state identity avoids remounts.
- `screens/workspace/use-mounted-tab-set.ts:29-50`: active, modified, then MRU;
  the ordinary pane cap is supplied as three by `workspace-panel-host.tsx:127`.
  Modified tabs are retained even when they exceed that soft cap.
- `screens/workspace/workspace-deck-retention.ts:3-34`: web warm deck limit ten,
  inactive TTL ten minutes; native keeps one mounted workspace. A workspace
  hydration gap is not permission to prune an inactive entry.

Differences: GPUI desktop supports native split panes. A defensive per-window
resident admission limit is additional to Paseo's per-pane soft target. It never
silently evicts protected input or ends a backend session.

## Typed identity and context

One registry belongs to each app window; `RegistryId` fences callbacks across
window recreation. `SessionId` is its index. Every entry additionally binds authenticated
`NodeId` and a monotonic `ResidentEpoch`. A second node claiming an existing id
is a typed `OwnerConflict`, not a replacement. Explicit cross-node handoff must
retire the old epoch first through the lifecycle owner.

Proposed types in `view::residents` (no GPUI or I/O):

- `SessionAddress { node: NodeId, session: SessionId }`.
- `ResidentToken { registry: RegistryId, address: SessionAddress, epoch: ResidentEpoch }`.
- `PanelClaim { workspace: LayoutKey, tab: TabId }`; a move keeps this claim.
- `SessionContext { resident: ResidentToken, claim: PanelClaim, pane: PaneId,
  visibility: Visible | RetainedHidden, focus: Focused | Background }`.
- `ExpectedNode(NodeId)` is required by identity-scoped effects. A host alias is
  only a locator; every adapter checks `NodeClient.node().id` before each read,
  follow or mutation. A changed alias returns `NodeChanged`, never auto-rebinds.
- `view::Machine.node` supplies the authenticated welcome id from the shared
  package base; no separate alias-to-id cache is introduced. Resolving an older
  session route is read-only until the authenticated identity
  is known. The result must match its outstanding request ticket. A successful
  host rename changes the locator, not the expected node or resource epoch.

Workspace membership comes only from the registry owner's typed lookup. The
existing UUID `WorkspaceId` is reused. A missing/old registry may retain the
explicit session-keyed layout bridge; cwd and immutable creator metadata never
supply workspace ownership or root/child close authority.

## One owner, explicit access

`app::residents::SessionResidents` owns `SessionView` and `ConversationArea`
resources per session, plus the per-session editor/composer resource handle.
The shared provider/option catalog remains shared. The existing global message
search controller (`ConversationArea.search`, `search::Searching`) is intentionally
window-scoped and survives session changes; its search/palette owner extracts it
once before ConversationArea becomes resident-owned. It is not instantiated or
cloned per resident; only a search result's navigation captures resident context. Standalone `TerminalId`
resources remain the terminal area's; a SessionId TUI session is a distinct
presentation variant and delegates its grid to that owner.

The public app seam is:

- `claim_session(address, locator, claim, window, cx) -> Result<ResidentToken>`;
  idempotent for the same address/claim; capacity is checked before allocation.
- `context(token, claim, pane) -> Result<SessionContext>` verifies current epoch
  and current layout ownership. It does not infer permission from focus.
- `session_view(context)`, `conversation(context)`, `composer(context)` and
  corresponding scoped mutation closures return typed `Missing`, `StaleEpoch`
  or `OwnerConflict` instead of a default/global resource.
- `render_agent_panel(shell, context, tokens, window, cx)` uses those exact
  resources. A renderer never opens/creates an agent while drawing.
- `set_visibility(claim, Visible | RetainedHidden)`, `focus_claim(claim)` and
  `release_claim(claim, reason)` update the single owner. At most one claim in
  a window has keyboard/composer focus; several panes may be visible.
- `receive_local(token, event)` admits only synchronous owner-local events. Link
  events use `receive(binding, response)` and operation replies also require their
  admitted operation ticket, as specified below. Session id alone is insufficient.

Only the central event dispatcher routes link events. Timeline `Seat` becomes
an explicit keyed context; its read/mutate paths must resolve the same resource.
Origin/side-question subviews derive a typed subview from that context, never a
second session registry. Context is captured in fields, timers and callbacks.
The sole keyboard interceptor uses the registry's focused context.

## Hidden resources, admission and release

Named policy constants are defined once in `view::residents`:

- `PANE_RESIDENT_TARGET = 3`, matching the reference's ordinary MRU target.
- `MAX_RESIDENT_SESSIONS = 64`, an additional admission bound per app window.
- `UNCLAIMED_RESIDENT_TTL = ten minutes`, reusing the reference's inactive TTL.
- `RESIDENT_ATTACH_DEADLINE = thirty seconds`, a finite client attach/read bound.
- `MAX_RESIDENT_OPERATIONS = 64`, an additional bound on simultaneous admitted
  effects per resident. Capacity refusal sends nothing and preserves prior tickets.

The per-pane target is soft: visible, focused or modified resources are retained
first. A hidden retained resident keeps its text, caret/selection, scroll,
find/disclosure state, queued UI decisions and subscriptions; it cannot autofocus,
mark itself read or take keyboard/IME input. The timeline owner keeps its existing
paging/window rules; the registry never clones a transcript. The resident-count
bound is not a claim of a fixed byte memory ceiling.

A clean hidden entry outside the MRU target can dehydrate only after each owner
returns `Ready(snapshot)`. Snapshots are concrete owner types, not Any/JSON maps.
Its tab metadata remains; revealing it reconstructs owner state and resumes the
stream from the last valid sequence, with the existing gap recovery. An unclaimed
clean entry expires at the TTL. Pending sends, uploads, lifecycle operations,
unsaved text, composing IME, active editors and draft permission/question answers
return `Hold(reason)` and block eviction. No protected entry is evicted to make
room: if there is no eligible entry at the hard cap, allocation returns localized
`Capacity` and the current input/focus remains intact.

Closing a tab does not itself kill/archive the backend. It releases a claim only
after its `ClosePlan` is acknowledged; root, child and unknown roles remain
separate. Archive success must be authoritative and not-live before local close.
When the last claim releases, subscriptions can stop; a backend session continues.
Retiring an epoch fences late callbacks before frontend tasks are cancelled.
Host forget/disconnect cancels frontend work and retains protected input; a write
already on the wire is an uncertain outcome, never an automatic retry.

Release notifications (`ResidentDehydrating`, `ResidentReleased`, `FocusChanged`)
carry the token and claim. Composer/attachment owners cancel only their own UI
jobs and preserve or explicitly discard input under their existing policy. Closing
the application releases frontend resources; local-node quit policy stays with
the desktop owner. No registry eviction signals a backend process.

## Link and failure contract

`link::residents` owns one follow handle per resident token, reusing the existing
`NodeClient` and typed session stream. Registration is one command/event arm in
the extracted shared files. No node RPC or protocol number is added.

The resident epoch protects resource lifetime; it is deliberately not the transport
fence. A same-node reconnect retains protected input and the resident epoch while
retiring its old transport admission **before** starting another attach. Layout's
registry owns a monotonic `FollowGeneration` per resident and admits exactly one
`FollowBinding { resident: ResidentToken, follow: FollowGeneration }`. The counter
is checked for exhaustion and is never reset while the resident epoch remains
live. Every explicit reattach, disconnect, cancellation, observed locator identity
change, and replacement of the underlying authenticated connection invalidates
that admission and all outstanding operation admissions. A new attempt gets a
new generation even when the same NodeId reappears (including alias A→B→A).

`link::residents` captures the binding together with the actual authenticated
`NodeClient` connection lease used for that attempt. It never re-resolves the
alias into another client after admission. The lease is a transport instance fence,
not a metadata/identity cache: the shared connection owner retires it before a
reconnect or client replacement and reports that retirement to the dispatcher.
An adapter checks both that its lease is still live and that that client's welcome
NodeId equals `ExpectedNode` before admitting each read, follow or mutation.
Layout checks the current binding before accepting attach metadata, records,
errors or replies; a queued old binding event cannot create or replace admission.
Attach reports its authenticated binding before records on the same bounded app
channel. Reordered or queued old events fail the binding check without changing
loading/error state, timeline, input, focus or lifecycle.

Every operation additionally captures an opaque `OperationTicket` allocated by
its owning adapter and admitted against that exact binding and operation kind.
The owning adapter obtains its ticket from the sole registry admission API, whose
monotonic private ordinal prevents readmission of a consumed ticket. An action
reply must match both the current binding and its outstanding ticket;
completion consumes the ticket once. Timeline/composer/attachments/permissions/
lifecycle owners own their returned tickets; layout owns binding validation and retirement,
and link owns connection-lease validity. No owner may route a network event through
`receive_local` or recreate admission from a received reply. Retiring admission
marks an already-sent mutation uncertain and preserves protected input; a late
success cannot close a tab or clear a draft, and no mutation is retried automatically.
An authoritative subsequent read may resolve that uncertainty under a fresh binding.
Cancelling one resource cannot replace another follow. Backpressure uses the existing bounded
channel and cancellation; there is no unbounded per-session queue or silent event
loss. Attach/request waits are finite; disconnect and retry use the existing gap
recovery. There is no automatic retry of an uncertain mutation.

Failure states are `Resolving`, `Loading`, `Ready`, `Disconnected`, `Capacity`,
`StaleEpoch`, `OwnerConflict`, `NodeChanged`, `Unavailable`, `Closing` and
`Released`. A hidden resource's failure remains on that resource. A lost route or
book alias never authorizes a mutation against a new node under the same name.

## Adapter ownership and migration

- Layout: resource registry, claims, admission, selection/focus and lifetime.
- Timeline: `ConversationArea`, keyed Seat rows, folds/scroll/find and record projection,
  bounded history paging, and its typed dehydration snapshot.
- Permissions/questions: request forms and answer drafts behind the same context,
  typed eviction holds, and identity-scoped answer effects.
- Composer: per-session field/fullscreen/slash/recall and its typed snapshot.
- Attachments: trays, upload/send leases and release notifications per composer.
- Lifecycle: archive/close/reload/deep-link hooks and expected-node checks.

The current globals are replaced through these scoped accessors, not cloned into
another permanent manager. Pure context/retention tests land first. The registry
can be staged test-only until the owners' adapters compile; the product cutover
removes the old global storage and single-follow path together. No runtime flag
can enable both managers. Source-ready contexts and access signatures are shared
before adapter implementation; a whole native gate and captures cover the final
cutover, not merely the staged core.

## Required evidence

Two sessions in two panes: independent text, caret/selection, scroll and disclosures;
background streaming into the correct visible pane; hidden input preserved without
focus/read claims; move/split retains the same resident token; archive one pane
leaves the other unchanged. Adversarial tests cover alias replacement after deep
link resolution, a stale epoch after eviction/recreation, handoff to another node,
same-node reconnect with queued old attach metadata/error/action replies, observed
alias A→B→A, duplicate operation replies, pending upload/send/IME eviction holds,
hard-cap refusal, cancelled attach and stale close acknowledgement. Tests use typed fakes and no real agent or live node.
