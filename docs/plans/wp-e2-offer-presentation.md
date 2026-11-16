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

Quota-stop checkpoint: corrected88ab has strict0, related64/0/0, wasm0,
VMquality0 and normal App build/export0. The changed-state capture packet has
24 main plus8 phone lower-view originals and1 blank-link prerequisite, all
personally opened. Reviewer/receiving/owning-whole and complete E2 journey
remain unverified; exact runs, safety/cleanup and restart steps are recorded
in wp-e2-resume.md. No new long run or merge starts during this warning.

## Coordinator visual RETURN, applied by the visual lane (2026-11-16)

The coordinator's comparison of the 88ab captures with Paseo's
`hosts/host-confirmation-sheet.tsx` 19-90 returned three items; G4 is paused,
so the visual lane is the temporary writer on package base 14.13.

[Decision Log]
- Purpose: every row of the confirmation card has Paseo's `DetailRow` shape and
  every value can be selected, as Paseo's.
- Before: machine and fingerprint were label + muted code value (`code_row`),
  but each route was one foreground line "TCP (endpoint)"; no value could be
  selected; at 1280 a long endpoint broke after the kind so "TCP" read as a
  label.
- Choice:
  - View: `OverviewText::route_kind` names the kind alone ("TCP", "Relay" /
    "릴레이", "Remote SSH" / "원격 SSH"; `route()` now uses it, unchanged
    output); `OfferRoute` prepares `kind(lang)` and the bare `endpoint()` once
    in place of the joined `words(lang)`.
  - App: machine, fingerprint and each route are
    `settings::selectable_code_row` rows (kit leaf
    backend/m12-settings-selectable-code-row, additive): the label in the
    foreground, the value muted in the code face, selectable through the kit's
    `plain_text`. Route rows are titled by their kind with the endpoint as the
    value (Paseo's `relayLabel` 40-45 over its endpoint).
- Kept (accepted differences): values in the code face, the "Name for this
  machine" field, several routes in place of Paseo's single relay.
- Impact: a long endpoint wraps inside its row (it may break at any character)
  and is never clipped; the card's rows all read the same.

Tests: the view projection test now checks the kinds in both languages and the
bare endpoints, and that no URL canary reaches either; the kit leaf has its own
rail and wrapping tests (`settings::selectable_tests`).
