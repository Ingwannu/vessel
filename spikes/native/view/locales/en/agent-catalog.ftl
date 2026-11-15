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
