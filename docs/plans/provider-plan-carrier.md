# Provider plan carrier — staged source only

[Decision Log]
- Purpose: W23's actor must retain a completed native proposal after successful
  TurnEnded without misclassifying every Plan-labelled permission as durable.
- Existing boundary: shared timeline PlanProposal/DecidePlan values are W23's;
  AgentInput and AgentOutput are the one driver command/output boundary.
- Alternatives: reuse ordinary permission IDs, parse labels, or submit a GUI
  prompt. None preserves actual provider proposal identity and acceptance.
- Choice: boxed AgentOutput::PlanProposal plus correlated AgentInput::decide_plan
  using the full typed key. Boxing keeps the bounded output queue compact.
- Effects: every current ACP/Claude/Codex edge explicitly refuses Unsupported;
  no proposal producer, native RPC, collaboration change or follow-up is enabled.
  A closed/lost reply preserves uncertainty, never reports native acceptance.

W22 owns the eventual Codex producer/transition, W23 owns the synthetic actor
permission, lifetime, persistence and acks. W23's NativeCommand carrier remains
unchanged. N3's future configuration owner remains independent. No new wire
number or dependency is introduced by this internal carrier. The probe ignores
plan outputs as non-catalogue metadata, like other non-ready timeline outputs.

This isolated commit is based on W23 staged6235655ed and is not consumer-ready.
It requires W23's actor arm and exhaustive consumers before a combined build.
Two carrier tests are drafted (exact correlation/refusal, closed/lost reply) but
not run. No local cargo/rustc, real agents or credentials were used.
