# SDK stream identity and terminal cause receiving union

## Decision Log

- Purpose: a caller must neither receive metadata for another requested log nor
  infer a normal end from a generic exhausted/fatal stream.
- Existing assets: the SDK already owns subscription metadata, the receiving
  Source state and the resubscribe/catch-up loop. Both corrections extend those
  existing boundaries, without another identity cache, wire type or method.
- Alternatives: separately gate the same SDK twice; infer end from None; or join
  the two independently reviewed corrections and verify their actual interaction.
- Choice: preserve both original commit ranges, merge the announced node head,
  and add one actual framed lag-resubscribe interaction regression as a separate
  receiving commit. The integrator takes the exact qualified union head.
- Why: rejected resubscribe metadata can arrive beside a normal peer End. Only
  the accepted original log's typed end is eligible; identity refusal must retain
  its original fatal error and cannot become a normal-end observation.
- Effects: no protocol, schema, Method, dependency, native transport or app change.
  Ordinary RPC errors remain their original CallError inside StreamError::Call.

Identity source remains the e3ad027f range; its test-only canonical deadline
correction is 19be4df61. Typed terminal cause remains the separate d320924e
range; its test-only canonical deadline correction is a191ddc47. Both are merged
without rebasing. The only merge conflict was test-module registration and the
fake peer's optional typed-end probe alongside its subscription ordinal; both
behaviours remain, with the explicit probe checked before ordinary subscriptions.

The joint framed test receives original records through catch-up, rejects the
actual second Subscribe reply's wrong SessionId, yields UnverifiedIdentity,
retains LogEnd::FatalError through repeated None, and preserves cursor and
subscription count without retry. The original connection still answers its
ordinary session metadata call. It introduces no manufactured EOF or success.

Predecessor exact runs executed identity94 unit +2docs and typed-end93unit
+2docs with zero failures, and their respective strict checks passed. Those
results do not execute the corrected fixtures or this receiving union. Current
changed-source strict, all applicable tests, whole root gate and V3 disposition
are required before handoff. Native follow and expected-end Window work still
wait for the announced node/app landing and their own original delivery fences.
