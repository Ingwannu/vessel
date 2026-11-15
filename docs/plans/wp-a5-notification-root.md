# Original notification recipient and root identity

Status: exact993c60b5 Kit all-target strict and the root_owner GPUI test pass.
Bounded V1 source review is still owed. This is not a desktop registry or whole
A5 handoff; the current documentation successor changes no Rust source.

[Decision Log]
- Purpose: notification and desktop host callbacks must not send an old source
  action to a replacement root while an old Shell is still retained elsewhere.
- Existing assets: GPUI's original window handle and weak entity; the kit's
  existing root wrapper owns the component-layer content identity.
- Alternatives: inspect component Root in each area, retain a second window
  list, or assume a live entity still belongs to its original window.
- Choice: reuse the kit root boundary with root_contains_view, checking either
  direct root identity or its existing wrapped content entity identity.
- Why: this adds no window registry, authority, focus selection, external API,
  dependency or view retention. Only the kit knows its own wrapper.
- Impact: callers must enter the original window, check this predicate and then
  separately recheck their original connection/source. False never picks a
  successor. The shared desktop owner still owns any generic host fallback.

The helper's GPUI regression checks direct and wrapped original content against
a different retained entity. The A5 consumer separately tests the registered
platform callback, multiple real windows, close/reopen, root replacement and
connection replacement; those checks are not implied by this helper test.

The initial exactccb1be75 Kit all-target strict run failed only on the helper's
manual Result-to-Option predicate lint. It is corrected to Result::is_ok_and;
no runtime/ownership behavior changes. The sequential wrapper stopped before
the test. The failed evidence remains recorded; corrected normal-lane checks
and V1 source review are still required before adoption.

Actual exact-source evidence:

- wp-inbox-adapter-20261115T164833Z-993c60b5: Kit all-target strict rc0,33s.
- wp-inbox-tests-20261115T171102Z-993c60b5: actual Kit compilation and one
  root_owner GPUI case1/0/0,339s build plus the test.

The corrected checks ran sequentially in the normal short lane, with no second
priority allowance. App consumer platform/window tests are separate evidence
and do not promote an unreviewed host registry or OS delivery.
