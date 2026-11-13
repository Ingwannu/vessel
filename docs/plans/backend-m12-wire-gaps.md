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
