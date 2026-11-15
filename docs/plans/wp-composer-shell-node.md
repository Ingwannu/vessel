# Composer command discovery

Contract: ADR 0085, proposed. Base: d954fa3a.

Existing assets: reuse `AgentCommand`, bounded provider report conversion, prompt-free `probe`, workspace root validation and the one RPC method table. No new dependency.

## Resume point

Values skeleton: optional CommandKind, typed draft root/config and catalog, one pending COMPOSER_COMMANDS constant. Command metadata is stripped from conversation replies, event records and forwarded reply envelopes while unassigned or before its assigned version. ACP namespaced metadata is parsed at the conversion edge; handshake probes now retain command reports that arrive before Ready. No command-discovery RPC is registered or callable yet.

Narrow prototype compile initially failed because a test used AttachmentLimits::default instead of DEFAULT. This is fixed but the corrected tests and Clippy have not yet run. No whole gate or generated TypeScript refresh has run.

Next: verify the values; retain before/after-Ready command discovery under one bounded probe policy; validate draft root/relative directory/options before spawn; reuse existing probe cache with the full draft configuration in its key. Register the pending call only after the shared method-table head is announced. Native provider discovery must be implemented or explicitly Unsupported; no inferred/fabricated empty result.

Driver ownership is agreed: this package owns command metadata and catalog ingress in existing drivers. The provider registry package does not edit those paths. Timeline receipts/client ids are ADR 0084, not this package.

The values candidate `57f2f0ec` passed remote proto command tests 4/4 (`node-values-remote`). Quality B head `29eebb8b` merged without conflicts at `b0da28bf`. The pending unary/Operate `agent.draft_commands` row, workspace validation, complete-draft cache identity, model-credential scope guard and prompt-free runtime selection are now implemented but unverified. Catalog probes wait for Ready and Commands in either order. Ordinary options probes retain their old Ready behavior. Native catalog drivers currently return a typed Unsupported before spawn; implementing their catalog ingress remains work, not parity. Tests for post-Ready report, unsupported pre-spawn refusal, config validation and draft cache replacement have been added but not run. TypeScript regeneration and every whole gate remain pending.
