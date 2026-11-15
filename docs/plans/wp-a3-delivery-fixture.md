# A3 delivery preparation (proposal, no product exposure)

The canonical producer is W12's `dd13ce378..e42a387bf4093f5e09d1ec18ead68ae6673fed2e`.
Its staged delivery lease is already on the lead line. Common product exposure
remains under the lead's one-writer slot; A3 does not remove delivery/export/
mint/queue staging attributes independently.

[Decision Log]
- Purpose: reject a search page that became stale after its worker finished but
  before the UI consumed the queued event, without retaining the old transport.
- Existing assets and constraints: `SelectedConnection` already guards the
  request before polling and after completion. W12's one weak `DeliveryGuard`
  validates the original lease at UI installation. `testing::peer` already uses
  a real NodeClient and framed in-memory RPC. `Link::detached` has no actor or
  admitted connection, so it cannot issue a genuine delivery guard.
- Alternatives: a fake-valid guard constructor, another lease/epoch source,
  changing global HOME for a live Link, or an isolated child Link process. The
  first three violate ownership or test isolation. A child process is possible
  for final capture proof, but is expensive for each small GPUI regression.
- Proposed choice for lead/architecture agreement: one test-support actor fixture
  behind the existing testing boundary, retaining private State and the shared
  framed peer. It accepts ordinary Command values and returns ordinary Event
  values through the actual dispatch/search/queue code. Its script driver can
  observe a framed Call, return a typed protocol Reply/error, retire that actual
  Connected record, or close the peer. Only State admits the welcome identity;
  the app receives the derived machine/context and never a client or allocator.
- Why: the app can hold an actually queued Search event, retire the original
  connection before applying it, and prove that the receiver refuses it even
  before a newer Fleet event arrives. No app-test API manufactures success
  guards, exposes raw State, or changes the production validation path.
- Cost and boundaries: the fixture needs an agreed narrow test-support API and
  module registration. The lead accepted these requirements for the shared
  fixture; that decision is not an implementation or runtime receipt. Current
  detached UI fixtures would either use this actor for reply
  callbacks or explicitly seed the pure view for component-only setup; the
  latter is never queued-delivery proof. Actual app receive proof still requires
  a real actor-issued event and final capture/interaction evidence.

The planned A3 envelopes carry `Option<DeliveryGuard>` plus their exact original
ConnectionContext. None is permitted for correlated local preflight refusal;
native success needs a real guard. The UI validates immediately before applying
the result, with no await. W12's eventual resident cutover additionally checks
the full binding and opaque operation ticket before touching a resident. A3
does not add a registry, follow manager, lease cache or public guard constructor.

Publication of the common API and any fixture exposure belongs to the lead's
agreed writer slot. Until then the current A3 source does not claim the queued
delivery gap or final resident authority is closed.

## Source preparation receipt (not a compile or runtime pass)

The isolated branch
`backend/m12-wp-search-delivery-prep` joins exactly W12 e42 without changing
common staging attributes or testing-module exposure. Owned A3 publishers now
prepare weak-guard envelopes and the shared guarded send. A refused send drops
the selected client before sending an original-ticket local connection refusal.
Native receivers require matching original admission/ticket and validate the
guard immediately before applying data. A global cached row retains only weak
validity, checked again when preparing and activating a destination.

This branch intentionally awaits the lead's common product exposure and G4's
shared scenario, followed by the approved two-peer extension. Positive detached GPUI reply fixtures have only their new fields
completed; None success is refused, so they still need migration to the real
actor before they can pass. No fake validity constructor or acceptance bypass is
added. This branch is not merged to the working A3 main and must not be submitted
for a gate or adopted as working source yet. The canonical context-bound session
opener and resident binding/ticket admission still belong to W12/W2's cutover.

The lead's later fixture ruling assigns the first shared scenario to G4 under
`link::testing::scenario`: one private State and one framed peer, preserving
W2's frozen actor fixture. A3 waits for that checked scenario to land, then
claims an extension-only writer slot for two peers in the same State. Scripts
remain typed framed Call/Reply values, and retirement/close targets the original
peer through canonical admission. No second actor, router or fabricated guard
is permitted. The two-machine A3 cases remain pending that extension; no
production or queued-UI authority follows from this preparation.
