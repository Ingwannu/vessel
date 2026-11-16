# Bind owned RPC authority to the actual launched process

This is the correction contract for V3 row268, not an implementation verdict.
The frozen implementation4cb3581b remains returned even where its ordinary tests
pass. Shared RPC changes start only after the announced node head ed4cd137 with
W19's caller-drop fence; that exact head is now merged before the split below.

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

The launcher installs a Tokio SIGCHLD listener before spawn and retains it in
the controller for its whole lifetime, including Keep. This replaces inherited
SIG_IGN/SA_NOCLDWAIT that could otherwise auto-reap an exited child before peer
qualification. Registration failure is a typed launch refusal, never a PID or
child-alive fallback. An embedding host that changes SIGCHLD after registration
is unsupported, matching nodectl's existing limit. This is a local launch
prerequisite, not a change to Proc's shared spawn/seed contract; other spawners
do not use a retained PID as this ownership proof.

This registration is process-global, rather than a reversible per-controller
setting. Tokio 1.54.1's Signal caveat explicitly says its installed OS handler
is never unregistered, including after the last Signal value drops. Stopping or
dropping this controller therefore does not restore inherited automatic child
reaping; an embedding must account for its other children as well. Retaining the
listener is the controller's explicit ownership precondition, not a promise to
restore the embedding's prior disposition on teardown.

The Linux fixture re-execs only the isolated test binary under `/bin/sh` with
ignored CHLD and an empty environment. It first witnesses actual auto-reaping
and ECHILD without registration; after the real launch helper registers, it
witnesses the known child's zombie state until its explicit wait. The outer
Proc-owned group and finite deadlines bound the fixture. No operator config,
real agent, arbitrary PID signal or test hook in the public controller is used.
This fixture is prepared source until its exact runtime result is obtained.

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
record only decreases. Shared RPC/token/closing/task-tracker construction also
moves unchanged to `node/rpc.rs` to keep the startup function below its existing
line limit. Both transports still share those same original resources. These
extractions do not add another shutdown owner.
The node-side code is preparation until the launcher split and qualified tests
join it; it is not a working Windows Owned handoff by itself.

## Open, qualify, then authenticate

`vessel_rpc::LocalTransport` owns one opened OS handle and exposes its qualified
kernel peer PID where available. It sends nothing on open, has no Clone or
reconnect operation, and adds no Proc dependency. Windows PID queries explicitly
refuse rather than invent evidence. `vessel_client::LocalConnection` keeps that
transport private: `NodeClient::open_local` opens without credential or Hello
output, `peer_process` observes that same handle, `send_launch_challenge` writes
the one typed private challenge, and consuming `authenticate` presents the token
and starts RPC on that same handle. Private open errors do not retain its name.
Canonical `connect_local` preserves its existing Attached policy by composing
the same opener and authentication directly.

The controller's Unix path compares kernel peer PID against its retained,
unreaped direct child before authentication or any child wait. ProcessLifetime
is used only to exclude already revoked/reaped/released local authority, never
as a claim that an OS process is alive. Windows Started carries a nonoptional
preopened connection whose fresh stdout echo matched in constant time; Unix
Started instead requires the kernel query. The authenticated greeting and
Serving state still have to match the intended persistent identity. Child wait
is biased first only after proof; it is additional defense, not the proof.
Existing getrandom supplies independent 32-byte name/challenge values, and
subtle supplies constant-time echo comparison, per the integrator's approved
external-dependency ruling. No rand or hand-written cryptographic compare is
introduced. Root lock generation and exact-source qualification remain owed.

The managed binary invariant is exactly one managed stdout record, with all
ordinary logging elsewhere. The reader strictly bounds its first record and
rejects prefetched extra bytes/duplicate records before admission. It does not
wait for daemon EOF or claim no future byte can ever be emitted. A later record
never replaces proof or permits redial. This follows V3's framing clarification;
stream/API tests and Windows execution remain separate required evidence.

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

The Linux regressions share one actual stopped-server barrier: the original
child emits Ready, its server stops and releases the home, while the retained
child process remains alive. A real same-home successor then answers Serving
with the same persisted identity/token. One case leaves the original alive;
another exits it and observes its known zombie without reaping it behind the
controller. In both cases, the successor is ready before the original exit
receipt can win. Releasing the file-read barrier must yield the specific peer
refusal, account for original-child cleanup/reaping, and leave the successor
Serving. Subsequent reuse is Attached and stop/restart must refuse.

A third case replaces the stopped endpoint with an actual listener in the test
process. That wrong peer must observe EOF with zero credential or Hello bytes,
while startup returns PeerUnverified and reaps only the original child. It uses
the same real child and held file-read barrier; no controller test hook or fake
peer-credential result can hide token output before proof. These authored cases
remain unexecuted until the feature test run records their actual results.

Also retain normal Owned/Attached, malformed readiness, cancellation, deadlines,
uncertain cleanup, release and keep tests. Windows tests must cover precreation,
post-exit takeover, attempted second-instance relay, stale/wrong stdout echo,
token-not-sent-before-proof and same-handle post-proof closure. Fake/Linux cases
do not prove Windows instance or DACL semantics.

The same-home causal proof is now selected by the local-node library unit tests;
its testkit locator builds the exact feature fixture before the case clock.
The separate managed_local_node integration target continues to use Cargo's
same-package real vesseld binary. Neither path borrows a shared old binary or
silently skips an unavailable feature build. The actual run must confirm both
same-source binary compilation and all named test execution.

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


## Announced helper-base join

The bc9cacc6 helper base's command diagnostic privacy applies to both ordinary
and releasable Proc spawning. The existing cleanup guard, release capability
and first-sweep behavior are preserved; the shared private spawn-command helper
wraps failures with the caller's existing diagnostic policy. The desktop's
Runtime refusal has no command payload and retains its typed cause. Managed
logging delegates to the binary owner's same independent portable_pty WARN upper
bound while adding its existing Store tee; unmanaged modes keep the binary
owner's stderr path. No broader user filter can reopen private backend logging.
This merge requires its own combined source/runtime qualification; earlier
pre-helper checks do not prove it.

## Correction fixture timing and ownership

The correction's local transport, first-record and retained-peer cases use the
canonical testkit prebuild-before-clock outer bound. Named five-second phases
bound original accept, credential frame, handshake closure, fixture entry and
original checked exit separately inside the meaningful thirty-second case bound.
The ignored-SIGCHLD control waits the original std Child for ECHILD; the positive
case retains the actual registered SIGCHLD receiver, takes one known-child zombie
snapshot after its exit notification, then waits/reaps that same child. No sleep
or repeated proc scan orders this control. The self-reexec is isolated in an
owned outer process group, with a positive stdout ACK; output/handshake futures
remain scoped and are not detached on an assertion or deadline failure. This
source correction is unexecuted and does not qualify other fixture ancestry.

## Failed-start cleanup is observed by its existing owner

A failed proof previously dropped its exact Child and relied on Tokio's orphan
queue. That requested cleanup, but supplied no owner-observed completion. The
controller now keeps the original owned child outside the cancelable readiness
and authentication future. Its existing thirty-second start budget still covers
preflight plus proof; success adds no cleanup phase. A failed owned launch sends
SIGTERM through that retained handle and awaits the same checked wait within a
separate named five-second cleanup bound. No replacement RPC or numeric-PID signal
is used. Confirmed cleanup returns the unchanged original startup error.

An unconfirmed request, wait or cleanup deadline adds CleanupUnconfirmed around
the unchanged original start cause and concrete cleanup cause. The original
owned Child is dropped immediately before this error returns: its hard-kill
fallback was requested but its settlement is not claimed. No owner is leaked or
kept after this result. Cancellation drops the same retained owner as the final
fallback. A previously reaped child's first sweep is never retried by numeric
process-group ID.

The cfg(test)-only, task-scoped witness observes the SAME cleanup owner's checked
wait result. A separate field-drop witness runs after the original Child's drop
path; it is not OS-exit evidence. Neither witness adds a waiter, process owner,
public Proc/Child API or release-build state. The real fixture's installed
SIGTERM listener is acknowledged before malformed readiness; it cannot ignore
SIGKILL. Tests require original-error preservation, checked reap on the ordinary
case, and cleanup-deadline uncertainty plus immediate owner drop on the ignored
SIGTERM case. They are authored source, not executed proof.

The same-home integration regression can now audit one immediate proc absence
snapshot after confirmed cleanup, without polling for completion. Its exited-child
barrier uses an actual SIGCHLD event before the known-child zombie snapshot.
The same-home and wrong-peer causal cases now live under local-node's Linux
unit tests, so they consume the private cfg(test) settlement witness from the
actual cleanup owner. The former separate managed_local_peer integration target
is removed; normal managed_local_node public-API integration remains in node.
The shared fixture DTO is included from its single node-owned source, as approved;
only test wiring adds serde/serde_json/futures-util/RPC alongside testkit. Store
access stays in node_files: a private test-only synchronous call reuses its exact
identity reader while the causal fixture intentionally holds the sole blocking
worker. No second credential parser, database or Store consumer is introduced.
The testkit locator builds the exact feature-only node fixture before the case
clock through its established nested-build owner, errors on unavailable binaries,
and adds no normal crate edge. T's runner still owes its fixed prebuild step.
These moved tests and locator are unexecuted until changed-source checks run.
