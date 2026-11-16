# Assigned workspace metadata wire validation

Status: authored tests on assigned protocol 25; changed-head execution and review
remain owed. This supplements the registry assignment rather than enabling any
provisioning, generation, archive, config, editor or process effect.

[Decision Log]
- Purpose: a feature version pin alone cannot prove old-hop refusal, domain CAS
  or person authority on a running node.
- Existing constraints: use the real typed methods, recording RPC fake, memory
  transport and isolated node fixture. No feature override or new protocol row.
- Alternatives: treating generic transport fakes as authority proof, or deriving
  identity/backing from a cwd. Neither supplies the required fact.
- Choice: independent transport admission and real-node domain tests. Offline
  Store seeds are metadata only, closed before Node startup; same-path records
  remain independent and nonexistent backing paths are never provisioned.
- Impact: direct/self/nested old-hop SDK requests must send no registry frame;
  raw old-hop calls must fail before the recording fake dispatches, with GetNode
  as a supported ordering control. Real owner calls exercise all six methods,
  active/foreground filtering, exact identity, manual reset, pin/unpin and
  source/revision release CAS. Original accepted whole failures remain history;
  the announced assignment head has its own green combined gate.

Additional authored cases use actual paired Read/Operate/Admin credentials,
local fake-agent credentials and the existing real fleet fixtures. Direct and
relayed people exercise metadata without replacing origin or intersected grants;
Read-only target grants refuse writes, Read-only origin refuses Operate, and
Operate-only origin cannot pass the independent Read envelope. Local/delegated
Agent calls use existing/absent IDs and must all return Forbidden. Delegated
unchanged registry reads must return within the fixture phase bound rather than
retaining the ordinary long-poll. Current raw unknown request params reach the
recording handler; this proves forward decoding separately from domain authority.

All authored cases still require exact changed-head execution. The real fleet
fixture's existing join wait is reused without introducing new clock polling.
Runtime outcome, every listed supported-wire cell and immutable gate review must
be recorded before full matrix readiness is claimed. These tests exercise fake
harnesses only and never execute a real provider, editor or provisioning effect.

The first actual matrix run passed the typed old-hop no-frame case and three
real direct Owner/paired/Agent cases, then stopped on its own fleet fixture:
four simultaneously retained relayed device first hops exhausted the relay's
existing per-node/address admission share during another pairing. That red is
retained; the raw RPC cases later in Cargo's order did not execute.

The fixture now keeps one explicitly relayed paired-device positive path and
uses the existing actual direct-pairing helper for the three independent scope
refusals. Their delegated target hops still use the real fleet/relay route.
No protocol, limits, grants, authority assertions, sleeps or product behavior
change. This checks authority across actual paired transports while respecting
the existing relay admission budget. Corrected exact-head execution is owed.

The corrected relay fixture then executed all seven cases successfully: typed
SDK no-frame (one), real Node authority (four), and raw pre-dispatch/future-field
compatibility (two). This closes that fixture failure only; review separately
returned the test lifetime boundary, and that evidence remains predecessor proof.

The lifetime correction uses the canonical prebuilt `within` five-second memory
case budget and named one-second Hello/Welcome, request/response, dispatch and
server/peer close waits. An owned task handle remains outside the timed,
asserting future; on failure it is still joined, with abort-and-join only as a
bounded cleanup fallback. The original failure is rethrown after cleanup rather
than being interpreted as success. Real Store seeds and node/fleet ownership
steps reuse the existing named twenty-second e2e step inside the unchanged
sixty-second safety net. Seed writers close before assertion rethrow; node and
relay handles remain outside authority assertions and drain on that failure path.
No product timeout, grant, quota, assertion or policy is relaxed. Exact corrected
strict and functional checks remain owed.
