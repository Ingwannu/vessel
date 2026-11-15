# Exclusive source bounds for projected timeline entries

This isolated prerequisite is based on package base4 `3cd0d3fc`. It contains
only `Entry.through`, its fold updates, fixture fields and the source-bound
regressions extracted from A7 `9fbda65af`; it carries no A7 screen stack.

[Decision Log]
- Purpose: draft forks must reject a projected prefix if an earlier row contains
  an update after the selected anchor's source boundary.
- Constraint: `Entry.seq` names the first record, while `rev` counts mutations.
  Gaps and late updates prevent deriving a last source cursor from either.
- Alternatives: reconstruct a separate projection or guess from revision count.
- Choice: additive public `Entry.through: EventSeq`, an exclusive watermark set
  on push/change from the current fold's already advanced record cursor.
- Why: the single projection knows the exact record at each row mutation.
- Effects: consumers conservatively reject `entry.through > selected_through`.
  This is not a contiguous contributing-record range or historical snapshot;
  invalidation may advance the bound without changing the body. Replay overlap
  does not advance it. No node call, protocol field or delivery behavior changes.

Validation: the original A7 source at `9fbda65af` passed 1,059 view all-target
tests, including these exact late-tool and merged-text/gap regressions, in
`wp-timeline-models-20261115T084837Z-9fbda65a`. This isolated base4 extraction
has diff/size checks only; W2 will compile its exact integration before consuming
the bridge. This is neither a whole-native gate nor V4 package approval.
