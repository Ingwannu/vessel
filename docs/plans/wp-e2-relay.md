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


## Pending wire skeleton (not a runtime handoff)

The private test-only DTO boundary is now removed in favor of the requested
existing settings API fields. `NodeSettings.relay` is optional and absent
means unknown support; `SetNodeSettings.relay` accepts only a closed consent
choice. The existing method rows/scopes/shapes/forwarding stay unchanged.
A nonempty relay choice, including mixed changes, requires the single pending
RELAY_SETTINGS_PROTOCOL constant; it remains None, so callers/dispatch refuse
it before effects. Read and legacy write replies hide the unassigned field.
Unrelated value literals add None without changing previous expectations.

New pure protocol cases cover explicit old absence, method authority/shape,
mixed pending-field admission, both reply projections, and structural privacy
against extra route/token/link/home fields. Numbers use existing constants;
this branch assigns no protocol or schema number. The node still reports no
relay field internally until its existing owner is wired. Runtime/CLI/app
consumers, generated TypeScript and security/gate proof are not ready; this
skeleton must not be integrated or used as an implemented relay operation.


Pure wire-skeleton check: exact2d3683e9 passed8/0/0 in
`devices-relay-narrow-20261115T113015Z-2d3683e9`, ordinary remote lane,
with actual vessel-proto compilation. Evidence/step digests:
`/tmp/work/m12-g4/relay-20261115/proto-test-evidence-01.json`.
This is not Clippy, generated-TS freshness, a root/native whole gate,
actual settings persistence/application, Admin guard or relay runtime proof.
The field is still pending None and must not be consumed as an available
app operation. Remote numeric UID/GID and live service flow are unverified.


## Runtime implementation source checkpoint (unverified)

The existing Relays owner now retains one configured registration generation
and its completion tracker. Its safe advertisement receiver controls only
default device invitation routes; explicit fleet callback overrides remain
independent. Startup rejects more than MAX_CONFIGURED_RELAYS before token/TLS
I/O; saved consent is loaded before registration starts. Private generation
state tests use a controlled task barrier, not a real relay transport.

The existing Settings lock is now owned by one shutdown-tracked mutation
before spawning any blocking writer. A caller timeout/drop does not release
that mutation's capacity. Complete mixed validation precedes persistence;
live application follows the committed snapshot. The relay waiter has the
named ten-second deadline; a timed-out stop remains owned and disabling.
A weak Relays reference avoids a new ownership cycle. Startup relay preparation
moves into node/configured.rs to keep its operator authority in one place and
to shrink the existing node.rs size record; no size limit is raised.

The CLI adds an explicit enabled/disabled choice and prints only the same
safe registration/callback facts. New relay failures publish closed messages
and codes; private storage causes remain local and are not logged through
this boundary. No new dependency, relay endpoint or credential input exists.

This is source only until the requested narrow compilation/tests complete.
Actual write-failure/cancellation/restart and real isolated relay barriers,
permission/old-peer integration, generated TypeScript, V3 security review and
the final root gate remain owed. The pending protocol constant is still None;
no remotely available relay mutation or app consumption is claimed.
