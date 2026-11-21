## Settings › Terminals: a machine's terminal profiles (ADR 0117), after
## Paseo's settings.host.terminalProfiles strings, "machine" where Paseo says
## "host". Short labels are buttons.

# The section's title (Paseo's sectionTitle).
terminal-profiles-title = Terminal profiles

# The one card while the machine is not connected (Paseo's unavailable).
terminal-profiles-unavailable = Connect to this machine to manage terminal profiles

# The one card while the machine's node predates terminal profiles; Paseo shows nothing then.
terminal-profiles-too-old = This machine's node is too old for terminal profiles. Update the node to manage them here.

# The card when the list has no rows (Paseo's emptyState).
terminal-profiles-empty = No profiles yet. Add one to launch terminals with a specific command.

# The section heading's plus button, its accessible name (Paseo's addProfileTitle). Keep short.
terminal-profiles-add = Add terminal profile

# A row's Move up button, its accessible name (Paseo's moveUp). Keep short.
terminal-profiles-move-up = Move up

# A row's Move down button, its accessible name (Paseo's moveDown). Keep short.
terminal-profiles-move-down = Move down

# A row's Edit button, its accessible name (Paseo's editProfile). Keep short.
terminal-profiles-edit = Edit profile

# A row's Remove button, its accessible name, and the remove confirmation's destructive button (Paseo's remove). Keep short.
terminal-profiles-remove = Remove

# The edit dialog's title when it adds a row (Paseo's addProfileTitle).
terminal-profiles-add-title = Add terminal profile

# The edit dialog's title when it changes a row (Paseo's editProfileTitle).
terminal-profiles-edit-title = Edit terminal profile

# The edit dialog's Name field label (Paseo's nameLabel). Keep short.
terminal-profiles-name-label = Name

# The edit dialog's Command field label (Paseo's commandLabel). Keep short.
terminal-profiles-command-label = Command

# The edit dialog's Arguments field label (Paseo's argsLabel). Keep short.
terminal-profiles-args-label = Arguments

# The Name field's placeholder (Paseo's namePlaceholder): a product name, not translated.
terminal-profiles-name-placeholder = Claude Code

# The Command field's placeholder (Paseo's commandPlaceholder): a program's name, not translated.
terminal-profiles-command-placeholder = claude

# The Arguments field's placeholder (Paseo's argsPlaceholder): a program's option, not translated.
terminal-profiles-args-placeholder = --dangerously-skip-permissions

# Under the Arguments field (Paseo's argsHint).
terminal-profiles-args-hint = Space-separated arguments passed to the command

# Under an empty Name field after Save (Paseo's nameRequired).
terminal-profiles-name-required = Name is required

# Under an empty Command field after Save (Paseo's commandRequired).
terminal-profiles-command-required = Command is required

# The edit dialog's Save button (Paseo's save). Keep short.
terminal-profiles-save = Save

# The Save button's words while the save waits for the machine (Paseo's saving). Keep short.
terminal-profiles-saving = Saving...

# The dialogs' Cancel button (Paseo's common.actions.cancel). Keep short.
terminal-profiles-cancel = Cancel

# The remove confirmation's title (Paseo's removeConfirmTitle).
terminal-profiles-remove-title = Remove profile?

# The remove confirmation's message (Paseo's removeConfirmMessage); $name is the profile's name.
terminal-profiles-remove-message = Remove "{ $name }"?

# Over the list when a move or removal was not kept (Paseo's common.errors.unableToSave).
terminal-profiles-unable-to-save = Unable to save

# Why a change was not kept: this device may read the machine's settings but not change them.
terminal-profiles-forbidden = This device may not change this machine's settings.

# Why a change was not kept: the machine refused it or did not answer.
terminal-profiles-refused = The machine did not take the change. Try again.

# Why a draft cannot be kept: a field, or the command with its arguments, is longer than a profile may be.
terminal-profiles-too-long = This profile is too long to keep.

# Why a draft cannot be kept: a field holds a character a profile may not have (a control character).
terminal-profiles-character = This profile holds a character that is not allowed.

# Why a draft cannot be kept: the machine already keeps the most profiles it may.
terminal-profiles-too-many = This machine already keeps the most profiles it may.
