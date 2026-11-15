## Read-only observations of one backing setup attempt, not workspace admission.
# A missing structured snapshot does not mean setup succeeded.
setupobs-absent = No structured setup observation is available.
# Missing capability or canonical admission cannot enable the observation backend.
setupobs-unsupported = Structured setup observations are not available here yet.
# Safe failure label; raw peer diagnostics are never shown here.
setupobs-read-failed = Setup observation could not be refreshed.
# Overall and per-command structured status, independent from output text.
setupobs-running = Running
# Only the structured runner result establishes completion.
setupobs-completed = Completed
# The structured runner reported a failure.
setupobs-failed = Failed
# Cancellation is confirmed by a terminal runner state, not a signal receipt.
setupobs-cancelled = Cancelled
# A newer or missing state is never interpreted as success.
setupobs-unknown = Unknown state
# Approved but not yet reached by this invocation.
setupobs-waiting = Waiting
# A completed empty setup says nothing about workspace trust or service readiness.
setupobs-no-commands = This observation contains no setup commands.
# Explicit generation-bound cancellation of the displayed invocation.
setupobs-cancel = Cancel this setup attempt
# Result when a process ended without a known exit code.
setupobs-exit-unknown = Ended with an unknown exit code
# Structured process creation failure.
setupobs-spawn-failed = Could not start the command
# Structured setup command deadline result.
setupobs-timed-out = Timed out
# Command preparation or output recording failed.
setupobs-io-failed = Could not prepare or record the command
# The producer retained head and tail but omitted middle output.
setupobs-truncated = Some output was omitted from the middle.
# An expanded command may not have written printable output.
setupobs-no-output = No output yet.
# A cancellation signal acknowledgement is not a reaped process result.
setupobs-cancel-requested = Cancellation requested. Waiting for the runner to finish.
# Read the next structured observation for the actual terminal state.
setupobs-cancel-not-running = The runner reports that this attempt is no longer running.
# An old generation's action did not cancel its replacement.
setupobs-cancel-stale = The displayed attempt changed. Nothing was cancelled.
# An unrecognized receipt cannot establish completed cancellation.
setupobs-cancel-unknown = The cancellation result is unknown. Check the runner state.
