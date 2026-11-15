# Startup recovery R2: staged engine and service

ADR 0059's accepted contract remains the behavior to implement. R1's source
`2890410c` and result record `7a00b4d6` passed value-only source and whole-gate
review; that verdict does not cover the runtime below. This branch starts
from that record and merges announced node `ec5b1162` at a clean boundary.
No R2 runtime code has been installed yet.

[Decision Log]
- Purpose: implement the recovery settings and startup coordinator while its
  RPC fields remain unregistered.
- Existing parts: the Settings engine owns saved-value precedence and runtime
  application; DataDir owns parsing, atomic private writes and the head's
  unreadable/later-file set-aside policy. Recovery must extend these owners.
- Choice: modify/reuse them with a bounded node-owned FIFO operation. Stage
  the node's recovery service under cfg(test), with tests using actual store,
  fake providers and loopback. No duplicate settings engine or second parser.
- Alternatives: a second settings actor, raw JSON in the node, or a test-only
  RPC/authority bypass would lose ordering or cross ownership boundaries.
  Existing Tokio and workspace APIs suffice; no new dependency is planned.
- Consequence: source reviewed, staged code stays on this track until the
  later wiring commit. A staged test pass is not product startup approval.

## Merge baseline

The new node head owns PROTOCOL_VERSION, store schema, method requirements,
all existing tests and the settings-file set-aside rules. R1's feature fact
remains Option None; no request, call or new migration was added by this merge.

Conflicts resolved under the shared merge rule:

- README: retain the complete node-head text, append R1's status paragraph.
- ARCHITECTURE: retain the complete node-head text, append R1's value section.
- rpc/older.rs: retain the head's label-aware projections and tests byte for
  byte; reapply only R1's Get/Set settings projection and its test module.
  The old Park/Wake/Fork/Rewind no-op arms are not restored.

Code and generated declarations otherwise merged without conflict. The
merged baseline needs fresh narrow checks and its whole gate before runtime
work. R1's submitted source/record remain unchanged in their own branch.

## Staging boundary to implement

The entry module is cfg(test) until wiring after the shared required_protocol
accessor and Quality B's method table. Keep its ordinary typed interface and
real actor behavior; do not suppress dead code, widen visibility merely to
quiet a lint, introduce a feature flag, or build a second engine for tests.
Cross-crate file/store ownership and the existing settings actor must remain
single owners while the service is staged. Record the precise dependency
boundary before changing code.

Required evidence remains ADR 0059's: cancellation/drop during actual save and
post-save cleanup, Off generation and pending cleanup before later On, fresh
stored-grant and policy/manual-stop checks after awaited load/ready, own-run
cleanup only, clean-stop and killed-daemon fake-provider recovery, bounded
launch concurrency and no prompt for excluded or manually stopped sessions.
The raw legacy setter test seeds a nondefault policy through DataDir and
checks the actual ordinary Admin reply bytes; it adds no recovery RPC field
or special dispatch path.

## Status

Merge baseline pending verification. Engine and service not implemented in
this branch. No new client/CLI setting, node boot hook, UI or real-provider
execution is authorized by a source-only staging verdict. Wiring remains a
separate reviewed change after the shared admission seam arrives.
