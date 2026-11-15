# Project pull-request provider/service candidate

Status: candidate for validation on the additional builder. The complete
local workspace run is red; this is not a passed node slice or a callable
project-listing feature.

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

## Next validation

The additional builder must validate the clean committed candidate before
source review is requested. Preserve the local red run, its selected passes
and load observations. Product wiring, authority/forwarding/older-peer
fixtures, client/CLI/fleet consumers and their complete gate remain later
work. No UI, capture, real-forge, Windows or macOS execution is claimed.
