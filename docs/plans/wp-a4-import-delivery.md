# A4 first product packet: own-history import and actual queued delivery

Base: lead line e1b73c9e. This is a straight native-only extraction; root crates,
web and both root Cargo files are unchanged. Native history resumes through the
existing agent.history/session.create and existing-session get/resume calls.

[Decision Log]
- Purpose: deliver usable own-history import and free the shared delivery writer
  slot without importing unfinished subagent/resident/draft-seed ownership.
- Existing assets: W12's checked canonical lease and Weak DeliveryGuard, current
  protocol history calls, kit controls and the actor's framed test peer.
- Choice: real actor-admitted scope, guarded RPC before/after polling, weak queued
  answers validated immediately before Shell model application. None is allowed
  only through a correlated-error constructor. Follow remains literal cfg(test).
- The protocol getter has a real existing-method capability caller. No wire call,
  number, fake guard, alias fallback, client/allocator API or new dependency is added.
- History retains at most 10000 rows / 16 MiB owned metadata; each explicit import
  has at most100 entries, sequential receipts, no replay of uncertain effects.
  Capacity refusal preserves accepted rows and selections. Native titles/accounts/
  folders and provider continuation remain bound to the original admission.
- One testing::actor fixture drives ordinary Command through State::handle and
  actual typed framed Call/Outcome/Event. It privately admits and retires Connected
  and closes the actual peer; callers get no client/State/allocator/guard factory.
  It is crate-internal cfg(test), not an App GPUI test-support export.
- Scope excluded: provider/managed child DTOs and runtime, resident/query minter,
  child renderer/find/snapshots, archive-finished bulk, curated draft fork and B9.
  Those stay in the separate A4 trees; no full A4 parity claim follows here.

Validation on this extracted immutable head is owed. Prior full-tree tests are
predecessor evidence only. Strict app/view/link and Link tests run on the VM;
V1 reviews the product export/visibility and actual callers before lead adoption.
