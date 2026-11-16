# Existing virtual-list scroll notification

[Decision Log]
- Purpose and intent: C3's bounded body resource cache needs to invalidate its
  old viewport markers on real outer-owner scrolling. Pending/focused resources
  alone do not prevent visible sections from evicting/refetching one another.
- Existing assets and constraints: VirtualList already owns GPUI ListState.
  ListState already has exactly one set_scroll_handler callback. No current
  W13 or B2 writer/reservation exists: both directly confirmed no overlap before
  installation, as the coordinator and native lead required.
- Alternatives: another list/geometry state in the feature (rejected), a vendor
  range correction (outside this leaf), or forwarding the existing range as
  current (incorrect: pinned scroll() constructs it from original scroll_top
  after changing the anchor; this is source evidence, not a runtime failure).
- Choice: one additive on_scroll callback forwarding ONLY handled-input
  notification. Ignore event.visible_range; do not imply movement on a clamped
  input. Existing constructor/install/measure/render paths remain byte-identical.
  No vendor, focus, selection, physics, renderer, dependency or root edit.
- Why: the actual owner can clear presentation markers before prepaint and use
  its actual current clipped section bounds afterward, rather than trusting a
  stale range or maintaining another scroll model. This grants no file/RPC or
  repository authority.
- Cost and invariants: the callback replaces a prior callback on clones of this
  same list, so install it once for its owning instance. GPUI holds its list
  borrow during notification: never query/mutate VirtualList there. Defer such
  owner work. Retained old callbacks still need their feature's instance checks.
  Programmatic owner reveal/resize/scrollbar/cache paths are not claimed as
  handled native-wheel events; the consumer must prove its invalidation paths.

## Verification boundary

Isolated linear leaf on announced package base13 (2ebeedf9). The new real-window
case drives the same native wheel packet with absent/present callbacks, asserts
one notification for the present case, reads actual new list top only AFTER
input dispatch and compares motion with the no-handler path. No latest range
is forwarded, so a stale range cannot be consumed through this API. The fixture
has no node, filesystem observer, credentials or second list state.

All source is currently UNCOMPILED and the test is UNEXECUTED. Standalone pinned
rustfmt, size/token and source-preflight checks precede the remote Kit strict
and actual window-test checks. V1 then the lead review this isolated leaf.
This is not C3 body/cache fairness, all Kit, full native, UI/capture/OS approval.
No change to another caller is included; actual C3 visibility-marker protection
and >32-visible-section/cached scroll tests remain separate consumer work.
