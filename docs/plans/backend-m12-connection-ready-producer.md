# Generic connection-ready producer on the package base

This isolated producer is based on8d5513e4. It extracts no state and changes no
kept-list, connection or Fleet behaviour: the existing hand_kept call happens
first, then an empty typed array of area Event notices. The refresh call retains
its parsed book key. Consumers add their own typed notice in this one fan-out;
there is no shared generation counter or second connection hook. Same-node fast
reconnect is a new connection and reaches the hook even for an equal Fleet.

[Decision Log]
- Purpose: diagnostics and schedules share the actual reconnect moment.
- Constraint: consuming an unrelated menu or screen branch is unnecessary.
- Alternatives: per-area hooks or cache-difference inference.
- Choice: one NodeName/NodeId input and a bounded typed Event list, empty here.
- Why and cost: no new root registration per area; consumers own their payload
  and invalidate their own outstanding tickets. Removal/rename/offline evidence
  still needs its existing explicit invalidation and is not invented here.

This is a source-only dependency producer. Fast source checks do not prove a
native gate or reconnect consumer behaviour. Consumers add their notice and
regression tests and include this exact change in their final package checks.
