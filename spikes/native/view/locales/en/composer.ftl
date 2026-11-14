## The session's composer: what its controls say on a running session.
## Reference: Paseo composer/agent-controls (AgentControls): a change the
## agent does not take is an error toast; nothing is blocked during a turn.

# Error toast after a running session did not take a change of its model,
# effort, permissions or another option. $reason picks the sentence, by the
# machine's answer. The agent's own words are not shown here (they are the
# machine's, often English); a whole sentence each.
composer-option-refused = { $reason ->
    [busy] The agent can't make this change now.
    [not-offered] The agent no longer offers this choice. Pick another one.
    [read-only] This device can only view this machine, so it can't change a session.
    [gone] This session is no longer running on its machine.
    [unsupported] This agent can't change its options while the session runs.
    [unavailable] The change couldn't be sent now. Try again in a moment.
   *[other] The change didn't go through. Try again.
    }

## What an agent offers, kept and refreshed behind the list (owner word 28).
## Reference: Paseo's provider snapshot keeps the last list; it shows no
## words when a refresh fails. Ours says so once, quietly, in the list.

# A muted line at the top of the model, effort or permissions list when the
# list shown is the last one kept and asking the machine again failed. One
# sentence; the list still works.
composer-offers-stale = This list couldn't be refreshed, so it may be out of date.

# The `/` menu's one line when no command answers what is typed, or the
# agent has offered none (yet). Paseo's agentAutocomplete.noCommands.
composer-commands-empty = No commands found

# The new chat's `/` menu's one line when no session of the chosen agent on
# that machine has said its commands yet (the node reports commands only
# from a running session; the menu shows the last ones kept). One sentence.
composer-commands-unknown = This agent's commands show here once one of its sessions has run.
