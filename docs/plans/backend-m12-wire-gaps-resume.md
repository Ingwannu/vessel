# N3 resume after the overnight credit stop

The coordinator's stop instruction limits this session to row 2. Start no
new slice or branch merge until the next session explicitly resumes work.

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

## Current row 2, awaiting gate and review

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
the source commits and renumbering preview/timezone/old-peer fixtures. Its
gate was still running when reported. This N3 branch still declares 12;
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
