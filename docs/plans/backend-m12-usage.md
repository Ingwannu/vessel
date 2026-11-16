# Milestone 12: usage and limits (N2)

## Before implementation

Decision: **modify and reuse** ACP `TurnUsage`/`ContextUsage`, the store's
atomic append, the idle reader cache and the existing scope guard.
Usage scans need their own cancellation-safe admission limit; the idle
cache is not a bound on queued or running jobs.
There is no aggregate query yet. SQLite and chrono already cover the
storage and time requirements; no new external service is needed for
reported usage. ADR 0042 owns the additional credential boundary.

## Reference acceptance list

1. Open a conversation. Read the turn's input, output, cache and reasoning
   tokens when reported; inspect context separately. Read the session
   total and reported cost without inventing values that were not sent.
2. Open Usage. Choose a machine and a rolling day or week; compare totals
   by agent/profile. No reports is different from zero tokens. See the
   accounting start and any incomplete retention range.
3. Inspect an agent's account. See every reported window separately,
   its usage percentage, when it resets, and the last successful refresh.
   Unknown period/reset stays unknown. Refresh one account. A failure
   keeps the last successful numbers visibly stale, with retry.
4. While external lookup is off, see protocol reports and an administrator
   opt-in row explaining the saved sign-in read and its vendor destination.
   A viewer cannot enable it. Missing sign-in explains how to sign in.
5. Hit a limit. See which window prevents work and when it resets. If all
   blocking resets are known, choose resume at reset or cancel that choice.
   Hand off to another available signed-in profile through the existing
   checked move. Never draw an inert fallback or resume action.
6. Check settings and session surfaces in English/Korean, dark/light,
   wide/phone, with long names, many windows, no data, errors and a viewer.

Sources followed (public repository-relative paths):

- Paseo `packages/app/src/usage/card.tsx`, `window-bar.tsx`,
  `reset-line.tsx`, `usage-section.tsx`, `host-usage-section.tsx`,
  `agent-usage.tsx`: provider header, status, windows, freshness and errors.
- Paseo `plugins/claude-usage-source/server/usage.ts` and
  `plugins/codex-usage-source/server/usage.ts`: local account routing and
  vendor shapes; `packages/server/src/server/session/usage/usage-session.ts`.
- GOAT `apps/server/src/provider/claudeUsageLimits.ts`,
  `codexUsageLimits.ts`, `apps/web/src/components/chat/UsageLimitRecoveryBanner.tsx`
  and `docs/user/usage.md`: controls, stream snapshots and recovery.

## Contract

The first node slice adds `usage.query` (read, protocol 21 at the node
line's merge; proposed as 19 after the protocol-18 node line). It takes a
closed period (day/week/session), a closed scope (machine, agent/profile,
session), and an optional session sequence cursor. Machine/agent requests
must use day or week. Session period includes retained samples only.
The reply holds token totals with count of reporting turns, costs grouped
by unit only when permitted, per-agent totals, and a bounded turn page.
Typed `invalid_request` rejects mismatched periods; `forbidden` rejects
agent machine/account access; `not_found` rejects unknown sessions.
Pagination and accounting coverage are explicit, never silent truncation.

## Progress and checks

Node accounting slice implemented and checked on Linux. It uses
`rust_decimal` (MIT) for exact input/range validation and `num-bigint`
(MIT OR Apache-2.0) for lossless integer differences and sums; values outside
its range stay explicit gaps. Reported zero costs and independent currency
baselines are preserved. Token overflow is flagged. This first slice does
not yet collect account limits, schedule recovery, draw a Usage page or
include the proxy's separately reported token source.

Real sign-in, vendor endpoints and live recovery are unverified by design. Review is per node/app slice; passing unit tests
alone is not screen parity or live-account proof.

### Implementation findings

The first whole-workspace clippy runs exposed the protocol's deliberately
exhaustive consumers (web bridge, relay forwarding, client and RPC test
handlers), the existing CLI sequence parser, and the ban on float money
arithmetic. They were extended using the existing seams; no lint was
weakened. All build output and downloaded web dependencies are outside
the checkout. The two local Cargo configurations remain untracked.

### Provider source inventory (reference source read, not a live-account check)

Paseo's providers emit session token/context reports separately from the
usage-source plugins. Claude's result usage is input/cache/output plus
`total_cost_usd`; Codex's app server supplies `thread/tokenUsage/updated`.
Pi/OMP poll the provider's own session statistics at turn completion (and
while active); OpenCode reads session-step usage and its model catalogue.
Vessel continues using ACP's equivalent reports rather than starting a
second provider connection just to count the same work.

| Paseo source | Local source it reads | Vendor response it reports |
| --- | --- | --- |
| Claude | selected Claude file or macOS Keychain; Pi/OMP OAuth stores | OAuth usage: five-hour, weekly, model/surface weekly limits; profile identifies organization |
| Codex | selected Codex login; OpenCode/Pi/OMP OAuth stores | ChatGPT usage: duration-bearing primary/secondary, additional named limits, review limits, credits |
| Copilot | configured token variables or local GitHub CLI login | GitHub Copilot plan and quota reset date; its current plugin reports no percent windows |
| Cursor | token variables, headless CLI login or desktop SQLite state | Dashboard period percentages and reset; spend balance when percentages are absent |
| Grok | token variables or CLI login | credits billing: explicit weekly/monthly enum, used percent, allowance and balance |
| Kimi | token variables or Kimi login | coding usage and individually named/duration-bearing limits; no assumed period |
| MiniMax | key variable, native credentials or native configuration | token-plan per-model interval/weekly totals, remaining percentages and resets; application errors can arrive with HTTP 200 |
| OpenCode Go | the native OpenCode API-key entry | rolling (unknown length), weekly and monthly percentages and resets |
| Z.AI | configured key variables | subscription details; its current plugin reports no percent windows |

These are source findings, not a claim that all nine adapters have been
implemented in Vessel. Provider-controlled custom URLs are not accepted
as new credential destinations; the fixed-origin rule of ADR 0042 wins.
Gemini and unknown ACP agents have no external limit adapter promised by
this inventory. Their own reported usage still counts.

### Preliminary independent review

The reviewer found that ACP defines cumulative **session** cost, not
process cost. The initial process-reset assumption was removed. Baselines
now survive the same native identity's Ready, including load-time replay;
a new identity, decreasing counter or profile change becomes an explicit
unknown baseline. Existing sessions' first totals are not today's spend.
An increment crossing the selected time boundary is not assigned wholly
to that period. Latest cumulative session cost is a separate observation.
The reviewer also requested optional-field coverage and an explicit sparse
read-only page contract; both are in the wire and regression fixtures.

Build scheduling changed during this slice: subsequent Cargo builds run
under the shared build-slot wrapper. A queued slot is a wait, not a hung
compiler; no parallel build was started to bypass it. The source gate
before the preliminary cost review passed, but the revised tree requires
its own complete gate and is not reported as passed on that older result.

Retention keeps exactly the newest reports when timestamps tie. Its row
count is maintained by SQLite triggers (including session cascades), so
an ordinary event batch does not walk the entire 100,000-report index.
A small-bound regression fixture checks tie ordering and cascade counts;
the production cap stays 100,000.

The first whole run passed the CLI usage scenario and 143 of 145 node
end-to-end tests. The new viewer test failed before querying because an
ordinary test node advertises no public pairing route; its invitation
now names only its actual ephemeral loopback listener, following the
existing secure tests. The other failure was the existing reinstalled
fleet-node test waiting for a report newer than a timestamp taken after
startup; it has been reported to its owner, without changing its grant
policy or widening its deadline. The forwardable-set fixture also now
explicitly includes the intentional new routed read call. These are
intermediate results, not a clean whole gate.

The coordinator assigned the pre-existing reinstall-report test race to
the node integrator after several tracks reproduced it. This track must
not change that test. Reporting may name that single known race in the
whole run and must include one standalone run of it. The race concerns
the restart/report timestamp and potentially independent retry jitter;
it is not permission to raise timeouts or weaken a grant check.

Per-agent day/week totals include every profile when no profile is selected.
Restricting that default to the implicit account would miss the same agent's
work on other accounts. A named filter selects the profile recorded with
each report (including historical names), not its current assignment.
The real-node fixture covers implicit default, explicitly named default,
and a separate profile, plus the named-profile restriction.

The first complete no-fail-fast run reached all accounting unit tests,
which passed. Three legacy migration fixtures then failed because their
manual downgrades left the new usage column/tables in place. They now
undo the usage ledger before rebuilding their old schemas; production
migration checks remain strict. Two separate fleet grant failures in
that run are assigned to the node integrator: late grant latency and
stopping while a grant is in flight. The coordinator requires one isolated
run of each and authorizes reporting/committing the first slice with
those named integration exceptions. They are not called a clean gate,
and no timeout or product rule is weakened here.

The page end-to-end run was 34/36. First-paint theme coordination and the
20,000-entry find/outline case are integration exceptions assigned to the
page's row-one work: that branch contains the persistence coordination
fix and passed all 36 tests, including the large timeline case. This track
must not chase or loosen either test. Each is run once alone and both
results are reported; the integrator reruns them after merge. The first
accounting slice is reviewed on its Rust run and the passing web lint,
types, unit tests and bundle build, with these exceptions named.

Conversation registration is agreed at the front of the dock: one
`session_line` from node totals, then `limit_notice`, both optional and
above permission/question cards. The dock's empty condition includes both
values. The existing per-turn tokens stay in the conversation's divider;
no second total is computed there. These renderers are not implemented
by the accounting slice.

### Accounting slice verification

The first node commit's final whole workspace run used `--no-fail-fast`: 1,651 passed,
2 failed and 9 intentionally ignored. The only failures were the two
integration exceptions assigned elsewhere: the member stopping during a
grant, and the shell beside a scheduled session still being observed as
running. The latter is fixed on the integration branch; both predate this
slice. They are not described as a clean whole pass. All four new real-node
usage cases, the CLI usage case, the accounting unit cases and the repaired
legacy migration fixtures passed. Format and whole-workspace all-target
Clippy passed with warnings denied. Generated TypeScript passed its two
freshness checks. Web lint, type checking, 243 unit tests and bundle build
passed. The page end-to-end run was 34/36; its two assigned cases also
failed when each was run once alone (a click timeout and a refused
connection to the fake machine). Their integration rerun remains required.
The three requested isolated fleet-grant runs are reported separately;
they do not replace the whole run above. No native gate or screen approval
is claimed by this node-only slice. Tests and builds ran as the workspace
owner; no service permissions changed. No real sign-in or vendor endpoint
was accessed.

### Accounting review corrections

The first source review returned three issues; its earlier gate is not
approval. Exact Decimal parsing must reject underflow, and partial money
sums must remain integers until the final checked wire conversion. The
regressions include 1e-29 reports, 1e-28, a large cumulative increase,
separate 1e16 + 1 + 1 costs in every order, and an unrepresentable final
sum whose contributing reports are all marked unaccounted.

Usage scans are admitted by two shared, immediately acquired permits.
Their blocking closures own those permits even after cancellation,
including time spent queued in the blocking pool. A deterministic fixture
holds one blocking worker, cancels both running and queued callers, checks
an immediate typed refusal, then releases work and queries the real store.
The ordinary idle connection cache is not a concurrency limit.

The CLI now includes reported reasoning counts and turn coverage, retains
unreported-turn counts when no token report exists, and shows optional
reasoning/cache fields per turn. Whole outputs distinguish reported zero,
missing fields and partially reported aggregates; the process fixture
supplies one reported-zero turn and one turn omitting the optional fields.
These corrections require a fresh gate and review before being called
passed. Account-limit lookup, recovery and native screens remain pending.

### Resume after the overnight credit cap

The first node-only commit is `cb267035` on `backend/m12-usage`. V1
returned it for exact cost arithmetic, cancellation-safe scan admission,
and CLI reasoning/turn coverage. The correction remains part of that first
slice. Gate results above belong to the first commit; a correction without
its own completed gate is not approved. The review is with track V1; the
coordinator and node integrator must approve before integration.

No account lookup, wait-and-continue recovery, native usage page, meter or
conversation usage renderer is complete. No live-account check was made.
After the accounting correction passes review, merge the lead at a clean
point, keep the search migration before the named usage migration, and
cover parking's load-before-Ready boundary without resetting same-session
cost. Then continue the external-limit node contract before its consumers.
The planned saved-login lookup defaults off until an administrator opts in;
its actual setting and disclosure row still need implementation.

Track E2 reserves the Usage settings section and renderer registry seam;
this track supplies `usage::render(&Shell, &Tokens, bool, &mut Context<Shell>)`.
Track C reserves two optional elements at the front of the conversation
dock: session totals, then a limit notice. Its existing turn tokens and
context meter remain separate. Both tracks are waiting for these renderers.
The read-only meter is reserved with V1, but no new control or branch merge
is started after the credit watcher's finish-current-slice instruction.

### Correction checkpoint under the finish-current-slice instruction

The correction is preserved as work in progress because the previous whole
suite needed about 39 minutes of test execution alone, before compilation
and the shared build queue. Formatting has passed. At this checkpoint,
the full-workspace, all-target Clippy retry is queued; the new accounting,
admission and CLI regressions, TypeScript comment regeneration and web
rerun are not yet verified. The correction's whole workspace test is also
unverified. The earlier whole results are not evidence for this correction.
Subsequent verification is recorded separately; this checkpoint is not a
review pass or permission to integrate.

All three requested grant cases passed once alone on the original commit:
reinstall, slow grant and stopping during a grant. These isolated passes
do not erase the original whole-run failures or change their ownership.
No grant test, timeout or product policy was changed by this track.

The review correction checkpoint is `7032f3cd`. V1 read its complete
22-file diff and confirmed that all three production source findings were
addressed, with no additional blocking source finding. This is source
review only: the reviewer did not execute the new fixtures, and gave no
gate, integration or UI approval. A subsequent Clippy attempt found two
unchecked clock additions in the precision fixture's helper; those use
checked time arithmetic now. Production source is unchanged by that fix.
The all-target check is retried; functional regression results remain
unverified until actually recorded.

The first focused store execution was 18 passed and 3 failed. It exposed
another boundary: the JSON library's default float parser changed tiny
reported values during storage/reload, so 1e-28 was rejected as an
unrepresentable decimal and a 2e-29 cumulative observation moved by one
ULP. The shared dependency now enables `float_roundtrip`, and a full
context-usage JSON regression covers these numbers. This changes JSON
decoding and requires fresh verification; the earlier Clippy pass applies
to `017ff192` before that dependency-feature correction. No failure is
relabeled a pass. The focused rerun is recorded separately.

### Final scoped verification checkpoint

The current production correction is `1e227b55`, after `017ff192` and
`7032f3cd`. With JSON round-trip parsing enabled, all 21 store usage
tests and both selected protocol tests passed (23 total). This includes
all three precision cases that failed before the parser correction,
exact accumulation/order tests and queued/running cancellation admission.
The previous all-workspace/all-target Clippy pass was before that final
JSON-feature change, so latest full Clippy remains unverified.

TypeScript regeneration completed with 2 selected tests passed.

The correction's whole workspace execution, CLI/node process regressions,
web rerun, live-account behavior and native usage/limits surfaces remain
unverified or unimplemented. Earlier whole/web results apply only to the
first node commit. V1 resolved its three original production source
findings; this is not gate or integration approval. No branch merge or
new control was started after the finish-current-slice instruction.
All work runs as the workspace owner; no service permission changed.

### Web closeout after generated declarations

- Web lint: passed.
- Web typecheck: passed.
- Web unit tests: passed.
- Web build: passed.

The page end-to-end exceptions remain with integration; they were not
rerun during closeout. Latest full Rust Clippy/test and CLI/node process
regressions after the JSON-feature correction remain unverified. No native
gate or screen approval is claimed. No service permission was changed.

### Resumed node-only integration

The accounting stack is being merged with the protocol-18 node line before
further feature work. This branch provisionally used protocol 19 for
`usage.query` and schema 14 for the usage migration; at the merge they
became protocol 21 and schema 15 (side questions took 19 and schema 14,
`device.self` 20). The minimum remains 2.
Generated TypeScript is regenerated from the merged Rust contract.

The current scope ends at node, store, wire, client, CLI and fake-fleet
behavior. Native Usage screens and the conversation slots are deferred
to the screen tracks; no screen or native gate is claimed here. External
lookup and recovery remain planned, not implemented.

The merge preserves cumulative baselines across wake completion and treats
fork origins as existing history. Rewind and profile-move replay still
need an explicit observation-origin boundary: the session can forward a
usage report while suppressing the replayed Ready event. That boundary
will be covered separately after the baseline merge gate; its observed
cost must not silently become new spend.

The resumed full Rust gate and required full web gate, including its own
headless fake-fleet Playwright suite, are pending. Previous failures and
scoped passes above describe the older tree, not this merge.

### Merged Rust baseline verification

The node merge is preserved as `2b9ec3a2`. On that merged source,
formatting and all-target workspace Clippy passed. TypeScript regeneration
passed both selected tests. The complete workspace test run passed:
1,802 passed, 0 failed, 10 existing ignored tests across 110 result groups,
including doc-tests. No test was excluded by this track.

All 174 node end-to-end tests passed, including the four usage cases.
The formerly failing grant and scheduled-session cases passed on this
node line. The five cost-precision regressions, queued/running cancellation
admission regression, CLI whole-output unit fixtures and CLI process
usage fixture also passed. The changed crates were compiled in this run.

The full web gate is still pending. This Rust baseline is not integration
approval: the replay-observation accounting boundary described above
remains to be fixed, and external limits lookup remains unimplemented.

### Complete merged baseline gate

Web lint and typecheck passed; all 417 unit tests in 81 files passed, and
the production bundle built successfully. With that bundle embedded in a
fresh Rust binary, all 62 Playwright tests passed, including the 20,000-entry
timeline case (31.7 seconds). No case was skipped or excluded.

The first browser attempt used an older bundle: the verification script
built Rust binaries before rebuilding the page, while `vessel-web` embeds
its assets at compile time. That invalid attempt was stopped, its own
fleet shut down, and it is not counted as a page regression. The corrected
order is: build the bundle, build Rust with `VESSEL_WEB_REQUIRE_BUNDLE=1`,
then run the complete browser suite. Served static JavaScript and favicon
bytes matched the current bundle. The HTML itself is painted per request
and must not be compared byte-for-byte to the unpainted build artifact.

The merge baseline now has a complete passing gate. Runtime replay origin
remains the next accounting correction before an integration-ready report;
limits lookup and recovery are still unimplemented. No native UI work or
live-account verification is included.

### Runtime replay observation correction

Reuse the session runtime's existing replay flag, rather than guessing from
a Running status or a Ready event that a move, wake or rewind suppresses.
`ContextUsage.observation` is optional protocol-21 metadata, set by the
runtime and defaulting to current for older records. Replayed cumulative
increases produce a baseline/gap. The latest reported total and context
occupancy remain visible. Older event and forwarded conversation replies
strip this metadata. Identity/profile changes retain their conservative
unknown-baseline policy; no new database migration is necessary.

Regressions cover increased replay with no visible Ready, old/new wire
projections, and actual fake-ACP node wake, rewind and profile move. The
fake uses the vendor's UsageUpdate response shape with cumulative costs
0.5, 10, 12 and 14. Verification of this correction is pending; the passing
baseline above does not prove the newly changed source.

The replay correction passed its selected gate: formatting, all-target
Clippy for the six changed crates, 3 protocol and 22 store usage tests,
and all 3 real-node replay cases (wake, rewind and profile move). The
profile case proves the old profile keeps only its earlier 0.5 cost while
the new profile receives only the subsequently observable increment.
The fixture now uses the current protocol constant and a flat scenario
body; latest whole-workspace, generated declarations and full web
verification are pending before integration approval.

On the replay correction, latest all-target workspace Clippy and both
TypeScript regeneration tests passed. The generated wire declaration now
includes optional `observation` and the closed `UsageObservation` enum.
The correction's whole Rust and web runs are still pending.

### Final gated accounting slice after replay correction

Production correction `57ebcb3c` and generated declarations `c7063c08`
passed formatting, full-workspace/all-target Clippy and both TypeScript
generation tests. The whole workspace run passed 1,807 tests, with
0 failures and 10 existing ignored cases across 110 result groups. All
177 node end-to-end tests passed, including the 3 replay cases.

Web lint, typecheck, all 417 unit tests in 81 files and the production
build passed. Rust binaries were built after the bundle with the bundle
required. Served main JavaScript and favicon matched the built files.
All 62 Playwright tests passed (3.6 minutes); the large timeline case
passed in 26.2 seconds. Traces, screenshots and video stayed off.

A screen can call `NodeClient::usage(QueryUsage)` for session totals or
rolling machine/agent totals. A session sample page continues while
`next` exists, even if cost redaction left its visible samples empty.
`optional_reports` gives per-field coverage; `unreported` preserves whole
turns missing counts. `current_costs` is a cumulative observation, distinct
from period spending. Cost requires operate permission and remains grouped
by reported unit. A session's context report is not an aggregate input.

The provisional additions are `usage.query` and optional
`ContextUsage.observation` in protocol 21; the minimum is unchanged at 2.
The usage ledger is schema 15, after side context's schema 14.
No other new call, event kind, scope or migration was added by the replay
correction. Integration may renumber the additions.

External account-limit lookup, limit-notice facts and recovery scheduling
are not implemented by this slice. Native screens and controls are outside
this resumed node-only scope. Real credentials and vendor endpoints were
not used; a live-account check remains unverified by instruction. Commands
ran as the workspace owner, with no service permission or ACL changes.

### Execution provenance limitation

The final accounting run did not retain a pre/post frozen-source manifest.
Its command results, compilation logs, immutable source commit and subsequent
generated-declaration/document commits are retained, but these do not
retrospectively establish the working tree's cryptographic identity during
execution or pin untracked build inputs. Review and integration approval
are still pending. No pre-run attestation is being reconstructed, and the
unchanged gate is not rerun to conceal this missing record. The next
genuinely changed slice will capture its selected inputs and identities
before and after execution.

### Compatibility review correction

Review returned the stack for one common seam: fleet envelopes still
reported only their own protocol-6 requirement, concealing a protocol-21
usage call. The correction reuses the common recursive `Call::since`
semantics: maximum envelope and inner method/field version, for all three
wrappers. The usage arm remains unchanged. Client tests include direct,
call-on-self, forward, relay and deliver refusal without a sent frame. Raw
server tests force each wrapper targeting the welcomed node and verify
refusal before handler dispatch, while supported envelopes reach the
handler. A field-only version requirement is covered too.

The correction's new gate will capture selected project inputs and git
identity before and after execution. Selection includes all tracked files,
the local build configurations and the embedded web bundle; dependency
lockfiles and tool versions are recorded. External dependency-cache and OS
contents, build outputs and test artifacts are outside that selection.
This does not reconstruct the preceding run's missing attestation. Review
and new verification are pending. No quota implementation is included.

The compatibility correction passed selected all-target Clippy and six
library regressions (protocol 4, client 1, RPC server 1). Its actual pre/post
selected-input manifests matched at immutable source commit `09b89440`,
including the two local build configurations and embedded bundle. This
is a new execution record for the changed correction, not an attestation
for the earlier accounting run. The remaining preexisting test comment
now describes the common maximum-version rule; its assertions are kept.
Latest whole-workspace and web verification remain pending.

### Compatibility correction: actual verification

Source commits `09b89440` and `8ab23bda` close the recursive envelope version
requirement. The selected protocol/client/RPC regressions passed all six
cases, including ordinary supported envelopes and an inner field requiring a
later version. The latest full run checked immutable source `8ab23bda`: fmt,
full workspace/all-target Clippy and two TypeScript declaration freshness
checks passed. Rust reported 1,808 passed, 1 failed and 10 ignored across 110
result sections. The one failing test was
`grants::a_reinstalled_node_grants_again_once_the_home_forgets_it`: its
whole-test safety net elapsed at `tests/e2e/support.rs:46`. This fixture
already contains the earlier pre-restart stamp correction. Its remaining
failure belongs to node integration under the coordinator's assignment; this
slice changes neither its assertions nor deadlines. It was run once alone
afterward: 1 passed, 0 failed (16.74 seconds). A standalone pass does not
erase the full run's recorded failure.

Independent web verification reported lint, type checks, 417 unit tests and
bundle build passed; all 62 browser tests passed (6.8 minutes). The Rust
binaries were built after the current web bundle, with the bundle required;
the browser suite used its own fake fleet and all configured cases. No live
node, real agent, real vendor credential file or vendor endpoint was
exercised.

Both new runs retained actual pre/post manifests from the same source commit.
The full Rust run selected 2312 inputs and matched identity
`72aebd1f9b0bb5a872f7fb0703ec973629b5f697f19df2a0823fd6b36177502a`; the
isolated/web run selected 2312 inputs and matched
`b83f198dd8cde1c32d1e1fcffe897ae4970580cde18c73cece5be2960db08909`. Selection
covers all tracked files, local build configurations, the embedded bundle and
installed npm lock; tool versions, chosen build flag hashes, generator and
gate-script hashes are recorded. External cache/toolchain/OS contents and
test/build artifacts are excluded. This is selected-input execution
provenance, not a reconstruction of the old accounting run's missing pin or a
universal filesystem attestation.

This accounting/compatibility stack is submitted for review with the explicit
whole-run fixture exception; integration approval is pending. The provisional
wire additions remain `usage.query` and the optional
`ContextUsage.observation`; minimum protocol stays 2; the merge numbered
them protocol 21 and schema 15. A screen uses
`NodeClient::usage(QueryUsage)` or the checked fleet call, respects
`cost_visibility`, optional report coverage, `incomplete` and the `next`
cursor even when redaction leaves an empty page.

Account quota lookup and recovery are still unimplemented in this commit.
Their next contract is opt-in local saved-sign-in reads, fixed vendor origins
and typed window/reset facts; no native or page screen is part of this track's
resumed node scope. Live-account and cross-platform behavior remain
unverified. No service, live file permission or deployment changed.

The fake UI served the current static JavaScript and favicon byte for byte.
All five recorded fake-fleet processes were gone after teardown; no virtual
display was started. Both local Cargo build configurations remain local.

### Quota continuation contract (implementation pending)

ADR 0042 now fixes the next node contract: cached `usage.limits` reads,
operate-only explicit refresh, admin-only `usage.lookup`, a launch-pinned
session association, current-profile account selection, token-wide vendor
cooldown and cancellation-safe ownership of every blocking phase. No new
wire or executable implementation is included in this contract commit.
The next implementation must prove these boundaries against synthetic
vendor-shaped testkit data; live credentials and endpoints remain excluded.

### Opt-in limits continuation: implementation and acceptance boundary

The continuation implements the ADR's saved-sign-in contract in
`vessel-proto::limits`, `vessel-harness::limits`, `vessel-session::limits`,
`vessel-store::usage_lookup`, `vessel-node::limits` / `service::limits`,
`vessel-client::usage`, and the CLI's `usage::limits` / `limits_render`.
The testkit exports the shared synthetic Claude/ChatGPT response shapes from
`vessel-testkit::limits`; parser and real-node tests use those same bytes.
No native renderer, control or page is included.

Additional source references read for this continuation are Paseo
`plugins/claude-usage-source/server/{usage,stores}.ts` and its usage/keychain
fixtures, `plugins/codex-usage-source/server/{usage,stores}.ts` and fixtures,
plus `packages/server/src/server/plugins/usage-sources/index.test.ts` for
identity, stale observation and refresh coalescing. The already listed GOAT
recovery banner remains a reference, not a claim that recovery shipped.

Visible node/CLI acceptance implemented, subject to the results below:

1. Start off; retained usage remains available. The CLI explains exactly
   which saved access token/account identifier a refresh reads here and
   which fixed vendor receives it. An admin can save on/off; cancellation
   of the requesting RPC does not abandon the independent owner's choice.
2. Select a configured harness/profile, request a refresh, then read it
   from cache without credential/vendor I/O. Independent periods and scopes,
   unknown/missing/zero values, paid use and credits remain distinct.
3. Read the actual live session binding even after profile reload; preserve
   it through park/wake and replace it only after a successful move. A stored
   session after node restart refuses to guess a current login.
4. A read-only paired device can see cached sanitized facts but no balance,
   cannot refresh and cannot change opt-in. An agent can request only its
   own local session. Fleet delivery applies the existing destination and
   origin scope intersection. Pre-19 direct and wrapped calls are refused
   before sending/dispatch.
5. A failed refresh labels the previous observation as stale only for the
   same observed login. Missing/changed login clears it. Retry-After applies
   across aliases of the same token and Codex/OpenCode. Queued/running
   blocking work retains bounded admission after cancellation.
6. Full general windows produce factual notice identities and an optional
   latest known future reset. Paid-use eligibility suppresses a false stop
   notice. A screen can draw these facts in the viewer's language. A failed
   turn's cause, durable wait-and-resume and switching accounts are NOT
   implemented by this slice; original acceptance for those remains open.

Wire additions beyond the accounting stack are `usage.limits` and
`usage.lookup`, both provisional protocol 19 (protocol 23 on the node line,
after `usage.query`'s 21 and the inbox's 22). No additional event or database
schema change. A screen enumerates profiles with the existing call, requests
`GetUsageLimits` cached/refresh, toggles `SetUsageLookup` with admin authority,
uses the returned source for disclosure, and formats the returned timestamps
locally. It must not interpret a full bar as proof of a stopped turn or offer
a non-working automatic continuation action.

Live vendor accounts, real credential stores, macOS Keychain execution,
Windows/macOS runtime behavior and every native screen remain unverified.
Automated checks use temporary synthetic logins, recorded vendor shapes and
fake agents only. No live permission, service or deployment change.

#### Limits verification checkpoint

The relevant nine crates' all-target Clippy check passed. The focused `limit`
run passed 43 tests with no failures or ignored cases, including real-node
profile reload/wake/move/restart and fleet grant tests, parser/source admission
and cancellation cases, both new methods' old-peer refusal and full CLI output.
Both TypeScript generation checks passed. The fixture policy file's mode/bound
regression and an added rewind-association assertion await the whole run.
This checkpoint does not claim a clean whole gate or integration readiness.
The earlier accounting stack still has its disclosed grant fixture gate hold;
a standalone pass did not erase that whole-run failure. No vendor, real-login,
macOS runtime or native screen check was performed.

#### First whole limits run and correction

The frozen `6fb685d0` whole run executed 110 Rust sections: 1,840 passed,
four failed, ten ignored. Fmt, whole Clippy and two TypeScript checks passed;
web lint, types, 417 unit tests, bundle, fresh binaries and all 62 browser tests
passed. Contemporary selected-input pre/post identities matched, and served
assets matched the bundle. The five recorded browser-fleet processes ended.
The whole Rust failure is retained, not described as an integration pass.

Two failures are this slice's tests: the fixed forwardable-method list omitted
the two new intentional methods, and doing a move after a fake rewind hit the
correct filename guard for its deliberately non-file-safe fake id. The
correction adds precisely those expected methods and tests rewind last, after
the successful move; no production guard, assertion or deadline is relaxed.
Two existing remote tests timed out during node startup. Their logs stopped
after migration, before the quota owner starts; cause and integration
disposition are unverified. All previously named grant cases passed in this
new run, without erasing their earlier failure record.

Source review also adds the reference's OpenCode provider check: account
lookup remains supported, while session lookup requires current native
OpenAI model evidence before any login read and rechecks it afterwards.
Absent/ambiguous model evidence is not guessed. The new correction and provider
regressions await their focused and whole runs.

#### Correction focus and next whole gate

After the fixture-order and pinned-method corrections, relevant all-target
Clippy passed. The full harness/node/proto library run passed 451 tests;
all nine limits real-node tests passed, including the OpenCode model-provider
guard before credential access. Each previously failing remote test passed
once alone on SSD temporary storage (35.83 s and 16.27 s), with unchanged
timeouts and assertions. This does not erase the earlier whole-run failures.

The following compatibility cleanup centralizes usage methods and replay
provenance on one named boundary; its final number is assigned at merge.
That cleanup awaits the next whole run. The next gate uses SSD test data,
records load/memory before and after, and prebuilds both testkit binaries and
`vesseld` in the same environment and enclosing build slot before tests.
It retains contemporary selected-source pre/post identities. The whole gate
and review are pending; no integration-ready pass is claimed.

#### Immutable corrected whole run, 2026-11-14

Executed source: `32b404f8e6a5664f201cd0a4801d9f10c8613945`. Both prescribed
prebuilds, fmt/check, workspace all-target Clippy, two TypeScript declaration
checks and the complete workspace test command passed. The Rust log contains
110 result sections: **1,846 passed, zero failed, ten ignored**. The corrected
move/rewind and forwardable-list cases, current-provider refusal before login
access, both remote cases and the previously named grant cases passed in this
whole run. No assertion, deadline or test selection was relaxed.

Web lint, types, all 417 unit tests in 81 files, bundle, post-bundle Rust binary
build and all 62 headless browser tests passed (3.3 minutes; the large timeline
case took 24.6 seconds). The binary build actually recompiled `vessel-web`,
and six served asset hashes matched the new bundle on the isolated fake
fleet. This proves that regression suite, not a new Usage screen or native UI.

The gate held one build slot throughout and prebuilt testkit binaries and
`vesseld` with the same environment before tests. Test data used owned mode-0700
SSD temporary storage, with two Rust test threads. One-minute load before/after
was 24.67/11.75; memory records were retained. Build outputs and logs remained
on the build disk. All five independently recorded browser-fleet process
identities were gone afterwards; the owned test directory was emptied.

Contemporary pre/post selection covered 2,346 inputs: every tracked file, both
local Cargo configs, the bundle link and its files, the installed npm lock,
tool versions, gate/generator identities and recorded environment flags. Both
identities were
`7c5e8ab56876e9142beb30c5defbd2b9b207f6c35104d1b135c1f38e9de67009`,
with no changed selected path. This is evidence for this immutable run only;
it does not retrospectively pin or erase any earlier failed/unpinned run.

The corrected stack is submitted for review; no reviewer or integration
approval is inferred. The ADR remains proposed. Live credentials/vendors,
platform Keychain, non-Linux runtime behavior, native screens and automatic
wait/resume/account switching remain unverified or unfinished as stated above.

## F3 bounded backend preparation and activation hold

The later generic account-report preparation has an independently verified product
whole at `87a1f2c6dd5a310713d892748aa21566cb3760bc`: eighteen steps0,
Rust2759/0/12, web465, E2E80, forge4 and generated freshness5/diff0. Test-only
failure-ownership corrections at `e1ff8671c7c5aba13a5b5ade9786a245b2d851f6`
have current all-target harness/node strict0 with usage-fixture and actual426/0/0
(harness82, node344), including all23 changed-file cases. Other tracked files3566
have identical product-baseline blobs; the mixed test container's product prefix
is byte-identical. These scoped records do not accept the complete native/web/CLI
feature or real vendor/platform operation. Earlier failures remain retained.

Activation is explicitly held for ADR 0104. Usage HTTP is the shared tracked DNS
primitive's third consumer, after relay and dictation. The default reqwest resolver
in `vessel-harness/src/limits/http.rs::Http::new` and the additional Local source
body cannot yet prove physical native DNS settlement on async cancellation.
New account methods/feature constants remain `None` until the shared primitive
is consumed through `dns_resolver`; no private resolver or fork is introduced.
Recorded fixtures, credential completion and request cancellation are not native
DNS proof. G4 owns the shared consumer list; receiving integration/whole and any
later activation assignment remain separate. No new worker whole is authorized
merely for recording this hold.

2026-11-16 receiving wiring (takeover of the paused track): the client now takes
the node's tracked resolver through `dns_resolver`; see ADR 0042 "Account HTTP
consumes the tracked resolver" and ADR 0104's receiving note. Tests: Harness
`limits::resolution_tests` (no resolver means no client; both vendor names reach
the injected hook, one shared client), node `http_resolution::tests` (port 0,
closed-scope refusal with no default-resolver connection, a real loopback request
through the tracked lookup) and `limits::dns::tests` (the scope stays open until
both owners end). The assembly test `shared_admission_tests` runs the real
`Limits::configured` path including retirement. Activation stays a separate,
integrator-numbered step.
