# Diagnostics collection framing. No private values are interpolated.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-client = Client
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-desktop = Desktop
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-hosts = Hosts
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-pending = Waiting
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-running = Collecting
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-done = Collected
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-offline = The host is not connected. Reconnect and refresh.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-busy = Diagnostics is busy. Try again when the current read finishes.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-unavailable = This diagnostic source is not available in this build.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-unsupported = This host does not support this diagnostic.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-forbidden = This device cannot read this diagnostic. Check its host access.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-timed-out = The diagnostic timed out. Refresh to try again.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-invalid-reply = The host returned an invalid diagnostic. Refresh after reconnecting.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-failed = The diagnostic could not be collected. Refresh to try again.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-too-large = The combined report is too large. Run a report for one host.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-exhausted = No more diagnostic requests can be issued. Reopen the app.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-daemon = Daemon diagnostics
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-process = Process diagnostics
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-logs = Daemon logs
# Independent basic and daemon source collection, not a refresh of credentials.
diagnostic-flow-daemon-note = Basic host information and the daemon report are collected independently.
# Only the daemon self-process numeric metadata; unsupported sources remain explicit.
diagnostic-flow-process-note = Only the daemon’s own numeric process metadata is collected. Unsupported sources stay explicit.
# The bounded failed-call summary excludes raw daemon logs.
diagnostic-flow-logs-note = Raw daemon logs are not collected. The report includes only a bounded summary of failed node calls.
# Projection allowlist for the shareable report, not a general redaction guarantee.
diagnostic-flow-privacy = Reports use allowlisted host metadata. Host names, routes, keys, paths and raw errors are excluded.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-report = Run report
# Heading and per-host summary for host diagnostics, not a partial-status badge.
diagnostic-flow-partial = Host diagnostics
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-no-hosts = No hosts are saved. You can still copy the client report.

# Short source availability badge; the row description carries the full explanation.
diagnostic-flow-unavailable-badge = Unavailable

# A settled host read whose independent sources are incomplete.
diagnostic-flow-collected-partial = Collected with unavailable sections

# A reconnect invalidates the previous observation, including same-node reconnects.
diagnostic-flow-connection-changed = The host connection changed. Run the report again.

# Availability is resolved by each explicit host report, never inferred from this badge.
diagnostic-flow-host-dependent = Depends on host
