# Project pull-request provider/service candidate

Status: provider/service candidate `04d32734` and liveness correction
`c85e1531`/`ed34a92b` passed separate whole gates. The test-only correction
is awaiting source review of the returned liveness boundary.
The earlier local workspace run remains red; project listing is not callable.

## Implemented boundary

The real GitHub and FakeForge providers implement bounded repository pages
with a shared typed summary decoder. Credential associations use host-bound
allocation identities and invalidate on observed refusals without changing
token selection or retry policy. The staged node owner checks the current
association at admission, after loading and before publishing a queued
completion. Admission stays with queued/running root work, jobs and waiters;
refresh publication ordering prevents an older flight replacing a newer
result. A failed job closes only its child service and drains its waiters.

The node module remains `cfg(test)` until shared protocol admission, the
connection request budget and reviewed client/CLI wiring are ready. No RPC,
existing reply/event field, protocol number, dependency edge, store schema
or native/page change is added. `pr.list` remains watched-session derived.

## Local evidence

The selected real-behavior checks passed: 24 node project tests and 52 Forge
library tests, including loopback HTTP with shared credentials, actual
FakeForge state, registered git projects without sessions, pagination,
root/remote changes, cancellation, driven cache time and completion order.

The root run passed formatting, size (776 files, no new exception), crate
boundaries (22 crates, ten owned external dependencies), both binary
prebuilds and default/fixture-feature all-target Clippy. Its whole workspace
run completed 113 summaries: 2,019 passed, three failed, twelve existing
ignores. The failures were unchanged safety deadlines:

- `a_kitty_encoded_ctrl_bracket_detaches_after_the_keys_before_it`
- `a_piped_prompt_shows_the_whole_reply_then_detaches`
- `an_agent_keeps_schedules_of_its_own_and_reads_saved_prompts`

No failed assertion or deadline was changed. These failures remain red;
there is no flaky or saturation waiver. A process observation during the
remote CLI tests showed a nested product-node build; that does not establish
the cause of the earlier CLI failures. The later explicit fixture-feature
launcher and TypeScript regeneration steps were not reached after the
workspace failure and are not claimed passed.

All 1,736 selected source/policy inputs matched before and after the run.
This is source evidence, not an environment, compiler or dependency
attestation. The normal user ran the checks (UID/GID 1000); isolated SSD test
storage retained mode 0700 and was emptied after checking for remaining
processes. No production process, permission, real agent or real forge was
changed or used by the new fixtures.

## Additional-builder validation

The clean committed candidate passed all eighteen steps of the complete
Linux gate: page dependency installation, format, size, boundaries, default
and TypeScript/fixture-feature Clippy, TypeScript regeneration with no
declaration diff, binaries, both nested-build prebuilds, workspace tests,
web lint/typecheck/unit/fonts/E2E and the explicit fixture-feature tests.
The workspace reported 113 summaries: 2,022 passed, zero failed and twelve
existing ignores. Each of the three local timeout cases passed in this
whole run. TypeScript passed two tests, web unit tests 417, web E2E 62 and
the fixture launcher four. No case was replayed or excluded.

The actual gate ran from 02:53:38 to 03:06:45 UTC on 2026-11-15, after its
ordinary queue wait. Start/end one-minute load was 14.60/9.80; the workspace
test step started/ended at 9.38/7.11. It used eight build jobs and eight test
threads. The checkout record resolves exactly to the candidate, and all
eighteen step-log hashes match their recorded digests. Changed crates were
actually compiled. These are execution and selected-source records, not
an attestation of every dependency, toolchain or runtime property.

The builder regenerates its page bundle from the commit and uses its own
isolated temporary storage. Local selected-source pins excluded the local
prebuilt page bundle and ignored configuration. This changed-environment
pass does not establish a cause for the retained local failures. No
deadline, assertion, shared test or product behavior was changed between
the two runs.

## Review and following work

The completed source review found no additional product/provider, shared-root,
cache or credential blocker, but returned unbounded new fixture waits.
Passing execution alone does not close that quality return. The correction
retains every existing assertion, the one-worker blocking pool, channel
barriers and existing finite deadlines. It adds the existing testkit outer
safety net and named phase limits to the new waits, accounts explicitly
for the expiry test's virtual advance, and replaces synchronous fixture
git output with an owned asynchronous child killed and reaped on timeout.
A driven timeout probe checks that dropping the timed release sender lets
the blocking worker exit before runtime shutdown; a held-input fake checks
that the owned child is killed and reaped after its output deadline.
The fixture also cancels its async owners when an assertion or safety
boundary drops it. These are
test-only changes; the original source and passing gate remain retained.

The correction's first selected run ended at the short-lane limit. Forge
completed 52 tests; the node library never produced a final summary because
the newly added timeout-cleanup probe remained running. This is not a pass
or an unrelated shared failure. That probe advanced only to the nominal
deadline; millisecond timer rounding can leave it short of expiry, while
the active blocking worker disables automatic paused-clock advancement.
The corrected probe advances past the deadline and also has an independent
blocking-channel wall-time safety net. Its outer budget explicitly includes
the virtual advance. The original lifecycle/pool/barrier checks and finite
HTTP/failed-job/root deadlines remain unchanged.

### Corrected-source validation

The ordinary selected check on `ed34a92b` completed 298 library tests:
Forge 52 and node 246, with no failure or ignore. Both the timeout-release
and owned-child kill/reap probes explicitly passed. No unchanged retry was
used to erase the preceding short-lane limit or its stuck probe.

The complete additional-builder gate then passed all eighteen steps on
that same committed source. It reported 113 workspace summaries: 2,024
passed, zero failed and twelve existing ignores; TypeScript two, web unit
tests 417, E2E 62 and fixture-feature tests four. It actually recompiled
node and forge. The gate ran from 04:14:33 to 04:27:32 UTC on 2026-11-15;
start/end load was 10.36/8.00, workspace-test load 15.07/11.33, with eight
build jobs and eight test threads. Every step returned zero without skips.

All 1,740 selected source/policy/delta inputs matched immutable `ed34a92b`
Git blobs and the local worktree after the run. All eighteen step logs
matched their recorded digest prefixes. An assertion comparison, removing
only new phase wrappers and normalizing whitespace/trailing commas, kept
all 199 original assertions and found five additional cleanup assertions.
This comparison supports the manually reviewed diff; it is not a general
proof of semantic equivalence. Ignored configuration, dependencies and the
whole runtime environment remain outside this selected-source proof.
No local runtime remains; local isolated test storage is empty and retains
UID/GID 1000 and mode 0700. No production permission was changed.

Submit the corrected source and retained executions to review as a staged
provider/service slice. It remains on the track branch: product wiring,
authority/forwarding/older-peer fixtures, client/CLI/fleet consumers and
their complete product-configuration gate remain later work. Shared
protocol admission and the connection request budget are prerequisites.
The existing `pr.list` remains watched-session derived. No UI, capture,
real-forge, Windows or macOS execution is claimed.
