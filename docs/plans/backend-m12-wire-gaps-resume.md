# N3 current checkpoint

Current state: ADR 0060 source `ba6702b3`/evidence `bad27224` has V1
source/scoped-evidence PASS, not integration. The resolved/staged local
merge of `9c7a45ca` now includes the exact owner's correction `a365d295`
for the four connector large_futures errors. The owner and V1 cleared that
one-file source correction; the owner's whole gate is still pending, so no
upstream whole PASS is inferred. N3's corrected baseline gate is active as
`base-node21b.sh`, with main log/exit marker `base-node21b.log/.exit`.
Do not alter Rust or issue another Cargo command while it runs.
Rows 5–7 are not implemented. The cleanup-record recovery disclosure is
below; the restored JSON is not an original-file mtime attestation.

## Committed and reviewed

- Row 1, machine zone and scheduler-backed preview: `f81798c3`, integrated
  at protocol 13 by `187dc847`.
- Row 2, fleet join/leave/sync/status: final source `caaea148`, integrated
  at protocol 15 by `ff2981f2`; publication subsequently uses 16.
- Row 3, device self: contract `5cff884e`, implementation `3d486ff4`,
  evidence `5d10d4dd`. V1 passed the node source and evidence and forwarded
  the exact call to the coordinator and waiting device/inbox tracks.
  Whole root: 110 suites, 1,781 passed, zero failed, ten ignored; TS two,
  page units 417, full page E2E 62 and complete CLI fleet passed.
- Row 4, unreachable invitation: contract `41518e9c`, implementation
  `aea21c54`, evidence `1c3abe9c`. V1 passed the node source/evidence and
  forwarded the exact refusal. Whole root: 110 suites, 1,786 passed, zero
  failed, ten ignored; TS two, page units 417, full page E2E 62 and complete
  CLI fleet passed. The coordinator forwarded rows 3 and 4 for integration.

No native screen, other-OS runtime or remote reachability approval is claimed.
The resumed baseline was `b981f5f2`, protocol 18, gated before feature code.
Rows 3/4 were subsequently integrated at protocol 20 (device.self in
`2e051820`, route refusal in `50b561d0`). MIN_PROTOCOL_VERSION remains 2.

## Existing exact contracts

`GetDeviceSelf {}` / `device.self` returns `DeviceSelf { device?: DeviceId,
scopes: Vec<Scope> }`; use `NodeClient::device_self()` or
`vessel node devices self`. Only a directly authenticated paired device gets
its own id. Other authenticated principals omit it. Canonical effective
scopes are required, including empty/no-Read grants and admission clipping;
ownership, agent limits and profile policy remain separate constraints.
Unsupported is unknown authority, never an inferred grant. Only this method
is Authenticated/None; all legacy methods retain Some(scope). Envelope version
admission includes the inner call, with direct and forwarded old-peer tests.

Row 4 adds no call or request field. Existing `device.invite` remains Admin
and since 4. `Conflict` carries `ErrorDetail::DeviceInvite { reason:
DeviceInviteError::NoRoutes }` and guidance to configure reachable --public
or --relay, then create another invitation. Route absence is checked before
randomness/link/pending admission, including an explicit empty override.
No bind inference, probe or automatic relay is added. Older clients keep the
human guidance; non-Admin still gets Forbidden without configuration detail.

## Current slice: file-seeded FakeForge (ADR 0060)

Contract-first commits: `39df0f9c`, then assigned-number move `ef3fd425`.
Gated implementation: `ba6702b3`, evidence `bad27224`. V1 passed source and scoped executed evidence; combined integration and merge remain pending.
The separate feature-gated `vessel-forge-fixture` binary injects the real
FakeForge through ForgeSetup.custom. Product vesseld has no fake selector,
even with the feature enabled. No new wire, dependency edge or protocol bump.

Version-1 JSON groups complete typed wire PullRequest values by RepoRef.
Wrappers reject unknown fields/versions, duplicates and disagreeing summaries.
Limits: 1 MiB, 16 repositories, 128 initial/runtime pulls, bounded failing
checks/merge methods, caches and retained attempts. Explicit failures are
no_access/rate_limited/down/signed_out, with separately configured refused
merges. No HTTP/account/token fallback. The file is read once before listening.

Build the CLI, node and testkit bins with feature vessel-node/forge-fixture.
The fresh fleet script generates eleven local repositories and a seed, starts
two loopback fixture nodes, pairs through CLI pipes and checks actual PR facts
and read-only refusals. `forge-fixtures.json` maps project/session ids for
screens. Push URLs are local bare repositories. PID shutdown verifies UID,
executable, data path and kernel start time, then signals a pinned Linux pidfd.
The native README contains exact commands and schema. No screen is changed.

## Current gate and evidence

Evidence filenames below are relative to the track scratch directory's
`resumed/`. Keep the embedded page assets immutable throughout Rust gates.

- `forge-feature-seed.log`: nine actual FakeForge tests passed, including
  bounds, privacy, per-repo failures, merge refusal and concurrent capacity.
- `forge-ssd-feature-{clippy,test}.log`: feature Clippy and all four actual
  launcher tests passed from empty data directories; tests took 4.03 s.
  The original 90 s total and 20 s readiness/shutdown bounds are unchanged.
- `forge-feature3-{build,page,fleet}.log`: build, all 62 page E2E (6.3 min)
  and complete CLI fleet passed. Production, forge library, CLI and Python
  fixture sources have not changed since this runtime proof; subsequent
  Rust changes only restore fresh startup in the new launcher test helper.
- `forge-feature3-permissions-cleanup.json`: UID/GID 1000, 18 private paths
  at 0600/0700, both verified node PIDs absent, no owned runtime remaining.
  `forge-processes-check.json` proves mismatch refusal and matching-child
  shutdown. No display or preview process was started.
- `forge-ssd.sh` finished with exit zero: fmt, whole Clippy and literal
  whole cargo test --workspace passed. `forge-ssd-root-summary.json` records
  111 suites, 1,796 passed, zero failed, ten existing ignores. Main log
  `forge-ssd.log` records beginning/end one-minute load 21.42/10.17 and
  available memory 15,155/17,498 MiB. The compilation-phase snapshot was
  load 22.00 and 16,192 MiB available. No timeout was waived.
- `forge-ssd-input-check.json` verifies all 1,028 Rust/Cargo inputs unchanged
  throughout the gate. The test log records actual recompilation of forge
  and node. Both previously failing remote cases and canary passed in the
  whole run; remote four passed in 161.28 s, CLI integration 39 in 8.41 s.
- SSD test scratch is private mode 0700; targets and logs remain on cold
  storage. The gate checks no owned process remains before emptying scratch
  and wrote `forge-ssd-cleanup.json`: empty, UID/GID 1000,0700, no process
  remaining. `forge-final-audit.json` rechecks all 18 private fleet paths,
  no owned runtime and empty SSD scratch. The only Rust input difference
  from the earlier page/fleet runtime run is the new launcher test helper.

The temporary in-process bootstrap workaround used for a slow HDD startup
was removed before the SSD gate. The final tests again exercise first startup
from empty data. Its historical pass is not the final fresh-start proof.

## Failures retained, not waived

Initial owned preflights exposed lint issues, a capacity assertion that reused
an already-open branch, and an inconsistent test-produced checks summary.
Those were corrected without weakening the decoder or a timeout. One valid
fresh HDD startup took about 24 s; stderr held only 862/65536 bytes, so no
pipe backpressure was established. SSD fresh-start tests now pass as above.

`forge-final2-test.log`: 9 completed suites, 181 passed, two failed, one
ignored, stopping at unchanged CLI remote 120 s safety nets:
a_fresh_machine_is_installed_started_and_used and
a_dropped_ssh_connection_during_attach_resumes. Each passed once alone on
the exact same executable, CARGO unset, in 29.77 s and 20.65 s. Solo passes
are diagnostic, not a waiver. No documented exception covered these tests.
V1 escalated the blocker; the coordinator assigned shared-test ownership to
the node integrator and authorized a changed-condition whole run with load
records after the recorded machine starvation. N3 did not edit shared tests.

`forge-normal.log`: that HDD attempt ended at the CLI canary's unchanged
60 s safety net; 7 completed suites, 178 passed, one failed, zero ignored.
The later coordinator SSD decision arrived while it ran, so it was left
unchanged. Only after exit were the new test helper and ADR installed with
current mtimes; final feature checks and the one authorized SSD whole run
then started. No deadline, assertion, retry count or shared test was changed.

The nested-build amendment arrived while the SSD whole gate was already in
progress (Clippy passed; tests queued). Keep that run unchanged. Every next
whole gate must prebuild vessel-testkit --bins and vessel-node --bin vesseld
inside its slot, in the same environment as tests, before starting test
clocks; record both commands at the log head. The integrator owns the shared
clock correction. The amendment caps slots/jobs in the shared helper and
classifies a timeout with one-minute load above 32 before/after as saturation:
run that test once with load under 24; other timeouts go to the test owner.
If the SSD whole run repeats a shared failure, route its log to V1/the owner;
do not classify it as flaky or rerun unchanged without authorization.

Prior row-3/4 failure details and passing final input/compile evidence remain
in backend-m12-wire-gaps.md and the row evidence commits. Their final private
path counts were 13 and 14; production permissions were unchanged.

## Finish and next order

Source `ba6702b3` is committed. The whole gate, frozen inputs, compilation
and final cleanup are verified; this checkpoint records its evidence. V1 passed source and scoped executed evidence and forwards the slice. Send the waiting PR-screen track
the exact command/schema only after integration is confirmed. No integration, UI or other-OS runtime approval is claimed for ADR 0060.
Do not rerun the unchanged passing gate. A complete private review report
identifies the scratch logs, source pins, commands, shared script seams and
limits above.

Next is actual project-wide open PR listing without a live session, ADR 0061.
The earlier claim that it was already upstream was withdrawn: pr.list still
covers watched sessions only. Then row 6 (registry address/version states),
then row 7 (webhook ADR and security decisions before code). No later slice
has started. Merge the node line at a clean boundary before new wire work,
then gate that baseline as directed by the coordinator.

Keep the two ignored Cargo configs, fonts and untracked web build/dependency
links; never stage them. Use cargo slots and detached gates, one Cargo at a
time. Test-written executables use Sandbox::executable. No real agent prompt,
preview pairing/ports/units, push, secret output or process-name kill. Review
route is V1, coordinator, node integrator; do not directly message the latter
or lead except to answer them. Keep project=vessel.

## Integration baseline before ADR 0061

Incoming node head is `9c7a45ca`, protocol 21. This clean-boundary merge is
in progress, not gated/committed yet. Keep the integrator's protocol-20
self/error metadata, usage/side-question changes and shared test corrections;
retain ADR 0060's fixture and evidence. The new M4 quality rule also splits
only the new launcher test helper at readiness; no assertion or deadline
changes. No ADR 0061 implementation has started. The next detached baseline
gate must include both fast scripts, A5 prebuilds and whole root tests in one
slot. Never repeat the earlier unchanged passing fixture gate.

Active baseline runner: `base-node21.sh`, with the Rust sequence in
`base-node21-slot.sh`, main log `base-node21.log`, final marker
`base-node21.exit`. The one shared-slot command is queued; do not restart
it or issue another Cargo command. Web lint/typecheck/417 units/build and
both new fast root scripts passed. `base-node21-inputs.json` pins 1,116
Rust/Cargo inputs; web assets are separately frozen for this merged source.
After root checks: unchanged TypeScript regeneration, feature launcher
checks, built real CLI, full page E2E and complete fake fleet. Final trap
stops only verified owned nodes and empties private SSD test scratch.

All merge conflicts are resolved/staged; MERGE_HEAD is the incoming node
head above, but the merge is not committed until its gate passes. Accepted
generated proto.ts is byte-identical to that head; git diff --check flags
its inherited generated whitespace. Do not hand-edit it. The page lint
explicitly excludes generated declarations. The build also emits its
existing future native-config warning, not a failure. V1's accepted fixture
runtime page evidence is `forge-feature3-page.log` (62 passed).

## Latest blocker: shared connector Clippy

`base-node21.sh` ended 101 at full Clippy: four large_futures diagnostics in
vessel-remote's connector tests at lines80/103/120/137, on the newly added
within wrappers. The file is byte-identical to incoming `9c7a45ca` (SHA256
f463dd6710b88fe61d44400aa65d154554534a401824c58791b4be829973db2f).
N3 does not own/edit those shared tests. The exact blocker went to V1 for
the coordinator/integrator route; no waiver, timeout change or unchanged
whole retry. Both A5 prebuilds passed (40.41 s and 2m06s), as did the fast
scripts and web checks. Root tests and later runtime stages were not reached.
Resource front/end one-minute load16.70/15.63, available17015/15418MiB;
all1,116 Rust/Cargo inputs unchanged. The merge is still uncommitted.

The independent `base-node21-owned.sh` check finished zero: feature Clippy,
all four fresh launcher tests (1.12 s), and both TS generators passed. The
accepted incoming generated declarations remained byte-identical; all 1,116
Rust/Cargo inputs stayed unchanged. Its dedicated cleanup record confirms
empty SSD scratch, UID/GID1000,0700, and no remaining process. No gate or
fleet remains active. No new listing implementation has begun. Await the
shared owner's correction before a new whole baseline gate; never edit the
shared connector file here or rerun the unchanged failed whole gate.

Evidence handling correction: the copied baseline cleanup helper retained
its old fixed output filename, overwriting the prior cleanup JSON. The new
actual record is preserved separately as `base-node21-ssd-cleanup.json`.
The prior record was reconstructed from its captured tool output, also read
by V1 before overwrite; `cleanup-record-recovery.json` documents provenance
and explicitly excludes original filesystem mtime attestation. Root logs,
source pins and final audit were unaffected. Future helpers use unique output
names. Actual SSD scratch is empty, UID/GID1000,0700, with no process left.

## Owner correction and corrected baseline

The owner supplied `a365d29575ff44c096b352cb678151e07cccb45e`, sole parent
`9c7a45ca`. Import only its connector.rs patch into the held merge; the file
is byte-identical to that object (SHA621496262692054e7fcbcf787cfe3214317aebfff5682941bebac93319040062).
Four async bodies are Box::pin arguments to the existing within; LIMIT60,
assertions and lints are preserved. `base-node21-owner-delta.json` proves
this is the sole Rust input change since the passing owned-helper checks.
Those four launcher/TS checks and web417 results remain valid and are not
repeated. V1 independently source-cleared the owner patch, not a whole gate.

`base-node21b.sh` is detached: fmt/size/boundaries, both A5 prebuilds and whole
Clippy/tests in one slot, final real binaries, then the previously unreached
full page E2E and CLI fleet. It reuses the unchanged frozen web bundle.
Final cleanup writes only its unique base-node21b-ssd-cleanup.json file.
All failures and original source pins are retained. The merge remains
uncommitted until the corrected baseline passes. No new listing code yet.

## Current priority: fixture gate/CI follow-up

The coordinator identified required-feature coverage missing from the default
workspace/CI. Its integrated source candidate is `4660f784` on `a365d295`;
it is published only after the owner's gate 21 succeeds. The queued N3
baseline was already admitted when cancellation was considered, so no signal
was sent. It ended at an own optional test-helper refactor's unused address
argument in default builds. That uncommitted optional refactor was removed,
restoring fixture tests exactly to the V1-passed source and candidate466.
No lint/expectation/deadline was changed. Crates/Cargo now match466 exactly.

The small CI follow-up is prepared separately on branch
`backend/m12-forge-gate`, worktree sibling `vessel-m12-forge-gate`, base466,
so it cannot include the original branch's native ancestry or held merge.
It adds a serial public root runner, both explicit forge-fixture CI commands,
and gate/README documentation. No product Rust/wire behavior changes. Active
runner is `forge-gate.sh`; its log/exit marker and source pins use forge-gate
prefixes in scratch. It is queued in one shared slot, uses SSD test data,
A5 prebuilds and the same environment. The commit is owed after validation
and integration confirmation; tell the coordinator its hash.

The old held merge remains uncommitted. Do not mistake its two failed
baseline runs for the CI follow-up's gate or for the earlier ADR0060 PASS.
A6 is read: until the shared required_protocol accessor lands, ship value
types/node logic only, no new registered call or second version mechanism.
Actual project-wide PR listing (ADR0061) remains unbuilt.
