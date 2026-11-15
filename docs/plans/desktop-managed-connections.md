# Managed-local admission in the canonical native registry

Status: contract only. Shared connections source waits until both W2's real
protocol caller and W12's stream promotion land, in the lead's assigned order.
No source/runtime/security approval is claimed. The base-6 merge is being
resolved; its legacy local-registration adapter is not a compiling handoff.

## Decision log

- Purpose: expose the authenticated desktop-managed home to existing host,
  device, project and onboarding consumers without a second registry.
- Constraints: one State.clients map, one State.admissions allocator; a book
  entry is persisted configuration and cannot be fabricated for local launch.
- Alternatives: a fake BookEntry, alias-only local exceptions, cached Connected
  reuse, another epoch counter. All are rejected.
- Choice: private Book/ManagedLocal binding in the existing Connected record,
  typed controller provenance, fresh canonical contexts and exact retirement.
- Consequence: local appearance/book editing stays unavailable; source/stream
  and process-ownership proofs remain independent.

## Operations and results

`admit_local` takes the controller's immutable authenticated transport/provenance,
its fixed expected NodeId and validated generated name. It checks actual welcome
identity and the current book before minting through State.admissions. Typed
refusals distinguish identity change, book/local collision, exhausted admission,
and book read failure. Any replaced local validity is retired before publication.
There is one named managed-local cap of one. No fake URL, BookEntry or PID appears.

A second distinct source is refused with `LocalOccupied`, retaining the active
source. Replacement is an explicit controller transition: retire/remove the old
source first, then attempt admission. The generated name is deterministic:
`desktop-<canonical NodeId display>` (the existing validated Name grammar).
The sole shared helper is `view::local_node::names::{generated_name,is_reserved}`.
E2 reserves only `desktop-` followed by the canonical 36-byte lowercase UUID
form (8-4-4-4-12 lowercase hexadecimal digits with hyphens) before any write.
`desktop-pc` and `desktop-home` remain valid book names. Matching is exact and
case-sensitive; uppercase UUID/prefix variants are ordinary distinct names.
Whitespace is rejected by the existing Name parser, never trimmed into a
reserved destination. The helper accepts validated Name so both callers apply
the same boundary.
An existing book record, including a legacy record matching that exact generated name, wins;
local admission reports a visible reserved-name collision rather than renaming
itself or modifying the book. A duplicate notification for the same opaque
transport source is idempotent and keeps its existing context.

Registration retains only the controller transport/provenance and its latest
admission receipt for exact removal, never Connected/validity or another epoch
allocator. A source clone is not a new physical connection; state/log changes
must not churn its context. A typed opaque controller-transport lease may identify
that immutable source, separately from canonical UI admission epochs and from
interruption confirmation. Reload retires old contexts first and re-admits the
same still-authenticated source with a fresh context/watch. Removal checks the
exact expected context and managed binding, so late removal cannot touch a
successor. The context is not process signal authority.

When an actual source replacement/removal is observed, retire its validity
BEFORE awaiting any book read, agent discovery or new admission, including when
the replacement later fails. This differs from duplicate same-source updates
and refused/failed ordinary book edits, which preserve validity. The actor owns
the latest receipt: reload retires first, allocates and stores the new receipt
in that same actor BEFORE publication. Removal carries the source lease, then
uses that actor's latest receipt; removal of an old source is ignored. The lease
correlates immutable controller transports, not epochs or signal authority.
Client closure, observed child exit or controller lease loss retires the local
context. No retained registration may automatically re-admit a transport after
its controller has withdrawn it.

Reconciliation preserves only the exact active local receipt when its name is
absent from the book. An observed book collision retires/refuses local admission
before book dial/publication; no shadow, fallback or re-add over a book record.
Local Fleet facts use the existing read_sessions reducer, including semantic
session-list refusal retaining authenticated node/context for usable device RPC.

Canonical Machine gets typed Unknown/Book/ManagedLocal provenance for Settings;
legacy snapshots default to Unknown and cannot establish Book edit authority. Before
any file commit, book edit/upsert rejects reserved local destinations and requires
an explicit book source. Rename/forget never move a managed client as a book
entry. Failed ordinary edits do not retire anything; successful ordinary book
edits retain the existing physical-client/agent/follow behavior. E2 owns the book
write integration. No local rename/color/badge persistence, invented book entry,
new preference store or appearance registry is introduced.

The E2 preflight/helper contract is a typed book-only source check plus a
reserved-destination check at its existing mutation owner, before dispatching
blocking I/O. A rename/upsert refused by this check leaves both admissions
intact. The reservation is the deterministic generated-name grammar, not a transient
preflight result: it therefore survives caller timeout/cancellation and the
actual blocking worker's later rename. The existing owner retains its operation
receipt until ACTUAL worker completion and preserves its preimage/epoch fences;
uncertain completion is not reported as a definitive failed edit. No new
reservation registry is introduced. Export serializes the book source only;
rename, forget, upsert and export never convert ManagedLocal into a file entry.

Onboarding Target wraps canonical ConnectionContext and keeps its existing Visit.
The controller's expected NodeId must equal Context.node. Same-node reconnect,
old context, old visit and stale before-poll outcomes are refused. Devices retain
the unchanged SelectedConnection call/pre/post fences. No raw getter or public
arbitrary-effect guard is added to SelectedConnection.
Queued result installation also requires W12's opaque actor-issued delivery
validity capability, exactly as book contexts do; ManagedLocal has no bypass.

This registry extension has admission/retirement/snapshot effects only. Desktop
process controls use the separately reviewed, app-owned Child/controller lease;
no host alias/context/PID creates that authority. Host overview will be provenance
and navigation only; process controls stay in desktop General settings and the
global quit/update path. If a host-context process action is later exposed, it
needs both exact context-before-effect and its actual typed controller lease.

## Planned paths and validation

Shared third-slot paths: link/src/connections.rs, connections/local.rs and tests,
connections/edits.rs, connections/snapshots.rs; view/src/connection or fleet Machine
provenance and corresponding constructors; the E2-owned book-write preflight.
Feature paths: link/src/local_node/{mod,actor,registration}.rs,
link/src/refresh.rs; view/src/onboarding/scope.rs; link/src/onboarding.rs;
app/src/areas/onboarding/scope.rs and local-node/host-overview projection.
README/this plan record the boundary; no crate edge, RPC or schema changes.

Barrier regressions must exercise local/book collision; successful and failed
book edits; reload and same-node reconnect; stale registration/removal; and
session-Forbidden with a usable device RPC. Scope tests reject old context/Visit
and before-poll retirement. Root process/launcher tests remain separate evidence;
prior G4/W2/W12 passes do not approve this future source.
Additional barriers cover distinct-second-source refusal; replacement retirement
before a held book read (including failed admission); duplicate-source stability;
reload receipt update followed by current versus stale source removal; controller
closure/lease loss with no automatic re-admission; Unknown snapshot provenance;
all four book persistence paths; and held I/O -> caller timeout -> local admission
attempt -> worker release. The last case must preserve the managed-name
reservation through actual completion without treating an uncertain commit as
definitively failed.
