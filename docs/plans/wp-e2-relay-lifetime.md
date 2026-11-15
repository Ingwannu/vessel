# E2 isolated relay registration lifetime

Modify the existing NodeLink runner and node Relays accept closure; use std,
Tokio JoinSet and existing cancellation tokens. A second relay runner or a
second gateway would have the wrong lifetime. Existing data carriers become
the gateway's when its synchronous accept callback returns; they are not in
the pending registration drain. ADR0096 records why the join is required.

## Scope and references

This package is based on the coordinator-announced gated node head ede20014.
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
