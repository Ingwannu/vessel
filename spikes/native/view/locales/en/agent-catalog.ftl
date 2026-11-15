## Settings › Providers: registry offers resolved by the selected machine.

# Section title above the registry tiles. No values; keep short.
agent-catalog-title = Add provider

# Search field for visible registry names and ids. No values.
agent-catalog-search = Search providers

# Record a pinned command on the machine; this does not run an installer.
agent-catalog-add = Add

# Short pending Add control label. No values.
agent-catalog-adding = Adding…

# Whole sentence explaining the record-only Add action; references its label.
agent-catalog-add-help = Select “{ agent-catalog-add }” to record this provider. It does not install or start the provider.

# Registry source id is already present, even under another name/version.
agent-catalog-recorded = This provider is already added on this machine.

# Add remains allowed. A package runner must be installed before chat starts.
agent-catalog-runner-missing = You can add this provider, but its package runner must be installed on the machine before you can chat.

# The node resolves this catalog entry to one of its built-in agents.
agent-catalog-built-in = This built-in provider is managed separately.

# The node only reports an archive; it cannot add this entry as a command.
agent-catalog-download = Install this provider on the machine, then add it by its command.

# Nothing in the node's catalog can run on its platform. No values.
agent-catalog-unavailable = This provider is not available for this machine.

# Successful registry with no entries; different from a failed fetch.
agent-catalog-empty = No registry providers were listed by this machine.

# A nonempty search yields no matching registry title/id. No values.
agent-catalog-no-matches = No providers match your search.

# The node has no fetched copy yet; never claims successful empty results.
agent-catalog-not-read = The provider registry has not been read yet.

# Initial fetch, beside the existing small loader. No values.
agent-catalog-reading = Reading the provider registry…

# Registry-only retry/refresh control; keep short.
agent-catalog-refresh = Refresh

# A reused book route was refused before sending a registry read.
agent-catalog-changed = The machine behind this name changed. Select “{ agent-catalog-refresh }” to read its provider list before trying again.

# The existing Agents hub refused a new worker.
agent-catalog-busy = Settings is busy. Try again when the current operation finishes.

# Reconnect before retrying the registry read.
agent-catalog-offline = The machine is not connected. Connect it, then select “{ agent-catalog-refresh }”.

# A definite older peer has no registry method; not empty data.
agent-catalog-older = Update Vessel on this machine to read the provider registry.

# The node refused registry read or explicit refresh permission.
agent-catalog-permission = This device does not have permission for this registry action. Ask the machine’s administrator to grant access.

# Only the client wait ended; does not claim cancellation of node fetch.
agent-catalog-timeout = The machine did not return the provider registry in time. Select “{ agent-catalog-refresh }” to try again.

# Unmodified machine prose is diagnostic detail, not this sentence.
agent-catalog-read-error = Could not read the provider registry. Select “{ agent-catalog-refresh }” to try again.

# Browser transport is absent, not an empty catalog.
agent-catalog-no-transport = This browser cannot connect to a machine yet.

## Record-only provider form and its local/node refusal instructions.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-form-title = Add provider

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-command-add = Add by command

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-name-label = Name

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-name-placeholder = my-provider

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-name-help = Use a new name with letters, numbers, dots, underscores or hyphens.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-program-label = Program

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-program-placeholder = Program name or absolute path

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-arguments-label = Arguments

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-arguments-placeholder = One literal argument per line

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-arguments-help = Each line is one argument. Quotes and spaces are kept as written; no shell is used.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-form-help = This records a provider definition on the machine. It does not install, start or sign in to the provider.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-close = Cancel

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-pending-help = Closing this form does not undo a request already sent to the machine.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-invalid-name = Choose another provider name. Use 1–64 letters, numbers, dots, underscores or hyphens; built-in names are reserved.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-name-taken = A provider already uses this name on the machine. Choose another name.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-invalid-program = Enter a valid program name or absolute path. Put its arguments in the separate field.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-form-changed = The machine or registry entry changed. Close this form and open it again from the current list.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-form-unavailable = Read the machine’s provider list before adding a provider. Close this form and select “{ agent-catalog-refresh }”.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-form-exhausted = Settings cannot reserve another request. Restart Vessel before trying again.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-add-changed = The machine behind this name changed. Close this form and select “{ agent-catalog-refresh }” before trying again.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-add-unsupported = Update Vessel on this machine before adding a provider.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-add-permission = This device cannot add providers on the machine. Ask the machine’s administrator for access.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-add-conflict = The provider name or registry version conflicts with the machine’s current definitions. Read the list again and choose another name if needed.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-add-rejected = The machine could not add this provider. Check the name and command, then try again.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-add-unknown = Could not confirm whether the provider was added. Close this form and select “{ agent-catalog-refresh }” before trying again.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-add-offline = The machine is not connected. Connect it before adding a provider.

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-added-label = Added

# Whole localized form label or instruction; no machine prose is interpolated.
agent-catalog-type-label = ACP

# Whole localized form label or instruction; no machine prose is interpolated.

# Whole metadata label; values are separate code fields.
agent-catalog-website = Website

# Whole metadata label; values are separate code fields.
agent-catalog-repository = Source

# Whole metadata label; values are separate code fields.
agent-catalog-download-link = Download

# Whole metadata label; values are separate code fields.
agent-catalog-launch-preview = Program and arguments

# Whole metadata label; values are separate code fields.
agent-catalog-registry-environment = Environment supplied by the registry

# Added legacy definition action in its overflow menu. No values.
agent-catalog-remove = Remove provider

# Confirmation heading; name is the exact returned display name.
agent-catalog-remove-title = Remove { $name }?

# Exact consequence for the legacy definition file, not a package uninstall.
agent-catalog-remove-help = This removes the added agent definition from this machine. It cannot be undone.

# Cancel-first destructive confirmation footer.
agent-catalog-cancel = Cancel

# Confirm one retained removal; keep short.
agent-catalog-remove-confirm = Remove

# Local confirmation refusal; no administrative request was sent.
agent-catalog-remove-refresh = Select “{ agent-catalog-refresh }” to read the current agent definitions before removing an agent.

# Local confirmation refusal; no administrative request was sent.
agent-catalog-remove-unavailable = Only added agent definitions reported by this machine can be removed.

# Local confirmation refusal; no administrative request was sent.
agent-catalog-remove-changed = The machine or agent definition changed. Open the removal confirmation again from the current list.

# Local method is unassigned; no frame was sent to the machine.
agent-catalog-add-unassigned = This version of Vessel cannot add providers yet. Update Vessel on this device before trying again.
