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

The first node slice adds `usage.query` (read, protocol 12). It takes a
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
