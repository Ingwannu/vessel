# First product session resident: one admitted slot

Status: contract amendment passed review. The staged concrete resource map and
its admission minter are not a live product path.

## Decision Log

- Purpose and intent: fence queued records, metadata and failures before the
  broader multi-pane resource conversion finishes. SessionId or alias+SessionId
  alone cannot distinguish an old connection's queued response from its successor.
- Existing implementation and constraints: the old session/conversation globals
  and single follow have unbound ingress. The reviewed resident design already
  supplies one admission table, typed claims and attempt/ticket fencing. Owner
  adapters and scoped field constructors still require coordinated conversion.
- Alternatives: leave unsafe ingress until every multi-pane feature is complete;
  introduce another legacy admission/cache; or start the same resident owner with
  one resource slot and grow its storage in place. Another manager is rejected.
- Choice: the first product step of the same owner has one admitted resource slot.
  It uses the same canonical `Residents` table and concrete `SessionResources`.
  Converted paths lose their old global storage/access and raw follow in the
  same change. Scoped access refuses an absent or different active resource.
- Why: the smaller first conversion can establish identity and attempt fencing
  without claiming simultaneous panes or keeping an unsafe fallback alive.
- Benefits, costs and effects: old queued events cannot mutate a successor. Until
  storage grows, switching away from protected input can refuse with a localized
  Hold. That temporary limitation is explicit; protected input is never dropped.

## One owner and storage

`SessionResidents` remains the sole window owner. Its admission table is neither
copied nor recreated during navigation. A slot holds the exact resident token,
layout claim, concrete SessionView/ConversationArea/composer resources, original
connection context and current FollowBinding. There is no parallel alias/node
cache, alternate RegistryId or separately minted generation.

The first product storage has capacity for one live resource bundle. Existing
staged map code is not enabled alongside it. Later multi-slot storage grows this
same owner in place; it preserves its table, epochs, tickets and retained entities.
There is no runtime switch between old and new managers and no shell clone/swap.

Old `Areas.session`/`Areas.conversation` storage and direct ingress are removed
for every converted path in the same change. Missing or different scoped access
returns a typed refusal, never the selected global resource. A genuinely pending
creation remains the creation owner's typed draft until its real receipt arrives;
no invented SessionId or resident is allocated for it.

## Admission before opening

1. The caller captures the original shared ConnectionContext and expected
   SessionAddress at the interaction/creation receipt. It must not derive the
   expected node from a later Fleet lookup or another alias.
2. The layout owner obtains the real LayoutKey and TabId from the single layout
   book, checks the current Agent target and expected node/session consistency,
   and uses authoritative workspace membership when opening a workspace key.
   An explicit older-session key remains a compatibility key, never workspace
   ownership. Loading/unavailable registry data cannot become guessed ownership.
3. The sole owner claims the concrete slot and produces SessionContext before
   binding input callbacks or installing hydration state. Admission refusal alone
   leaves the old slot, its input, focus and still-valid operation admissions
   intact. It cannot defer an independent transport retirement described below.
4. The owner retires the previous attach admission and notifies concrete owners
   of their retired operation tickets before minting/launching the next follow.
   A fresh FollowBinding exists before the link sees an attach request.
5. The link uses G4's original selected connection lease. It checks actual welcome
   node and admitted connection generation before effects and after awaits before
   exposing metadata, records, errors or EOF. It never exports a raw client.

Rendering only borrows this already admitted resource; it never creates/open an
agent or begins a follow. The single-slot phase does not advertise another live
agent pane when that resource cannot be resolved.

## Holds and transition

The existing hold contract is unchanged: nonempty text including whitespace,
caret/selection/undo resources in use, IME, request forms, overlays, pending sends,
uploads, confirmations and uncertain wire mutations prevent destructive release.
All concrete owners must transfer typed clean snapshots before destructive
resident/resource retirement or dehydration, not before transport retirement.
Otherwise switching refuses; a persisted draft alone does not make live input
safe to discard. A failed admission must not clear the old composer or its holds.

Observed disconnect, attempt cancellation, locator change or actual lease
replacement invalidates the old FollowBinding and operation tickets immediately,
even if replacement admission refuses or dirty/IME/pending/uncertain holds keep
the original resources. Clean snapshot/Ready is never a precondition for this
transport retirement. Already-sent writes become Uncertain and input remains.

Epoch retirement precedes frontend cancellation/resource release. Same-node
reattach keeps the resident/input epoch but retires its old FollowBinding and
single-use operation admissions before launching another attach. An already-sent
operation becomes uncertain at its owner, retaining its input/form; a stale reply
cannot leave a successful-close authority or clear a later draft. No mutation is
automatically retried. The owner receives typed original binding/kind/ticket
retirement information so an obsolete Sending indicator can become Uncertain.

Field callbacks retain the same token+claim and can refresh that exact local
presentation before checking current visible/focused input eligibility. Network
replies never take this local refresh path and never create admission from data.

## Carrier conversion and scope

The session lifecycle owner adapts its open, metadata and mutation carriers on
top of these owner APIs. The layout owner remains the only admission minter.
Every converted stream event carries original connection context and binding;
operation completions additionally carry their owner kind and single-use ticket.
Unbound Records/SessionLost/Acted routes cannot remain as a fallback for an
admitted resource. The central dispatcher rejects old carriers before mutation.

The old single follow is removed for converted paths with their new guarded
follow carrier, not left running beside it. Independent terminal resources keep
their terminal owner's lifecycle; no terminal kill/archive is inferred here.

## Required evidence before product adoption

- Original same-node connection followed again: queued old attach metadata,
  record, failure and action completion cannot mutate the new admission.
- Alias A-to-B-to-A, expected node mismatch and reused SessionId: reject old or
  conflicting authority before opening, input mutation or network effects.
- Duplicate completion cannot consume another operation or clear later input.
- Refused single-slot switch preserves dirty text, pending/uncertain forms and
  actual field entity. IME and overlay holds cannot be bypassed by dehydration.
- Refused switch combined with retirement of the old lease still rejects queued
  old records, metadata and action completions; protected input cannot mutate or
  clear and already-sent effects become uncertain despite retaining the slot.
- Clean switch retires admission before cancellation and rejects old callbacks;
  no converted global storage, raw follow or global accessor fallback remains.
- Fresh hidden records reach only their admitted resource; hidden content cannot
  mark itself read, receive keyboard input or take focus.
- Ordinary source/narrow checks cover the entire converted product union. Staged
  getters or synthetic guards alone do not prove live stream or UI behavior.

This first step does not establish multi-session panes, final retention parity,
OS behavior or whole-package approval. Those remain the final resource/layout
cutover's checks and captures.

## Staged admission implementation

The first minter extends the same `SessionResidents` owner. `SessionOpening`
resolves original `ConnectionContext` plus session through `WorkspaceLookup`;
loading/unavailable ownership refuses instead of falling back to a guessed key.
Admission rechecks current authoritative membership before touching the book or
resources, and refuses changed placement instead of retargeting a deferred intent.
`Shell::admit_session_open` plans a real Agent tab in the existing layout book,
claims resources before invoking the supplied scoped factory, retires the old
attempt through concrete owner callbacks and returns `SessionAdmission` containing
original connection, full presentation context and fresh binding. The book is
published only after admission succeeds. No hydration, follow or operation is
started by this minter.

The initial single-slot policy is conservative: another retained resident refuses
even when clean. An explicit all-owner snapshot/close must release it first. This
avoids an implicit destructive transition before every concrete adapter is ready.
A refusal leaves the previous layout, fields and still-valid tickets intact.
Independent `connection_retired` processing matches only the stored original
context, invalidates old binding/tickets and notifies owners despite a refused
switch; a delayed old notice cannot retire a newer admitted connection.
Only actual shared lease retirement takes this context-scoped path. A follow-only
EOF/error/cancel retires its exact `FollowBinding`; it must not retire a newer
follow sharing the still-active physical connection.

These methods remain inside the literal test-only resource module. Product
conversion still owes removal of every converted global/raw ingress, actual
scoped owner factories, guarded transport and final receive checks. Existing
legacy constructors used in minter tests are fixtures, not approved factories.
