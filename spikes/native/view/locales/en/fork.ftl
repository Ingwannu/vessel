## Forking a conversation. Reference: Paseo's assistant-fork-menu and
## use-fork-agent. Vessel creates a session beside the source, not a draft tab.

# Turn footer tooltip. Keep short.
fork-tooltip = Fork chat from here
# Our node's one fork target. Keep short.
fork-in-new-session = Fork in a new session
# A refused fork; diagnostic detail is retained only during the migration.
fork-failed =
    { $kind ->
        [plain] Failed to fork chat
       *[detail] Failed to fork chat: { $message }
    }
# Heading above a fork that never held a conversation.
fork-empty-heading = The fork did not start
# The agent ignored the chosen point; nothing can be continued here.
fork-empty-not-at-point = The agent did not fork at the chosen point. Nothing here can go on; remove it and fork again.
# The agent cannot fork; nothing can be continued here.
fork-empty-cannot-fork = This agent cannot fork a conversation. Nothing here can go on.
# Remove an empty fork, without deleting its origin. Keep short.
fork-remove = Remove
# An origin read failed. Each selector owns the whole recovery sentence.
fork-origin-unread =
    { $kind ->
        [removed] The conversation it was forked from was removed.
        [plain] The conversation it was forked from could not be read. Open the session again to try again.
       *[detail] The conversation it was forked from could not be read: { $message }. Open the session again to try again.
    }
# First line of a fork; title is the origin's session title.
fork-from-title = Forked from “{ $title }”
# First line when the source's title is unknown or blank.
fork-from-other = Forked from another session
