## Rewind from a person's message. Reference: Paseo's rewind menu and
## use-rewind-agent-mutation; Vessel rewinds the conversation, not files.

# The person's rewind mark tooltip. Keep short.
rewind-tooltip = Rewind to this message
# Noninteractive warning at the top of its menu.
rewind-warning = This action cannot be undone
# The one mode supported by the node. Keep short.
rewind-conversation = Rewind conversation
# The person's message copy action. Keep short.
rewind-copy-message = Copy message
# Frame notice after refusal. $kind distinguishes absent/present diagnostic;
# $message is the node's existing diagnostic. Its typed replacement is separate.
rewind-failed = { $kind ->
    [detail] Failed to rewind agent: { $message }
   *[plain] Failed to rewind agent
    }
