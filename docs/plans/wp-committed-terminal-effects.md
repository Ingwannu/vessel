# Saved-only terminal effects in the canonical preference owner

Status: provisional receiving source/design. E2 field source9d1427e6 and C2
owner sourcecd3e9805 are consumed under their explicit conditional authorization.
The field source was not compiled by its owner. C2's staged callback is not yet
promoted here. No ordered effect, current-context terminal, multiwindow, resident
terminal or UI/settings completion is claimed.

[Decision Log]
- Purpose: lowering retained terminal history irreversibly discards local rows.
  A failed optimistic preference edit cannot recover those rows by rolling back
  a scalar. Apply this effect only after the sole writer reports Saved.
- Existing assets: the one Coordinator/Client, exact receipts, canonical watch,
  typed ScrollbackLines, C2's model/replay setter, and the App preference observer.
- Constraint: watch delivery can coalesce Saved(0), Saved(8), then Failed(other).
  Exact initiating receipts are not a broadcast delivery channel for all windows.
- Alternatives: trim optimistically, add another watcher/writer/history cache,
  or retain an unbounded commit journal. Those lose history on failure, duplicate
  ownership, or retain unbounded state.
- Choice: extend the same Coordinator with weak per-Client effect subscriptions.
  Each slot retains only the minimum and final committed limit since its last
  drain. One short lock orders subscription, publication and drain. File I/O and
  model callbacks run outside it. No second canonical Prefs snapshot is stored.
- Tradeoffs: delivery lock failure refuses effects/admission; effects cannot be
  rolled back. This receiving version targets C2's current legacy owner and must
  not be advertised as an all-resident terminal delivery proof.

## Planned invariants before activating the callback

- Snapshot retains an optional last committed terminal record: validated limit
  plus existing Revision. Readable file load starts with that record; LaunchOnly
  starts with none and the terminal's existing DEFAULT applied value.
- Only successful Saved writes that change this terminal value advance the
  record. Failed and LaunchOnly writes leave it intact. A later failed unrelated
  write cannot hide a successful minimum/final pair.
- The subscription hub holds weak opaque Client identities and fixed-size
  delivery state, never windows, Shells, a second preference store or history.
  Expired subscriptions are pruned. Subscription reads the canonical watch while
  holding the same lock used by publication, so initial delivery cannot miss a
  save. New clients receive only the latest committed value, no old minimum.
- Drain holds that same ordering lock, then returns owned minimum/final data and
  releases it before any model callback. Poison/unavailable delivery must return
  a typed failure and refuse publication/currentness; partial effect publication
  must not become visible to a drain before canonical publication completes.
- The existing App observer drains effects independently of optimistic projection
  and rollback. No write or callback is duplicated. The lifecycle settled fence
  includes pending delivery, in addition to the observed canonical revision.
- Before replacing/opening a terminal owner, drain synchronously into the current
  owner. Reuse that exact returned final applied value for the successor; a
  separate later global read could assign an intervening commit to the wrong
  owner. A save after the drain belongs to the successor's normal next delivery.
- C2 owns in-flight replay, matching-opening validation, stale completion refusal
  and pending-minimum clearing at replacement. This coordinator does not take
  over those rules or promise restoration of discarded history.

Required proof: real file success/failure and launch-only, slow/coalesced two-
Client delivery, late subscription, unavailable delivery refusal, minimum/final
ordering, actual App two-window/model and opening/replay union tests. Remove the
literal terminal settings staging guard only with the real caller and checks.
