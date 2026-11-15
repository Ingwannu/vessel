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

Implementation, narrow checks and the corrected-base whole gate are complete.
Next: V3 source/security review and the integrator's combined proof after the
required package-base merge. Lifecycle owns reload orchestration;
voice owns text and enable/restore state.

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
for every current transport. Actual override transport remains unavailable.

Some replaces the complete Vessel supplemental instruction layer for this
launch; it does not replace the vendor-native base or append to an earlier
transient layer. None uses the unmodified captured launch context. A future
Supported transport must prove that resumed sessions actually restore it. All
current transports are Unsupported, so no override can be installed by this
change. The initial narrow compile found an import used only by a doc link;
the link now uses its fully qualified public path.

## Validation checkpoint

On 2026-11-15, commit 087583af passed Clippy for vessel-agent, vessel-harness
and vessel-session with all targets and warnings denied (remote run
wp-providers-narrow-20261115T065633Z-087583af). The scoped system_prompt unit
selection passed all four tests with zero failures (run
wp-providers-narrow-20261115T071400Z-087583af). The test gate also built the
commit's web bundle and fake executables. These checks establish bounded values,
redaction, pre-spawn refusal and error classification; they do not establish
voice enablement or a supported native override/restore path. The whole root
gate and review remain pending.

The one whole gate ran on 0eb6d895 (source identical to the narrowly checked
087583af): wp-providers-20261115T072649Z-0eb6d895. Rust reported 2270 passed,
1 failed, 12 ignored; web unit tests 442 passed; browser tests 64 passed,
1 failed; forge fixture 4 passed. Formatting, source size/boundaries, all Clippy
variants, generated TypeScript and builds passed. The two failures are existing
paths unchanged by this package: vessel-mcp's archive/resume test received a
Conflict while its ended log was still writing, and web security.spec.ts:98
observed missing sessionStorage at line 118. The single exact-SHA archive replay
passed (wp-providers-narrow-20261115T075238Z-0eb6d895). The single exact-SHA
security replay failed the same assertion
(wp-providers-narrow-20261115T075839Z-0eb6d895). Original red evidence is retained
under /tmp/work/wp-providers/prompt-whole, with both replay folders beside it.
This is not a green whole gate. The coordinator/integrator own the baseline
security correction and combined proof; this package has no web/bridge changes.
Source/security review is requested with that gate limitation stated explicitly.

The coordinator identified the security assertion as a known base race and
authorized exact c5da5305 plus one whole rerun. That merge is b7e8c329. Its whole
run wp-providers-20261115T081109Z-b7e8c329 passed: Rust 2271/0/12, web unit
442/0, browser 66/0, forge 4/0, with every other step zero. Logs are in
/tmp/work/wp-providers/prompt-whole-basefix. No real agent or credential was used.

After that run finished, the required base 6c782cb6 merged cleanly as 5b036eba.
The agent, harness and session trees are byte-identical to the green b7e8c329;
the base's independent scripts/project changes and native-app files are not
this feature. V3 must audit that merge; the new combined head is not claimed
to have run this package's whole gate. The integrator's combined proof remains
owed before consumer-ready handoff.

The feature commit's short message omitted the A8 literal inventory; history
has not been rewritten. Mechanical None additions are audited here: test setup
constructors in agent/tests/{agent,codex,conversation,options,questions}.rs;
agent/src/drivers.rs::tests::setup, claude/launch/tests.rs and
claude/session/tests.rs::setup, codex/session.rs's unit-test launch setup; session/again.rs,
session/runtime/agent.rs (probe and start), and session/runtime/launch.rs. No
existing test assertion changed. The field-bearing commit includes those
completions, and source review can verify the one-line-only head-test diffs.

## Layer semantics and current-head evidence

Paseo voice-session.ts:491-555 captures the stripped configured instruction
base. Disable may supply Some(saved base + explicit disabled policy), so restore
must not be equated with None. This seam does not provide an effective-base
getter. Lifecycle must distinguish known absent Vessel context from unavailable
capture before voice composes a replacement; neither native files nor built-in
vendor instructions may be guessed. Capability must be checked before stopping
the existing agent. Current/custom MCP preservation and authorized VoiceTools
context are separate prerequisites, not supplied by this field.

The required node head ba45a6e3 merged as b05bf2e7. Both architecture appendices
were kept, head first; no owned source conflict or test assertion changed. Strict
all-target Clippy for agent/harness/session passed on b05bf2e7 in run
wp-providers-narrow-20261115T085155Z-b05bf2e7 (cargo rc0, 22 seconds). The whole
gate remains the exact b7e8c329 evidence above; no combined whole-gate claim is
made for the later required merges. The following clarification changes only
comments and this contract document.
