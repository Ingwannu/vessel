## Git comparison surfaces. Paseo workspace.git.diff and diffViewer, plus
## Vessel's typed recovery sentences; diagnostics are separate code detail.

# Title of the Changes pane. Keep short.
git-panel-title = Changes
# Accessible label of the comparison mode picker. Keep short.
git-panel-mode = Diff mode
# Comparison choice: merge base against working files, including new files. Keep short.
git-panel-mode-all = All changes
# Comparison choice: HEAD against working files. Keep short.
git-panel-mode-uncommitted = Uncommitted
# Comparison choice: merge base against HEAD only. Keep short.
git-panel-mode-committed = Committed
# Tooltip and accessible name of the diff options trigger. Keep short.
git-panel-options = Diff options
# Menu choice that shows diff bodies directly inside Changes. Keep short.
git-panel-inline = Inline diff
# Layout choice for one column with both sides of a change. Keep short.
git-panel-unified = Unified diff
# Layout choice for old and new text in two columns. Keep short.
git-panel-split = Side-by-side diff
# Menu action enabling source-line wrapping. Keep short.
git-panel-wrap = Wrap long lines
# Menu action disabling wrapping and restoring horizontal scrolling. Keep short.
git-panel-no-wrap = Scroll long lines
# Menu action that omits whitespace-only changes. Keep short.
git-panel-hide-whitespace = Hide whitespace
# Menu action that restores whitespace-only changes. Keep short.
git-panel-show-whitespace = Show whitespace
# Menu action expanding every file body. Keep short.
git-panel-expand-files = Expand all files
# Menu action folding every file body. Keep short.
git-panel-collapse-files = Collapse all files
# Menu action expanding every changed-file folder. Keep short.
git-panel-expand-folders = Expand all folders
# Menu action folding every changed-file folder. Keep short.
git-panel-collapse-folders = Collapse all folders
# Button label for rereading git facts; it does not fetch remotes. Keep short.
git-panel-refresh = Refresh
# Refresh button label while rereading facts. Keep short.
git-panel-refreshing = Refreshing
# Title of the compact sheet that chooses a file to scroll to. Keep short.
git-panel-jump = Jump to file
# Status while the first repository status read is pending.
git-panel-checking = Checking repository...
# Empty state only when authoritative project facts say this is not a repository.
git-panel-not-repository = Not a git repository
# Empty state after a complete comparison contains no changed files.
git-panel-empty = No changes to display
# Empty-state action switching from Committed to Uncommitted. Keep short.
git-panel-see-uncommitted = See uncommitted changes
# Empty-state action switching from Uncommitted to Committed. Keep short.
git-panel-see-committed = See committed changes
# Heading when the bounded diff document cannot be previewed.
git-panel-too-large-title = This diff is too large to preview
# Next step beneath the too-large preview heading.
git-panel-too-large-description = Narrow the comparison to preview it here
# Placeholder for a binary file body. Keep short.
git-panel-binary = Binary file
# Heading of the collapsible branch commit section. Keep short.
git-panel-commits = Commits
# Accessible description of the history loading skeleton.
git-panel-commits-loading = Loading commits…
# Empty state for a complete empty commit listing.
git-panel-commits-empty = No commits yet
# Complete recovery sentences; never infer these from git output.
# Complete recovery after the app could not enqueue a read; nothing reached the machine.
git-panel-problem-unsent = Could not send. Try again.
# Complete recovery after losing the machine connection.
git-panel-problem-connection = The machine is disconnected. Reconnect and refresh the changes.
# Complete recovery after a read grant was refused.
git-panel-problem-access = This device cannot read changes on this machine. Check its access.
# Complete recovery for an older or unassigned protocol method.
git-panel-problem-version = Update Vessel on this device and the machine to view these changes.
# Complete recovery when the registered folder no longer exists.
git-panel-problem-missing = This folder is no longer available. Open another workspace.
# Complete recovery for a transient read refusal or timeout.
git-panel-problem-unavailable = The machine cannot read these changes right now. Refresh and try again.
# Complete recovery for an otherwise unclassified read failure; diagnostic is separate.
git-panel-problem-failed = The machine could not read these changes. Refresh and try again.
# Complete recovery when the machine cannot provide this read; this does not say the folder lacks git.
git-panel-problem-unsupported = This machine cannot provide these changes. Check Git and Vessel on the machine, then refresh.
# Alert heading for a failed status or comparison read; recovery and git output follow separately.
git-panel-failed-title = Changes could not be read

# Complete recovery when the selected authenticated connection changed; never choose another alias.
git-panel-problem-destination = The connection changed. Reopen this workspace and refresh the changes.
# Complete recovery when a file body names a different path; never install it under the requested file.
git-panel-problem-file = The machine returned another file’s changes. Refresh and try again.

# Neutral recovery for different stored/requested base refs; both identities are separate code detail.
git-panel-problem-base = The stored and requested base branches differ. Check both branches and choose the comparison again.

# Brief commit acknowledgement inside the same git menu row; exact facts remain in the success notice.
git-panel-commit-success = Committed

# Brief push acknowledgement inside the same git menu row; exact facts remain in the success notice.
git-panel-push-success = Pushed

# Brief pull acknowledgement inside the same git menu row; exact facts remain in the success notice.
git-panel-pull-success = Pulled

# Brief fetch acknowledgement inside the same git menu row; exact facts remain in the success notice.
git-panel-fetch-success = Fetched

# Complete recovery when the app's bounded concurrent read admission is full; no node request began.
git-panel-problem-capacity = Too many changes are being read at once. Try again.
# Complete recovery after this read's deadline; this does not claim the connection is lost.
git-panel-problem-timeout = This read did not finish in time. Try again.

# Complete guidance when the canonical workspace root has not been admitted; do not infer non-Git.
git-panel-problem-workspace = Workspace repository information is unavailable. Open a registered workspace and try again.
# Complete recovery when lifetime correlation identities are exhausted; retry alone cannot help.
git-panel-problem-keys = No more requests can be identified safely. Restart Vessel and try again.

# Honest notice when the node marks the summary incomplete; zero listed files is not an empty comparison.
git-panel-partial = Some changed files are not listed. Refresh or narrow the comparison.

# Label before the exact original identity in a diagnostic code block.
git-panel-detail-expected = Expected

# Label before the exact peer identity in a diagnostic code block.
git-panel-detail-received = Received

# Label before the exact recorded worktree base ref; no particle follows its name.
git-panel-detail-stored-base = Stored base branch

# Label before the exact requested branch or commit base in a diagnostic code block.
git-panel-detail-requested-base = Requested comparison base
