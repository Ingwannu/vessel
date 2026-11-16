# Minimal staged lifecycle resource producer

Status: source preparation on announced app base `a7aab9d6`; not consumer-ready.
The full A2 source/test snapshot `6859d9314` remains unchanged and separate.

## Decision Log

- Purpose: let the sole W12 resident bundle own concrete lifecycle holds without
  pulling in the Sessions screen, legacy opener or prototype link dispatcher.
- Existing assets: canonical W12 `SessionResources`, `SessionAdmission`, bindings,
  tickets and scoped receive/settle; the A2 snapshot and state implementation.
- Constraint: the earlier adapter depended on page-local view/link types. A
  producer cannot require an unannounced whole feature branch to compile.
- Alternatives: a second resource map, a legacy-global fallback or a duplicate
  ticket registry. These cannot preserve the canonical resource and admission.
- Choice: one test-only `areas::session_lifecycle` owner module, local pure view
  intent/UI-ticket values and compatibility re-exports of those same types.
  Existing standard/Tokio/GPUI assets suffice; no library or boundary is added.
- Effects: W12 alone adds `SessionResources.lifecycle` and its factory/vote/
  retirement calls after checked adoption. This packet adds no bundle field,
  screen/route, RPC, pending row, actor request, global swap or product promotion.

## Module and data flow

`view::session_lifecycle` defines only local UI `LifecycleAction` and
`LifecycleTicket`. The latter cannot substitute for canonical `OperationTicket`.
`view::sessions` re-exports the same values; there is no second definition.

`areas::session_lifecycle` owns Snapshot, LifecycleResources, loading/decision/
revoked/pending/uncertain phases and local Disposition. The old
`areas::sessions::resident` path re-exports the same module, without registering
or implementing the Sessions page. Both area modules remain literal `cfg(test)`.

Only original delivery validity followed by canonical receive may call
`Snapshot::received(resources, admission)`. It joins the original connection,
context resident, binding/address and actual open/shown/metadata IDs. It never
infers origin from global Open or a legacy binding, and never refreshes visibility
merely to install hidden fresh metadata.

Snapshot retains the exact FollowBinding, not just its stable resident token.
prepare must match that binding. Full retirement of a Ready snapshot's binding
changes its own hold to Loading while retaining cached launch values; guarded
metadata on the new binding is required before a new lifecycle intent. Dismissing
a retired decision likewise returns to Loading. A stale retirement notice cannot
invalidate a newly hydrated successor. Sending/Uncertain input retention is
unchanged. This is full-retirement freshness, not the expected-end sealing policy;
the latter and ACK-held Window work remain a separate coordinated amendment.

Failure Disposition is an actor-correlated local fact. NoSend means definite
pre-poll/pre-effect refusal; late errors, lost replies and timeouts remain
Uncertain. This module cannot establish that fact from an RPC error. The actor
adapter is outside this packet. Only canonical exact-ticket settle may invoke
failed/acknowledged; a local outcome never grants mutation, close or retry rights.

## Body-equivalence qualifications and evidence

Snapshot receive/readiness bodies and the intent/UI-ticket values are copied
from immutable `6859d9314`. Core phase/decision/retirement/acknowledgement logic is
retained. Failure input deliberately changes from the prototype Link Failure to
local NoSend/Uncertain; no actor mapping is included or claimed equivalent.
The four GPUI component tests use the same real staged W12 minter and retained
fields; the failure test supplies local dispositions instead of a fake peer error.
These are domain/hold component checks, not transport/queued-delivery, keyboard,
IME or whole user-flow evidence. The isolated SessionView constructor is only a
fixture; a real context-scoped composer factory remains required.

The old A2 strict run `wp-sessions-narrow-20261115T162219Z-6859d931` passed all-target
View/Link/App Clippy. Its requested actual test run remains separate. Neither is
proof for this newly isolated producer: exact changed-source strict checks and
actual component tests are required before a bounded adoption packet. No whole
native gate, inspected capture, V4/V1 adoption or live lifecycle authority is
claimed by this source preparation.
