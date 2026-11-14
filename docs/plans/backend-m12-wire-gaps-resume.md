# N3 resume and current checkpoint

Resumed 2026-11-14. The previous credit stop is superseded: node-side rows
3 onward are authorized, one slice at a time. Node branch fast-forwarded to
`b981f5f2`, protocol 18. Baseline gate passed: 110 suites / 1,771 passed /
0 failed / 10 ignored, page units 417 and full Playwright 62. See the new
row 3 contract and the final sections below; older checkpoints are history.
Row 3 contract-first ADR is `5cff884e`; implementation is built. Whole root
gate passed: 110 suites / 1,781 passed / 0 failed / 10 ignored. Final page
E2E passed all 62, and the complete CLI fake fleet passed. Source hashes
were unchanged during the gate. Implementation review is pending with V1.
Review goes to V1 only; it forwards accepted work to the node integrator.
Do not message the integrator or lead directly unless answering them.
Keep project=vessel and machine cargo slots; every gate runs detached.

## Committed and reviewed

- `b9cf9397`: row 1 contract.
- `f81798c3`: machine timezone, scheduler-backed preview, typed failure,
  old-peer adaptation, client/CLI/testkit/fake-fleet support. V1 passed the
  source/contract and forwarded it; the node integrator found no blocker.
  Whole workspace: 105 suites, 1,648 passed, zero failed, nine ignored.
  Protocol/web TS libraries: 159/54. Page units: 243; page E2E: 36.
  Final fmt/clippy passed. Native editor and other OS runtime are unverified.
- Eight consecutive grant-stop checks on row 1 passed with the existing
  five-second assertion. The exact executable and Cargo-unset invocation
  record is `row1-grant-stop-invocation.txt` in this track's scratch folder.
- `27ef259a`: exact cherry-pick of the node integrator's `bce3dd9e`, moving
  the reinstalled-member timestamp before restart. No independent fix here.
- `582283c7`: exact cherry-pick of integrator `d3cb09b3`, timing the late
  member's grant from joining, after node creation. Existing 10/20-second
  bounds remain. Their final validation is not claimed by this note.

## Historical row 2 checkpoints (superseded by final result below)

Credit-stop checkpoint: preserved as the commit titled `wip: fleet membership
controls and bounded sync status`. It is not an approved/ready slice. At that
checkpoint the sixth clippy was still queued, and the whole workspace test,
TS generation, page/fake-fleet gate and current-artifact grant repeats had not
run. A later entry must explicitly supersede those unverified items.

Implemented: `JoinFleet`, `LeaveFleet`, `SyncFleet`, `GetFleetStatus`, client
methods, selected-node CLI operations and offline compatibility, safe status
projection, bounded device windows with cancellation-retained admission,
manual notifications into the existing loops, serialized old/new loop
lifetimes, shared membership commit locking, typed uncertain join and large
status refusals, partial-leave recovery, real-node regressions and fake fleet.
`NodeClient::leave_fleet` and `sync_fleet` return unit on wire Ack.

V1's early contract review requested byte/item/reply limits, lock lifetimes,
post-redemption uncertainty and partial-leave fixtures. These followups are
implemented; V1 has not approved the whole row. The status generation is
historical without current-home association: do not label it up to date by
comparing bare generation numbers. ADR 0044 records the decisions.

Gate evidence in the track's scratch folder:
- `row2-clippy.log`: first attempt waited 24m06, failed method-table docs and
  redundant Ack qualification.
- `row2-clippy-reviewed.log`: second attempt failed two documentation ticks.
- `row2-clippy-current.log`: third reached node checking, failed unused
  results; those are fixed. No lint or assertion was weakened.
- `row2-clippy-fourth.log`: fourth attempt reached CLI and failed its invitation-encode error mapping
  and generation formatting. Those were fixed with a distinct source-carrying
  error and the typed generation getter. The membership-token lifetime fix
  was installed while that attempt was running; do not treat it as a final
  gate for the latest source.
- `row2-clippy-final.log`: fifth whole clippy, covering those fixes and the
  latest source; check its final result. No success is claimed in this note.
- `row2-source-before-final.json`: frozen Rust/Cargo input hashes and mtimes
  for the final attempt; verify its aggregate again after the gate.
- Fifth attempt failed two store-test style lints and an obsolete supervisor
  field; fixed without exceptions. `row2-clippy-ts.log` is the sixth attempt,
  whole workspace/all targets plus protocol/web TypeScript features. The
  latest frozen input record is `row2-source-last.json`. Its feature flags
  must be stated in the gate report; no whole-test result is implied.
- Sixth attempt reached the normal CLI binary and found its JSON error
  dependency was test-only. It has been promoted to a normal dependency,
  keeping the same workspace version. `row2-clippy-last.log` is the seventh
  whole workspace/all-targets attempt with protocol/web TS features; check
  its final result. The previous source hash is superseded by this Cargo
  dependency correction, not an unchanged-tree rerun.
- Seventh attempt reached node E2E and found the two new test modules lacked
  the existing `cfg(test)` registration used by their neighbours; helpers
  were therefore outside Clippy's test context. It also found the lifecycle
  test was too long. Test-only registration is restored and the same history
  is split into join/failure and manual-sync helpers, without removing
  assertions. `row2-clippy-e2e.log` is the eighth whole workspace/all-targets
  attempt with protocol/web TS features; check its final result.
- `row2-fmt-fourth.log`: fmt check before that attempt; later formatting
  changes were made only while the command remained queued.
- `row2-fixture-failure-check.log`: one-line assertion failure, exit 1 and
  no traceback passed. Real row 2 fleet startup is not yet claimed checked.

Still required unless a later final entry below records completion: clean
whole workspace test with counts; TS regeneration and generated declarations;
page lint/typecheck/unit/build/E2E gate; actual fake-fleet check and cleanup;
four consecutive reinstalled-node checks and the relevant grant-stop/leave
checks on the current executable. Confirm changed crates really recompiled.
Run one cargo command at a time through the machine's cargo-slot helper.
Do not clean build caches, interrupt a queued build or replay an unchanged gate.

## Integration and consumers waiting

The node integrator merged row 1 as protocol 13 in `187dc847`, preserving
the source commits and renumbering preview/timezone/old-peer fixtures. The integrator subsequently reported its full gate passed: 1,692 tests,
zero failures, and 62 page E2E cases. Eight slow-grant and eight grant-stop
checks passed on that integrated head. This N3 branch still declares 12;
row 2 also uses that original track version. Do not create a further N3
protocol bump. Consume the reviewed integrated head at a clean boundary in
the resumed session and keep all N3 additive calls at 13, minimum unchanged.
No such merge was made after the stop notice.

- G4 waits for reviewed row 2 fleet controls/status, then row 3 device.self
  and row 4 unreachable-invite refusal. Do not integrate unverified calls.
- G2 has the reviewed row 1 contract; its native editor/captures are its own.
  It still waits for row 7 webhook calls/listener.
- N1 and all app tracks wait for row 3: reserved only, `GetDeviceSelf {}`
  -> `DeviceSelf { device: Option<DeviceId>, scopes: Vec<Scope> }`,
  `NodeClient::device_self()`. Authenticated without Read; only own paired
  id, otherwise absent; current effective/clipped/intersected scopes. No
  implementation or reviewed commit exists on this branch yet.
- G1 waits for row 5 project open PRs without live sessions.
- G3 waits for row 6 registry flag/environment and reachable too-old/new.

Rows 3-7 are unimplemented, not complete. Earlier scratch preparation for
row 3 (`row3-device-self.rs`, `row3-decision.md`) and row 6 source notes is
not installed or verified. Re-read the integrated source before using it.
Row 7 needs ADR 0043 and a safe resolution of ADR 0034's harness permission
boundary before opening a listener; a prompt fence/Read scope is not a shell
sandbox. No webhook listener, real agent prompt or preview service was used.

Keep the two local Cargo configs, font assets and web dependency/bundle
symlinks; never commit them. Before stopping, verify no owned fixture/node/UI
process remains and record the final commit and exact verification below.

## Latest checkpoint

- Row 2 source is preserved by WIP commits `2ced8723`, `cc972008` and
  `a32a0ac6`. All remain unapproved for integration.
- `row2-clippy-e2e.log`: eighth whole workspace/all-targets clippy with
  `--features vessel-proto/ts,vessel-web/ts -- -D warnings` passed. It waited
  76 seconds for its machine slot; Cargo reported 1m27s. Latest fmt and diff
  checks passed. No lint exception, expected value or timeout was weakened.
- The current command is `VESSEL_WRITE_TS=1 RUST_TEST_THREADS=2 cargo test
  --workspace --features vessel-proto/ts,vessel-web/ts --no-fail-fast`, through
  the machine build-slot helper. Its log is `row2-workspace-ts-test.log`.
  This is the complete workspace with the TS-generating tests enabled,
  rather than a package subset. It has not finished at this checkpoint;
  report the exact flags, final suite counts and failures, not a guessed pass.
- TS declarations may change when those tests reach their generator. Commit
  them and the final result only after inspecting the completed log. Page
  and real fake-fleet gates plus current-binary grant repeats are still
  unverified. Protocol-13 integration and rows 3-7 remain deferred.

### Runtime findings and followup

The complete TS-enabled run finished: 105 suites, 1,669 passed, four failed,
nine ignored. Failures were the lifecycle fixture's first manifest query
(a file was visible before publication), two protocol classification tests
that did not yet account for the new fleet methods, and the existing remote
release-fallback test's unchanged 60-second timeout. The last is not classified
as flaky and its timeout/code has not been changed here.

After the run ended, the first three fixtures were corrected without changing
expected authority, state or time limits. Initial readiness now checks the
actual read-only RPC under the existing STEP deadline; the public watch is
unavailable before the first publication, so its probe is paced. New Admin
methods are pinned explicitly; the fleet additions share preview's first
version rather than a later global protocol maximum. No join is auto-retried.

`row2-clippy-runtime-fixes.log` passed the complete workspace/all targets
with TS features after the first corrections. Page lint/typecheck, 55 files /
243 unit tests, bundle build and 36 E2E cases passed; logs are `row2-web-*`.
The current original root gate is fmt, full clippy without extra features
(`row2-final-clippy.log`), then exactly `cargo test --workspace` with two test
threads (`row2-final-workspace-test.log`). Read their final results before
claiming success. Generated `web/src/wire/proto.ts` is current and must be
committed with this followup. This remains WIP until the full gate, real
fake-fleet check and required grant repeats are complete.

### Real fleet verification

Followup `caaea148` preserves the runtime fixture corrections and generated
wire declarations. The TS generator's own whitespace is retained. The original
full Clippy passed (`row2-final-clippy.log`); its sequential exact whole test
is still running at this checkpoint. No product source changed during it.

The complete native fake-fleet start passed, including the row 2 wire checks:
home/member/standalone roles, real join/leave/fresh join, manual sync, actual
apply and home report, and a separate read-only book with hidden member rows
and refused sync/leave. The full script finished with exit zero. Invocation:
`bash spikes/native/scripts/fleet.sh start <scratch>/fleet-row2 <target>/debug`.
`row2-fleet-results.json` records binary hashes, check names and ownership.
Run as UID/GID 1000 with umask 077: fixture directories 0700, tokens, member
credential and both node books 0600, all owned 1000:1000. No chmod/chown fix
was needed. Both recorded node PIDs were stopped and absent afterward. No
pairing link or credential output was displayed. Native screen rendering and
other OS execution are not verified by this backend check.

### Final root gate and grant checks

The exact original root gate completed successfully on the current source:
fmt, `cargo clippy --workspace --all-targets -- -D warnings`, then
`cargo test --workspace` (RUST_TEST_THREADS=2), each build/test through the
machine slot helper. The test result is 105 suites, 1,666 passed, zero failed,
nine ignored. Node unit tests: 198; node E2E: 151. Log:
`row2-final-workspace-test.log`. Changed crates are explicitly recompiled in
the log. This supersedes earlier incomplete whole-run checkpoints.

The earlier TS-enabled whole run remains recorded as 1,669 passed / four
failed / nine ignored. The three new-fixture failures are corrected; all pass
in the final root run. The existing remote release-fallback test also passed
in the final root run without any change to its code or 60-second timeout.
Its earlier timeout remains an unexplained prior failure, not a waived
known-load exception.

On the exact node E2E executable named in the final root log, reinstalled
member grant passed four consecutive runs and grant-stop passed eight. Each
ran with CARGO removed so the locator used existing binaries. The whole
workspace continued through unrelated later tests while these checks ran;
no second Cargo command was started. `row2-grant-repeats.json` records each
argv/log/exit, UID/GID 1000, pre/post executable SHA256 equality and no Rust
source changes. No timing bound or assertion changed.

The literal requested filtered TS regeneration command is now running:
`VESSEL_WRITE_TS=1 cargo test -p vessel-proto -p vessel-web --features ts
--lib typescript`, with the same slot helper; log `row2-final-typescript.log`.
Do not report it passed until its result is recorded below. The previous
TS-enabled whole run did pass the generator tests. The owned-process audit
found no remaining fixture/node/UI process; no preview or real agent used.

## Final handoff: row 2 gated, V1 review pending

This section supersedes all earlier unverified checkpoints. Row 2 source is
committed in `2ced8723`, `cc972008`, `a32a0ac6`, `caaea148`; the final clean
root gate and grant repeats are recorded in `bcc9106c`. Full source/contract
review is with V1, followed by the coordinator and node integrator; no merge
approval or native UI approval is claimed. The earlier WIP commit subjects
are retained without history rewriting.

- fmt passed; whole workspace/all-targets Clippy passed.
- Exactly `cargo test --workspace`: 105 suites, 1,666 passed, zero failed,
  nine ignored (`row2-final-workspace-test.log`). Node units 198, E2E 151.
- Literal TS generation passed both tests (`row2-final-typescript.log`);
  generated declarations were already current at `caaea148`.
- Page lint/typecheck/build, 243 unit tests and 36 E2E passed (`row2-web-*`).
- Actual native fake-fleet start/check passed (`row2-fleet.log`,
  `row2-fleet-results.json`), including read-only book, apply/report and
  join/leave/sync. Assertion failure check prints one line, no traceback.
- Reinstalled-member 4/4, grant-stop 8/8 (`row2-grant-repeats.json` and
  twelve individual logs). Exact executable/argv, CARGO unset, UID/GID and
  stable pre/post SHA256 are in the record; no test bound changed.
- Final owned-process audit empty (`row2-process-audit.json`). Own fleet
  PIDs stopped, no app/display left. UID/GID 1000; fixture directories 0700,
  tokens/credential/books 0600. No permission/ownership modification.

References followed: Paseo server `connected-hosts/index.ts` (actor authority,
serialized persistence and secret-free summaries), `connected-hosts/session.ts`
(connection authority/refusals), `pairing-offer.ts` and CLI `commands/daemon/
pair.ts`. Paseo has no Vessel fleet publication/apply engine: those semantics
reuse ADR 0012/0013 and the existing loops. Shared native files changed only
for fake-fleet shape preparation/checks and their README; the exact hunks
are `row2-shared-native.diff`. No app/kit screen changed; no captures claimed.

Next session: first address V1 findings, then consume the reviewed protocol-13
integration (`187dc847`) at a clean boundary and keep every N3 addition at 13,
minimum unchanged. No new bump, no merge during this stop-limited session.
G4 waits for reviewed row 2 calls/status, plus future rows 3/4. All other
consumer waits and unimplemented rows 3-7 listed above remain unchanged.

## Current row 3 handoff (supersedes earlier checkpoints)

Contract-first commit: `5cff884e`. Implementation is the commit containing
this checkpoint; its hash is added in the following documentation checkpoint.
The node baseline is `b981f5f2`; rows 1 and 2 are integrated upstream at 13
and 15, with fleet.published at 16. The upstream PR-without-session feature
already exists at 13: verify its contract when reaching row 5, do not duplicate
it. This slice provisionally adds protocol 19; MIN_PROTOCOL_VERSION stays 2.

New call: `GetDeviceSelf {}` / `device.self` -> `DeviceSelf { device?:
DeviceId, scopes: Vec<Scope> }`. Client `NodeClient::device_self()`, CLI
`vessel node devices self`. Only a direct paired principal gets its own id;
owner/agent/delegated return none. Scope projection uses the same admitted
principal as the guard, including clipped/intersected/empty/no-Read grants.
Unavailable stays unknown. The method metadata now expresses authenticated
calls without an additional scope, and typed envelope version admission
includes inner methods and fields. See ADR 0044 and the implementation plan.

Final root: fmt, whole clippy, whole workspace test all passed, 110 suites /
1,781 passed / 0 failed / 10 ignored (RUST_TEST_THREADS=2). TS generators both
passed; page lint/typecheck, 417 units, build, and all 62 E2E passed. The full
fake fleet passed CLI-only paired-person, reader and Sync-only authority
checks plus existing schedule/fleet/conversation shapes. All seven changed
crates have recorded compilation lines; all frozen Rust inputs are unchanged.
Failure history, including three earlier model timer failures, is recorded
in the implementation plan; no timeout, assertion or lint was weakened.

Logs in this track's scratch directory, under `resumed/`:
- `row3-final-{fmt,clippy,test,build,e2e,fleet}.log`, exit marker is zero.
- `row3-final-{inputs,root-summary}.json`: 110/1781/0/10, unchanged inputs.
- `row3-compilation-evidence.json`, `row3-typescript.log`, `row3-wire-tests.log`.
- `row3-web-{lint,types,test,build}.log`.
- `row3-model-diagnostic.{json,log}`, `row3-baseline-model.{json,log}`,
  `row3-partial2-lib.log`: the earlier diagnostic failures, not waived.
- `row3-shared-native.diff`: complete hunks of the three shared fixture files.
- `row3-permissions-cleanup.json`: UID/GID 1000, 13 private books/token/
  credential paths checked at 0600/0700, two verified fixture node PIDs
  stopped and absent. Fixture umask 077 is new; production permissions are
  unchanged. No native screen or other-OS runtime validation is claimed.

Review destination is V1; it forwards accepted work to the node integrator
and other waiting screen tracks. Every action surface, G4 Devices/Sync and
N1 inbox need this exact call. Review has not been approved by this note.
Next row is row 4: no-route public invitation refusal, ADR first, before
minting an invitation, with a typed actionable error and old-peer behavior.
Then verify integrated row 5, implement registry/version-state row 6, then
hook row 7 only after reading ADR 0034 whole and deciding ADR 0043's security
questions. No new native/page screen work, no direct message to integrator
or lead unless answering them, no preview pairing or real-agent prompts.
