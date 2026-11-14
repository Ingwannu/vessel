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
