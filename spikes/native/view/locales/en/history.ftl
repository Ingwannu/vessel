## Import sheet. Reference: Paseo i18n importSession (resources/en.ts
## 401-438, ko.ts 405-442) and the host chooser (hosts/host-chooser.tsx).
## $host is a machine's name; $provider is an agent's name, such as Codex.

# The sheet's title, and the new-session page's button that opens it.
history-title = Import session
# The sheet's first page when no machine is in context: choose one.
history-choose-host = Import from host
# Placeholder of the first page's search over machines.
history-search-hosts = Search hosts...
# The first page when no machine matches its search.
history-no-hosts = No matching hosts
# Shown when no machine is connected.
history-connect-host = Connect to a host to import sessions
# Placeholder of the sheet's search over the listed sessions.
history-search = Search sessions...
# The line under the title when the list covers the whole machine.
history-scope-host = Sessions on { $host }
# The line under the title when the list covers the current folder only.
history-scope-workspace = This workspace
# The small button beside "This workspace": list the whole machine. Keep short.
history-show-all = Show all
# The provider filter's choice that shows every agent. Keep short.
history-filter-all = All
# Title of the provider filter's list on a phone.
history-filter-title = Filter by provider
# Accessible name of the provider filter; $provider is the choice in effect.
history-filter-label = Filter: { $provider }
# The header's refresh button: its tooltip and accessible name.
history-refresh = Refresh sessions
# Shown while the first sessions are read.
history-loading = Loading recent sessions...
# The machine has no agent that can list its sessions.
history-no-providers = No importable providers are enabled.
# One agent's sessions could not be read; a Retry button follows.
history-failed-provider = Could not load { $provider } sessions
# The button that reads one failed agent again. Keep short.
history-retry = Retry
# The pressed session could not be imported.
history-failed-import = Could not import selected session.
# The button after the list that shows more sessions. Keep short.
history-more = Load more
# A listed session without a title.
history-untitled = Untitled session
# A listed session whose prompts the machine did not read (the line under its title).
history-no-prompt = No prompt preview
# The pressed row while its session is imported. Keep short.
history-importing = Importing...
# Empty list: nothing on the machine to import.
history-empty-recent = No recent sessions to import.
# Empty list: the search matches nothing.
history-empty-matches = No sessions match your search.
# Empty list: every listed session is already in the app.
history-empty-imported = All recent sessions are already imported.
# Empty list: the filtered agent has no sessions.
history-empty-provider = No { $provider } sessions found.

# Import failure: the machine is not connected.
history-offline = Connect to this machine to read its history.
# Import failure: the request could not be sent.
history-unsent = The request was not sent. Try again.
# Import failure: the machine is too old to list sessions (Paseo updateHost).
history-unsupported = Update the host to import sessions.
# Import failure: no answer in time.
history-timeout = Reading history timed out. Refresh to try again.
# Import failure: the import may have happened.
history-uncertain = The import may have completed. Refresh history before trying again.
# Import failure: the machine refused.
history-refused = The machine refused the request. Refresh to try again.
# Import failure: not allowed on this connection.
history-forbidden = This connection does not have permission to import conversations.
# Import failure: already open elsewhere.
history-conflict = This conversation is already open elsewhere. Refresh history to find it.
# Import failure: the machine answered nonsense.
history-invalid-reply = The machine returned an invalid reply. Refresh before trying again.
# Import failure: the app ran out of request numbers.
history-exhausted = Request identities are exhausted. Reopen the app.
# Import guard when a saved machine name points to a different node.
history-identity-changed = The machine identity changed. Refresh history before importing.
# A finite retained-history or batch admission limit.
history-capacity = Choose a narrower folder or import fewer conversations at once.
