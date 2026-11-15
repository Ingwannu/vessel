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

## Launch contract extraction

The handshake file is at its shrinking size budget. AgentSetup is moved intact
to setup.rs and re-exported from its existing path before the field is added.
This refactor changes no input, behaviour or test expectation; protocol logic
stays in handshake.rs. The feature commit then adds the optional field and
completes existing literals with None.

## Implemented contract

AgentSetup now has system_prompt: Option<SystemPromptOverride> in setup.rs,
re-exported from its previous path. The value has bounded validation and
redacted Debug, with no persistence codec. Driver capability delegates to the
harness transport policy. Every current driver reports Unsupported and the
spawn boundary returns LaunchRefusal::SystemPromptUnsupported before looking
for an executable, including drivers not built in this version. Session error
mapping preserves the Unsupported wire classification. No driver request or
user prompt is sent by this change.

Existing AgentSetup literals are completed with None only in the session's
again/runtime launch and probe constructors, driver test setup, ACP/Codex
integration test setup and Claude launch/session test setup. No existing test
assertion, provider choice or option-completion arm is changed. New tests pin
the shared bound, controls, redacted setup/error Debug and pre-spawn refusal
for every current transport. Checks and execution are pending.

None selects the original native instruction source for this launch; a future
Supported transport must prove that resumed sessions actually restore it. All
current transports are Unsupported, so no override can be installed by this
change. The initial narrow compile found an import used only by a doc link;
the link now uses its fully qualified public path.
