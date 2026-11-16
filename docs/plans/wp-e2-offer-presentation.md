# E2 safe invitation confirmation presentation

[Decision Log]
- Purpose: a person checks the offered identity and connection endpoints before
  pairing. A URL's user information, path, query and fragment are not endpoint
  facts and must not enter the confirmation or Offer Debug output.
- Existing assets: view::connections::{Endpoint, Route}, the existing Overview
  route wording in both languages, and the shared settings card/rows/dialog.
- Choice: modify and reuse those assets through a pure prepared OfferRoutes
  projection, shared by native and browser decoding. Cache route words at
  decoding, so Render only borrows/clones shared text.
- Alternatives: a second URL parser, raw URL strings joined during Render, or
  another connector/book writer. None is needed for presentation.
- Cost: endpoints with different hidden paths can look identical. Displayed
  identity is an invitation's claim, never proof of an authenticated handshake.

Reference: pinned Paseo host-confirmation-sheet.tsx19–66 (description, identity,
fingerprint, relay endpoint, Cancel/Connect) and70–107 (detail rows and layout).
Vessel keeps the existing invitation/device-credential flow rather than Paseo's
password offer. A Vessel invitation can list several direct/relay routes, so
each safe route is shown separately with its kind, in original preference order,
instead of labelling a joined list of all routes as Relay. Existing local-name
input remains a Vessel adaptation. No new action, permission or effect is added.

E2's ADR0097 secure pair_over issued receipt and typed book upsert/writer are
still missing. This slice does not implement them or change legacy redemption,
credential persistence, retry, confirmation authority or result semantics.
The original decoded offer stays private to the existing Link effect; only its
safe projection is public. No new limits or authority are inferred from this
projection: the existing wire decoder's admission limits remain separate.

Checks: pure native/browser-shared projection tests cover direct/relay ordering,
IPv6 host/port, hidden URL canaries, both languages, empty routes and safe Debug.
Link regression checks the real decoder plus unchanged private decoded routes;
native strict/tests and wasm checks run only through the build VM. No actual
invitation, network, book write or live service is used for these checks.
Changed confirmation drawing still needs final consumer capture comparison;
source/check results do not imply full E2, QR, consent or SDK readiness.

First exact source30670442 strict run failed only clone_on_ref_ptr in the new
row composition. Use explicit Arc::clone; retain that red result. Related
runtime/wasm/quality steps had not run when the suite stopped. The corrected
source needs its own narrow results before any pass or capture claim.
