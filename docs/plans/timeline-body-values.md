# Standalone canonical body values on package base 8

This portable prerequisite contains only the original eleven ToolCallDetail body
variants, validated supporting identities/units and ClientMessageId::random().
The body/unit files are byte-identical to96ed8b0b8; identity.rs is byte-identical
to0be6f4b10. This reuses the canonical vocabulary already consumed in A7's typed
fixture display kernel instead of creating a second native model.

The source is based on the official package base8 (91407173). Its patch may be
cherry-picked onto the node head82ae7c91 without importing app ancestry; app
consumers still wait for the integrator-adopted reviewed/checked producer.
There is no ToolCall/AgentEvent/Question/PermissionRequest attachment, status
change, method row, default handler, schema, provider/session/store/client runtime,
compatibility traversal or dependency change. TIMELINE_PARITY stays None.
Paths/URLs/commands/child identities are display evidence and never authority.

Paseo references remain ADR0084's protocol agent-types tool-call vocabulary and
tool-call presentation normalization; all eleven serde fixtures are the original
canonical tests. A7/W4 may compile only the borrowed body kernel and typed fixture
presentation against these values, without rich event/RPC activation.

Existing standalone0be strict compilation was predecessor evidence. This new
portable head requires its own exact proto all-target TS strict and pure body
fixtures, plus V3 source review and integrator disposition. It is not a checked
whole timeline or positive provider/runtime declaration.
