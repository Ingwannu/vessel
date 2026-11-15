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
- Choice: for an owned launch, prove the peer on the actual opened local
  transport before sending any persisted token. Unix uses the kernel peer PID against the still-unreaped
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

1. Each launch gets a fresh OS-random private pipe name. A bounded typed name
   seed travels only through the managed child's dedicated stdin, followed by
   EOF, rather than argv or an inherited environment variable. It is never
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

## Shared transcript values

`vessel_proto::local_launch` owns the private transcript once, alongside the
existing readiness record. `BootstrapChallenge` contains 32 caller-supplied OS
random bytes; its exact URL-safe unpadded representation has a fixed width.
`BootstrapReady` contains the public NodeId and the echoed challenge. Neither
value grants process authority, creates entropy, authenticates a connection or
compares a response on the launcher's behalf. Debug output redacts the challenge and the separate `BootstrapPipe` name seed.
Strict decoders reject alternate markers, widths, UUID spelling, padding and
extra lines before a consumer can use the data. The launcher must compare its
own challenge in constant time after correlating the retained stdout and pipe.
These local-only records add no RPC method, schema or TypeScript wire type.
The prepared value tests and feature regression are not execution evidence until
their exact source runs.

## Node-side private server assembly

On Windows, only the explicit managed binary mode consumes the bounded stdin
seed and places it in `Config.desktop_bootstrap`; the library discovers no
process environment. Node startup binds the first and only server instance
before issuing readiness. Creation errors use a typed, path-free error. The
tracked bootstrap task connects once, reads exactly the fixed challenge width,
and hands its result to the binary for inherited stdout output. It then moves
the same server handle into existing local token admission and RPC serving.
There is no accept loop, alternate challenge endpoint or instance recreation.
Node cancellation also stops the pending challenge, and a dropped result
receiver closes the instance before authentication.

The public node handle's existing observation/shutdown methods move unchanged
to `node/handle.rs` to keep assembly within its existing file budget; the size
record only decreases. This extraction does not add another shutdown owner.
The node-side code is preparation until the launcher split and qualified tests
join it; it is not a working Windows Owned handoff by itself.

## Canonical Attached compatibility limit

The coordinator scoped the credential-before-proof requirement to Owned
promotion. Canonical Attached probing remains the existing shared client path:
on Windows the globally enumerable canonical pipe receives the token first.
This is a known pre-existing Windows local-IPC limitation, not a guarantee
provided by this package. Unix's canonical socket remains inside the private
0700 node home. The launch-private proof must never be described as protecting
that separate canonical Attached path.

A subsequent shared base item will add mutual token-knowledge proof to canonical
Windows IPC for the CLI, attach and desktop together, through vessel-rpc's local
transport. It requires its own ADR and V3 review after this package; it is not
implemented or activated here. Refusing the current compatibility path was
considered, but would break existing Attached reuse and Keep-to-reopen behavior
without fixing the other clients. This documented compatibility decision does
not relax Owned's private transcript or permit token output before Owned proof.

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

## Windows API probe scope

`managed_local_pipe` prepares Windows-only integration probes against the real
safe named-pipe API: precreation refusal with redacted errors, second-instance
refusal before and after connection, authenticated RPC over the retained
original handle, canonical input unable to settle the private challenge, and
post-proof namespace replacement unable to receive bytes from the old handle.
These in-process node probes do not prove the parent/child inherited-stdout
correlation or the complete Owned controller. Those causal tests remain part of
the later launcher join. Until executed they are source only; wine results must
remain explicitly approximate, and actual Windows execution remains unverified.
