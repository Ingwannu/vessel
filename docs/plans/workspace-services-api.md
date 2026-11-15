# Workspace scripts and services consumer API

Status: staged DTOs, method rows and typed SDK are authored; generated wire
and checks are pending. No runtime handler or active service API is supplied.
ADR 0066 owns the project-file and service security policy. This package
does not qualify the staged Preview/backend primitives as a working engine.

## Decision

[Decision Log]

- Purpose: native consumers need one typed workspace script/service view,
  environment-plan view and administrative publication interface. Private
  helper functions and legacy root-based script calls are not that API.
- Existing assets: WorkspaceId, ScriptName, ScriptEntry, ServiceEnvName,
  ServicePort, CommandsDigest, TerminalInfo/ServiceState, the canonical
  registry, the method table and NodeClient::call. Reuse these owners.
- Constraints: ScriptPlace is a legacy project/session placement, not an
  independent workspace identity. A port plan, health and a proxy URL are
  observations/metadata, never process/socket ownership or Preview authority.
- Alternatives: extending protocol-eleven calls would change old request
  semantics; an app-local DTO would duplicate the node contract; arbitrary
  environment maps could expose inherited credentials. Reject all three.
- Choice: new workspace-scoped values in proto::workspace_services, staged
  method rows, typed client wrappers, then the qualified node actor/handlers.
  Preserve the legacy API independently; never infer WorkspaceId from cwd.
- Cost: the staged API is deliberately Unsupported. Native fixtures can
  compile against it, but runtime controls stay unavailable until wiring,
  ownership proofs, complete display and the package gate are qualified.

## Reference read

Paseo sources, read rather than copied:

- protocol/messages.ts3069–3096,4693–4726: workspace script list/start/stop
  requests, a workspace/script target and projected status responses.
- protocol/messages.ts4070–4084: name/type/hostname, nullable port and URLs,
  running/stopped lifecycle, nullable health, exit code and terminal id.
- server/script-status-projection.ts61–76,100–259: pending health becomes
  null; configured entries remain while stopped; running removed entries
  remain; stopped removed entries disappear; locale/numeric name ordering.
- server/workspace-script-runtime-store.ts1–76,98–104: one latest entry per
  workspace/name, independent of directory equality.
- server/worktree-bootstrap.ts412–481: plan all configured services, refresh
  a selected stopped service, construct peer environment, register its route.
- server/workspace-service-port-registry.ts55–92,113–156,160–284: cached
  plans, explicit uniqueness, global dynamic reservations, rollback/release.
- server/workspace-service-env.ts18–77,100–126: normalized peer names,
  HOST and own/peer port/URL values; URL omission when unavailable.
- server/script-health-monitor.ts33–53,89–160,209–227: periodic TCP probes,
  observation grace, consecutive failure policy and changed-state publication.
- server/service-proxy.ts124–235,403–469: hostname/URL projection, public
  preference and collision refusal. Registered public routes are anonymous
  before daemon auth (bootstrap.ts774–789; service-proxy.ts758–798).
- server/authorization/operation-permissions.ts220–222: read versus write.

Vessel differences, with reasons: typed bounded refusals instead of free-form
status strings; explicit pending/unavailable instead of null/healthy guesses;
existing supervised terminal states rather than only running/stopped; stable
bytewise ScriptName order on paged transport, avoiding locale-dependent
cursors (a view may sort the collected rows for presentation); Read never
plans or executes a helper; the existing approval/automation/start-lease
floor; loopback-controlled listener and HOST independent of the node bind;
and explicit per-service Admin consent for anonymous public publication.
These are not claims that Paseo already has this permission/lease model.

## Feature and method table

One `WORKSPACE_SERVICES: Option<ProtocolVersion> = None`. No number is
assigned by this package. Its doc names complete native status/env/public
controls, approval display in every client, the qualified node/runtime and
the canonical registry/connection join as assignment prerequisites.
PROJECT_FILE_KEYS remains independently unassigned until its existing full
native/web display condition is met; this API cannot bypass that condition.

Append rows at the END of rpc/table.rs, with `(staged WORKSPACE_SERVICES)`
and unary shape. The rows below exist only as staged values with default Unsupported,
not active handlers. They qualify the feature as
`$crate::workspace_services::WORKSPACE_SERVICES` and DTO paths from `$crate`.
Staged Handler defaults answer Unsupported and require the constant
to stay None. Add exactly each new Method variant to workspace/policy.rs's
refused deny arm, with no other node service edit in the values package.

| Variant / client wrapper | Wire name | Scope | Forwardable | Request -> response |
| --- | --- | --- | --- | --- |
| ListWorkspaceScripts / workspace_scripts | workspace.script.list | Read | false | WorkspaceScriptQuery -> WorkspaceScriptQueryResult |
| StartWorkspaceScript / start_workspace_script | workspace.script.start | Operate | false | StartWorkspaceScript -> WorkspaceScriptOutcome |
| StopWorkspaceScript / stop_workspace_script | workspace.script.stop | Operate | false | StopWorkspaceScript -> WorkspaceScriptOutcome |
| RestartWorkspaceScript / restart_workspace_script | workspace.script.restart | Operate | false | RestartWorkspaceScript -> WorkspaceScriptOutcome |
| GetWorkspaceServiceEnv / workspace_service_env | workspace.service.env | Read | false | GetWorkspaceServiceEnv -> ServiceEnvPlan |
| GetServiceSettings / service_settings | service.settings | Admin | false | GetServiceSettings -> ServicesAdminSettings |
| SetServiceGateway / set_service_gateway | service.set_gateway | Admin | false | SetServiceGateway -> ServiceSettingsOutcome |
| SetServiceExposure / set_service_exposure | service.set_exposure | Admin | false | SetServiceExposure -> ServiceSettingsOutcome |

Client wrappers live in a separate client module and delegate to the existing
typed call path. No second method/number/dispatch table, raw call string or
transport is introduced. Test direct admission before sending, forwarded
inner refusal before effects, current/older peers, method scope/shape,
default staged Unsupported and every new policy deny line. Keep existing
family cases unchanged; append new names where the head's family test
requires them. Only the real handler commit changes staged to pending.

### Exact principal and delegation admission

Every row above is nonforwardable, including both Read calls. Add its
variant to the false arm of the existing Method::forwardable match;
no second forwarding list or string-based routing is introduced. The
current contract grants no fleet.forward, fleet.relay or fleet.deliver
path. The staged Unsupported default is not the forwarding policy.
This conservative staging choice does not inherit the legacy script
calls' forwarding permission. Broader forwarding requires a separate
coordinator decision and security review before the real handler changes it.

For all five workspace-scoped Read/Operate methods, the intended real
handler accepts only a directly authenticated Principal::Owner or
Principal::Device with the row's scope granted by THIS node. It must still
validate the canonical local WorkspaceId and the runtime admission rules.
A foreign workspace id or caller-provided directory cannot select/create
local ownership. Principal::Agent and Principal::Delegated always refuse,
including a delegated origin claiming Owner or a forged Admin scope.

The three dedicated settings methods use the stricter same direct rule:
Principal::Owner or a directly authenticated Principal::Device with Admin,
for BOTH settings Read and writes. A device with only Read/Operate is
refused. Every Principal::Agent and Principal::Delegated is refused even
if its scope vector contains Admin; an origin's Owner label is not the
connection's Owner principal. No transport address or peer node label
substitutes for this node's authenticated principal and grant. A paired
device directly authenticated to this node is a direct Device; a call
relayed on behalf of another node remains Delegated and is refused.

The real handler must enforce this direct-principal check independently
of the envelope's nonforwardable check, before settings lookup, registry
lookup, snapshot construction, helper work, reservation, persistence or
any network/process effect. Values-only staging adds only the required
policy deny lines; it does not implement or claim this future admission.
While the feature is None, the shared protocol/staged refusal remains
in force even for otherwise eligible direct callers.

Tests pin all eight forwardable=false choices and existing forwardable
entries unchanged. Crafted Forward/Relay/Deliver envelopes must refuse
before the handler/effect spy, including owner-labelled origins, foreign
nodes, agent origins and forged Admin scopes. Direct handler admission
tests cover Owner, direct Device with/without the exact scope, Agent with
all scopes, and Delegated owner/device/agent origins with all scopes.
Use retained lookup/write/reservation/spawn/dial counters to prove every
refusal precedes effects; no private setting may be observed by a rejected
Read. Staged tests pin pre-send refusal and default Unsupported separately
from the real handler's later direct-principal tests.

## Snapshot and operation values

- `WorkspaceScriptRef { workspace: WorkspaceId, script: ScriptName }` is a
  query target, not a capability. Identity is exactly this pair.
- `ServiceSnapshotId` is an opaque UUID metadata etag, not a PID, actual-run
  token or workspace incarnation. A node restart creates new observations.
  Replacing an actual ProcessLifetime changes the etag even if visible
  terminal fields coincide. Ordinary activity does not grant permission.
- `WorkspaceScriptQuery` owns workspace plus a query enum: Initial,
  After(snapshot), or Page(cursor). After is the existing bounded terminal
  long-poll budget; Page never waits and requires the same snapshot.
- `ServiceCursor` carries snapshot id and the last ScriptName. It cannot
  select a foreign workspace. A changed/missing snapshot is a typed stale
  response, never a partial combination of two observations.
- `WorkspaceScriptQueryResult` is Snapshot(page), NotChanged(snapshot),
  Unavailable(reason), Stale(current), or Unknown. No-source is not an empty
  successful script list. `ServicePage` privately bounds/validates entries,
  unique names, order, workspace association and continuation consistency.
- `ServicePage` carries workspace, snapshot id, configuration summary,
  bounded rows and optional next cursor. Configuration is None, Current
  (CommandsDigest and approved/unapproved), Invalid, or Unavailable. Detailed
  approval still uses the existing ProjectCommands owner and projection.
- A row owns a validated ScriptEntry and Configured/RemovedRunning origin.
  It optionally references the existing TerminalInfo observation; its old
  placement is not membership authority. Only a captured approved definition
  can describe a removed running row. Never reconstruct one from cwd or a
  guessed command. Stopped removed rows are excluded, as in the reference.
- Plain scripts have no service observation. A service observation contains
  port-plan state, health and proxy state. Constructors/serde refuse a plain
  script with service-only fields rather than silently dropping those fields.
  Adding an approved entry field still breaks exhaustive display projections.
- Port plan is Unplanned, Reserved(port, Explicit/Helper/Range/Ephemeral),
  or Unavailable. A reservation/declared port is never an observed listener.
- Health is NotObserved, Pending, Healthy, Unhealthy, Unavailable or Unknown.
  Reuse the existing health clock and named scan/grace/failure budgets.
  TCP success proves neither HTTP scheme nor private Preview authorization.
- Start/restart requests include the displayed snapshot and definition digest;
  stop includes the displayed snapshot. These are optimistic fences only.
  The server rechecks canonical ownership/current approval before effects.
- `WorkspaceScriptOutcome` is Accepted(current snapshot), Already(current),
  NeedsApproval(ProjectCommands), Refused(typed reason), or Unknown. Accepted
  means the tracked operation was admitted, not a healthy or listening service.
  Stopping/revoking remains visible until actual owned cleanup/drain settles.
  A lost response is reconciled by observation, never a blind start retry.

Record fields are pinned for the values package (enum tags use `kind`):

| Record | Fields |
| --- | --- |
| WorkspaceScriptRef | workspace, script |
| ServiceCursor | snapshot, after (ScriptName) |
| ServicePage | workspace, snapshot, configuration, rows, next |
| WorkspaceScriptRow | definition (ScriptEntry), origin, terminal (optional TerminalInfo), service (optional ServiceObservation) |
| ServiceObservation | port_plan, health, proxy |
| ServiceProxyUrls | local, public (optional ServiceLink values); preferred is a derived getter, not a third stored fact |
| ServiceLink | access (NodeLoopback/AnonymousPublic), url (ServiceProxyUrl) |
| ReadyServiceEnv | workspace, snapshot, script, host, port, url (optional), peers |
| ServiceEnvPeer | script, env_name, port, url (optional) |
| ServicesAdminSettings | revision (optional ServicesSettingsRevision), gateway, exposures |
| ServiceExposureView | target (WorkspaceScriptRef), choice, effective |
| SetServiceGateway | expected (optional ServicesSettingsRevision), gateway |
| SetServiceExposure | target, expected (optional ServicesSettingsRevision), change |

A query carries workspace and Initial, After { snapshot }, or Page { cursor }.
Start/restart carry target, snapshot and definition digest; stop carries
only target and snapshot. Accepted/Already replies carry workspace, snapshot
and row; NeedsApproval carries workspace and commands; Refused carries
workspace and a typed reason. An outcome never claims OS cleanup from an ack.

Proxy Planned/Registered carry ServiceProxyUrls; Unavailable carries its
reason. Public exposure change is Private or AnonymousPublic { snapshot,
definition, acknowledgement }; acknowledgement has one explicit
ReachableWithoutVesselAuthentication choice, never an implicit boolean.
Effective publication is Private, Inactive(reason), Registered(url),
Revoking, or Unknown. Settings outcomes distinguish Unchanged(settings),
Saved(settings, Applying/Applied/Unavailable), Refused(reason), and Unknown.

Unavailable reasons distinguish RegistryUnavailable, RuntimeUnavailable,
AutomationBlocked, ApprovalRequired, InvalidConfiguration, PlanUnavailable,
OwnershipUnavailable, ObservationExpired, SchemeUnavailable,
ListenerUnavailable, IncarnationUnavailable, SettingsUnavailable,
WorkspaceInactive and Unknown. Mutation refusals distinguish stale snapshot
or settings, missing script, wrong kind, inactive/blocked workspace,
configuration/approval refusal, Busy and typed Unavailable. Future unknown
control choices refuse; observations remain non-authoritative data.

The new methods' responses and any nested NeedsApproval go through existing
peer projection. Test every direct, nested and forwarded approval path;
do not expose a digest/approved field to a peer that cannot display it.
Unknown observations do not enable any operation; unknown mutation choices
refuse. No ProcessLifetime, PID or kernel proof is serialized by these DTOs.

## Retained snapshots and waiting reads

Pagination does not create a retained snapshot archive. The runtime keeps
at most the latest materialized snapshot per canonical workspace and no
per-cursor copies. A replacement makes an old cursor Stale; a missing or
expired current materialization is Unavailable/Stale, never reconstructed
as though it were the old observation. A waiter retains only its etag and
bounded watch state, not a page or prior row collection.

The retained workspace count uses the qualified registry owner's
vessel_store::registry::MAX_WORKSPACES; current configured row counts use
MAX_PROJECT_COMMANDS, with removed-running rows bounded by the existing
node terminal MAX_LIVE owner. These qualified dependencies are prerequisites
for wiring, not constants copied into proto. If the owners are absent,
the real handler stays Unavailable. Count caps do not replace a byte cap.

The runtime's node-wide retained snapshot byte pool uses
MAX_SERVICE_CACHE_BYTES from Bound ownership. It accounts for all owned
rows, definitions, terminal metadata, generated environment peers and URL
text, including any snapshot allocation still retained by an asynchronous
operation after replacement. Charge capacity before allocating/retaining;
release only when the actual owner is dropped, not when its caller times
out. Over-budget data produces a typed unavailable/busy result without
truncation. An old snapshot cannot remain current when new data failed
admission. Idle materializations expire after the existing terminals
MAX_WAIT_SECS duration, refreshed only by an actual complete new observation.
No cursor access or caller activity extends an old observation's lifetime.

After queries use the existing service Connection wait accounting:
NodeService::wait_slot and Waits/WaitSlot from node/waits.rs, including their
PER_CONNECTION and PER_ORIGIN owners. They additionally take a node-wide
MAX_SERVICE_WAITERS permit. Past either share, return the current snapshot
or NotChanged immediately without waiting, as the existing wait mechanism
does. Page/Initial never hold a long-poll slot. The existing terminal
MAX_WAIT_SECS is the deadline; transport close, workspace invalidation and
node shutdown also end the wait. The actual wait task holds both shares
until it settles; dropping only the caller must not release detached work's
capacity. No alternate waiter registry or per-origin counter is introduced.

Tests must pin no history retained for stale cursors, byte/count cap
refusal, replacement/eviction invalidation, cancellation and close cleanup,
expiry without activity renewal, node-wide waiter exhaustion across many
connections/origins, existing share exhaustion, and release only after
actual ownership ends. DTO/page/RPC frame limits alone are not evidence
of these runtime bounds; this remains an explicit real-handler gate.

## Environment and URL values

`GetWorkspaceServiceEnv` targets an exact service and the displayed snapshot.
It returns NotPrepared, Ready(plan), Unavailable(reason) or Unknown. This is
READ OF AN ALREADY COMMITTED PLAN: no allocation, reservation, file command,
helper spawn, refresh, process start, health probe or public enable occurs.
Start's guarded planner owns those effects, including the not-yet-started
peer services, rollback and release. No node-wide helper/range is added.

Ready contains typed loopback bind host, own ServicePort, optional own proxy
URL and bounded peers (ScriptName, ServiceEnvName, ServicePort, optional URL).
It contains only generated service variables, never inherited environment,
credentials or an arbitrary string map. Runtime injection is HOST plus
VESSEL_PORT/VESSEL_URL and VESSEL_SERVICE_<NAME>_PORT/_URL. Omit unavailable
URL variables. Reuse the existing normalization/collision owner once.
HOST defaults to loopback even when the node listens on wildcard: unlike
Paseo's daemon-bind inference, an RPC bind is not permission to expose a
service. An approved program may ignore HOST; this is not an OS sandbox.

`ServiceProxyUrl` is bounded http/https origin metadata: host, nonzero port
when explicit, no credentials, query, fragment, control/whitespace or path
other than root. Parsing errors never retain/format the submitted URL.
Node construction checks trusted service/control origin separation.
`ServiceProxyState` distinguishes Unavailable, Planned and Registered;
projection is not registration/reachability proof. Each link distinguishes
NodeLoopback from AnonymousPublic. Local URLs use the actual dedicated
service listener's port, not the node-control port; absent listener means
no URL. Public projection requires valid per-service consent and configured
public gateway. It may be Planned before a process listens; public-first
preference is metadata for environment injection, not route activation.
Registered publication separately requires fresh complete ownership proof.

A remote app must not open a NodeLoopback URL on its own machine as though
it reached the node. Private browser admission remains the separate node
PreviewService lease for the attested BACKEND origin. A proxy URL or healthy
snapshot cannot mint that lease. No node credential enters a generated
service URL, page or profile. The real listener/forwarding package still
requires its reviewed ingress/header policy and actual drain qualification.

## Dedicated Admin and private persistence

- `ServicesSettingsRevision` is an opaque settings CAS etag. None means
  absent and is checked as absence, never an unconditional overwrite.
- `ServicesAdminSettings` contains that revision, gateway choice and a
  bounded list of per-service requested/effective choices. Get is Admin,
  not ordinary Read. Workspace snapshots expose only their service's link
  state, not the private file or other workspaces' consent records.
- Gateway choice is LocalOnly or Public(listener, public base origin).
  Public listener is a validated literal socket endpoint; configuration
  never activates any service by itself. Public base requires a usable DNS
  base, not userinfo/path/query/fragment or a node-control/browser endpoint.
- Exposure choice is Private or AnonymousPublic with explicit typed
  acknowledgement that the service becomes reachable without Vessel auth.
  Enable carries the displayed snapshot and CommandsDigest. The node derives
  the canonical binding itself; a caller-supplied URL/PID is not a grant.
- Changing gateway choice atomically clears existing public service choices
  and closes the old ingress/relays. A new gateway alone enables no service;
  unchanged gateway input does not revoke or write. Each public service
  requires fresh explicit consent for the replacement gateway.
- Requests carry the settings revision. Unknown choices refuse. Failed
  persistence preserves the prior file and policy; no implicit fallback.
  Results distinguish unchanged, saved/applying and applied observations.
  A timed-out call is not a revocation/drain receipt. Active HTTP/WS owners
  close before completion/capacity reuse; no success claim before settlement.
- Public consent binds exact workspace/script, current definition digest
  and the CANONICAL workspace physical incarnation. Actual run observers
  stay in memory; they are not persisted or supplied by clients. Fresh
  complete run/socket evidence is required for every effective route.
- No qualified canonical-incarnation producer exists yet. Enable/restore
  must therefore return Unavailable before writing an enabled grant. Do
  not replace the missing type with a UUID, revision or cwd of our own.
- The private services document will hold its settings revision, gateway
  choice and bounded map of anonymous consent records. A record owns target,
  CommandsDigest and the required canonical incarnation value. Private choice
  removes that record. No actual-run id, PID, effective route/health or URL
  observation is persisted as authority. No per-record counter duplicates
  the document's settings CAS revision.
- The private StoredServiceConsent schema is NOT implemented by this DTO
  package: its incarnation field must use that future qualified owner type.
  Missing is not optional/implicitly trusted. This unresolved producer is
  an explicit activation prerequisite, not a placeholder storage decoder.
- The Settings owner supplies the reviewed SavedNodeSettings envelope and
  private services slot in the SAME settings.json, atomic/set-aside rules
  unchanged. Dedicated Admin setters alone reach it. No generic set_settings
  path, ordinary Read exposure, second file, lock, writer or shadow cache.

## Bound ownership

Until code exists these are the single documentary definitions, moved to
their responsible types with the numeric copies removed in the code commit:

- Page entries and generated environment peers: the qualified shared
  MAX_PROJECT_COMMANDS, never a duplicated project-file count literal.
- ServiceProxyUrl encoded bytes: workspace_services::MAX_PROXY_URL_BYTES.
- Public service choices per node: workspace_services::MAX_PUBLIC_SERVICE_CHOICES.
- Retained service snapshot bytes per node: MAX_SERVICE_CACHE_BYTES = 8 MiB.
- Concurrent service long-poll waiters per node: MAX_SERVICE_WAITERS = 64.
- Host label and complete hostname: workspace_services::MAX_PROXY_LABEL_BYTES
  and MAX_PROXY_HOST_BYTES; the reference hash suffix uses
  PROXY_HASH_SUFFIX_BYTES = 8. Hostname generation is a single owner and
  must match the reference normalization/collision rules before wiring.
- SERVICE_PROBE_TIMEOUT = 500 milliseconds and
  DYNAMIC_ALLOCATION_ATTEMPTS = 10, from the reference. Other health and
  long-poll limits reuse their existing named owners, without new literals.

New finite wire/count/URL limits are a bounded-resource/typed-refusal floor
where the reference has none. No silent truncation. Existing RPC frame
limits bound aggregate decode before these validated domain values.

## Tests and delivery

1. Contract security review before DTO source. Use an isolated values package
   from the announced head with staged rows and the shared project-command
   bound, preserving A8 history. Never merge the unqualified A10 helper stack
   just to publish DTOs. Reuse the reviewed ServiceEnvName value owner unchanged
   if it is not yet on that head; no second normalization implementation.
2. Validated constructor/serde bounds, unique names, cursor consistency,
   plain/service invariants, no empty-as-unavailable success, health/URL
   distinctions, credential-safe URI errors and unknown-choice refusals.
3. Method table/shape/scopes/staged-default/None tests; all eight explicit
   nonforwardable choices, typed client/envelope/forged-origin refusal and
   the exact direct-principal matrix above before effects. No parallel
   registry/dispatch/number mechanism.
4. Before runtime: canonical active/automation/approval/start and restart
   leases, actual prior wait, finite physical work, plan/cache ownership,
   rollback, all approved env peers and no effect on Read/cancellation.
   Qualify the retained-snapshot byte/count/lifetime and global/shared waiter
   limits above before wiring; values/page bounds do not discharge them.
5. Before public routes: reviewed Settings envelope API, mandatory canonical
   incarnation, dedicated Admin validation, trusted ingress/header policy,
   failed-write preservation and active HTTP/WS revocation/drain tests.
6. Native consumers show pending/unavailable honestly and an explicit
   anonymous-public warning. All approved fields still display or refuse.
7. Narrow changed-crate checks per source step; one enclosing feature gate
   before real handler integration. No staged fixture qualifies a service,
   private browser, platform or owner account. No app files are owned here.

## Resume point

The DTOs, eight staged rows and typed SDK wrappers are authored in the
isolated values package. Source review, exact changed-source checks and
generated-wire freshness remain pending for the latest additions. No private
settings disk model, runtime handler or actual backend/public activation is
implemented by this package. Values and fixture tests cannot qualify the
separate Preview/allocator/helper/runtime producers.

The first strict package check found an omitted exhaustive added-agent reply
classification and one unnecessary qualification in a test. Service replies
carry terminal/session identifiers and command definitions, not a session's
agent specification, so their explicit compatibility arms join the existing
no-added-agent branch. Existing compatibility arms and test expectations stay
unchanged. The original compile/lint failure remains recorded; no tests ran.

The values tree now also contains the announced registry metadata protocol
head. Its assigned constants and schema remain unchanged, while the service
feature stays unassigned. The canonical physical-incarnation and connected
peer/scheme producers are still absent; neither this merge nor an etag grants
runtime authority.

The next strict check reached the remaining value fixtures: the empty Admin
request now shares the existing empty-object sample arm; optional-value refusal
conditions use the equivalent positive `is_none_or` form. The environment test
constructs its known-valid selected target separately from the fallible plan
constructor, so the latter returns only the rule's validation result. No input
set, assertion, expected error or deadline changes. This failed compile/lint
receipt also remains recorded and ran no tests.

The legacy web bridge has its own exhaustive admission decision. It refuses
all these staged methods before obtaining a node client, including Admin
reads; the values package provides no qualified web consumer. Its complete
refused-method test gains only the new methods, and decoding tests cover
service Read and Admin requests. A future browser admission change needs a
separate reviewed decision, rather than inheriting CLI/native admission.

The third strict check reached the client fixtures and rejected ownership of
an error that the assertion only inspects. The fixture now borrows each
returned error; its exact variant/method assertions and all budgets remain
unchanged. That earlier check ran no tests either.
