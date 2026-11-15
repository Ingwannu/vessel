# Named launch profiles: integration checkpoint

## Committed source and evidence

The isolated package starts at `d432ef0f`; implementation source is
`2f1c9f5a8b4df53347e40d3eb7f9753698342c03` (initial package `b9d118624`).
Store schema 19 and the unassigned `NAMED_LAUNCH_PROFILES` feature remain.
All five typed methods are unavailable over every transport until the
integrator assigns the feature. The exact calls and authority are in
[the package plan](wp-a10-node.md).

The complete run is 17/18 steps, Rust 2,405 passed / one failed / 12 ignored;
the sole unchanged shared LSP query timeout passed its one explicitly
authorized same-source replay. Both raw results remain retained. This is
not a wholly green raw run or a blanket timeout waiver. TS six, web units
464, browser E2E 67 and forge fixtures four passed. Full results, source
identity and the observation-only load rule are in
[the verification record](wp-a10-node-verification.md).

The earlier four fixture failures were corrected without weakening any
assertion or deadline: selected libraries 649/0 and all four names passed
in the complete current run. Existing compiler/TS/red-run history is kept.
The separate project-pull service and unrelated native ancestry remain
excluded from this package.

## Review and resume order

1. Submit the complete implementation and result companion as one package.
   Prior ADR acceptance covers contracts only through `d616e8f9`; current
   refinements and executable behavior still need implementation review.
2. Resolve actual review defects with changed-source checks. No unchanged
   whole run or second LSP replay is authorized by this record.
3. The integrator assigns the protocol, confirms schema numbering and runs
   a wholly green combined-line gate. Assigned happy-path wire evidence is
   distinct from the current pending-feature refusals and internal runtime
   tests. The shared LSP owner's separate fixture correction is not imported.
4. Hand only the reviewed, numbered contract to native/MCP consumers after
   integration. Native UI, validated app-local legacy records, per-NodeId
   marker/in-flight persistence and MCP wrappers remain their owners' work.

The node owner, client migration receipt and internal fenced native read
are implemented. A fenced read is not an atomic session/account/run
snapshot. The app must use its checked delivery guard and actual target;
remembered choices remain startup defaults, not favorite records.

No implementation-review PASS, callable/numbered feature, native/MCP/UI,
real-provider, other-OS, hosted-CI or release approval is claimed here.
No production service, permission, real account, preview pairing or push
was changed. Retain the source and both red/isolated logs when resuming.
