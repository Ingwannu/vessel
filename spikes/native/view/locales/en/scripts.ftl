## Project detail: supervised scripts and observed listeners (WP-B1).

# Project scripts: short action/status, or complete explanatory notice.
scripts-title = Scripts and services

# Project scripts: short action/status, or complete explanatory notice.
scripts-refresh = Refresh

# An original detached operation survives UI retirement; a receipt is not success.
scripts-retired = This tab changed while a request was in flight. Its original request is retained, and its reply will be kept. A failed reply can still mean an unknown remote outcome. Reload current status before any new action; the request will not be replayed.

# Explicit reload only after the original request returns; never replay it.
scripts-reload-retired = Reload current status

# Owner-local retention limit refuses another panel without dropping requests.
scripts-retained-limit = Too many script views retain requests. Dismiss received results to free space; pending requests are kept.

# Local history only; dismissing must never cancel or replay a remote request.
scripts-retained-results = Earlier script requests are retained. A failed reply can still mean an unknown remote outcome. Dismissing clears received local notices only; pending requests are kept and no request is replayed.

# Explicit acknowledgement of exactly the received results shown now.
scripts-dismiss-received = Dismiss received results

# Project scripts: short action/status, or complete explanatory notice.
scripts-loading = Reading scripts…

# Project scripts: short action/status, or complete explanatory notice.
scripts-empty = This project has no configured scripts or services.

# Project scripts: short action/status, or complete explanatory notice.
scripts-run = Run

# Project scripts: short action/status, or complete explanatory notice.
scripts-restart = Restart

# Project scripts: short action/status, or complete explanatory notice.
scripts-stop = Stop

# Project scripts: short action/status, or complete explanatory notice.
scripts-preview = Open preview

# Project scripts: short action/status, or complete explanatory notice.
scripts-host = Service host

# Project scripts: short action/status, or complete explanatory notice.
scripts-host-hint = Enter the host your browser can reach; remote loopback ports may need your own tunnel.

# Project scripts: short action/status, or complete explanatory notice.
scripts-host-placeholder = localhost or a remote host

# Project scripts: short action/status, or complete explanatory notice.
scripts-pending = Working…

# Project scripts: short action/status, or complete explanatory notice.
scripts-idle = Not started

# Project scripts: short action/status, or complete explanatory notice.
scripts-starting = Starting

# Project scripts: short action/status, or complete explanatory notice.
scripts-running = Running

# Project scripts: short action/status, or complete explanatory notice.
scripts-backoff = Waiting to restart

# Project scripts: short action/status, or complete explanatory notice.
scripts-exited = Exited

# Project scripts: short action/status, or complete explanatory notice.
scripts-failed = Failed

# Project scripts: short action/status, or complete explanatory notice.
scripts-stopped = Stopped

# Project scripts: short action/status, or complete explanatory notice.
scripts-unknown = Unknown state

# Project scripts: short action/status, or complete explanatory notice.
scripts-ports = Discovered ports

# Project scripts: short action/status, or complete explanatory notice.
scripts-no-ports = No listeners found for this run.

# Project scripts: short action/status, or complete explanatory notice.
scripts-scan-pending = The first port scan is pending.

# Project scripts: short action/status, or complete explanatory notice.
scripts-scan-failed = Port discovery is unavailable. Refresh to try again.

# Project scripts: short action/status, or complete explanatory notice.
scripts-scan-more = The node has more listeners than this list shows.

# Project scripts: short action/status, or complete explanatory notice.
scripts-approval = Review and approve this project’s current commands above before running scripts.

# Project scripts: short action/status, or complete explanatory notice.
scripts-read-failed = Scripts could not be read. Refresh to try again.

# Project scripts: short action/status, or complete explanatory notice.
scripts-call-failed = The request did not complete. Refresh before trying another action; a timed-out command may already have started.

# Project scripts: short action/status, or complete explanatory notice.
scripts-no-project = The run has no project.

# Project scripts: short action/status, or complete explanatory notice.
scripts-missing-script = The configured script no longer exists. Refresh the project.

# Project scripts: short action/status, or complete explanatory notice.
scripts-missing-dir = The configured working directory is unavailable.

# Project scripts: short action/status, or complete explanatory notice.
scripts-invalid-file = The project file is invalid; no script was started.

# Project scripts: short action/status, or complete explanatory notice.
scripts-already-running = The script is already running. Refresh to see it.

# Project scripts: short action/status, or complete explanatory notice.
scripts-refused = The node refused this action. Refresh the project.

# Project scripts: short action/status, or complete explanatory notice.
scripts-missing-host = Choose the service host before opening a preview.

# Project scripts: short action/status, or complete explanatory notice.
scripts-invalid-host = Use a DNS name or IP address without credentials, port or path.

# Project scripts: short action/status, or complete explanatory notice.
scripts-invalid-port = This listener has no usable TCP port.

# Project scripts: short action/status, or complete explanatory notice.
scripts-exhausted = Request identifiers are exhausted. Reopen the app.

# Project scripts: short action/status, or complete explanatory notice.
scripts-script = Script

# Project scripts: short action/status, or complete explanatory notice.
scripts-service = Service

# Project scripts: short action/status, or complete explanatory notice.
scripts-root = Project root

# Project scripts: short action/status, or complete explanatory notice.
scripts-copy = Copy URL

# Port row refusal for a remote loopback socket; a tunnel is operator-managed.
scripts-remote-loopback = This listener is loopback-only on the remote machine. Set up a tunnel and enter its local host.
# A preview callback must recheck its observed socket before opening the browser.
scripts-stale = Refresh this listener before opening a preview.
# Keep old supervised processes accessible after their configuration is removed.
scripts-retained = Runs no longer in the project file
# Never treat an unrecognized newer-node setup state as successful completion.
scripts-setup-unknown = This app cannot interpret the setup state. No session was started; update the app and inspect the worktree.
# A listed command set contains approved fields this app cannot fully display.
scripts-unsupported-config = This command set contains configuration this app cannot fully show. Update the app before approving or running it.
# A read of invalid configuration does not imply that prior supervised runs stopped.
scripts-invalid-config = The project file is invalid. No new run is authorized; existing runs remain available below.

# Declaration is separate from observed listening sockets.
scripts-declared-port = Declared port

# Cancels the client creation walk; it does not signal the setup runner.
scripts-cancel-creation = Cancel session creation

# Exact automatic terminal definitions covered by a project digest.
scripts-automatic-terminals = Automatic terminals (file order)
# Approval: title shown when an automatic terminal has no configured name.
scripts-terminal-name-absent = Unnamed terminal
# Approval: summary when a terminal name, command or file order changed.
scripts-terminal-changes = Automatic terminal changes are shown below
# Approval: complete terminal inspection does not enable pending approval support.
scripts-terminal-approval-pending = These terminal definitions are shown for inspection. Approval support for this configuration is not enabled yet.

# Passive reveal of this exact script `TerminalId` in the same workspace layout.
scripts-open-terminal = Open terminal

# Runtime facts are distinct from the current project file declaration.
scripts-observed-run = Recorded run command and directory
# Existing script.restart repeats its captured run plan, even if the file changed.
scripts-restart-uses-run = Restart repeats this run's command and working directory.

# Original operation receipt only; never current process or health authority.
scripts-receipt-pending = Waiting for the original request to return.

# Original operation receipt only; never current process or health authority.
scripts-receipt-uncertain = The request failed; its remote outcome may still be unknown.

# Original operation receipt only; never current process or health authority.
scripts-receipt-acknowledged = The node acknowledged this request. Refresh current status before another action.

# Original operation receipt only; never current process or health authority.
scripts-receipt-started = The node reported a started run. Refresh current status; this receipt does not show current health.
