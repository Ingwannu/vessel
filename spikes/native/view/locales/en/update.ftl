## Updates in the title bar, About and the updated-on-start banner.
## About follows Paseo settings.about, desktop.updates.status and desktop.daemon.logs.
## Title-bar wording keeps the existing Zed-inspired labels. Values are data,
## and every dynamic message is formatted before a frame is drawn.

# Title bar while checking for a build. Keep short.
update-checking = Checking for updates…

# Title-bar action once a build is ready. Keep short.
update-restart = Restart to update

# Title-bar status when an attempt fails. Keep short.
update-failed = Update failed

# Title-bar retry action. Keep short.
update-try-again = Try again

# Settings page title. Keep short.
update-about = About

# About: running app's version row title. Keep short.
update-app-version = App version

# About: hint below the running version's row title. Keep short.
update-this-device = This device

# About: update status row title. Keep short.
update-app-updates = App updates

# About: check now action. Keep short.
update-check = Check

# About: check action while waiting. Keep short.
update-checking-short = Checking...

# About: install action before a newer build is ready. Keep short.
update-update = Update

# About: a check is running.
update-status-checking = Checking for app updates...

# About: the running build is the newest.
update-status-up-to-date = App is up to date.

# About: the attempt failed; the safe explanation is shown below.
update-status-failed = Failed to update app.

# About: no check has been made this run.
update-status-idle = Update status has not been checked yet.

# About: application log row title. Keep short.
update-log-file = Log file

# About: open the log folder. Keep short.
update-open-logs = Open logs

# About and General: copy a folder's path. Keep short.
update-copy-path = Copy path

# Frame notice after the log folder path was copied.
update-log-path-copied = Log path copied.

# About: no log folder path is known.
update-log-unavailable = Log path unavailable

# About: open this build's release notes. Keep short.
update-whats-new = What's new

# About: hint below the release-notes action.
update-whats-new-hint = Release notes for this version

# Install confirmation dialog title. Keep short.
update-install-title = Install desktop update

# Install confirmation: this app on this computer will be replaced.
update-install-message = This updates Vessel on this computer

# Install confirmation action. Keep short.
update-install-confirm = Install update

# Close install confirmation without installing. Keep short.
update-cancel = Cancel

# About: automatic-update switch label. Keep short.
update-automatic = Update automatically

# About: effect of disabling automatic checks.
update-automatic-note = When off, Vessel looks for a new build only when you check.

# About: the development build has no updater.
update-no-updates = This build does not update itself (a development build).

# Dismiss the updated-on-start banner. Keep short.
update-dismiss = Dismiss

# Title-bar progress. $percent is a numeric percentage. Keep short.
update-downloading = Downloading update… { NUMBER($percent) }%

# Banner after restarting into a build. $label is its raw release label.
update-updated-to = Updated to { $label }

# Numbered build display. $label is the release label without v; words such as dev stay data.
update-version = v{ $label }

# About install action for a ready numbered build. $label has no v prefix. Keep short.
# $kind selects a numbered release or a build word such as dev.
update-update-to = { $kind ->
    [numbered] Update to v{ $label }
   *[word] Update to { $label }
    }

# About: a numbered build is downloading. $label has no v prefix.
# $kind selects a numbered release or a build word such as dev.
update-status-downloading = { $kind ->
    [numbered] Update found: v{ $label }. Downloading...
   *[word] Update found: { $label }. Downloading...
    }

# About: a numbered build is ready. $label has no v prefix.
# $kind selects a numbered release or a build word such as dev.
update-status-ready = { $kind ->
    [numbered] Update ready: v{ $label }
   *[word] Update ready: { $label }
    }

# Safe fallback only if message preparation itself fails; diagnostics stay in logs.
update-details-unavailable = Update information is unavailable.
