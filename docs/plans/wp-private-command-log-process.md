# Private command spawn-log fixture isolation

[Decision Log]
- Purpose: make the actual spawn event privacy assertion independent of other
  tests' first tracing-callsite registration (Tracker774).
- Existing constraint: LIB tests run in parallel and tracing caches interest
  process-wide; installing a thread-local subscriber cannot retroactively own
  another thread's first registration.
- Alternatives: rebuild global interest caches or run this case in its own test
  binary. Rebuilding can race a concurrent first registration too.
- Choice and reason: move only the actual spawn-log case and its unchanged tiny
  subscriber into an integration test binary with one test. It owns the first
  registration without global cache resets, sleeps or serialization changes.
- Impact: product code, all privacy assertions, real fake process/exit checks,
  sentinel and deadline are unchanged. Private cause-retention tests remain LIB
  tests because they require private access and do not capture this callsite.

This is the coordinator-authorized head-test correction for the named gate59
intermittent. Ordinary current Proc all-target strict and this actual integration
case are required before V3/normal receiving. It is no Windows execution or
creation-controller proof; the broader diagnostics scope remains unchanged.


The first strict run found two helper unwrap lints after the move: its original
cfg(test) ancestor no longer surrounded that module in an integration crate.
The helper module now explicitly retains that same literal cfg(test) boundary.
The helper bytes and actual test body/assertions remain unchanged; no lint
suppression, product behavior or failed-assertion handling is changed. The raw
strict failure is retained. The corrected source still owes exact strict and
actual isolated-binary execution.
