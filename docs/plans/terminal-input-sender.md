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


Baseline9cd4f631 executed the two new regressions:0 passed/2 failed,
respectively oversized serial call and closed admission returning Queued.
This is semantic red after successful compilation, not a timeout or harness
failure. Its original raw log and assertions are preserved.

The correction retains one queue and each pending UTF8 blob with a typed
consumed prefix. Preparing the next call copies/scans only its64KiB prefix,
not the entire remaining backlog. New text never appends to a partially consumed
blob, preventing retained consumed prefixes from growing indefinitely. Quota
still counts the original bounded pending scalar values; only the taken prefix
releases quota. Adjacent untouched keys/resize behavior and full logical stream
remain unchanged. Resize now takes the existing validated GridSize and reports
closed/allocation refusal explicitly, rather than accepting bare dimensions.

Each take returns one opaque SendReceipt plus the same Outgoing wire payload.
Settlement checks allocation identity once; old same-owner, duplicate and
replaced-owner receipts are Stale without opening the current lane. Close keeps
its explicit discard-unsent meaning and returns the retired in-flight identity
for the actual caller's uncertainty handling, never cancellation or auto-retry.
No sent task or payload is owned by this pure queue. The old aggregate-million-
character test now asserts the same complete million-character stream plus its
final key across individually bounded calls, instead of requiring one invalid
oversized call. Existing resize/key order/failure/no-retry assertions remain.
New receipt and retained-prefix allocation regressions are added; actual changed-
source checks and receiving integration remain owed.


Exactb9b696ec executed the full changed View package:1075 unit,131 integration
and two doc tests,1208/0/0 total. Both old-code semantic failures now pass;
all original ordering/no-retry/full-stream assertions and new owned-receipt/
consumed-prefix tests ran. Strict found only a Copy count getter borrowed by
reference and an unbound pop_front result. The follow-up passes the Copy unit
by value and binds the completed item; product decisions, tests and expected
values are unchanged. A changed-source strict receipt remains owed; unchanged
runtime assertions are not rerun for those two qualifier-only corrections.

Important caller limit: a local timeout does not itself prove physical operation
settlement. The eventual guarded transport owner must retain busy/uncertainty
until its actual operation fence permits settlement or explicitly retires this
owner. The queue receipt grants neither cancellation nor remote write authority.
No native input/transport/IME, whole consumer gate or review is implied here.
