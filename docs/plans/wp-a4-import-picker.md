# Import selectors on the shared per-open picker

This is a separate consumer follow-up to the adopted minimum import sheet.
Automatic Open remains parked; selecting an account or target never navigates,
reloads another session, clears a composer or changes a managed relationship.
The implementation is prepared, with consumer execution and review still owed.

[Decision Log]
- Purpose: restore the machine, harness and account controls from Paseo's
  import-session-sheet/list flow without stale rendered input changing a newer
  opening or following a rebound book alias.
- Existing assets: the sheet's persistent visit/scope capture; shared
  ComboboxState and G2's checked per-open weak render lease (on the announced
  package base); original ConnectionContext and selected-call/DeliveryGuard
  fences; existing agent.history and session.create resume intake.
- Alternatives: a second popup renderer or an area interception before toolkit
  focus/highlight/close would duplicate the shared engine. Resolving a machine
  alias at choice time would substitute an unobserved connection.
- Choice: keep the same controls, geometry and kit engine. Each rendered row
  captures a full Scope, including original connection, harness and account.
  Choice rechecks both the drawn visit/scope and current exact metadata before
  local state or request effects. Actual RPC/reply admission remains in link.
- Lifecycle: opening is unavailable while a history/profile read or import is
  pending. Every accepted visit/reply/scope replacement closes the old engine
  cycle. Fleet's sole installation calls history_fleet immediately after its
  memory replacement; exact DeviceRetired calls history_retired before Devices
  may return. Only control metadata changes retire the opening, not ordinary
  session activity. Delayed retirement absent from the current scope/fleet
  does not close a successor.
- Tradeoff: unavailable/stale connection metadata keeps the selector disabled
  until explicit fresh sheet entry. UI leases and fleet labels convey no RPC,
  account or workspace authority. No new transport or context store is added.

## Boundaries and verification

All logic and tests are in the import area's picker modules. The shell receives
only the two registration calls. A test-only shell/events child module invokes the actual
private dispatcher without starting IO or widening its product API; it exposes
only the two typed metadata events. It cannot mint a transport/guard/admission.

The GPUI regression retains actual already-open A controls, opens B inside the
real pre-key dispatcher boundary and applies A's old keys/pointer/outside input
before redraw. Reopen, scope ABA, same-node reconnect, alias rebinding and exact
retirement use the same fixture epoch source. It checks B scope/open/parent/focus,
no new history request, no inherited highlight and a positive current B choice.
The dispatcher hook assertions run before returning from the actual event arm.
These are planned consumer checks, not borrowed proof from the Kit suite.


## First receiving compile correction

The first exact796 consumer pair stopped at compilation, before GPUI execution.
The retired event carries the canonical ConnectionTag (original alias/epoch),
not a ConnectionContext. The owner now compares that tag with each already
captured context's tag; it neither projects a second node identity nor guesses
an epoch. Test-only path qualification, unit callback terminators and the literal keystroke lifetime are
corrected in the same batch. The original diagnostics stay recorded; no test
scenario, assertion or deadline is weakened. No runtime result is inferred.
