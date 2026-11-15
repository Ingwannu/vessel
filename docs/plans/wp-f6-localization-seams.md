# F6: shared Fluent contracts for installation-owned resources

The plugins package owns per-installation text, lifetime and contributions.
The words track supplies two reuse seams inside view; neither creates a second
core translator nor imports installation resources into its static registry.
Reference behavior is the plugin client translation subscription and cleanup in
`packages/plugin/src/client/translations.tsx:54` and its message contract in
`packages/plugin/src/text.ts`. ADR 0079 owns native plugin contribution scope.
No reference translation assets are imported.

## Bundle configuration

[Decision Log]
- Purpose: plugin text must use the same number/date/particle functions and
  pseudo transform as the core while retaining its own resource ownership.
- Existing assets: concurrent Fluent bundles, embedded numeric/date rules and
  typed Number/Date argument bridges already implement the app contract.
- Alternatives: copying configuration drifts; extending the core registry leaks
  installation state into its process cache and loses generated-key guarantees.
- Choice: `localization::resources::bundles<R>(Locale)` prepares empty active and
  English bundles; `R: Borrow<FluentResource>` supports owned core resources and
  installation-owned `Arc<FluentResource>` without another dependency.
- Why: a validated Locale selects core-owned formatting data; no plugin message
  can replace the separate private DATETIME pattern bundle. Each fallback keeps
  English's own plural rules. Both bundles use the pseudo transform when chosen.
- Effects: the caller owns resource validation/addition, per-message fallback,
  value preparation/cache and disposal. Number/Date Fluent bridges are crate
  private and belong only at that argument boundary. No frame-time parsing,
  global plugin cache, file access or language RPC is added.

`resolution.rs` owns typed initialization/format errors and generic resolution;
`formats.rs` depends on that lower layer; `resources.rs` configures the bundles;
`runtime.rs` uses them for the unchanged core cache/generated API. Installation
callers retain their own owned-key error type: core FormatError's static keys
are not widened into a dynamic string API.

Tests compare Arc-backed NUMBER, DATETIME and Korean PARTICLE results against
whole core sentences for English/Korean/pseudo, English plural fallback for a
missing translated message, namespace isolation, an attempted DATETIME pattern
lookalike and release of the last installation resource Arc.

State: source and narrow checks ready. View all-target Clippy passed; all 21
localization tests passed, including the two new Arc-resource tests and retained
plain-getter zero-allocation, fallback, pseudo and language-selection tests.
Changed view code recompiled. Formatting, size/boundary/token checks passed;
there are no new size/token exceptions, dependencies or core catalog changes.
The consumer package's whole native gate and integration review remain owed.
The earlier preferred-system-language package was gated at exact `8a944a23`;
its gate/captures do not cover this follow-on. Package base 2 (`6036eeb8`) was
merged after that gate completed; its root crates remain unchanged.
