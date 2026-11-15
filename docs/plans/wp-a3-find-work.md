# Find work regression, independent of build VM contention

[Decision Log]
- Purpose: retain the page's 5,000-entry performance regression intent without
  treating scheduler delay on the shared gate VM as search work.
- Existing constraint: E2's unchanged 970563a2 failed the cold 2 s wall bound at
  2.2675 s in its whole gate and 2.1993 s in the exact narrow rerun. The latter
  VM load was 34–39 on 20 CPUs. Those receipts remain failures of the old bound;
  they are not evidence of a functional search failure or measured UI latency.
- Alternatives: repeatedly rerun, increase the timeout, or count the work. No
  finite wall-clock margin is independent of host preemption, and rerunning
  unchanged code cannot establish the algorithm's bound.
- Choice: extract the existing cache verbatim into `timeline/find/cache.rs`,
  adding counters only in library test builds at the real entry visit and cache
  rebuild sites. A 5,000-entry test requires one visit/fold per cold entry, zero
  warm folds across 20 queries, reuse across query changes, exactly one fold
  after one revision change, and no more than 200 entries per Finder frame.
  Both cold/warm and sliced paths must preserve the complete 20,000 matches.
- Why: deterministic work budgets detect loss of caching, duplicate traversal,
  or loss of frame yielding while remaining valid under arbitrary VM load.
- Tradeoff: this does not certify a latency SLA or detect every constant-factor
  slowdown within one fold. The original representative Markdown workload and
  opt-in `VESSEL_FIND_TIMINGS=1` cold/warm/slice diagnostics remain for actual
  measurement; the integration test now also compares every complete result.

No runtime algorithm, cache key, slice size, dependency or API changes. The cache
module is private; its public `FindCache` path stays unchanged. Counters compile
only into unit tests, not the app or integration-test library. Validation is one
remote native View all-target test run, including existing Markdown/Unicode,
cache invalidation, incremental search and the new work regression.

## Receiving execution evidence

The isolated code range is dd13ce378..00ebfdeafaac650462d73853b54050ac57ca2f84:
four Rust paths and this rationale only. The module/reexport correction 2a4ca95a4
and saturating test-counter correction 00eb are included. No full A3 controller,
RPC or navigation implementation is part of this prerequisite.

E2 receiving source bc394c26c1a4c6a034cefa67c7bfd9590fc59300 executed run
`settings2-20261115T172956Z-bc394c26`. Its four actual steps (fonts, native quality,
strict Clippy and native tests) returned zero; raw tests were 1516/0/2. Both
`find_work_is_linear_cached_and_bounded_per_frame` and
`a_find_over_5000_entries_preserves_results_cold_warm_and_sliced` explicitly
passed. All four producer Rust blobs, including the FindCache registration,
are independently byte-identical to that executed source, and all four full
step-log hashes match the receiving receipt. The native-test log SHA256 is
160e93ad7e2a8b1edc813cedb6416c812a453e0f4f71e649e83db15d237f0f9a.

This is bounded same-source receiving execution evidence, not a standalone full
A3/native feature, latency, new format/build/export or later combined-union
approval. Earlier wall-clock failures remain retained. Reuse this one correction
rather than create a second threshold change; consumers still qualify their own
integration and preserve any frozen gate/review boundary.
