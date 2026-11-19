## Settings Diagnostics controls, after Paseo's settings.diagnostics.app.
## The hint names Vessel's machines and agents where Paseo names its daemon
## and providers. Short labels are buttons. The report is fixed technical data.

# The Settings navigation entry and page title. Keep short.
diagnostics-title = Diagnostics

# The report row and its dialog title. Keep short.
diagnostics-report-title = App diagnostic

# The Run row hint. The copied technical report retains its fixed schema.
diagnostics-description = Collect connection, machine, agent, and log details

# Collect the report and open its dialog. Keep short.
diagnostics-run = Run

# Copy the collected report. Keep short.
diagnostics-copy = Copy

# Collect the report again. Keep short.
diagnostics-refresh = Refresh

# The copied-toast label for the report, lower case after "Copied" (Paseo's settings.diagnostics.app.copyLabel).
diagnostics-copied-label = diagnostic

## The app diagnostic's progress (Paseo's settings.diagnostics.app.running and
## its progress rows, "Client" said "App" here). A label is a machine's name.

# Shown while the first diagnostic is collected, beside a spinner.
diagnostics-running = Running diagnostic...

# The progress row for the app's own facts. Keep short.
diagnostics-step-app = App

# A progress row while a machine is asked. $label is the step's name.
diagnostics-step-running = { $label }: running

# A progress row once a machine answered, or had nothing to ask.
diagnostics-step-done = { $label }: done

# A progress row when asking a machine failed.
diagnostics-step-failed = { $label }: failed

# The Copy action's accessible name in a diagnostic's title row (Paseo's copyAccessibility).
diagnostics-copy-label = Copy diagnostic

# The Refresh action's accessible name in a diagnostic's title row (Paseo's refreshAccessibility).
diagnostics-refresh-label = Refresh diagnostic

# The Refresh action's accessible name while it collects (Paseo's refreshingAccessibility).
diagnostics-refreshing-label = Refreshing diagnostic

## An agent's Diagnostic (Paseo's settings.providers.diagnostic).

# The button beside an agent's details, and its dialog's title. Keep short.
diagnostics-agent = Diagnostic

# The dialog's body when nothing was collected.
diagnostics-agent-none = No diagnostic available

# Why a diagnostic, or a node's log, could not be read: no connection.
diagnostics-problem-offline = The machine is not connected.

# Why it could not be read: the node refused. $code is the wire code, such as "forbidden".
diagnostics-problem-refused = The node refused it ({ $code }).

# Why it could not be read: no answer within the call's bound.
diagnostics-problem-timed-out = The machine did not answer in time.

## A machine's log (Paseo's desktop.daemon.logs, "daemon" said "node").

# The Overview section holding the log row (Paseo's desktop.daemon.title, "Daemon" said "Node"). Keep short.
diagnostics-log-section = Node


# The Overview row's title. Keep short.
diagnostics-log-title = Log file

# The row's hint and the dialog's path line when the path cannot be read.
diagnostics-log-unavailable = Log path unavailable

# Copy the log's path. Keep short.
diagnostics-log-copy-path = Copy path

# Open the log's dialog. Keep short.
diagnostics-log-open = Open logs

# The log dialog's title.
diagnostics-log-dialog = Node logs

# The dialog's contents when the file has no lines.
diagnostics-log-empty = (log file is empty)

# The toast after the path was copied.
diagnostics-log-copied = Log path copied.

# Shown while the dialog reads the log, beside a spinner.
diagnostics-log-loading = Loading node logs...

# The row's hint when the node writes to the terminal it was started in.
diagnostics-log-terminal = This node writes its log to the terminal it was started in.

# The row's hint when the node's log goes anywhere but a file it keeps.
diagnostics-log-elsewhere = This node's log is not a file it keeps (a service manager's journal, for one).
