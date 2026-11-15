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

## Exact validation and limits

- Base e1b73c9e is the lead's green1432/0/2 line; no native base move after this cut.
- Exact86f55fbe View+Link all-target tests ran1220/0/0 (Link113, View967,
  integration/examples140), run wp-transfer-import-tests-20261115T132643Z-86f55fbe,
  /tmp/work/wp-transfer/import-line-priority-tests-132643, completed13:32:19 UTC.
  All three actual actor/history delivery regressions ran and passed.
- Exact288370e3 strict app/view/link all-target Clippy ran0, actual Checking for
  app/kit/link/view and Finished22.38s, run
  wp-transfer-import-strict-20261115T134557Z-288370e3,
  /tmp/work/wp-transfer/import-line-serial-strict-1-134556, completed13:46:36 UTC.
- Source deltas after86 change no product behavior:8f removes redundant
  std::mem:: qualification;288 removes two now-redundant queue type paths,
  moves the unchanged created-receipt helper before its inline test module and
  explicitly asserts two previously ignored successful history admissions.
  Coordinator6f651138/30d1658a preserves the1220 predecessor proof and permits
  bounded serial strict corrections. No unchanged test rerun is claimed.
- Original queued86 jobs were cancelled only under coordinator8229 permission,
  terminal143 records preserved before one priority replacement each. All later
  strict runs have exact immutable source and separate evidence; no duplicate
  build, local Cargo/Rustc, real agent/browser or service/permission change.
- This final documentation commit changes no Rust/native executable source from
  288. V1 still reviews product export/visibility and actual callers before lead
  adoption. Whole native/GPUI receive/capture and final A4 parity are not implied.
