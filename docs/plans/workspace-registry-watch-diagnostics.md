# Registry invalidation test phase diagnostics

[Decision Log]
- Purpose: a timeout must identify setup, write acknowledgement, watcher wake,
  read/CAS or close rather than attributing an entire Store lifecycle to a wake.
- Existing constraint: the original test grants ten seconds for all of that
  work, including migrations and SQLite/filesystem scheduling. Its outer Elapsed
  error cannot identify the stalled operation.
- Alternatives: enlarge the timeout, reset a timeout per phase, or change the
  assertions. Those would change the existing observation or its total budget.
- Choice: retain the exact public test name and all existing assertions. Each
  named phase receives the same absolute deadline derived once from the original
  ten-second bound. It gets only the remaining time, never another ten seconds.
- Impact: future failures name the actual phase. This is test-only diagnostic
  precision; no notifier, writer, schema, request, runtime bound or policy changes.

The original loaded-run failure remains recorded. Publishing happens after
commit and before the write ACK; a lifecycle-wide timeout is not evidence that
changed() waited ten seconds after that ACK. The diagnostic change neither
classifies the original runtime phase nor erases it. Its own source/narrow proof
is separate from the coordinator-authorized observation of the unchanged case.
