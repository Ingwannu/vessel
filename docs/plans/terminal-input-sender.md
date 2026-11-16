# Opening-owned terminal input sender (staged T2)

[Decision Log]
- Purpose: preserve ordered keys/paste/resize while asynchronous writes settle,
  without exceeding the existing node send-call bound or accepting stale replies.
- Existing assets: view::terminal::TerminalSender is the single pure queue. It
  has no product caller yet. Reuse it; no actor, writer or second queue is added.
- Constraints: SendTerminal::MAX_BYTES is64KiB. The current queue joins up to
  one million characters and take returns all joined text in one call. Closed
  admission reports Queued despite retaining nothing. The legacy answered has
  no outstanding-operation identity. These must not become a live T2 contract.
- Alternatives: let the node reject oversized coalescing, drop/refeed a failed
  prefix, create per-key concurrent sends, or infer which reply is current.
- Choice: bounded ordered UTF8 chunks using the protocol's existing byte ceiling;
  explicit closed/full/allocation refusal, preserving caller input; opaque
  per-dispatch ownership for single-use settlement, independent of route authority.
- Why: the existing serial queue already preserves the logical stream. Chunking
  an encoded paste never inserts another resize/key inside it; late or duplicate
  replies cannot release another pending write. Failed sends are never retried.
- Effects: no new RPC, parser, transport, focus/IME or actual input activation.
  Original owner/lease admission and explicit uncertainty for sent bytes remain
  the actual caller's responsibility. No receipt proves physical PTY consumption.

## Reference and stated differences

The old module header's data/terminal-sender.ts reference is absent in the current
reference checkout; do not cite it as observed code. Actual Paseo
terminal/runtime/terminal-stream-controller.ts45-106 fences retained subscriptions,
and client/daemon-client.ts6568-6570 sends terminal input through sendSessionMessage;
2141-2157 refuses disconnected transport. The websocket transport.ts42-79 drains
above1MiB buffered bytes on supported platforms; it does not establish an RPC
acknowledgement. Vessel's RPC transport needs one serial call in flight to avoid
concurrent handler reorder. This is an explicit transport adaptation, not copied
reference implementation. The existing protocol's64KiB call ceiling is reused,
never independently redefined. Refusing a closed owner is an explicit typed
result rather than the legacy silent successful no-op.

Next: real baseline regressions for oversized coalescing and closed admission,
then same-queue correction and owned settlement tests. No source/runtime or
native whole result from other terminal work is borrowed for this leaf.
