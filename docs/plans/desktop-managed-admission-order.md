# Managed-local transition order (implementation reservation, no source PASS)

The lead's third-slot hold still applies: W2 real A4 and W12 actual product
promotion must both land before connections source is edited. Staged delivery
range dd13ce37..e42a387b alone does not unlock this work. The accepted contract
remains desktop-managed-connections.md (SHA9de1420); this note records its actor
transition mechanics, not another registry or permission source.

## Source and receipt

- Controller Connection carries an immutable transport lease and provenance.
  Status/log notifications reuse it; successful physical replacement mints a
  new lease. Source drop/original physical closure irreversibly retires it.
- Registration retains that transport recipe plus the actor's LATEST optional
  ConnectionContext receipt and UI ticket. It never caches Connected/validity.
- Connected alone owns W12's canonical SelectedLease anchor. Managed binding
  retains provenance/source correlation and a bounded observer cancellation
  handle, not another client or admission allocator.
- Qualification after reading e42: SelectedLease has a receiver, NO revoker.
  The earlier proposed weak-lease revoke call does not exist and will not be
  implemented by retaining another validity Sender/client. Instead the SAME
  SelectedLease gets a private optional controller-source receiver (Book=None).
  Synchronous validate and the existing pre/post-await and bounded-send races
  fail on source retirement OR sender loss, even before actor cleanup delivery.
  DeliveryGuard delegates that same predicate; no parallel guard is introduced.
- The observer only queues the exact original source + context for actor cleanup;
  it owns no selected lease, client or validity Sender. Retiring Connected cancels
  its old observer before another admission is published. W12/G4 gave direction
  only; V1/lead confirmation and the fixed third slot remain prerequisites.

## Transition table

| Input | Required order | Result |
| --- | --- | --- |
| Same active source notification | Check current exact receipt; keep it | Same context and guard, no churn |
| Distinct source while previous source is active | Do not mutate either admission | LocalOccupied |
| Actual replaced/withdrawn source | Retire old validity BEFORE book/discovery awaits | Old guard immediately fails, even if new admission fails |
| Controller withdrawal of current source | Match opaque source, use actor's LATEST receipt, retire/remove | Reload cannot strand removal on the old receipt |
| Late admission observer report | Require exact source AND captured context | Cannot retire a newer same-source reload or successor |
| Stale old-source withdrawal | Source does not match current recipe | Ignore; no successor retirement |
| Reload | Retire old map/watch, clear receipt, then re-admit still-active recipe | Fresh canonical epoch stored BEFORE publication |
| External book collision | Retire local before book dial/publication | Book wins, visible BookCollision, no shadow |
| Refused/failed ordinary book edit | Preserve both admissions and original book fences | No local retirement |
| Source loss while book read is held | Canonical source criterion refuses immediately; recheck after await | No new admission/publication for retired source |

A current source may retry admission after an external collision is removed,
through the existing actor refresh/reload path, only while that actual controller
source remains active. A withdrawn source is removed and cannot auto-re-admit.
No second poller, alias resolver, counter or pending-operation map is added.

## Publication and queued delivery

Use the existing read_sessions reducer for the local Machine, including semantic
session-Forbidden retaining usable device RPC. Explicit local cap is one, in
addition to the existing book cap. Machine provenance defaults to Unknown for
legacy snapshots; namespace grammar is never provenance or book edit authority.

Mint the SAME W12 DeliveryGuard from the selected canonical admission, once per
request; queue only its weak clone with Resolve/Bind/local-target answers. A
preflight refusal has no guard. The UI validates exact original context, Visit
and operation ticket immediately before installation, without an await. A stored
Ready binding must also refuse a subsequently retired guard. No local bypass.

Book writer reservation uses E2's sole proto::book_values::ManagedLocalAlias.
Existing explicit Book entries may be removed/renamed away/no-op renamed; only
new generated destinations are reserved. Held blocking I/O, caller cancellation
and actual worker completion remain ordered by E2's existing owner/receipt.

## Required source-lifetime barriers

Direct cases cover source withdrawal after an accepted queue send but BEFORE
App installation; in-flight Unary retirement; body-ready completion racing source
loss; source sender dropped while its last value remains Active; canonical owner
drop with old work still alive; bounded-send backpressure retirement; and stale
observer cleanup versus a successor. Unchanged Book=None cases remain required.
These are planned tests, not current execution or product-promotion evidence.
