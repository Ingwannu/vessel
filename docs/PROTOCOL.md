# Protocol

The types are defined in `crates/vessel-proto`; this file is the changelog
and a reader's summary. Bump `PROTOCOL_VERSION` on any breaking change and add
an entry here. Additive, optional fields do not need a bump.

## TypeScript declarations (`web/src/wire/`)

The page's declarations are generated from these types (ADR 0017);
nothing here changes the wire. Since milestone 11 a primitive alias is
declared by what it is (`vessel-proto/src/ts/aliases.rs`, one table):

- **A handle**, minted by a node, the bridge, a harness or the page's
  bridge client and only handed back by the page, is branded:
  `type SessionId = string & { readonly __brand: "SessionId" }`. A
  string or number is not one until a guard of its shape says so
  (`web/src/bridge/ids.ts`), so two kinds of handle cannot be swapped.
  The numeric ones too (`CallId`, `StreamId`, `SubscriptionId`): the
  first two come from one counter and are looked up in different maps.
- **Plain**: what a person types or names (`BookName`, `ProfileName`,
  `SecretName`, `SessionTitle`, `FilePath`, `EnvName`, `HttpUrl`,
  `KeyBinding`, …), checked where it is typed; quantities (`EventSeq`,
  `Offset`, `DurationMs`, `TokenCount`, `ProtocolVersion`, sizes,
  revisions); secrets (`SecretText`, `SecretValue`, `PairingLink`,
  `InvitationSecret`, `PresentedToken`, `TicketMac`), which the page
  takes from a person and hands on once; and content (`TerminalBytes`,
  `DraftText`).

A new alias is classified in that table, by this rule, in the change
that adds it: the generator's test fails on an alias in no class, and on
an entry neither generated file declares any more.

Unassigned protocol numbers use `Option<ProtocolVersion>`, not a numeric
stand-in. A connection admits calls through `Call::required_protocol()`,
which is `Call::since()`: both return `Option<ProtocolVersion>`, the
method's number raised by any later field it uses, and for a fleet envelope
by the call it carries. The client refuses `None` before allocating an id
or queuing a frame with `CallError::UnassignedProtocol { method }` (code
`Unsupported`); the server refuses before dispatch. Each future feature
supplies its optional threshold, assigned by integration only after its
clients can read or approve the new data.

Numbers are assigned at merge, in merge order (2026-11-15): the next
feature merged takes `PROTOCOL_VERSION + 1`, and a reservation only marks
intent. A number is never skipped: a node built while a lower number waited
for its feature would advertise the higher one and so claim a feature it
lacks.

A pending method has a row in the method table like any other (Quality B):
`Variant / handler / "wire.name" [Scope, (pending FEATURE), shape]:
Request => Response;`, where `FEATURE` is the feature's
`Option<ProtocolVersion>` constant. `Method::since()` is that constant, so
the method is refused on every connection while it is None, and assigning
the number is a one-line change to the constant. The row's last column is
the method's shape (`Method::shape()`): `unary` (one reply), `stream` (the
reply opens a stream of frames: `session.subscribe`, `terminal.subscribe`)
or `envelope` (`fleet.forward`, `fleet.relay`, `fleet.deliver`: answered
once with the carried call's reply, whose outcome it is). No pending method
is registered by this foundation.

A staged method (2026-11-15) is a pending method whose request and response
types land before any node serves it: `[Scope, (staged FEATURE), shape]`.
It is refused on every connection the same way (its constant is None, which
the row's generated code asserts at compile time). Its `Handler` method has
a default body answering `Unsupported` without the request's values, and
the node's guard generates nothing for it, so a values-only package needs
no runtime to compile. The package that serves the method changes `staged`
to `pending` in the commit that adds its handler; from then on the guard
requires the handler, as for every assigned or pending row.

## Version 25 (2026-11-15, milestone 12): workspace registry metadata (W11, ADR 0082)

Protocol 25 introduces the independent workspace registry's six metadata
methods: `workspace.registry`, `workspace.list` and `workspace.get` (Read),
and `workspace.rename`, `workspace.pin` and `workspace.release_automation`
(Operate). Each is unary and fleet-forwardable. A person calls them: an
agent principal, direct or delegated, is refused before any lookup, and a
forwarded call from a person (owner or device) holds the delivering device's
scopes intersected with its origin's. A delegated `workspace.registry` reads
at once, without the long poll. `workspace.release_automation` clears only
the retained blocked-source gate, by compare-and-set on the expected
revision and the exact typed source; it approves no command and starts no
automation.

Only these six activate. Workspace creation and filesystem provisioning,
generation, resource start and exit, archive, restore and deletion, project
config reads and writes, editor launch and D2 effects have no method row and
remain unavailable. The six were reviewed with
`PROJECT_WORKSPACE_REGISTRY = None` (the pending section below) and assigned
25 at their node-line merge, the next free number in merge order. The
providers (W22), once reserved 24, take the next free number when they
merge. The minimum protocol remains 2 and the store schema (20, which
already holds the registry tables) is unchanged.

## Version 24 (2026-11-15, milestone 12): node diagnostics (N1, ADR 0074)

Protocol 24 introduces `node.diagnostics(GetDiagnostics {}) -> DiagnosticReport`:
Read, unary, not fleet-forwardable (Forward, Relay and Deliver refuse it).
It was reviewed and gated with its number unassigned (`None`), then assigned
24 at its node-line merge. Protocol 24 had been reserved for
`providers::PROVIDERS` (W22); by the numbering rule above, the providers take
the next free number when they merge. The minimum protocol remains 2 and the
store schema is unchanged.

The request has no fields; unknown fields are refused. The report has node
facts and explicit process, store, agents, sessions, listeners, relays and
recent-failures sections, each available or unavailable for a stated reason.
Process values are a checked process id, optional parent and JSON-safe
resident and virtual sizes. A busy or stopping node answers the existing
`Unavailable` code with a closed `ErrorDetail::Diagnostics` reason
(`busy`, `stopping`). The report carries no raw log, environment, path,
key, token, name or error text. Below 24 the client refuses before sending
(`NotInProtocol`) and a node refuses a forced request before dispatch.

## Version 23 (2026-11-14, milestone 12): saved-sign-in limits (N2, ADR 0042)

N2 proposed 19 for these together with `usage.query`; `usage.query` took
21 and the inbox 22 at the merges, so these two calls are 23 and
`usage.query` keeps 21. The minimum protocol remains 2.

Protocol 23 introduces `usage.limits(GetUsageLimits) -> UsageLimits`
and `usage.lookup(SetUsageLookup) -> ExternalLookup`. These are separate from
retained `usage.query` history. For `LimitsScope::Account`, omitted profile
selects the configured default, and the reply names the selected profile.
For `LimitsScope::Session`, the node uses the nonsecret binding of the actual
live launch, including successful profile moves. Reloading profiles does not
retarget it. Stored sessions without a live binding return `unassociated`.

Cached reads require read and perform no credential/vendor I/O; `refresh`
additionally requires operate and observes admission and Retry-After. Only
admin can change the machine-wide policy, saved before acknowledgement.
Agents may read only their own local session. Forwarded calls retain the
existing destination/intersected-origin authorization. All three envelopes
require the maximum of envelope and inner method/field versions, so a pre-23
hop refuses both methods before sending or dispatching them.

`UsageLimits` carries `policy`, canonical `account`, typed source disclosure,
`AccountLimitState`, `cost_visibility` and optional `LimitNotice`. Off clears
external observations; enabling does not poll. State distinguishes not yet
requested, in progress with a retained observation, ready, and safe failure
with optional same-login retained observation and actual cooldown. The
snapshot's timestamp remains its observation time after a failed refresh.
No credential, credential path, account fingerprint, email, raw API body or
vendor error is serialized. Credit balances require operate; eligibility
booleans remain visible without claiming a monetary unit.

Each window has a bounded stable id, explicitly reported period (unknown when
absent), provider slot, independent account/model/surface/feature/review scope,
optional finite reported percentage, and optional reported reset time. Zero
and missing are distinct; consumers clamp only for drawing. No duration is
inferred from the plan or primary/secondary slot. `incomplete` prevents a
partial response being presented as a complete set. Additional paid-use and
credits/allowed flags remain distinct from subscription fullness.

A notice names reported full general windows and gives their latest future
reset only when all selected full windows have one. Explicit paid/credit/
allowed permission suppresses the notice. It is not a stopped-turn diagnosis
or an automatic recovery action. Screens localize and format these facts,
show retained observations as stale, and offer only implemented operations.

## Version 22 (2026-11-14, milestone 12): the inbox and push (N1, ADRs 0040 and 0041)

N1 proposed 19; side questions took 19, `device.self` 20 and `usage.query`
21 at the merge. Nodes speak 2..=22. Inbox and push are additive; older
clients receive no new unsolicited frame and new calls are refused locally
below 22. The inbox's tables are store schema 16.

`inbox.list {before?}` requires Read and returns exact whole-node counts
(`total`, `waiting`, `unread`), at most 16 `entries`, and an exclusive `next`
cursor. An entry has its session and source sequence, timestamp, current
session/project labels, shared read state and typed permission/question or
finished/failed/limit item. `inbox.get {key,fetch?:"summary"|"live_request"}` returns that exact current entry
or null. `inbox.mark {key,read}` changes shared attention; agent origins,
including delegated agents, are forbidden. Answers still use Operate-scoped
`session.send` / question answer calls and their existing validation. No
request disappears merely because it is read. Oversized forms are explicit
`in_session` references, never shortened option sets.

`push.key` (unit request, `params: null`) returns the node's public VAPID identity. `push.subscribe
{subscription:{endpoint,p256dh,auth}}` replaces this authenticated paired
device's subscription and returns its 48-hour `expires_at`; `push.unsubscribe`
(unit request, `params: null`) removes it. These Read-scoped calls refuse owner-token, agent and delegated
principals and accept no supplied device identity. Endpoint/key/auth parsing
is bounded; diagnostics redact capability material. Registration and sending
both check current authority, revocation and expiry. `push.presence
{focus,activity}` is connection-scoped: focus is session/elsewhere/background,
activity is active/unchanged, with time assigned by the server. Agent and
delegated origins cannot report a person's activity. Closed connections lose
presence; an idle open connection does not prevent push indefinitely.

All additions negotiate through `Call::since`; minimum protocol is unchanged.
An older client receives no unsolicited new frame. ADRs 0040 and 0041 specify
cursor movement, lifecycle, limits, notification watermarks and relay privacy.

Opt-in inbox live-request fetch recovers one actor-owned pending form after
history pruning. Summary is the compatible default. Expanded replies are
capped at 15 MiB; larger forms remain exact compact references with no clipped
choices, and an answered or withdrawn live request returns absence.

## Version 21 (2026-11-14, milestone 12): reported usage (N2, ADR 0042)

N2 proposed 19; side questions took 19 and `device.self` 20 at the merge.

`usage.query` requires `read`. `QueryUsage` selects the machine, an
agent/profile or a session, and a rolling day/week (or all retained
samples of a session). `after` pages the session's samples by exclusive
event sequence. Totals are independent of that cursor. At most 100 samples
and 256 agent/profile groups are returned. Optional counts stay absent
when the agent did not report them. Cache counts are not added again to
input or total tokens. Cost units are never combined.

`UsageReport` declares accounting start, age retention and incomplete
coverage. `cost_visibility` distinguishes no reported cost from cost
withheld because the caller lacks `operate`. That withholding covers
totals, groups and individual cost samples in this new call. Existing
session/event calls retain their earlier cost visibility contract.
Agents may query only their own session on its own machine; delegated
calls apply the destination's existing scope intersection. A machine or
agent query with the session period/cursor is `invalid_request`, a missing
session `not_found`, a scope/ownership refusal `forbidden`.

The minimum protocol remains 2. No event kind changed. `ContextUsage`
adds optional `observation: current|replayed` metadata in protocol 21;
`current` is the omitted default. The runtime marks reports received during
conversation loading, including a move, wake or rewind whose Ready is
consumed. Replayed costs establish baselines/gaps without spending a
historical increase. Older peers receive their original context report
without the metadata, including through forwarded conversation replies.
New calls are refused
by the existing negotiated-version guard on an older node; an old client
continues reading the existing ACP reports in the conversation log.

Cost timing and optional-field coverage (ADR 0042): `current_costs` is the
session's latest cumulative agent report, independent of the requested
rolling period. Increment samples carry `since`; an interval crossing the
period boundary contributes an unaccounted-cost notice, never its whole
charge to that period. Existing/resumed/unknown counters produce baseline
samples until their increments can be observed. `optional_reports` counts
turns reporting each cache/reasoning field; their sums are reported
subtotals, with missing turns explicitly distinguishable.

A read-only session page can have no visible samples and still have a
`next` cursor: the page contained only costs. Clients continue while
`next` exists and do not interpret an empty page as end-of-history.

For `UsageScope::Agent`, an omitted profile includes every recorded profile
of that agent. A supplied name restricts the historical profile recorded
with the sample. Account quota lookup is a different scope: its default
profile identifies a configured login, never an aggregate of accounts.

Usage scans admit at most two queued or running blocking jobs per node;
excess calls receive `unavailable` with advice to retry shortly. Cancellation
does not release admission until the blocking job finishes. Cost arithmetic
uses exact decimal inputs and integer accumulation. Final sums or increments
that cannot survive the finite wire number's decimal spelling are omitted.
For an omitted unit sum, `unaccounted_costs` counts every contributing cost
sample, rather than silently presenting a rounded subtotal.

The negotiated version requirement for a typed fleet envelope includes its
inner call's method and fields, recursively, for forward, relay and deliver
(version 20's one rule; N2 added the same).
An older intermediate peer refuses `usage.query` before sending or dispatch,
even when a forced envelope targets that peer itself. Older supported
envelopes retain their existing requirement and dispatch behavior.

## Version 20 (2026-11-14, milestone 12): caller authority (N3, ADR 0044)

- `device.self {}` returns `DeviceSelf { device?: DeviceId, scopes: Scope[] }`.
  Authentication is required without an extra scope, so a caller without
  Read can inspect its own grant. Unknown request fields are refused. Only
  a directly paired caller gets its own id; owner/agent/delegated calls omit
  it. Scopes are canonical, capped by the existing five-value universe, and
  reflect the same principal checked by the guard, including agent clipping
  and delegated intersection. No database or lifecycle policy changes.
- This is an additive call, with minimum protocol still 2. Old direct and
  routed hops refuse before send/dispatch. Typed envelopes now include the
  inner call's method/field version in their requirement, since every hop
  decodes and re-encodes it; an old hop must not drop an unknown field
  (version 19's side link needed the same, and the line keeps this one
  general rule for both).
- Method metadata's Rust `scope()` is optional: None explicitly means an
  authenticated call with no additional grant, not anonymous access. Every
  legacy method retains Some(scope). Scopes do not replace ownership,
  profile policy or agent limits. Unsupported self queries remain unknown
  authority to the caller, never an inferred full or read-only grant.

### Pairing setup refusal (N3 row 4, ADR 0044)

`device.invite` retains its original method/version and Admin requirement.
If neither a direct address nor a relay is advertised, the node refuses
before creating invitation material: Conflict with optional detail
`{ kind: "device_invite", reason: "no_routes" }`. `DeviceInviteError` also
has an explicit Unknown future reason. The human message says to configure
a reachable --public address or --relay, then create another invitation.
The same refusal is sent on older negotiated connections: their decoders
ignore unknown detail and retain the message. A configured route is not a
remote connectivity proof; callback invitations use their effective override.
An older node can retain its previous behavior; this refusal guarantees the
behavior of the updated node, not a retroactive change to an older server.

## Version 19 (2026-11-14, milestone 12)

The track raises the current version once to 19; minimum remains 2.

### Side questions (N5, ADR 0045)

- `SessionSpec.side_question_of?: SessionId` links a separate structured
  conversation to a local parent. It never resumes the parent's native id.
  Ordinary creation naming this field uses the same serialized open/reuse
  operation and inherits the parent's current configuration.
- `session.side_open { session } -> SessionInfo`: reuse the newest
  unarchived side, including an ended/interrupted one, or start a new side.
  Opening never implicitly resumes an ended side.
- `session.side_prompt { session, question: PromptText } -> Delivered`:
  refresh the parent context and start a turn in the named side. A live
  parked side wakes first; a failed wake consumes only that request. Busy
  or ended sides are conflict. General prompt and queue operations refuse
  sides, including scheduled and tracked prompts; old clients can read
  them but must update or use `vessel session side` to ask.
- `session.side_reset { session, side } -> SessionInfo`: serialized
  expected-id replacement. A stale side is conflict. Deletion failure
  creates nothing; creation failure after deletion leaves the old side
  gone. Opening again retries; no rollback is promised.
- All three calls need read and operate. Direct and delegated agents are
  refused; delegated people retain the usual scope intersection. Calls
  can cross the fleet and the existing encrypted relay. Neither questions
  nor answers are written into the parent conversation.
- Context uses current status/attention, an independent latest-plan
  projection (schema 14; twenty steps, 1,200 characters each), and a
  labelled partial tail of at most 128 records. Snapshots are at most
  24 KiB in UTF-8 including metadata and markers; complete serialized ACP requests at
  most 48 KiB, including prefixes, JSON escaping and the request envelope.
  TurnStarted retains only the question; context is transient provider
  input carried atomically with it. Native slash questions are refused
  explicitly rather than moved behind a context prefix. Question text uses the existing 16 KiB `PromptText`.
- Parent deletion leaves existing side logs readable but prevents new
  questions/resume. Reset inherits current profile/model/choices; an
  existing side keeps its own configuration. Imports clear node-local
  parent links. Saved/scheduled templates refuse side specs.
- A parent rewind clears its plan projection and limits the partial
  transcript to records after the latest rewind, so discarded future
  messages cannot be presented as current. A new plan report replaces
  the cleared projection. Generic session.fork refuses side sources;
  side_reset is the supported start-over path.
- All direct and currently supported nested session specs require
  protocol 19 when they name a side link, so old nodes cannot silently
  ignore it. The optional field is ignored by older apps. No new events
  or stand-ins are required.

## Version 18 (2026-11-14, milestone 11's track A2, ADR 0057's slice 0)

Nodes speak 2..=18. No method, no event, no record: one field of the
node's settings. A client sends a change naming it to no older node
(`Call::since`; `unsupported`, "needs protocol 18"), since an older node
would ignore it and keep every agent on ACP without saying so.

### Agents over their own protocols (track A2, ADR 0057)

Types in `vessel_proto::settings`.

- `NodeSettings` gains `drivers?: AgentDrivers { claude_code, codex,
  open_code }`, each an `AgentDriver`: `acp` (the Agent Client Protocol,
  the agent's own ACP mode or its adapter), `direct` (the agent's own
  protocol: Codex's app-server, Claude Code's stream-json, `OpenCode`'s
  server) or `unknown` (a newer node's). Absent when every agent runs
  over ACP, as from a node older than 18. Gemini and agents added on the
  node have no protocol of their own and always run over ACP.
- `node.set_settings { drivers?: SetAgentDrivers { claude_code?, codex?,
  open_code? } }` changes the agents it names. `unknown` is refused
  (`invalid_request`); `direct` is refused (`unsupported`) for an agent
  whose driver this build does not offer yet. Which are offered is the
  build's, not the version's: Codex's from its driver's slice 2 (it asks
  a person for permissions and questions as over ACP); Claude Code's and
  `OpenCode`'s not yet. A
  change is kept in `settings.json`; a choice a newer build saved runs
  as ACP here and stays in the file.
- What a setting changes: the driver of that agent's sessions started
  after it, chosen at each creation and start again, and used by its
  probes (`harness.options`) and listings (`agent.history`). A session
  keeps its driver for its life, across moves, parks and rewinds, since
  its options are that driver's. The node runs the agent's own protocol
  only for a version it was checked against (the agent's `--version`);
  for an older or unreadable one the session runs over ACP and its log
  says why, and for a newer one it runs and its log says so.
- Nothing else on the wire changes: a direct driver produces the same
  events, options, permission requests and questions as ACP.

## Version 17 (2026-11-14, milestone 11's track A2 slice 4, on milestone 12's node line, ADR 0053)

Nodes speak 2..=17. One method, no event, no record. Track A2 wrote it
at 14 beside forking; on the node line 15 (N3's fleet controls) and 16
(`fleet.published`) were taken first, so it is 17 here. A client
sends it to no older node (`Call::since`; `unsupported`, "needs protocol
17"); a node refuses it on an older connection.

### Importing from an agent's own history (track A2, ADR 0053)

One method; types in `vessel_proto::history`.

- `agent.history { harness, profile?, cwd?, cursor? }` (operate,
  forwardable, refused to agents) answers `AgentHistoryPage {
  conversations: [HistoryEntry], next? }`: the conversations the agent
  keeps under that profile, newest first, as it lists them over ACP
  (`session/list`; the node starts the agent as for `harness.options`,
  and no session is created or loaded). `cwd` (absolute) keeps those
  held in that folder; `cursor` is a page's `next` as it was given (at
  most 4,096 bytes). A page holds up to 1,000; it can be empty with a
  `next` after the node has asked past several empty pages of the agent's.
- `HistoryEntry { harness_session, cwd, title?, updated_at?, in_vessel?
  }`: the agent's id, its folder (absolute), its title cut as a prompt's
  title is, when it was last active, and the newest session on this node
  (live or kept) on that conversation.
- Importing is `session.create` with `mode: structured { resume:
  harness_session }`, the entry's `cwd`, the `profile` it was listed
  under and, for a title, its `title`; the agent's replay is the new
  session's log.
- A node of 17 refuses `session.create` with `resume`,
  `session.resume` and `session.move_profile` with `conflict` while
  another live session (parked included) holds that conversation, or is
  starting on it or moving it: two agents on one transcript would
  corrupt it. A session holds its conversation under every profile its
  transcript is in: the one it started under, and each profile
  `session.move_profile` took it to or from, since a move hard-links the
  transcript into the target home for good (also when the new agent did
  not start). A session started again holds the profiles of the moves
  its log still records. The default profile is one profile whether it
  is named or not. This is the node's own check, so a client of any
  version meets it. A copy into another profile (`session.export` then
  `session.import`, which the CLI's `vessel session copy` and `vessel
  session move` use; the move kills the source after the copy runs) is
  another file and is not refused. The listing marks such an entry
  `in_vessel`; a client opens that session rather than import it.
- Errors: `invalid_request` (a relative folder, a cursor no listing gave,
  the agent refusing it), `unsupported` (an agent that does not list or
  load over ACP), `not_found` (a profile or added agent that is not
  there), `spawn_failed` (the agent did not start or answer within its
  handshake), `unavailable` (two listings already run on the node).

## Version 16 (2026-11-14, milestone 12's node line, ADR 0056)

Nodes speak 2..=16. One method, no event, no record; a client refuses it
locally on an older node (`unsupported`, "needs protocol 16").

`fleet.published {}` (admin, not forwardable, refused to agents; the browser
bridge passes it) answers what this machine shares through its fleet, by
name and metadata, never file contents, secret values or a tool server's
command, arguments, address or headers. Tagged `role`:

- `standalone`: it shares nothing (a value, not an error).
- `home { bundle? }`: what it publishes; absent before its first bundle.
- `member { home, bundle?, applied? }`: the bundle it last received from
  `home`, and the version last applied here (older than `bundle`'s when
  the last apply did not finish).
- `unknown`: a newer node's role.

`PublishedBundle { generation, files: SharedFile[], servers, secrets }`;
`SharedFile { path, kind: instructions | skill | other, size, executable }`;
`servers` are tool (MCP) server names, `secrets` the secret names they use,
each once, sorted. Over a bound (4096 files, 256 servers, the reply's byte
budget) the whole read is refused: `unavailable` with
`detail: { kind: fleet_control, reason: published_too_large }`, which a
version 15 client reads as `unknown`. The member protocol
(`fleet.manifest`, `fleet.peers`) is unchanged and stays `sync`-only.

## Version 15 (2026-11-13, milestone 12, N3 row 2)

Nodes speak 2..=15. Track N3 built its fleet controls as part of its
protocol 12 and asked for them to share the preview's 13; on the node line
13 and 14 were already published without these calls, so they are 15:
a client refuses them locally on an older node (`unsupported`, "needs
protocol 15") rather than sending a method that node cannot decode. No
event and no record; `ErrorDetail::FleetControl` is a new kind that a
13 or 14 client reads as `unknown`, and an older one skips with `detail`.

`fleet.join { link, grant }` (Admin) redeems one single-use invitation and
returns `JoinedFleet { home }` only after persisting the selected machine's
membership. `fleet.leave {}` (Admin) removes membership and its credential,
keeping materialized files; already left succeeds. `fleet.sync {}` (Admin)
returns Ack when accepted by the existing loop, not when applied. These
mutations cannot be forwarded or used by an agent. A disconnected join may
have redeemed its invitation; clients never automatically retry it.
`fleet.status {}` (Read) returns `FleetSyncStatus { role, local, members? }`.
`role` is standalone, home or member (home id and join time). `local` is last
applied generation/time, home contact time, relative conflict paths and typed
problem categories; history can predate the current join. Only the home's
own authenticated owner/paired Admin sees per-member device reports.
Read-only and delegated reads omit `members`; they never receive credentials,
secret names, absolute destination paths or arbitrary local error text.
Refusals use `ErrorDetail::FleetControl { reason }` (ADR 0044).

## Version 14 (2026-11-13, milestone 11's track A2, on milestone 12's node line)

Nodes speak 2..=14. Track A2 built forking and rewinding as part of its
protocol 12; on the node line N3's schedule preview had taken 13 first,
and a client built at 13 has no variant for the two new records, so they
and the two calls are 14 here. A client sends none of these calls to an older node
(`Call::since`); a node refuses them on an older connection
(`unsupported`, "needs protocol 14").

### Forking and rewinding (track A2, ADR 0052)

Two methods, two fields, two records; types in `vessel_proto::forking`
and `vessel_proto::agent`.

- `AgentAbilities.fork`: the agent advertised `session/fork` (ACP's
  unstable `sessionCapabilities.fork`). Absent from older nodes; a fork's
  own handshake decides.
- `TurnEnded.point?: TurnPoint { message, conversation }`: where a fork
  or a rewind can start; set when the turn's last agent message carried
  an id (ACP `messageId`, at most 256 bytes) and nothing but the turn's
  end followed it. `conversation` is the agent's conversation the turn is
  in. Absent otherwise, and in records written before 14.
- `session.fork { session, after? }` (operate, forwardable): a new session
  of the caller's continuing `session`'s conversation after the turn
  whose `turn_ended` record is `after`, or as it is now (refused
  `conflict` while a turn runs). Answers the new session at creation, as
  `session.create` does; it runs in the source's folder with the
  source's current choices. Answering at creation is deliberate: the
  check needs the agent started and a replay read, longer than a call
  should hold. A fork whose agent does not fork at the point ends
  `failed { message: "the agent did not fork at the chosen point",
  empty_fork: true }`.
- `SessionStatus::Failed.empty_fork` (since 14, absent when false): a
  fork that never held a conversation (its agent refused the fork, could
  not load it for the check, or did not start). Nothing can be resumed;
  a client offers to remove it (`session.delete`).
- The fork the agent made for a refused check is deleted from the
  agent's own history on the same connection (ACP `session/delete`,
  which both installed adapters offer: Claude's deletes the transcript,
  Codex's archives the thread). An agent that does not offer
  `session/delete` keeps it, and nothing else removes it: Vessel never
  writes another program's files, so it lists in that agent's history
  (`agent.history`) as a conversation no session holds.
- `session.rewind { session, after }` (operate, ownership-checked,
  forwardable): the session goes back to the end of that turn; between
  turns (`conflict` otherwise); a parked session is woken first. Answers
  the session, its `harness_session` the new conversation.
- Errors of both: `not_found` (no such session or record), `unsupported`
  (a terminal session, a record that is not a turn's end with a point,
  an agent that cannot fork or did not fork at the chosen point),
  `spawn_failed` (the agent did not start; for a rewind the old
  conversation goes on).
- A rewind whose fork fails and whose old conversation then does not
  start again either (the agent cannot load any conversation) answers
  `spawn_failed`, and the session ends `failed` ("the rewind failed and
  the agent could not start the conversation again (...); resume the
  session to go on from before the rewind"): no agent is left running,
  and its `harness_session` is the one it had, so `session.resume` goes
  on from before the rewind.
- Records (`since 14`; a version 2-13 client gets a `vessel:` log line):
  `forked { from, after? }`, a fork's first record after `starting`;
  `rewound { after, harness_session, files_changed }`: the records
  between `after` and it are no longer the conversation (they stay in the
  log), the agent's conversation from now on, and how many tool calls left
  behind edited, deleted or moved files (not put back).

## Version 13 (2026-11-13, milestone 12, N3)

Nodes speak 2..=13; the minimum is unchanged. Additive N3 calls are gated at
13. `NodeInfo.timezone` is an optional IANA name, absent when detection fails
or from old nodes, and omitted on negotiated older connections.

`schedule.preview` (Read) takes `cadence`, optional UTC `after`, and bounded
`count` (1–10, default 5). It returns `after`, the effective `cadence` (an
interval's resolved anchor), and exactly `count` next UTC `runs`. Reuse the
returned cadence when saving. No schedule or session is created. Errors
are `invalid_request` with optional typed `detail: { kind: schedule_preview,
reason: unknown_zone | unknown_cadence | no_future_run | time_out_of_range }`,
or `unavailable` for the bounded worker pool. Existing errors omit detail;
future unknown detail kinds decode as unknown. Clients never parse messages.
The optional error `detail` is additive and is not stripped from older
connections: older error decoders ignore unknown fields. The error code and
message remain sufficient for those peers.

## Version 12 (2026-11-13, milestone 11, wave 2's second half)

Nodes speak 2..=12. Track A2's (`docs/plans/backend-m11.md`, "Track A2:
contract"); later tracks of the wave add to it. A client sends none of
these calls to an older node (`Call::since`); a node refuses them on an
older connection (`unsupported`, "needs protocol 12").

### Searching every session's messages (track A2, ADR 0037)

One method, `since 12`, read, forwardable, in `vessel-web`'s list; no
event, no change to an existing type. Types in `vessel_proto::search`.

- `session.search { query: SearchQuery, session?, before?: SearchCursor,
  limit?: SearchLimit }` → `SearchPage { hits: SearchHit[], next?,
  reached? }`. `SearchQuery`: 1 to 256 characters once trimmed, at most
  16 words, no control characters. `SearchLimit`: 1 to 100, 20 when
  absent. `SearchCursor`: the node's handle, a decimal number in a
  string.
- `SearchHit { session, seq, at, role: prompt | reply, snippet }`: `seq`
  is the record where the message's run starts. `Snippet { parts:
  SnippetPart { text, matched }[], cut_before, cut_after }`, at most 240
  characters, white space runs as one space, matches marked by part.
- Every word must be in a message, in any order, as a substring, without
  case (not without diacritics). Prompts and the agent's replies only;
  never thoughts, tool calls or output. A reply is found once it ends.
- Newest first. `next` while older messages may still match; a page may
  hold fewer hits than asked and still have it, when its work ran out
  (10,000 candidates through the index; 20,000 messages when every word
  is under three characters or `session` is given), with `reached` the
  time of the oldest message it read.
- Failures: `invalid_request` (query, limit or cursor), `not_found` (the
  `session`), `unavailable` (two searches already run on the node),
  `internal`.
- Not ownership-checked: agents read the fleet's sessions (ADR 0013), so
  they search them (the `search_sessions` tool).

### Parking an idle agent (track A2, ADR 0038)

Two methods, one optional field, two records; types in
`vessel_proto::parking`. The status is unchanged: a parked session is
`running`.

- `SessionInfo.parked?: Parked { since, reason: ParkReason }`; absent
  while the agent runs, for an ended session, and from older nodes.
  `ParkReason` (tagged `reason`): `idle`, `asked`, `wake_failed { message
  }` (at most 1 KiB), `unknown`.
- Records `parked { reason }` and `woken` (`since 12`); a version 2-11
  client gets `vessel: the agent's process was stopped (...)` and
  `vessel: the agent was started again` as `harness_log` lines.
- `session.park { session }` and `session.wake { session }` (operate,
  ownership-checked, forwardable; answer `SessionInfo`). `park`:
  `conflict` while starting or moving, while a turn runs, while a
  permission or question waits, or while the queue would move;
  `unsupported` for a terminal session or an agent that cannot resume.
  `wake`: answered once the agent is ready; `spawn_failed` when it does
  not start (the session stays parked, saying why).
- A prompt of any delivery, a queue change that starts one, an option
  change and a move wake a parked session first; after a failed wake a
  `queue` delivery is queued with the queue held `failed`. Reads and a
  cancel do not wake it.
- The node parks idle sessions on its own after `--park-after` minutes
  (60 by default; 0 never), never one that someone follows live.
- `node.settings {}` (read) answers `NodeSettings { park_after }` in force
  (and `drivers`, protocol 18);
  `node.set_settings { park_after? }` (admin) changes what it names and
  answers the same. `ParkAfter` (tagged `kind`): `never`, `idle { minutes
  }` (1 to 10,080), `unknown` (a newer node's; refused in a change,
  `invalid_request`). Neither is forwardable. A change is kept across
  restarts (`settings.json`) and wins over `--park-after`; it applies to
  live sessions at once. A node older than 12 answers `unsupported`.

## Version 11 (2026-11-12, milestone 11, wave 2)

Nodes speak 2..=11. Wave 2's protocol, raised once for the whole wave;
its tracks add to it side by side (`docs/plans/backend-m11.md`, "Wave 2:
tracks"), each with its part here, and the join of the first half fixed
two gaps the page found ("Integration 2 (node)"). Track D's schedules,
saved prompts and the starter told come last ("Integration 3 (node)"
says where they meet the rest).

### Pull requests (track B2, ADR 0029)

Six methods, one optional field, no event. Every type is in
`vessel_proto::forge`; every enum a forge may grow has `unknown`.

- `SessionInfo.pull_request?: SessionPullRequest { access, summary?,
  feedback }`: for a live session with a place. `access` says whether the
  node can see the session's pull request and why not (`ready { login? }`,
  `not_placed`, `no_branch`, `on_base`, `no_remote`, `not_supported {
  host }`, `signed_out { host, gh }`, `refused { host, reason, login? }`,
  `rate_limited { host, until }`, `unreachable { host, retry_at }`,
  `limit`, `ended`); `summary` (`PullSummary`: repository, number,
  address, title, state, draft, head commit, checks, review decision,
  mergeability, updated at) is the last one read. Cleared when the
  session ends. A forge's text (a title, a check's name, an address) is
  made lossy by the node, so it always decodes.
- `forge.status {}` (read): every host and how the node is signed in to
  it (`source: environment | gh`), at most 16; never a token.
- `pr.list {}` (read): every watched session's pull request.
- `pr.get { session, refresh }` (read): the whole `PullRequest` (checks
  with the failing ones named, people's reviews, times, the repository's
  merge methods). `refresh` reads now, at most once per 10 s per session
  (more often is answered from memory); a refresh that comes before the
  node has first read the session's folder (its branch, just after the
  session started) waits for that read and answers with what it finds
  (since 2026-11-14; before, it answered at once with no pull request);
  an agent may refresh only a
  session it may operate. A session that has ended answers `access:
  ended`; one the node does not watch (not placed) `not_placed`.
- `pr.open { session, title, body?, base?, draft }` (operate): from the
  branch the session's folder is on, into `base` or the project's default
  branch; a person's call pushes the branch first (never forced), an
  agent's never pushes (`not_pushed`). Answers `opened`, `exists` or
  `refused { not_pushed | no_commits | push { refusal } | access {
  access } | rejected { message } }`.
- `pr.merge { session, number, method, head }` (operate, a person only):
  `head` a full commit id; refused `no_pull_request`,
  `other_pull_request`, `not_open`, `draft`, `head_moved { head }`,
  `not_mergeable`, `method_not_allowed`, `merge_queue`, `cannot_pin`,
  `access`, `rejected`. The head is pinned in the forge's request too.
- `pr.feedback { session, enabled }` (operate, a person only): failing
  checks, changes requested, unresolved comments and conflicts sent to the
  session through its queue; off by default; `unsupported` for a terminal
  session. Each event is sent once, as one prompt beginning `Vessel: pull
  request feedback` (at most 20 events and 16 KiB, then `and N more`),
  queued as the owner's; while it waits, newer events are added to it.
  A review's or comment's words are quoted only when their author may
  write to the repository; nothing is sent while the session waits on a
  permission or a question.
- All forwardable and in the web bridge's list. Agents: `pr.get`, and
  `pr.open` on a session they may operate; the rest is `forbidden`.
- The board: a watched session's pull request places it in Validating,
  In review or Ready by the reducer of ADR 0019, which every client of
  protocol 8 and later already draws.

### Git apart from a repository's own configuration (track B2, ADR 0030)

- `GitRefusal::repository_config { key?: ConfigKey, reason }`: the
  repository's own configuration (its `.git/config`, `config.worktree`
  and what they include) sets something the node's git does not run
  with: `program` (an `ssh`, a credential helper, a proxy, a signing
  program, an upload or receive program, on a run that would start it),
  `credential` (a proxy, a TLS check, a resolver, a header or a cookie
  file that could send a network run's credential elsewhere), `layout`
  (`core.worktree`), `unnamable`, `too_many` (drivers), `too_large`;
  never the value. On a read the same refusal is `conflict` with the key
  in its message. A protocol 10 client reads it as `unknown`.
- `ConfigKey`: one line, at most 256 characters.
- Folders that are git's own (a `.git` folder by git's own naming rules,
  a folder inside one, a bare repository) are `invalid_request` for
  `project.add`, `project.init` and a clone's target.

### Two gaps the page found, closed in the join (ADR 0027's amendment)

- **A file's write is guarded by its content.** `WriteExpect` is
  `absent` (create only), `content { hash }` or `any`; protocol 10's
  `version`, which compared `FileVersion.modified` in nanoseconds (past
  what a JavaScript number holds) exactly, is gone (protocol 10 was
  never released). A page that saves an edit sends `file.write { root,
  path, text, expect: { kind: "content", hash } }`, `hash` being
  `FileVersion.hash` of the version it read or last wrote (a string,
  held exactly). Failures: `conflict` when nothing is there, or its
  content is not that (another write landed, the file grew past 8 MiB);
  nothing is written, and the page reads the file again to show both.
  `absent` answers `conflict` when something is there. A version without
  `hash` (a file over 8 MiB) cannot be guarded: the editor offers no
  guarded save for it. `FileVersion.modified` is milliseconds since the
  epoch, for showing only, never compared. A `content` write needs
  protocol 11 (`Call::since`).
- **`ProjectCommands::Listed.last_approved?: ApprovedCommands { commands:
  CommandSet { setup, teardown, scripts }, digest, approved_at }`**: the
  set last approved for the project, while the listed set is not
  approved, so a page can show what changed. Present only when the node
  kept what that approval covered: from protocol 11 an approval of a
  digest the node described lately (16 sets remembered by digest) keeps
  its set (store schema 11); an older approval has none and is never
  shown. Bounded as the listed set is (a file of at most 64 KiB). A
  protocol 10 client is sent the set without it.

### Terminals, scripts and ports (track C, ADR 0031)

Thirteen methods, one optional field and no event; ADR 0031 and the
plan's "Track C: contract" (with its review) say why each choice.

- A terminal is its own thing, carried by the sessions' terminal
  transport: its records are `EventRecord`s numbered from 0 (`status`,
  `terminal` output, and `agent { harness_log }` for Vessel's own
  `vessel:` lines), followed with `ServerFrame::Event` and ended with
  `SubscriptionEnd` as a session's are, paged as `EventPage`. Kept in
  the node's memory only (no store schema change); it ends with the node.
- Types (`vessel_proto::terminals`, `scripts`, `ports`): `TerminalId`;
  `TerminalPlace` (`session { session }` or `project { project, dir? }`);
  `TerminalKind` (`shell { program }`, `script { script, command }`,
  `service { script, command }`, `unknown`); `TerminalInfo { id, place,
  folder, kind, title?, created_by, created_at, ended_at?, status,
  service?, size?, oldest_seq, next_seq, revision }`; `ServiceState`
  (`starting`, `running`, `backing_off { attempt, code?, restart_at }`,
  `exited { code }`, `failed { reason }`, `stopped`, `unknown`) with
  `ServiceFailure` (`gave_up { restarts }`, `not_approved`, `spawn`,
  `unknown`); `ScriptName` (1 to 64 of letters, digits, `-`, `_`, `.`),
  `ScriptEntry { name, kind: script | service, command, dir? }`;
  `ListeningPort { port, address, pid, process?, owner: session |
  terminal }`, `PortList { ports, scan: pending | scanned { at } |
  unavailable { reason }, revision, more }`.
- `ProjectCommands::Listed.scripts`: the `[scripts.<name>]` and
  `[services.<name>]` tables of `vessel.toml` (`command`, optional `dir`
  inside the root; at most 32). The approval's digest covers them: a
  file without scripts keeps version 1's digest exactly (approvals given
  before stay), one with scripts is digested as version 2, which no
  earlier approval matches. A protocol 10 client is answered `invalid`
  for a set with scripts, since it could not show what it would approve.
- Methods (all in `vessel-web`'s list; all but `terminal.subscribe`
  forwardable; long polls wait at most 30 s in the caller's wait shares,
  and never when forwarded):
  - `terminal.create { place, size?, title? }` (operate): the account's
    shell in the session's folder, or in the project's (`dir` inside
    it); `unsupported` on a node without one. `terminal.list { place? }`,
    `terminal.get { terminal, changed_from? }`, `terminal.read { terminal,
    from, limit, wait? }` (at most 512 records and 1 MiB a page),
    `terminal.subscribe { terminal, from }` (read; answers
    `TerminalSubscribed { subscription, terminal }`); `terminal.send {
    terminal, input }` (at most 64 KiB; a script takes none:
    `unsupported`), `terminal.rename`, `terminal.kill` (ended, kept with
    its output), `terminal.close` (ended and forgotten) (operate).
  - `script.list { place }` (read; the checkout's commands with their
    approval, and its runs), `script.run { place, script }` and
    `script.restart { terminal }` (operate) answering `started {
    terminal }`, `needs_approval { commands }` or `refused { reason:
    no_project | no_such_script | no_such_dir | invalid_file { message } |
    already_running { terminal } }`. A run's command is the very parse
    whose digest was approved; a service is started again after a
    failure (1, 2, 4, 8, 16 s; it gives up after 5 restarts without a
    60 s run; an exit with 0 is not restarted) only while its set is
    still approved.
  - `port.list { changed_from? }` (read): the TCP listeners of the live
    sessions' and terminals' process trees, found without privilege
    (`/proc` on Linux, `lsof` on macOS, `netstat` on Windows), at most
    every 3 s and only within 60 s of a `port.list`; at most 256.
- Limits: 64 live terminals a node, 8 beside a session, 16 in a project;
  32 ended ones kept; a terminal's scrollback 4,096 records and 1 MiB
  (the oldest go, which a reader sees as a pruned range).
- Agents: `script.*` are `forbidden`. `terminal.create` only beside its
  own session or one it may operate, and only when both that session's
  profile and the agent's own session's set `agent_terminals = true`
  (`profiles.toml`; off for every default profile; the agent's node
  checks its own before forwarding); at most 4 at once; every other call
  and `port.list` only on its own terminals; they end when its session,
  or the session they are beside, ends.
- A terminal's environment is the node's without Vessel's credentials
  (`VESSEL_*TOKEN`, `VESSEL_*SECRET`, `VESSEL_AGENT_SESSION`,
  `VESSEL_NODE_SOCKET`, and a model route's variables when the node runs
  inside a session) and without the forge's sign-in (`GH_TOKEN`,
  `GITHUB_TOKEN`, `GH_ENTERPRISE_TOKEN`, `GITHUB_ENTERPRISE_TOKEN`), with
  `TERM=xterm-256color` and `COLORTERM=truecolor`; a script's adds
  `VESSEL_PROJECT_ROOT`, `VESSEL_WORKTREE`, `VESSEL_BRANCH` and
  `VESSEL_SCRIPT`. Setup and teardown (protocol 10) lose the same
  variables.
- Where they meet the rest: `session.delete` (and retention) ends and
  forgets the session's terminals before its worktree is tidied;
  `session.mark { archive }` ends them and keeps them, with their output;
  a session's end alone ends only agents' terminals tied to it. A live
  terminal inside a worktree keeps it (`kept { in_use }`), and so does one
  starting there (it holds the worktree until registered; a removal that
  holds it first makes the start `conflict`, before anything runs); one
  in a project makes `project.remove` `conflict`.
- A session's terminals are not on `SessionInfo`: `terminal.list {}` (at
  most 96) grouped by `place` gives every session's at once.
- The bridge: `follow_terminal { stream, node, terminal, from }`, whose
  records and states come as a followed session's do.

### Agents beyond the four (track G, ADR 0032)

- `HarnessRef`: an agent a node can run, one of the four as the string it
  always was (`"claude-code"`, `"codex"`, `"gemini"`, `"open-code"`) or
  an agent added on that node by its command, `{"added": "<name>"}`.
  `AgentName` is the profile name rule, never a name of the four in any
  case (`claude`, `opencode`, `open-code` included). It replaces
  `HarnessKind` in `SessionSpec.harness`, `Harness.kind`,
  `ProfileInfo.harness`, `ListProfiles.harness` and
  `GetHarnessOptions.harness`.
- `Harness.terminal?`: `{ state: "available" }` (left out) or
  `{ state: "unavailable", reason }`; an added agent has no terminal.
- A client of protocol 2 to 10 is never handed an added agent:
  `harness.list`, `profile.list`, `profile.reload` and `session.list`
  leave them out; a reply about one (`session.get`, `session.create`,
  `session.move_profile`, `session.rename`, `session.resume`,
  `session.mark`, `session.subscribe`, `session.import`) is replaced by
  `unsupported`, inside the fleet's envelope too; calls acting on one,
  and `session.subscribe`, are refused before they run. A call naming an
  added agent needs 11 (`Call::since`).
- `agent.list` (`read`) → `AgentInfo[]`: the four first, then the agents
  added on the node by name. `AgentInfo { agent: HarnessRef, detail }`,
  `detail` tagged by `kind`:
  - `known { installs: InstallOffer[], sign_in: SignInHelp }`: the
    vendor's install commands (`part` `harness` or `adapter`) and how a
    person signs in (`command?`, `home?`, the variable a profile sets for
    another account, `methods`);
  - `added { added: AddedAgent }`: `label?`, `program` (`AgentProgram`, a
    name on the search path or an absolute path), `args`, `env` (the
    names it sets, never a value), `secrets` (`{ variable, secret }`, the
    variable and the node secret it is filled from), `inherit`, `mcp`
    (the fleet servers it gets), `source` (`{ from: "command" }`).
- `agent.add` (`admin`, never an agent principal: `forbidden`) `{ name,
  agent: { from: "command", definition }, replace? }` → `AgentInfo`.
  `definition`: `AgentDefinition { label?, program, args, env:
  { name, value: string | { secret } }[], inherit, mcp }`. Names are one
  space with profiles, compared ignoring case: a taken one is `conflict`
  unless `replace`. The node writes `agents.toml` (0600) and the change
  is in effect for the next session; one breaking the file's rules is
  `invalid_request`, one `profiles.toml` would no longer read with is
  `conflict`. Never forwarded across the fleet.
- `agent.remove` (`admin`, as `agent.add`) `{ name }` → `Ack`;
  `not_found` for a name not defined (exact case); `conflict` while a
  profile extends it. Running sessions of the agent go on.
- An added agent's session is structured only (`session.create` with a
  terminal mode: `unsupported`), cannot move profiles or be exported or
  imported (`unsupported`), and starts in an environment of its own: the
  search path it was found on, the user's home, the system's variables
  listed in `vessel_harness::PASSED_ON` (a proxy only without a user and
  password in it), `inherit`, then `env`, a secret filled in from the
  node's `secrets.toml`. A secret the node lacks makes `session.create`,
  `session.resume` and `harness.options` a `conflict` naming it. It gets
  the `vessel` MCP server and, of the fleet's, only those its `mcp`
  names. `harness.list` lists one whose program is found, with no
  version (an unknown program is never run to ask).
- `agent.status` (`read`; `refresh` refused to an agent and to a call
  delivered from another node: `forbidden`) `{ agent?, profile?,
  refresh? }` → `AgentReadiness[]`, each `{ agent, profile, installed,
  signed_in, checked_at }`. Without `agent`, every agent's default
  profile, from what the node knows (`refresh` and `profile` then are
  `invalid_request`); with it, its `profile` (its default when absent;
  `not_found` for an agent or profile the node lacks, `invalid_request`
  for a profile of another agent).
  - `installed`, tagged by `state`: `found { path, version?, structured }`
    (an added agent's version is its handshake's `agentInfo.version`; it
    is never run with `--version`), `missing { program }`, `too_old` /
    `too_new { path, version?, speaks, needs }` (its handshake's ACP
    version), `unknown { reason }`.
  - `signed_in`, tagged by `state`: `yes`, `no { help: SignInHelp }`
    (`methods` are the `agent` sign-in methods its handshake listed),
    `not_required` (a routed profile), `unknown { reason }`, `reason`
    tagged by `reason`: `not_installed`, `not_checked`, `in_use`,
    `not_said`, `no_home`, `unread`, `timed_out { after }`, `failed {
    step }` (`spawn`, `initialize`, `session_new`, `exited`).
  - `refresh` runs, for Claude Code and Codex, the agent's own status
    command (`claude auth status`, read by its JSON `loggedIn`; `codex
    login status`, by its words) within 10 s, never beside a running
    session of that profile (`unknown { in_use }`), its output never
    logged or sent; for Gemini CLI, OpenCode and added agents, a handshake
    with no prompt (`harness.options`'s probe and its ten minutes):
    `auth_required` (-32000) to `session/new` is `no`, a completed one
    `unknown { not_said }`. A status command's answer is kept five
    minutes, a handshake's ten; a refresh within ten seconds of the last
    answers what was kept.
- `agent.catalogue` (`read`; `refresh` as `agent.status`'s) `{ refresh? }`
  → `Catalogue { fetched_at?, version?, entries, skipped }`: the node's
  copy of the ACP registry, empty until a person asks for a fetch. A
  fetch: https only (plain http only to a loopback address), no redirect,
  no proxy, 10 s to connect, 30 s in all, 2 MiB; at most 1,000 entries,
  each bounded, and one not pinned to an exact version is skipped and
  counted; a failure is `unavailable` and keeps the copy; a refresh
  within a minute of the last fetch answers the copy. Each
  `CatalogueEntry { id, name, version, description, website?,
  repository?, license?, runs }`, `runs` tagged by `kind`: `known {
  harness }` (one of the four by its npm package), `command { program,
  args, env, runner_found }` (`npx -y <package>@<version>` or `uvx
  <package>`, exactly what would be added), `download { archive,
  sha256? }` (never downloaded), `unavailable`. Not forwarded across the
  fleet.
- `agent.install` (`admin`, never an agent principal nor a delivered
  call: `forbidden`) `{ agent: HarnessKind, part: "harness" | "adapter" }`
  → `InstallJob { id: InstallId, agent, part, command, started_at }`: the
  vendor's documented command (the one `agent.list` shows), its program
  found on the node's search path, run as the node's user in the user's
  home with the environment an added agent gets plus `npm_config_*`,
  standard input closed. `invalid_request` for a part the agent does not
  have, `harness_not_installed` when the installer's program (`npm`) is
  missing, `conflict` while an install runs or the agent has live
  sessions. One at a time; 15 minutes, then stopped with its children.
  While it runs, `session.create`, `session.resume` and `harness.options`
  for that agent answer `conflict`.
- `agent.install_output` (`admin`) `{ job, from?, wait? }` →
  `InstallOutput { from, bytes, next, truncated, state }`, the shape of
  `worktree.output`: standard output and error as printed, the first 1 MiB
  kept, 64 KiB a page, `wait` holding the read (30 s at most) until there
  is more or it ends; `state` tagged by `state`: `running`, `succeeded`,
  `failed { code? }`, `timed_out`. `not_found` for a job that is not the
  last one (the node keeps only the last, until it stops). Neither call
  is forwarded across the fleet.
- `AgentToAdd::Registry { id, version, mcp }` (`from: "registry"`): the
  node builds the definition from its own copy (`not_found` when the
  entry is not there at that version; `invalid_request` when it runs no
  command here), with the entry's `env` as text and `source: { from:
  "registry", id, version }`.

### Schedules (track D)

Nine methods, all `since 11`, forwardable, in `vessel-web`'s list
(`docs/plans/backend-m11.md`, "Track D: contract" and its review; ADR
0033).

- Types (`vessel_proto::automation`):
  - `CronExpr`: five fields (minute, hour, day of the month, month, day of
    the week), lists, ranges, steps on `*` or a range, month and day names,
    `@hourly` to `@yearly`; parsed on decode into bit sets, so a bad one
    does not decode. `TimeZoneName` (an IANA name's shape; the node says
    whether it exists), `ScheduleName`, `PromptText` (16 KiB, no control
    character but tab and line ends), `EverySeconds` (60 to 31,622,400),
    `MaxRuns`, `RunNumber`.
  - `Cadence`: `every { seconds, anchor? }` (elapsed time from the anchor
    the node fills), `cron { expr, zone }`, `unknown`.
  - `ScheduleTarget`: `new_session { spec, after }` (a structured
    `SessionSpec`, whose agent may be one added on the node, track G;
    `after` is `archive`, the default, or `keep`) or
    `session { session }` (through its queue), `unknown`.
  - `ScheduleDefinition { name?, cadence, target, prompt, catch_up,
    max_runs?, ends_at? }`, `CatchUp` (`once`, `skip`), `ScheduleState`
    (`active { next_due }`, `paused { since }`, `ended { at, reason }`),
    `ScheduleInfo { id, definition, state, runs, last_due?, last_run?,
    defined_by, created_at, updated_at, revision }`.
  - `ScheduleRun { number, due_at, at, trigger, outcome }`; `RunTrigger`
    (`due`, `caught_up { missed }`, `asked { by }`); `RunOutcome`
    (`making`, `started`, `waiting`, `finished { session, end }`,
    `prompted { session, delivered }`, `skipped { reason }`, `failed {
    error: RunError, session? }`, `interrupted`). `RunError`'s code is a
    `RunErrorCode`, `ErrorCode` with `unknown`, so a list of runs stays
    readable to an older client when a newer code appears.
  - Every enum that will grow has `unknown`; one sent to a node is
    `invalid_request`.
- Methods: `schedule.list`, `.get`, `.runs` (read); `schedule.create`,
  `.update` (the whole definition, under its `revision`), `.pause`,
  `.resume`, `.run` (once now, answered with the run's outcome; after 30
  s with no room, `unavailable`), `.delete` (operate).
- `invalid_request` besides the contract's: a new session's spec over 16
  KiB encoded (as a saved prompt's), and an `after` the node does not
  know (it would neither archive the session nor wait for it).
- Room: at most 4 runs are made at once and at most 8 scheduled sessions
  are in their first turn and not waiting for a person; runs wait, the
  oldest due first, then by id, the first that fits going first.
- A run acts as the schedule's `defined_by` as they are when it runs (a
  revoked device ends it; an agent's schedules go with its session, and
  with its run when the session is started again), and meets the checks
  a call of theirs would. Agents: 8 schedules per session, their own
  session as a heartbeat's target, never `run` now, an `every` anchored
  by the node; delegated callers may only read, pause, resume and
  delete. A run's session is archived after its first turn as a person's
  archive does it: its terminals and services end, its pull request is
  no longer watched.
- What happens at a daylight-saving gap or fold, after a stopped node, on
  a clock stepped either way, and to a run under way when the node stops,
  is the contract's "Time" and ADR 0033.
- Store: schema 12 (`schedules`, `schedule_runs`, `saved_prompts`, `hooks`,
  `hook_deliveries`, `hook_keys`).

### Saved prompts (track D)

Four methods, all `since 11`, forwardable, in `vessel-web`'s list (ADR
0033).

- `SavedPrompt { id, name, text, project?, session?, defined_by,
  created_at, updated_at, revision }`: `PromptName` (64 characters),
  `PromptText`; `project` absent is the machine's; `session` a structured
  `SessionSpec`, what a session started from it runs with, its place (if
  any) in the prompt's project.
- Methods: `prompt.list { project? }` (read; that project's and the
  machine's, `not_found` for an unknown project; by name, the machine's
  first of one name); `prompt.create`, `prompt.update` (name, text and
  `session` under its `revision`; the project stays), `prompt.delete`
  (operate).
- A name is unique within its project (or the machine's): `conflict`, as
  for an old revision and past 256 on the node. `session` is checked as
  `session.create` would check it (`invalid_request` for a terminal or
  resuming spec, or a place outside the prompt's project).
- A person's text: agents and delegated callers may list, not write
  (`forbidden`). Saved prompts are kept by the node, never read from a
  repository's files (ADR 0033 says why).

### The starter told (track D)

- `PromptSession.notify: bool` (left out when false; a call with it true
  needs 11). Only a session's agent asks, on its own node
  (`invalid_request` for the owner and devices, `forbidden` through
  `fleet.forward`, `unsupported` for a terminal caller). The session told
  is the caller's own.
- It is about the turn this prompt starts or joins, exactly: the
  session's actor says which (at once when it starts or steers, when its
  queue entry starts a turn when queued; never for an entry removed).
  Each event is told once, as a prompt queued into the caller's session
  (`delivery: queue`: never into a running turn): `needs_you` for each
  permission or question of that turn (at most 8), then `finished` (its
  end, the last 1,500 characters of its reply, and the record to read
  from) or `ended` (the session's final status). Text only, from the
  node, starting `vessel: session`; the queue entry's `by` is the
  finished session (`Creator::Agent`). The session's own words in it
  (its reply's end, what it asks, why its turn failed) are between
  `<vessel-session-output-…>` tags whose name is random, with a line
  saying they are that session's output, not instructions.
- Bounded: 32 turns followed per told session and 256 on the node
  (`conflict` beyond), 60 notices an hour into one session (the rest
  dropped and logged), none into a session a person stopped. Nothing is
  stored.

## Version 10 (2026-11-11, the two tracks joined 2026-11-12)

Nodes speak 2..=10; version 10 adds fifty methods, six events (with
stand-ins for older clients) and optional fields, in two parts built side
by side in milestone 11 (`docs/plans/backend-m11.md`): the conversation
("Track A: contract" and its review; ADRs 0024 to 0026) and projects,
worktrees, git and files ("Track B: contract", which has every call's
failures and bounds; ADRs 0023 and 0027). Where the two meet is the
plan's "Integration (node)".

### The conversation

Ten of the methods, the six events and most of the optional fields.

- Types (`vessel_proto::conversation`):
  - `AgentAbilities { steer, resume, image, audio }`: what the agent's
    ACP `initialize` answer says (`_meta.steering.supported`,
    `loadSession`, `promptCapabilities`). In `AgentEvent::Ready.abilities?`,
    `SessionInfo.abilities?` (kept by the store) and
    `HarnessOptions.abilities?`.
  - `ContentBlock::Attachment { attachment: AttachmentRef }`, `AttachmentRef
    { id, name, kind, mime, size }`, `kind` `image`, `audio` or `file`:
    only in what a client sends and its echo; the node replaces a
    client's reference with its own record by id.
  - The queue: `PromptQueue { entries, held? }`, `QueuedPrompt { id,
    content, queued_at, revision, by }`, `QueueHold` (`cancelled`,
    `failed`, `ended`, `asked`); `Delivery` (`start`, `queue`, `steer`,
    `interrupt`) and `Delivered` (`started`, `steered`, `queued { entry
    }`); `QueueChange` (`edit { entry, revision, content }`, `move {
    entry, before? }`, `remove`, `send_now { entry, delivery }`, `hold`,
    `resume`, `clear`).
  - `AgentCommand { name, description, hint? }`, `CommandName` (no `/`,
    no white space).
  - Questions: `Question { id, message, tool_call?, fields }`, each field
    `text`, `number`, `integer`, `boolean`, `choice` or `unknown`;
    `QuestionAnswer` (`accept { values }`, `decline`); `QuestionOutcome`
    (`answered`, `declined`, `cancelled`).
  - `ContextUsage { used, size, cost? }`, `TurnUsage { total, input,
    output, thought?, cached_read?, cached_write? }` in
    `AgentEvent::TurnEnded.usage?`.
  - `SessionMarks { pinned, archived_at?, unread }` in `SessionInfo.marks`
    (left out when nothing is marked); `Mark` (`pin`, `unpin`, `archive`,
    `unarchive`, `read { through }`, `unread`).
  - `AttachmentLimits` (in `Conversation`), and `Conversation { queue,
    commands, usage?, permissions, questions, abilities?, limits }`.
- Input: `SessionInput::AnswerQuestion { id, answer }`; `SessionInput::
  CancelTurn { turn? }`, the cancel naming the `turn_started` it means.
- Events, each with the stand-in a version 2-9 client gets at the same
  `seq`:
  - `queue_changed { queue }`: `vessel: queue: N waiting (held: why)`;
  - `commands_changed { commands }`: `vessel: commands: /a, /b and N
    more`;
  - `usage_changed { usage }`: `vessel: context: U of S tokens`;
  - `agent { steered { content } }`: `vessel: steered: <first line>`;
  - `agent { question_asked }`: `vessel: the agent asks: <message>`;
  - `agent { question_resolved }`: `vessel: question answered`
    (`declined`, `cancelled`);
  - a `turn_started` naming an attachment: the same event, each
    attachment a `resource_link` to `attachment:<id>`.
  An agent event's version is read from its content as well as its kind
  (`AgentEvent::since`).
- Methods, all forwardable: `session.prompt` (operate), `session.queue`
  (operate), `session.conversation` (read), `session.resume` (operate),
  `session.mark` (operate), `session.delete` (operate),
  `attachment.begin`, `attachment.write`, `attachment.finish` (operate),
  `attachment.read` (read). The `operate` ones that name a session are
  ownership-checked for agents; `attachment.write` and `attachment.finish`
  belong to the caller that began the upload. Their failures and bounds
  are the plan's contract.
- A version 10 call, an `answer_question`, a prompt naming an attachment,
  or a cancel naming its turn needs protocol 10 (`Call::since`).
- What older peers get:
  - Every reply is rewritten for the connection's version
    (`Reply::for_protocol`), the fleet envelope's nested reply included:
    before this, a `session.read` page passed back through
    `fleet.forward`, `fleet.relay` or `fleet.deliver` reached an older
    home unchanged.
  - Ended sessions: from this version, an ended session that is not
    archived is in `building` as `exited`, and only an archived one is in
    `archive` (ADR 0026). A version 8-9 peer still gets every ended
    session in the archive (`SessionInfo::for_protocol`).
- Logs: a session's final `status` is the last record of a run, no longer
  of the log: `session.resume` continues the same log after it.
- What each call does (the contract's details that the summary above
  leaves out): `session.conversation`, `session.prompt` and `session.queue`,
  `cancel_turn`'s `turn`, and questions: `question_asked`,
  `question_resolved` and `session.send`'s `answer_question`
  (`invalid_request` for an answer that does not fit the form,
  `not_found` for an unknown question, `conflict` once it is settled),
  offered only to the agent of a session the owner or a device created;
  marks and deletion: `SessionInfo.marks` everywhere a session's info is
  read, `session.mark` (`archive` stops a live session and answers once it
  has ended; a `read` past the end of the log is `invalid_request`; an
  agent marking `read` or `unread` is `forbidden`), `session.delete`
  (`conflict` for a live session and for one whose log is still being
  written; its answer, below), the board's placement of ended sessions,
  and retention's order (archived first, pinned last); starting again: `session.resume`
  (the row reopened before the new run writes, the log continued after
  the final status, the title, choices and creator kept, the queue back
  held `ended`, out of the archive, a new agent credential; it answers
  once the agent is ready, `spawn_failed` with the reason the log ends
  with otherwise), and a subscription the store was serving ends
  `lagged` when its session started again meanwhile; attachments:
  `attachment.begin`, `.write`, `.finish` and `.read` (an upload is its
  caller's, `not_found` to anyone else; an image or audio only to an
  agent that said it takes them, `unsupported` otherwise and while it has
  not said; a piece that ends at or before what the node has is answered
  with it, one that starts elsewhere is `invalid_request`; an image that
  does not start as its type does is `invalid_request` and its upload
  dropped; `conflict` once the session ended, and for a kept file that
  was changed on disk; uploads open at a restart are dropped), and every
  prompt naming one (`session.prompt`, a queue `edit`, `session.send`, and
  a queued prompt when it is sent; a queue `edit` with no content is
  `invalid_request`, as an empty prompt is) checked against the session's own
  records, the per-prompt bounds and what its agent takes then
  (`invalid_request`); and every call from the command line (`vessel send
  --queue|--steer|--interrupt|--attach`, `vessel session
  queue|answer|resume|pin|unpin|archive|unarchive|read|unread|delete`)
  and, but marks other than archiving, deletion and attachments, from the
  agent tools (`send_input`'s `delivery`, `read_conversation`,
  `change_queue`, `answer_question`, `resume_session`,
  `archive_session`). A `vessel send` without a delivery or a file still
  makes `session.send`, so it works with an older node.

### Projects, worktrees, git and files

Forty of the methods, two optional fields and no event.

- Types (`vessel_proto::workspace`, one module per area): `ProjectId`,
  `WorktreeId`; `RelPath` (relative, `/`-separated, never absolute,
  climbing, a drive, or any `.git` by git's `verify_path` rules: a bad
  one does not decode); `CloneUrl` (`https`, `http`, `ssh`, `git` or
  `user@host:path`; no `<transport>::`, leading `-` or credentials);
  bounded names (`BranchName`, `RefName`, `RemoteName`, `CommitId`,
  `ProjectName`, `FolderName`, `CommitMessage`, `ProjectCommand`,
  `CommandsDigest`, `ContentHash`, `FindQuery`, `GrepQuery`); every reply
  enum that will grow has `unknown`.
- `SessionSpec.place?`: `project { project, dir? }` or `worktree {
  worktree }`. The node fills `cwd` with the folder it resolves to (so a
  protocol 9 client still sees where the session runs), refuses a place
  together with a `cwd`, and starts the log with `vessel:` notices (the
  folder, the branch and its base, setup's result and, after a failure,
  its last lines). In a worktree whose setup runs: `conflict`. A
  `session.create` with a place needs protocol 10 (`Call::since`). The
  first session started in a worktree made for no session claims it.
  `SessionInfo.workspace? { project, worktree?, branch? }`, filled from
  the place on every answer.
- Methods, all forwardable, in `vessel-web`'s list; agents may call only
  `project.list`, `project.get`, `project.commands`, `worktree.list`,
  `worktree.create` (at most 8 they made), `worktree.output`, and
  `session.create` in a project or a worktree they made (or made for a
  session they may operate; a `cwd` of their own inside the node's data
  folder, where worktrees are made, is `forbidden`: they name the
  `place`); every other one answers them `forbidden`. Creations at once
  count against the 8 together.
  - Projects: `project.list` (read; with the git found: `found {
    version }`, `too_old`, `missing`), `project.get { project,
    changed_from? }` (read; a long poll until the revision differs, at
    most 30 s), `project.add { path, name? }`, `project.clone { url,
    parent, folder?, name? }` (answers `cloning`; progress in `state`),
    `project.init { path, name? }`, `project.remove` (forgets; files
    stay), `project.commands { project, base? }` (read),
    `project.approve { project, digest, approved }` (all operate).
    Roots refused: a file system's root, any folder that is or holds the
    home folder, any folder holding the node's data folder or inside it;
    `project.init` checks the folder it would make before it makes
    anything (`.` and `..` refused). A project's `vessel.toml` is read
    only as a regular file inside its folder, at most 64 KiB; one that
    leads out of it is as if there were none.
  - Worktrees: `worktree.list { project? }`, `worktree.create { project,
    branch: new { name?, base? } | existing { name } | snapshot {
    worktree, name? }, setup: run | skip }` answering `created`,
    `needs_approval { commands }` or `refused { reason }`;
    `worktree.output { worktree, from, wait? }` (a page of at most
    64 KiB of setup's output, a long poll); `worktree.setup`;
    `worktree.remove { worktree, if_lost: keep | snapshot | discard }`
    answering `removed { snapshot? }` or `kept { reasons }`;
    `worktree.snapshots { project }`, `worktree.drop_snapshot`.
  - Git (`root`: `project { project }` or `worktree { worktree }`):
    `git.status`, `git.changes { scope, ignore_whitespace }` (scopes
    `unstaged`, `staged`, `uncommitted`, `base { base? }`, `commit {
    id }`), `git.diff` (one file's typed hunks, paged by hunk),
    `git.show` (a file at a commit, paged), `git.branches`, `git.log`,
    `git.stashes` (read); `git.stage`, `git.unstage`, `git.discard`,
    `git.commit`, `git.fetch`, `git.push`, `git.pull` (fast-forward
    only), `git.switch`, `git.rename_branch`, `git.stash` (operate),
    each with typed refusals (`GitRefusal`). A call's wait for one of
    the node's git slots counts against its time (`timed_out`). Every
    call checks its root again: a folder replaced since it was recorded
    (a link to another), or a worktree whose `.git` no longer names its
    repository's record of it, is `conflict`, and git is not run in it.
  - Files: `file.list`, `file.read`, `file.find { query, key? }` (by
    name; a newer search with the same key replaces a running one),
    `file.grep` (read); `file.write { expect: absent | version | any }`
    (protocol 11 replaces `version` with `content { hash }`, below),
    `file.create_dir`, `file.rename`, `file.delete` (operate). A path
    that leads out of the root, one that cannot be followed (a link to
    nowhere, a file where a folder should be, no permission) and one
    that is not there all answer `not_found`; what is read is opened
    first and judged a regular file on the opened file (a FIFO is not
    waited on).
- Store schema 9 (`projects`, `worktrees`, `approvals`).

### Where the two meet

- `session.delete` answers `SessionDeleted { worktree? }` (track A's
  branch answered `Ack`; nothing of protocol 10 was released, so the
  reply changed in place): `worktree` is `TidiedWorktree { worktree,
  outcome }` for the worktree made for the session (`WorktreeInfo.
  created_for`), `outcome` `removed` or `kept { reasons }`. The session
  is deleted either way; the worktree goes only by the rule a person's
  `worktree.remove { if_lost: keep }` follows (ADR 0027), so a delete
  never loses uncommitted work or commits no other branch has, never
  snapshots or discards unasked, and says when it kept something (also
  in `WorktreeInfo.kept`).
- Retention dropping a session tidies its worktree by the same rule, in
  the background after the prune; the node also tidies, when it starts,
  every worktree whose session is gone. Ending or archiving a session
  tidies nothing: `session.resume` starts it again in the same folder.
- `session.resume` of a session whose folder is gone (a worktree a
  person removed, or one tidied with the session it was made for, or a
  folder removed by hand) is `conflict`, naming the folder; so is one
  into a worktree whose setup, teardown or removal runs. Nothing of the
  session changes.
- Tidying keeps a worktree git could not inspect (`kept {
  inspection_failed }`) and does not try it again; a person removes it
  with `snapshot` or `discard`. One kept because a session was in it is
  tried again once that session is gone.
- `SessionInfo.workspace` is filled on every answer that carries a
  session's info: `session.list`, `.get`, `.create` (and an import's),
  `.move_profile`, `.rename`, `.subscribe`, `.resume` and `.mark`.
- An imported session's place is dropped (it named the other machine's
  project); its title comes with it, its marks and queue stay behind.
- Store: schemas 7 and 8 are the conversation's, 9 the workspace's.

## Version 9 (2026-11-11)

Nodes speak 2..=9; version 9 adds four methods, two events (with
stand-ins for older clients) and optional fields. Milestone 10's plan
(`docs/plans/backend-m10.md`, "Contract (protocol 9)", track C) and ADR
0022.

- Types (`vessel_proto::options`): the agent's own session config
  options, passed on as the agent reports them; Vessel writes no list of
  models, efforts or modes of its own.
  - `OptionId`, `OptionValueId`: the agent's ids, opaque, echoed back.
  - `OptionCategory`: `mode`, `model`, `model_config`, `thought_level`,
    `other` (ACP's categories; none, a custom one, and one a newer node
    sends read as `other`).
  - `SessionOption { id, name, description?, category, kind, .. }`,
    `kind` one of `select { current, values }` (each value `{ id, name,
    description?, group? }`, in the agent's order; `group` is the name of
    the group the agent files it under), `toggle { on }`, or `unknown`: a
    kind a newer node sends, which a client skips (without it, one such
    option would fail the decode of the whole list).
  - An agent with only ACP's older `modes` (and no option in the `mode`
    category) gets a `mode` option made from them (`session_mode` if the
    agent uses that id), set with `session/set_mode`.
  - `OptionChoice { id, kind, .. }`: what a client asks for, `select {
    value }` or `toggle { on }`.
- `SessionTitle`: a string on the wire, parsed: trimmed, not empty, one
  line without control characters, at most 120 characters. A bad one does
  not decode. A title taken from a prompt is its first non-empty line, cut
  at 80 characters.
- `SessionMode::Structured.options?: OptionChoice[]`: set in order once
  the session exists (after `session/new` or `session/load`), after
  `model`; one the agent does not offer, or refuses, fails the session
  (`failed`, the reason in its message), as an unknown model does.
- `SessionSpec.title?`: the title asked for at creation.
  `SessionInfo.title?`: the session's title now, the one truth for it:
  the spec's, otherwise (structured sessions) the first line of its first
  prompt with text, then whatever `session.rename` gave it last. The node
  keeps it across restarts. Absent until it has one, and from older nodes.
- A `session.create` choosing options or naming a title needs protocol 9
  (`Call::since`): an older node would start the session in the agent's
  default mode, or unnamed, rather than refuse.
- Events:
  - `options_changed { options }`: the whole list. Every structured
    session logs one right before its `ready`, as the options stand after
    the handshake's choices, empty when the agent offers none (so
    "nothing offered" is told from "not known yet"); then one whenever
    they change (a `session.set_option`, or the agent's own report of a
    change, also from the agent a profile move restarts).
  - `renamed { title }`: from the first prompt or `session.rename`.
  - A version 2-8 client gets a `harness_log` line in their place:
    `vessel: options: Model Haiku, Mode plan, Fast mode off` (each
    option's name and its current value's name, or its id when the agent
    does not list it; options of an unknown kind left out; `none` for no
    options), and `vessel: renamed to <title>`.
- Methods, all forwardable:
  - `harness.options { harness, profile? }` (read) returns `{ options,
    probed_at }`: what a harness's agent offers before any session of it
    exists. The node starts the agent under the profile (no prompt is
    sent), reads the handshake, and stops it; the answer is kept ten
    minutes, or until the harness reports another version, and callers
    asking meanwhile share one probe. A harness that cannot run
    structured sessions on the node answers an empty list. Errors:
    `harness_not_installed`; `spawn_failed` when the agent does not start
    or complete its handshake (not signed in, for example; the message
    has the agent's last lines), within the handshake timeout;
    `invalid_request` for an unknown profile or one of another harness.
  - `session.options { session }` (read) returns `SessionOption[]` as the
    agent last reported them, waiting for the handshake of a starting
    session. `unsupported` for a terminal session, `conflict` for an
    ended one (its log's last `options_changed` is what it ran with).
  - `session.set_option { session, choice }` (operate, ownership-checked
    like `session.send`) returns `SessionOption[]` after the change, while
    idle or mid-turn as the agent allows; a value in effect changes
    nothing. `invalid_request` for a choice the agent does not offer
    (nothing is sent to it), `conflict` for the agent's refusal (its
    message in the error's), `unsupported` for a terminal session.
  - `session.rename { session, title }` (operate, ownership-checked)
    returns the `SessionInfo`; any live session, terminal ones included.
    `conflict` for an ended one.
- Sessions that continue elsewhere keep what they run with:
  `TransferDetail::Session.options?` (`session.export`) carries the
  session's choices as they stand (its last report's values; before any,
  its spec's), and `session.import` applies them after the load, failing
  where the agent there does not offer one; the title travels in the
  exported info and is kept. A profile move applies them to the restarted
  agent likewise.

## Version 8 (2026-11-11)

Nodes speak 2..=8; version 8 only adds one optional field, so there is
no version gate: a version 2-7 client ignores it, and a version 8 client
reading an older node finds it absent. Milestone 9's plan
(`docs/plans/backend-m9.md`, track A) and ADR 0019.

- `SessionInfo.board?`: a session's place on the board, derived by the
  node and never sent by a client: `{ column, display_status, activity,
  last_activity_at }`.
  - `column`: `building`, `validating`, `needs_review`, `ready` or
    `archive`. `archive` is not a lane: clients list those sessions
    apart.
  - `display_status`: the phrase for the card, derived inside `column`:
    `working`, `blocked`, `exited`, `no_signal`, `interrupted`,
    `awaiting_pr`, `idle` (building); `fixing_ci`, `addressing_comments`,
    `needs_review`, `review_scheduled`, `reviewing`, `review_failed`,
    `review_pending`, `draft` (validating); `ci_failing`, `commented`,
    `changes_requested`, `needs_human_review` (in review); `mergeable`,
    `approved`, `merged`, `closed` (ready); `terminated` (archive).
  - `activity`: `active`, `idle`, `waiting_input` or `exited`, what the
    agent was last seen doing.
  - `last_activity_at`: when the node last saw the agent do something:
    the handshake completing, a turn's start or end, a permission asked
    or answered, terminal output, the session's end.
- The model is agent-orchestrator's (`contract/kanban.go`), ported
  (`vessel_session::board::derive`): the column first, from lifecycle
  facts, then the phrase from what that column cares about.
- What a node reports today:
  - A live structured session is `building`: `working` while starting
    and during a turn, `blocked` while a permission request waits,
    `interrupted` after a cancelled turn until the next one starts, and
    otherwise `idle`.
  - A live terminal session is `building`: `working` while it prints,
    `idle` once it has printed nothing for five seconds.
  - Any ended session (`exited`, `failed` or `interrupted`, live in this
    run or read back from the store) is `archive` / `terminated`, activity
    `exited`, its end as its last activity. The client shows the status
    word on an archived card. (Version 10 changes this: an ended session
    is `building` / `exited` until it is archived; a version 8-9 peer
    still gets the placement above.)
  - No node gathers pull requests yet, so `validating`, `needs_review`,
    `ready`, `awaiting_pr` and the phrases of those columns are not sent;
    `no_signal` is not either (no Vessel session is expected to report on
    its own). They are in the contract so the next milestone adds facts,
    not wire types.
- Nothing new is pushed: `board` comes with every `SessionInfo` a node
  already returns (`session.list`, `session.get`, `session.create`,
  `session.move_profile`), current as of the call. Inside the node the
  session's info watch wakes on a change of column, phrase or activity;
  `last_activity_at` alone moves without waking it, as `next_seq` does.
- A client talking to a node older than protocol 8 gets no `board` and
  keeps the placement the session's status implies.

## Version 7 (2026-11-07)

Nodes speak 2..=7; version 7 only adds: the LSP contract (milestone 6)
and model routing (milestone 5), each appending its rows to the RPC
table with `since 7`.

### LSP contract (milestone 6)

The M6 LSP rows append to the RPC table with `since 7`:
`lsp.workspace_symbols`, `lsp.document_symbols`, `lsp.definition`,
`lsp.references`, `lsp.hover`, `lsp.implementations`, `lsp.type_definition`,
`lsp.incoming_calls`, `lsp.outgoing_calls`, `lsp.diagnostics`. All need Read
and are forwardable under the existing scope intersection. The runtime serves the implemented capability subset and returns
`unsupported` when the selected server does not advertise a feature.

Each request selects `workspace: {root, language}`. Languages are `rust`,
`type_script`, `python`, `go`, `cpp`. Files are relative paths without
traversal. Positions use 1-based lines and Unicode scalar columns; ranges
have exclusive ends. Page requests default to offset 0, limit 50; the node
caps limits at 200. Pages are sorted but may shift when files change.

Responses carry context: covered files, optional observation time (the oldest contributing cached report), freshness
(`saved`, `cached`, `unknown`), indexing (`unknown`, `busy`, `observed_idle`)
and explicit workspace completeness. An empty diagnostics cache is unknown,
not clean. Hover is bounded untrusted plaintext/Markdown; clients disable
raw HTML. Server JSON never appears in this contract. ADR 0016 and the M6
plan define confinement and the operator-only full-analysis opt-in.

### Model routing (milestone 5)

`docs/plans/backend-m5.md`, ADR 0015. `model.list` follows the LSP rows
in the RPC table.

- Types: `ModelRouteName` and `ProviderName` (a route's and a provider's
  name in a node's `models.toml`: 1-64 letters, digits, `.`, `_` or `-`);
  `ModelApi`: `anthropic_messages`, `openai_responses` or
  `openai_chat_completions`; `TokenUsage { input, cache_read, cache_write,
  output, reasoning }`, each a count of tokens, every field defaulting to
  0, where `input` excludes cached tokens and `reasoning` is part of
  `output` (each API is converted to this at the node's proxy);
  `ModelStop`: `end_turn`, `tool_use`, `max_tokens`, `stop_sequence`,
  `refusal`, `pause` or `error`; `ModelFailure`: `rate_limited`,
  `overloaded`, `upstream_error`, `refused_request`, `context_too_long`,
  `timeout`, `truncated` or `unauthorized`, the class of a failure and
  never the provider's message.
- `ProfileInfo.model?`: the route a profile names (`model = "<route>"` in
  `profiles.toml`); `SessionInfo.model?`: `{ route, model }`, the route a
  session was launched with and the provider's model its harness is pinned
  to. Both optional and absent from older nodes, so no version gate.
- Event `model_call { route, model, api, usage, stop, failure?,
  duration_ms }`: one per call a session's harness made through the node's
  proxy, metadata only (never prompt or completion content, tool arguments
  or keys). A version 2-6 client gets a `harness_log` line in its place at
  the same position: `vessel: model call to <model>: <in> in, <out> out`,
  `<in>` counting cached input, or `vessel: model call to <model> failed
  (<failure>)`.
- `model.list` (read, forwardable) returns `ModelRoute[]`: `{ name,
  provider, api, model, small_model? }`, the routes the node offers. No base
  URL, header or secret name crosses the wire. There is no usage row:
  usage is in each session's log, and clients sum it.
- `session.create` under a profile naming a route the node's
  `models.toml` does not have: `invalid_request`, the route named.
  `session.move_profile` keeps a session's route: a move to a profile of
  another route, or between a route and none, is `unsupported`.

## Version 6 (2026-11-06)

Nodes speak 2..=6; version 6 only adds. Milestone 4's F3, F4 and Track X
implement it (`docs/plans/backend-m4.md`, "Protocol 6 (accepted)"), all
built: the envelope, `fleet.grant`, `fleet.callback`, `fleet.forget`,
`fleet.peers`, `fleet.nodes`, `session.read` with `wait`, the transfers
and the secret rows. (Until each landed, its methods answered
`unsupported`, "not built on this node yet", after the scope check like
every method.)

- Scope `relay`: passing calls on to other fleet nodes. Independent of
  `sync`, and a member gets it only when the home's owner asks (`vessel
  fleet invite --relay`); `vessel fleet invite` grants `sync` alone, never
  `admin`. `device.invite` naming `relay` needs protocol 6 (`Call::since`),
  and `device.list` leaves `relay` out for version 2-5 clients.
- Envelope (Track X): `fleet.forward { node, call }` (read) asks the
  caller's own node to run `call` on fleet node `node`; `fleet.relay {
  node, call, origin }` (relay) is a member asking its home to pass it on,
  refused when `origin.node` is not the member's own node; `fleet.deliver
  { call, origin }` (read) is the home having a member run it, the last
  hop, accepted only from that member's home. All three return
  `Forwarded { reply }`, the inner call's reply; an inner call that fails
  fails the envelope with its error. `origin` is `{ node, by, scopes }`,
  `by` one of `{"kind":"owner"}`, `{"kind":"device","id"}`,
  `{"kind":"agent","session","depth"}`. The target runs the inner call
  with its delivering device's scopes intersected with `origin.scopes`,
  and applies ownership and the agent limits to the origin. Subscriptions
  and the envelope's own methods cannot be forwarded
  (`Method::forwardable`). Each connection must understand the envelope
  (at least protocol 6) and the full inner call, including newer fields:
  `Call::since` recursively takes their maximum. Both client and server
  check this before sending or dispatching, including a forward to the
  welcomed node itself. Thus the protocol 22 inbox cannot cross a hop
  negotiated at 21, even when that hop is also the final target.
- Fleet: `fleet.grant { link }` (sync) hands the home a pairing link made
  on the member, so the home holds a credential there (a member's node
  sends it when it connects, with the scopes chosen at join, `read` and
  `operate` by default; the device the home's redemption creates is the
  only one the member takes `fleet.deliver` from, and leaving revokes it).
  A member grants once per join: if its owner revokes the home's device,
  the home stays cut off until the node joins again.
  The home keeps a record of every node id it bound to a key and a name,
  and of the node every member device granted for, revoked and replaced
  devices included, and answers `conflict` to a grant that would rebind a
  device, bind an id to another key or a key to another id, rename a
  node, name the home's own id, or use a name the home or another member
  holds (also while that member's device is revoked). The same id and key
  through another device is the node joined again, and replaces the
  older grant. An id goes to a new key, or a node to a new name, only
  after `fleet.forget { node }` (admin, not forwardable; `vessel fleet
  forget <node>` against the home) and then only for a member device
  paired after it: the call withdraws the invitations outstanding on the
  home, a pairing that redeemed one before and completes after is
  revoked, and it returns `NodeForgotten { key, name?, devices,
  withdrawn }`. Any member device paired after it may then claim the id
  and name, and the first claim binds them, so the owner invites the node
  again before anything else and it joins from there. A
  refusal over a revoked or forgotten node says only that the name is
  reserved for, or the key bound to, another node, without ids or times;
  one over a live member's name names that member; the home's log has the
  details. `conflict` also while the same device has a grant in
  progress; `invalid_request` to a link with no route or more than 8; and
  `unavailable`, without details, when it cannot pair (within 30 s).
  `fleet.callback { node, key }` (sync; from a member device only, never
  forwarded) returns `Callback { relays, ticket? }`: the relays the home
  registers with and a registration ticket for `key` (`RelayTicket { key,
  expires_at, mac }`, never the relays' token; none when the home has no
  relay). A device the home's record binds to `node` and `key`, holding a
  current grant, gets a ticket for a day. One that has not granted yet gets
  one for ten minutes, only within an hour of its pairing (never reaching
  past that), only for a claim the record would take, and for one node and
  key, which the home records and its grant must then claim (`conflict`
  otherwise); others get `forbidden`. A member whose node has no route of
  its own asks on each connection, and again halfway to the ticket's
  expiry while connected, registers with those of the relays it dials (at
  most 8: any on a host it reaches its home by; otherwise `wss://`, not
  `localhost` nor a non-canonical numeric host, and only at public
  addresses its host resolves to when dialed) and names them, and only
  them, in its grant's link; another home's callback, or leaving, unlinks
  them. With none, it sends no grant and records why
  in its fleet state. `fleet.peers` (sync) and `fleet.nodes` (read)
  return `FleetNode[]` (`node`, `name`, `protocol`, `reachable`), the
  home included.
- Transfers (F4): `session.export { session, force? }` (operate,
  ownership-checked for agents) and `memory.export { profile?, project? }`
  (admin) snapshot files into a transfer and return `TransferManifest {
  transfer, files, detail }`, `files` in the bundle's shape (path to
  `{hash, size, executable?}`), `detail` `{"kind":"session",
  "harness_session", "cwd", "info"}` or `{"kind":"memory", "harness",
  "profile"?, "project"?}`. `transfer.read { transfer, hash, offset }`
  (read) returns a `BlobChunk` as `fleet.blob` does; `transfer.close {
  transfer }` (read) drops it; only the caller that opened a transfer may
  read or close it, and transfers expire after 10 idle minutes and at
  restart. `session.import { from: {node, session}, profile?, cwd?, start?
  }` (operate) pulls a session here and returns `SessionImported {
  harness_session, profile?, cwd, written, session? }`; `memory.import {
  from: {node, profile?, project?}, profile?, project? }` (admin) returns
  `MemoryImported { written, same, beside? }`, conflicting files written
  beside the existing ones. Limits: 256 MiB per file and 2 GiB per
  transfer (`MAX_TRANSFER_FILE`, `MAX_TRANSFER_TOTAL`); a transfer over
  either is `invalid_request`, naming the limit and the size.
  As built (F4): an import from another fleet node sends the source's
  `session.export` or `memory.export`, `transfer.read` and
  `transfer.close` through the envelope as the importing caller, so the
  source's grant to the home must cover them (`admin` for memory) and its
  transfer belongs to that delivery; a node in no fleet answers
  `not_found`. A profile with no harness home on the target yet is
  `conflict`, before anything is fetched. A `TransferId` is a UUIDv4;
  the holder check, not the id, keeps a transfer to its opener (another
  caller gets `not_found`, as for an unknown id). At most 16 transfers
  are open on a node (`unavailable` beyond). `session.export` of a
  terminal session is `unsupported`, of one whose agent is still starting
  `conflict`; `force` copies the transcript as far as it is written,
  cancelling nothing. `memory.export` without `profile` is Claude Code's
  default profile. `session.import` refuses (`conflict`) the profile and
  directory the session already is in on the same node, and a different
  file in the way, before writing anything; an unknown target profile is
  `invalid_request` before the source is asked; `start` needs the
  directory to exist (`invalid_request`). `memory.import` with `project`
  needs `from.project`.
- Secrets (F3), all admin: `secret.set { name, value }`, `secret.list`
  (names only), `secret.delete { name }`, `secret.copy { name, to }`. There
  is no `secret.get`: a value leaves a node only into another node's store.
  `secret.copy` sends `secret.set` to `to` through the envelope, so it
  needs the target's owner to have granted the home `admin` there.
  As built: `secret.copy` of a secret not set here is `not_found`, to this
  node itself `invalid_request`; a target that did not grant the home
  `admin` is `forbidden`, the message saying how to (`vessel fleet join
  --grant read,operate,admin` there), and the home itself, which grants
  its members only `sync` and `relay`, is `forbidden` with a pointer to
  `vessel fleet secret set` there. No error carries a value. The target
  is asked `secret.list` through the envelope first (forwardable for
  this, names only), and `secret.set` carrying the value goes only if that
  passes, so a target that would refuse the value never receives it.
- `session.read` takes `wait?` (seconds, capped at 60, protocol 6): with
  nothing at `from` yet, the node answers as soon as a record lands there
  or the session ends, or when the wait ends (an empty page). An ended or
  stored session answers at once. Waiting reads may hold at most half of a
  connection's calls (32 of 64); one origin at most 8 on a node; all
  origins of one other node at most 16 there; and on the home, the reads
  one relaying member device passes on at most 16, whatever origins they
  name. A read past any of these answers at once, as without `wait` (passed on to
  another node, it goes without its wait). A client that gets an empty
  page before its wait could have ended should pause before asking again.
- `SessionInfo` takes `via?` (the home a creating call came through) and
  `origin?`: `{"kind":"remote","node","by"}` (`by` owner or `{"kind":
  "device","id"}`) for a session created by a forwarded call, `{"kind":
  "imported","node","session"}` for an imported one. Absent for a session
  of this node's own, so older clients see nothing new; `created_by` uses
  only the variants protocol 5 has.

- Relay registration and limits (the relay's own wire, not negotiated;
  2026-11-06):
  - `register` may carry `ticket {key, expires_at, mac}` instead of
    `token` (exactly one of the two, else `rejected {reason: "token"}`):
    HMAC-SHA256 under the relay's token over `"vessel relay ticket
    v1\0"`, the key's 32 bytes and `expires_at` (seconds since the Unix
    epoch, 8 bytes big-endian). A fleet's home mints one for a member's
    key (`fleet.callback`). The relay takes it only for the key that then
    proves possession, unexpired and expiring at most 48 h ahead
    (`rejected {reason: "ticket"}` otherwise), and closes a registration
    made with it when it expires; the node registers again with a newer
    one.
  - A relay holding its most nodes answers a registration `rejected
    {reason: "full"}` (after the token or ticket and the proof); a node
    registered with a ticket holds a place like any other and frees it
    when the ticket expires. It closes a client or a still-registering
    control connection past its connection limits with `4503` (full,
    transient), and answers a socket past its limits at accept `503`
    before closing it, including one past its address's rate of new
    connections; through a trusted reverse proxy, where the client's
    address is known only from the request, that rate is checked at the
    upgrade and refused with `4429`. A connection that authenticates (a
    node registered, a node's data connection claimed, a client still
    spliced 7 s after pairing, which its node has admitted) gives its
    share of the rate back. An IPv6 peer is counted by its `/64`, `/56`
    and `/48` (the wider blocks with two and four times each per-address
    limit). A client of one node from one address past
    `--max-node-clients-per-address` while the node admits them is closed
    with `4503`. Behind a trusted proxy, a client-address header that is
    not one bare address is answered `400`.
  - Deadlines a peer must keep: a request's headers within 10 s; a
    node's first control message within 2 s of the upgrade; a client's
    first record (its handshake message) within 2 s of the relay pairing
    it with the node. When every open registering place is taken, a
    control connection holds none until its first message: a key
    registered at that relay before takes one of the places kept for
    known nodes, if one is free; any other is refused `full` at once.
  - A node gives a channel a relay spliced to it 5 s
    (`vessel_relay::RELAYED_ADMISSION`) for the Noise handshake and the
    device preamble together, and drops it otherwise (a direct channel
    keeps 10 s for each); at most 40 channels wait at once through each
    relay link, apart from 64 direct ones, of which one address (an IPv4
    address or an IPv6 `/64`) holds 7, a `/56` 14 and a `/48` 28; past
    those the carrier is dropped at once. Behind a trusted reverse proxy
    (`vesseld --trusted-proxy`) the address is the client's from the
    proxy's header, in the one form a relay takes (one bare address, else
    `400`; without it, the proxy's own). A client must send its preamble
    as soon as its handshake is done.
  - A node that does not know `full` or `ticket` reads the message as
    malformed and retries; a client that does not know `4503` sees a
    plain close; a relay from before tickets refuses a ticket as
    malformed. The limits themselves are the relay's configuration
    (`vesseld relay --max-*`, README).

## Version 5 (2026-11-06)

Nodes speak 2..=5; version 5 only adds, and older connections keep working
(below). Milestone 4 implements it (`docs/plans/backend-m4.md`); until its
tracks land, nodes answer the new methods with `unsupported`, record every
session as created by the owner, and never emit `profile_changed`.

- Every method has a first version (`since`, the table in `vessel-proto`
  `rpc.rs`): `node.stop` 3, `device.*` 4, the methods below 5, the rest 2.
  A call also needs a later version when it uses a later field
  (`Call::since`): `session.create` with `spec.profile`, `device.invite`
  with `scopes`. The client refuses such a call on an older connection
  without sending it (`CallError::NotInProtocol`, code `unsupported`), and
  the node refuses it (`unsupported`, "needs protocol N"), so neither side
  ever relies on the other ignoring a field it does not know.
- Profiles (ADR 0011): `SessionSpec.profile` (absent is the harness's
  default); `SessionInfo.profile` (the profile it runs with now) and
  `SessionInfo.created_by` (`{"kind":"owner"}`, `{"kind":"device","id"}`,
  `{"kind":"agent","node","session"}`; absent from older nodes, read as
  owner). `profile.list { harness? }` (read) and `profile.reload` (admin)
  return `ProfileInfo[]` (`name`, `label?`, `harness`, `sets`: variable
  names only, never values). `session.move_profile { session, profile?,
  force? }` (operate) returns the session. Event `profile_changed { from?,
  to? }`. Errors: `session.create` naming an unknown profile, or one of
  another harness, is `invalid_request`; `profile.reload` against a file
  that does not parse is `conflict` (the profiles in effect stay, and the
  message says where without quoting a value). `session.move_profile`:
  an unknown profile or one of another harness is `invalid_request`; a
  terminal session, or a harness whose session files Vessel cannot move,
  is `unsupported`; a running turn without `force`, a target profile that
  is not logged in, an ended session, a session that is still starting,
  missing or conflicting session files, and input sent while a move runs
  are `conflict`; an agent that does not start under the new profile is
  `spawn_failed` (the session goes on under its previous profile, with
  no `profile_changed`).
- Fleet (ADR 0012), scope `sync`: `fleet.manifest` returns the `Manifest`
  (`generation`, `files`: bundle path to `{hash, size, executable?}`,
  `mcp`: server name to transport, with `{"secret": "NAME"}` references,
  never values); `fleet.blob { hash, offset }` returns `{data (base64),
  total}`, at most 1 MiB per chunk; `fleet.watch { after }` is a long poll
  that returns the home's generation as soon as it differs from `after`
  (older too, so a member follows a home whose data was reset), or `after`
  itself when the home's wait limit (50 s) passes; `fleet.report { generation, conflicts? }` returns `ack`.
  `fleet.members` (admin) returns `FleetMember[]`.
- Bundle paths: `instructions.md` is the home's instructions file, which
  each member writes under its harness's own name (`CLAUDE.md`,
  `AGENTS.md`, `GEMINI.md`); `skills/<name>/...` is a skill, `<name>` being
  the `name` in its `SKILL.md` (not its directory name). Other top-level
  paths are reserved; a member ignores what it does not know. A path
  segment may not be a Windows device name (`CON`, `COM0`-`COM9`,
  `CONIN$`, ...) or a short-name alias (`NAME~1`). There is no mode beyond
  `executable`: members write fleet files owner-only.
- Scopes: `sync` is new. `device.invite` takes `scopes?`; without it a
  device gets `read`, `operate` and `admin`, as in version 4.
- Agents (ADR 0013): on the local socket, an agent's `vessel mcp` sends
  `{"type":"agent","session","secret"}` as its first frame instead of the
  token (a token is never a JSON object). A credential that is unknown,
  wrong, or of an ended session is answered like a wrong token
  (`rejected`, `unauthorized`); an admitted connection closes when its
  session ends. An agent has its session creator's scopes, at most `read`
  and `operate`; `session.send`, `session.kill` and `session.move_profile`
  on a session it did not start (directly or through sessions it started)
  are `forbidden`, as are a ninth live session of its own and a session
  four levels of agents deep. Sessions it creates record `created_by:
  {"kind":"agent","node","session"}`.
- Older connections: a node adapts what it sends to the negotiated
  version, where it encodes frames. `profile_changed` reaches a version 2-4
  client as `agent { harness_log { line: "vessel: profile changed from A
  to B" } }` at the same `seq` (the log stays gap-free), in subscriptions
  and in `session.read` pages alike, and `device.list` leaves out scopes
  the client's version does not have.
- Vessel's own notices in a session's log are `agent { harness_log {
  line: "vessel: ..." } }` records, in terminal sessions too (for example
  right after `starting`, an MCP server left out because this node lacks
  a secret, naming the secret, never a value). A terminal client that
  shows only terminal output can ignore them; `vessel log` prints them.

## Version 4 (2026-11-06)

Nodes speak 2..=4; version 4 only adds. Milestone 3 (relay, secure channel,
pairing) implements it; until then nodes answer `device.*` with
`unsupported` and send no `public_key`.

- Methods: `device.invite { label?, ttl_secs }` returns
  `{ link, expires }`; `device.list` returns `DeviceInfo[]`;
  `device.revoke { device }` returns `ack` and closes that device's
  connections.
- Scopes: every method declares one (`read`, `operate`, `admin`; the table
  in `vessel-proto` `rpc.rs`). Calls from a paired device without it get
  `forbidden`. The node token and the local socket hold every scope.
- `NodeInfo.public_key`: `"x25519:<base64url>"`, absent when unknown.
- Secure channel (ADR 0009): `Noise_NK_25519_ChaChaPoly_SHA256`, prologue
  `vessel/secure/1`, empty handshake payloads (both messages 48 bytes).
  Each carrier message is one record of at most 65,535 bytes: one sealed
  chunk, `flags` (`FIRST = 1`, `FINAL = 2`, others refused), the frame's
  `u32` big-endian length in a `FIRST` chunk, then data. Every chunk but a
  frame's last is full; frames are UTF-8 and at most 16 MiB. Any failed
  record or broken rule closes the channel. Details in
  `docs/plans/backend-m3.md` ("Seams", `vessel-secure`), implemented by
  `vessel-secure`.
- Relay possession proof: the same NK handshake, relay as initiator to the
  registering key, prologue `vessel/relay-proof/1`.
- Preamble inside the channel, before `hello`: the client sends
  `{"type":"device_auth","device","secret"}` or
  `{"type":"pair_request","invite","device_name"}`; the node answers
  `authenticated`, `paired` (with the new credential and `NodeInfo`), or
  `refused` with `unknown_device` | `revoked` | `invalid_invitation` |
  `unavailable`, then closes.
- Pairing link: `vessel://pair#` + base64url(JSON `PairingOffer`): node id,
  key, relay and direct routes (`ws://` or `wss://`), the single-use
  invitation, and its expiry. The offer sits after `#` so it never reaches a
  web server.
- Relay control connection (`vessel_proto::relay`): node sends `register
  {routing_id, key, token}`, relay sends `prove {message}`, node answers
  `proof {message}`, relay sends `registered` or `rejected {reason}`, then
  `incoming {conn}` per waiting client (tickets and the `full` refusal
  came with version 6: "Relay registration" there). Routing ids and connection ids are
  32 lowercase hex characters. Relay close codes: `4404` no such node,
  `4408` not accepted in time, `4413` too large, `4429` rate limited.
- Node close code on a secure-channel websocket: `4401` the handshake failed
  (most likely the client pinned another key). Relays forward a node's close
  code to the client unchanged. Clients treat `4401` as final: re-pair.
- Closing (2026-11-07): a relay or a node that closes a websocket with a
  code then reads and discards what the peer still sends, until the
  peer's own close or end, for at most 1 s, so the close is not lost to a
  reset (a socket dropped with unread bytes is reset, and on Windows the
  reset throws away a close frame still in flight). A peer should answer
  a close and stop sending; one that keeps sending is dropped after that
  second.

## Version 3 (2026-11-06)

Nodes speak 2..=3. Version 3 only adds, so version 2 peers keep working.

- `node.stop { "when": "now" | "when_idle" }` returns `ack`. `now` is the
  signal shutdown: live sessions end `interrupted`. `when_idle` drains: new
  `session.create` calls get `unavailable`, live sessions continue, and the
  node exits when none is live. A client that negotiated 2 never sends it.
  The `ack` is written before the node closes the connection, `now`
  included. (Builds before milestone 2's node side answered `unsupported`.)
- `NodeInfo.state`: `serving` | `draining`. When absent (a version 2 node),
  read it as `serving`.
- `vesseld attach` exit codes (`ProxyExit`): `0` closed, `80` no node
  answered and starting one failed, `81` data directory or token unusable,
  `82` token refused. `127` comes from the remote shell and means "not
  installed". Codes are never renumbered.
- Byte-stream framing, for SSH stdio and the node's local socket: each frame
  is a big-endian `u32` length, then that many bytes of UTF-8 JSON (the
  same text as a websocket frame). The maximum is 16 MiB, checked before
  allocating. On the local socket only, the first frame carries the token
  (ADR 0007), then the usual `hello`. A wrong or missing token is answered
  with `rejected` (`unauthorized`), and the connection closes.

## Version 2 (2026-11-05)

- Transport: one websocket at `/rpc`, JSON text frames.
  `Authorization: Bearer <token>` is required on the upgrade. `GET /health`
  stays open.
- Handshake: the client sends `{"type":"hello","protocol":{"min":2,"max":2},
  "client":{...}}`. The node replies with `welcome` (negotiated version plus
  `NodeInfo`), or with `rejected` and then closes.
- Calls: `{"type":"request","id":N,"call":{"method":"session.create",
  "params":{...}}}` is answered by `{"type":"response","id":N,"outcome":
  {"ok":{"method":"session.create","result":{...}}}}` or by
  `{"outcome":{"err":{"code":"not_found","message":"..."}}}`.
- Methods: `node.info`, `harness.list`, `session.list`, `session.get`,
  `session.create`, `session.send`, `session.kill`, `session.read`,
  `session.subscribe`, `session.unsubscribe`.
- Streams: `session.subscribe{session, from}` returns a subscription id. Then
  `{"type":"event","subscription":S,"record":{"seq":..,"at":..,"event":..}}`
  frames follow, ending with `subscription_ended`
  (`session_ended` | `unsubscribed` | `lagged{resume_from}`).
- Ordering guarantees (ADR 0006): the node never sends `event{S}` before the
  `response` that carries `Subscribed{S}`, and sends nothing for `S` after
  `subscription_ended{S}`. For `session.unsubscribe`, the client receives
  `subscription_ended{unsubscribed}` before the call's `ack`. Records within a
  subscription arrive in `seq` order with no gaps; a gap is only ever
  announced as `lagged{resume_from}`. If the node stops serving a
  subscription for any other reason after announcing it, it also ends it
  with `lagged{resume_from}` at the first record not written, so the client
  resyncs instead of waiting.
- Limits: each connection has a bound on calls in flight. Calls over the
  bound are answered at once with `unavailable`; the node keeps reading the
  connection, so `cancel` and `session.kill` always get through.
- Session modes: `terminal` (base64 PTY bytes) and `structured` (ACP-derived
  `AgentEvent` timeline).

## Version 1 (Phase 0)

REST routes and one websocket per session stream. Replaced entirely by
version 2.

## Staged project-file terminal approvals (ADR 0066)

`CommandSet` and `ProjectCommands::Listed` gain a default-empty `terminals`
list (`FileTerminal`: optional validated title, validated command). No call
is added. The version is assigned at integration through
`workspace::PROJECT_FILE_KEYS`, currently `None`; `PROTOCOL_VERSION` stays
unchanged. Until assigned, any set naming terminals is returned as `invalid`
on every negotiated connection. This also covers a terminal-bearing prior
approval and script/worktree approval replies. Older nodes omit the field;
new readers treat omission as no terminal commands. Parsing and approval are
staged ahead of automatic terminal creation, which is not implemented yet.

The same unassigned `PROJECT_FILE_KEYS` also gates a declared `ScriptEntry.port`
(nonzero TCP port, optional/absent by default). A current or previous approval
with one is invalid for every peer while unassigned, through every direct and
nested approval reply. `ScriptEntry` keeps its JSON kind string and validates
kind and port together; a plain script with a port is refused. The TOML parser
reads `services.<name>.port` through that validated declaration and includes it
in the version-three approval. It assigns no number and starts no allocator
or process.


The same gate also covers optional `service_ports` on `CommandSet` and
`ProjectCommands::Listed`. Its `ServicePorts` owner contains a range and/or
literal port-script path; an empty block or an unknown choice is refused.
Omission from an older node decodes as absent. The file parser keeps block
presence even without services, and version three fills its two reserved
slots with the canonical range and approved literal program. A current or
prior block is invalid on every approval reply while unassigned. CLI shows
both choices; web refuses current, prior and approved-folder blocks until
it can display them completely. No allocator or process runs in this stage.

## Pending independent workspace registry (ADR 0082; assigned 25, see Version 25)

The registry dependency declares workspace.registry, workspace.list,
workspace.get, workspace.rename, workspace.pin and workspace.release_automation
in the single method table with PROJECT_WORKSPACE_REGISTRY = None. No wire
number is assigned here. Clients/forwarded calls refuse pending methods before
routing; existing session/project replies are unchanged.

The atomic snapshot carries node identity, a monotonic registry revision,
independent workspace descriptors and explicit session/terminal ownership. Cwd
and a node-book alias never identify a workspace. Empty workspaces are retained.
Foreground/background visibility belongs to the workspace; bound session listing
must be derived from this authority, while legacy unbound sessions remain a
separate compatibility surface.

Descriptors expose Active, Archiving, Restoring or Archived, exact local/managed
backing, explicit title and pin, and the retained automation gate. Source release
requires both the expected revision and exact typed external source; it changes
only that gate. Person-only policy and Read/Operate scope checks apply. Archive,
restoration, resource launch and create receipts are separate orchestration
packages using the same durable registry/start-lease fence. Complete snapshots
are bounded by the store's descriptor and retained resource budgets below the
existing RPC frame limit. No cwd inference, implicit trust, replay or teardown
occurs in a presentation mutation.

## Diagnostics before their number (ADR 0074; assigned 24, see Version 24)

At its review checkpoint `node.diagnostics {}` returned a bounded
DiagnosticReport through a pending method-table row. It needs Read and is
unary, not fleet-forwardable. DIAGNOSTICS_PROTOCOL was None until the
integrator assigned 24 at the merge; while unassigned, every client send and
server dispatch refused it through required_protocol.
The report adds explicit own-process numeric observations, source states and
closed resource-refusal detail; it never carries raw logs or environment.
