## Forking a conversation. Reference: Paseo's assistant-fork-menu and
## use-fork-agent. Vessel creates a session beside the source, not a draft tab.

# Turn footer tooltip. Keep short.
fork-tooltip = Fork chat from here
# Our node's one fork target. Keep short.
fork-in-new-session = Fork in a new session
# Heading above a fork that never held a conversation.
fork-empty-heading = The fork did not start
# The agent ignored the chosen point; nothing can be continued here.
fork-empty-not-at-point = The agent did not fork at the chosen point. Nothing here can go on; remove it and fork again.
# The agent cannot fork; nothing can be continued here.
fork-empty-cannot-fork = This agent cannot fork a conversation. Nothing here can go on.
# Remove an empty fork, without deleting its origin. Keep short.
fork-remove = Remove
# First line of a fork; title is the origin's session title.
fork-from-title = Forked from “{ $title }”
# First line when the source's title is unknown or blank.
fork-from-other = Forked from another session

# Fork refusal recovery: Point. Diagnostic detail stays in logs.
fork-problem-point = The selected point cannot be used to fork this conversation. Choose another turn and try again.

# Fork refusal recovery: Access. Diagnostic detail stays in logs.
fork-problem-access = This device is not allowed to fork this conversation.

# Fork refusal recovery: Version. Diagnostic detail stays in logs.
fork-problem-version = Update Vessel on this device and the machine before forking.

# Fork refusal recovery: Missing. Diagnostic detail stays in logs.
fork-problem-missing = This conversation is no longer available.

# Fork refusal recovery: not ready. Diagnostic detail stays in logs.
fork-problem-not-ready = This conversation cannot be forked right now. Open it again before retrying.

# Fork refusal recovery: Unsupported. Diagnostic detail stays in logs.
fork-problem-unsupported = This agent cannot fork conversations.

# Fork refusal recovery: missing agent. Diagnostic detail stays in logs.
fork-problem-agent-missing = This conversation's agent is not installed on the machine.

# Fork refusal recovery: Start. Diagnostic detail stays in logs.
fork-problem-start = The agent could not be started to fork this conversation. Try again.

# Fork refusal recovery: Unavailable. Diagnostic detail stays in logs.
fork-problem-unavailable = The machine cannot fork this conversation right now. Try again later.

# Fork refusal recovery: Failed. Diagnostic detail stays in logs.
fork-failed = Could not confirm the fork. Check the session list before trying again.

# Origin read recovery: Removed. Retrying never clears the fork.
fork-origin-removed = The conversation it was forked from was removed.

# Origin read recovery: Access. Retrying never clears the fork.
fork-origin-access = This device is not allowed to read the conversation it was forked from.

# Origin read recovery: Version. Retrying never clears the fork.
fork-origin-version = Update Vessel on this device and the machine to read the conversation it was forked from.

# Origin read recovery: Unavailable. Retrying never clears the fork.
fork-origin-unavailable = The machine is unavailable. Open the session again to retry reading the conversation it was forked from.

# Origin read recovery: Failed. Retrying never clears the fork.
fork-origin-failed = The conversation it was forked from could not be read. Open the session again to try again.

# Unknown empty-fork failure. The original diagnostic stays in session state.
fork-empty-other = The agent could not start the fork. Nothing here can go on; remove it and fork again.
