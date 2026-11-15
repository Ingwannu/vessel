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
