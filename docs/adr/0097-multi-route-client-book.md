# ADR 0097: one typed client book for host identity and connections

Status: proposed, 2026-11-15. Security review by V3 is required **before**
the persisted format or credential commit path changes. This document is
not an implemented API, migration or approval to consume staged code.

## Context and reference

E1 owns the app's client-book API and host screens. G4 owns pairing and
connection admission, and consumes that API. W17 owns the node's host
metadata registry and CLI; it does not create another client registry.
A display label, CLI book alias, authenticated NodeId and physical route
are different values.

Paseo 0.15.4 references read for this decision:

- `packages/app/src/types/host-connection.ts`: `HostProfile`,
  `upsertHostConnectionInProfiles`, normalization and connection identity.
- `packages/app/src/runtime/host-runtime.ts`: direct/SSH probe and upsert,
  host appearance, remove-last-connection and `upsertHostConnection`.
- `packages/app/src/screens/settings/host-appearance-section.tsx` and
  `components/rename-modal.tsx`: the local label is independent of routes.

Paseo groups routes under a server identity, preserves custom appearance
and a usable preferred route, and removes the host when its last route
is removed. Its upsert also matches an endpoint independently of server
identity. Vessel must not transfer a pinned identity or credential on
that basis: a reused address is not proof of the same node.

Existing assets are `vessel-client::book::{BookFile, BookEntry}`,
`Pinned`, `SecureConnector`, typed addresses and credential bindings,
private atomic file replacement and `toml_edit` comment preservation.
G4 extends the existing `Connected` admission with a non-reused epoch;
E1 does not add a second connector or NodeId-to-first-alias resolver.
The current book has one transport per alias, although a secure transport
contains several direct/relay endpoints. Its documented read/edit/rename
window can lose a competing write. Native SSH is not yet connected by
the app merely because a typed SSH address exists.

## Alternatives

1. Another app-only host database beside the CLI book. Rejected: pairing,
   CLI edits and admission would have different sources of authority.
2. Match aliases, labels or endpoints and overwrite the matching entry.
   Rejected: labels are mutable, aliases can be ambiguous and endpoints
   can be reassigned. A matching welcome NodeId on bearer transport is
   also not cryptographic proof of a node key.
3. Extend the existing typed book and its one mutation path. Selected.
   Use the existing TOML and standard-library file locking assets;
   no new storage, transport or editing-engine dependency is needed.

## Proposed format and compatibility

The existing unversioned document is legacy format 1. Reading it never
rewrites it. Existing alias/default/pin edits keep its layout when a
format-2 feature is not needed. The first multi-route or appearance edit
writes `format = 2` and replaces only the affected entry. An older binary
rejects that unknown field rather than silently truncating routes.
New code reads both forms; mixed legacy entries in a format-2 document
remain valid. A route-list entry may not also declare legacy transport
fields. Unknown versions/fields and invalid references are refusals.

Illustrative format (placeholders, not usable credentials):

```toml
format = 2
default_node = "work"

[nodes.work]
id = "<NodeId>"
label = "Work machine"
color = "violet"
badge = "automatic"
next_route_id = 3
preferred_route = 1

[[nodes.work.routes]]
id = 1
kind = "secure_direct"
endpoint = "ws://machine.example:7077"
key = "<NodePublicKey>"
credential = { file = "<private credential file>" }

[[nodes.work.routes]]
id = 2
kind = "secure_relay"
endpoint = "wss://relay.example/vessel"
key = "<the same NodePublicKey>"
credential = { file = "<the same private credential file>" }
```

`BookEntry` owns a validated nonempty ordered `Connections` collection,
a `PreferredConnection` that references it, and a checked allocation
counter. `ConnectionId` is a nonzero monotonically allocated integer
newtype, scoped to one entry; removing a route does not reuse its id.
Overflow is a refusal. Legacy routes receive deterministic ids in their
existing order (secure direct first, relay second); the first conversion
persists those ids. No random id changes every time a file is read.
An admission always binds the complete route specification as well as
its id, so changing a hand-edited endpoint cannot inherit its authority.
Removing/recreating an alias also cannot reuse a connection epoch.

The payload is a closed enum: bearer websocket with its token-file
reference, SSH with its typed target/remote directory, secure direct,
or secure relay with its pinned key and credential-file reference.
Secrets stay in their current separate private files. Secure routes in
one profile must agree on NodeId, key and credential binding. Credential
validation happens before dialing, as in ADR 0010. Typed values are
shared at a deliberate lower-crate API boundary; app/view cannot import
the client I/O crate. The exact public identifier registration is part
of the implementation review, not an untyped string bridge.

Mixed authentication modes require an explicit selected route. Automatic
failover is restricted to the same security class and credential binding;
secure authentication refusal never falls through to a bearer or SSH
route. Legacy bearer/SSH keeps its existing trust model; the app does
not describe its welcome id as a cryptographic key proof. Adding socket,
pipe or forwarded routes requires its real transport owner first, not an
unchecked extra enum payload.

## Typed mutation and identity rules

The one client API accepts a typed mutation, explicit target and expected
entry preimage. Its result distinguishes unchanged, committed, conflict,
ambiguous target and local I/O failure. The link actor owns blocking I/O
on a dedicated blocking task and applies the result to the UI only after
a successful commit. No raw credential or filesystem error reaches view.

An upsert carries the node identity observed by the successful connection,
its security evidence and the exact connection specification. Secure
proof comes from the verified pinned channel; an arbitrary UI string is
not that proof. Direct/SSH retains the existing `Pinned` welcome-id check.
The API does not accept an unverified supplied NodeId as discovery.

- New host: an explicit unused parsed alias is required. Display label
  is the independent Unicode `Label`; its absence displays the NodeId.
- Existing host: match the authenticated NodeId and, for secure routes,
  the pinned key. Preserve alias, custom label/appearance, default and
  preferred connection when still present; append a genuinely new route.
- More than one existing alias with that identity: return `Ambiguous`
  unless the person supplied an explicit matching target. Never select
  the first alias in map iteration. Do not silently coalesce project keys.
- Same endpoint with a different identity, or same NodeId with a different
  pinned key: return a typed identity conflict. Replacement is a separate
  explicitly confirmed operation, never an upsert side effect.
- Legacy unpinned entry: bind it only through an explicitly selected
  target and its successful pinned connection; URL equality alone does
  not upgrade it. An unrelated alias may not be captured by discovery.
- Last route removal removes the local host/default selection. It does
  not stop/delete anything on the node or revoke an unrelated credential.

All ordinary failed book edits preserve selected machine, clients and
follows. Successful route/credential/identity changes retire only the
admission whose actual binding changed; a display-only edit does not
replace a physical connection. G4 owns the exact selected alias, epoch,
expected NodeId and live `NodeClient::node().id` preflight immediately
before effects. A request never resolves a NodeId by first matching alias.

## File transaction and credential outcomes

Every cooperating writer, including add/remove/rename/default/pin and the
CLI, uses one client-owned transaction path. A private sidecar file is
opened without truncation, `File::try_lock` admits one operation, the
book is reread under that lock, expected entry/preferred selection is
checked, the edited document is parsed again, and the existing private
atomic replacement commits it. Busy is typed and returned without a
poll/retry loop. Unlock is explicit on guard drop, matching the existing
fleet file-lock lesson about duplicated descriptors after fork.

No network request or invitation redemption runs while the file lock is
held. Cancellation cannot release a still-running blocking file operation
or claim that a completed rename was undone. Files remain owner-only
0600 and newly created directories 0700 on Unix; Windows keeps the user
profile ACL. No chmod of an existing shared parent is introduced.

Manual editors and older processes do not honor an advisory lock. Check
the reread preimage again before replacement and return a conflict when
it changed; this detects ordinary conflicts but is not a universal
compare-and-swap against a hostile/noncooperating writer. Document that
remaining race rather than claiming a filesystem transaction with it.

Pairing has an irreversible remote stage. G4 owns invitation consumption;
the book API must never redeem it or automatically replay it after a
local failure. The typed outcome records which stage actually happened:
not redeemed, redeemed with credential only in memory, credential saved
but book not committed, or committed. Only the latter is a saved host.

Prefer writing a newly issued credential to a fresh private file, leaving
an existing active credential untouched until the book points to the new
one. On book failure retain the issued credential and a link-owned local
retry handle; retry validates its NodeId/key and redoes only the local
commit. Never delete or restore an older credential that another window
may now use. If an existing integration has already replaced the active
credential file, report that partial outcome and retire its old admission
even though the book write failed. Such an outcome is not ordinary edit
failure. No success event or remote mutation is fabricated to hide it.
Durable recovery across app restart needs its own reviewed receipt design;
until that exists, say recovery is local to the current operation and
require explicit revoke/reissue after a lost receipt, never invitation
replay. A secret must not be discarded on the pretext that a book failed.

## Verification and limits

Before format code lands, V3 reviews schema, identity proof, downgrade
rules, credential partial outcomes and locking. Implementation tests must
cover legacy byte-preserving no-op/pin edits, stable route ids and
counter overflow, duplicate ids, unknown fields/version, missing preferred
route, same identity/new route, key/id conflict, duplicate-alias ambiguity,
explicit target preservation and failed-write unchanged state.

Barrier-driven real actor tests cover rebind/A→B→A, rename/forget and
late replies using G4's one admission. Process-level cooperating-writer
and cancellation tests exercise the actual lock/replacement boundary.
Credential tests inject each local failure after remote redemption and
prove redemption count stays one, correct admission retirement, no secret
canary in any event/Display/Debug/export, and private file/directory modes.
Named input/route/export bounds are required in their owning validated
types; their concrete limits and compatibility fixtures must be reviewed
before the parser accepts new format data.

Screens and typed fixtures alone do not establish transport support,
cryptographic authentication, multiwindow safety or a successful host
upsert. Existing source/gate/capture approvals retain their bounded scope.
