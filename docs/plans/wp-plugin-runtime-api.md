# Portable plugin helper API slice

Base: announced node head9c150c6e (gate50c). This slice carries ONLY canonical
plugin value models, the standalone vessel-plugin library/own fixtures and its
workspace/boundary registration. It does not import the earlier W15 desktop
join, local-node crate, staged node manifest/source/inspection code, CLI init,
private settings proposal, service handlers, RPC rows, installer or app work.

[Decision Log]
- Purpose: let the node integrator adopt the reviewed supervision prerequisite
  without inheriting unrelated package drafts or treating a library API pass as
  an installed plugin feature.
- Reuse: proto graph/private V1 messages/canonical catalog and the now-shared
  proc checked-wait contract. Source files match the reviewed development tree.
- Choice: one separate crate with its explicitly authorized public library API,
  actual feature-gated fake-helper fixtures and no product Node dependency.
- Reason: framing/correlation/cancellation/group ownership belong to one helper
  supervisor, while installation trust, human authorization and broker/session
  policy belong to the eventual actual node host.
- Cost: this is usable only by trusted node-owned embedding code. It does not
  make a plugin runnable through a UI/CLI or qualify any missing owner adapter.

The reviewed scalar/graph/private helper values retain PLUGINS=None. Private
HelperProtocol V1 is not an assigned public node method number. There is no
additional RPC transport, source credential, driver or settings key. Unknown
server families remain refused; tools/providers/usage are not advertised by the
initial closed initialize/ready/invoke/hook protocol.

The library consumes a node-prepared absolute command/cwd, cleared allowlisted
environment and bounded argv. This is trusted UNSANDBOXED code with the node
account's access. This constructor does not prove source ownership, full
artifact identity, review, source acquisition or executable qualification.
A present Tokio handle is required before spawning; the caller must supply the
actual Node runtime with process I/O/timers and retain its lifecycle through
cleanup. An already-cancelled parent refuses new starts, but the real node host
still needs a serialized start/owned-child registration fence against shutdown.

One actor bounds16 admitted calls/8MiB queued-and-active data, keeps permits
through cancellation until settlement/retirement, uses monotonic correlation and
rotates generation on semantic declaration/subscription replacement. The frame
reader checks4MiB before allocation. Raw serde errors are sealed; private logs
retain bounded count/bytes/line prefixes, not scoped diagnostics. Caller/context
authorization and business decoding remain outside this generic actor.

Owned shutdown retires protocol pipes separately, obtains the checked leader
cleanup receipt, then bounds stderr EOF drain. An open pipe times out, aborts
and joins its reader, retaining a marked captured prefix; it cannot claim
complete logging or descendant death. OwnedGroupExit proves leader reap plus
accepted first cleanup request/already absent, not death of every descendant,
durable PID identity, socket ownership or a sandbox.

Original scoped evidence remains separate: V3row169 values, row219 selected
28 process/unit cases+strict and row235 catalog/source boundaries. Those earlier
runs are predecessor evidence on their exact trees. This portable slice requires
new exact-source proto/plugin fixture execution, strict checks, applicable
root/boundary/TS checks and a quick portable-source review. The integrator's
whole node gate remains the combined-head qualification; this document claims
none of those new results in advance.

W3 owns the new opt-in CommandDiagnostics::Redacted primitive. Its source-only
checkpoint8cafce576 is NOT adopted here. The plugin prepared-launch boundary
will consume the reviewed exact API/head when available; no producer code or
process identity/reaping implementation is copied or edited in this slice.
Actual Windows/macOS/helper installation and all F6 product/UI work remain owed.


## Exact portable execution and map correction

Exact1cbafe18b4104fbc0974ecd52e4778c63162edf6 strict Proto/Plugin/Node all-target
fixture+TS checking passed in wp-plugins-runtime-api-check-20261115T164548Z-1cbafe18.
Actual Proto/Plugin checked and Node compiled; Finished2m00s. The separate same-head
wp-plugins-runtime-api-test-20261115T164548Z-1cbafe18 executed all A5 preparations
successfully, then22 library units,6 actual helper process cases,388 proto units
and1 proto doctest:417/0/0 with0filtered. Empty fixture/doc groups are not passes.
Held stderr after real checked cleanup, prefix/noEOF drain, Ready deadline and
request-received cancellation regressions ran by name.

The first portable quality run passed fmt/size then refused boundaries because
this package recorded its edge in ADR0001 but omitted ARCHITECTURE's executable
crate table. No source/runtime test failed in that run; TS did not execute after
the blocking refusal. Adding the identical proto/proc row to that actual source
of truth changes no Rust, manifest or lock bytes. The corrected quality run and
V3 upgrade plus integrator combined gate remain owed. Old417/strict source hashes
remain their exact evidence; no complete F6/NodeHost/platform approval follows.


## Prepared-helper diagnostic consumer on the announced node line

The reviewed command-diagnostics primitive now reaches this package through the
announcedbc9cacc6 node head. PreparedCommand always opts into proc's Redacted
policy after validating its existing absolute/minimal-environment/argv bounds.
Supervisor still calls the same public decorated vessel_proc::spawn entry; no
new runner, reaper, subprocess API or authorization bypass is introduced.

Why: a trusted helper's argv, program or cwd may contain private data. Hiding
only PreparedCommand's outer Debug did not protect proc's actual spawn logs or
nested spawn-error chains. The one shared policy protects that diagnostic edge,
while the original executable/argv/environment/cwd remain what the OS receives.
It does not hide OS argv, sanitize helper output, sandbox code or manufacture
candidate/source authority. Node product wiring and permissions remain separate.

The added actual failed-spawn case uses only a missing synthetic helper path
and walks every RuntimeError/SpawnError source Display/Debug level for a canary.
The existing prepared-launch unit also checks raw-spec privacy and unchanged
argv. Current-source strict and feature-enabled actual helper fixtures are owed;
previous portable runtime results are not claimed for this new consumer.

### Canonical case-clock follow-up

The first diagnostic-consumer6c fixture run executed23 library cases plus6 actual
helper cases successfully, paired with strict success. That is predecessor
execution only: the newly added async missing-program case still needed the
repository's canonical whole-case safety net. It now uses a named30s
vessel_testkit::within clock (shared fake build before clock), retaining every
canary/source and argv assertion. An unexpected successful spawn must settle
its same owned supervisor under a named10s cleanup phase before failing the
case; it never becomes successful evidence.

Only the existing local workspace vessel-testkit dev edge is added, with no
normal product edge, registry package or public API. The manifest-only candidate
used the fixed root-dev-lock offline export recipe. Its exact201451a0 run
wp-plugins-runtime-diag-lock-20261116T014945Z-201451a0 returned a valid audit and
only the vessel-plugin-to-vessel-testkit dev dependency edge in Cargo.lock.
Original lock bytes and every exported SHA were checked before applying that
canonical patch. Registry packages/versions/checksums are unchanged; the isolated
offline recipe compiled nothing and ran no build script or network operation.
No unlocked build or unrelated resolution was used. Corrected-source locked
strict and the actual fixture suite remain owed on this canonical lock.

Exact35f3271c now passes locked/offline all-target fixture-enabled strict and
the unfiltered fixture-enabled package tests. The latter executes23 library
units plus6 actual helper cases,29/0/0 with0filtered; empty fixture/doc groups
are not counted. All four prescribed preparation/cargo steps pass. The new
whole-case failed-spawn canary, held-stderr cleanup/drain and cancellation cases
ran by name. This closes the changed-source narrow evidence only. The isolated
dev edge's fast format/size/boundary checks, V3's selected review and normal
receiving integration remain separate; no full F6/NodeHost/installer or other-OS
acceptance is claimed.
