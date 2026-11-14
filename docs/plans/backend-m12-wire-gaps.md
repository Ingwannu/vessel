# Track N3: wire gaps

Status: in progress. Each row is a node slice, reviewed separately.

## Row 1: machine time zone and schedule preview — contract

[Decision Log]
- Purpose: a schedule editor must show the target machine's zone and the
  same due times that its scheduler will save.
- Existing assets and constraints: ADR 0033 already decides cron, intervals,
  gaps and folds; `automation::time::Due` implements them on the node.
- Alternatives: repeat the calculation in clients, or move I/O and the zone
  database into the protocol crate. Both add a second policy or dependency.
- Choice: extend the existing node implementation with a read-only RPC.
- Why: one calculation, one set of DST rules, no session or saved schedule.
- Effects: protocol 13 adds the call (12 at first; renumbered when the
  node integrator merged it after track A2, which holds 12); protocol 2
  remains the minimum.

`NodeInfo.timezone` is an optional `TimeZoneName`, captured at start from
  the system's IANA name. Unknown stays absent; old nodes omit it and old
  clients ignore it. No fallback claim that the machine is in UTC.

`schedule.preview` (read, since 13) takes `cadence`, optional `after` and
`count: PreviewCount` (1–10, default 5). It returns the effective `cadence`, `after` and `runs` (UTC
instants strictly after it). Without `after`, the node reads its clock once;
an interval without an anchor uses that instant. The caller reuses the
returned cadence in create/update, preserving the anchor across save. It calls the scheduler's
`Due`, on the blocking pool, with four node-wide concurrent previews at
most. The closure holds its permit until completion, even after RPC
cancellation or disconnect. Saturation refuses before enqueueing. Each search keeps ADR 0033's eight-year bound. It writes nothing.

Refusals: `invalid_request` with typed `SchedulePreviewError` data for an
unknown zone, unknown cadence, out-of-range time, or no future run; `unavailable` when the
preview slots are held. No partial list, including when the tenth run goes beyond the supported
time range. Chrono instants outside jiff's range are refused explicitly. Read-only devices and agents may
call; a delegated caller retains the effective read scope. Old peers refuse
this method by its declared version before execution. Welcome and node.info
for negotiated versions below 13 omit the timezone field; a new client
decodes older nodes without it. Tests cover old peers, read/no-read device
scopes, agents and delegated effective read scope.

`NodeClient::preview_schedule` and `vessel schedule preview` expose it.
The CLI's omitted `--tz` uses its own machine's zone, matching existing
create/edit semantics; remote callers pass the node's reported zone explicitly.
An unknown local zone requires an explicit `--tz`.

References: Paseo `packages/server/src/server/schedule/cron.ts` and its
`cron.test.ts`, schedule service and CLI schedule commands. Vessel retains
ADR 0033's intentional gap/fold semantics rather than changing scheduling
policy to match a different implementation.

## Checks and progress

Dependencies: `thiserror` (already pinned by the workspace, MIT/Apache-2.0)
provides the typed preview error without manual Display duplication.

Implementation built; review pending. Tests include reference UTC/day-field
cases, explicit anchors before/after the cursor, nanoseconds, gap/fold policy,
unknown/no-match/time-range reasons, count 0/11, cancellation and saturation.

Validation on Linux, running as the normal user:
- Whole `cargo test --workspace`: 105 suites, 1,648 passed, zero failed,
  nine existing ignored measurements/limits; node unit 191 and node E2E 146.
  No failing test was reclassified as a load exception in this clean run.
- TypeScript generation with both libraries' tests: protocol 159 and web 54
  passed. The generated declarations are committed with the Rust contract.
- Page lint and typecheck passed; unit tests 55 files / 243 cases passed;
  production bundle build passed; final E2E 36 passed, including the unchanged
  20,000-entry timeline in 32.5 seconds. The failed runs below are retained.
- Fake fleet checked node Asia/Seoul, CLI UTC default, ten exact instants
  and impossible-expression refusal. It uses only the testkit agents.
  Data directory 0700 and token/node-book files 0600, owned by the runtime
  user. All fixture processes were stopped. A postcheck first used the
  wrong node-book filename; the correct TOML file's mode was then checked.
- Final whole fmt and clippy passed. The final clippy, after strengthening
  the cached-home assertion's request, waited 40 minutes 15 seconds for a
  machine build slot and completed in 2 minutes 38 seconds; it was neither
  restarted nor bypassed. The unchanged whole workspace test was not rerun.

No preview service or real agent is used. Windows/macOS runtime zone detection
and the native editor are not claimed verified; the app's integration and
captures belong to G2. The existing scheduler's gap/fold policy stays unchanged.

### Problems encountered during implementation

- The initial compile found that the protocol crate did not yet depend on
  `thiserror`; the workspace-pinned dependency was added explicitly.
- The next compile found the RPC client's exhaustive timeout classification
  and unary request table; both now classify the preview as a normal read.
- The first TypeScript generator run found an incorrectly placed alias in
  its sorted classification table. `PreviewCount` is now before
  `ProcessName`; the generating test enforces this ordering.
- `npm ci` replaces a node_modules symlink. After installation the generated
  dependencies were moved to the scratch disk and the symlink restored.
  The directory must retain the basename `node_modules` for Node's package
  lookup; renaming that directory broke ESLint's dependency resolution and
  was corrected without changing installed packages.
  No package or system configuration was changed.

- Adding timezone increased the web registry's internal change enum beyond
  its size lint. Its status payload is now boxed; the page's serialized
  shape and behaviour are unchanged. The gate verifies the shared seam.

- The first whole-workspace run stopped at a new preview boundary fixture:
  188 node unit tests passed, one failed (`TimeOutOfRange` versus the
  fixture's `NoFutureRun`). Jiff reserves room for all UTC offsets; the
  last civil day of year 9999 is already outside its timestamp range.
  That refusal remains tested with its proper typed reason, and a separate
  test starts five minutes before `Timestamp::MAX` to check no partial list
  for both cron and intervals. This was a fixture error, not a load failure.

- The next workspace run passed all four new real-node preview tests and
  all six preview decision/concurrency tests, then stopped in protocol
  tests (152 passed, two failed). The exact forwarding allowlist needed
  the intentional new read call. The future-error test also exposed that
  an adjacently tagged unit fallback rejected an unknown kind's payload.
  `ErrorDetail` now uses an internally tagged struct variant, preserving
  the JSON shape while ignoring unknown kinds and their extra fields.
  Its Rust form is `ErrorDetail::SchedulePreview { reason }`.

- The following whole run reached node E2E: 145 passed and one failed,
  `grants::stopping_a_member_does_not_wait_for_its_grant`. This was not
  classified as a known load-only failure: after cancellation of an in-flight
  grant the member admitted its first pull, and HDD apply work exceeded the
  existing five-second shutdown assertion. The narrow prerequisite checks
  cancellation after the grant, after recording its result, and at every new
  pull admission (including simultaneous watch/timer readiness). It clears
  the cached home on those exits. It does not cancel or roll back an apply
  already started. The real-node regression retains five seconds and checks
  no first AGENTS.md was written; an authenticated in-memory boundary test
  checks no generation was applied and the cached home is unavailable.
  This prerequisite is separate from the preview contract and is included
  because the whole workspace gate exposed a real lifecycle bug.

- The CLI preview retains the existing create/edit default zone (the CLI
  machine). Selecting the remote machine's zone only for preview would make
  the same omitted-zone expression change meaning when saved. The native
  editor can explicitly choose the independently reported machine zone.

- The page E2E gate finished 34 passed, two failed. The Models failure's
  captured page was the unauthenticated launch instruction: `openUi` waited
  for the shell (which renders before authentication), then a full navigation
  discarded the in-flight bootstrap before its replacement reached tab
  storage. The helper now waits for the replacement's existence without
  exposing it. The theme failure compared the served index with the page's
  optimistic DOM paint before the bridge acknowledged persistence. The
  theme checks now wait for the actual server-painted index, including
  before slow-link first-paint capture and cleanup. No timeout, product
  permission, assertion value or first-frame condition was loosened.
  These are test coordination prerequisites, independent of the wire change.

- In the next page run, the original Models and theme checks passed. The
  sidebar-shortcut test then failed at its first hide key (the sidebar
  remained visible), before the palette assertion. It redundantly reloaded
  the already-ready board and sent a key as soon as the new shell painted,
  before its shortcut effects were installed. The test now uses `openUi`'s
  authenticated board directly; the hide/open/Escape/restore assertions and
  deadlines are unchanged. This removes a navigation unrelated to the
  behaviour under test rather than retrying a missed key.

- The second page run ended 34 passed / two failed: the corrected original
  two cases passed, the sidebar initialization case above failed, and the
  20,000-entry timeline reached its existing 15-minute deadline. Its final
  capture showed both turns ended and the find panel absent, while the
  palette advertised only `/`: the earlier shortcut reassignment persisted
  into this independent test. Reset-all had checked only optimistic DOM
  paint and closed its UI process before the reset was acknowledged. The
  test helper now reuses the bridge's typed frame decoder to retain one
  preference acknowledgement per page (never frames or capabilities), and
  dependent changes plus cleanup wait for that receipt. This also covers
  sidebar, send-key and grouping cleanup. No timeline assertion or timeout
  is changed; the same Ctrl+F/default-key test still has to pass.

## Row 2: fleet controls and applied state

Numbered 15 when the node integrator merged it on `backend/m12-node`
(this branch declares 12 and asked for preview's 13): 13 and 14 were
already published there without these calls, and a call's version is the
first one whose nodes have it, so a newer client refuses them locally on
an older node. The text below is as built on this branch.

Decision: extend the current join/leave implementation and existing serial
publisher/member loops (ADR 0044, written before the code), with Admin
mutations and a credential-free Read projection. Paseo's ConnectedHosts
source governs actor authority, persistence before acknowledgement and
summaries without connection secrets; fleet publication/revocation remains
ADR 0012/0013. The accepted sync request is distinct from completed apply.
Implemented in `2ced8723` with runtime fixture followup `caaea148`. The final
root gate passed (105 suites, 1,666 passed, zero failed, nine ignored), as did
TS generation, the page gate and real fake-fleet checks. V1 review and
protocol-13 integration remain pending; no native action UI is approved.

Exact contract: root `JoinFleet { link: PairingLink, grant: Vec<Scope> }`
returns `JoinedFleet { home: NodeInfo }`; `LeaveFleet {}` and `SyncFleet {}`
return Ack; `GetFleetStatus {}` returns `FleetSyncStatus { role, local,
members? }`. `FleetRole` is standalone/home/member (home id, join time), plus
unknown. `FleetApplied` carries optional generation/applied_at/contacted_at,
bundle-relative conflicts and typed `FleetSyncProblem` categories. All
control refusals have `ErrorDetail::FleetControl { reason }`. Client methods
are `join_fleet`, `leave_fleet`, `sync_fleet`, `fleet_status`. Membership
mutations require Admin, are not forwardable, and refuse agent/delegated
principals; Read status omits members unless the home's local connection is
owner or paired Admin. No secrets, absolute destination paths or file errors
are projected. Sync accepts one coalesced wake, not an apply receipt.

The Tokio mutation permit spans preflight, network pairing and commit. The
OS try-lock is held only during blocking preflight and commit, distinct from
the apply lock and shared with init. Commit rechecks the role; a racing
offline operation can consume an invitation but cannot replace membership.
Post-redemption admission/write/task failures are typed `join_uncertain`;
inspect status before creating another invitation, never automatically retry.
Blocking closures own their permits through RPC cancellation. Their wakeup
survives loss of the RPC. Partial leave still wakes and reconciles the former
home's device before returning failure; a retry removes an orphan credential.
Source and target materialization
stay in the existing engines. Explicit manual sync can wake retry waits;
automatic retry backoff is unchanged. A leave does not roll back an admitted
apply. The fake fleet keeps the Settings behind/conflict fixtures, adds a
real member with a long name, exercises join/leave/sync, and creates a real
Read-only credential with `node pair --read-only` to check identity omission
and mutation refusal. No screen changes or native captures are claimed.

Status reads cap state JSON at 1 MiB, membership JSON at 64 KiB, local
conflict entries at 256 and aggregate reply conflicts at 512; BundlePath is
already limited to 1,024 bytes. Admin reports use an indexed 128-record
device window, checking its explicit `more` before building a result. They
inspect only those reports and check counts before cloning; no whole report
map snapshot or complete device list is made by status. Serialization into a
nonallocating sink caps payload below 1 MiB with 1,024 bytes reserved for the
RPC envelope. Four read permits survive all blocking work. Exceeding any
budget returns typed `status_too_large` for the whole result; there is no
truncated list that could be mistaken for empty or complete. Last generation
is historical without current-home association; do not compare its bare
number with the current home's to label it up to date.

The first whole clippy waited 24 minutes 6 seconds for a slot, then exposed
missing method-table documentation and redundant Ack qualification. Both
are fixed without lint exceptions. Review followups prepared while queued
were installed with current modification times before compilation began;
the additional cap E2E was installed while only protocol checking had begun,
before node checking. No successful gate on an earlier source is claimed.

Row 1 implementation is `f81798c3`; the native app integration remains G2's.

Coordinator followup for row 1's shutdown prerequisite: the exact E2E binary
from the clean workspace gate ran the grant-stop regression eight consecutive
times; all eight passed. Existing five-second stop assertion and no-first-file
assertion were unchanged. Setup-inclusive durations were 22.42, 8.37, 4.30,
9.05, 13.61, 9.26, 15.82 and 20.78 seconds. Already-built current fake binaries
were used, with Cargo unavailable to the runtime locator, so no rebuild or
unchanged whole-gate rerun was charged to those eight checks.

## Row 2 final verification

Final source: `caaea148` atop `2ced8723`, `cc972008`, `a32a0ac6`; documentation
checkpoints include `73332e50` and `bcc9106c`. Whole fmt/clippy/test passed,
with changed crates explicitly recompiled: 105 test suites, 1,666 passed,
zero failed, nine ignored; node E2E 151 and node units 198. The literal
filtered TS generation command passed both generators and left declarations
unchanged. Page lint/typecheck, 243 units, build and 36 E2E passed.

The first TS-enabled whole run had four failures (1,669 passed, nine ignored):
the new lifecycle fixture queried before publication, two protocol authority/
version lists lacked the added methods, and an existing remote fallback test
hit its unchanged 60-second limit. The new fixtures were corrected without
weakening assertions or deadlines. All four pass in the final whole root run;
the earlier remote timeout remains unexplained, not waived as a load exception.

Four consecutive reinstalled-member checks and eight grant-stop checks passed
on the final whole run's exact node E2E executable, with CARGO unset and its
pre/post hashes equal. Real fake-fleet CLI checks cover join/leave/sync, long
member labels, applied reports, standalone refusal and read-only privacy/
refusals. UID/GID 1000, private directories 0700, credentials/books 0600; no
ownership or permission change. Owned fixture processes were stopped by PID.
No native screen or other-OS runtime claim is made. See the resume note for
log names, pending review and consumers. No new slice or branch merge followed
the credit-stop instruction.

## Row 3: self authority, contract before code

ADR 0044's 2026-11-14 amendment defines the query, authentication-only method
metadata, canonical scopes, identity privacy, old-hop version gate and CLI.
Provisionally protocol 19 after merging node-line `b981f5f2` (protocol 18).
The final report names the new call/fields so the integrator can renumber.
Existing fleet controls are already merged at 15, fleet.published at 16.

Baseline verification before new feature work: whole fmt/clippy passed;
110 suites, 1,771 Rust tests passed, zero failed, ten ignored. Page lint,
typecheck, 417 units, build and all 62 Playwright cases passed. Logs are
`resumed/base2-*` in this track's scratch folder. The first baseline attempt
passed 1,771 tests but failed compiling the web doctest because this worker
rebuilt the bundle during the Rust gate. The corrected execution first
froze all 165 built assets using the supported VESSEL_WEB_DIST override,
then ran the full gate without asset mutation. No failure was waived and no
product source changed to force a pass. No new feature code preceded the
clean baseline. Keep assets stable during every subsequent Rust gate.

Row 3 typed testkit authority fixtures enable an optional dependency on the
already-pinned workspace `vessel-proto`, only with `authority-fixtures`
(the node's dev-dependency). This keeps response fixtures typed without
hand-written wire strings, while ordinary fake binary builds keep their
existing dependency cost. It creates no cycle or external dependency.

## Row 3 implementation and verification (`3d486ff4`)

New request `GetDeviceSelf {}` (`device.self`) returns `DeviceSelf { device?,
scopes }`; `NodeClient::device_self()` and `vessel node devices self` use it.
Only a directly authenticated paired device gets its own id. Owner, agent
and delegated responses omit the id and expose the admitted principal's
canonical effective scopes, including an explicitly empty grant. Scopes are
required in the response; no unavailable-query fallback infers authority.
Ownership, agent limits and policy remain separate enforcement boundaries.

The generated method table now represents an additional scope as
`Option<Scope>`: only this method is explicitly Authenticated/None, with no
anonymous principal. All prior methods keep their Some(scope). Envelope
version admission includes the inner typed method and fields, so older hops
refuse before dropping a field or dispatching an unknown call. Direct and
forwarded old-peer fixtures cover the boundary.

Root fmt, whole clippy and literal whole workspace tests passed: 110 suites,
1,781 passed, zero failed, ten ignored, with RUST_TEST_THREADS=2. All three
new real-node E2E tests passed in that run. TS generation passed both
libraries; targeted protocol/RPC checks passed 182/34. Page lint, typecheck,
417 units and build passed; all 62 page E2E and the complete CLI fake fleet
passed. UID/GID 1000, 13 private paths checked at 0600/0700, both verified
fixture node PIDs stopped and absent. The resume checkpoint records logs.
Rust source hashes were unchanged throughout the full gate.

Earlier targeted node units, using default test concurrency, failed three
existing model proxy timer tests: a_full_proxy_makes_room_by_closing_tokenless_connections,
a_harness_that_stops_reading_ends_its_call, and
a_head_cut_off_by_its_deadline_is_answered_408. One isolated diagnostic per
test reproduced the first failure and passed the other two; the old baseline
binary passed the first in its isolated diagnostic. No cause is claimed from
that comparison. All three passed in the final whole workspace run. No
assertion, deadline or lint was weakened; these failures were not waived.

Shared native changes are fixture-only: private operator bootstrap book,
paired person app book, CLI-pipe pairing, Read-only and Sync-only self checks,
and an owner/agent/delegated response fixture. The fixture script sets umask
077 so newly created credential books stay private. No screen or production
permission change belongs to this slice. Protocol 19 is provisional; the
integrator assigns the final number, and the only new wire method is
`device.self` with the request/response above. MIN_PROTOCOL_VERSION is 2.

## Row 4 contract before code

Amend ADR 0044 first. Reuse Devices::make_invitation and effective advertised
Routes; refuse before minting when neither direct nor relay exists, including
an empty tracked override. Preserve explicitly supplied callback routes.
New typed detail: DeviceInvite { reason: DeviceInviteError::NoRoutes } on
ErrorCode::Conflict, with setup guidance; existing device.invite method and
client/CLI calls remain unchanged. See the ADR for rejected alternatives,
old-peer fallback and required fixtures. No native screen work is included.

Row 4 implementation shares route selection for public and tracked issuance;
no bind-address inference, network probing or relay configuration is added.
The client keeps the existing invite calls and typed remote error; its API
documents the new Conflict reason. A future/unknown detail remains readable,
and a protocol-4 connection to the updated node gets the same guidance.
Older server behavior itself is not changed by a new client's version.
Existing in-memory fleet and CLI offer-inspection tests now explicitly
advertise their fabricated route; no assertion or safety timeout is relaxed.
The fake fleet's no-route fixture uses only the fake binary PATH, scratch
home/data/config and a local ephemeral listener, and reaps its exact child.

The first whole row 4 run stopped at node E2E: 173 passed, two failed.
Both legacy fixtures issued an empty-route invitation and rewrote it to the
live listener afterward: callback's member-without-relay guidance case and
secure's frozen-direct-route keepalive case. Their preparation now advertises
an explicit placeholder, retaining the existing live-route replacement and
all assertions/timeouts. All other completed suites passed, including 215
node units and the new current/old-peer no-route cases. The source remained
frozen throughout that failed attempt; prepared fixes were installed with
current timestamps only after it exited. A changed-source full root gate
follows; no clean final result is claimed at this checkpoint.

## Row 4 final verification (`aea21c54`)

The changed-source fmt, whole Clippy and literal whole workspace tests passed:
110 suites, 1,786 passed, zero failed, ten ignored (RUST_TEST_THREADS=2).
Node units 215 and node E2E 175 passed, including both corrected legacy
fixtures. The two source files fixed after the first whole failure are the
only differing Rust inputs between attempts. All 1,022 final Rust inputs
remained unchanged during the passing gate; changed crates were recompiled.
Both TS generators, page lint/typecheck, 417 units and build passed; all
62 page E2E passed in 5.1 minutes. No page scenario was skipped. The complete
CLI fake fleet passed, including an isolated real no-route node's exact
setup guidance and empty stdout. Its child was reaped; both main node PIDs
were verified, signalled and absent, with no owned runtime left. UID/GID 1000,
14 private book/token/credential paths checked at 0600/0700. No production
permission change, native screen or other-OS runtime approval is claimed.
Logs and pending review are recorded in the current resume checkpoint.

Next priority is the file-seeded FakeForge fleet node, then actual project
PR listing (row 5), registry/version states (row 6), and hooks (row 7). The
fake backend must be test-only; shipped vesseld may honor no fake backend
switch or flag. Its ADR and seed contract are not decided by this slice.

## File-seeded forge fixture: contract before code

After row 4's passed source review, the next slice is the test-only forge
node described in ADR 0060. Reuse FakeForge and
ForgeSetup.custom. A separate feature-gated vessel-forge-fixture binary avoids
a node/testkit dependency cycle; product vesseld has no fake switch, even
when the feature is enabled. Version-1 JSON groups typed PullRequest values
by RepoRef, with explicit per-repo failures and refused merges. Bounds and
safe diagnostics are part of the decoder, and seeded fake operations remain
bounded. No new dependency edge, external crate or wire version is needed.
The executable is built/tested with its feature; the whole root gate and
real CLI fake fleet remain required. Only after merge does the blocked PR
screen track receive the exact command and seed format. Project-wide PR
listing is the following slice, not claimed by this one.

The coordinator assigned ADR 0060 to the fixture forge and ADR 0061 to the
following project-wide PR listing. ADR 0059 belongs to restart recovery.
