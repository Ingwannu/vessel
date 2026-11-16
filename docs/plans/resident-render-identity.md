# Resident identity at the rendering edge

## Decision Log

- Purpose: scope mounted row/pacing state by the original resource incarnation,
  so equal alias/session/event ordinals cannot reuse another resource's state.
- Constraint: the canonical registry and epoch deliberately expose no raw scalar
  getters. A renderer must not copy an allocator, serialize private fields, use
  Debug output or substitute a hash-only identity with collision risk.
- Alternatives: new registry/epoch getters would expose separate representation
  details; a renderer-local counter would duplicate ownership; equality inside
  retained pacing state alone cannot separate simultaneous mounted resources.
- Choice: `ResidentToken` implements lossless Display inside its existing owner.
  It formats registry UUID, node UUID, session UUID and the full-width resident
  epoch, in a fixed tagged slash-delimited form. UUID and unsigned decimal
  representations cannot introduce that delimiter, so components stay distinct.
- Why: consumers need one typed rendering identity, not four new constructors or
  a second lifetime mechanism. All private fields remain private and unchanged.
- Consequences: the app caches the key once when it captures a Seat and appends
  its existing claim/origin presentation path where necessary. Display grants no
  resource access, network address, binding, ticket, lease or input authority.
  Admission continues to compare the canonical typed token/context itself.

This leaf changes no allocator, equality, wire/serde contract, persistence or
table. The two deterministic pure cases cover all rendered components, separate
registry/node/session/epoch keys and the maximum-width epoch. They are authored
but not executed until the receiving View checks. Actual GPUI structural pacing,
simultaneous panes, stale callbacks and snapshot identity remain consumer proof.
