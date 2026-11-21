## Settings › Plugins (ADR 0115): a machine's plugins, in Paseo's words
## (settings.plugins.*) with "machine" for its "host", as the page's
## section says them. $id is an installation id such as "night".

# Section heading above the page's cards; keep short. No values.
plugins-title = Plugins

# Warning title at the top of the page. No values.
plugins-trusted-title = Plugins are trusted code

# Warning text under its title. No values.
plugins-trusted-description = They run on this machine without sandboxing. Install only code you trust.

# Row title of the machine-wide switch; keep short. No values.
plugins-global-title = Enable plugins

# Hint under the machine-wide switch. No values.
plugins-global-hint = Global switch for every configured plugin

# Label above the install field; keep short. No values.
plugins-source-label = Plugin source

# Placeholder of the install field: the one source this protocol installs. No values.
plugins-source-placeholder = Directory on this machine, ~/… or /…

# Button that installs the field's source; keep short. No values.
plugins-install = Install plugin

# The install button while it installs; keep short. No values.
plugins-installing = Installing…

# Status badge of a running plugin; keep short. No values.
plugins-status-running = running

# Status badge of a disabled plugin; keep short. No values.
plugins-status-disabled = disabled

# Status badge of a plugin that could not start; keep short. No values.
plugins-status-failed = failed

# A row's "…" menu: its name, and its title as a sheet on a phone. $id is the installation.
plugins-menu = Actions for { $id }

# Spoken name of a row's switch while the plugin is off: what pressing it does.
plugins-switch-enable = { $id }: Enable

# Spoken name of a row's switch while the plugin is on.
plugins-switch-disable = { $id }: Disable

# Menu row that reloads a plugin from disk; keep short. No values.
plugins-reload = Reload

# The reload row while it reloads; keep short. No values.
plugins-reloading = Reloading…

# Menu row, and the confirmation's button, that removes a plugin; keep short. No values.
plugins-remove = Remove

# The remove row while it removes; keep short. No values.
plugins-removing = Removing…

# Menu row that opens a plugin's logs; keep short. No values.
plugins-logs = Logs

# Note after an install. $id is the installation made.
plugins-installed = Installed { $id }

# Note after a reload.
plugins-reloaded = Reloaded { $id }

# Note after a plugin's switch was turned on.
plugins-enabled = Enabled { $id }

# Note after a plugin's switch was turned off.
plugins-disabled = Disabled { $id }

# Note after a removal.
plugins-removed = Removed { $id }

# Note after a removal was cancelled.
plugins-kept = Kept { $id }

# Note after the machine-wide switch was turned on. No values.
plugins-global-enabled = Plugins enabled

# Note after the machine-wide switch was turned off. No values.
plugins-global-disabled = Plugins disabled

# Title of the removal question.
plugins-remove-title = Remove { $id }?

# Text of the removal question. No values.
plugins-remove-message = This removes the configuration. The source directory is not deleted.

# The removal question's button that keeps the plugin; keep short. No values.
plugins-cancel = Cancel

# Warning title while the machine is not connected. No values.
plugins-offline-title = Plugin host is offline

# Warning text under it. No values.
plugins-offline-description = Reconnect to this machine to manage its plugins.

# Warning title for a machine whose Vessel predates the plugin calls. No values.
plugins-update-title = Update this machine to manage plugins

# Shown while the list is first read. No values.
plugins-loading = Loading plugins…

# Error title when the list could not be read; the reason follows. No values.
plugins-error-title = Unable to load plugins

# Button that reads the list again; keep short. No values.
plugins-retry = Retry

# Shown when nothing is installed. No values.
plugins-empty = No plugins configured

# A row's source line. $path is the directory on the machine.
plugins-directory = Directory: { $path }

# Title of a plugin's logs dialog.
plugins-logs-title = Logs: { $id }

# The logs dialog before the first answer. No values.
plugins-logs-loading = Loading logs…

# Button that reads the logs again; keep short. No values.
plugins-logs-refresh = Refresh

# The logs' Refresh while it reads; keep short. No values.
plugins-logs-refreshing = Refreshing…

# Error title when the logs could not be read; the reason follows. No values.
plugins-logs-error-title = Unable to load plugin logs

# The logs dialog when the plugin wrote nothing. No values.
plugins-logs-empty = No plugin output yet

# Why a call failed: no connection, so nothing was sent. No values.
plugins-problem-offline = The machine is not connected. Check that it is on and Vessel is running there.

# Why a call failed: the connection dropped while waiting. No values.
plugins-problem-dropped = The connection dropped before the answer; it may or may not have happened.

# Why a call failed: no answer in time. No values.
plugins-problem-timed-out = The machine did not answer in time; it may or may not have happened.

# Why a call failed: the machine's Vessel has no such call. No values.
plugins-problem-unsupported = That machine runs an older Vessel; update it to use this.

# Why a call failed, when the node gave a code but no words. $code is the wire code, such as "forbidden".
plugins-problem-refused = The machine refused it ({ $code }).

# A plugin screen whose machine is not connected (Paseo's "Plugin host is offline.", with "host" read as "machine"). No values.
plugins-screen-offline = Plugin machine is offline.

# A plugin screen that no running plugin declares (Paseo's words). No values.
plugins-screen-unavailable = This plugin surface is unavailable.

# The button that closes a plugin screen (Paseo's "Close plugin"). No values.
plugins-screen-close = Close plugin

# The tooltip on that button (Paseo's "Close"); the button's name stays "Close plugin". No values.
plugins-screen-close-tip = Close

# The line at the top of a plugin screen on a device without admin. $machine is the machine's name.
plugins-screen-read-only = Changing this needs admin access to { $machine }.
