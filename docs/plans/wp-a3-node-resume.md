# Prompt metadata ROOT resume — quota WIP

Checkpoint: 2026-11-16 03:43 UTC. The quota watcher ordered only the active slice,
no new slice/merge/build, and committed resume notes. No feature completion or
production acceptance is claimed. Read these notes and inspect the existing
remote run before any new submission; queued/running/done runs are never replayed.

Source pin `a5adae47556ad3a57b563f0e4cdc883ba2e46864` remains frozen on the
returned-review base. No newer node head is merged. These resume changes are
only documentation; do not reinterpret their commit as a newly executed gate.

V3 row267 returned one test-lifetime finding. The four owned test files now
use shared testkit outer safety and named finite phases; blocking releases are
also bounded. The three approved workspace dev edges use the tool-generated
lock patch only, independently checked. No normal product change since f7.

Strict03 `wp-search-node-narrow-20261116T014900Z-a5adae47` passed all targets
for Client/RPC/Store, locked and -Dwarnings. Actual Store rebuilt; identical
Client/RPC were cached. Cargo SHA256:
6a704517c1db5d6b659bc2e9db436cd8377b08396163c6cad12ad065dd13ea48.

Only own existing run `wp-search-node-narrow-20261116T030227Z-a5adae47` is
ordinary test02 on the SAME a5 source, queued16 at03:41. Exact selection:

    cargo test --locked -p vessel-client -p vessel-rpc -p vessel-store --lib --test prompt_index prompt_index -- --include-ignored --skip prompt_index::fold::tests --nocapture

Expected actual six cases: Client1, RPC1, Store cancellation1, Store integration3
including measurement. Recover the recorded result, binary/case counts, exact
spec/checkout and all step SHA values before any new submission or PASS claim.

Previous runtime a966 executed5/1: five boundary/durable cases passed; only the
new five-second full-text indexing setup phase failed before metadata paging.
A5 preloads the SAME20,001 ordered records in1,000-record batches using the real
indexed barrier. Five-second phases and30-second outer safety remain unchanged;
no synchronization sleep/performance threshold or product bound was relaxed.
Writer publishes indexing before append ACK; the source ordering was checked.
Earlier red logs remain. Measurement remains metadata evidence, not a UI SLA.

Original f7 whole remains17/18 steps green, raw2388/1/13. Its sole SSH fixture
failure was the named long-TMPDIR runner case, closed on SAMEf7 by remote33/0/0
under the fixed runner. Both logs remain; never call this an uninterrupted whole
PASS. The exact authorized stale LimitPercent alias cherry-pick changes no TS.

After test02: if6/0, append exact receipt and request final SAME V3 row267, then
hand the frozen range to the node integrator. No new whole/base/product change
is authorized for this correction. If red, preserve it and diagnose only the
owned cause; no unchanged reruns or deadline lengthening. Final V3 is pending,
PROMPT_INDEX_PROTOCOL remains None, positive assigned-RPC/native/UI acceptance
is not claimed. The integrator owns assignment and newer-base union.
