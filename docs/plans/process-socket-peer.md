# Connected socket-peer observation

Status: contract proposed; implementation and runtime qualification absent.
ADR 0066 owns the decision. This plan is independent of the already
qualified ProcessLifetime carrier and does not broaden its guarantees.

## Scope and reuse

One new Proc module, socket_peer, observes a connected TCP peer's actual
accepted socket/process evidence. Existing listening/scanner decoding and
the qualified shared proc reader/seeded identity are the only OS sources.
No Child, reaper, provider ledger, workspace registry or browser policy is
implemented here. No public network listener, credential or permission is
changed. The node's D2 lease owns the verified dial and W14's policy proxy
consumes the owned stream; origin metadata is only an origin matcher.

Paseo read: server/local-service-connection.ts4–28 and
server/service-proxy.ts319–400. Its fixed-loopback connection forwards data
without this kernel peer proof. Vessel's stricter accepted-peer boundary
and its reason are stated in ADR 0066. The legacy display scanner is not
authority: it can skip malformed/unreadable data and choose a first owner.

## Producer prerequisites

The assigned head has ProcessId/ProcessLifetime and selected-file
observations, not a qualified seeded ProcessIdentity or connected-peer
reader. Do not call an unmerged source-only API, clone a parser or create a
local workspace generation. Until those producers exist, use test-only
models or typed Unavailable and keep actual Preview/proxy consumers closed.
Canonical workspace start/restart, automation and incarnation are separate
node admission prerequisites. None follows from a PID, live observer or
TCP-health result.

## Implementation order

1. Review this security contract. Fix concrete API/bounds/decoding details
   against the actual shared producer APIs before adding product code.
2. Extract only shared legacy parsing with behavior preserved, if needed;
   strict connected-row handling is its own change and keeps source errors.
3. Build complete bounded peer evidence behind the staged boundary using
   those shared readers and identity checks. Refuse ambiguous/partial facts.
4. Wire the node lease-owned connector: no HTTP/WS/TLS bytes before proof,
   physical-work permits retained through cancellation, active streams
   closed by lease/run revocation. No independent browser reconnect/dial.
5. Narrow changed-crate checks while developing, then the enclosing runtime
   package gate and security review before removing staging or publishing
   consumer APIs. Real platform/loopback tests stay distinct from models.

## Resume point

Only ADR and this plan exist. No socket_peer code or exported API is added,
no dependency changed, no network operation run, no product caller enabled.
The current D2 helper/health preparation has its own immutable narrow run;
its results do not qualify this future socket-peer backend.
