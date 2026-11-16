# Directory session marks retain the rendered connection

[Decision Log]
- Purpose: a retained sidebar/history row or confirmation must not send an old
  session UUID to a replacement alias or same-node connection admission.
- Existing assets: Machine::connection_context, the sole G4 SelectedConnection
  and weak DeliveryGuard, existing session.mark, and the directory projection.
- Alternatives: re-resolve an alias at click/confirm, add an identity cache, or
  admit an unrelated resident merely to mark a directory row.
- Choice: capture the original machine/card context once, then use the existing
  selected call/send guard and a page-owned pending UI request. Standard types
  and the existing Tokio bounded channels/semaphore suffice; no dependency is
  added.
- Why: capture is intent only. The actor's actual lease remains authority, and
  the same weak guard must validate queued delivery immediately before applying.
- Effects: this changes only directory metadata. It cannot close/fresh a tab,
  clear input, change resident readiness, or arm/settle a resident stop ticket.

S2 owns row/menu/confirmation callers and retains the same captured clone. W21
owns the isolated value/actor/page entry. G4 and W12 agreed the boundary before
implementation. Opened-panel Archive/Exit/Fresh still requires W12's canonical
stop admission and exact Window path. An external mark does not create that
expectation; its observed stream end follows ordinary full retirement.

The capture factory takes the SAME rendered Machine and Card, refuses missing
context or a foreign row, and retains the original read cursor. Mark::Read may
not borrow a later cursor. Local tickets correlate one pending page update;
they are neither resident operation tickets nor physical admission identities.

Preflight refusal carries no fabricated guard or successful data. Once a call
may have started, retirement, timeout, unreadable/wrong reply and publication
failure remain uncertain; no automatic replay or optimistic input rollback.
Only an original guarded reply for the exact session and pending request can
settle its matching directory work. The ACK does not optimistically install its
unary snapshot into Fleet: the existing canonical Fleet reader owns projection
installation, including a fresh read after settlement. This avoids introducing
another projection-generation cache or racing older queued Fleet snapshots.
Resident resources remain untouched.

Validation must include actual framed mark/reply, alias and same-node admission
replacement before dispatch, retirement after queueing, wrong-session replies,
stale/duplicate local tickets, read boundaries, and no resident input mutation.
Source preparation is not a checked consumer handoff or full A2 approval.
