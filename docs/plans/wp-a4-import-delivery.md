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
  exact run logs, completed13:32:19 UTC.
  All three actual actor/history delivery regressions ran and passed.
- Exact288370e3 strict app/view/link all-target Clippy ran0, actual Checking for
  app/kit/link/view and Finished22.38s, run
  wp-transfer-import-strict-20261115T134557Z-288370e3,
  exact run logs, completed13:46:36 UTC.
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
- The predecessor documentation commit f7cff1ac changes no executable source
  from288. V1 still reviews product export/visibility and actual callers before lead
  adoption. Whole native/GPUI receive/capture and final A4 parity are not implied.

## R223 correction batch (source prepared, checks pending)

[Decision Log]
- Purpose: settle the four source findings without expanding the shared slot.
- Constraint: the resident guarded opener is not a product API yet. Receipts
  therefore remain in the import sheet; automatic alias Open and global Reload
  registration are removed. Existing rows say Restore session. This delays direct
  navigation but preserves the active draft and conversation across reconnection.
- Choice: one persistent ticket source captures each rendered visit, original
  scope and row selection. Row/footer/picker/dismiss intentions reject stale
  captures before focus, model or command changes. Every accepted intention and
  accepted page invalidates old captures; close/reopen cannot reuse a ticket.
- Publication retains the existing eight-call semaphore and bounded Event queue.
  After guarded publication's five-second deadline, only the correlated refusal
  waits for receiver capacity or closure. A second dropping timeout could lose a
  committed write receipt forever. The cost is bounded retained tasks while the
  live receiver is stalled; no create is repeated and no unbounded queue is used.
- Native tests reuse Tokio's existing test-util feature for deterministic clock
  advancement. Framed peers own their task before handshake suspension; normal
  cleanup aborts and joins with ownership retained through named test phases.
- Requested regressions: actual retained GPUI row/footer/glyph/Escape callbacks
  across reopen and scope ABA, current-row control, postvalidation receipt with
  alias replacement, full publication queue beyond both previous deadlines,
  withheld-Welcome phase failure and observable owned transport destruction.
- The shared selected/export/actor files are untouched by this correction.
  Exact strict and affected tests remain pending. Earlier1220/0 and strict0 are
  immutable predecessor evidence; R223 remains RETURN until V1 rechecks.

## R223 remaining popup path: first-use parked

V1 accepted the source corrections for P2-1/3/4 at573071f7, with strict/tests
still owed. P2-2 also covers the shared combobox engine's key/outside handlers,
which mutate the current popup before the area's choice callback. The shared
picker owner owns that per-open admission change. Per coordinator/lead ruling,
this packet parks popup first-use completely: current machine/harness/account
remain read-only labels, with no combobox registration, trigger, state accessor,
choice callback or area interception. Import uses the existing current-draft
machine/harness/default account. In-sheet scope selection is unavailable until
a checked shared admission seam supports a separate follow-up. This is an
explicit feature limitation, not an assertion that the engine issue is fixed.
The row/footer/dialog captured-visit regressions remain applicable and required.

## First executed correction result: fixture panic exposed

Exactdeb5 App/Link test run reached Link114 passed/1 failed. Stronger owned-peer
join detected a previously ignored fixture panic: the mismatched-create case
used Behaviour::Session, whose response helper only accepted get/resume. The
old dropped JoinHandle let disconnection satisfy Uncertain for the wrong reason.
The fixture now returns the scripted typed CreateSession response for that call,
so the unchanged wrong-directory assertion exercises receipt validation and the
owned task must also join successfully. No product or shared actor code changes.
App tests were not executed after the Link failure; current results remain owed.

The exactdeb5 strict run also returned three redundant test helper paths and
Clippy's await_holding_lock on the first call-log assertion block despite its
explicit drop. All three helpers now use their already imported answer function;
the first synchronous assertion uses a lexical scope before bounded peer reap,
as the other call-log cases already do. No assertion, budget or lint is relaxed.

Exactc625 Link tests115/0 now include actual malformed-create receipt validation
and owned peer cleanup. Strict returned one wildcard-enum fixture branch; the
fixture now separates explicit Created from existing-session responses and checks
the actual method before constructing its typed response. No wildcard allowance
is added. The requested App/Link --lib command selected only Link because the App
is a binary crate; no App execution is inferred. The next App regression explicitly
selects its binary target. Product and shared owner files are unchanged.
