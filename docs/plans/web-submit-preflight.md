# Prepared submission transport

[Decision Log]
- Purpose: refuse unsupported identified submission before attachment work and
  prevent a changed book destination from receiving a prepared payload.
- Existing constraint: browser calls already use a shared `NodeClient`; its
  `check_call` applies the real negotiated support rule without sending a frame.
  A book key alone is mutable and cannot identify a prepared recipient.
- Alternatives: duplicating version predicates in JavaScript, introducing a node
  preflight RPC, or treating a local check as an admission ticket.
- Choice: local bridge `CheckSubmit` uses that existing SDK check; optional
  `ClientFrame::Call.expected_node` checks the actual selected client's NodeId
  immediately before forwarding. No dependency or node method is added.
- Reason: the same SDK admission rule remains authoritative and each actual
  upload/submit dispatch independently refuses recipient changes.
- Tradeoff: the check says nothing about session/provider admission and cannot
  promise a later call will succeed. A failed later call is never retried here.

`wire::ops` owns the local request/result values, `submission` owns the SDK
preflight and recipient comparison, and `calls` applies the comparison to the
actual client used to send. Browser `submission-calls` exposes the local check,
the constrained submit, and constrained begin/write/finish closures compatible
with the existing chunk uploader. Existing callers omit the additive constraint
and retain their prior wire shape. The constraint only refuses; it grants no
session, file, provider or node authority.

The composer owner must capture one ClientMessageId, original payload/policy and
authenticated NodeId before preparation; retain the exact final references and
payload through uncertainty; and never reupload, allocate another id or choose
legacy delivery after a possibly dispatched identified submission. This transport
commit does not activate that composer pipeline. While TIMELINE_PARITY is None,
the actual SDK reports Unsupported before any identified node RPC.

Prepared regressions cover local unassigned refusal, mismatched recipient refusal
before forwarding, correct-recipient ordinary dispatch, constrained upload and
submit frames, and no automatic retries. They use isolated nodes/fake harness
installation and a socket fixture, never a real provider. Changed-source strict,
generated TypeScript equality and actual tests remain owed; source preparation
alone is not runtime evidence.
