# E2 isolated relay registration lifetime

Current disposition: V3 SOURCE PASS through498; all seven new runtime cases
passed on corrected ef78, but the full gate is RED on one session-start E2E.
No consumer-ready or full relay-consent claim. Physical DNS work remains open.

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


## Corrected-source lint result and final gate boundary

Narrow devices-relay-life-20261115T133940Z-5a02b847 checked the actual changed
relay library and reached its test targets, then failed on unused atomic/watch
return values and a local Notice type declared after statements. No runtime
test ran. Explicitly consume the irrelevant previous atomic values, drop the
completed watch borrow, and move Notice to module scope; no assertion, budget
or production behavior changes and no lint is suppressed. Apply the same
atomic-result discipline to the newer public-run callback test. Both red
predecessors are retained.

V3 source-passed the normal-drain implementation only at ede20014..5a02b847;
that is not a pass for these newer fixtures, merged source or runtime. The
first final whole root gate will cover the corrected current merged source,
including strict Clippy and all new actual relay tests. No extra unchanged
narrow or whole pass is inferred. The separate settings runtime and internal
OS DNS-work limitation remain outside this source verdict.

## First final whole gate: partial, not a runtime pass

Run devices-relay-life-20261115T142643Z-498465b2 checked frozen498465b29.
web-ci, fmt, rust-size, boundaries, whole strict Clippy, TS-feature Clippy
and forge-feature Clippy each returned0. The TS step returned101: proto's
three selected tests and web's declaration freshness passed, while
wire::ts::tests::every_primitive_alias_is_classified_and_every_entry_still_declared
failed with the stale LimitPercent alias entry. The gate stopped there.
Runtime, loopback, workspace, web checks and E2E after TS did not run; zero
runtime tests cannot be presented as a pass or a completed package gate.

The coordinator identified this as the inherited base defect fixed by the
integrator's9dfdf0a1 in gate49's batch. This package has no changes to proto,
web wire or root Cargo relative to82ae7c91. It does not edit that allowlist.
After the announced gated node head49 is available, merge that exact head
and run the full whole once. The first red log remains retained.

V3 extended SOURCE PASS to498, including the public-run callback fixture and
lint-only changes, with GATE OWED. This is separate from the unreviewed
settings runtime370, and does not waive physical DNS-work lifetime/bounds.

## Approved base-only TS correction

The coordinator subsequently allowed exactly9dfdf0a1 without waiting for the
larger gate49 batch. Its one-line primitive alias removal was cherry-picked
as6a0d9d873; no other node-base work is imported. All relay implementation
and fixture bytes remain the reviewed498 source. One full whole is now due
on this corrected frozen tree; the partial run is not upgraded by that intent.
When the announced gate49 head arrives, its identical fix must merge cleanly;
an unexpected conflict is reported instead of rewriting shared source.


## Corrected full gate: runtime exercised, E2E red

The approved alias-only correction ran as the frozen whole
`devices-relay-life-20261115T152818Z-ef78d0fd`. All18 stages actually ran:
17 returned0; web-e2e returned1. Main Rust totals are2396 passed,0 failed,
12 ignored, including each of the five pending-owner unit cases and both
actual secure-carrier/public-run integration cases. Web unit tests464 and
forge4 passed. TS and all three strict variants passed; this is actual runtime
evidence, unlike the first partial gate. It is not a green full gate.

The one E2E failure was `app.spec.ts:61`, the terminal session case. It failed
inside `kit.ts:137` waiting for the new-session route after creating a session;
key echo was never reached. The other67 E2E cases passed. No web source or
session-start source was changed by this isolated package. That observation
alone does not prove a base race, and no test/assertion/deadline is changed.
Keep both full/partial red logs. No third whole rerun is inferred or submitted.
The coordinator receives the actual named failure for the next gate decision.

The integrator has now announced gated node9c150c6e (gate50c, protocol24,
schema20), which includes the identical9df alias change. Its first clean-point
merge attempt had no alias conflict, but independent pending/diagnostics module
registrations overlapped in link.rs, plus two appended documentation sections.
The merge was aborted back to exact ef78 after preserving a private conflict
patch. The additive registrations and both documentation sections will be kept
when merging again; no foreign diagnostics behavior will be replaced.
This does not turn ef78 results into a combined9c source pass. Physical DNS
worker and repeated-toggle resource bounds remain unproved; settings/consent
UI remains outside this lifetime seam.
