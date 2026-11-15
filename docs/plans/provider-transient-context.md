# Transient provider system context

Status: proposed

## Reference and Decision Log

Paseo `packages/server/src/server/session/voice/voice-session.ts:491-555`
reloads an existing agent with voice system instructions and later restores its
base configuration. The current AgentSetup carries no typed system instruction.

- Purpose and intent: give lifecycle reload a transient, bounded instruction
  value without putting it in SessionSpec, a user message, argv or environment.
- Existing implementation and constraints: AgentSetup is launch-only; existing
  drivers have no tested override-and-restore contract. A field alone must not
  silently grant support. Voice owns the actual instruction text and its bound.
- Alternatives considered: a normal user prompt; arbitrary environment/option
  substitution; an unbounded string; advertising Codex support solely because
  its schema has developerInstructions on start/resume.
- Choice: a non-serializable, redacted SystemPromptOverride wrapping the existing
  bounded PromptText type, an optional AgentSetup field, and an explicit harness
  transport capability. Unsupported requests fail before process spawn.
- Why: callers can compile against one typed seam while preserving the honest
  absence of a supported transport. Schema field existence does not prove that
  removing a transient override restores the original native configuration.
- Advantages, costs and effects: no new dependency or wire field; current session
  behaviour is unchanged when the field is absent. Every transport initially
  refuses the override. Actual voice enablement remains unavailable until a
  transport proves both replacement and restoration with fake protocol tests.

The outer launch value reuses AutomationText::PROMPT_MAX and its parser. Voice
may impose a stricter independent bound. Prompt content is absent from Debug,
errors, persistent session state, user logs and generic command/environment
values. No live agent, account file or network is needed to test the refusal.

## Resume point

Contract recorded before implementation. Next: bounded value, AgentSetup field,
transport capability and pre-spawn refusal; complete existing literals with
None only. Then narrow checks, one package gate and review. Lifecycle owns reload
orchestration; voice owns text and enable/restore state.
