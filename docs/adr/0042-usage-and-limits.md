# ADR 0042: Reported usage and account limits

Status: proposed (milestone 12).

## Context

ACP already supplies `TurnUsage` and `ContextUsage`, including an optional
running session cost. A context size is not billed token usage. Provider
inputs disagree about whether cached tokens are included. Summing context
updates, or adding the proxy's count to the agent's count, doubles usage.

Paseo's `plugins/claude-usage-source/server/usage.ts` reads the selected
local OAuth login and asks Anthropic's usage API; its Codex counterpart
asks the ChatGPT usage API. They discover accounts locally, leave refresh
to the harness, retain the last successful report on failure, and expose
windows separately. `packages/server/src/server/config.ts` enables usage
by default. GOAT also obtains limits from Claude's `get_usage` control
response and Codex's `account/rateLimits/read`, and records rate-limit
notifications. Its recovery banner offers resume and snooze at reset.

## Decision

- Reuse the ACP reports, event writer, SQLite readers and existing
  client authentication. No scraping, price catalogue, billing estimate,
  transcript scan, or inferred token count. Proxy reports are a separate
  source and never added to ACP reports for the same session.
- Commit token samples and cost increments in the event append's own
  transaction. Each sample is keyed by session and event sequence. Count
  a turn once. An absent report remains absent; a reported zero is zero.
  Keep cache reads/writes and reasoning apart as the agent reports them.
- ACP cost is cumulative for the native session (ACP v1 `UsageUpdate`
  and `Cost`), not the process. Keep per-unit baselines across `Ready`
  for the same native session. A changed native identity, a decreasing
  counter, a profile handoff, or a report replayed during startup becomes
  an explicit baseline/gap rather than spending that cumulative amount
  again. Only a session newly created after accounting began, with no
  resume/import origin, has a known zero starting point. Existing
  sessions' first observations are not assigned to the current day.
  Negative amounts are ignored. Never combine different cost units.
  Every increment carries its observation interval: a period cutting
  that interval gets an explicit gap rather than the whole charge.
  Session queries also show the latest cumulative reports independently
  of time-period increments. Their visibility uses the same cost scope.
- The session runtime marks context observations made while loading a
  conversation as `replayed` (the optional protocol-21
  `ContextUsage.observation` field). This includes profile moves, wakes
  and rewinds whose Ready event is deliberately consumed by the replay
  filter. Such an observation can establish a baseline or a gap, never
  spend an increase under the previous account. The runtime sets the
  origin; a vendor cannot choose it. An ordinary observation defaults to
  `current`, omitted on the wire. Older peers receive the original context
  report without this metadata. This reuses the existing replay boundary
  instead of inferring a load from status, counter value or wall time;
  those facts cannot distinguish a hidden Ready from new work. Changed
  native identity or profile still clears the baseline conservatively.
  The existing event/projection storage keeps this metadata; no new
  retention policy, credential access or database migration is needed.
- Keep at most 90 days and 100,000 usage samples per node, independent
  of event byte pruning, with session deletion cascading to its samples.
  The bounded per-session cost baselines remain until session deletion,
  independently of sample retention, so pruning cannot charge old totals
  again. No historical backfill: reports say when accounting began and whether
  their bounded scan was incomplete. Daily means the past 24 hours and
  weekly the past seven days, explicitly labelled rolling periods.
- Each account limit is a latest successful snapshot, not an accumulating
  counter. Report the provider's window identity, used percentage,
  duration when supplied, reset time and observation time. Unknown
  durations stay unknown (Paseo's Codex rule). A failed refresh retains
  the successful snapshot with a separate stale/error state.
- A node may read its agent's stored sign-in only on that agent's machine,
  from the operator-configured profile home. Credentials never enter the
  protocol, relay, database, diagnostic text or logs. No client-selected
  path, endpoint, header or command. The harness owns refresh; Vessel
  never writes the sign-in. Bounded local reads and requests only to the
  provider's fixed HTTPS origin, no redirects or proxy credentials.
- **External lookup defaults off**, deliberately differing from Paseo:
  starting a node must not implicitly expand a stored sign-in's use.
  While off, Usage shows the agent's own reports and one opt-in row:
  it explains that the node reads that agent's saved sign-in on this
  machine and sends it only to that vendor to read account limits.
  Only an administrator changes this policy. Turning it off drops
  cached external snapshots and prevents queued refreshes from publishing.
- `read` sees token aggregates and limits; `operate` additionally sees
  cost in the new usage calls. This does not retroactively change the
  existing conversation/event read contract, which can include reported
  cost. Agent principals may read only their own local session's usage;
  no account discovery, machine totals or another machine's account data.
  Delegated calls use the existing scope intersection at the destination.
- Recovery is explicit: identify all exhausted windows, use the latest
  known reset among them, and offer wait-and-continue only with a known
  future reset. Persist the intent; cancel on an intervening user turn,
  changed account, deletion or explicit cancellation. Recheck state before
  sending once. Offer account handoff only through the existing checked
  profile move; do not offer an action for an unavailable route.
- Protocol 21 adds usage calls (proposed as 19 after the protocol-18
  node line; side questions took 19 and `device.self` 20 at the merge);
  the ledger is schema 15. The minimum stays 2. Older clients see the
  existing conversation reports. New clients on older nodes receive the
  existing version refusal and show that usage is not available there.
- Protocol 23 adds `usage.limits` and `usage.lookup` (the track proposed
  them as 19 with `usage.query`; the inbox took 22 at its merge first).

## Alternatives

Read transcript files as GOAT's history view does: rejected because the
node already has a typed, atomic event stream and transcript semantics
vary. Derive costs from tokens: rejected because subscription prices and
cache accounting do not prove what was charged. Send sign-in material to
a fleet home: rejected because routing does not confer credential custody.
Automatic fallback without consent: rejected because it changes the
account that spends money and may change the agent's behaviour.

### Exact cost arithmetic and bounded reads

The input domain is each finite wire number's decimal spelling, rather
than the binary float's expansion. `rust_decimal` 1.43 (MIT, default
features off, std only) validates that spelling with `from_str_exact`;
ordinary parsing and scale-reducing arithmetic can silently round tiny
values. Inputs outside its exact range become explicit cost gaps.
The shared JSON dependency enables `float_roundtrip`: its default parser
can otherwise change a finite number by one ULP while reading the wire or
reloading a stored report, before exact accounting even sees it. A whole
context-usage response fixture checks that its cost survives serialization
and decoding at the tiny and large boundaries. This selects the existing
JSON library's precise parser; it adds no dependency.

`num-bigint` 0.4 (MIT OR Apache-2.0, std only) holds integer multiples of
1e-28 for differences and accumulation. The active dependency closure is
`num-integer`, `num-traits` and the latter's build helper `autocfg` (all
MIT OR Apache-2.0); randomness, serialization and property-test features
are off. With at most 100,000 samples and 32 cost units, each accumulator
needs fewer than 210 bits. An unbounded-sized vendor number never enters
it. This replaces repeated Decimal-to-float partial sums, which lose
independent small costs, and avoids an order-dependent intermediate
rescale or overflow. Maintaining only a checked Decimal sum was rejected
because checked arithmetic can still reduce its scale.

Only the final sum or increment crosses back to a finite wire number.
Its decimal spelling must parse exactly to the expected Decimal value.
A result outside that range or without a lossless wire representation
is omitted, never rounded; an omitted unit counts all its contributing
cost samples in `unaccounted_costs`. Other units remain independent.
Current cumulative reports stay the provider's original observations.

The general reader reuses idle connections; that cache does not limit
concurrent jobs. Usage scans therefore have a separate two-job admission
limit shared by every Store clone on the node. An excess call is refused
immediately with a typed busy error and retry advice. A permit belongs
to its blocking closure while queued and running, so aborting the RPC or
closing its connection cannot admit replacement scans before old work
ends. Other legacy reads keep their existing behavior.

## Consequences and verification

A bounded ledger costs disk space and does not claim complete history
before this version or beyond retention. Limits are provider capabilities,
not guaranteed for every ACP agent. Tests use sanitized fixtures matching
vendor responses and the process-level testkit only. No real credential
file or vendor endpoint may be read during this work. Live-account lookup,
including actual reset and recovery, remains explicitly unverified until
an operator checks it separately.

### Envelope version compatibility

An intermediate peer must understand the typed inner method and its fields.
The common `Call::since` rule therefore takes the maximum of the envelope's
version and the inner call's full requirement for forward, relay and deliver.
Checking only the target would let a forced self-forward execute a new
usage call on an older negotiated connection. The same central calculation
guards client sending and server dispatch; no second version checker is
introduced in the node or renderer. Regression tests cover local refusal,
raw forced requests before dispatch and supported older envelopes.

### Saved-sign-in lookup contract (node-only continuation)

This section defines the saved-sign-in implementation contract. Its node,
client and CLI implementation is under verification; source presence is not
a gate, integration, native-screen or live-account approval.

`usage.limits` selects one configured account binding or the selected profile
of one session. An omitted account profile selects the configured default; it
does not mean all historical profiles, as in `usage.query`. The existing
`profile.list` supplies the bounded enumeration to a Usage screen. Results
explicitly describe the last observation of a saved sign-in, not proof of
which credential an already-running process remembers. Profile reload or an
external login swap can make those different. No percentage alone is accepted
as the cause of a failed turn.

A cached read requires `read` and performs no credential or vendor I/O. An
explicit refresh additionally requires `operate`, because it initiates
credential use and network egress; this deliberate restriction does not hide
already obtained limits from read-only devices. `usage.lookup` requires
`admin`. Cost and credit balances require `operate`; eligibility flags remain
visible, so paid continuation is not mistaken for a hard stop. Agents may
select only their own local session. The destination checks these rules after
delegated-scope intersection, as for usage aggregates.

Session-scoped lookup uses a nonsecret binding captured with the actual launch
profile. A catalogue reload cannot redirect it to a different home; a
successful move replaces it, a failed move preserves it, and park, wake and
rewind keep it. The binding includes its account label atomically, so it
cannot be labelled using a separately read pre-move profile. Use the existing
per-session state, not another mutable registry. This binding is RAM-only and
contains no credential. If a stored session no longer has a known launch
binding, return an explicit unassociated state rather than look up a possibly
different current profile. Account-scoped lookup always uses the current
configured profile, which a Settings page deliberately selected. A file at
either location may still have been replaced while a process remembers its old
token; the response makes that limitation clear.

An owning actor serializes policy changes across RPC cancellation. Only Off/On
is stored in an owner-only atomic `usage-lookup.json`; no token, fingerprint
or snapshot is persisted or synchronized. A malformed or unreadable choice
fails off. The command queue holds at most 32 messages, there are at most two
probe jobs and 256 latest account entries in RAM, and each response holds at
most 128 independent windows. A cancelled RPC cannot drop a job's admission
while its blocking local read is queued or running. Turning lookup off cancels
the current generation and clears its cache before acknowledgement. Old
generation results never publish after Off or a later On. The actor drains
rather than aborts blocked reads.

The harness descriptor selects the supported source. Claude reads its
configured `.credentials.json`, preferring the matching macOS Keychain item
under Claude Code's account/service convention. Custom config paths use the
NFC-normalized directory's SHA-256 prefix and never fall back to the default
service. Codex reads its configured `auth.json`; OpenCode's OpenAI OAuth login
uses its XDG data store. No refresh token is requested or used. Known foreign
routes, API-key-only logins, unknown agents and unsupported stores answer an
explicit safe category. The harness remains responsible for refreshing or
replacing a sign-in.

Only a bounded GET to Anthropic's `/api/oauth/usage` or ChatGPT's
`/backend-api/wham/usage` is allowed. Origins, headers and methods are fixed
in the harness crate, redirects and inherited proxies are disabled, connect
timeout is ten seconds, request timeout fifteen, and each local file/response
is bounded to one MiB. Headers are sensitive and raw bodies are zeroed on
release. Raw credential/parser/HTTP details never reach logs, replies or
diagnostics. Vendor Retry-After limits forced refreshes too. A bounded
per-node vendor/token cooldown applies across profile aliases and policy
toggles; known active deadlines cannot be silently evicted to bypass the
vendor. Use response receipt time for relative Retry-After seconds. A failed
refresh retains a snapshot only after matching the observed saved-login
fingerprint, including the selected account identifier; missing or changed
logins discard it. Internal equality keys stay in RAM and never leave the
harness/node boundary. A credential-free HTTP client can be cached per node;
its certificate loading is blocking work owned by the same admitted job.
Cancellation never detaches a queued/running file or certificate reader: await
these phases, check cancellation, and select cancellation only around
asynchronous network I/O.

Unlike Paseo's default-on behavior, Vessel starts off. A screen should show
`usage.query` observations while off and one working admin opt-in row: "Read
this agent's saved sign-in on this machine and send its access token (and
Codex account identifier, when present) only to Anthropic or ChatGPT to read
account limits." It can name the typed source without exposing a private local
path. It calls `usage.lookup` to change the policy, then `usage.limits` with
refresh; subsequent cached reads perform no I/O.

The wire keeps provider-reported periods separate, leaves unknown lengths
unknown, preserves zero versus missing percentages, scopes model/surface/
feature/code-review allowances independently, and preserves explicit
paid-use/credits/allowed flags. A full-window notice reports identities and
the latest future reset only when every selected full general window has one.
It does not assert that a turn failed because of those windows, or offer an
unimplemented resume/switch action. Persistent automatic recovery remains a
separate unfinished item of the original acceptance list.

The library default backend is Disabled. Binaries explicitly select Local,
while the persisted policy remains off. Test builds with fakes refuse Local
even if accidentally selected. Recorded fixtures are confined to one canonical
temporary root and contain only synthetic vendor-shaped data. No check in this
slice may read a real credential or call a vendor. Actual login, vendor
authorization, macOS Keychain and live reset behavior remain unverified until
an operator checks them.

This continuation deliberately exposes factual notices only. A full window is
not a stopped-turn diagnosis, and no automatic resume or account switch is
advertised before its persisted intent and authority checks exist. The
original recovery acceptance remains unfinished. Native app screens, controls
and page work belong to their own tracks.

The implementation reuses workspace chrono, reqwest/TLS, zeroize, SHA-256 and
Unicode normalization. The latter reproduces Claude's Keychain service
identity; std alone cannot normalize NFC. No new third-party package is
selected. Harness-specific decoding and endpoint choice stay in
vessel-harness; the node owns authorization and lifetime, and vessel-store
owns atomic policy persistence. This avoids teaching the client a vendor
format or moving credentials across a fleet link.

Launch bindings share immutable nonsecret configuration through `Arc`. This
avoids copying path/account-selection strings through each launch future and
cache lookup; it also keeps existing launch futures within their stack-size
bound without weakening tests. Equality compares the captured values rather
than allocation identity. The tradeoff is one allocation per resolved binding;
credential contents remain ephemeral and outside that allocation.

A multi-provider agent's saved login is not evidence of its current provider.
OpenCode account lookup reads its saved OpenAI OAuth login; session lookup
additionally requires exactly one current model selection with the native
`openai/` prefix, as Paseo's Codex source `sessionRoutes` does. Missing or
ambiguous evidence is unassociated, and a known different provider is refused.
The node checks the existing in-memory agent options before local credential
I/O and again after waiting for the probe, including cached reads and parked
sessions. A concurrent provider change cannot publish that account's report as
facts of the changed session. This adds no prompt, credential discovery or
model default inference; account lookup remains independently available.

### Compatibility integration

The accounting query, account-limit methods and replay provenance share the
named usage protocol boundary in `vessel-proto::usage`. The final number is
given at merge. The method table accepts constant expressions so changing
that one boundary cannot accidentally leave replay provenance behind. This
changes no wire shape or supported behavior at the current boundary.

## Addendum: source account reports for all usage surfaces

Status: proposed. Existing protocol-21 accounting and protocol-23 saved-login
limits remain authoritative and unchanged. The new report family has no assigned
protocol number and must not be advertised or called before its consumers and
method-table row are reviewed.

### Purpose and intent

A profile's saved-login quota is not the full report model used by Paseo's
Usage cards, sidebar pins, host-filtered modal, per-agent details and CLI.
Those surfaces need stable source/account report identities, independent
windows, balances, details, source tone and successful-observation provenance.

### Existing implementation and constraints

`usage.query` is the retained reported ledger; `usage.limits` reads one bound
saved account with immutable launch association. `usage.lookup` controls saved
sign-in use. Do not reconstruct a generic report by treating ledger totals as
quota, identifying an account by its label, or discarding failed refresh state.
The current built-in probes support Claude and OpenAI saved sign-ins through
Claude Code, Codex and OpenCode. The plugin helper contract (ADR 0079) is an
open extension point, not permission to introduce a second report schema.

### Alternatives considered

1. Extend `UsageLimits` into an unbounded source/plugin payload: would conflate
   its saved-profile association with source-account discovery and break clients.
2. Fetch vendors from each UI: would move credentials off the owning machine,
   duplicate policy/cache semantics and exclude the CLI.
3. Add a typed report family, reuse the reviewed limits engine where applicable,
   and supply one open source registry shared with the plugin helper adapter.

### Choice and reasons

Choose the third. Define one `usage_reports` contract with validated source,
account and report identifiers; a report id is opaque to clients and can be
resolved by the same discovery path used for listing. Typed scope distinguishes
all/report-id selection from a session's launch association, so combining a
session selector with report ids is unrepresentable.

A report entry carries source/account metadata and an exhaustive attempt state:
not requested, refreshing, available, unavailable, or failed. Only successful
observations own values and their timestamp. Failed or pending attempts may
retain a same-binding success, visibly marked retained. Windows preserve used
and remaining percentages separately, source-provided tone, default-summary
metadata, reset instant, optional run-out forecast and shortfall. Balances keep
reported units and values; no prices or estimates. Details declare visibility.
All lists and identifiers are bounded at decoding and construction edges; an
oversized or duplicate identity refuses the response instead of silently losing
an account. Bounds and serialization tests live beside the types.

The registry owns discovery, cache identity, per-source admission,
identical-fetch coalescing, immutable generation and successful history. Paseo's
five-minute source freshness was the reference design; Vessel's executable Read
is strictly cache-only, including an old observation or a miss. Expiry does not
initiate credential discovery or a vendor fetch; an explicit authorized Refresh
is required. Source adapters own provider schemas, fixed network destinations and
local login reading; the registry never inspects opaque plugin-owned inputs.
A source-local opaque login reference keeps those inputs in its adapter. Plugin
Ready/discover/fetch uses these same report types, with the plugin generation
fence and cancellation from ADR 0079. No helper is claimed available before
its actual adapter is wired.

### Credential use and authorization

Preserve the existing floor: lookup defaults off; Admin changes it; Read is
cache-only and cannot read a credential, invoke discovery that reads a login,
or reach a vendor. Explicit Refresh additionally requires Operate and may
perform bounded source discovery/fetch on the owning machine. A cached miss is
not an invitation to perform I/O. The session scope uses the reviewed immutable
launch binding and current provider proof before and after awaiting work.
Read-only callers receive no monetary balances or Operate-only details.
Off cancels admitted discovery/fetch, invalidates its publishing generation,
forgets snapshots, and acknowledges only after the persisted policy applies.

Built-in HTTP calls keep fixed vendor origins, no redirects, no proxy/header
inheritance, bounded bodies and existing deadlines. Adapters never refresh or
write the agent's saved login. Safe typed failures replace raw HTTP/credential
messages. No credential, fingerprint, opaque helper input or private path is
serialized in the public report, logged or written to diagnostics. Test sources
and recorded logins are test-only or under the existing fakes isolation; they
cannot supply a production report.

### Advantages, costs and effects

One report/cache model serves the native UI, page, CLI and plugin source
adapter, while the existing accounting/limit calls remain compatible. Source
identity is distinct from account and from presentation. Added costs are a new
bounded report family, source adapters and lifecycle tests. Streaming individual
updates requires the reviewed method-table/observation contract; it must not be
improvised as a second protocol table. Any initially unsupported source or
transport path stays explicit in the package tracker and cannot count as parity.

### Captured identity read (F3 review correction)

An after-check cannot exclude selector ABA: outer A, inner resolution B, final A.
The generic saved-report path therefore sends the captured canonical account and
resolved LimitBinding directly to the existing limits actor, never through a
service that resolves the profile again. It still verifies current session/provider
association, account equality and policy generation before publication. No global
profile generation is needed to distinguish the actual read's provenance.

SourceReportEntry serializes distinct validated source/account fields as well as
its opaque handle. One constructor derives all three atomically; deserialization
rejects disagreement. Consumers group by those typed fields and inspect by the
opaque handle. Labels and client-side handle parsing cannot define identity.

### Additional-source consent does not inherit the legacy grant

[Decision Log]
- Purpose: the old global lookup choice disclosed three saved-login stores.
  Reusing On as permission for seven additional stores would exceed that consent.
- Reuse: preserve the global default-Off Admin floor and its three existing
  adapters. Add bounded source metadata and explicit revisioned source grants;
  use the store's existing owner-only atomic replacement rather than a database
  containing report snapshots or credentials.
- Choice: known store and fixed-recipient enums provide translated disclosure
  without actual paths/endpoints. InheritedGlobal is only for the three existing
  projections. New sources start Off; unsupported adapters cannot appear Enabled.
  Admin usage.source_lookup accepts an exact registered id/current revision and
  never fetches. Explicit Refresh still additionally requires Operate and global On.
- Failure rules: missing/invalid source grants fail Off; duplicate/oversized
  policies refuse as a whole. Changed disclosure requires fresh consent; source
  revocation invalidates pending publication and snapshots before acknowledging
  persisted policy. The plugin runtime must supply its own real disclosure and
  provenance before any plugin source becomes supported.
- Costs: a separate consent control for newly supported sources; it avoids silently
  changing credential-use authority when a node gains a provider adapter.

The setter was reviewed with its protocol None and is assigned 26 together with
the reports (see "Account methods assigned"). It refuses Unsupported sources;
values and persistence tests alone are not executable source or consumer parity.

### F3 executable source ownership and refresh presentation

The additional fixed adapters and source actor now live in the modules documented
in ARCHITECTURE's source/account usage section; protocol assignment remains pending.
The actor owns the consent write even if its requesting RPC disappears. Source Off
invalidates current work and copied snapshots before acknowledging the persisted
choice; global Off remains the floor. Fixed known vendor store/recipient metadata
lets clients translate exact disclosures without credential paths or endpoints.

An explicit Refreshing state records the immediately preceding attempt's typed
Available/Unavailable origin. Only an actually current success remains summary-
eligible while pending; a failed/unavailable retained observation stays card-only
and keeps its original success time. Legacy refresh states with no such proof
project Unavailable. No percentage or forecast diagnoses a stopped turn.

The separate feature-gated recorded node executable exercises production policy,
cache and projection with scratch-only synthetic inputs. It does not establish
successful live vendor support, assigned wire admission, plugin execution or
additional-source immutable launch association. Those remain separate acceptance
conditions; no local Cargo or manual VM binary copy is a validation substitute.

### F3 final node verification freeze

The two adapter families share the same configured admission context, rather than
minting separate permits for saved-login and additional-source jobs. Actual work
keeps its permit until it stops. The additional HTTP adapter now selects policy
cancellation around the actual request future; precancelled work never polls the
request, and cancellation after admission drops it. Network-free barrier tests
exercise those two production paths without credentials or vendor endpoints.

Exact `9bd4a9c86` passed the five changed crates' all-target strict Clippy check,
including TypeScript and the required usage-fixture feature, in run
`wp-usage-consent-values-20261115T142252Z-9bd4a9c8`. The two HTTP cancellation
regressions passed in `wp-usage-source-tests-20261115T142251Z-9bd4a9c8`. Earlier
four-crate library coverage passed 746 tests at `05b7a342`; the later real mixed-
family admission regression passed at `d5f1804b`. Those are distinct snapshots.
Generated declarations recovered from the latter source are committed unchanged;
current freshness and the complete root gate remain owed at this freeze.

The recorded fixture's earlier managed export passed independent source, recipe,
feature and byte checks. Its isolated UID/GID-1000 boot, same-source CLI status,
normal stop and scratch permissions were checked separately. That older binary
is not evidence for this frozen runtime, assigned report calls or UI acceptance.
No production service, credential, vendor or permission change was performed.

### Global revocation reaches admitted source work

[Decision Log]
- Purpose: committed global Off must stop credential results from beginning vendor
  HTTP and must drop active HTTP even if the settings caller disappears.
- Constraint: the legacy policy actor already cancels its canonical observation
  generation independently of its RPC. Caller-owned registry cleanup and Clear
  cannot be the enforcement boundary. Node/source cancellation and physical
  reader permits must remain intact.
- Choice: each independent source job observes that captured global generation,
  cancels its existing probe token and awaits the same actual work. The source
  owner separately observes the canonical generation to invalidate entries and
  its copied-snapshot generation without a follow-up RPC command. No new policy
  store, authority, generation allocator or detached cancellation task is added.
- Compatibility: source grants survive global Off; On does not fetch. Existing
  source Off, node shutdown, bounds, cooldowns, coalescing and physical draining
  remain. Stale global observations are refused before they can prune current work.
- Regression: real guarded service/actors warm a success, hold credential work or
  a network-free HTTP future, commit durable Off, block the setter's registry
  cleanup and drop its future before Clear. Tests require independent cache
  invalidation, zero late HTTP polls or active HTTP drop, and fresh Off/On ids.
  Fakes-only recorded HTTP is an explicit format-boundary capability; it never
  selects an arbitrary endpoint or appears in installed node arguments.

The original frozen whole run `wp-usage-source-tests-20261115T145906Z-5421d793`
is retained RED: web installation passed; fmt stopped on this package's harness
re-export ordering before any later gate step. Corrected source, strict/runtime
checks, current generated freshness and the eventual changed-source whole gate
remain owed. No vulnerability fix or package acceptance is claimed from inspection.

The first correction library run at `ad82f001` passed 347 tests and failed the
two new regressions at their recovery assertion. All preceding revocation/drain
assertions executed, but the tests incorrectly required the entire cached list
to be empty after Off/On: legacy configured-account metadata may legitimately
remain as NotRequested or unavailable entries without successful observations.
The corrected assertion requires no Cursor source account/id and no successful
or retained values in any returned entry. It preserves the no-fetch check and
new-id/Available explicit-refresh control. This remains a failed run, with the
corrected exact execution and strict check still owed; production code is unchanged.

### Saved report admission retains the captured policy

[Decision Log]
- Purpose: an old generic report request must not start saved-login work by
  borrowing a later On after an intervening Off.
- Constraint: identity registration and actual legacy actor admission have an
  await between them. A final publication check prevents a result escaping but
  cannot undo a credential read or vendor operation already admitted.
- Choice: the internal generic-report read carries the existing policy
  Observation into the serialized limits actor. It refuses a revoked observation
  before consulting the cache or starting work. Direct legacy `usage.limits`
  keeps admission-time policy semantics; no wire or new policy generation exists.
- Alternative: another service-side precheck leaves the same queueing race.
- Regression: pause the real pipeline after identity registration, complete
  Off/On in the actual actor, then release admission. The stale request must
  conflict and leave the cache NotRequested; a new request must read the same
  synthetic recorded sign-in/body successfully. No real sign-in or HTTP is used.

The earlier A/B fixture incorrectly supplied CODEX_HOME to the route-fact reader,
which does not capture harness-home variables. It now uses HarnessEnv's existing
home reader and asserts that A and B proofs differ while their account identity
is equal. Earlier passing executions did not prove distinct resolved homes;
stronger corrected execution is owed.

### Explicit refresh completion is source-specific

Additional-source Refresh requests await/join their bounded source job. A cached
Read may immediately expose a Refreshing attempt admitted elsewhere. The three
legacy saved-login limits retain their existing behavior: the first Refresh
awaits its actual probe, but a read/Refresh arriving while that binding is already
Refreshing immediately returns that state, including through reports/inspect.
The complete list/inspect budget bounds the request; it is not a universal promise
that every returned report is terminal. Consumers own their local request pending
state separately from the cached attempt phase, so a foreign Refreshing snapshot
cannot permanently disable an explicit refresh or a cache-only retry.

### Account HTTP activation waits for tracked native DNS (ADR 0104)

The usage client in `vessel-harness/src/limits/http.rs::Http::new`, also used by
`AccountUsageProbe::Local`'s body reader, is a known third consumer of the shared
tracked DNS resolution primitive (after NodeLink relay and dictation endpoints).
Its current reqwest builder uses the default resolver. Cancelling the async body
future does not establish that an already-started native resolver closure ended;
the two-request admission limit must not be presented as a physical DNS drain
witness. This is a source-derived limitation, not an executed DNS or vendor probe.

[Decision Log]
- Purpose and intent: keep actual native DNS work owned through completion using
  the one shared ADR 0104 primitive and reqwest's `dns_resolver` hook.
- Existing constraints: recorded fixture tests bypass native resolution; successful
  request cancellation, credential drain and the product whole do not prove it.
- Alternatives considered: a private usage resolver, a fork and interpreting async
  cancellation as physical completion would duplicate ownership or misstate proof.
- Choice and reason: bounded backend preparation may be reviewed with an explicit
  activation hold. Both new feature constants and all three new account method
  rows stay unassigned (`None`) until the reviewed shared resolver is wired.
- Impact: no active-readiness or physical-DNS-drain claim. No new worker whole is
  requested for this documentation; the eventual receiving integration owns the
  shared consumer checks and whole gate. Existing legacy behavior is unchanged.

### Account HTTP consumes the tracked resolver (2026-11-16)

The hold above is now met in source; activation is still a separate step.
The node owns one root `ResolutionBudget` (node setup) and, for the one
persistent `LimitContext` client, one `ResolutionScope` sized from the existing
read bound (`LimitContext::READ_SLOTS`). Its reqwest adapter
(`vessel-node::http_resolution`) is injected as an `Arc` of reqwest's `Resolve`
through `ClientBuilder::dns_resolver`; Harness still depends on no relay crate.

[Decision Log]
- Purpose and intent: every provider lookup, for the active saved-login read and
  for the pending account sources alike, is owned until native completion.
- Existing constraints: one OnceCell client shared by both source owners; a
  reqwest lookup carries no request identity.
- Alternatives considered: per-read scopes (no identity to key them on), a
  default-Gai fallback when nothing is injected (the untracked work again).
- Choice and reason: the coordinator's client-scoped ruling. Both source owners
  run on one local task tracker; a retirement task on the node tracker waits
  for both, then closes and drains the scope, so node stop cannot finish first.
  A context without a resolver builds no client and answers Unsupported.
- Impact: the already active `usage.limits` Local read now resolves through the
  tracked scope too (legacy behaviour changes only in who owns the lookup and
  in the finite lookup quota: a full or closed scope reads as Unreachable).
  The three account rows and both feature constants stay `None` until the
  integrator assigns them at merge.

### Account methods assigned (2026-11-16)

`ACCOUNT_REPORTS_PROTOCOL` and `USAGE_SOURCE_CONSENT_PROTOCOL` share protocol 26,
assigned in merge order on the node line together with the tracked-resolver
wiring above. The coordinator ruled that the node activation lands before its
native, page and CLI consumers; they fall back to `usage.limits` (23) on older
or unassigned peers and follow as their own packets. The consent rules above
are unchanged by the assignment.

### Revocation reaches the file before it is acknowledged (2026-11-16, review row 318 F1)

[Decision Log]
- Purpose: a person's Off must survive a restart. Before this change the setter
  applied Off in memory, then decided whether to save by comparing the request
  with memory. After a failed save, retrying the same Off compared equal, skipped
  the save and acknowledged success while the file still granted the source; the
  next start read the file and revived the grant without consent.
- Choice: the owner keeps what the file holds as far as it knows (`persisted`,
  `None` after an unreadable file or a failed save) and decides the write against
  that. Revocation still applies in memory at once; an acknowledgement now always
  means the file holds the choice.
- Alternatives: a dirty flag beside memory (same effect, one more state to keep
  consistent); re-reading the file before every choice (extra I/O and a race with
  the owner's own writes).
- Cost: one extra write when a file was unreadable at start. Regression tests in
  `vessel-node::usage_reports::sources::tests::persistence` fail a save through a
  read-only data directory, retry Off, and read the consent through a restarted owner.
