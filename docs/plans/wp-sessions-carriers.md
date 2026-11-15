# Session carrier conversion behind the sole resident owner

Status: preparation only. The frozen W12 single-slot amendment is not product
admission code. No live resident or connection safety is claimed here.

## Decision Log

- Purpose and intent: reject queued old metadata, records, EOF/failure and action
  replies after an attach or physical connection has been retired.
- Existing assets and limits: `view::residents::FollowBinding` identifies the
  canonical resource/attempt; G4 `ConnectionContext` and SelectedConnection own
  physical admission. Existing `session::stream::follow` emits only SessionId and
  `ActAnswer` uses alias+SessionId. Aborting the producer cannot retract an event
  already waiting in the bounded UI queue.
- Alternatives: compare only NodeId, mint another epoch, clone the resident table,
  or retain legacy globals on missing scoped access. Those cannot establish the
  original connection and resource attempt and are rejected.
- Choice: modify the existing carriers to retain both canonical identities and
  use the original selected lease for every converted effect. No dependency or
  identity allocator is added. W12 remains the only resident admission owner.
- Why: a same-node reconnect changes connection authority, while a repeated
  attach on that connection changes FollowBinding. Both must match at ingress.
- Benefits, costs and effects: conversion requires the concrete timeline/composer
  owners in the same product union; a typed carrier alone is insufficient.

## Owned interfaces and data flow

W12 admits the real layout claim, SessionContext and FollowBinding before W21
requests metadata or follows records. W21 retains that binding and the original
ConnectionContext together; their NodeId must match. Neither receipt can recreate
admission from an arriving response. SessionAddress remains the canonical resident
value already re-exported by `view::sessions`, not another address definition.

The link obtains G4 SelectedConnection from that exact context. Its call guard
checks actual welcome identity and irreversible validity before polling and after
awaiting. The staged SelectedFollow guard is enabled only with its real consumer;
it must remain the one attach/next implementation, without a raw-client escape.
After channel backpressure, each event still carries original context and binding,
plus G4's opaque delivery-validity capability from that same selected admission.
A context value or a later Fleet comparison alone cannot fence an event already
queued ahead of its DeviceRetired notice. The app must recheck the actual shared
delivery capability before W12 `receive` passes concrete resources to an owner
adapter. That capability remains G4-owned, with no raw client or second validity
cache. It is a required pending producer, not an API invented in this package.
An old event cannot refresh visibility or manufacture a new slot.

Lifecycle mutations from an admitted session additionally carry the canonical
OperationKind::Lifecycle and single-use OperationTicket. W12 settles admission
before close/retarget or draft release. Retirement converts an already-sent action
to uncertainty at its owner and keeps protected input/confirmation state; neither
retry nor late success may clear another draft. Refused replacement admission
cannot postpone independent old-transport retirement: dirty/IME resources remain,
while old follow/operation authority retires immediately and sent writes become
uncertain. Close callbacks retain LayoutKey
and ClosePlan, so reused TabId values in another workspace cannot be closed.

Sessions/history actions on an unopened resource capture the rendered Machine's
ConnectionContext and the history owner's existing correlation ticket. They do
not allocate a phantom resident or borrow selected-session state. They cannot
clear a resident composer or gain local tab-close authority from a cache update.
The selected lease still fences their effect and queued result. Deep-link lookup
may wait for an authenticated address under its one total deadline; after lookup,
opening must pass the same real resident admission before hydration or follow.

W21 converts stream/metadata/lifecycle carriers; W10 converts the concrete timeline
adapter; W12b and W6 convert composer/attachment owners. The sole table/resource
owner is W12. Legacy Records/SessionLost/Acted ingress cannot remain as a fallback
for an admitted resource. Old global storage/accessors disappear in the same union
for every converted path, including ordinary open, creation completion, terminal
history and deep-link/notification entry. Missing authority refuses locally.

## Evidence still required

Use actual bounded queues to retain an old event across same-node reattach and
A→B→A replacement with a reused SessionId. Test metadata, record, failure/EOF and
mutation replies independently, including a reply queued before retirement and a
duplicate completion. Refused switching must preserve actual dirty field entities,
IME/overlay holds and pending/uncertain forms. Hidden fresh records cannot mark
read or take focus. Inspect the converted product union for global/raw fallback;
passing pure resident tests or post-await checks does not establish these facts.

## Staged concrete lifecycle resource

The exact W12 staged getter source is adopted separately from its proposed live
single-slot admission. `areas::sessions::resident` is literal cfg(test). Its
LifecycleResources belongs in W12's one SessionResources bundle; W12 registers
that field only after this adapter is checked. It starts with Loading Hold, never
default Ready. Snapshot capture borrows only the exact canonical SessionContext
getter and checks retained Open binding, shown conversation and SessionInfo agree.
It copies typed domain configuration/options, never field entities or a Shell.
Missing metadata or another resource is a typed refusal without global fallback.

This owner retains separate UI decision and canonical operation tickets. An old
confirmation cannot settle a newer decision. Matching retirement keeps an unsent
confirmation as a revoked Overlay; a sent operation retains its exact original
binding/ticket/action as UncertainEffect. Fresh hydration cannot erase uncertainty.
Only a successful canonical single-use settle after delivery validity may call
the acknowledgement adapter. This does not itself perform RPC, clear input,
retarget/close tabs, or establish dirty/IME/form snapshot coverage for other owners.
The existing SessionView constructor remains isolated test setup, not scoped input.

The current package-base-6 projection merge retains G4's reconcile/read-refusal
policy and protects stale Fleet publication. It has not performed this carrier
conversion. Source review, changed-source tests, whole native gate and inspected
captures remain separate evidence.
