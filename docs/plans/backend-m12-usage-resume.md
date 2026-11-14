# Usage: node continuation

Accounting and its review corrections are committed through `8ab23bda`:
`cb267035`, `7032f3cd`, replay origin `57ebcb3c` with declarations `c7063c08`,
and common envelope compatibility `09b89440` / `8ab23bda`. The latest actual
results and selected-input provenance are in `backend-m12-usage.md`.

The full Rust run has one explicit integration-owned fixture exception:
1,808 passed, 1 failed, 10 ignored. The exact failing fleet test passed once
alone. All 417 web unit tests and 62 browser tests passed, as did fmt,
Clippy, TypeScript freshness, lint, types, bundle and required binaries.
Both new runs retained matching selected-input pre/post identities. This
does not replace the older run's missing provenance record. Whole-run
failure remains recorded; review and integration approval are pending V1.

Next: finish the ADR 0042 quota contract, then implement opt-in lookup and
notice facts across wire, harness, node, client, CLI and the testkit. The
external lookup is off by default. There is no quota implementation in
this commit and no live-account verification. A launch-pinned nonsecret
session binding must prevent profile reload from choosing another account;
Retry-After must apply across aliases of the same token; cancellation must
retain admission through every blocking read and certificate setup.
Prepared drafts still need those corrections, compilation and all gates.

Screen work is outside this continuation. Consumers can use `usage.query`
with its coverage, cost visibility and pagination; account-limit calls are
still pending. No native Usage control, page or view model was delivered.
The provisional protocol/schema numbers and shared scope integration stay
with node integration. No push, deployment or live permission change.
All recorded fake-fleet processes were stopped; no display was started.
