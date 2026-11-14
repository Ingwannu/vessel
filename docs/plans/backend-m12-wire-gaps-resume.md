# N3 current checkpoint

## Ready and reviewed

Row 3: contract `5cff884e`, implementation `3d486ff4`, evidence `5d10d4dd`.
V1 passed the source and evidence with no blocking finding and forwarded the
exact contract to the coordinator and waiting Devices/Sync/inbox tracks.
Whole root: 110 suites, 1,781 passed, zero failed, ten ignored. Page units
417, full page E2E 62, complete CLI fake fleet passed. No native screen or
other-OS runtime approval is claimed.

Row 1 is integrated at protocol 13 (`f81798c3`, merge `187dc847`): node time
zone and scheduler-backed preview. Row 2 is integrated at 15 (final source
`caaea148`, merge `ff2981f2`): fleet join/leave/sync/status. Publication is 16.
Their detailed evidence remains in the implementation plan and git history.

## Row 4: complete, ready for review

Contract-first ADR amendment: `41518e9c`. Gated implementation: `aea21c54`.
V1 passed the node source/evidence and forwarded the exact refusal.
The changed-source final root gate passed: 110 suites, 1,786 passed, zero
failed, ten ignored, RUST_TEST_THREADS=2. Both TS generators and page lint,
typecheck, 417 units and build passed. Final binary build, all 62 page E2E
and the complete CLI fake fleet passed in `row4-final2.sh`; its exit marker is zero. No new slice has started.

The initial resumed baseline was fast-forwarded to `b981f5f2`, protocol 18,
and gated before feature code: 110/1771/0/10, page 417/62. Read-only inspection
later found node branch `18d86c2a` at protocol 19 (side questions); it was not
merged into the frozen row 4 tree. This branch's 19 is provisional: the
coordinator/integrator must renumber it and run their combined gate.
MIN_PROTOCOL_VERSION remains 2.

## Exact contracts

- Row 3: `GetDeviceSelf {}` / `device.self` -> `DeviceSelf { device?:
  DeviceId, scopes: Vec<Scope> }`; `NodeClient::device_self()`;
  `vessel node devices self`. The request rejects unknown selectors; scopes
  are required and canonical. Only a direct paired device gets its own id;
  owner, agent and delegated callers omit it. The admitted principal is the
  guard's source, including clipping/intersection and empty/no-Read grants.
  Ownership, agent limits and profile policy remain separate. Unsupported
  means unknown authority, never an inferred grant. Method metadata now
  expresses Authenticated/None explicitly; every legacy method retains its
  Some(scope). Typed envelope admission includes the inner call's version.
- Row 4 adds no call or request field. Existing `device.invite`, client
  invite methods and `vessel node pair` retain their Admin requirement.
  `ErrorDetail::DeviceInvite { reason: DeviceInviteError::NoRoutes }` with
  Conflict says to configure a reachable --public address or --relay before
  creating another invitation. DeviceInviteError also has Unknown. Effective
  public/callback routes are checked before randomness, encoding or pending
  admission; an explicit empty override refuses, not falls back. No bind
  inference, probe or automatic relay activation. Configured routes are not
  remote reachability proof. Older clients keep the human guidance; older
  servers themselves retain their previous behavior. Non-Admin still gets
  Forbidden without configuration detail. CLI refusal prints no link or QR.

## Evidence and failures

Logs are in the track scratch directory under `resumed/`:

- Baseline `base2-*`; row 3 `row3-final-*`, `row3-typescript.log`,
  `row3-web-*`, `row3-permissions-cleanup.json`, compilation/input evidence.
- Row 4 final root: `row4-final2-{fmt,clippy,test}.log`, root exit zero,
  `row4-final2-root-summary.json`: 110/1786/0/10, unchanged Rust inputs.
- `row4-typescript.log`; `row4-web-{lint,types,test,build}.log` are the
  unchanged passing TS/page inputs. `row4-final2-{build,e2e,fleet}.log`
  are the passing runtime stages. `row4-final2.exit` is zero.
- `row4-final2-inputs.json`, `row4-frozen-check.json`, and
  `row4-input-changes-between-gates.json` identify the fixed sources.
- First row 4 whole attempt: `row4-final-test.log` and
  `row4-first-root-summary.json`, stopped after 37 suites, 938/2/7;
  node E2E 173/2. Callback's member-without-relay and secure's frozen-direct
  keepalive fixtures issued an empty-route invitation then appended a live
  address afterward. Both now advertise a placeholder explicitly and retain
  their existing live-listener replacement. Both passed in final whole node
  E2E (175/0), with node units 215/0. The failed run remained frozen; fixes
  were installed with current timestamps after exit. No assertion or timeout
  was weakened. These two fixture files are the only changed Rust inputs
  between the two whole attempts.
- Targeted row 4 clippy first required explicit RouteUrl annotations in its
  new unit test; corrected. The first CLI whole-stderr expectation omitted
  the existing error wrapper; corrected the expected sentence only. Final
  targeted route unit, real-node E2E two, CLI three and error decoder one
  passed before the whole gate.
- Row 3 earlier model timer failures and the baseline asset-sequencing
  mistake are recorded in the implementation plan. No failure was waived.
  Keep the embedded page assets immutable through every Rust gate.

## Permissions, processes and remaining review

Row 3 verified UID/GID 1000, 13 private book/token/credential paths at
0600/0700, both owned fleet node PIDs stopped and absent. Fixture umask 077
was added there; production permissions were unchanged. Row 4's final audit (`row4-permissions-cleanup.json`) verified UID/GID 1000
and 14 private paths at 0600/0700. Its extra no-route child was reaped through
Popen; both main fleet node PIDs were verified by UID, executable and data
directory before signalling and are absent. No owned runtime remains. All
1,022 frozen Rust inputs are unchanged. No production permissions changed.

Submit the gated row 4 commit/evidence to V1 only. It forwards accepted work;
do not message the integrator or lead directly unless answering them. No
native/page screen work or other-OS runtime claim belongs to this slice.
Keep the two local Cargo configs, untracked web build/dependency links, the
project label, cargo slots, detached gates and private scratch homes. Never
prompt a real agent, pair with the preview, push, expose a secret or stop a
process by name. Test-written executables use Sandbox::executable.

## Next, in order

1. File-seeded FakeForge node for test fleets, to unblock PR-screen captures.
   ADR first: repositories, pull requests/checks/reviews/mergeable states,
   no-access/rate-limit/refused-merge failures, startup and deliberate limits.
   Testkit binary or fakes feature only: shipped vesseld must honor no fake
   forge environment switch or flag. Test real FakeForge behavior, whole
   root gate, V1, integrator; give the blocked PR-screen track the exact
   command and seed format after merge. Contract first recorded in `39df0f9c`, moved to assigned ADR 0060 before
   implementation. Project-wide PR listing will use ADR 0061.
2. Actual row 5, project PRs without a live session. The earlier resumed
   instruction's claim that it was already integrated at 13 was withdrawn:
   both b981f5f2 and later 18d86c2a still have Read/11 unit pr.list and
   forge/watcher.rs::list maps only watched sessions. Protocol 13 was row 1.
   V1 received the correction. Check the then-current line before coding.
3. Row 6 registry flag/environment and reachable too-old/too-new agent states.
4. Row 7 hook calls/listener/CLI: read ADR 0034 whole and settle ADR 0043's
   bind/exposure, signatures, replay, limits and action authority before code.
