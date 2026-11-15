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
