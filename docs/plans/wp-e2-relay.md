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
