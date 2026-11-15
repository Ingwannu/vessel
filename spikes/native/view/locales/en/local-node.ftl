# Desktop local-node control: title. No credentials or process IDs.
local-node-title = Local node
# Desktop local-node control: manage. No credentials or process IDs.
local-node-manage = Manage local node
# Desktop local-node control: manage-note. No credentials or process IDs.
local-node-manage-note = Start the bundled local node when Vessel opens.
# Desktop local-node control: status. No credentials or process IDs.
local-node-status = Status
# Desktop local-node control: keep. No credentials or process IDs.
local-node-keep = Keep running after quit
# Desktop local-node control: keep-note. No credentials or process IDs.
local-node-keep-note = Leave the local node running when closing Vessel. Attached nodes are always left running.
# Desktop local-node control: start. No credentials or process IDs.
local-node-start = Start node
# Desktop local-node control: restart. No credentials or process IDs.
local-node-restart = Restart node
# Desktop local-node control: stop. No credentials or process IDs.
local-node-stop = Stop node
# Desktop local-node control: not-installed. No credentials or process IDs.
local-node-not-installed = The local node is not installed in this build.
# Desktop local-node control: paused. No credentials or process IDs.
local-node-paused = Automatic local management is paused.
# Desktop local-node control: working. No credentials or process IDs.
local-node-working = Updating local node state…
# Desktop local-node control: owned. No credentials or process IDs.
local-node-owned = Managed by this app
# Desktop local-node control: attached. No credentials or process IDs.
local-node-attached = Attached to an existing local node; stop and restart unavailable
# Desktop local-node control: kept. No credentials or process IDs.
local-node-kept = Left running after releasing app ownership.
# Desktop local-node control: stopped. No credentials or process IDs.
local-node-stopped = Stopped · process exit confirmed
# Desktop local-node control: disconnected. No credentials or process IDs.
local-node-disconnected = Connection closed; process state is unknown.
# Desktop local-node control: failed. No credentials or process IDs.
local-node-failed = Local node operation could not finish.
# Desktop local-node control: protected-note. No credentials or process IDs.
local-node-protected-note = Active sessions are protected. Stop or restart never interrupts them without confirmation.
# Desktop local-node control: registration-failed. No credentials or process IDs.
local-node-registration-failed = The local node could not be registered. Check saved machine names and retry.
# Desktop local-node control: quit-refused. No credentials or process IDs.
local-node-quit-refused = Vessel is still open because the local node could not safely stop.
# Desktop local-node control: keep-quit. No credentials or process IDs.
local-node-keep-quit = Keep node running and quit
# Desktop local-node control: interrupt-quit. No credentials or process IDs.
local-node-interrupt-quit = Interrupt active sessions and quit
# Desktop local-node control: cancel-quit. No credentials or process IDs.
local-node-cancel-quit = Stay in Vessel
# Explicit local-node log viewer: logs; never interpolate credentials.
local-node-logs = Local node logs
# Explicit local-node log viewer: refresh-logs; never interpolate credentials.
local-node-refresh-logs = Refresh
# Explicit local-node log viewer: logs-unavailable; never interpolate credentials.
local-node-logs-unavailable = No retained local-node logs are available for this launch.

# Settings host picker: start or resume the bundled managed-local node.
local-node-enable = Enable local node

# Copy only the app-managed local log path, never a remote address.
local-node-copy-log-path = Copy log path

# Explicit local-node confirmation action: confirm-stop. Applies only to the captured owned child.
local-node-confirm-stop = Stop local node?

# Explicit local-node confirmation action: confirm-restart. Applies only to the captured owned child.
local-node-confirm-restart = Restart local node?

# Explicit local-node confirmation action: interrupt-stop. Applies only to the captured owned child.
local-node-interrupt-stop = Interrupt sessions and stop

# Explicit local-node confirmation action: interrupt-restart. Applies only to the captured owned child.
local-node-interrupt-restart = Interrupt sessions and restart

# Leave a local-node stop/restart/quit confirmation without changing the process.
local-node-cancel = Cancel

# App-update restart refusal, after the owned node protected its active sessions.
local-node-confirm-update = Restart Vessel to update?
# Explicitly authorize interrupting the captured owned node before app-update restart.
local-node-interrupt-update = Interrupt sessions and restart to update

# Exact generated local destination collides with an existing saved book entry.
local-node-name-collision = A saved machine already uses this local node's generated name. Rename or remove that saved entry to register the local node.
# The canonical registry's one-local-source cap refused a distinct second source.
local-node-already-registered = Another managed local connection is already registered.
# The original authenticated source differs from the controller's expected node.
local-node-identity-changed = The managed local node's authenticated identity changed. Reconnect it before continuing.
# The controller withdrew its original transport; this is not proof of OS exit.
local-node-transport-retired = The managed local connection is no longer available. Reconnect it before continuing.

# Explicitly authenticate reuse after ownership was surrendered; never re-own the process.
local-node-reconnect = Reconnect local node
