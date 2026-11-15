# Correlated schedule calls and the app reply boundary

[Decision Log]
- Purpose: list, pause, resume, run-now and delete use the node's existing typed
  calls and always settle the requesting model, including local rejection.
- Constraint: a slow run may last beyond navigation or disconnection. The
  fleet loop and conversation events must not wait for it. Missing target
  facts must not erase a successful schedule list or claim target deletion.
- Alternatives: inline actor awaits, unbounded spawns or optimistic local writes.
- Choice: four shared named permits, held through bounded answer delivery;
  independently fallible project/worktree reads only when target facts require
  them; original errors carried to the app's presentation boundary. The model
  retains its own matching ticket ledger, including invalidated operations.
- Why and cost: explicit Busy before dispatch; absent connection and queue
  rejection remain known-unsent; a disconnected/mismatched write reply is
  Unconfirmed. No write retry and no fake browser success. Supplemental facts
  are from bounded node lists; absent names remain unavailable, never gone.

Data flow: schedules::Request → Command::Schedules → schedules::Hub → existing
NodeClient methods → Event::Schedules → areas::schedules::Area → view::Page.
A full/stopped command queue produces the same typed Answer via unsent::settle.
Browser builds answer NoTransport without RPC or fabricated schedules. Every
actual call still relies on the client's method-since check, without a local
protocol-number guess. ScheduleError preserves CallError as its source; its
Display is diagnostic only. The app classifies supplemental read failures only
when preparing the view snapshot.

The app module currently only settles replies. No route, sidebar entry,
control, create/edit/history item or artificial schedule data is exposed in
this registration prerequisite. Drawing, viewer-zone formatting and first-use
fake-fleet verification remain later parts of the first list slice.

Other-owner seams: Link module/Command/Event/State initialization and one match
arm; the browser's one refusal arm; the existing machine fixture's one Hub
field; Areas module/state/init; one Shell event arm in its newly split dispatcher;
and the unsent match arm. No other area's renderer or policy changes.
The LinkError enum remains unchanged; rustfmt only removes the copied trailing
separator blank line from its new file. The first move-only fmt check reported
that separator and is not recorded as a pass. The actual enum/doc body is
unchanged after normalizing this final separator. Final Link record is lowered
from988 to the measured951; no allowance is added or raised.

Verification is pending at this checkpoint. The two moves are source-only;
only the final registration head receives a whole A13 gate. Old5b checkpoint
results cannot prove these later files. Captures and actual RPC/fleet first use
are not supplied by unit error/correlation checks.


The app root's500-line bound required a separately authorized move-only shell
child module, between LinkError and this commit. This registration adds exactly
one schedule dispatch arm to that module. Its parent root now measures416;
the dispatcher98. The original Link command handler was also at its function
budget: the new feature delegates to its own State::schedule method rather than
adding a multi-line RPC branch to that existing handler. No old command body
or the other track's options block is moved or altered by that delegation.

Local selected tests passed2 Link and2 App cases on the pre-shell-move working
layout. The subsequent all-target Clippy correctly rejected the overlong new
root branch and unwraps in three test helpers. Those failures are retained, not
reported as a pass. The branch now delegates and deterministic setup unwraps
are within test functions; no lint or expected output was weakened. The moved
layout and final helper forms still need the exact head's native gate.
Typed node refusals and local Busy now retain corrective help even for initial
load failure (all18 portable cases check this decision); only an ordinary
offline read uses the reference's generic failed-load path. A pairing refusal
must not be hidden behind a retry that cannot fix its credential.


## Registration repair and connection lifecycle

[Decision Log]
- Purpose: preserve the existing Reload command and make a newly established
  connection observable even when its Fleet is byte-for-byte unchanged.
- Constraint: the root Link module cannot grow beyond its shrinking record;
  another track independently owns the options/keep_commands move.
- Alternatives: infer reconnection from Fleet differences, duplicate an area
  hook, or raise the root allowance. Each hides a real boundary or adds debt.
- Choice: restore `Command::Reload => self.reload().await` unchanged; move the
  private `State::hand_kept` unchanged into `nodes.rs` beside the single generic
  `connection_ready`. That helper hands kept offers first, then fans out a
  closed typed list of area notices (currently Schedules Connected only).
- Why and cost: a fast reconnect resets old Forbidden evidence independently of
  Fleet equality. Future areas add a notice entry at this one connection point.
  No RPC, credential, pairing, persisted option or existing Reload behavior changes.

The moved `hand_kept` method's exact SHA-256 is
`12bf687a41a413c4bf2d0b23fcd7a1dc18739ec181793866ffd8005d32aa0ddf`.
The root's inherited method comment moves with it; the options block stays put.
The root record falls again to its measured count, never increases.
A regression uses the existing framed peer fixture with a fresh client sharing
its prior NodeId: it verifies Connected is delivered before an identical Fleet.
The app regression establishes Forbidden, applies Connected and refuses a stale
old-connection answer. The shared typed JSON fixture is decoded only in tests;
App gains the existing workspace serde_json as a dev dependency, not runtime I/O.

The immutable registration run on `1ae85911` failed native-clippy with E0004:
Reload was absent from the exhaustive command match. Fonts, fmt and quality
passed; build and tests were not reached. That red result is retained as such,
not attributed to repaired source. Per A14, the repair receives narrow checks;
the next whole A13 run belongs to the completed visible list package.

Narrow repair verification: changed View/Link/App/Kit all-target Clippy passed.
View/Link/App library/binary tests passed 877 + 59 + 40, failed 0, ignored 1.
Both new lifecycle regressions ran and passed. Actual Checking/Compiling entries
exist for the changed crates; no old artifact is claimed as repaired evidence.
The digest above is UTF-8 from the method signature through its closing brace,
with one final newline (802 bytes), identical before and after the move.
Final measured Link root is 926 product lines; the record is lowered to 926.
The separately queued Alert-only tests are not part of this repair claim.
No RPC/fake-fleet/visible list/OS interaction claim follows from these unit checks.
