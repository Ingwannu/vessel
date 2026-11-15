# Bind owned RPC authority to the actual launched process

This is the correction contract for V3 row268, not an implementation verdict.
The frozen implementation4cb3581b remains returned even where its ordinary tests
pass. Shared RPC changes wait for the announced node head with W19's caller-drop
fence; preparing the fixture does not bypass that writer order.

## Decision log

- Purpose: owning a direct Child must not authorize stopping a different process
  that reuses the home's persisted NodeId, token and canonical endpoint.
- Existing constraint: Ready currently identifies the persistent node only.
  Racing child wait against Serving can combine an old Child with a replacement
  NodeClient. The node lock can be released before process exit. Connection
  closure, wait bias, another alive check and wall-clock started_at do not bind
  the two executions.
- Alternatives: lock-only evidence; trusting Ready/identity; a random pipe name
  alone; a new public RPC; unsafe peer APIs. These either retain the race, expose
  the token to a squatter, or violate the approved boundaries.
- Choice: prove the peer on the actual opened local transport before sending
  any persisted token. Unix uses the kernel peer PID against the still-unreaped
  direct Child. Windows uses the private single-instance transcript below.
- Why: ordinary connections remain Attached and cannot acquire owner authority.
  The proven transport never reconnects; later owner-only calls use that same
  connection. A missing/mismatched/unsupported proof is a startup refusal for a
  launch we started, never an automatic Attached fallback.
- Costs: a split open/prove/authenticate local-client seam, a bounded private
  bootstrap transcript, platform-specific tests, and a separate feature-only
  real-node fixture. No arbitrary PID signal or second process owner is added.

## Unix and connection lifetime

The kernel peer is observed on the opened Unix stream before credential output.
Its PID must match the retained unreaped Child, whose PID cannot be reused at
that point. Query failure or absent peer data refuses Owned. An already reaped
or released owner cannot satisfy the check. Use biased child-wait-first selection
as additional defense, not as the ownership proof.

The current NodeClient opens once and passes one framed stream into one RPC
driver. Clone shares that Connection; transport closure is terminal and there
is no transparent redial. The correction must preserve this property. Reopening
the endpoint requires a fresh proof and cannot retain an earlier Owned lease.

## Windows private transcript

Other local users are in scope. Same-user processes are excluded because they
can read the private home token. Pipe names can be enumerated, so their random
spelling is not a secret capability and default DACLs are not process-private.

1. Each launch gets a fresh OS-random private pipe name. It is child-only, never
   persisted or logged. Canonical IPC remains unchanged and has no challenge path.
2. The child creates its dedicated bootstrap server with first_pipe_instance,
   reject_remote_clients and max_instances(1). Creation conflict refuses startup.
3. The parent opens the pipe without sending the node token, then generates one
   fresh bounded OS-random challenge for that opened connection.
4. The child reads exactly one challenge on that exact retained server instance
   and echoes it in its inherited stdout Ready record. No second pipe instance,
   alternate channel, recreation or reconnect may produce the accepted echo.
5. The parent strictly checks the bounded exact stdout framing and compares its
   challenge in constant time. Duplicate/extra/wrong/missing/late input refuses.
6. Only then does normal token authentication run over the same preopened handle,
   followed by Owned RPCs. Any lost handle remains lost; no namespace fallback.

If these constraints cannot be established safely, the platform remains typed
Unsupported for Owned. unsafe_code stays forbid. Windows compile plus a console
probe under wine is only an approximation; real Windows behavior stays unverified
until actually executed there.

## Causal fixture and checks

The node-package vessel-local-node-fixture follows the existing forge fixture:
required feature local-node-fixture, no default/release/install activation, no
new crate edge or test-only controller API. It runs the real Node with explicit
scratch home/programs and no TCP listener. A bounded loopback control stream
reports public process facts and requests only that fixture's orderly stop.

The Linux regression holds a blocking-file-read barrier after owned spawn, lets
the real original child emit Ready and exit, then starts a real same-home node
with the same persisted identity/token. Read-only zombie observation of the known
fixture PID witnesses actual exit without reaping it behind the controller.
The replacement's Serving reply is ready before the original exit receipt is
observed. Releasing the barrier must yield the specific peer refusal, account
for original-child cleanup/reaping, and leave the replacement Serving. Subsequent
reuse is Attached and its stop/restart must refuse.

Also retain normal Owned/Attached, malformed readiness, cancellation, deadlines,
uncertain cleanup, release and keep tests. Windows tests must cover precreation,
post-exit takeover, attempted second-instance relay, stale/wrong stdout echo,
token-not-sent-before-proof and same-handle post-proof closure. Fake/Linux cases
do not prove Windows instance or DACL semantics.

The fixed feature build/test step belongs to T's gate tooling, as test-forge does.
No current default gate result is claimed to execute this staged feature.
