# Per-turn file checkpoints: contract-first plan

Status: ADR 0077 design contract accepted, 2026-11-15, on `a852bde3`.
The node implementation remains under validation. App product work waits for
the reviewed canonical node contract on the app line and the agreed live keyed
timeline context/registration seam. Ownership is assigned; no checkpoint UI has
been implemented or validated.

## Reference and existing assets

The exact source mapping, alternatives, scope and stated differences are in
[ADR 0077](../adr/0077-per-turn-file-checkpoints.md). Paseo's files mode and
Claude SDK rewindFiles define files-only behavior; GOAT supplies the surveyed
turn-associated presentation. This is not conversation rewind. The existing
removed-worktree snapshots restore into a fresh worktree, not into a turn.

Decision: modify existing session/workspace infrastructure, with new focused
checkpoint modules. Reuse RelPath, hardened Git enumeration, regular-file
checks, bounded channels and existing hashing/storage crates. Do not introduce
a provider SDK or infer snapshots from tool-call counts. Native link session
and workspace asks already carry conversation rewind and snapshot list/drop;
reuse their typed dispatch pattern only when the new node contract exists.
Kit dialog, alert and toast already exist; no new general control is needed.

## Sequence after contract acceptance

1. Central owners supply the updated app baseline. Do not independently merge
   node history into this package or invent a protocol/schema number.
2. Implement/review bounded node capture, private persistence and lifecycle
   handshake with fakes; no advertised ready checkpoint until durable publish.
3. Implement/review authenticated preview, restore journal/status and protocol
   adaptation with crash/race/permission tests. A failed restore is not success.
4. Only on the reviewed node contract: pure native checkpoint state and tests,
   typed link commands, turn marks, complete paginated confirmation, and result
   presentation. Shared shell/link roots receive registration lines only.
5. Follow the current package gate/capture procedure once at package end;
   report real tested behavior, explicit scope/platform limits, and unknowns.

## Resume point

Completed: reference/source research and accepted ADR 0077 design contract
(`a852bde3`). Preview/journal/verification share the frozen effective
permission-policy image. The actor's capture wait is bounded without claiming
blocked I/O is cancellable; timed-out workers retain their bounded slot and
reservation until cleanup. Expired generations cannot publish ready checkpoints.
Non-root metadata and held-I/O barrier tests remain required evidence for later
implementation, not results of this document.

No product edits, merge, runtime test, fake fleet, capture or compiled gate.
Next: central owners assign baseline, implementation, schema and protocol
integration. Contract acceptance does not assign that work or prove runtime
capability; no package UI or node implementation starts before that sequence.

## Current producer/consumer boundary

The app tree has consumed package base 8e (82ca07a7) through its authorized line merge,
including the fixed native lock and the shared layout/delivery seams.
The node producer remains on its separate checkpoint branch. Its latest source
includes physical admission for ordinary writers and retained startup/lifecycle
operations, actor pause and turn hooks, and pending checkpoint metadata. These
are source changes under validation, not a reviewed runtime capability. The
FILE_CHECKPOINTS protocol requirement remains None and no checkpoint UI has been
implemented or captured in this app tree.

The app will consume the reviewed node contract through the package line, using
Machine.node and the shared pane/dialog/preflight APIs already on this base.
Remaining app work is per-turn marks, all-page changed-file confirmation, typed
busy/stale/limit outcomes, verified result presentation and durable reconnect
status. No local Rust compilation is used; the base's native gate is baseline
evidence only and does not validate this unfinished checkpoint package.


### Package line fixed lock update

Consumed dd13ce37 from the authorized app line after confirming no native gate
was running. It includes G2's reviewed trigger/menu changes and the fixed native
Cargo.lock; --locked is now usable for future app checks. No node worktree head
was merged into this tree and no checkpoint UI has been implemented yet. Node
contract/runtime review and a consumer-ready producer remain prerequisites.


### App preparation on package base 8 (no feature implementation)

Consumed 91407173 through app-line merge da48212c. No running app whole gate
was frozen, no node worktree head or unreviewed checkpoint values carrier was
merged, and no Cargo command ran locally. This base merge is not checkpoint
validation. Node contract/runtime review and the producer sequence remain owed.

Existing integration points inspected: view/timeline/{entry,rewind}.rs keeps
stable EventSeq/EntryKey identities and conversation rewind independently;
app/areas/conversation/rewind.rs owns the current message action renderer and
kit menu/tooltip integration. The new feature will reuse those kit controls
and the shared dialog focus lease. It must not send the existing Rewind action,
replace composer text, hide history, or infer a file checkpoint from provider
message ids/tool counts. Node TurnAssociated metadata ties PromptAccepted's
exact sequence to the actual TurnStarted; AfterTurn names its exact ended event.
Timeline registration remains coordinated with W10 before shared product edits.

The intended vertical slice is view/checkpoints pure enum state, link/checkpoints
correlated typed calls, app/areas/checkpoints drawing/actions, and EN/KO strings
in L's existing text format. Loading, unavailable, paging, confirmable, restoring,
awaiting reconnect status, verified result and typed refusal are explicit states.
The dialog is confirmable only after every frozen changes page is present for
one plan/expiry. A new selected host/session, stale ticket, changed node identity
or disconnected/expired plan cannot apply an old response to the current panel.
RestoreId survives response loss and reconnect; retries query durable status
before another action. Existing queue, conversation and composer stay untouched.
Runtime Busy/FeatureUnavailable remains visible until reviewed node activation;
private fixtures or baseline native gates cannot establish a working app feature.


### App line 8e preparation (no feature implementation)

Consumed announced package base 8e (82ca07a7) through app-line merge ff2f20cb.
No app whole gate was running. Shared owner-retirement and picker admission
seams remain the canonical controls. This app tree takes no node branch or
unreviewed values carrier; checkpoint UI remains held for the reviewed producer
contract. The node producer separately consumes announced node9c150c6e and is
under narrow validation. Base gates do not validate this package's future UI.

### UI admission and retry details before implementation

Base 8e's kit DialogContent/DialogState and captured dismissal/return-focus path
remain the sole renderer. The checkpoint area will use that existing dialog for
all frozen changed-file pages and a footer action bound to one exact plan. Layout
retirement hooks run synchronously to invalidate tickets; they must not issue RPC
or reenter layout. W10's timeline registration and W2's existing conversation
rewind renderer remain owned integration points, not files to rewrite locally.

Files eligibility is independent of the current rewind_point/conversation ability.
A provider unable to rewind conversation can still have a Ready file checkpoint;
a provider able to rewind conversation can still have no usable file image.
TurnAssociated and the exact BeforeTurn/AfterTurn boundary supply association.
The files action must not inherit the current conversation-mode eligibility
check, synthesize an association from a row index, or send Act::Rewind.

After response loss, restore_status can truthfully remain InProgress when files
have succeeded but the result marker still needs publication. Status is read-only.
The app must retain the original RestoreId and plan, and retry checkpoint.restore
with that same identity to finish metadata; it must not create another operation
or get stuck querying status forever. Existing-operation admission precedes plan
expiry and never reapplies files. Stale node/session/panel tickets still refuse
late replies, while a detached operation's durable identity remains available for
reconnect result lookup. These are future implementation/test obligations.

### Timeline owner agreement (contract preparation only)

W2 and W10 confirm that the current reviewed line has no generic turn-action
registration API. The legacy rewind::with_row/menu remains conversation-only.
W10 proposes one caller-owned optional element slot in A7 timeline::entry_element's
user/turn wrapper, with the completed footer for AfterTurn. The checkpoint area
supplies its independent button, disabled reason, dialog and action.

No stable signature or consumer-ready A7 hash is promised: full A7 review and the
product scoped SessionContext cutover remain pending. W9 must first send the
checked node producer's exact boundary/ref/phase contract. Registration is then
one agreed owner change, not a second menu or renderer. Dispatch captures the
canonical accepted prompt/turn/checkpoint references and full original context,
then resolves EntryKey/source before any numeric index. No route/context type is
invented locally, and shared timeline/rewind files remain untouched meanwhile.

### Package base13 and scoped checkpoint ownership (preparation only)

The app consumes announced2ebeedf9 through app-only merge22f2e6df. The canonical
view/residents SessionContext carries registry/resource incarnation, layout claim,
pane, visibility and focus. A7's ResidentSeat is explicitly staged: the current
product timeline still renders through legacy Seat function pointers. The new
checkpoint action cannot claim this cutover happened just because the types exist.

W12 owns OperationKind/ResourceOwner admission. A checkpoint operation needs its
own correlated pending state and confirmation/reconnect holds; it cannot settle a
timeline read, fork or conversation rewind ticket by borrowing a generic kind.
The owner is asked to identify the canonical namespace before adoption. No local
context/route/registry, selected-session fallback, public signature or registration
line is invented. W10's optional entry/footer element remains the owned hook, after
the checked node boundary/reference contract and product scoped cutover exist.
The node is still under independent validation; no unreviewed values producer or
node branch is merged into the app. There is no checkpoint UI or capture proof yet.
