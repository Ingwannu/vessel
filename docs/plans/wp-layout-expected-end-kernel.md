# Expected end policy in the sole resident binding

Status: source candidate only. No runtime, actual framed carrier, lifecycle owner,
Window callback, product follow promotion or UI claim follows from this model.
The bilateral policy is [the expected-end contract](wp-layout-expected-end.md).

## Decision Log

- Purpose and intent: retain one prearmed typed stop's correlation across either
  End/ACK order while preventing stale records, effects or Window completion.
- Existing assets and constraints: the same `Residents::Resident` already owns
  follow generation, operation admissions and owner votes. App `SessionResidents`
  owns actual resources; its entire module remains test-staged. Connection lease,
  typed stream end, lifecycle intent and Window lifetime are external facts.
- Alternatives: defer all retirement, use EOF as success, add another ticket map,
  or distinguish receiving/sealed/retired in that same binding.
- Choice: the last option plus one optional prearmed expectation in the same
  resident. Its opaque original ticket identifies WirePending/WindowHeld work.
- Why: ACK consumes a wire ticket once; normal End arriving afterwards must not
  destroy the remaining original Window work merely because that ticket is gone.
- Benefits, costs and effects: no new registry, allocator, transport or dependency.
  Generic receive/operation admission rejects sealed state; full exact-binding
  retirement still recognizes sealed state and notifies with an empty operation
  list when ACKed work remains. Owners must handle that binding notification.

## API and data flow

`admit_stop(binding, StopAction::{Archive,Exit,Fresh})` mints and prearms exactly
one existing Lifecycle operation atomically. A second expectation refuses before
mint. This does not send a call or replace original intent/owner preparation.
Existing protected owner votes are not weakened by admission.

Only the actual ordered normal-end consumer, after its original guard and all
preceding record installs, may call `end_follow`. With no expectation it returns
full `FollowRetirement`. Otherwise it returns distinct `FollowSeal`, denies
record/snapshot ingress and new operations, and retires every other ticket.
Duplicate End refuses without retiring the retained expectation. This pure method
does not establish that the caller observed a typed End; the SDK/carrier join is
still required. Generic EOF, fatal error and sequence exhaustion cannot supply it.

`settle_stop` consumes the exact original wire admission before returning either
opaque `StopWindowReceipt` or refusal. Generic `settle_operation` cannot consume
an armed stop. ACK changes only the same expectation's phase and never readiness.
An ACK-first End preserves that exact held local work, without another admission.

Immediately before synchronous Window work, the caller must recheck the SAME
original guard/context plus exact Window, key/claim/tab/ClosePlan, concrete owner
votes and draft conditions. `validate_stop_window` checks only the independent
binding/receipt correlation; `finish_stop_window` consumes that correlation once
before owner mutation. Neither function grants layout or network authority.
`revoke_stop_window` handles original deadline/refusal/stale Window without new
wire work or a budget extension; a sealed binding fully retires and never reopens.

`retire_binding` accepts current receiving or sealed state, retires the original
expectation and returns its exact full notification even with no wire tickets.
`begin_follow`, physical connection retirement and resource retirement cannot
carry Window authority to a successor. The staged App retirement wrapper uses
this full path; its receive/admission validation remains strict. The remaining
prearmed local expectation also blocks clean dehydration even if owner votes are
incorrectly supplied as Ready. No protected input is stored or cleared here.

## Verification boundary

Authored model cases exercise both End/ACK orders for all three allowed actions,
wrong/generic/duplicate tickets, single expectation, full retirement before ACK
and after ACK with an empty operation list, same-node successor refusal, local
revocation, and owner/dehydration holds. They do not execute actual transport,
real input, concrete lifecycle state, window close, IME or effects. They now pass
in the receiving union below. Actual framed SDK typed-end and original guard,
carrier, owner and originating Window integration remain separate required proof.

The first combined strict check, `wp-layout-core-union-20261115T185314Z-bdfdde62`,
rejected one documentation spelling and test-only map access lint in this kernel;
it did not execute tests. The correction changes those two lines only. The same
run's independent Tooltip compile errors and its correction remain separately
scoped; no kernel or UI runtime success is claimed. Corrected base-13 union run
`wp-layout-core-union-20261116T002833Z-dcb0d416` then reached App test compilation
and found missing `Window`/`Context` imports in the new fixture. Only those imports
are added; no assertion or policy changes follow from this compile correction.

## Executed receiving union

Exact `b9df6b605c37539544732878974758faada7a611` completed requested locked
View/Kit/App all-target strict run `wp-layout-core-union-20261116T003646Z-b9df6b60`
with fonts/cargo0 (actual Kit/App Checking; View cached). Same-head runtime
`wp-layout-core-union-20261116T005309Z-b9df6b60` compiled View/Kit/Link/App and
passed 1527/0/2 ignored: App 133, Kit 252, View 1000 and integration and example
tests 142 (integration 131 plus example 11). All nine pure stop
cases and the staged GPUI full-retirement/empty-ticket/input marker case passed
explicitly. All nine kernel Rust blobs equal the executed source; the selected
portable native tree equals that union, without a new-commit execution claim.
The App parent stays literally test-gated. Real original Guard, ordered typed
SDK End, concrete Lifecycle ACK-held state and originating Window commit proof
remain owed; no live follow/stop/close/IME/native-quality/feature approval follows.
