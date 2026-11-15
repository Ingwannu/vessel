# ADR 0097: one typed client book for host identity and connections

Status: contract accepted by V3, 2026-11-15, for the amendment at
`001d9673`. Original changed-source fast outputs at `9a173ca0` were
independently checked. This is a docs-contract verdict, not approval of
the prepared identifier implementation, persisted format, credential
commit path, migration or staged runtime. Those require their own scoped
checks and security review before handoff.

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
   no new storage, transport or editing-engine library is needed. The
   coordinator approved the client's cfg(unix) direct workspace rustix/fs
   dependency for this boundary on 2026-11-15. Architecture and ADR0001
   record that ownership; no client-to-store/fleet edge is added. The
   current file code is private test staging, with compilation, code review
   and production integration still owed.

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
last_route_id = 2
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
counter. `BookRouteId(NonZeroU32)` is the coordinator-approved shared value in
`vessel-proto`: client-minted and not on the wire. Its exact range is
1..=4,294,967,295. `last_route_id` retains the greatest allocated id even
after removal; `next()` returns the next id or explicit exhaustion. No
wrap, zero sentinel or reused id is allowed. A missing counter in a
route-list entry, zero, a counter below any allocated id, or an invalid
preferred id is refused. Both TOML i64 and JavaScript numbers represent
the entire range exactly; no numeric cast/string coercion is needed. Legacy routes receive deterministic ids in their
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
Evidence is a closed type distinguishing `WelcomeOnly { node }` from
`PinnedChannel { node, key }`, obtained from the successful connector;
a NodeId string cannot construct the latter. Association checks BOTH
the stored entry trust requirement and incoming evidence, not just the
incoming transport class. The following matrix is normative:

| Stored trust | Incoming evidence | Association permitted |
| --- | --- | --- |
| Secure NodeId+key | PinnedChannel with exact NodeId+key | Automatic merge only when the identity target is unique; otherwise explicit matching alias/preimage |
| Secure NodeId+key | WelcomeOnly claiming the same NodeId | No automatic match or append; typed stronger-proof-required refusal with no write/retirement |
| Welcome-only pinned NodeId | WelcomeOnly with exact NodeId | Unique identity or explicit matching alias/preimage, retaining welcome-only trust |
| Welcome-only pinned NodeId | PinnedChannel with exact NodeId | Explicit selected alias/preimage plus a distinct confirmed security migration; do not silently promote old weaker routes |
| Unpinned legacy entry | Either evidence class | Explicit alias/preimage and distinct confirmation; no endpoint-only capture |
| Any trust | Different NodeId or a different stored secure key | Identity conflict; separate explicit replacement flow only |

Adding a weaker route under an existing secure profile is outside ordinary
upsert. If subsequently supported, it requires a distinct human-confirmed
mixed-class association to an explicitly selected alias and current
preimage. That weaker route retains WelcomeOnly proof and cannot inherit
the stored key or participate in automatic secure failover. Until that
flow exists, refuse the association. A typed key observation is never
manufactured for an SSH/bearer route. Existing default/preferred/custom
appearance fields survive every permitted merge or migration.

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
local failure. The typed outcome records what is actually known:
`NotDispatched`, `RedemptionUncertain`, `IssuedInMemory`,
`CredentialSavedBookPending`, or `Committed`. Only the latter is a saved
host. `RedemptionUncertain` begins when dispatch may have reached the
node but no definitive response was received: the invitation may have
been consumed and a credential issued even if the reply was lost. It
is not a retryable pre-redemption failure. Close, timeout and cancellation
cannot turn that stage into NotDispatched or invent an issued credential.

Before dispatch G4 reserves one bounded operation record and retention
capacity, with a non-reused typed operation id owned by its existing link
hub rather than the requesting view. That owner outlives window/request
cancellation, retains the truthful outcome and handles a late response.
Exactly one dispatch is permitted. Uncertainty is reconciled through an
authorized device inventory/revocation or explicit revoke/reissue guidance;
never by repeating the invitation. Local retry is available only when the
issued credential/result is actually held. Before local commit the exact
selected entry/preimage and admission are checked again: a renamed,
forgotten or rebound target cannot receive a late commit.

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

V3 has reviewed this schema/identity/downgrade/partial-outcome/locking
contract. Implementation is not covered by that verdict. Its tests must
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
The concrete limits below are part of the proposed parser/worker contract;
boundary and one-over fixtures are required before it accepts new data.

Screens and typed fixtures alone do not establish transport support,
cryptographic authentication, multiwindow safety or a successful host
upsert. Existing source/gate/capture approvals retain their bounded scope.


## Concrete resource and safe-file contract (V3 amendment)

Compatibility inventory: the current book/URL/SSH/credential readers have
no general finite file/route count cap; NodeName already limits ASCII
aliases to 64 bytes. The new view Label validates source trimming/blankness
but currently has no byte cap. A scan of public client Rust literals
that parse as TOML with nodes found 24 samples: maximum 246 input bytes,
2 entries, 2 expanded routes and 36 UTF-8 bytes in a field. This is a
limited fixture inventory, not a survey of real users or validity of each
transport. Syntax-invalid fixtures were excluded. No private book was
read. Paseo's stored host schema also lacks a general host/route count
cap; these are explicit Vessel safety limits, not claimed Paseo defaults.

| Named bound | Exact limit and units | Reason/compatibility |
| --- | --- | --- |
| MAX_BOOK_INPUT_BYTES | 1,048,576 UTF-8 bytes | Ample space beyond published fixtures and ordinary host lists, including retained comments; stop before read-to-string/TOML allocation |
| MAX_BOOK_OUTPUT_BYTES | 1,048,576 UTF-8 bytes | Includes all rewritten text/comments; bound before allocating the final output buffer and before replacement, never truncate comments |
| MAX_BOOK_EXPORT_BYTES | 131,072 UTF-8 bytes | Safe allowlisted UI export only, no credentials/paths/raw document; same bounded report scale as diagnostics |
| MAX_BOOK_ENTRIES | 1,024 | Desktop host list budget, far above public 2-entry example; over-limit import is refused, never partially dropped |
| MAX_ROUTES_PER_ENTRY | 64 | Supports several networks/relays without unbounded probing or UI rows |
| MAX_BOOK_ROUTES | 8,192 | Also checked globally, including expanded legacy direct/relay routes; per-entry and byte limits apply simultaneously |
| MAX_BOOK_ADDRESS_BYTES | 2,048 UTF-8 bytes | Canonical URL/SSH authority or relay endpoint, checked both before and after normalization; not Unicode characters |
| MAX_BOOK_PATH_BYTES | 4,096 UTF-8 bytes | Token/credential/remote-directory path, independent of platform native path representation; non-UTF-8 persisted strings refused |
| MAX_HOST_LABEL_BYTES | 4,096 UTF-8 bytes | Additional storage bound over Label's source trim/blank policy, preserving ordinary Unicode names; never a 4,096-character claim |
| MAX_CREDENTIAL_FILE_BYTES | 4,096 UTF-8 bytes | Fixed id/key/device/secret schema needs much less; bounded read/serialization, no raw echo |
| MAX_BOOK_IO_WORKERS | 2 active, zero queued | Shared owner for cooperating native/CLI transaction API; each process has the same finite local admission, OS lock coordinates processes |
| MAX_ACTIVE_BOOK_BUFFER_BYTES | 4,194,304 aggregate | Two workers, each at most one 1 MiB encoded preimage and one 1 MiB output; no duplicated full-document snapshots in retry handles |
| MAX_RETAINED_BOOK_OPERATIONS | 8 per link owner | Includes dispatch-uncertain and issued-but-uncommitted pairing, not only visible dialogs |
| MAX_BOOK_RETRY_RECORD_BYTES | 65,536 per operation | Bounded canonical candidate mutation plus issued credential/result, not an entire book |
| MAX_RETAINED_BOOK_RETRY_BYTES | 524,288 aggregate | Reserve count and bytes before irreversible redemption; no eviction of unresolved credentials to admit another operation |
| BOOK_IO_WAIT | 5 seconds | Normal caller wait, not a promise to interrupt a blocking syscall |

Appearance color/badge are closed enums (their exact serialized words
only); alias uses the existing 64-byte NodeName validation; pinned NodeId
and key use their fixed typed encodings. Every route-list id is a
BookRouteId, unique within its entry. The allocation counter covers all
used ids, and preferred must exist. Expansion of legacy routes observes
all count/field/byte limits before creating the validated collection.
Over-limit legacy input is a typed refusal with the original left intact,
not a read-time rewrite or silently removed route. No scalar silently
changes from an integer to floating point/string to extend its range.

Input uses a finite byte reader that admits at most input-limit+1 bytes
before UTF-8 decode/TOML construction. Serialization writes into a capped
sink, counting comments as well as values; checking an already unbounded
String after allocation is insufficient. Parsed allocations are also
bounded by the encoded input and fixed schema/count limits, but the 4 MiB
buffer budget describes retained encoded documents, not all TOML heap
allocation or a measured process memory maximum. Candidate retry records
are encoded against their smaller bound before dispatch, and full books
are released after the worker finishes.

New admission refuses Busy when either the worker or retained-operation
capacity is full; it does not spawn waiting workers or retry on a timer.
The operation owns its permits through actual read/write/unlock completion,
even when its requesting future/window is gone. A wait deadline yields
TimedOut only if no mutation was dispatched and the worker is prevented
from committing; if commit may already be running, retain a typed
CommitUncertain operation and resolve it once the worker actually answers.
That is distinct from RedemptionUncertain, which is G4's remote boundary.
No timeout admits a replacement worker against still-held capacity. Both
byte and count reservations precede irreversible dispatch; failure to
reserve is Busy/TooLarge with no redemption. Issued credentials are never
evicted on an age timer. Explicit successful commit/reconciliation/revoke
resolves a record; an explicit acknowledged loss is a separate terminal
outcome, never automatic replay. Durable restart recovery remains pending.

The client owns one private safe-file module for its book, sidecar and
new credential targets. Existing plain read_to_string/OpenOptions paths
are not sufficient proof of this contract. Reuse the existing safe
rustix openat pattern (already used by fleet/store) through the client's
own file boundary, without a client-to-store dependency or unsafe code.
Unix uses a trusted opened config directory, directory-relative
NOFOLLOW/CLOEXEC and NONBLOCK final opens, then verifies a regular owned
file from the opened handle before reading/locking. Sidecar opens do not
truncate, including before acquiring the lock. New credential files are
create-new with random private names and no replacement of existing files.
Atomic replacement is relative to the same verified directory handle.
Reject symlinks, FIFOs, devices and other special targets before reading;
NONBLOCK prevents a FIFO open from consuming an indefinite worker first.
Windows must open the reparse point itself using the safe platform
OpenOptions backend, reject reparse/special targets from the opened handle
and preserve the profile ACL; a pre-open metadata check alone is not
claimed race-free. The implementation must demonstrate this backend or
refuse that operation on that platform explicitly. No speculative success
path is accepted as Windows support.

Required added regressions: secure-existing/bearer-incoming spoofed NodeId,
reverse-direction explicit migration, duplicate aliases, default/preferred
preservation and no-write/no-retirement refusal; lost response after remote
consumption, cancellation before/after receipt and close before persistence
(exactly one redemption, truthful terminal stage, no late commit after
rename/rebind); FIFO/symlink/special targets and a controlled held-I/O
worker whose caller cancels but permit remains held; every byte/count/counter
boundary and one over; complete redacted error/source chains with a canary.
The published lock/writer, credential and typed-id implementations still
need their actual narrow tests and security review; this ADR is not them.
