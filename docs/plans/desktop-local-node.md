# Desktop-owned local node lifecycle

## Purpose and reference

Paseo 0.15.4 `packages/desktop/src/daemon/daemon-manager.ts` owns a launched
child separately from an already-running daemon; its stop/restart/quit flows
must not transfer ownership from a PID record. `daemon/quit-lifecycle.ts` closes
transports before stopping an owned daemon and installing an app update.
The desktop survey documents the thirty-second startup deadline, default
management enabled, default keep-running-after-quit disabled, and seven stop
reasons. Service files and arbitrary process signalling remain outside this package.

## Existing assets and decision log

- Purpose: one reusable controller for the desktop app's own child, shared with
  local CLI orchestration without importing node runtime/session machinery.
- Constraints: `vessel-proc::Child` already owns group/job cleanup, `DataDir`
  already owns private files and OS locks, `NodeClient` already authenticates
  local connections and implements `node.stop`. Proc currently kills on drop,
  so it cannot implement an explicit keep-running-after-quit choice.
- Alternatives: a bare Tokio child (loses group/job ownership); trusting an
  owner PID (does not grant ownership); starting a node inside the app (pulls
  runtime and storage into the UI); another wire method (the app launches locally).
- Choice: `vessel-local-node` depends on client/proto/store/proc and the already
  present Tokio/thiserror. Proc's consuming `Child::detach` returns an opaque,
  observation-only `Detached` handle, preserving the live child and group/job.
- Why: owned child handles are the sole launch/cleanup authority. Reused nodes
  remain Attached; a socket or owner record never becomes an Owned handle.
- Costs: the launching runtime must remain alive for the detached child reaper
  until process exit. Once the launcher exits the OS reparents the child. A
  detached handle never offers signalling or conversion back into an owned child.

## Boundaries and safety contract

Only `vessel-local-node/src/launch/` names Proc or calls detach. Only its
`node_files` module uses the Store facade (DataDir/StartLock/NodeLock); it never
opens SQLite. Readiness requires a bounded dedicated machine-readable child
notification followed by local-token authentication, intended NodeId, and
Serving state. A human log line is never a readiness contract. Neither a
successful handshake nor an owner-file PID is proof that the app launched it.
All startup/stop/wait operations have named deadlines. An unowned node is never
signalled, restarted or stopped on quit. An owned child's cleanup is by its
existing Proc handle. Stop acknowledgement and connection closure are distinct
from the owned child's awaited exit. Restart requires exit before another launch.

The node binary will emit the dedicated readiness record only in explicitly
requested desktop-managed launch mode, on its inherited stdout pipe; no credential
appears there, in argv, logs or errors. The local token remains an owner-only file.
All library paths and binaries are supplied explicitly; no HOME/PATH/environment
lookup in the library. No new RPC, schema or protocol version is introduced.

## Detach implementation conditions

Proc preserves default kill-on-drop for every ordinary Child. Windows job
creation must permit release only through the consuming detach operation; the
ordinary wrapper still explicitly terminates its owned job. A Detached retains
an unreaped direct child handle and private launch/group identity. Its wait reaps
only that direct child, never sweeps or signals its descendants. Dropping it while
the original runtime runs transfers the direct child to an explicit background
reaper. The caller keeps that runtime until application exit; runtime shutdown
is not a substitute for waiting when the launcher remains alive.

Before returning any spawn error after a process exists, Proc retains cleanup
ownership and terminates/reaps its child. There is no interval in which a normal
child escapes because detachable Windows jobs do not use kill-on-close.

## Tests owed before handover

- Detached survives handle drop and retains its process group; its reaper/wait
  observes and reaps the direct child; ordinary Child drop still kills its group.
- Fake managed child: ready/authentication identity/state mismatch, bounded
  malformed notification, timeout/cancellation cleanup, attached versus owned,
  repeated quit, live-session protection, verified stop then restart, keep on quit.
- Private isolated directories and fake programs only. Real agent/service-manager
  flows and macOS/Windows behaviour are separate from Linux fixture evidence.

### Release capability and Windows crash policy

The safe process wrapper fixes kill-on-close at job creation and cannot clear it
later. Standard `spawn` therefore retains its bound Windows job policy. Only
`spawn_releasable` creates the explicitly releasable job; its still-owned Child
has the same explicit drop cleanup. `Child::detach` rejects a bound launch and
returns that Child in a typed refusal, preserving authority. Only a releasable
launch can consume authority into Detached. This capability split avoids
weakening ordinary children on launcher crash and avoids unsafe job mutation.
The local-node launch module is the only consumer of both the releasable spawn
and detach capabilities. Its launcher runtime remains until application exit.
Shared bounded readiness is owned once by `vessel-proto::local_launch`; node main
emits it for `--desktop-managed`. There is no node-to-launcher dependency.


Startup diagnostics use an observation-only callback issued immediately after
owned spawn and before awaiting readiness. It retains only the bounded stderr
reader, so cancellation/malformed readiness still drops and cleans up the owned
child while the UI can explicitly read the failure's log tail. Error values do
not retain or print log text. Reused nodes have no captured owned stderr source.

The launcher-exit fixture starts a separate test executable and waits for that
launcher's actual exit before probing the synthetic child over parent-owned
loopback IPC. It covers detached survival, ordinary bound drop and releasable
still-owned drop on the same source path on Unix and Windows. EOF/x cleans up the
fixture without numeric PID control. A Linux run only proves Linux; Windows job
survival remains unverified until that same case executes on Windows.
