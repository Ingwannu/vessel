# Host orchestration consumer

The view module owns fixed connection context, consumer, visit and attempt,
selection drafts, fresh removal confirmation and uncertain-write recovery.
Link performs typed node calls; App draws existing kit controls and validates
the received weak envelope immediately before the view's full scope/ticket
check. Onboarding retains the same opaque envelope until its own final apply.

[Decision Log]
- Reuse the canonical Skills DTO/client contract and the existing SelectedLease,
  DeliveryGuard and bounded sender. No app-owned settings file, raw client,
  alias re-resolution, second actor or guard constructor is introduced.
- SelectedConnection::check_call performs active/support/active against the
  original SDK connection. It reserves no id/frame and is not a scope verdict.
  Actual effects still use selected.call and the node's authenticated guard.
- Preflight refusal carries no fabricated guard. Admitted results retain weak
  original validity through queuing. A lost write receipt becomes Uncertain,
  retains input and requires a fresh read; a stale scope cannot settle a newer
  visit or retire its connection.
- Eight bounded permits include publication/backpressure. The fifteen-second
  timer bounds the RPC, not the entire physical effect or eventual refusal
  delivery. Guarded publication has its own named five-second budget; its
  correlated refusal retains admission until receiver settlement or closure.
- Tools read/patch is Unsupported because the landed carrier has no runtime
  NodeSettings.mcp field. Skills calls retain MCP_PARITY=None; no test flips
  this constant or invents a callable node method.

The shared actor remains its owner's exact blob. Actual actor tests cover the
None preflight followed by a real History frame as a writer barrier. Synthetic
Skills payloads with real original-lease guards prove envelope/queue validity
only; they do not prove a successful Skills RPC while the feature is unassigned.
Previous prototype App5/Link5/View7 results are older-source receipts. This
straight packet still needs current-base strict, Link and owner-model tests,
wasm checks, V1, the shared-slot union and final native gate/captures/review.
Root files are untouched; no public delivery or production activation is claimed.
