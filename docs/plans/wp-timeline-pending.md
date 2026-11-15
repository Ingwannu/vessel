# Identified optimistic message model

This pure source prerequisite uses W23's exact standalone
`ClientMessageId`/`SubmitPromptReply` values (`96ed8b0b` + allocator `0be6f4b1`).
It registers no node call or event and does not activate a second Tail pipeline.

[Decision Log]
- Purpose: equal texts must remain distinct submissions, and uncertainty must
  preserve the original pending row without automatic replay.
- Constraint: the existing Tail is text matched; W6 owns the single preparation
  and transport ingress, while W23 owns durable receipts/canonical echoes.
- Alternatives: duplicate local acceptance enums, text matching, or an independent
  retry/delivery manager in the conversation area.
- Choice: one pure `PendingMessages` ledger with a named 32-row admission bound,
  exact caller identities and direct canonical receipt values. W6's shared ingress
  admits before preparing bytes or node I/O; its payload/retry owner stays W6.
- Why: receipt correlation and presentation are testable independently of transport.
  A response bound to another captured request cannot mutate either pending row.
- Effects: Accepted, including Queued, waits for canonical observation; Unknown
  stays pending/reconciling and never authorizes resend. A durable acceptance wins
  over later conflicting/uncertain observations. Local refusal/failure removes
  only the exact identity. Late replies cannot recreate removed rows. Diagnostics
  expose sizes/identities rather than pending text.

The caller checks already observed canonical identities before a new admission;
the ledger is not a second durable replay-key store. New admission after an exact
canonical echo therefore cannot create an orphan row for an already accepted ID.
Attachment-only messages may have empty text. Immutable file refs and bytes remain
with W6; no preview attachment metadata is invented in this model.

Planned atomic app seam:

- `conversation_sending(id: ClientMessageId, text: &str, cx) -> Result<(), PendingFull>`;
- `conversation_send_failed(&ClientMessageId)` for local preparation refusal;
- `conversation_submission(&SessionId, &ClientMessageId, &SubmitPromptReply, cx)`.

The old unkeyed Tail and shared submit ingress are replaced together when W6's
consumer is staged. Until then this prerequisite is model source, not a second
production optimistic manager or a delivery-retention claim. Session/node/epoch
binding remains with the common resident/event routing contract. Six tests cover
duplicate text, exact echo/late replies, unknown outcomes, queued handoff timing,
capacity/retry admission, request mismatch and acceptance precedence. Exact-head
execution evidence is pending when this source is first committed.
