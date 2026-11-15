# Comparison acknowledgement and history observation

The approval-continuation fixture read history as soon as the comparison job
reported Started. It indexed an empty result when the actor had acknowledged
its provider command but had not published the queued TurnStarted event.
Started promises one ordinary acknowledgement, not a durable event checkpoint.

[Decision Log]
- Existing assets: the fake plain turn completes; test support already follows
  records with read_until and a bounded step. No external package is needed.
- Choice: observe that actual turn end, then perform each original history read.
  The earlier TurnStarted is already in the published prefix even if the
  subscriber wakes before the current turn-end record updates next_seq.
- Why: the job watch/ACK cannot observe actor publication. Sleep, retrying the
  snapshot, changing ACK semantics or increasing a timeout would hide or expand
  that boundary. Negative/no-prompt reads keep their existing nonwaiting helper.
- Impact: original attachment bytes/ids, resource/prompt counts, dropped-method
  ownership, cancellation and refusal expectations remain unchanged. Two positive
  reads use the observation helper; product text clarifies the existing meaning.
  No product ordering, wire, schema, dependency or authority changes; no existing timeout was increased.

## Source evidence

The helper sets Started directly on Delivered::Started. Session start_loaded
uses pump, whose biased select prefers a completed command over queued agent
outputs. The agent enqueues TurnStarted before answering, but actor publication
and persistence are later. A normal history read waits through the actor's
published prefix when that read arrives, not outputs still queued for it.

Unmodified comparison/delivery/pump/read sources are identical on the two named
node heads. Hand-over runs after an ended run's persistence finishes; neither
the comparison helper nor this live first-turn assertion uses the ended
broadcast, hand-over or runtime removal. Fake plain ends its turn and keeps
its process/session live. Teardown occurs after the assertion.

## Evidence and limits

| Execution | Result | One-minute load, launch to finish |
| --- | --- | --- |
| Unmodified node `0618c4fb`, exact case 20 times | 20 passes, 0 failures | 17.22→17.13 (range15.81–17.22) |
| Unmodified node `a365d295`, exact case 20 times | 20 passes, 0 failures | 16.03→24.33 (range16.03–26.38) |
| Fixed case 20 times | 20 passes, 0 failures | 17.10→22.55 |
| Whole required workspace test attempt | 1,973 passes, 1 timeout, 12 existing ignores | 27.95→14.75 (whole gate) |
| Exact remote timeout case, sole isolated replay | 1 pass, 0 failures, 219 filtered; 4.93 s | 16.36→17.17 (includes queue/compile) |
| Remaining explicit forge-feature tests | 4 passes, 0 failures; 1.69 s | 16.88→17.65 (includes queue/compile) |

Each repeat used one bounded short-lane ticket per head, exact test selection,
separate SSD homes and a worker-owned target with one worker-scheduled command at a time (repository helper builds inherit that command).
Every individual repeat log confirms one actual case. Source was clean for each
original-head sample; the current head was changed after its 20 completed. These
samples did not reproduce the original intermittent failure and do not prove
its absence. The fixed first rebuild took 6m12s; isolated remote rebuild 8m17s;
both tickets completed below the unchanged short-lane limit.

Node all-target Clippy passed after two narrow lint returns (a new doc code span
and redundant new-helper qualifications). The public root runner then passed
format, source-size budget (769 files, 31 existing exceptions), boundaries,
both A5 prebuilds, workspace all-target Clippy and forge-feature Clippy. Its113
workspace result summaries contain only the remote token fixture's initial
control Connection::connect Timeout at `remote.rs:162`, before token changes or
exit-code assertions. The repaired comparison case passed in that full run.
No saturation cause is asserted. The original failed run remains separate from
the isolated pass, whose directory-as-token error is an intentional test path.

The workspace failure prevented the runner's final feature-only tests from
starting. That exact remaining command passed separately. This evidence does
not constitute a clean whole workspace pass; no unchanged whole rerun was made,
and no timeout, assertion or ignore was weakened. Clean combined validation
remains owed before integration approval.

All 1,722 selected source inputs match before/after/current manifests,
SHA-256 `508fed8191f8416433260d8df0a863fa7435a8b2c60db2b3ff8701126da52bd3`.
Selection excludes docs/native/ignored local config/dependencies/output; it is
not a full environment attestation. Changed client/node code actually compiled.
No browser, native screen, real provider or other-OS result is claimed. The
separate startup-values slice stays paused while this repair is finalized.

## Integration exception

The unchanged startup-handshake fixture is a shared integration intermittent.
The timeout-only policy accepts its sole isolated passing replay as evidence
for this test/doc commit. The original whole run remains failed; the next
combined integration gate still owes a clean result. This patch does not
repair that shared fixture or expand its handshake budget.

Evidence paths are relative to the review result bundle, not repository files:
`comparison-final-gate.log`, `comparison-final-root.log`,
`comparison-remote-alone.log`, `comparison-forge-remaining.log`,
`comparison-{after,before,fixed}-repeat.log` and their per-run directories,
and `comparison-final-source{,-after,-current}.sha256`. The counts, timing,
load and selected-source limits above apply to those exact artifacts.

Changed files were owned by UID/GID 1000. No live service or production
permission was changed. Final owned-target/test-home process scan was empty;
all three SSD test homes were cleared and their private 0700 roots kept.
