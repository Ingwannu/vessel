# The shell's event-dispatch module

[Decision Log]
- Purpose: area event registrations must not push the window root beyond its
  file budget. The parent is exactly500 product lines before a new dispatch arm.
- Constraint: one event dispatcher, unchanged order/body and state access. The
  lead's checked1d1f11df tree has no equivalent split; its method is identical.
- Alternatives: an allowance, removing blank lines, textual include!, or an
  unrelated screen-specific dispatcher. None provides the desired module seam.
- Choice: the existing shell/ folder gains events.rs, registered by mod events.
  The existing inherent method moves unchanged apart from fn to pub(super) fn.
- Why and cost: the child can still access shell-private state; pub(super)
  exposes it only to its original shell module, not another app area. The
  effective boundary stays the same, and tooling sees an ordinary module.

The method body digest, excluding its visibility/signature and trailing file
separator, is69691504dd113a8c852367a28b53538fddba4bf50b2a356974added022737b72
before and after. The original whole method hash is
c07b513d15c24fa4aadf8440363bb7b4565a29c52fd5571a698ab169df5f2e3c.
The explicit visibility exception was authorized because a private method in a
child cannot be called by its parent; pub(super) preserves the old boundary.
No event arm, callback, actor, state field or matching expectation changes.

Order: LinkError move, this dispatch move, then schedule registrations. The
first two are source-only intermediate commits. The native gate checks the
following registration head, with records lowered there. Neither move is
claimed as independently gated, and no known-failing quality gate is run on
these intermediate commits. No source from the other track's options move is
included.
