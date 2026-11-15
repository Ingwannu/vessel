# E2 isolated relay registration lifetime

Modify the existing NodeLink runner and node Relays accept closure; use std,
Tokio JoinSet and existing cancellation tokens. A second relay runner or a
second gateway would have the wrong lifetime. Existing data carriers become
the gateway's when its synchronous accept callback returns; they are not in
the pending registration drain. ADR0096 records why the join is required.

## Scope and references

This isolated package started on gated node head ede20014 and now includes
the announced node head82ae7c91 (gate46g), merged before its final package gate.
Vessel's `link.rs199–240/339–351` was the source finding: detached data opens
could survive the registration future. Paseo's relay consent remains the
feature reference in ADR0096; its implementation is not copied. This seam
changes no UI and needs no screenshot as transport evidence.

Product paths: `vessel-relay::link` creates one bounded child owner for each
existing run; `link/pending` reserves before spawn, cancels and joins through
callback completion. `vessel-node::relays::Serving::run` adds one cancellation
fence before handing a carrier to the existing gateway. No protocol/schema,
SDK signature, TLS/reach/registration codec or crate boundary changes.

## Tests and limits

New bounded unit cases exercise capacity/factory admission, completion reaping,
run-drop cleanup, a real loopback websocket upgrade held before its101 reply,
and a final callback held across stop using a test-only blocking barrier.
The latter uses block_in_place and a finite release deadline; production's
callback remains a nonblocking handoff. An actual loopback relay integration
case establishes Noise data before stopping registration and exchanges data
after its completed drain. Every new wait has a named five-second phase and
thirty-second outer safety; task ownership survives failed waits.

The shared relay test fixture now retains its task while awaiting stop and
aborts it on drop, so the new deadline cannot detach the server. Existing
fixture callers retain their consuming stop API and original assertions.
No existing test budget or expected result is weakened.

Source/format/size/boundary preparation only so far. Narrow compile/test,
V3 review and the one final isolated root package gate are pending. No real
relay, preview node, account, service configuration or permission changed.
Remote service-account/numeric UID/GID verification is not claimed.


## First narrow result and correction

Exact7d32d2bf Clippy returned three source lints: NodeLink documentation
needed code markup, two admission arms had the same empty body, and the
unit-valued pending.open statement needed its semicolon. The correction
changes no production decision or budget and suppresses no lint. No tests
ran. The actual changed vessel-relay library was checked; no Node success
is claimed from that failed predecessor.

The callback race regression now holds the actual open() accept callback
across stop, after a real loopback websocket upgrade, rather than simulating
a final callback inside a generic child. Its same five-second phase and
thirty-second outer budgets remain; cleanup owns every pending task. This
stronger test is new source, with no pass inferred from its predecessor.

The socket resolver implementation remains unchanged. Joining these async
children proves their final callback decision, not termination of an OS DNS
worker inside Tokio/tungstenite. Loopback tests do not prove physical DNS
cancellation or resolver-work bounds across repeated control enable/disable;
that full-consent limitation was sent to V3, not silently treated as a pass.
The existing accepted secure-carrier lifetime remains the handoff boundary.


## Node base reconciliation

The82ae7c91 merge preserved both the incoming CLI presentation notes and this
package's shutdown notes in README. Its incoming changes alter no byte under
vessel-relay or in vessel-node's relays.rs. The queued corrected narrow run
devices-relay-life-20261115T133940Z-5a02b847 remains evidence for its exact
pre-merge source only. The final root package gate must use the merged head;
no old result is relabelled as a combined-tree pass.


## Public runner acknowledgement barrier

A further integration regression holds the actual NodeLink::run accept callback
after a real relay data open. It observes that the public run's JoinHandle and
Stopped state cannot acknowledge while the callback is held, then releases it
and joins the same owned handle. This catches omission of the pending.stop
hook itself, which an owner-only DataOpens unit case would not catch. The
existing encrypted-carrier survival assertions are retained; their common
registration setup is shared. Named five-second phases, outer thirty-second
safety and abort ownership are unchanged.

This additional test is newer than the queued5a02 narrow source. That narrow
result cannot cover it; the final merged package gate must compile/run it.
No additional narrow run or runtime PASS is claimed from this preparation.
