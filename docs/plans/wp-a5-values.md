# A5 portable attention values

Status: source-only dependency proposal; exact narrow check and V3 adoption
review pending. No node runtime, method row, SDK call, assignment, migration,
field installation or capability advertisement is included.

[Decision Log]
- Purpose: unblock shared native/CLI consumers on one canonical DTO without
  importing the full A5 node branch or inventing a local wire shape.
- Existing assets: A5's pending notifications/inbox_attention types, existing
  secret wrapper, typed identifiers and Inbox counts.
- Alternatives: whole node merge into the app; a second terminal/unread DTO;
  enabling pending methods before the actual producer/consumer checks.
- Choice: reuse exactly two pure proto modules and their module declarations.
- Why: no ingress/effect or runtime authority is added. The optional protocol
  constant stays None and these request values have no registered method in
  this producer. Existing Root/App registration remains separate owner work.
- Cost: consumers must wait for scoped review/integrator landing and their own
  union checks; these values alone cannot establish runtime parity.

SessionAttention.explicit_unread is a separate deliberate whole-session flag,
not historical log marks OR-ed into Inbox counts. Exact Seen/resolution does
not clear that flag; whole-session Read does in the staged runtime.

TerminalActivity::Untracked is explicit, distinct from serde(other) Unknown.
Missing/lost/future activity must never become neutral in workspace projection.
The actual terminal constructor/reducer and native adapter are separate work;
this producer does not change TerminalInfo or any existing reply/request field.
Known plain-shell construction may use Untracked only after canonical wiring.

Activity auth uses the existing secret wrapper and typed terminal identity. Its
Debug remains redacted; merely exporting a value adds no Principal, handler,
listener, environment injection or permission. W17's eventual IPC producer must
retain the real restricted handler/admission and bounded silent failure path.

No provisional protocol/schema number or generated wire artifact is committed.
Pure tests distinguish Untracked/future states, future claim refusal and revision
exhaustion. All Rust checks run on the build VM. No production credentials,
services, permissions, ownership or infrastructure are changed.
