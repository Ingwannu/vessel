# ADR 0044: Fleet controls and a connection's own authority

Status: proposed, 2026-11-13 (protocol 12, rows 2-4).

Context: ADR0012's join/leave CLI writes the node's data directory. A paired
native client cannot reach it. Existing fleet.members is Admin and names
member devices; Read must not become device administration. Scope sets are
independent, so a device cannot rely on having Read to learn its own scope.

Decision Log:
- Extend the current join/leave implementation at an Admin RPC boundary.
  Keep offline CLI behavior, with --node selecting the wire equivalent.
  Single-use invitation redemption must never be automatically retried.
- Serialize this node's RPC membership mutations with a non-queueing Tokio
  permit spanning preflight, network pairing and commit. A cross-process OS
  try-lock is held only inside blocking preflight and commit, never across a
  network await. Commit rechecks the role under that lock: a racing offline
  operation can win while pairing, but cannot be overwritten. A redeemed
  invitation followed by failed commit admission, writing or task completion
  is typed `join_uncertain`; inspect status before creating a fresh link,
  never automatically retry. Blocking work owns admission after RPC loss.
- Leave removes membership before its credential. Even when credential
  removal fails, wake the supervisor and reconcile the former home's device
  before returning the error. Restart sees standalone, and retrying leave
  cleans an orphan credential; materialized files remain. A commit's wakeup
  is inside blocking work, so cancellation cannot hide a completed change.
  The supervisor cancels and joins a stale member before reconciliation
  waits for its grant lock, and before starting a successor. Existing admitted
  apply finishes; an old loop cannot clear a successor's cached connection.
- Sync-now is accepted work, not a durability receipt. Coalesce requests
  through bounded notifications to the existing publisher/member loops;
  do not launch an independent apply or interrupt an already-started apply.
- Read gets role and local apply history, relative conflict paths and typed
  issue categories. No secret values/names or arbitrary file error text.
  Only a local owner or paired Admin additionally gets the existing home's
  member-device reports; delegated reads do not disclose those identities.
  Reads cap state JSON at 1 MiB and membership at 64 KiB before decoding,
  local conflict entries at 256, total reply conflicts at 512 (paths already
  at most 1,024 bytes), and payload at 1 MiB minus envelope reserve. Admin
  members use an indexed 128-device window with explicit `more`, refusing
  `status_too_large` if incomplete. No partial result is sent. The status path
  never clones the entire existing report map or lists all devices first.
- device.self is an authenticated self-inspection exception to Read. It
  returns only the paired caller's own id (otherwise absent) and effective
  scopes after existing clipping/intersection; it cannot choose another id,
  list devices, grant a scope or undo revocation's connection close.
- A public invitation without advertised direct or relay routes fails before
  a single-use secret is minted. Internal tracked fleet callbacks keep their
  explicitly supplied routes. A known advertised route is configuration,
  not a promise that a remote network will successfully connect.

Alternatives: invent another apply engine; return success only after a
potentially slow home round trip; expose all device rows as Read; make scope
inspection Admin or Read and leave restricted devices unable to ask. These
respectively add races, tie one RPC to slow I/O, widen identity visibility,
or preserve the screen's inability to decide its own available actions.

References: Paseo ConnectedHosts persists registrations with actor authority
and omits connection secrets from summaries; its server_info reports the
session's current permissions. Pairing explicitly requires a reachable route.
Fleet publication/revocation policy remains Vessel ADR0012/0010/0013.

Consequences: status may describe an apply before the most recent join; it is
explicit history with no current-home association. A client must not compare
its bare generation with the current home's and label it up to date. Leaving
keeps materialized files. Existing in-flight apply can finish. Unknown future
roles/problems are explicit variants; old peers refuse new calls before send
or dispatch. This adds no new scope and changes no minimum protocol version.
