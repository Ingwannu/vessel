# Providers: confirmed removal over the existing link

This follows the accepted rules-only `3177e772`. It connects that intent to
the existing Agents command, worker and answer paths. No Remove trigger or
confirmation dialog is drawn yet: the shared ProviderRow control remains
with its owner. This is not complete Providers or profile editing.

## Contract and reference

Paseo 0.15.4 `screens/settings/providers-section.tsx:111–171` and `410–440`
confirm an added provider's removal, serialize the action and clear its
pending state after failure. Vessel uses ADR 0032's existing `agent.remove`,
introduced in protocol 11. The node retains admin authorization and the
refusal when an account profile extends the agent. No wire, node or root
workspace code changes here.

[Decision Log]
- Purpose and intent: a confirmed book name must not redirect a mutation
  to a replacement connection, and a lost acknowledgement must not cause
  a repeated removal.
- Existing assets and constraints: `Pages`, its one-shot `RemovalRequest`,
  `Command::Agents`, the four-permit Agents hub, and the original typed
  client call/error already exist. The node accepts a name without a
  revision precondition and may finish a write after the caller stops waiting.
- Alternatives considered: a separate removal queue, a name-only callback,
  an automatic reconnect/retry, or treating a timeout as confirmed failure.
- Choice: consume the view request through `Command::remove_agent`; retain
  its machine, physical node, name and ticket as one operation. The current
  client's immutable node identity is checked in the worker immediately
  before the call, with no intervening await. Protocol refusal also precedes
  any removal frame. One named 30-second deadline bounds the unary call.
- Why: this keeps the existing transport/backpressure policy and lets the
  existing ACK consumer settle exactly the original page ticket. The
  private-field target cannot be freely manufactured by drawing code.
- Advantages, costs and effects: no automatic resend. An uncertain outcome
  retains visible facts but requires a successful discovery before another
  confirmation. Hidden definition changes after dispatch remain the existing
  wire limitation; this is not a distributed revision transaction.

## Replies, failures and retained state

An acknowledged removal retains the actual connection's `NodeId` and the
confirmed `AgentName`. The existing view transition checks machine, ticket,
operation, node, name and current snapshot identity. It removes only that
definition's facts and closes only that selected detail. Other selections
and the retained install log survive. The app refreshes its existing fleet
and active provider page after an accepted ACK; stale replies do nothing.

`Forbidden`/`Unauthorized` and profile `Conflict` retain their complete typed
wire error and original diagnostic. Their instructions are separate complete
English/Korean messages. A changed node and an older protocol are pre-send
refusals. A disconnect, unexpected reply, deadline, or potentially incomplete
internal/unavailable node error is an explicitly unknown mutation result.
No transport diagnostic is interpolated into a translated sentence.

The page's typed `RemovalFreshness` requires discovery after uncertainty.
A failed discovery, unrelated status check, or stale answer cannot clear
it. A successful current discovery clears it; if it establishes that the
selected definition is absent, those obsolete details close. Definite
refusals retain the earlier retry behavior. Removal errors use the existing
page alert above either the list or details, with no new control skin.

The public command constructor is the app/link boundary consuming the view
ticket, not a public internal service added to silence dead code. Its
integration test drives confirmation, command, the actual hub, RPC ACK and
the original page transition. The production command handler and ACK/error
consumer are connected; the UI producer remains explicitly absent pending
the reviewed shared control. No staging flag or lint suppression is added.

## Verification boundary

Local narrow Clippy for link/all targets passed. Nine removal RPC tests and
41 agent-status view tests passed on the final preparation tree. Tests use
the existing RPC memory transport and real `NodeClient`, including handshake,
request and cancellation behavior. A post-action read drains earlier writes
when proving that an old/replaced peer received no removal. The hub's actual
permit bound is exercised with pending requests. Tokio's dev-only test clock
feature proves the product deadline without sleeps or a longer timeout.

Preparation failures remain red in their logs: incorrect fixture RPC variant
names and unused results, overly wide test visibility and wildcard matches,
then a new fixture that encoded an added harness as a builtin string instead
of its documented object shape. They were corrected without relaxing an
assertion, lint or deadline. The exact-commit remote whole native gate passed
on `7a2c7eb2`: `agents-20261115T032805Z-7a2c7eb2`, every native step zero
with no skips, 1059 runtime passes plus two document passes, zero failures
and the existing ignored private browser capture. All nine RPC, three
uncertainty and one new alert cases passed. Changed crates actually compiled.
Queue wait was 808 seconds; execution took 62 seconds, load 8.17 to 8.35.
The preceding native all-target Clippy run also passed. V1 later returned only new-test liveness, as recorded below; that old
execution is retained without rerunning its unchanged source.

These tests simulate typed authorization/conflict replies; they do not prove
real node administration policy or persisted file mutation. No real agent,
installation, preview node or production configuration was used. There are
no new UI captures for this call-only slice and no claimed removal-screen
approval. The earlier 40-image normal/list/detail packet has its own scoped
V1 visual pass; it is not evidence for this new call path.

## Review correction: finite test phases

V1 returned `7a2c7eb2` for new-test liveness only, with no additional product
blocker. Its successful execution remains evidence for that old source;
the returned source is not marked accepted. The correction adds a named
finite budget to handshake, peer reads/writes, ordered barriers, hub delivery,
observation, event receipt and task completion, plus an outer safety net.
The original assertions, ordering, product deadline and explicit virtual-time
advance stay intact. The peer's intentional idle budget and outer budget
include the product-time advance, so neither changes that case into an early
disconnect. Abort guards are retained before setup awaits and around the
spawned call, ensuring that phase or outer failure cannot detach those tasks.

Only tests and these notes change. A changed-source A13 run is required;
the previous green gate is not reused as proof of the correction. Models
implementation is parked separately until this return is addressed.

[Decision Log]
- Purpose: make a broken fixture or event path fail with its phase name,
  and release spawned work when a test exits early.
- Constraint: this separate native workspace uses only memory transport;
  the root testkit's `within` always prepares process fakes with a nested
  root build, which is inapplicable here.
- Alternatives: add that root dependency merely for its helper name, or
  rely only on the product deadline. Neither bounds this suite correctly.
- Choice: use the existing Tokio dev dependency for a test-local outer
  safety net and named phases. The outer/peer-idle budget includes the
  product deadline; ordinary phases retain one five-second constant.
- Why: no fake process, new dependency or product behavior is needed.
- Costs and effects: timeout panics identify the phase and limit. The
  paused case finishes the observation phase before its thirty-second
  advance and starts its join phase afterward; no short phase spans it.
  Its later barrier also proves the peer survived the intentional wait.

The correction's local narrow Clippy passed, and all nine removal tests
passed, including the paused-time case. Both changed native crates were
actually compiled. A comparison retained all 34 original assertions in
order after removing only timeout wrappers; the product source and explicit
virtual advance were unchanged. Formatting and compile-free quality checks
precede the immutable correction commit; A13 and V1 disposition follow.

The first correction gate on `6fe855ab` stopped at native Clippy: the last
added rationale comment omitted code markup around the root helper's name.
Fonts, formatting and quality passed; build and tests did not run. That
execution remains failed. The markup is corrected without changing a test
or product instruction, followed by a new exact-source A13 run.
