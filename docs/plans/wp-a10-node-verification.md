# Named launch profiles: recorded Linux checks

## Source and availability

Implementation source: `2f1c9f5a8b4df53347e40d3eb7f9753698342c03`, based on `d432ef0f`.
The allocated store schema is 19, expressed relative to title-origin schema
18. `NAMED_LAUNCH_PROFILES` remains None. All five methods are pending;
this evidence does not make them callable or assign a protocol number.
The result documents are a separate companion to this tested source.

The original and expanded ADR 0076 contracts passed document review through
`d616e8f9`. Later implementation refinements and the executable package
still await their one implementation review. In particular, the current
canonical account facts, concrete CAS field spelling, internal fenced read
and publication ordering must be reviewed with their code and tests.

## Complete package run

Run `wiregaps-narrow-20261115T163004Z-2f1c9f5a` executed all 18 steps without a skip.
Its raw workspace result is 117 summaries: 2,405 passed, one failed and
12 ignored. Seventeen steps passed. The raw run remains failed.

| Step | Return code |
| --- | --- |
| `web-ci` | 0 |
| `fmt` | 0 |
| `rust-size` | 0 |
| `boundaries` | 0 |
| `clippy` | 0 |
| `clippy-ts` | 0 |
| `clippy-forge` | 0 |
| `ts` | 0 |
| `bins` | 0 |
| `nested-fakes` | 0 |
| `nested-vesseld` | 0 |
| `test` | 101 |
| `web-lint` | 0 |
| `web-typecheck` | 0 |
| `web-test` | 0 |
| `web-fonts` | 0 |
| `web-e2e` | 0 |
| `test-forge` | 0 |

Separate feature results: TS six passed with no generated diff; forge
fixture four passed. Web unit tests 464 and browser E2E 67 passed. Both
required fake/node prebuilds ran before the workspace tests. Default,
TS-enabled and forge-fixture Clippy all passed.

Observed one-minute load was 41.53 at the whole run's start and 27.08 at
its end; the Rust step recorded 40.66 and 30.66. These are boundary samples,
not the exact load when the failing assertion executed.

## One named shared case and its single replay

The sole failure was
`manager::tests::the_least_recently_used_document_makes_room_for_a_new_one`,
`vessel-lsp/src/lifecycle_tests.rs:212`: file 55 returned `Err(Timeout)`.
This is a timed query under the existing three-second fixture request
budget, before the final eviction/reopen assertion; the 90-second outer
safety limit did not fire. The complete LSP crate is unchanged from the
package baseline. No test assertion, timeout or product behavior was
changed in this package to handle it.

The coordinator assigned the shared LSP case to its module owner and
explicitly authorized one replay. An initial requested load threshold was
replaced with an observation rule because the runner cannot enforce it at
Cargo entry. No low-load guarantee is claimed.

Replay `wiregaps-narrow-20261115T180412Z-2f1c9f5a` used the same immutable source and only:

```text
cargo test -p vessel-lsp --lib -- --exact manager::tests::the_least_recently_used_document_makes_room_for_a_new_one
```

Exactly one test passed, zero failed or ignored; 21 were filtered out.
The libtest body took 3.34 seconds. The runner's Cargo-step load samples
were 43.19 at entry and 39.32 at completion; these are observed step
boundaries, not inferred per-case measurements.

The evidence is therefore **17/18 whole-run steps plus this named case
closed by the one authorized replay**, with both raw logs retained. It is
not a wholly green raw workspace run, a waiver or proof of the LSP root
cause. No unchanged whole run or second single-case replay was performed.
A wholly green combined-line gate remains the integrator's obligation.
The shared owner's separate deadline change is not included here.

## Corrected package cases and retained failures

The first complete run on `db2e4fdf2` was 2,402 passed, four failed and
12 ignored. Its four failures were the new runtime fixture treating an
immediate kill request as terminal, the exact Admin-method allowlist,
and two older-inbox migration replay fixtures omitting the new table.
The correction retained every semantic assertion and existing deadline.

The selected node/protocol/store library run
`wiregaps-narrow-20261115T160539Z-2f1c9f5a` passed 649 tests. All four
formerly failing names also passed in the complete run recorded above.
Eight shared configuration-owner barriers, both real fake-ACP probe
reload/cancellation cases, actual CLI refusal and store commit/CAS/restart
cases executed successfully. This is fake-driver evidence, not a claim
about a real provider or UI.

Earlier compiler/TS preparation failures are retained. The final TS stage
passed the actual omission/null regression and strict alias inventory;
the generated snapshot's six manifest hashes were checked, its patch was
empty and all three generated files matched the frozen source. The
existing generator whitespace style was preserved without manual edits.

## Identity and remaining work

All four pre/post manifests for the complete run and replay match the same
2,012 selected tracked inputs. They cover the selected source, manifests,
fixtures, scripts and documents; they do not attest ignored configuration,
assets, environment, dependency caches or the toolchain. Actual changed
crates were compiled. Raw step logs, per-step hashes and independent
recounts are retained under the run identifiers above.

Next: one implementation review, integration-owned protocol assignment and
combined gate, then the reviewed numbered consumer handoff. The native
editor, application-owned legacy markers and MCP facade are separate
consumer work. Remembered startup choices are not legacy favorite records.
No real agent prompt, account/preference read, production service, preview
pairing, push or production permission change was part of these checks.
Other OS, hosted CI, release and UI behavior remain unverified.
