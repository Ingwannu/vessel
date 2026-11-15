# M12: the running session's composer controls (lead, 2026-11-14)

The coordinator's next rows for the native app, in the session's composer,
one slice each, Paseo's `composer/agent-controls/` and `composer/` as the
reference, value for value:

a. change the model, effort or permissions of a running session;
b. `/` commands (the agent's commands and skills, `facts.commands`);
c. attach images and files (`attachment.*`), shared with C's queued rows.

## Slice a: options on a running session

What exists:

- The node: `session.set_option` (protocol 9) answers every option after
  the change; the agent's refusal is the call's error. The session also
  records `options_changed` (always the whole list) after a change, after
  the handshake and whenever the agent changes one itself.
- The client: `NodeClient::set_option`.
- The view: the timeline's fold keeps the latest `options_changed` list
  (`facts().options`), so the session's current options are already known.
- The draft: the model pill (model, agent, effort), the effort card, the
  mode pill and their lists, all owned by the draft's state
  (`areas/draft/pickers.rs`, `effort_card.rs`, `features.rs`): they read
  the draft's probe and picks and write the draft's picks.

What Paseo does on a running agent (`composer/agent-controls/index.tsx`,
`AgentControls`): the same controls as the draft; choosing a model calls
`setAgentModel`, a thinking option `setAgentThinkingOption` (its provider
notice, when there is one, as a toast), a feature `setAgentFeature`; each
choice is also kept as the provider's preference; a failure is a toast with
the error's words. It does not block a change during a turn: the agent's
refusal is what the person sees.

Plan:

1. Link (done): `Act::SetOption(OptionChoice)`, kind `ActKind::SetOption`;
   success is `Acted::Done` (the new options arrive as the record); a
   refusal is the act's failure, which the conversation's bar already
   says, and a command the link does not take settles through
   `unsent.rs` like every act.
2. The controls take a target: the draft (its picks, as now) or a running
   session (its options from the fold; a choice sends `Act::SetOption`).
   The lists, the effort card and the density measure move into one
   controls state that the draft and the session each own, so the drawing
   is shared, not copied.
3. The session's composer draws the pills on its row beside send/stop, as
   the draft's composer does, only for a structured session whose agent
   reported options.
4. A choice is also kept as the agent's default for new drafts, as Paseo
   keeps the provider's preference.
5. Tests: a choice on a session sends the act with the option's id and
   value; a refusal leaves the pill on the session's reported value.
   Captures on the fake fleet (both themes, sizes, languages; a refusal).

Not in this slice: the refusal's words follow the coordinator's rule for
original text (the app's sentence, the agent's words in a tooltip) once
the one mapping for every area exists (its own slice, after c).
