## Subagent track, provider history and explicit managed detach.
# Subagent label or complete safe notice; keep controls short.
subagents-title = Subagents
# Subagent label or complete safe notice; keep controls short.
subagents-back = Back to parent
# Subagent label or complete safe notice; keep controls short.
subagents-close = Close
# Subagent label or complete safe notice; keep controls short.
subagents-refresh = Refresh
# Subagent label or complete safe notice; keep controls short.
subagents-empty = No visible subagents
# Subagent label or complete safe notice; keep controls short.
subagents-loading = Loading subagents…
# Subagent label or complete safe notice; keep controls short.
subagents-partial = Some subagent history is unavailable.
# Subagent label or complete safe notice; keep controls short.
subagents-missing = Subagent history is unavailable.
# Subagent label or complete safe notice; keep controls short.
subagents-unsupported = This node does not support subagent relationships yet.
# Subagent label or complete safe notice; keep controls short.
subagents-untitled = Untitled subagent
# Subagent label or complete safe notice; keep controls short.
subagents-managed = Managed session
# Subagent label or complete safe notice; keep controls short.
subagents-provider = Provider task
# Subagent label or complete safe notice; keep controls short.
subagents-running = Running
# Subagent label or complete safe notice; keep controls short.
subagents-waiting = Waiting for input
# Subagent label or complete safe notice; keep controls short.
subagents-idle = Idle
# Subagent label or complete safe notice; keep controls short.
subagents-completed = Completed
# Subagent label or complete safe notice; keep controls short.
subagents-failed = Failed
# Subagent label or complete safe notice; keep controls short.
subagents-canceled = Canceled
# Subagent label or complete safe notice; keep controls short.
subagents-ended = Ended
# Subagent label or complete safe notice; keep controls short.
subagents-unknown = Status unknown
# Subagent label or complete safe notice; keep controls short.
subagents-detach = Detach
# Subagent label or complete safe notice; keep controls short.
subagents-detach-title = Detach subagent?
# Subagent label or complete safe notice; keep controls short.
subagents-detach-body = Make this session independent. Its process, workspace and conversation continue.
# Subagent label or complete safe notice; keep controls short.
subagents-detaching = Detaching…
# Subagent label or complete safe notice; keep controls short.
subagents-hide = Hide from track
# Subagent label or complete safe notice; keep controls short.
subagents-archive-finished = Archive finished
# Subagent label or complete safe notice; keep controls short.
subagents-read-only = Provider task · read-only conversation
# Subagent label or complete safe notice; keep controls short.
subagents-older = Load earlier history
# Subagent label or complete safe notice; keep controls short.
subagents-offline = The machine is offline.
# Subagent label or complete safe notice; keep controls short.
subagents-identity-changed = The machine identity changed. Refresh before continuing.
# Subagent label or complete safe notice; keep controls short.
subagents-timeout = The subagent read timed out. Try again.
# Subagent label or complete safe notice; keep controls short.
subagents-uncertain = Detach may have completed. Refresh before trying again.
# Subagent label or complete safe notice; keep controls short.
subagents-invalid-reply = The node returned an invalid subagent result.
# Subagent label or complete safe notice; keep controls short.
subagents-refused = The node refused this subagent request.
# Subagent label or complete safe notice; keep controls short.
subagents-busy = A subagent request is already running.
# Subagent label or complete safe notice; keep controls short.
subagents-exhausted = Reopen the app before another subagent request.
# Whole track count. $count is an exact visible-row count.
subagents-count = { NUMBER($count) ->
    [one] { NUMBER($count) } subagent
   *[other] { NUMBER($count) } subagents
}
# Retained page budget; existing history remains visible.
subagents-retention-limit = Earlier provider history exceeds this pane’s retained-history limit.

# Active fan-out segments keep their own count and status; $count is a visible-row count.
subagents-pill-failed = { NUMBER($count) } failed
# Active fan-out segments keep their own count and status; $count is a visible-row count.
subagents-pill-working = { NUMBER($count) } working
