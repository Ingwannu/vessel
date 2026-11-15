## Composer shell, fullscreen and command menus. Reference: Paseo composer/input
## and agentAutocomplete. Keep button labels short; error sentences are complete.

# Expand control on a compact multi-line composer.
composer-shell-fullscreen = Fullscreen
# Collapse control in the fullscreen editor.
composer-shell-exit-fullscreen = Exit fullscreen
# Accessible name of the ordinary chat editor.
composer-shell-chat-input = Message agent...
# Accessible name when the same editor becomes terminal input.
composer-shell-terminal-input = Terminal prompt
# Built-in /exit description in autocomplete.
composer-shell-client-exit = Archive the current agent
# Built-in /clear description in autocomplete.
composer-shell-client-clear = Archive this agent and start a fresh draft
# Draft menu before a workspace has been chosen.
composer-shell-choose-project = Choose a project to see commands
# Draft menu before a provider/model has been chosen.
composer-shell-choose-model = Select a model to see commands
# Pending provider command discovery.
composer-shell-commands-loading = Loading commands...
# A failed discovery, before retry. No provider stderr is embedded here.
composer-shell-commands-failed = Failed to load commands
# Composer whose owning create flow preserves and locks its draft while submitting.
composer-shell-submit-locked = Sending...
# Editor retained in a view-only context.
composer-shell-read-only = This conversation is read-only.
# File/image link lookup did not find a target in the authorized workspace.
composer-shell-link-not-found = No matching file was found in this workspace.
# A file target belongs outside every root the Files area may open.
composer-shell-link-outside = This file is outside the available workspace folders.
# A provider error retains a typed reason for retranslation. $reason chooses
# a complete sentence and never contains a node error or an untrusted path.
composer-shell-command-problem = { $reason ->
    [unsupported] This provider does not expose a command catalog.
    [version] Update Vessel on this device and the machine to load commands.
    [access] This device is not allowed to load commands.
    [disconnected] Connect to this machine to see commands.
    [invalid] The draft settings could not be used to load commands.
   *[unavailable] Commands could not be loaded. Try again.
    }
