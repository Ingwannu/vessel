# Inbox and push: node-only resume

## Current node checkpoint

The three source corrections are committed in `b1487bee`: recursive fleet
protocol admission, stored archive/read-state inheritance when persistence
lags publication, and exact attempted-lease cleanup after HTTP 404/410. The
Models assertion and canary diagnostics were supplied by the shared-test
owner (`28e49d9b` and `7f87d41c`), consumed as `ddcc5166` and `d2e6e6a8`.
The canonical corrected-source root gate ran on `5342e173`; later documentation
updates do not change its 1,642 compiled inputs.

- Root: fmt and full clippy passed; 111 test summaries report 1,835 passed,
  zero failed and 10 existing ignored tests, with `RUST_TEST_THREADS=3`.
  All seven changed crates actually recompiled. The canary reached shutdown
  with a 3.440-second phase sum (Setup 0.537, MakeHome 0.102 seconds).
- Full web E2E: all 62 passed in 3.4 minutes, including page (10), Models
  (3) and Links (1). Lint, typecheck, 417 web unit tests and build passed
  on the identical web source. The feature's earlier TypeScript generator
  run passed both tests; this correction adds no DTOs and leaves the
  declaration bytes unchanged.
- SSD Links comparison: own `5342e173` passed three of three in 19.6
  seconds. Baseline `9c7a45ca` passed three of three in 18.6
  seconds (front/back load 14.29/12.61). The earlier canary baseline
  remains `7f87d41c`; the two comparisons are not relabelled as one run.

Runtime temporary data used private mode-0700 SSD directories; targets,
logs and bundles stayed on the data disk. Root test front/back one-minute
load was 22.31/16.15, full E2E 16.60/16.66; all gates kept uptime and memory
records. Sources and normal binary identities were pinned. Recorded fleet
processes and local build-prefix executables were gone before clearing their
own temporary directories. No test deadline, assertion or retry count changed.

The third standalone SSD canary is classified as saturation, not a pass or
a product defect: its front one-minute load was 33.69. The owner-set rule is
strict: either endpoint above 32 means saturation; any other timeout is an
owner defect; only a saturated test/crate may be repeated once below 24.
The HDD own Setup median is 21.228 seconds, not the slowest 31.449-second
run; SSD Setup median is 6.322 seconds. MakeHome medians are 5.935/3.742.
Queue/build/warmup time is separate from phase sums. The successful whole
gate began before the later fake/vesseld prebuild instruction, remained
unchanged, and ran the canonical nested builds. Future whole gates prebuild
`vessel-testkit --bins` and `vessel-node --bin vesseld` in the same environment
and slot before testing, so those builds do not contend inside test clocks.

The source review closed the three original returns. Final feature and
combined-line acceptance remain the reviewer's/integrator's decision. The
Store producer barrier proves durable append/mark ordering, not an
instrumented actor-persistence schedule. No native screen, folded client
model, OS notification/sound, live push service/phone or Windows ACL runtime
is claimed tested. The screen-facing node APIs below are ready for that
separate work; protocol numbers are assigned at merge (this branch used 19
provisionally, minimum 2; inbox schema migration 14 followed search's 13).
On the node line the inbox and push are protocol 22 and schema 16 (side
questions, `device.self` and usage took 19 to 21 and schemas 14 and 15).

## Scope and commits

The resumed scope is node/wire/store/client/CLI/testkit/generated declarations
and documentation. Native link/view/screens and desktop delivery belong to
the app workers; earlier native draft work is not a tested contract.

- `47a4bcb1`: original node/push WIP checkpoint, superseded by the following
  integration and acceptance validation.
- `146eee00`, `99af7444`: earlier contracts/resume checkpoints.
- `95a46c6c`: merged node line `b981f5f2` (protocol 18), preserving search,
  driver, fleet and other node work. This merge passed the whole root gate
  (1,814 passed, zero failed, 10 ignored; `RUST_TEST_THREADS=3`), both TS
  generators, web lint/typecheck/unit/build (81 files / 417 tests), and all
  62 Playwright E2E tests. Its commit body's phrase “10 ignored measurements”
  is inaccurate: three are measurements, six require the rust-analyzer/toolchain
  prerequisites, and one is the documented proxy-limit test.
- The acceptance follow-up adds true concurrent answer validation, exact CLI
  question answering, selected large-form inspection after pruning, stale tool
  snapshot invalidation and fresh-worker push restart validation. Its final
  gate is recorded below. Focused CLI
  inbox (3), renderer (2), node restart/push (1) and TS generator (2) tests
  passed; the declarations are unchanged. Fresh web lint/typecheck/build,
  417 unit tests and all 62 E2E tests also passed. The subsequent root gate
  passed fmt/clippy, then stopped in the unchanged fleet admission test;
  the diagnostic below records that prerequisite.

## Previous web failure

Upstream `59000e29` fixed the theme-paint race: `preferThrough` displays the
optimistic theme before its bridge operation finishes. The old test did one
HTTP read immediately; the merged test polls the served index for persistence.
The merged full E2E run passed all 62 tests, including that theme case. No web
page implementation was changed for this issue.

## Screen-facing contract

Protocol 19 was provisional; on the node line this is protocol 22. Minimum
is still 2. The seven new calls are `inbox.list`, `inbox.get`, `inbox.mark`,
`push.key`, `push.subscribe`, `push.unsubscribe`, `push.presence`; no fields
are added to older requests/responses. The inbox migration was 14 after
search's 13 on the track, and is schema 16 on the node line. Full DTO fields and limits live in `vessel-proto::{inbox,push}` and ADRs
0040–0041.

- Use `NodeClient::inbox(ListInbox { before })` for bounded pages plus exact
  whole-node counts; fold by node identity, not by a machine's mutable label.
- Use `inbox_entry(key)` for cheap source confirmation and
  `inbox_request(key)` only to expand one selected live form (15 MiB budget).
  Source pruning never changes that pending form's choices. Omitted tool
  details may be absent after log retention; never promise their recovery.
- Use `mark_inbox(key, read)` for shared read state. Read-only devices can mark
  but cannot answer; agent origins, including delegated/forwarded ones, cannot
  mark. Revocation removes access without undoing shared read state.
- Reuse session permission/question answer calls. The actor validates the
  current request with Operate scope and first-answer-wins semantics. Recheck
  source/connection/effective scope before sending and await projection removal
  after ACK. Never automatically retry an uncertain write.
- Paired Read devices own only their push subscription. `push_key`,
  `subscribe_push`, `unsubscribe_push` and `report_push_presence` are the
  client entry points. Report real activity; focus-only changes are Unchanged.
  Close the connection to remove its presence.

The CLI lists, shows one selected form, answers, marks read and marks unread.
It reuses the existing typed question/permission validation.

## Privacy and verification limits

The node's VAPID key stays in its owner-only data directory/file; no private
key travels over the wire or enters a diagnostic. The relay sees encrypted
traffic metadata; a push service learns generic attention timing and ciphertext
size, never request content. Fake HTTP delivery tests decrypt the exact generic
payload and verify VAPID. No real service, phone host, Windows ACL runtime,
OS notification, sound or native screen is claimed tested.

## Review and next work

Send the final gated commits and logs to V1 in one message; V1 forwards what
passes to the node integrator. Do not message the integrator or lead directly
unless replying. Follow V1 findings before any further slice. Native workers
consume the node calls above; they must not treat the old off-tree native drafts
as a reviewed model or screen. Existing C/S2/E2 integration ownership is with
those app workers under the resumed scope.

All cargo build/check/test commands use cargo-slot, one at a time, with gates
detached. Testkit builds its own fake binaries under the canonical test command.
Do not restore the historical native drafts, use a prebuilt-only test mode,
push, run real coding agents or touch preview processes/ports. The private local
resume note holds artifact paths and verified process identities.

## Earlier consuming-gate failure and resolved prerequisite

The acceptance follow-up root run stopped after 778 passed, one failed and
seven ignored tests across 35 completed binaries. Node unit tests were
225 passed / one failed: `fleet::mutation::tests::cancellation_keeps_membership_admission_until_blocking_work_ends`
received WouldBlock from the membership lock after reacquiring its semaphore.
The fleet source is identical to the integrated node-line head; later test
binaries were not reached, so this is not a full root success.

Disposable probes reproduce the lock mechanism with both an actual fork and
Rust's `File::try_clone`: closing one descriptor leaves the shared lock held
while another description reference remains; explicit `File::unlock` removes
that dependency. This does not identify the particular concurrent fork in the
failed gate. The fleet owner authorized a separate prerequisite on `9814cb0f`:
`ce228c10` changes only the reservation module and its tests, explicitly
unlocks before the file/permit are destroyed, and preserves the original
cancellation fixture exactly. Its full root gate passed 1,831/0/12 with
`RUST_TEST_THREADS=3`; the unchanged 1,054-file manifest matched that fixed
tree. Both lock tests passed. That validation belongs to the fixed node-line
tree, not this inbox consuming tree. The identical prerequisite is now applied
here for a separate full gate. No timing retry, deadline extension or weakened
assertion was added.

The frozen source manifest was unchanged through that run. All recorded root
and web-fleet process identities were gone after teardown. No live service or
native application was started. Preserve the failed run separately from the
next changed-tree gate; do not repeat a full gate on the unchanged tree.

## Effective-scope consumer handoff

N3's reviewed `3d486ff4` defines `NodeClient::device_self()` /
`GetDeviceSelf {}` -> `DeviceSelf { device: Option<DeviceId>, scopes: Vec<Scope> }`.
That separate commit is not merged into this checkpoint; consume it only on
the node integrator's gated, renumbered combined line. A missing or unavailable
call means unknown authority, never an inferred grant or a device-name lookup.
Only the direct paired caller receives its id; delegated results expose only
the effective intersection. The seven inbox/push calls remain Read-scoped;
that new authenticated self-inspection exception must not widen them.

## Corrected consuming-build isolation

A subsequent consuming run stopped at the new CLI large-form fixture: the
runtime fake did not contain that scenario. The isolated prerequisite and
older inbox checkout had shared one local output directory; source mtimes
made the standalone fake appear fresh after the other tree replaced it.
Canonical nested builds were used, but that shared artifact location made the
run invalid as final inbox evidence. Its 62 passing web E2E cases are retained
as diagnostic evidence, not the final consuming-tree gate.

The output directories are now separate. Refreshing the original source
mtimes changed no file content and forced canonical recompilation. The actual
runtime fake then contained the scenario and the focused CLI test passed,
including every field and option after pruning. The fixture now also refuses
an unexpected early turn end explicitly; its deadlines and assertions were
not relaxed. The final whole root gate and complete E2E run passed for this corrected
consuming build. No prebuilt-only mode, copied executable, skipped test or
cargo clean was used.

## Final consuming acceptance gate

The corrected consuming tree passed the exact whole root gate: `cargo fmt
--all`, `cargo clippy --workspace --all-targets -- -D warnings`, and
`cargo test --workspace`: **1,820 passed, zero failed, 10 default ignored**
across 110 result sections, with `RUST_TEST_THREADS=3`. The ignored cases are
three measurements, six language-server/toolchain prerequisites and one
documented proxy limit. No test or deadline was relaxed. This also corrects
the inaccurate “10 ignored measurements” wording in the old merge's body.

Both TypeScript generators passed and their declarations stayed unchanged
for this follow-up. Web lint/typecheck/build and 81 files / 417 unit tests
passed; the final full Playwright suite passed **62/62** with traces, video
and screenshots off. The prior theme-paint test also passed. The frozen
1,636-file source manifest remained unchanged through the final root and
E2E runs. The runtime fake was verified to contain the required large-form
scenario before E2E and after the whole CLI run.

The full run includes CLI approval/read/unread, invalid and accepted question
values, all 2,048 large-form choices after actual log/ring pruning, concurrent
first-answer-wins across two nodes/devices, actor recovery and backfill,
stale tool snapshot invalidation, and push privacy/presence/admission/restart
fixtures. The exact `ce228c10` reservation source is consumed here, with both
its original cancellation and new duplicate-description tests passing.

All 109 recorded root-process identities and five web-fleet identities were
gone after teardown; no executable from this track's build directory remained.
No live node, real coding agent, repository push, native app or virtual display
was used. No service-account or live file-permission change was made. Unix
private-key mode tests passed; Windows ACLs and real phone/push-service delivery
remain outside this evidence.

The implementation commit is `34986852`; the final handoff record below fixes
its exact hash and gate evidence. V1's inbox/push review remains pending; its separate
approval of the fleet prerequisite is not approval of this complete slice.

## Immutable handoff

- Inbox/push acceptance implementation: `34986852e42a344bc10a949d6da0028ea6cbc861`
  on `95a46c6cbce8c060cf83f4a23600a42da01e75fd`.
- Separately reviewed fleet prerequisite: `ce228c10be0466a5487415ecf70cea13cae95e2b`
  on `9814cb0f696fd619d1fa58157b88570d2182d8cf`. Its exact source is consumed
  in the inbox acceptance commit; no wholesale node-line merge was made during
  that slice.
- The final consuming source, not the earlier mixed-output run, passed
  1,820/0/10 with `RUST_TEST_THREADS=3`, both generators, web 417 and full
  E2E 62. The following handoff documentation changes no compiled source.
- Complete slice review goes to V1; V1 forwards accepted work to the node
  integrator. The prerequisite's independent PASS does not substitute for
  review of the inbox/push implementation. No further native work is assigned
  to this track in the resumed scope.

## Review correction batch

V1 returned implementation `34986852` / notes `27962002` despite the recorded
green commands. The original commits remain immutable. The follow-up fixes
three source boundaries together:

- Fleet envelopes recursively require the maximum of envelope and full inner
  call versions. Inbox calls cannot be sent over an 18 hop through a version
  6 envelope, even when forwarding to that welcomed node itself. Direct typed
  calls, call-on-self and all three envelope types have exact local refusal
  and no-frame tests. Forced raw frames check exact refusal before dispatch;
  old supported calls and supported inbox envelopes still reach the handler.
- Inbox insertion reads the stored session marks in its own append transaction:
  archived sessions cannot recreate entries; already-read delayed sources
  inherit the exclusive watermark and explicit unread semantics. Backfill
  reuses that rule. Ordered real SQLite commits and an async Store producer
  barrier cover delayed requests/final status, exact counts, lookup and both
  push candidate selection and final admission. This is the store boundary,
  not an instrumented actor-persistence scheduling test.
- A late 404/410 matches the attempted device, subscription and expiry. Explicit
  unsubscribe remains unconditional. Real loopback HTTP responses are held
  until same-capability renewal commits, then completion and exact stored
  expiry prove whether that lease survived or the current attempt was removed.

The correction changes no wire shape, scope, schema or protocol number. The
shared recursive rule is the same one used by the other node tracks; it does
not integrate their authority methods or scope representation. New validation
is recorded only after the changed-tree gate finishes; the old 1,820/62 results
are evidence for the returned tree, not for this correction.

## Correction validation checkpoint

The correction's whole formatter and whole workspace/all-target clippy check
pass. The whole test run stopped in the unchanged CLI canary's 60-second
safety net: seven completed binaries, 182 passed, one failed, none ignored;
CLI integration was 40 passed / one failed. All three inbox CLI cases passed,
including full selected-form recovery after pruning. The canary source is
identical to the pre-correction and integrated node-line versions. A timeout
alone does not establish whether the cause is load or a blocked operation.
The proposed static phase diagnostics remain outside the repository while the
file's owner decides; no deadline, assertion or non-vacuity check was changed.

Fresh-binary web E2E completed 60 passed / two failed. Every page/theme case,
including the earlier theme-paint failure, passed. The terminal-link case
stopped in session creation before its link assertions (the URL did not change;
the terminal button was disabled); its cause remains unresolved. The model-call
case received a valid `1.0 s` duration while its assertion accepts only `ms`.
The production duration function and its existing unit tests explicitly support
seconds and minutes. Directly invoking that unchanged function confirms the
boundary at 1,000 ms. This is evidence of the assertion's unit mismatch, not
proof that every failure is unrelated to the inbox change.

All web source and these pre-existing test files remain unchanged. Runtime
binary identities stayed stable through E2E; the fake fleet was stopped and its
state removed. No native screen, real agent or hosted push service was used.
The slice remains returned pending owner decisions, a complete changed-tree
root gate, full E2E and renewed immutable review. Neither the earlier green
commands nor these partial results are a correction PASS.

The new direct push refusal case, which the stopped whole run had not reached,
was subsequently executed once on this same frozen source and passed. The
checkpoint is deliberately `wip:`: owner decisions and complete validation
remain outstanding, and the proposed canary diagnostics are not installed.

## Owner diagnostics and three-per-tree comparison

Own source: `d2e6e6a8b13ae9f0e60e0ad66d15e1090dedba86`.
Baseline: node-line HEAD frozen at `7f87d41cb388a0f3de0bb708a9b6c6c662ffc7ae`.
Both use exactly the owner's canary diagnostic bytes
`dec70323fec15e73949922da0bd97e4c54151dd10acc27c2281a2912c49d95f9`.
Order alternates own/base three times. Each command executes exactly the one
canary, with the existing 60-second safety net and assertions unchanged.
`uptime` and `free -m` are recorded before and after every invocation. Both
use the same temporary node-data filesystem (HDD); build targets are separate.
The first baseline build is cold. Phase clocks exclude compilation, fake build
warmup and slot waiting. No extra retry was made.

Both trees pass two runs and time out once. Their median phase sums are
52,201 ms (own) and 54,145 ms (baseline). The two failures occur in JoinMember
after Setup and MakeHome have consumed most of the safety net; neither failed
run reaches shutdown. The measurements do not show an own-only slow phase.
They do not establish the underlying reason for the startup/home variability,
and do not replace the whole-root failure or provide a slice PASS.

## All phases (milliseconds)

| phase | own-1 | own-2 | own-3 | baseline-1 | baseline-2 | baseline-3 |
|---|---|---|---|---|---|---|
| Setup | 18497 | 21228 | 31449 | 21154 | 39545 | 14041 |
| MakeHome | 899 | 5935 | 19990 | 617 | 17668 | 7087 |
| JoinMember | 8099 | 6769 | 8675* | 6990 | 2790* | 3300 |
| JoinPlain | 8317 | 2892 | — | 10325 | — | 2207 |
| JoinRouteless | 6910 | 2356 | — | 6754 | — | 1770 |
| CopySecrets | 682 | 481 | — | 1211 | — | 602 |
| Profiles | 264 | 289 | — | 135 | — | 455 |
| Sessions | 2272 | 2904 | — | 414 | — | 3485 |
| AgentTools | 52 | 624 | — | 54 | — | 403 |
| Transfers | 1868 | 1374 | — | 3744 | — | 865 |
| SessionLogs | 128 | 363 | — | 142 | — | 296 |
| Credentials | 0 | 0 | — | 0 | — | 0 |
| Scan | 5 | 6 | — | 4 | — | 4 |
| AssertClean | 574 | 624 | — | 522 | — | 572 |
| StopHome | 705 | 513 | — | 1024 | — | 919 |
| StopMember | 859 | 1613 | — | 528 | — | 512 |
| StopPlain | 1259 | 1618 | — | 368 | — | 413 |
| StopRouteless | 810 | 1511 | — | 159 | — | 338 |
| StopRelay | 1 | 1 | — | 0 | — | 1 |
| total ms | 52201 | 51101 | 60114 | 54145 | 60003 | 37270 |
| exit | 0 | 0 | 101 | 0 | 101 | 0 |

`*` is the final partial phase reported when the timed-out future was dropped.
A Drop line also occurs on success; it alone is not evidence of a failure.
Setup includes relay and four node starts. `—` means the phase was not reached.

## Recorded load and planned outcomes

| run | result | phase sum (ms) | load before | load after | slot wait (s) |
|---|---|---:|---|---|---:|
| own-1 | pass | 52201 | 22.81, 25.16, 25.11 | 24.61, 25.32, 25.30 | 112 |
| own-2 | pass | 51101 | 20.84, 20.39, 22.38 | 17.94, 17.95, 20.35 | 364 |
| own-3 | timeout | 60114 | 23.34, 25.71, 24.08 | 25.32, 22.43, 22.64 | 562 |
| baseline-1 | pass | 54145 | 24.61, 25.32, 25.30 | 20.84, 20.39, 22.38 | 276 |
| baseline-2 | timeout | 60003 | 17.94, 17.95, 20.35 | 23.34, 25.71, 24.08 | 738 |
| baseline-3 | pass | 37270 | 25.32, 22.43, 22.64 | 24.92, 23.66, 23.28 | 480 |
All six invocations ran the selected test. Both source manifests stayed
unchanged and all processes from the two comparison targets were gone at the
end. The common-test owner can now review the aggregate setup/home budget as
requested. The terminal-link three-per-tree comparison remains next; no new
whole gate has been run after this table.

## SSD temporary-data confirmation

The corrected node consumed only the two shared-test owner corrections: the
Models duration assertion (milliseconds, decimal seconds and minutes/seconds)
and the canary's fixed-name phase diagnostics. Product formatters, test
assertions, safety-net deadlines and retries are unchanged.

Three exact canary executions used a private mode-0700 SSD temporary directory;
build targets and logs stayed on the data disk. This is a new execution
condition, distinct from the earlier six HDD comparisons.

| Run | Setup ms | MakeHome ms | Phase sum ms | Result |
|---|---:|---:|---:|---|
| SSD own 1 | 502 | 151 | 3151 | pass |
| SSD own 2 | 6322 | 3742 | 19712 | pass |
| SSD own 3 | 20113 | 21146 | 59955 | 60-second timeout |

The third sum includes a partial AgentTools phase; later phases, including
shutdown, were not reached. The first two runs became substantially faster,
but the third failed while six real workspace builds saturated the machine.
The startup and home-creation medians were 6.322 and 3.742 seconds. These
results support using SSD runtime data; they do not prove that temporary-data
placement alone removes the common test failure. Build/queue/warmup time is
separate from the phase sums.

All three logs retain load and memory records before and after. The third
front record preceded a long FIFO wait, so it does not measure load at test
start. No test was retried. The recorded test processes and local build-prefix
executables were gone before clearing this track's temporary data.

Next: the one authorized canonical whole root gate with SSD temporary data
and load records, then the six SSD Links comparisons and the full web E2E
gate. The implementation remains returned for source corrections until the
corrected immutable tree and its complete evidence are reviewed. No node
slice pass or native-screen proof is claimed here.
