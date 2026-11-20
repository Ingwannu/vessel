## A shell terminal's tab in a workspace pane (TRACKER 481/482), after Paseo's
## workspace.terminal and workspace.tabs strings. Short labels are buttons.

# Tab label of a terminal without a title (Paseo's workspace.tabs.fallback.terminal). Keep short.
terminal-tabs-fallback = Terminal

# The pane's centre when the terminal's machine is not connected (Paseo's workspace.terminal.hostDisconnected).
terminal-tabs-host-disconnected = Host is not connected

# The red line under the terminal when its program ended (Paseo's terminal stream "Terminal exited").
terminal-tabs-exited = Terminal exited

# The red line under the terminal when its machine no longer has it (Paseo's "Terminal not found").
terminal-tabs-not-found = Terminal not found

# The red line under the terminal when following it failed (Paseo's workspace.terminal.unableToSubscribe).
terminal-tabs-unable-to-subscribe = Unable to subscribe to terminal

# Toast when a new terminal could not be opened; the machine's own words are not shown.
terminal-tabs-create-failed = Failed to create terminal

# Close confirmation's title (Paseo's workspace.tabs.confirmations.closeTerminalTitle).
terminal-tabs-close-title = Close terminal?

# Close confirmation's message (Paseo's workspace.tabs.confirmations.closeTerminalMessage).
terminal-tabs-close-message = Any running process in this terminal will be stopped immediately.

# Close confirmation's destructive button (Paseo's workspace.tabs.confirmations.close). Keep short.
terminal-tabs-close = Close

# Close confirmation's dismiss button (Paseo's workspace.tabs.confirmations.cancel). Keep short.
terminal-tabs-cancel = Cancel

# Find bar's field placeholder and its accessible name (Paseo's paneFind.placeholder).
terminal-tabs-find-placeholder = Find in pane

# Find bar's previous match button (Paseo's paneFind.previous). Keep short.
terminal-tabs-find-previous = Previous match

# Find bar's next match button (Paseo's paneFind.next). Keep short.
terminal-tabs-find-next = Next match

# Find bar's close button (Paseo's paneFind.close). Keep short.
terminal-tabs-find-close = Close Find

# Find bar's counter when nothing matches (Paseo's paneFind.noMatches).
terminal-tabs-find-none = No matches

# Find bar's counter with matches and none current (Paseo's paneFind.matches); total may end in "+".
terminal-tabs-find-total = { $total } matches

# Find bar's counter: the current match of all (Paseo's paneFind.position); total may end in "+".
terminal-tabs-find-position = { $current } of { $total }

# Accessible name of the find bar's counter (Paseo's paneFind.results).
terminal-tabs-find-matches = Find matches

# Accessible name of the terminal's keyboard input (Paseo's native terminal input label).
terminal-tabs-input = Terminal input

# Settings > Terminal: the scrollback row's label (Paseo's settings.terminal.scrollback.label).
terminal-tabs-scrollback = Terminal scrollback

# Settings > Terminal: the scrollback row's description (Paseo's settings.terminal.scrollback.description), with a
# word on memory (P3 app review note 3: a million lines is a lot of memory for each busy terminal).
terminal-tabs-scrollback-hint = Lines kept in the built-in terminal buffer. Large values use more memory for each terminal.

# Settings > Terminal: the scrollback field's accessible name (Paseo's settings.terminal.scrollback.accessibilityLabel).
terminal-tabs-scrollback-label = Terminal scrollback lines

# A paste refused because more than 32 MiB already waits to go to the terminal (no Paseo source: Paseo has no bound;
# P3 app review note 1). Said once until a paste is taken again.
terminal-tabs-paste-too-large = The paste was not sent: more than 32 MiB is already waiting to go to this terminal.
