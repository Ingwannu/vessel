## The session's composer: what its controls say on a running session.
## Reference: Paseo composer/agent-controls (AgentControls): a change the
## agent does not take is an error toast; nothing is blocked during a turn.

# Error toast after a running session did not take a change of its model,
# effort, permissions or another option. $reason picks the sentence. $why is
# what the agent or the machine said, in its own words (often English); it
# follows the colon in the two sentences that show it.
composer-option-refused = { $reason ->
    [busy] The agent didn't make this change: { $why }
    [not-offered] The agent no longer offers this choice. Pick another one.
    [read-only] This device can only view this machine, so it can't change a session.
    [gone] This session is no longer running on its machine.
    [unsupported] This agent can't change its options while the session runs.
    [unavailable] The change couldn't be sent now. Try again in a moment.
   *[other] The change didn't reach the agent: { $why }
    }
