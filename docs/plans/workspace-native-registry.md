# Native canonical workspace metadata projection

This is C1's shared metadata hook for sidebar, palette and pane consumers. Its
source is the existing registry dependency (ADR0082), not a new registry or a
path-derived WorkspaceId. It is isolated from the remaining Add Project/config
forms so their unready contracts cannot be mistaken for this reader's scope.
The root, manifests, lockfiles and protocol remain byte-identical to package
base 13. Node protocol 25 already supplies six complete metadata methods.

[Decision Log]
- Purpose: empty and same-directory workspaces retain independent identity,
  explicit membership and host/project grouping on every native surface.
- Constraint: same-node reconnects can retire a queued observation before Fleet
  installation. A context value alone does not make cached data current.
- Alternatives: node-only caching, cwd matching, copying ownership per consumer,
  or treating missing initial state as legacy unowned. Those lose authority.
- Choice: read through one captured SelectedConnection, retain only its canonical
  weak DeliveryGuard after installation, validate it on EVERY row/ownership/root
  query, and reuse the one LayoutKey/WorkspaceLookup API. A read ticket is only
  correlation. No guard constructor or shared staging attribute is introduced.
- Effects: loading, unsupported, offline and refused stay distinct. Only a
  guarded complete registry can yield authoritative None for an unbound legacy
  session. Root/location metadata does not prove physical continuity or grant I/O.

## Modules and data flow

View workspace::registry_projection owns bounded pure validation, read tickets
and borrowed WorkspaceRow facts over canonical RegistrySnapshot/ProjectList/
WorktreeList. ProjectPlacement is exactly (NodeId, ProjectId); keys are existing
LayoutKey::Workspace. Descriptor title/pin/lifecycle/visibility/gate and explicit
Session/Terminal associations remain canonical, including historical members.
Missing/unready catalogs or unsupported nested placement remain unavailable.

Link workspace::registry_reads captures the original admission once, then calls
GetRegistry and the two catalogs through it. Separate calls make no atomic
catalog/registry promise. A private response validates the weak guard after queue
wait; the App cache retains that same weak source and validates it again on every
query. The existing actual Link actor/framed peer supplies the guard tests. The
App does not manufacture guards or retain a second connection/epoch/client.

Four calls or queued complete observations share MAX_REGISTRY_READS. Responses
retain the slot through the receive callback, so queueing cannot free capacity
while keeping peer payloads. Each queued combined observation fits the same
16 MiB encoded budget as the whole retained cache. Before accounting, at most
four reads can retain three independently frame-bounded replies. Accounting
allocates no second serialized snapshot. At most 64 hosts are retained; explicit
book rename/forget frees that host's tickets and slot. Errors are not immediately
retried. The App rotates hosts so long polls do not precede all initial reads.

App areas/workspace/registry_cache is the sole guarded WorkspaceLookup adapter;
registry_runtime reconciles actual Fleet contexts and starts the finite reads.
Consumers use the original context for rows and actual selected-action routing;
node-based metadata source selection never selects a client or alternate route
for an action. A retired/missing source or an equal-revision conflict refuses.

## Reference behavior and exclusions

Paseo workspace-provisioning-service.ts:220–304 establishes independent identity;
workspace-directory.ts:365–407 aggregates all workspace-relative roots and actual
working descendants. workspace-agent-activity.ts:21–48 is DISPLAY-only latest
root selection. This hook supplies ownership/coverage only. Status must join the
canonical W7 attention and W2 relation/current-role producers; missing clock or
role facts remain Unknown, without Creator/activity/cwd fallbacks.

This packet does not create or archive workspaces, run setup, approve commands,
release trust, restore backing, assign generation, implement config I/O or editor
launch, or choose an automatic workspace name. It changes no control renderer,
user-facing words, files/terminal/proxy authority or schema. Remaining C1 forms
and source-editor state stay in their owning continuation.

## Verification boundary

Pure tests cover independent same-path identities, exact associations, startup,
stale/rebound/capacity/conflicting snapshots, catalog/root consistency and Windows
metadata. Link tests use actual Command/State/framed RPC/result/Connected retirement
and prove queued guard invalidation plus retained read capacity. App tests prove
startup/refusal/forget only. Whole native/current-source checks remain owed.

The shared cross-App real actor fixture is still held by its owner pending the
lead's adoption. No positive cross-App cache freshness, S2/pane/onboarding runtime,
visual parity or full C1 completion is claimed before that actual proof. The
portable packet changes only feature modules and registration lines on base 13;
shared selected/delivery/fixture attributes and their minters are untouched.


The reader's test/testing-feature receipt is an owner-local normal-return
acknowledgement, not a guard or fixture constructor. It is minted only by actual
read publication. The outer signalling owner completes after the awaited operation
future drops its selected connection and senders; panic/cancellation signals
Abandoned instead. A guarded response and a fallback have at most two one-slot
channels per admitted read. A discarded receiver never blocks task exit. Queued
response capacity remains independent through receive. Real actor tests await
this receipt outside Tokio workers, including retirement during an unanswered
registry request. The cross-App Scenario's Devices-only close is never treated
as proof that a workspace read settled. This additive proof still requires its
own exact checks and does not qualify a shared Scenario before lead adoption.
