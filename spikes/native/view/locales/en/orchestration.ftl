## Host orchestration settings. Labels use the host's actual state, not a draft.
# Section and row titles. Keep short.
# Host settings heading above the tools switch.
orchestration-section = Orchestration
# Switch label for the host-controlled agent tools server.
orchestration-tools-title = Enable Vessel tools
# Supporting text below the tools switch; own means authority stays scoped.
orchestration-tools-hint = Agents can manage their own sessions, worktrees and schedules.
# Host settings heading above skill management.
orchestration-skills-section = Skills
# Switch label for installed orchestration skill bundles.
orchestration-skills-title = Orchestration skills
# Supporting text below the skills switch.
orchestration-skills-hint = Install bundled skills for your agents.
# Status hint when bundled managed files need maintenance.
orchestration-update-available = A bundled skill update is available.
# Complete recovery messages; server diagnostic text never appears here.
# Status shown while reading the chosen host; no mutation is implied.
orchestration-loading = Loading orchestration settings…
# Recovery hint when the chosen machine has no authenticated connection.
orchestration-unavailable = Connect to this machine to manage its orchestration settings.
# Shown for an older or feature-unavailable host; no request is invoked.
orchestration-unsupported = This machine does not support orchestration settings yet.
# Generic operation failure; private server diagnostics are never appended.
orchestration-failed = The operation could not be completed. Read the current state and try again.
# Permission refusal for host orchestration settings, not an agent permission prompt.
orchestration-access = This device is not allowed to change orchestration settings.
# A stale target or changed removal plan requires a fresh read.
orchestration-changed = The target or removal plan changed. Read the current state and try again.
# Bounded operation queue refused admission; the user may retry later.
orchestration-busy = Another orchestration operation is still running. Try again shortly.
# Deadline expired; the operation may still finish, so refresh before another write.
orchestration-unconfirmed = The change could not be confirmed. Refresh before trying again.
# Request identities cannot be allocated safely until the app restarts.
orchestration-exhausted = Restart Vessel before making another orchestration request.
# Short actions and standalone operation labels in confirmation lists.
# Button to read actual host state again without changing the picker draft.
orchestration-refresh = Refresh
# Button that opens confirmation for bundled additions and updates.
orchestration-update = Update
# Gear action and title of the skill selection dialog.
orchestration-choose = Choose skills
# Button committing the selection draft; removal may require separate confirmation.
orchestration-save = Save
# Disabled loading button while the selection acknowledgement is outstanding.
orchestration-saving = Saving…
# Dismiss without writing; disabled during an outstanding selection save.
orchestration-cancel = Cancel
# Destructive confirmation button for freshly listed deselected folders.
orchestration-remove = Remove
# Destructive confirmation button removing all installed managed skill folders.
orchestration-uninstall = Uninstall
# Loading label during a host skill operation.
orchestration-working = Working…
# Standalone operation verb beside a skill directory name in a confirmation list.
orchestration-add = Add
# Standalone maintenance verb beside a skill directory name.
orchestration-operation-update = Update
# Button opening the trusted public skill documentation on an explicit click.
orchestration-docs = Read about skills
# Selection sheet. Draft is seeded only when opened.
# Selection dialog switch: use all current and future bundled skills.
orchestration-all-title = All skills
# Explains that all is dynamic across future release catalogues.
orchestration-all-hint = Include skills added in future updates.
# Heading above per-skill selection switches.
orchestration-available = Available skills
# Selection dialog empty state when the host advertises no bundled skill names.
orchestration-empty = No bundled skills are available.
# Explicit confirmations. Names appear as a separate data list below the sentence.
# Confirmation dialog title before removing all managed skill folders.
orchestration-uninstall-title = Uninstall skills?
# Warns that whole folders include user additions; saved selection survives.
orchestration-uninstall-message = This removes whole skill folders, including added files. Your selection is kept for reinstall.
# Confirmation dialog title for freshly computed deselected skill folders.
orchestration-remove-title = Remove deselected skills?
# Deletion warning; exact folder names follow as separate data rows.
orchestration-remove-message = The machine found these folders to remove. All files in them will be deleted.
# Confirmation title for non-destructive bundled skill maintenance.
orchestration-update-title = Update skills?
# Maintenance explanation; ordered addition/update rows follow separately.
orchestration-update-message = Apply these bundled additions and updates.
