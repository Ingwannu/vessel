# E2 relay consent: node half

The contract is ADR 0096. This branch extends the existing Relays, Settings,
device invitation route projection, method field-version gate and CLI; no
parallel actor or store. The app half consumes the integrator's gated line.

## Resume point

Contract only, implementation not started. The startup routes/token/TLS remain
local authority. The relay field's number is assigned at merge. The sixteen-route and ten-second change bounds must use the ADR's named
constants in implementation, with barrier regressions and secret-free
projection tests. Contract/source review is not
a runtime, permissions, gate or UI pass.


## Preserved preparation before the next node base

V3 accepted the ADR0096 contract at9021e3b8 (contract only). Private
literal-cfg(test) relay DTO preparation is preserved as WIP before merging
the coordinator's gated node head. It is not exported, registered, compiled
or a working settings field; RELAY_SETTINGS_PROTOCOL remains pending None.
Actual Settings/Relays/CLI/wire integration must remove this staging boundary
and obtain security review and the final root gate. No node process, real
relay, account, file permission or configuration was changed. All subsequent
compilation/gates run through the approved build VM only.


## Implementation preparation: retain admitted mutation ownership

The existing settings lock is currently borrowed by the RPC future while a
spawn_blocking writer may outlive it. Relay consent cannot inherit that
cancellation gap. The implementation will reuse the existing node connection
TaskTracker and the same settings mutation lock: acquire the owned guard
before spawning one tracked mutation, retain it through protected persistence
and application, and return through a bounded waiter. A caller disappearing
or timing out never releases this mutation's capacity while work continues.
No second settings actor, registry or transport is introduced. The node's
shutdown tracker waits for the same owned work.

Configured relay generations will likewise retain their own completion
tracker inside the existing Relays owner; disabling cancels registrations,
not established data connections or independent fleet callbacks. No network
await holds the Relays state mutex. A single owner-derived advertisement
watch controls the default invitation routes, avoiding an Arc cycle through
Devices/service/gateway and preserving explicit tracked callback overrides.
These are preparation decisions within ADR0096, not implemented behavior or
barrier/permission proof. Protocol assignment, real isolated relay fixtures,
old-peer/mixed-request coverage and the root gate remain pending.
